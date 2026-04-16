pub mod dashboard;
pub mod history;
pub mod log_entry;
pub mod practices;
pub mod trends;
pub mod widgets;

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    Dashboard,
    LogEntry,
    History,
    Trends,
    Practices,
}

pub enum Action {
    None,
    Navigate(Screen),
    Quit,
}
