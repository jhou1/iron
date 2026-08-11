use crossterm::event::KeyEvent;
use ratatui::Frame;

use crate::db::Database;
use crate::tui::{
    dashboard::DashboardScreen,
    goals::GoalsScreen,
    history::HistoryScreen,
    lifecycle::{NavigationStatus, Reenter},
    log_entry::LogEntryScreen,
    practices::PracticesScreen,
    quotes_screen::QuotesScreen,
    trends::TrendsScreen,
    Action, ModuleId, NavigationIntent, ScreenId,
};

pub(crate) struct ScreenStore {
    dashboard: DashboardScreen,
    goals: GoalsScreen,
    log_entry: LogEntryScreen,
    history: HistoryScreen,
    trends: TrendsScreen,
    practices: PracticesScreen,
    quotes: QuotesScreen,
}

pub(crate) enum PreparedScreen {
    Dashboard(DashboardScreen),
    Goals(GoalsScreen),
    LogEntry(LogEntryScreen),
    History(HistoryScreen),
    Trends(TrendsScreen),
    Practices(PracticesScreen),
    Quotes(QuotesScreen),
}

impl PreparedScreen {
    pub(crate) fn target(&self) -> ScreenId {
        match self {
            Self::Dashboard(_) => ScreenId::Dashboard,
            Self::Goals(_) => ScreenId::Goals,
            Self::LogEntry(_) => ScreenId::LogEntry,
            Self::History(_) => ScreenId::History,
            Self::Trends(_) => ScreenId::Trends,
            Self::Practices(_) => ScreenId::Practices,
            Self::Quotes(_) => ScreenId::Quotes,
        }
    }
}

impl ScreenStore {
    pub(crate) fn new(db: &Database, no_color: bool) -> anyhow::Result<Self> {
        Ok(Self {
            dashboard: DashboardScreen::new(db, no_color)?,
            goals: GoalsScreen::new(db)?,
            log_entry: LogEntryScreen::new(db)?,
            history: HistoryScreen::new(db)?,
            trends: TrendsScreen::new(db)?,
            practices: PracticesScreen::new(db)?,
            quotes: QuotesScreen::new(db)?,
        })
    }

    pub(crate) fn render(&mut self, current: ScreenId, frame: &mut Frame) {
        match current {
            ScreenId::Dashboard => self.dashboard.render(frame),
            ScreenId::Goals => self.goals.render(frame),
            ScreenId::LogEntry => self.log_entry.render(frame),
            ScreenId::History => self.history.render(frame),
            ScreenId::Trends => self.trends.render(frame),
            ScreenId::Practices => self.practices.render(frame),
            ScreenId::Quotes => self.quotes.render(frame),
        }
    }

    pub(crate) fn handle_key(&mut self, current: ScreenId, key: KeyEvent, db: &Database) -> Action {
        match current {
            ScreenId::Dashboard => self.dashboard.handle_key(key, db),
            ScreenId::Goals => self.goals.handle_key(key, db),
            ScreenId::LogEntry => self.log_entry.handle_key(key, db),
            ScreenId::History => self.history.handle_key(key, db),
            ScreenId::Trends => self.trends.handle_key(key, db),
            ScreenId::Practices => self.practices.handle_key(key, db),
            ScreenId::Quotes => self.quotes.handle_key(key, db),
        }
    }

    pub(crate) fn prepare(
        &self,
        intent: NavigationIntent,
        db: &Database,
    ) -> anyhow::Result<PreparedScreen> {
        let prepared = match intent {
            NavigationIntent::Open(module) => match module {
                ModuleId::Dashboard => PreparedScreen::Dashboard(self.dashboard.reenter(db)?),
                ModuleId::Goals => PreparedScreen::Goals(self.goals.reenter(db)?),
                ModuleId::History => PreparedScreen::History(self.history.reenter(db)?),
                ModuleId::Trends => PreparedScreen::Trends(self.trends.reenter(db)?),
                ModuleId::Practices => PreparedScreen::Practices(self.practices.reenter(db)?),
                ModuleId::Quotes => PreparedScreen::Quotes(self.quotes.reenter(db)?),
            },
            NavigationIntent::NewLog => PreparedScreen::LogEntry(LogEntryScreen::new(db)?),
            NavigationIntent::EditHistoryLog { log_id } => {
                let entry = db
                    .get_log_by_id(log_id)?
                    .ok_or_else(|| anyhow::anyhow!("log {log_id} no longer exists"))?;
                PreparedScreen::LogEntry(LogEntryScreen::from_existing(db, &entry)?)
            }
        };
        debug_assert_eq!(prepared.target(), intent.target());
        Ok(prepared)
    }

    pub(crate) fn install(&mut self, prepared: PreparedScreen) {
        match prepared {
            PreparedScreen::Dashboard(screen) => self.dashboard = screen,
            PreparedScreen::Goals(screen) => self.goals = screen,
            PreparedScreen::LogEntry(screen) => self.log_entry = screen,
            PreparedScreen::History(screen) => self.history = screen,
            PreparedScreen::Trends(screen) => self.trends = screen,
            PreparedScreen::Practices(screen) => self.practices = screen,
            PreparedScreen::Quotes(screen) => self.quotes = screen,
        }
    }

    pub(crate) fn set_navigation_error(&mut self, source: ScreenId, message: String) {
        match source {
            ScreenId::Dashboard => self.dashboard.set_navigation_error(message),
            ScreenId::Goals => self.goals.set_navigation_error(message),
            ScreenId::LogEntry => self.log_entry.set_navigation_error(message),
            ScreenId::History => self.history.set_navigation_error(message),
            ScreenId::Trends => self.trends.set_navigation_error(message),
            ScreenId::Practices => self.practices.set_navigation_error(message),
            ScreenId::Quotes => self.quotes.set_navigation_error(message),
        }
    }
}

#[cfg(test)]
impl PreparedScreen {
    pub(crate) fn log_entry_for_test(&self) -> &LogEntryScreen {
        match self {
            Self::LogEntry(screen) => screen,
            _ => panic!("prepared target is not Log Entry"),
        }
    }
}

#[cfg(test)]
#[allow(dead_code)]
impl ScreenStore {
    pub(crate) fn log_entry_for_test(&self) -> &LogEntryScreen {
        &self.log_entry
    }

    pub(crate) fn practices_identity_for_test(&self) -> Vec<i64> {
        self.practices.ids_for_test()
    }

    pub(crate) fn goals_titles_for_test(&self) -> Vec<String> {
        self.goals.titles_for_test()
    }

    pub(crate) fn goals_context_for_test(&self) -> (Option<i64>, usize, bool) {
        self.goals.context_for_test()
    }

    pub(crate) fn log_entry_context_for_test(&self) -> ((Option<i64>, ModuleId), (String, String)) {
        let identity = self.log_entry.identity_for_test();
        let (note, rpe) = self.log_entry.form_value_for_test();
        (identity, (note.to_owned(), rpe.to_owned()))
    }

    pub(crate) fn history_context_for_test(&self) -> (Option<i64>, Option<String>, String) {
        self.history.context_for_test()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{PracticeType, SetData};

    #[test]
    fn edit_intent_prepares_exact_log_id_without_history_selection_access() {
        let db = Database::open_in_memory().unwrap();
        let practice = db
            .create_practice("Pull-up", PracticeType::Bodyweight)
            .unwrap();
        let log_id = db
            .create_log(
                practice.id,
                &[SetData::Bodyweight { reps: 10 }],
                Some("target"),
                None,
                None,
                Some(7),
            )
            .unwrap();
        let store = ScreenStore::new(&db, false).unwrap();

        let prepared = store
            .prepare(NavigationIntent::EditHistoryLog { log_id }, &db)
            .unwrap();

        assert_eq!(prepared.target(), ScreenId::LogEntry);
        assert_eq!(
            prepared.log_entry_for_test().identity_for_test().0,
            Some(log_id)
        );
    }

    #[test]
    fn missing_edit_target_does_not_replace_cached_log_entry() {
        let db = Database::open_in_memory().unwrap();
        let store = ScreenStore::new(&db, false).unwrap();
        let before = store.log_entry_for_test().identity_for_test();

        assert!(store
            .prepare(NavigationIntent::EditHistoryLog { log_id: i64::MAX }, &db)
            .is_err());
        assert_eq!(store.log_entry_for_test().identity_for_test(), before);
    }
}
