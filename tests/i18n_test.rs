use std::collections::HashSet;

#[test]
fn tr_returns_english_by_default() {
    std::env::set_var("LANG", "en_US.UTF-8");
    std::env::remove_var("LC_ALL");
    ironcli::i18n::init();

    let result = ironcli::i18n::tr("dashboard-goals");
    assert_eq!(result, "Goals");
}

#[test]
fn tr_returns_chinese_when_locale_is_zh() {
    std::env::set_var("LANG", "zh_CN.UTF-8");
    std::env::remove_var("LC_ALL");
    ironcli::i18n::init();

    let result = ironcli::i18n::tr("dashboard-goals");
    assert_eq!(result, "目标");
}

#[test]
fn tr_args_interpolates_values() {
    std::env::set_var("LANG", "en_US.UTF-8");
    std::env::remove_var("LC_ALL");
    ironcli::i18n::init();

    let result = ironcli::i18n::tr_args("dashboard-sessions", &[("count", 5.0.into())]);
    assert_eq!(result, "5 sessions");
}

#[test]
fn tr_args_interpolates_chinese() {
    std::env::set_var("LANG", "zh_CN.UTF-8");
    std::env::remove_var("LC_ALL");
    ironcli::i18n::init();

    let result = ironcli::i18n::tr_args("dashboard-sessions", &[("count", 5.0.into())]);
    assert_eq!(result, "5 次训练");
}

#[test]
fn tr_fallback_for_missing_key() {
    std::env::set_var("LANG", "en_US.UTF-8");
    std::env::remove_var("LC_ALL");
    ironcli::i18n::init();

    let result = ironcli::i18n::tr("nonexistent-key");
    assert_eq!(result, "nonexistent-key");
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
    use fluent_syntax::parser;
    use fluent_syntax::ast::Entry;

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
