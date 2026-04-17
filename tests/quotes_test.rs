use ironcli::model::Quote;
use ironcli::tui::quotes::pick_daily_quote;

fn make_quotes(texts: &[&str]) -> Vec<Quote> {
    texts
        .iter()
        .enumerate()
        .map(|(i, t)| Quote {
            id: i as i64 + 1,
            text: t.to_string(),
            position: i as i32,
        })
        .collect()
}

#[test]
fn empty_slice_returns_empty_string() {
    assert_eq!(pick_daily_quote(&[]), "");
}

#[test]
fn single_quote_always_returned() {
    let quotes = make_quotes(&["Only quote"]);
    assert_eq!(pick_daily_quote(&quotes), "Only quote");
}

#[test]
fn selection_wraps_around() {
    let quotes = make_quotes(&["A", "B", "C"]);
    // pick_daily_quote uses day_of_year % len, result is always one of A/B/C
    let result = pick_daily_quote(&quotes);
    assert!(["A", "B", "C"].contains(&result.as_str()));
}
