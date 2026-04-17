use chrono::Datelike;
use std::path::Path;

pub fn builtin_quotes() -> &'static [&'static str] {
    &[
        "The only bad workout is the one that didn't happen.",
        "Discipline is choosing between what you want now and what you want most.",
        "The pain you feel today will be the strength you feel tomorrow.",
        "Don't count the days, make the days count.",
        "Success isn't always about greatness. It's about consistency.",
        "The body achieves what the mind believes.",
        "Fall seven times, stand up eight.",
        "You don't have to be extreme, just consistent.",
        "Strength does not come from the body. It comes from the will.",
        "The hard days are what make you stronger.",
        "What seems impossible today will one day become your warm-up.",
        "Your body can stand almost anything. It's your mind that you have to convince.",
        "The clock is ticking. Are you becoming the person you want to be?",
        "No one is you, and that is your superpower.",
        "You are only one workout away from a good mood.",
        "It never gets easier. You just get stronger.",
        "Motivation is what gets you started. Habit is what keeps you going.",
        "The secret of getting ahead is getting started.",
        "Strive for progress, not perfection.",
        "A year from now, you'll wish you had started today.",
        "The difference between try and triumph is a little umph.",
        "Sweat is fat crying.",
        "Be stronger than your strongest excuse.",
        "If it doesn't challenge you, it doesn't change you.",
        "The only limit is the one you set yourself.",
        "Champions are made when nobody is watching.",
        "Push yourself because no one else is going to do it for you.",
        "Great things never come from comfort zones.",
        "Wake up with determination. Go to bed with satisfaction.",
        "Today I will do what others won't, so tomorrow I can do what others can't.",
    ]
}

pub fn select_quote<'a>(quotes: &'a [&'a str], day_of_year: u32) -> &'a str {
    if quotes.is_empty() {
        return "";
    }
    quotes[(day_of_year as usize) % quotes.len()]
}

pub fn load_quotes_file(path: &Path) -> Option<Vec<String>> {
    let content = std::fs::read_to_string(path).ok()?;
    let lines: Vec<String> = content
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();
    if lines.is_empty() {
        None
    } else {
        Some(lines)
    }
}

pub fn get_daily_quote() -> String {
    let day_of_year = chrono::Local::now().ordinal();

    let quotes_path = dirs::home_dir()
        .map(|h| h.join(".ironcli").join("quotes.txt"));

    if let Some(ref path) = quotes_path {
        if let Some(custom) = load_quotes_file(path) {
            let refs: Vec<&str> = custom.iter().map(|s| s.as_str()).collect();
            return select_quote(&refs, day_of_year).to_string();
        }
    }

    let builtins = builtin_quotes();
    select_quote(builtins, day_of_year).to_string()
}
