use ratatui::Frame;

use crate::db::Database;
use super::Action;

pub struct TrendsScreen;

impl TrendsScreen {
    pub fn new(_db: &Database) -> anyhow::Result<Self> {
        Ok(Self)
    }

    pub fn render(&self, _frame: &mut Frame) {}

    pub fn handle_key(&mut self, _key: crossterm::event::KeyEvent) -> Action {
        Action::None
    }
}
