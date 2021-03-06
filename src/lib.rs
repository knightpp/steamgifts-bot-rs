use std::num::ParseIntError;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum SgbError {
    #[error("expected status code 200, got {0}")]
    StatusCode(u16),
    #[error("json failed: {0}")]
    Json(&'static str),
    #[error("io error")]
    Io(#[from] std::io::Error),
    #[error("check your internet connection: {0}")]
    Internet(#[from]ureq::Error),
    #[error("failed logging in: {0}")]
    Login(&'static str),
    #[error("failed entering GA: {0}")]
    Enter(String),
    #[error("unknown error")]
    Unknown,
    #[error("could not parse integer from string")]
    ParseError(#[from] ParseIntError),
}

pub mod steamgifts_acc {
    pub mod entry;
    use crate::SgbError;
    use entry::Entry;
    use scraper::html::Html;
    use scraper::Selector;
    use ureq::SerdeValue;
    use std::borrow::Cow;

    #[derive(Debug)]
    pub struct URL<'c> {
        url_string: Cow<'c, str>,
    }

    impl<'c> URL<'c> {
        fn new(typ: URLType) -> URL<'c> {
            URL {
                url_string: match typ {
                    URLType::Main => Cow::Borrowed("https://www.steamgifts.com/"),
                    URLType::Href(x) => {
                        Cow::Owned(format!("{}{}", "https://www.steamgifts.com", x))
                    }
                    URLType::Post => Cow::Borrowed("https://www.steamgifts.com/ajax.php"),
                },
            }
        }
        pub fn as_str(&self) -> &str {
            self.url_string.as_ref()
        }
        pub fn to_string(&self) -> String {
            self.url_string.to_string()
        }
    }

    #[derive(Debug)]
    pub enum URLType {
        Main,
        Href(String),
        Post,
    }
    pub fn new(cookie: String) -> Result<SteamgiftsAcc, SgbError> {
        let xsrf = SteamgiftsAcc::get_xsrf(cookie.as_str())?;
        let acc = SteamgiftsAcc { cookie, xsrf };
        Ok(acc)
    }
    pub struct SteamgiftsAcc {
        cookie: String,
        xsrf: String,
    }
    // * Public decls
    impl SteamgiftsAcc {
        /// # Panics:
        /// * JSON response don't contains 'error' nor 'success' field
        /// * Failed to parse HTML
        /// * Tried parse number from string with no digits
        pub fn enter_giveaway(&self, ga: &Entry) -> Result<u32, SgbError> {
            // TODO: refactor
            let response = SteamgiftsAcc::post(self.cookie.as_str(), self.xsrf.as_str(), ga)?;
            if response.status() != 200 {
                return Err(SgbError::StatusCode(response.status()));
            }
            let json : SerdeValue = response.into_json()?;
            let msg_type = json
                .get("type")
                .ok_or(SgbError::Json("couldn't find 'type' field"))?
                .as_str()
                .unwrap();
            match msg_type {
                "success" => {}
                "error" => return Err(SgbError::Json("failed to enter GA")),
                msg => return Err(SgbError::Enter(format!("got msg = {}", msg))),
            };
            let points = json
                .get("points")
                .ok_or(SgbError::Json("couldn't find 'points' field"))?
                .as_str()
                .unwrap();
            points.extract_number()
        }
        pub fn get_points(&self) -> Result<u32, SgbError> {
            let html = SteamgiftsAcc::get(self.cookie.as_str(), URL::new(URLType::Main))?
                .into_string()
                .unwrap();
            let doc = scraper::html::Html::parse_document(html.as_str());
            let points_balance_selector = Selector::parse("span.nav__points").unwrap();
            let points = doc
                .select(&points_balance_selector)
                .nth(0)
                .expect("Cannot parse balance")
                .inner_html();
            points.as_str().extract_number()
        }
        pub fn parse_vector(&self) -> Result<Vec<Entry>, SgbError> {
            let html =
                SteamgiftsAcc::get(self.cookie.as_str(), URL::new(URLType::Main))?.into_string()?;
            let doc: Html = Html::parse_document(html.as_str());
            let giveaway_selector = Selector::parse(
                "div.giveaway__row-outer-wrap[data-game-id] div[class=giveaway__row-inner-wrap]",
            )
            .unwrap();
            let mut giveaways = Vec::with_capacity(50);
            for el in doc.select(&giveaway_selector) {
                let name = SteamgiftsAcc::select_name(&el);
                let points = SteamgiftsAcc::select_points(&el)?;
                let copies = SteamgiftsAcc::select_copies(&el)?;
                let entries = SteamgiftsAcc::select_entries(&el)?;
                let href = SteamgiftsAcc::select_href(&el);
                let entry =
                    Entry::new(name, URL::new(URLType::Href(href)), points, copies, entries);
                giveaways.push(entry);
            }

            Ok(giveaways)
        }
    }
    // * End Public decls
    // * Private decls
    impl SteamgiftsAcc {
        fn select_name(el: &scraper::ElementRef) -> String {
            let name_selector = Selector::parse("a.giveaway__heading__name").unwrap();
            el.select(&name_selector).nth(0).unwrap().inner_html()
        }
        fn select_entries(el: &scraper::ElementRef) -> Result<u32, SgbError> {
            let entries_selector = Selector::parse("div.giveaway__links a[href] span").unwrap();
            Ok(el
                .select(&entries_selector)
                .nth(0)
                .unwrap()
                .inner_html()
                .as_str()
                .extract_number()?)
        }
        fn select_href(el: &scraper::ElementRef) -> String {
            let href_selector =
                Selector::parse("h2.giveaway__heading a.giveaway__heading__name[href]").unwrap();
            let href = el
                .select(&href_selector)
                .nth(0)
                .expect("href not found!")
                .value()
                .attr("href")
                .unwrap();
            href.to_string()
        }
        fn select_points(el: &scraper::ElementRef) -> Result<u32, SgbError> {
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
        fn select_copies(el: &scraper::ElementRef) -> Result<u32, SgbError> {
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
        fn get_xsrf(cookie: &str) -> Result<String, SgbError> {
            let doc = SteamgiftsAcc::get(cookie, URL::new(URLType::Main))?.into_string()?;
            let doc: scraper::html::Html = scraper::html::Html::parse_document(doc.as_str());
            let selector = Selector::parse("input[name=\"xsrf_token\"]").unwrap();
            let error_msg = || SgbError::Login("xsrf_token");
            let out = doc
                .select(&selector)
                .nth(0)
                .ok_or_else(error_msg)?
                .value()
                .attr("value")
                .ok_or_else(error_msg)?
                .to_string();
            Ok(out)
        }
        // TODO save state of ureq::Request, too many construct for a simple get
        fn get(cookie: &str, url: URL) -> Result<ureq::Response, SgbError> {
            let url = url.to_string();
            let resp = ureq::get(url.as_str())
                // .timeout_connect(30_000)
                .set(
                    "Accept",
                    "text/html, application/xhtml+xml, application/xml",
                )
                //.set("Connection", "close")
                .set("Cookie", format!("PHPSESSID={}", cookie).as_str())
                // .set("DNT", "1")
                .set(
                    "User-Agent",
                    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:73.0) Gecko/20100101 Firefox/73.0",
                )
                .set("Host", "www.steamgifts.com")
                .set("Referer", "https://www.steamgifts.com")
                .set("TE", "Trailers")
                .set("Upgrade-Insecure-Requests", "1")
                .call()?;
            Ok(resp)
        }
        fn post(cookie: &str, xsrf: &str, entry: &Entry) -> Result<ureq::Response, SgbError> {
            let cookie = format!("PHPSESSID={}", cookie);
            let referer = entry.get_href().to_string();
            let post_data = format!(
                "xsrf_token={}&do=entry_insert&code={}",
                xsrf,
                entry.get_code()
            );
            let resp = ureq::post(URL::new(URLType::Post).as_str())
                .set("Host", "www.steamgifts.com")
                .set(
                    "User-Agent",
                    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/68.0.3440.84 Safari/537.36",
                )
                .set("Accept", "application/json, text/javascript, */*; q=0.01")
                .set("Referer", referer.as_str())
                .set(
                    "Content-Type",
                    "application/x-www-form-urlencoded",
                )
                .set("X-Requested-With", "XMLHttpRequest")
               // .set("Origin", "https://www.steamgifts.com")
                //.set("Content-Length", "70")
                .set("DNT", "1")
                .set("Connection", "close")
                .set("Cookie", cookie.as_str())
                .set("TE", "Trailers")
                // .timeout_connect(30_000)
                .send_string(post_data.as_str())?;
            Ok(resp)
        }
    }
    // * End Private decls
    trait MyTrait {
        fn extract_number(&self) -> Result<u32, SgbError>;
    }
    impl MyTrait for &str {
        fn extract_number(&self) -> Result<u32, SgbError> {
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
        #[should_panic]
        fn extract_number_no_numbers() {
            let str_num = "string with no numbers#@!#$%%^&*";
            assert_eq!(str_num.extract_number().is_err(), true);
        }
        #[test]
        fn select_name() {
            let fragment = Html::parse_fragment(GIVEAWAY_HTML);
            let name = super::SteamgiftsAcc::select_name(&fragment.root_element());
            assert_eq!(name, "Spandex Force: Champion Rising");
        }
        #[test]
        fn select_entries() {
            let fragment = Html::parse_fragment(GIVEAWAY_HTML);
            let entries = super::SteamgiftsAcc::select_entries(&fragment.root_element());
            assert_eq!(entries.unwrap(), 237u32);
        }
        #[test]
        fn select_href() {
            let fragment = Html::parse_fragment(GIVEAWAY_HTML);
            let href = super::SteamgiftsAcc::select_href(&fragment.root_element());
            assert_eq!(href, "/giveaway/IFesR/spandex-force-champion-rising");
        }
        #[test]
        fn select_points() {
            let fragment = Html::parse_fragment(GIVEAWAY_HTML);
            let points = super::SteamgiftsAcc::select_points(&fragment.root_element());
            assert_eq!(points.unwrap(), 3u32);
        }
        #[test]
        fn select_copies() {
            let fragment = Html::parse_fragment(GIVEAWAY_HTML);
            let copies = super::SteamgiftsAcc::select_copies(&fragment.root_element());
            assert_eq!(copies.unwrap(), 1u32);
        }
    }
}
