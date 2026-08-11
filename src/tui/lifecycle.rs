use crate::db::Database;

// ScreenStore uses this contract to prepare refreshed ordinary-screen replacements.
#[allow(dead_code)]
pub(crate) trait Reenter: Sized {
    fn reenter(&self, db: &Database) -> anyhow::Result<Self>;
}

// ScreenStore uses this contract when AppShell reports a Router transition failure.
#[allow(dead_code)]
pub(crate) trait NavigationStatus {
    fn set_navigation_error(&mut self, message: String);
}
