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

use crate::db::Database;
use crate::i18n::tr_args;
use crate::tui::{
    dashboard::DashboardScreen, goals::GoalsScreen, history::HistoryScreen,
    log_entry::LogEntryScreen, practices::PracticesScreen, quotes_screen::QuotesScreen,
    trends::TrendsScreen, Action, Screen,
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

fn refresh_history_for_navigation(db: &Database, history: &mut HistoryScreen) -> Result<()> {
    *history = HistoryScreen::new(db)?;
    Ok(())
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
    let mut quotes_screen = QuotesScreen::new(db)?;

    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            if area.width < 80 || area.height < 24 {
                let msg = tr_args(
                    "terminal-too-small",
                    &[("w", FluentValue::from(80)), ("h", FluentValue::from(24))],
                );
                let paragraph = Paragraph::new(msg).alignment(Alignment::Center);
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
                Screen::Quotes => quotes_screen.render(frame),
            }
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind != event::KeyEventKind::Press {
                continue;
            }
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
                Screen::Quotes => quotes_screen.handle_key(key, db),
            };

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
                            refresh_history_for_navigation(db, &mut history)?;
                        }
                        Screen::Trends => trends = TrendsScreen::new(db)?,
                        Screen::Practices => practices = PracticesScreen::new(db)?,
                        Screen::Quotes => {
                            quotes_screen = QuotesScreen::new(db)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{PracticeType, SetData};
    use crossterm::event::{KeyEvent, KeyEventKind, KeyEventState};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn ctrl_key(character: char) -> KeyEvent {
        KeyEvent {
            code: KeyCode::Char(character),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn returning_from_log_edit_refreshes_history_before_second_edit() {
        let db = Database::open_in_memory().unwrap();
        let practice = db
            .create_practice("Pull-up", PracticeType::Bodyweight)
            .unwrap();
        db.create_log(
            practice.id,
            &[SetData::Bodyweight { reps: 10 }],
            Some("original note"),
            None,
            None,
            Some(7),
        )
        .unwrap();
        let mut history = HistoryScreen::new(&db).unwrap();
        assert_eq!(
            history.selected_entry().unwrap().log.note.as_deref(),
            Some("original note")
        );

        let open_action = history.handle_key(key(KeyCode::Char('e')), &db);
        assert!(matches!(open_action, Action::Navigate(Screen::LogEntry)));
        let mut first_edit =
            LogEntryScreen::from_existing(&db, history.selected_entry().unwrap()).unwrap();
        for _ in 0..4 {
            first_edit.handle_key(key(KeyCode::Tab), &db);
        }
        first_edit.handle_key(ctrl_key('a'), &db);
        first_edit.handle_key(ctrl_key('k'), &db);
        for character in "new note".chars() {
            first_edit.handle_key(key(KeyCode::Char(character)), &db);
        }

        let save_action = first_edit.handle_key(ctrl_key('s'), &db);
        let target = match save_action {
            Action::Navigate(Screen::History) => Screen::History,
            _ => panic!("successful History edit must navigate back to History"),
        };
        refresh_history_for_navigation(&db, &mut history).unwrap();
        let current_screen = target;

        assert_eq!(current_screen, Screen::History);
        assert_eq!(
            history.selected_entry().unwrap().log.note.as_deref(),
            Some("new note")
        );

        let reopen_action = history.handle_key(key(KeyCode::Char('e')), &db);
        assert!(matches!(reopen_action, Action::Navigate(Screen::LogEntry)));
        let mut second_edit =
            LogEntryScreen::from_existing(&db, history.selected_entry().unwrap()).unwrap();
        let second_save = second_edit.handle_key(ctrl_key('s'), &db);
        assert!(matches!(second_save, Action::Navigate(Screen::History)));
        assert_eq!(
            db.list_logs_all().unwrap().remove(0).log.note.as_deref(),
            Some("new note")
        );
    }
}
