#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleId {
    Dashboard,
    History,
    Trends,
    Practices,
    Goals,
    Quotes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenId {
    Dashboard,
    LogEntry,
    History,
    Trends,
    Practices,
    Goals,
    Quotes,
}

impl From<ModuleId> for ScreenId {
    fn from(module: ModuleId) -> Self {
        match module {
            ModuleId::Dashboard => Self::Dashboard,
            ModuleId::History => Self::History,
            ModuleId::Trends => Self::Trends,
            ModuleId::Practices => Self::Practices,
            ModuleId::Goals => Self::Goals,
            ModuleId::Quotes => Self::Quotes,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationIntent {
    Open(ModuleId),
    NewLog,
    EditHistoryLog { log_id: i64 },
}

impl NavigationIntent {
    pub fn target(self) -> ScreenId {
        match self {
            Self::Open(module) => module.into(),
            Self::NewLog | Self::EditHistoryLog { .. } => ScreenId::LogEntry,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    None,
    Navigate(NavigationIntent),
    Quit,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigation_intents_have_exact_targets() {
        assert_eq!(
            NavigationIntent::Open(ModuleId::Goals).target(),
            ScreenId::Goals
        );
        assert_eq!(NavigationIntent::NewLog.target(), ScreenId::LogEntry);
        assert_eq!(
            NavigationIntent::EditHistoryLog { log_id: 42 }.target(),
            ScreenId::LogEntry
        );
    }

    #[test]
    fn module_ids_exclude_request_scoped_log_entry() {
        let modules = [
            ModuleId::Dashboard,
            ModuleId::History,
            ModuleId::Trends,
            ModuleId::Practices,
            ModuleId::Goals,
            ModuleId::Quotes,
        ];
        assert!(modules
            .into_iter()
            .all(|module| ScreenId::from(module) != ScreenId::LogEntry));
    }
}
