use rand::seq::IndexedRandom;
use crate::model::Quote;

pub fn pick_random_quote(quotes: &[Quote]) -> String {
    if quotes.is_empty() {
        return String::new();
    }
    quotes
        .choose(&mut rand::rng())
        .map(|q| q.text.clone())
        .unwrap_or_default()
}
