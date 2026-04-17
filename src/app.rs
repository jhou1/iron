use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::prelude::*;
use std::io::stdout;

use crate::db::Database;
use crate::tui::{
    dashboard::DashboardScreen,
    history::HistoryScreen,
    log_entry::LogEntryScreen,
    practices::PracticesScreen,
    trends::TrendsScreen,
    Action, Screen,
};

pub fn run() -> Result<()> {
    let db = Database::open_default()?;
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    let result = run_app(&mut terminal, &db);
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    result
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    db: &Database,
) -> Result<()> {
    let mut current_screen = Screen::Dashboard;
    let mut dashboard = DashboardScreen::new(db)?;
    let mut log_entry = LogEntryScreen::new(db)?;
    let mut history = HistoryScreen::new(db)?;
    let mut trends = TrendsScreen::new(db)?;
    let mut practices = PracticesScreen::new(db)?;

    loop {
        terminal.draw(|frame| match current_screen {
            Screen::Dashboard => dashboard.render(frame),
            Screen::LogEntry => log_entry.render(frame),
            Screen::History => history.render(frame),
            Screen::Trends => trends.render(frame),
            Screen::Practices => practices.render(frame),
        })?;

        if let Event::Key(key) = event::read()? {
            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                break;
            }

            let action = match current_screen {
                Screen::Dashboard => dashboard.handle_key(key, db),
                Screen::LogEntry => log_entry.handle_key(key, db),
                Screen::History => history.handle_key(key, db),
                Screen::Trends => {
                    let action = trends.handle_key(key);
                    trends.refresh_chart(db);
                    action
                }
                Screen::Practices => practices.handle_key(key, db),
            };

            match action {
                Action::Quit => break,
                Action::Navigate(screen) => {
                    match &screen {
                        Screen::Dashboard => dashboard.refresh(db)?,
                        Screen::LogEntry => {
                            if current_screen == Screen::History {
                                if let Some(entry) = history.selected_entry() {
                                    log_entry = LogEntryScreen::from_existing(db, entry)?;
                                } else {
                                    log_entry = LogEntryScreen::new(db)?;
                                }
                            } else {
                                log_entry = LogEntryScreen::new(db)?;
                            }
                        }
                        Screen::History => history = HistoryScreen::new(db)?,
                        Screen::Trends => trends = TrendsScreen::new(db)?,
                        Screen::Practices => practices = PracticesScreen::new(db)?,
                    }
                    current_screen = screen;
                }
                Action::None => {}
            }
        }
    }

    Ok(())
}
