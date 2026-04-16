use ratatui::Frame;

use crate::db::Database;
use super::Action;

pub struct DashboardScreen;

impl DashboardScreen {
    pub fn new(_db: &Database) -> anyhow::Result<Self> {
        Ok(Self)
    }

    pub fn refresh(&mut self, _db: &Database) -> anyhow::Result<()> {
        Ok(())
    }

    pub fn render(&self, _frame: &mut Frame) {}

    pub fn handle_key(&mut self, _key: crossterm::event::KeyEvent) -> Action {
        Action::None
    }
}
