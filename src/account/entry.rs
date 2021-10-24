use crate::account::URL;
use std::{fmt::Display, time::Duration};

#[derive(Debug)]
pub struct Entry<'url> {
    pub name: String,
    pub href: URL<'url>,
    pub price: u32,
    pub copies: u32,
    pub entries: u32,
    pub ends_in: Duration,
}

impl<'url> Entry<'url> {
    pub fn get_code(&self) -> String {
        self.href.to_string()[36..41].to_string()
    }
}

impl<'url> Display for Entry<'url> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // Use `self.number` to refer to each positional data point.
        write!(
            f,
            "[{:>40}] - Price: {:3} Chance: {:.3}% Ends in: {}",
            self.name,
            self.price,
            self.copies as f64 / self.entries as f64 * 100f64,
            humantime::format_duration(self.ends_in),
        )
    }
}
