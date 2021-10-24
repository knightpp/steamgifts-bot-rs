use async_std::task;
use console::style;
use serde::Deserialize;
use simplelog::LevelFilter;
use std::{cmp::Ordering, time::Duration};
use steamgiftsbot::account::{self, Account};
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
    app.with(tide::utils::After(|res: tide::Response| async {
        if let Some(e) = res.error() {
            log::error!("{}", e);
        }
        Ok(res)
    }));

    #[cfg(feature = "profile")]
    {
        let profiler_guard = Box::leak(Box::new(pprof::ProfilerGuard::new(10000).unwrap()));
        app.at("/profile")
            .get(|r| async { generate_report(r, profiler_guard).await });
    }

    app.at("/run").post(run_message);
    let port = std::env::var("PORT").expect("PORT env var not found");
    let addr = format!("0.0.0.0:{}", port);
    app.listen(addr).await?;
    Ok(())
}

async fn run_message(mut req: Request<()>) -> tide::Result {
    let m: Message = req.body_json().await?;
    task::spawn(async move {
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
    let acc = Account::login(msg.cookie.clone()).await?;
    let mut giveaways = acc.parse_giveaways().await?;

    if giveaways.is_empty() {
        return Err(anyhow::anyhow!("zero giveaways was parsed"));
    }

    if let Some(dur) = msg.filter_time.as_ref() {
        let dur = humantime::parse_duration(dur)?;
        giveaways = giveaways
            .into_iter()
            .filter(|g| g.ends_in.cmp(&dur).is_le())
            .collect();
    }
    use account::entry::Entry;
    // expensive first
    let sorter: fn(&Entry, &Entry) -> Ordering = match msg.sort_by {
        SortStrategy::Chance => |lhs, rhs| {
            (lhs.copies as f64 / lhs.entries as f64)
                .partial_cmp(&(rhs.copies as f64 / rhs.entries as f64))
                .unwrap_or(Ordering::Less)
                .reverse()
        },
        SortStrategy::Price => |lhs, rhs| lhs.price.cmp(&rhs.price).reverse(),
    };

    giveaways.sort_by(|lhs, rhs| {
        let r = sorter(lhs, rhs);
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
    for ga in &giveaways {
        if funds >= ga.price {
            if let Ok(updated_funds) = acc.enter_giveaway(ga).await {
                log::info!("{}", ga);
                funds = updated_funds;
                task::sleep(Duration::from_secs(5)).await;
            }
        }
    }
    Ok(())
}

#[cfg(feature = "profile")]
async fn generate_report(_: Request<()>, guard: &'_ pprof::ProfilerGuard<'_>) -> tide::Result {
    if let Ok(report) = guard.report().build() {
        let profile = report.pprof().unwrap();
        let mut content = Vec::new();

        use pprof::protos::Message;
        profile.encode(&mut content).unwrap();

        Ok(tide::Body::from_bytes(content).into())
    } else {
        Ok(
            tide::Response::builder(tide::StatusCode::InternalServerError)
                .body("Failed to generate report")
                .build(),
        )
    }
}
