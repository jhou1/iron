use crate::db::Database;
use crate::tui::{NavigationIntent, ScreenId};

use super::screens::ScreenStore;

#[derive(Debug, Default)]
pub(crate) struct Router;

impl Router {
    pub(crate) fn transition(
        &self,
        current: &mut ScreenId,
        screens: &mut ScreenStore,
        intent: NavigationIntent,
        db: &Database,
    ) -> anyhow::Result<()> {
        let prepared = screens.prepare(intent, db)?;
        let target = prepared.target();
        screens.install(prepared);
        *current = target;
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::default_constructed_unit_structs)]
mod tests {
    use super::*;
    use crate::tui::ModuleId;

    #[test]
    fn successful_transition_installs_target_before_route_commit() {
        let db = Database::open_in_memory().unwrap();
        let mut store = ScreenStore::new(&db, false).unwrap();
        db.create_goal("fresh goal").unwrap();
        let mut current = ScreenId::Dashboard;

        Router::default()
            .transition(
                &mut current,
                &mut store,
                NavigationIntent::Open(ModuleId::Goals),
                &db,
            )
            .unwrap();

        assert_eq!(current, ScreenId::Goals);
        assert!(store
            .goals_titles_for_test()
            .contains(&"fresh goal".to_string()));
    }

    #[test]
    fn failed_transition_does_not_commit_route_or_target() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        let db = Database::open(temp.path()).unwrap();
        let mut store = ScreenStore::new(&db, false).unwrap();
        let before = store.practices_identity_for_test();
        rusqlite::Connection::open(temp.path())
            .unwrap()
            .execute("DROP TABLE practices", [])
            .unwrap();
        let mut current = ScreenId::Dashboard;

        assert!(Router::default()
            .transition(
                &mut current,
                &mut store,
                NavigationIntent::Open(ModuleId::Practices),
                &db,
            )
            .is_err());

        assert_eq!(current, ScreenId::Dashboard);
        assert_eq!(store.practices_identity_for_test(), before);
    }
}
