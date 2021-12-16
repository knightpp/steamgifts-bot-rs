pub mod entry;
use crate::Error;
use scraper::html::Html;
use scraper::Selector;
use std::borrow::Cow;
use std::time::Duration;

use self::entry::Entry;

const USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:73.0) Gecko/20100101 Firefox/73.0";

#[derive(Debug)]
pub struct URL<'c> {
    url_string: Cow<'c, str>,
}

impl<'c> URL<'c> {
    fn main() -> Self {
        URL::new(Cow::Borrowed("https://www.steamgifts.com/"))
    }

    fn href<T: AsRef<str>>(path: T) -> Self {
        URL::new(Cow::Owned(format!(
            "https://www.steamgifts.com{}",
            path.as_ref()
        )))
    }

    fn post() -> Self {
        URL::new(Cow::Borrowed("https://www.steamgifts.com/ajax.php"))
    }

    fn new(cow: Cow<'c, str>) -> URL<'c> {
        URL { url_string: cow }
    }
}

impl<'u> AsRef<str> for URL<'u> {
    fn as_ref(&self) -> &str {
        &self.url_string
    }
}

impl<'c> std::fmt::Display for URL<'c> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.url_string.as_ref())
    }
}

pub struct Account {
    cookie: String,
    xsrf: String,
}

impl Account {
    pub async fn login(cookie: String) -> Result<Account, Error> {
        let mut acc = Account {
            cookie,
            xsrf: String::new(),
        };
        acc.load_xsrf().await?;
        Ok(acc)
    }

    async fn load_xsrf(&mut self) -> Result<(), Error> {
        let doc = self.do_get(URL::main()).await?.body_string().await?;
        let doc = scraper::html::Html::parse_document(&doc);
        let selector = Selector::parse("input[name=\"xsrf_token\"]").unwrap();
        let error_msg = || Error::Login("xsrf_token");
        let xsrf = doc
            .select(&selector)
            .next()
            .ok_or_else(error_msg)?
            .value()
            .attr("value")
            .ok_or_else(error_msg)?
            .to_string();
        self.xsrf = xsrf;
        Ok(())
    }

    pub async fn enter_giveaway(&self, ga: &Entry<'_>) -> Result<u32, Error> {
        let mut response = self.do_enter(ga).await?;
        if response.status() != 200 {
            return Err(Error::StatusCode(response.status() as u16));
        }

        #[derive(Debug, serde::Deserialize)]
        struct ApiResponse {
            #[serde(rename = "type")]
            result: String,
            points: String,
            entry_count: Option<String>,
            msg: Option<String>,
        }

        let resp: ApiResponse = response.body_json().await?;
        match resp.result.as_str() {
            "success" => Ok(resp.points.parse()?),
            "error" => Err(Error::Json("failed to enter GA")),
            unknown => Err(Error::Message(format!(
                "unknown API response status: {}: {:?}",
                unknown, resp.msg
            ))),
        }
    }

    async fn do_enter(&self, entry: &'_ Entry<'_>) -> Result<surf::Response, Error> {
        let cookie = format!("PHPSESSID={}", self.cookie);
        let post_data = format!(
            "xsrf_token={}&do=entry_insert&code={}",
            self.xsrf,
            entry.get_code()
        );
        let resp = surf::post(URL::post())
            .header("Host", "www.steamgifts.com")
            .header("User-Agent", USER_AGENT)
            .header("Accept", "application/json, text/javascript, */*; q=0.01")
            .header("Referer", entry.href.as_ref())
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("X-Requested-With", "XMLHttpRequest")
            .header("DNT", "1")
            .header("Connection", "close")
            .header("Cookie", cookie)
            .header("TE", "Trailers")
            .body_string(post_data)
            .send()
            .await?;
        Ok(resp)
    }

    pub async fn get_points(&self) -> Result<u32, Error> {
        let html = self.do_get(URL::main()).await?.body_string().await?;
        let doc = scraper::html::Html::parse_document(html.as_str());
        let points_balance_selector = Selector::parse("span.nav__points").unwrap();
        let points = doc
            .select(&points_balance_selector)
            .next()
            .ok_or_else(|| Error::Message("Cannot parse balance".to_owned()))?
            .inner_html();
        points.as_str().extract_number()
    }

    pub async fn parse_giveaways(&self) -> Result<Vec<Entry<'_>>, Error> {
        let html = self.do_get(URL::main()).await?.body_string().await?;
        let doc: Html = Html::parse_document(&html);
        let giveaway_selector = Selector::parse(
            "div.giveaway__row-outer-wrap[data-game-id] div[class=giveaway__row-inner-wrap]",
        )
        .unwrap();
        let mut giveaways = Vec::with_capacity(50);
        for el in doc.select(&giveaway_selector) {
            let entry = Entry {
                name: Account::select_name(&el),
                price: Account::select_points(&el)?,
                copies: Account::select_copies(&el)?,
                entries: Account::select_entries(&el)?,
                href: URL::href(Account::select_href(&el)),
                ends_in: Account::select_time(&el)?,
            };
            giveaways.push(entry);
        }

        Ok(giveaways)
    }

    async fn do_get(&self, url: URL<'_>) -> Result<surf::Response, Error> {
        let resp = surf::get(url)
            .header(
                "Accept",
                "text/html, application/xhtml+xml, application/xml",
            )
            .header("Cookie", format!("PHPSESSID={}", self.cookie))
            .header("User-Agent", USER_AGENT)
            .header("Host", "www.steamgifts.com")
            .header("Referer", "https://www.steamgifts.com")
            .header("TE", "Trailers")
            .header("Upgrade-Insecure-Requests", "1")
            .send()
            .await?;
        Ok(resp)
    }
}

impl Account {
    fn select_name(el: &scraper::ElementRef) -> String {
        let name_selector = Selector::parse("a.giveaway__heading__name").unwrap();
        el.select(&name_selector).next().unwrap().inner_html()
    }

    fn select_entries(el: &scraper::ElementRef) -> Result<u32, Error> {
        let entries_selector = Selector::parse("div.giveaway__links a[href] span").unwrap();
        el.select(&entries_selector)
            .next()
            .unwrap()
            .inner_html()
            .as_str()
            .extract_number()
    }

    fn select_href(el: &scraper::ElementRef) -> String {
        let href_selector =
            Selector::parse("h2.giveaway__heading a.giveaway__heading__name[href]").unwrap();
        let href = el
            .select(&href_selector)
            .next()
            .expect("href not found!")
            .value()
            .attr("href")
            .unwrap();
        href.to_string()
    }

    fn select_points(el: &scraper::ElementRef) -> Result<u32, Error> {
        let points_copies_selector =
            Selector::parse("h2.giveaway__heading span.giveaway__heading__thin").unwrap();
        let arr = el.select(&points_copies_selector);
        let v: Vec<_> = arr.collect();
        if v.is_empty() {
            panic!("select_points(): vector is empty");
        }
        if v.len() > 1 {
            v[1].inner_html().as_str().extract_number()
        } else {
            v[0].inner_html().as_str().extract_number()
        }
    }

    fn select_copies(el: &scraper::ElementRef) -> Result<u32, Error> {
        let points_copies_selector =
            Selector::parse("h2.giveaway__heading span.giveaway__heading__thin").unwrap();
        let arr = el.select(&points_copies_selector);
        let v: Vec<_> = arr.collect();
        if v.is_empty() {
            panic!("select_copies(): vector is empty");
        }
        if v.len() > 1 {
            v[0].inner_html().as_str().extract_number()
        } else {
            Ok(1)
        }
    }

    fn select_time(el: &scraper::ElementRef) -> Result<Duration, Error> {
        let time_selector = Selector::parse(".giveaway__columns span[data-timestamp]").unwrap();
        Ok(el
            .select(&time_selector)
            .next()
            .map(|a| a.text().collect::<String>())
            .map(|x| humantime::parse_duration(&x))
            .ok_or(Error::Unknown)??)
    }
}

trait MyTrait {
    fn extract_number(&self) -> Result<u32, Error>;
}
impl MyTrait for &str {
    fn extract_number(&self) -> Result<u32, Error> {
        let out = self
            .chars()
            .filter(|ch| ch.is_numeric())
            .collect::<String>();
        Ok(out.parse()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const GIVEAWAY_HTML: &'static str = r#"<div class="giveaway__row-inner-wrap">
					<div class="giveaway__summary">
						<h2 class="giveaway__heading">
							<a class="giveaway__heading__name" href="/giveaway/IFesR/spandex-force-champion-rising">Spandex Force: Champion Rising</a><span class="giveaway__heading__thin">(3P)</span><a class="giveaway__icon" rel="nofollow noopener" target="_blank" href="https://store.steampowered.com/app/380560/"><i class="fa fa-steam"></i></a><a class="giveaway__icon" href="/giveaways/search?app=380560"><i class="fa fa-search"></i></a><i data-popup="popup--hide-games" class="giveaway__icon giveaway__hide trigger-popup fa fa-eye-slash"></i>
						</h2>
						<div class="giveaway__columns">
							<div><i class="fa fa-clock-o"></i> <span data-timestamp="1580325720">2 hours</span> remaining</div><div class="giveaway__column--width-fill text-right"><span data-timestamp="1580314999">27 minutes</span> ago by <a class="giveaway__username" href="/user/fanchiotti">fanchiotti</a></div></div>
							<div class="giveaway__links">
								<a href="/giveaway/IFesR/spandex-force-champion-rising/entries"><i class="fa fa-tag"></i> <span>237 entries</span></a>
								<a href="/giveaway/IFesR/spandex-force-champion-rising/comments"><i class="fa fa-comment"></i> <span>0 comments</span></a>
							</div>
						</div><a href="/user/fanchiotti" class="giveaway_image_avatar" style="background-image:url(https://steamcdn-a.akamaihd.net/steamcommunity/public/images/avatars/d2/d25f15d279f1085d316d89f61ff9c8fc1b626185_medium.jpg);"></a><a class="giveaway_image_thumbnail" style="background-image:url(https://steamcdn-a.akamaihd.net/steam/apps/380560/capsule_184x69.jpg);" href="/giveaway/IFesR/spandex-force-champion-rising"></a>
				</div>"#;

    #[test]
    fn extract_number() {
        let str_num = "QWE123ZXCC456";
        assert_eq!(str_num.extract_number().unwrap(), 123456);
    }
    #[test]
    fn extract_number_no_numbers() {
        let str_num = "string with no numbers#@!#$%%^&*";
        assert_eq!(str_num.extract_number().is_err(), true);
    }
    #[test]
    fn select_name() {
        let fragment = Html::parse_fragment(GIVEAWAY_HTML);
        let name = super::Account::select_name(&fragment.root_element());
        assert_eq!(name, "Spandex Force: Champion Rising");
    }
    #[test]
    fn select_entries() {
        let fragment = Html::parse_fragment(GIVEAWAY_HTML);
        let entries = super::Account::select_entries(&fragment.root_element());
        assert_eq!(entries.unwrap(), 237u32);
    }
    #[test]
    fn select_href() {
        let fragment = Html::parse_fragment(GIVEAWAY_HTML);
        let href = super::Account::select_href(&fragment.root_element());
        assert_eq!(href, "/giveaway/IFesR/spandex-force-champion-rising");
    }
    #[test]
    fn select_points() {
        let fragment = Html::parse_fragment(GIVEAWAY_HTML);
        let points = super::Account::select_points(&fragment.root_element());
        assert_eq!(points.unwrap(), 3u32);
    }
    #[test]
    fn select_copies() {
        let fragment = Html::parse_fragment(GIVEAWAY_HTML);
        let copies = super::Account::select_copies(&fragment.root_element());
        assert_eq!(copies.unwrap(), 1u32);
    }
}
