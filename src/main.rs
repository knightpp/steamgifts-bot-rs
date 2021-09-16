use anyhow::{Context, Result};
use argh::FromArgs;
use console::style;
use std::{
    cmp::Ordering,
    fmt::{self},
    fs,
    path::{Path, PathBuf},
    str::FromStr,
    thread::sleep,
    time::{Duration, SystemTime},
};
use steamgiftsbot::steamgifts_acc;

#[derive(FromArgs)]
/** http://steamgifts.com bot written in Rust!
When no arguments supplied then a cookie will be read from `cookie.txt` */
struct Opt {
    /// set a path to a cookie file
    #[argh(option, short = 'f')]
    cookie_file: Option<PathBuf>,

    /// cookie value, string after 'PHPSESSID=', automatically saves to file
    #[argh(option, short = 'c')]
    cookie: Option<String>,

    /// daemonize
    #[argh(switch, short = 'd')]
    daemon: bool,

    /// filters giveaways that ends in X or earlier
    #[argh(option, short = 't')]
    filter_time: Option<humantime::Duration>,

    /// sorting strategy allowed values are: [chance, price]
    #[argh(option, default = "SortStrategy::Chance", short = 's')]
    sort_by: SortStrategy,

    /// reverse sorting
    #[argh(switch)]
    reverse: bool,
}

#[derive(Debug)]
enum SortStrategy {
    Chance,
    Price,
}
impl FromStr for SortStrategy {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use SortStrategy::*;
        let s = s.to_lowercase();
        if s.starts_with('c') {
            Ok(Chance)
        } else if s.starts_with('p') {
            Ok(Price)
        } else {
            Err("expected `chance` or `price`".to_owned())
        }
    }
}
impl fmt::Display for SortStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SortStrategy::Chance => f.write_str("chance"),
            SortStrategy::Price => f.write_str("price"),
        }
    }
}

fn main() -> Result<()> {
    let matches: Opt = argh::from_env();
    if matches.daemon {
        let epoch = SystemTime::UNIX_EPOCH;
        let some_seed = (SystemTime::now().duration_since(epoch)).unwrap().as_secs();
        let mut rng = oorandom::Rand32::new(some_seed);
        loop {
            let secs = rng.rand_range(5..15) as u64;
            let sl = || sleep(Duration::from_secs(secs));
            if let Err(x) = run(&matches, sl) {
                eprintln!("Error: {}", style(x).red());
            }
            sleep(Duration::from_secs(60 * 60));
        }
    } else {
        run(&matches, || pretty_sleep(Duration::from_secs(5)))?;
    }
    Ok(())
}

impl Opt {
    pub fn get_cookie(&self) -> Result<String> {
        let cookie_file = self
            .cookie_file
            .as_deref()
            .unwrap_or_else(|| Path::new("cookie.txt"));
        let cookie_arg = self.cookie.as_ref();

        if let Some(cookie) = cookie_arg {
            if let Err(e) = fs::write(cookie_file, cookie) {
                eprintln!("WARNING: cannot write file; {}", e);
            }
            return Ok(cookie.to_string());
        }

        if cookie_file.exists() {
            let file_content = fs::read_to_string(cookie_file)?;
            let first_line = file_content
                .lines()
                .next()
                .with_context(|| format!("failed to read from '{}'", cookie_file.display()))?
                .to_string();
            println!(
                "read {} bytes from file '{}'",
                first_line.len(),
                cookie_file.display()
            );
            Ok(first_line)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("file '{}' not found", cookie_file.display()),
            ))
            .context("cookie file not found")
        }
    }
}

fn run<F: Fn()>(matches: &Opt, sleep: F) -> Result<()> {
    let cookie = matches.get_cookie()?;
    let acc = steamgifts_acc::new(cookie)?;
    let mut giveaways = acc.parse_vector()?;

    if giveaways.is_empty() {
        return Err(anyhow::Error::msg("none giveaways was parsed"));
    }

    if let Some(dur) = matches.filter_time.as_ref() {
        giveaways = giveaways
            .into_iter()
            .filter(|a| a.ends_in.cmp(dur).is_le())
            .collect();
        println!("Found {} giveaways that ends in < {}", giveaways.len(), dur)
    } else {
        println!("Found {} giveaways", giveaways.len())
    }
    use steamgifts_acc::entry::Entry;
    // expensive first
    let sorter: fn(&Entry, &Entry) -> Ordering = match matches.sort_by {
        SortStrategy::Chance => |a, b| {
            (a.copies as f64 / a.entries as f64)
                .partial_cmp(&(b.copies as f64 / b.entries as f64))
                .unwrap_or(Ordering::Less)
                .reverse()
        },
        SortStrategy::Price => |a, b| a.price.cmp(&b.price).reverse(),
    };

    giveaways.sort_by(|a, b| {
        let r = sorter(a, b);
        if matches.reverse {
            r.reverse()
        } else {
            r
        }
    });
    let mut funds = acc.get_points()?;
    println!("Points available: {}", style(funds).bold().yellow());
    //std::thread::sleep(std::time::Duration::from_secs(5));
    //pretty_sleep(std::time::Duration::from_millis(5000));
    sleep();
    for ga in giveaways.iter() {
        if funds > ga.price {
            println!("{}", ga);
            funds = if let Ok(x) = acc.enter_giveaway(ga) {
                x
            } else {
                continue;
            };
        } else {
            continue;
        }
        //pretty_sleep(std::time::Duration::from_millis(5000));
        //std::thread::sleep(std::time::Duration::from_secs(5));
        sleep();
    }
    Ok(())
}

fn pretty_sleep(dur: Duration) {
    use std::convert::TryInto;
    const PB_WIDTH: usize = 70;
    const REFRESH_EVERY_MS: u64 = 100;
    let ms = dur.as_millis();
    debug_assert_eq!(ms.try_into(), Ok(ms as u64));
    let ms = ms as u64;
    debug_assert!(ms > REFRESH_EVERY_MS);
    let mut pb = pbr::ProgressBar::new(ms);
    pb.show_speed = false;
    pb.show_percent = false;

    pb.set_width(Some(PB_WIDTH));
    for _ in 0..(ms / REFRESH_EVERY_MS) {
        //pb.inc();
        pb.add(REFRESH_EVERY_MS);
        sleep(Duration::from_millis(REFRESH_EVERY_MS));
    }
    pb.finish_print(""); // clear by printing whitespaces
    print!("\r"); // return to start of the line
}
