use std::collections::HashSet;

// These tests mutate process-wide env vars (LANG/LC_ALL) and re-init the
// thread-local bundle, so they must not run in parallel with each other.
// We consolidate them into a single test to avoid races.
#[test]
fn tr_locale_switching() {
    // English
    std::env::set_var("LANG", "en_US.UTF-8");
    std::env::remove_var("LC_ALL");
    iron::i18n::init();

    assert_eq!(iron::i18n::tr("dashboard-goals"), "Goals");
    assert_eq!(
        iron::i18n::tr_args("dashboard-sessions", &[("count", 5.0.into())]),
        "5 sessions"
    );
    assert_eq!(iron::i18n::tr("nonexistent-key"), "nonexistent-key");

    // Chinese
    std::env::set_var("LANG", "zh_CN.UTF-8");
    std::env::remove_var("LC_ALL");
    iron::i18n::init();

    assert_eq!(iron::i18n::tr("dashboard-goals"), "目标");
    assert_eq!(
        iron::i18n::tr_args("dashboard-sessions", &[("count", 5.0.into())]),
        "5 次训练"
    );
}

#[test]
fn ftl_files_have_matching_keys() {
    let en_src = include_str!("../locales/en.ftl");
    let zh_src = include_str!("../locales/zh-CN.ftl");

    let en_keys = extract_message_keys(en_src);
    let zh_keys = extract_message_keys(zh_src);

    let missing_in_zh: Vec<&String> = en_keys.difference(&zh_keys).collect();
    let extra_in_zh: Vec<&String> = zh_keys.difference(&en_keys).collect();

    assert!(
        missing_in_zh.is_empty(),
        "Keys in en.ftl but missing from zh-CN.ftl: {:?}",
        missing_in_zh
    );
    assert!(
        extra_in_zh.is_empty(),
        "Keys in zh-CN.ftl but not in en.ftl: {:?}",
        extra_in_zh
    );
}

fn extract_message_keys(src: &str) -> HashSet<String> {
    use fluent_syntax::ast::Entry;
    use fluent_syntax::parser;

    let resource = parser::parse(src).expect("Failed to parse .ftl file");
    resource
        .body
        .iter()
        .filter_map(|entry| match entry {
            Entry::Message(msg) => Some(msg.id.name.to_string()),
            _ => None,
        })
        .collect()
}
