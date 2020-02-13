use clap::{App, Arg};
use console::style;
use std::error::Error;
use std::fs;
use std::path::Path;
use steamgiftsbot::steamgifts_acc;

extern crate clap;

// TODO: http://a8m.github.io/pb/doc/pbr/index.html
fn main() -> Result<(), Box<dyn Error>> {
    println!("{}", style("Started.").green());
    let matches = App::new("steamgifts.com bot")
        .version("0.1.0")
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
    match run(&matches) {
        Ok(()) => println!("{}", style("Done.").green()),
        Err(x) => println!("{}", style(format!("Error: {}", x)).red()),
    }

    if cfg!(target_os = "windows") {
        use std::process::Command;
        let _ = Command::new("cmd.exe").arg("/c").arg("pause").status();
    }
    Ok(())
}

fn run(matches: &clap::ArgMatches) -> Result<(), Box<dyn Error>> {
    let config = matches.value_of("cookie file").unwrap_or("cookie.txt");
    let config_exists = Path::new(config).exists();
    let cookie = matches.value_of("cookie").or(None);
    // if no cookie given, try find it in file
    let cookie = if cookie == None {
        if config_exists {
            let file_content = fs::read_to_string(config)?;
            let first_line = file_content.lines().nth(0).unwrap().to_owned();
            println!("read {} bytes from file '{}'", first_line.len(), config);
            first_line
        } else {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("cookie file '{}' not found", config),
            )));
        }
    } else {
        let cookie = cookie.unwrap();
        if !config_exists {
            fs::write(config, cookie)?;
        }
        cookie.to_owned()
    };

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
