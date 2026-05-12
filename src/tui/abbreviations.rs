use crossterm::event::KeyEvent;
use ratatui::Frame;
use crate::db::Database;
use super::{Action, Screen};

pub struct AbbreviationsScreen;

impl AbbreviationsScreen {
    pub fn new(_db: &Database) -> anyhow::Result<Self> {
        Ok(Self)
    }

    pub fn render(&self, _frame: &mut Frame) {}

    pub fn handle_key(&mut self, _key: KeyEvent, _db: &Database) -> Action {
        Action::None
    }
}
