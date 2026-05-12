use crossterm::event::KeyEvent;
use ratatui::Frame;
use crate::config::LlmConfig;
use crate::db::Database;
use super::{Action, Screen};

pub struct QuickLogScreen;

impl QuickLogScreen {
    pub fn new(_db: &Database, _config: &Option<LlmConfig>) -> anyhow::Result<Self> {
        Ok(Self)
    }

    pub fn render(&self, _frame: &mut Frame) {}

    pub fn handle_key(&mut self, _key: KeyEvent, _db: &Database) -> Action {
        Action::None
    }

    pub fn check_background_result(&mut self) {}
}
