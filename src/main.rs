use clap::{App, Arg};
use console::style;
use std::error::Error;
use std::fs;
use std::path::Path;
use steamgiftsbot::steamgifts_acc;

extern crate clap;

// TODO: http://a8m.github.io/pb/doc/pbr/index.html
fn main() -> Result<(), Box<dyn Error>> {
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    println!("{}", style("Started.").green());
    let matches = App::new("steamgifts.com bot")
        .version(VERSION)
        .author("knightpp")
        .about("steamgifts bot rewritten in Rust!")
        .arg(
            Arg::with_name("cookie file")
                .short("c")
                .long("config")
                .help("Sets a path to a cookie file")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("cookie")
                .long("cookie")
                .help("Cookie value, string after 'PHPSESSID='. Automatically saves to file.")
                .takes_value(true),
        )
        .get_matches();
    run(matches)?;
    println!("{}", style("Done.").green());

    if cfg!(target_os = "windows") {
        use std::process::Command;
        let _ = Command::new("cmd.exe").arg("/c").arg("pause").status();
    }
    Ok(())
}

struct Config<'a> {
    matches: clap::ArgMatches<'a>,
}
impl Config<'_> {
    pub fn new(matches: clap::ArgMatches) -> Config {
        Config { matches }
    }
    pub fn get_cookie(&self) -> Result<String, Box<dyn Error>> {
        let cookie_file = self.matches.value_of("cookie file").unwrap_or("cookie.txt");
        let cookie_arg = self.matches.value_of("cookie").or(None);

        if let Some(cookie) = cookie_arg {
            fs::write(cookie_file, cookie)?;
            return Ok(cookie.to_string());
        }

        return if Path::new(cookie_file).exists() {
            let file_content = fs::read_to_string(cookie_file)?;
            let first_line = file_content
                .lines()
                .nth(0)
                .ok_or(format!("failed to read from '{}'", cookie_file))?
                .to_string();
            println!(
                "read {} bytes from file '{}'",
                first_line.len(),
                cookie_file
            );
            Ok(first_line)
        } else {
            Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("cookie file '{}' not found", cookie_file),
            )))
        };
    }
}

fn run(matches: clap::ArgMatches) -> Result<(), Box<dyn Error>> {
    let cookie = Config::new(matches).get_cookie()?;
    let acc = steamgifts_acc::new(cookie)?;
    let mut giveaways = acc.parse_vector()?;

    if giveaways.len() == 0 {
        return Err(Box::new(simple_error::SimpleError::new(
            "None giveaways was parsed.",
        )));
    }

    // expensive first
    giveaways.sort_by(|a, b| a.get_price().cmp(&b.get_price()).reverse());

    let mut funds = acc.get_points();
    println!("Points available: {}", style(funds).bold().yellow());
    std::thread::sleep(std::time::Duration::from_secs(5));
    for ga in giveaways.iter() {
        if funds > ga.get_price() {
            println!("{}", ga);
            funds = acc.enter_giveaway(ga)?;
        } else {
            continue;
        }
        std::thread::sleep(std::time::Duration::from_secs(5));
    }
    Ok(())
}
