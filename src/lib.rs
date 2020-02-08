extern crate simple_error;
extern crate ureq;

pub mod steamgifts_acc {
    pub mod entry;
    use entry::Entry;
    use scraper::html::Html;
    use scraper::Selector;
    //use serde::{Deserialize, Serialize};
    //use serde_json::Result;
    use std::error::Error;

    #[derive(Debug)]
    pub enum URL {
        Main,
        Href(String),
        Post,
        Custom(String),
    }
    impl URL {
        // pub fn as_str(&self) -> &str{
        //     match self {
        //         URL::Main => "https://www.steamgifts.com",
        //         URL::Href(x) => format!("{}{}", URL::Main.to_string(), x),
        //         URL::Custom(x) => x.as_str(),
        //         URL::Post => "https://www.steamgifts.com/ajax.php",
        //     }
        // }
        pub fn to_string(&self) -> String {
            match self {
                URL::Main => "https://www.steamgifts.com/".to_string(),
                URL::Href(x) => format!("{}{}", "https://www.steamgifts.com", x),
                URL::Custom(x) => x.clone(),
                URL::Post => "https://www.steamgifts.com/ajax.php".to_string(),
            }
        }
    }
    pub fn new(cookie: String) -> Result<SteamgiftsAcc, Box<dyn Error>> {
        let xsrf = "".to_string(); //get_xsrf(ureq::get(URL::default().to_string().as_str()).call());
        let mut acc = SteamgiftsAcc { cookie, xsrf };
        // TODO: Error handling
        acc.xsrf = acc.get_xsrf()?;
        Ok(acc)
    }
    pub struct SteamgiftsAcc {
        cookie: String,
        xsrf: String,
    }
    // * Public decls
    impl SteamgiftsAcc {
        /// Panics:
        /// * POST response != 200
        /// *  
        ///
        pub fn enter_giveaway(&self, ga: &Entry) -> Result<u32, Box<dyn Error>> {
            //{"msg": String("Previously Won"), "points": String("397"), "type": String("error")}
            //{"type":"success","entry_count":"1,371","points":"138"}
            let response = self.post(ga);
            if response.status() != 200 {
                panic!(format!(
                    "POST response code != 200, got {}.\nDebug: {:?}",
                    response.status(),
                    response
                ));
            }
            let json = response
                .into_json()
                .expect("Error when trying parse JSON from response");
            let msg_type = json
                .get("type")
                .expect("Failed to get 'type' field from JSON")
                .as_str()
                .unwrap();
            let msg = match msg_type {
                "success" => None,
                "error" => Some(
                    json.get("msg")
                        .expect("json cannot find 'msg'")
                        .as_str()
                        .unwrap(),
                ),
                &_ => panic!("unknow message in JSON response"),
            };
            let points = json
                .get("points")
                .expect("Failed to get 'points' field from JSON")
                .as_str()
                .unwrap();
            if msg_type == "error" {
                println!("Error msg: {}", msg.unwrap(),);
            }
            Ok(points.parse::<u32>().unwrap())
        }
        pub fn get_points(&self) -> u32 {
            let html = self.get(URL::Main).into_string().unwrap();
            // TODO: Error handling
            let doc = scraper::html::Html::parse_document(html.as_str());
            let points_balance_selector = Selector::parse("span.nav__points").unwrap();
            let points = doc
                .select(&points_balance_selector)
                .nth(0)
                .expect("Cannot parse balance")
                .inner_html();
            points.as_str().extract_number()
        }
        pub fn parse_vector(&self) -> Result<Vec<Entry>, Box<dyn Error>> {
            let html = self.get(URL::Main).into_string()?;
            // TODO: Error handling
            let doc: Html = Html::parse_document(html.as_str());
            let giveaway_selector = Selector::parse(
                "div.giveaway__row-outer-wrap[data-game-id] div[class=giveaway__row-inner-wrap]",
            )
            .unwrap();
            //Selector::parse("div.giveaway__summary").unwrap();
            let giveaways = doc
                .select(&giveaway_selector)
                .filter_map(|el| {
                    let name = SteamgiftsAcc::select_name(&el);
                    let points = SteamgiftsAcc::select_points(&el);
                    let copies = SteamgiftsAcc::select_copies(&el);
                    let entries = SteamgiftsAcc::select_entries(&el);
                    let href = SteamgiftsAcc::select_href(&el);
                    Some(Entry::new(name, URL::Href(href), points, copies, entries))
                })
                .collect();
            Ok(giveaways)
        }
    }
    // * End Public decls
    // * Private decls
    impl SteamgiftsAcc {
        fn select_name(el: &scraper::ElementRef) -> String {
            let name_selector = Selector::parse("a.giveaway__heading__name").unwrap();
            let name = el.select(&name_selector).nth(0).unwrap().inner_html();
            name
        }
        fn select_entries(el: &scraper::ElementRef) -> u32 {
            let entries_selector = Selector::parse("div.giveaway__links a[href] span").unwrap();
            let entries = el
                .select(&entries_selector)
                .nth(0)
                .unwrap()
                .inner_html()
                .as_str()
                .extract_number();
            entries
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
        fn select_points(el: &scraper::ElementRef) -> u32 {
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
        fn select_copies<'a>(el: &'a scraper::ElementRef) -> u32 {
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
                1
            }
        }
        fn get_xsrf(&self) -> Result<String, Box<dyn std::error::Error>> {
            let doc = self.get(URL::Main).into_string()?;
            let doc: scraper::html::Html = scraper::html::Html::parse_document(doc.as_str());
            let selector = Selector::parse("input[name=\"xsrf_token\"]").unwrap();
            let error_msg = || {
                simple_error::SimpleError::new(format!(
                    "cannot login, is '{}' cookie right?",
                    self.cookie
                ))
            };
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
        fn get(&self, url: URL) -> ureq::Response {
            let url = url.to_string();
            let resp = ureq::get(url.as_str())
                .timeout_connect(30_000)
                .set(
                    "Accept",
                    "text/html, application/xhtml+xml, application/xml",
                )
                //.set("Connection", "close")
                .set("Cookie", format!("PHPSESSID={}", &self.cookie).as_str())
                // .set("DNT", "1")
                .set(
                    "User-Agent",
                    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:73.0) Gecko/20100101 Firefox/73.0",
                )
                .set("Host", "www.steamgifts.com")
                .set("Referer", "https://www.steamgifts.com")
                .set("TE", "Trailers")
                .set("Upgrade-Insecure-Requests", "1")
                .call();
            resp
        }
        fn post(&self, entry: &Entry) -> ureq::Response {
            let cookie = format!("PHPSESSID={}", &self.cookie);
            let referer = entry.get_href().to_string();
            let post_data = format!(
                "xsrf_token={}&do=entry_insert&code={}",
                self.xsrf,
                entry.get_code()
            );
            let resp = ureq::post(URL::Post.to_string().as_str())
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
                .timeout_connect(30_000)
                .send_string(post_data.as_str());
            resp
        }
    }
    // * End Private decls
    trait MyTrait {
        fn extract_number(&self) -> u32;
    }
    impl MyTrait for &str {
        fn extract_number(&self) -> u32 {
            let mut out = String::new();
            for c in self.chars() {
                if c.is_numeric() {
                    out.push(c);
                }
            }
            if out.is_empty() {
                panic!("trying to get number from string which doesn't contain numbers")
            }
            out.parse()
                .expect(format!("Extract failed, got: {}", self).as_str())
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
            assert_eq!(str_num.extract_number(), 123456);
        }
        #[test]
        #[should_panic]
        fn extract_number_no_numbers() {
            let str_num = "string with no numbers#@!#$%%^&*";
            str_num.extract_number();
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
            assert_eq!(entries, 237u32);
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
            assert_eq!(points, 3u32);
        }
        #[test]
        fn select_copies() {
            let fragment = Html::parse_fragment(GIVEAWAY_HTML);
            let copies = super::SteamgiftsAcc::select_copies(&fragment.root_element());
            assert_eq!(copies, 1u32);
        }
    }
}
