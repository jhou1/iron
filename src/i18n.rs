use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use unic_langid::LanguageIdentifier;
use std::cell::RefCell;

const EN_FTL: &str = include_str!("../locales/en.ftl");
const ZH_FTL: &str = include_str!("../locales/zh-CN.ftl");

struct I18nBundle {
    bundle: FluentBundle<FluentResource>,
}

thread_local! {
    static BUNDLE: RefCell<Option<I18nBundle>> = const { RefCell::new(None) };
}

pub fn init() {
    let lang = detect_locale();
    let ftl_src = if lang.starts_with("zh") { ZH_FTL } else { EN_FTL };
    let langid: LanguageIdentifier = if lang.starts_with("zh") {
        "zh-CN".parse().unwrap()
    } else {
        "en".parse().unwrap()
    };

    let resource = FluentResource::try_new(ftl_src.to_string())
        .expect("Failed to parse FTL");
    let mut bundle = FluentBundle::new(vec![langid]);
    bundle.set_use_isolating(false);
    bundle.add_resource(resource)
        .expect("Failed to add FTL resource");

    BUNDLE.with(|b| {
        *b.borrow_mut() = Some(I18nBundle { bundle });
    });
}

fn detect_locale() -> String {
    std::env::var("LC_ALL")
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_else(|_| "en".to_string())
}

pub fn tr(key: &str) -> String {
    BUNDLE.with(|b| {
        // Auto-initialize if not yet initialized for this thread
        if b.borrow().is_none() {
            drop(b.borrow());
            init();
        }
        let borrow = b.borrow();
        let i18n = match borrow.as_ref() {
            Some(i) => i,
            None => return key.to_string(),
        };
        let msg = match i18n.bundle.get_message(key) {
            Some(m) => m,
            None => return key.to_string(),
        };
        let pattern = match msg.value() {
            Some(p) => p,
            None => return key.to_string(),
        };
        let mut errors = vec![];
        i18n.bundle.format_pattern(pattern, None, &mut errors).to_string()
    })
}

pub fn tr_args(key: &str, args: &[(&str, FluentValue)]) -> String {
    BUNDLE.with(|b| {
        // Auto-initialize if not yet initialized for this thread
        if b.borrow().is_none() {
            drop(b.borrow());
            init();
        }
        let borrow = b.borrow();
        let i18n = match borrow.as_ref() {
            Some(i) => i,
            None => return key.to_string(),
        };
        let msg = match i18n.bundle.get_message(key) {
            Some(m) => m,
            None => return key.to_string(),
        };
        let pattern = match msg.value() {
            Some(p) => p,
            None => return key.to_string(),
        };
        let mut fluent_args = FluentArgs::new();
        for (k, v) in args {
            fluent_args.set(*k, v.clone());
        }
        let mut errors = vec![];
        i18n.bundle.format_pattern(pattern, Some(&fluent_args), &mut errors).to_string()
    })
}
