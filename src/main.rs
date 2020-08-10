use anyhow::{Context, Result};
use argh::FromArgs;
use console::style;
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
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
}

fn main() -> Result<()> {
    let matches: Opt = argh::from_env();
    run(matches)?;
    Ok(())
}

impl Opt {
    pub fn get_cookie(&self) -> Result<String> {
        let cookie_file = self
            .cookie_file
            .as_ref()
            .map(|f| f.as_path())
            .unwrap_or(Path::new("cookie.txt"));
        let cookie_arg = self.cookie.as_ref();

        if let Some(cookie) = cookie_arg {
            fs::write(cookie_file, cookie)?;
            return Ok(cookie.to_string());
        }

        if cookie_file.exists() {
            let file_content = fs::read_to_string(cookie_file)?;
            let first_line = file_content
                .lines()
                .nth(0)
                .context(format!("failed to read from '{}'", cookie_file.display()))?
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

fn run(matches: Opt) -> Result<()> {
    let cookie = matches.get_cookie()?;
    let acc = steamgifts_acc::new(cookie)?;
    let mut giveaways = acc.parse_vector()?;

    if giveaways.is_empty() {
        return Err(anyhow::Error::msg("none giveaways was parsed"));
    }

    // expensive first
    giveaways.sort_by(|a, b| a.get_price().cmp(&b.get_price()).reverse());

    let mut funds = acc.get_points();
    println!("Points available: {}", style(funds).bold().yellow());
    //std::thread::sleep(std::time::Duration::from_secs(5));
    pretty_sleep(std::time::Duration::from_millis(5000));

    for ga in giveaways.iter() {
        if funds > ga.get_price() {
            println!("{}", ga);
            funds = acc.enter_giveaway(ga)?;
        } else {
            continue;
        }
        pretty_sleep(std::time::Duration::from_millis(5000));
        //std::thread::sleep(std::time::Duration::from_secs(5));
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
        std::thread::sleep(Duration::from_millis(REFRESH_EVERY_MS));
    }
    pb.finish_print(""); // clear by printing whitespaces
    print!("\r"); // return to start of the line
}
