use crate::db::Database;

// Concrete app storage consumes this contract in the binary target.
#[allow(dead_code)]
pub(crate) trait Reenter: Sized {
    fn reenter(&self, db: &Database) -> anyhow::Result<Self>;
}

// The router consumes this contract in a later lifecycle migration task.
#[allow(dead_code)]
pub(crate) trait NavigationStatus {
    fn set_navigation_error(&mut self, message: String);
}
