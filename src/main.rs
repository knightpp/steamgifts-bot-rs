use async_std::task;
use console::style;
use serde::Deserialize;
use simplelog::LevelFilter;
use std::{cmp::Ordering, time::Duration};
use steamgiftsbot::steamgifts_acc;
use tide::Request;

#[derive(Debug, Deserialize)]
struct Message {
    cookie: String,
    #[serde(default = "default_filter_time")]
    filter_time: Option<String>,
    #[serde(default = "SortStrategy::by_chance")]
    sort_by: SortStrategy,
    #[serde(default)]
    reverse: bool,
}

fn default_filter_time() -> Option<String> {
    Some("1h".to_owned())
}

#[derive(Debug, Deserialize)]
enum SortStrategy {
    Chance,
    Price,
}
impl SortStrategy {
    fn by_chance() -> Self {
        SortStrategy::Chance
    }
}

#[async_std::main]
async fn main() -> tide::Result<()> {
    simplelog::SimpleLogger::init(
        LevelFilter::Info,
        simplelog::ConfigBuilder::new()
            .add_filter_ignore_str("surf")
            .build(),
    )
    .unwrap();
    let mut app = tide::new();
    app.at("/run").post(run_message);
    let port = std::env::var("PORT").expect("PORT env var not found");
    let addr = format!("0.0.0.0:{}", port);
    app.listen(addr).await?;
    Ok(())
}

async fn run_message(mut req: Request<()>) -> tide::Result {
    let m: Message = req.body_json().await.map_err(|x| {
        log::error!("Err: {:?}", x);
        x
    })?;
    task::spawn(async move {
        let m = m;
        log::info!("{}: Run started", m.cookie);
        if let Result::Err(e) = run(&m).await {
            log::error!("Err: {:?}", e);
        }
        log::info!("{}: Run finished", m.cookie);
    });
    Ok("".into())
}

async fn run(msg: &Message) -> Result<(), anyhow::Error> {
    log::info!("RUN with msg: {:?}", msg);
    let acc = steamgifts_acc::new(msg.cookie.clone()).await?;
    let mut giveaways = acc.parse_vector().await?;

    if giveaways.is_empty() {
        return Err(anyhow::anyhow!("zero giveaways was parsed"));
    }

    if let Some(dur) = msg.filter_time.as_ref() {
        let dur = humantime::parse_duration(dur)?;
        giveaways = giveaways
            .into_iter()
            .filter(|a| a.ends_in.cmp(&dur).is_le())
            .collect();
    }
    use steamgifts_acc::entry::Entry;
    // expensive first
    let sorter: fn(&Entry, &Entry) -> Ordering = match msg.sort_by {
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
        if msg.reverse {
            r.reverse()
        } else {
            r
        }
    });
    let mut funds = acc.get_points().await?;
    log::info!(
        "Points available: {}, Found giveaways: {}",
        style(funds).bold().yellow(),
        giveaways.len()
    );
    for ga in giveaways.iter() {
        if funds > ga.price {
            log::info!("{}", ga);
            funds = if let Ok(x) = acc.enter_giveaway(ga).await {
                x
            } else {
                continue;
            };
        } else {
            continue;
        }
        task::sleep(Duration::from_secs(5)).await;
    }
    Ok(())
}
