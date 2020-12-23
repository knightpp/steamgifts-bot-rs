use crate::steamgifts_acc::URL;
use std::fmt::Display;

#[derive(Debug)]
pub struct Entry<'url> {
    name: String,
    href: URL<'url>,
    price: u32,
    copies: u32,
    entries: u32,
}

impl<'url> Entry<'url> {
    pub fn new(name: String, href: URL, price: u32, copies: u32, entries: u32) -> Entry {
        Entry {
            name,
            href,
            price,
            copies,
            entries,
        }
    }

    pub fn get_code(&self) -> String {
        self.href.to_string()[36..41].to_string()
    }
    pub fn get_href(&self) -> &URL {
        &self.href
    }
    pub fn get_price(&self) -> u32 {
        self.price
    }
}

impl<'url> Display for Entry<'url> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // Use `self.number` to refer to each positional data point.
        write!(
            f,
            "[{:>40}] - Price: {:3} Chance: {:.3}%",
            self.name,
            self.price,
            self.copies as f64 / self.entries as f64 * 100f64
        )
    }
}
