use chrono::Datelike;
use crate::model::Quote;

pub fn pick_daily_quote(quotes: &[Quote]) -> String {
    if quotes.is_empty() {
        return String::new();
    }
    let day = chrono::Local::now().ordinal() as usize;
    quotes[day % quotes.len()].text.clone()
}
