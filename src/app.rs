use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use fluent_bundle::FluentValue;
use ratatui::prelude::*;
use ratatui::widgets::Paragraph;
use std::io::stdout;
use std::time::Duration;

use crate::config::Config;
use crate::db::Database;
use crate::i18n::tr_args;
use crate::tui::{
    abbreviations::AbbreviationsScreen,
    dashboard::DashboardScreen,
    goals::GoalsScreen,
    history::HistoryScreen,
    log_entry::LogEntryScreen,
    practices::PracticesScreen,
    quick_log::QuickLogScreen,
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
    let no_color = std::env::var("NO_COLOR").is_ok();
    let mut current_screen = Screen::Dashboard;
    let mut dashboard = DashboardScreen::new(db, no_color)?;
    let mut goals_screen = GoalsScreen::new(db)?;
    let mut log_entry = LogEntryScreen::new(db)?;
    let mut history = HistoryScreen::new(db)?;
    let mut trends = TrendsScreen::new(db)?;
    let mut practices = PracticesScreen::new(db)?;
    let config = Config::load();
    let mut quick_log = QuickLogScreen::new(db, &config.llm)?;
    let mut abbreviations = AbbreviationsScreen::new(db)?;

    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            if area.width < 80 || area.height < 24 {
                let msg = tr_args("terminal-too-small", &[
                    ("w", FluentValue::from(80)),
                    ("h", FluentValue::from(24)),
                ]);
                let paragraph = Paragraph::new(msg)
                    .alignment(Alignment::Center);
                let y = area.height / 2;
                let msg_area = Rect::new(area.x, y, area.width, 1);
                frame.render_widget(paragraph, msg_area);
                return;
            }
            match current_screen {
                Screen::Dashboard => dashboard.render(frame),
                Screen::Goals => goals_screen.render(frame),
                Screen::LogEntry => log_entry.render(frame),
                Screen::History => history.render(frame),
                Screen::Trends => trends.render(frame),
                Screen::Practices => practices.render(frame),
                Screen::QuickLog => quick_log.render(frame),
                Screen::Abbreviations => abbreviations.render(frame),
            }
        })?;

        if !event::poll(Duration::from_millis(100))? {
            if let Screen::QuickLog = current_screen {
                quick_log.check_background_result();
            }
            continue;
        }
        if let Event::Key(key) = event::read()? {
            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                break;
            }

            let action = match current_screen {
                Screen::Dashboard => dashboard.handle_key(key, db),
                Screen::Goals => goals_screen.handle_key(key, db),
                Screen::LogEntry => log_entry.handle_key(key, db),
                Screen::History => history.handle_key(key, db),
                Screen::Trends => {
                    let action = trends.handle_key(key);
                    trends.refresh_chart(db);
                    action
                }
                Screen::Practices => practices.handle_key(key, db),
                Screen::QuickLog => quick_log.handle_key(key, db),
                Screen::Abbreviations => abbreviations.handle_key(key, db),
            };

            if let Screen::QuickLog = current_screen {
                quick_log.check_background_result();
            }

            match action {
                Action::Quit => break,
                Action::Navigate(screen) => {
                    match &screen {
                        Screen::Dashboard => dashboard.refresh(db)?,
                        Screen::Goals => goals_screen = GoalsScreen::new(db)?,
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
                        Screen::History => {
                            if current_screen != Screen::LogEntry {
                                history = HistoryScreen::new(db)?;
                            }
                        }
                        Screen::Trends => trends = TrendsScreen::new(db)?,
                        Screen::Practices => practices = PracticesScreen::new(db)?,
                        Screen::QuickLog => {
                            quick_log = QuickLogScreen::new(db, &config.llm)?;
                        }
                        Screen::Abbreviations => {
                            abbreviations = AbbreviationsScreen::new(db)?;
                        }
                    }
                    current_screen = screen;
                }
                Action::None => {}
            }
        }
    }

    Ok(())
}
