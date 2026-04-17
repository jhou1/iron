use std::io::Write;

#[test]
fn builtin_quotes_not_empty() {
    let quotes = ironcli_quotes::builtin_quotes();
    assert!(!quotes.is_empty());
    assert!(quotes.len() >= 20);
}

#[test]
fn daily_quote_deterministic() {
    let quotes = vec![
        "Quote A".to_string(),
        "Quote B".to_string(),
        "Quote C".to_string(),
    ];
    let q1 = ironcli_quotes::select_quote(&quotes, 0);
    let q2 = ironcli_quotes::select_quote(&quotes, 0);
    assert_eq!(q1, q2);
}

#[test]
fn daily_quote_rotates() {
    let quotes = vec![
        "Quote A".to_string(),
        "Quote B".to_string(),
        "Quote C".to_string(),
    ];
    let q0 = ironcli_quotes::select_quote(&quotes, 0);
    let q1 = ironcli_quotes::select_quote(&quotes, 1);
    assert_ne!(q0, q1);
}

#[test]
fn daily_quote_wraps_around() {
    let quotes = vec![
        "Quote A".to_string(),
        "Quote B".to_string(),
        "Quote C".to_string(),
    ];
    let q0 = ironcli_quotes::select_quote(&quotes, 0);
    let q3 = ironcli_quotes::select_quote(&quotes, 3);
    assert_eq!(q0, q3);
}

#[test]
fn load_quotes_from_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("quotes.txt");
    let mut f = std::fs::File::create(&path).unwrap();
    writeln!(f, "Custom quote 1").unwrap();
    writeln!(f, "").unwrap();
    writeln!(f, "Custom quote 2").unwrap();

    let quotes = ironcli_quotes::load_quotes_file(&path);
    assert_eq!(quotes, Some(vec![
        "Custom quote 1".to_string(),
        "Custom quote 2".to_string(),
    ]));
}

#[test]
fn load_quotes_from_missing_file() {
    let path = std::path::Path::new("/tmp/nonexistent_ironcli_quotes.txt");
    let quotes = ironcli_quotes::load_quotes_file(path);
    assert_eq!(quotes, None);
}

mod ironcli_quotes {
    use std::path::Path;

    pub fn builtin_quotes() -> Vec<String> {
        ironcli::tui::quotes::builtin_quotes()
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    pub fn select_quote(quotes: &[String], day_of_year: u32) -> String {
        ironcli::tui::quotes::select_quote(
            &quotes.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
            day_of_year,
        ).to_string()
    }

    pub fn load_quotes_file(path: &Path) -> Option<Vec<String>> {
        ironcli::tui::quotes::load_quotes_file(path)
    }
}
