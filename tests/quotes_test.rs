use iron::model::Quote;
use iron::tui::quotes::pick_random_quote;

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
    assert_eq!(pick_random_quote(&[]), "");
}

#[test]
fn single_quote_always_returned() {
    let quotes = make_quotes(&["Only quote"]);
    assert_eq!(pick_random_quote(&quotes), "Only quote");
}

#[test]
fn selection_wraps_around() {
    let quotes = make_quotes(&["A", "B", "C"]);
    let result = pick_random_quote(&quotes);
    assert!(["A", "B", "C"].contains(&result.as_str()));
}
