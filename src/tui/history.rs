use ratatui::Frame;

use crate::db::Database;
use super::Action;

pub struct HistoryScreen;

impl HistoryScreen {
    pub fn new(_db: &Database) -> anyhow::Result<Self> {
        Ok(Self)
    }

    pub fn render(&self, _frame: &mut Frame) {}

    pub fn handle_key(&mut self, _key: crossterm::event::KeyEvent, _db: &Database) -> Action {
        Action::None
    }
}
