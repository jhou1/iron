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
    dashboard::DashboardScreen, goals::GoalsScreen, history::HistoryScreen, lifecycle::Reenter,
    log_entry::LogEntryScreen, practices::PracticesScreen, quotes_screen::QuotesScreen,
    trends::TrendsScreen, Action, ModuleId, NavigationIntent, ScreenId,
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
    let mut current_screen = ScreenId::Dashboard;
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
                ScreenId::Dashboard => dashboard.render(frame),
                ScreenId::Goals => goals_screen.render(frame),
                ScreenId::LogEntry => log_entry.render(frame),
                ScreenId::History => history.render(frame),
                ScreenId::Trends => trends.render(frame),
                ScreenId::Practices => practices.render(frame),
                ScreenId::Quotes => quotes_screen.render(frame),
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
                ScreenId::Dashboard => dashboard.handle_key(key, db),
                ScreenId::Goals => goals_screen.handle_key(key, db),
                ScreenId::LogEntry => log_entry.handle_key(key, db),
                ScreenId::History => history.handle_key(key, db),
                ScreenId::Trends => trends.handle_key(key, db),
                ScreenId::Practices => practices.handle_key(key, db),
                ScreenId::Quotes => quotes_screen.handle_key(key, db),
            };

            match action {
                Action::Quit => break,
                Action::Navigate(intent) => {
                    match intent {
                        NavigationIntent::Open(module) => match module {
                            ModuleId::Dashboard => dashboard.refresh(db)?,
                            ModuleId::Goals => goals_screen = GoalsScreen::new(db)?,
                            ModuleId::History => history.refresh_preserving_context(db)?,
                            ModuleId::Trends => trends = trends.reenter(db)?,
                            ModuleId::Practices => practices = PracticesScreen::new(db)?,
                            ModuleId::Quotes => quotes_screen = QuotesScreen::new(db)?,
                        },
                        NavigationIntent::NewLog => {
                            log_entry = LogEntryScreen::new(db)?;
                        }
                        NavigationIntent::EditHistoryLog { log_id } => {
                            let entry = db
                                .get_log_by_id(log_id)?
                                .ok_or_else(|| anyhow::anyhow!("log {log_id} no longer exists"))?;
                            log_entry = LogEntryScreen::from_existing(db, &entry)?;
                        }
                    }
                    current_screen = intent.target();
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
    use chrono::NaiveDate;
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

    fn history_with_filtered_nonzero_target() -> (Database, HistoryScreen, i64) {
        let db = Database::open_in_memory().unwrap();
        let bench_press = db
            .create_practice("Bench Press", PracticeType::Bodyweight)
            .unwrap();
        let leg_press = db
            .create_practice("Leg Press", PracticeType::Bodyweight)
            .unwrap();
        let pull_up = db
            .create_practice("Pull-up", PracticeType::Bodyweight)
            .unwrap();
        let sets = [SetData::Bodyweight { reps: 10 }];
        let logged_at = |day| {
            NaiveDate::from_ymd_opt(2026, 8, day)
                .unwrap()
                .and_hms_opt(12, 0, 0)
                .unwrap()
        };
        db.create_log_at(
            bench_press.id,
            &logged_at(12),
            &sets,
            Some("newest press"),
            None,
            None,
            Some(7),
        )
        .unwrap();
        let target_id = db
            .create_log_at(
                leg_press.id,
                &logged_at(11),
                &sets,
                Some("original target"),
                None,
                None,
                Some(7),
            )
            .unwrap();
        db.create_log_at(
            pull_up.id,
            &logged_at(10),
            &sets,
            Some("outside filter"),
            None,
            None,
            Some(7),
        )
        .unwrap();

        let mut history = HistoryScreen::new(&db).unwrap();
        history.handle_key(key(KeyCode::Char('/')), &db);
        for character in "press".chars() {
            history.handle_key(key(KeyCode::Char(character)), &db);
        }
        history.handle_key(key(KeyCode::Enter), &db);
        history.handle_key(key(KeyCode::Char('j')), &db);
        assert_eq!(history.selected_entry().unwrap().log.id, target_id);
        (db, history, target_id)
    }

    fn replace_note(screen: &mut LogEntryScreen, db: &Database, note: &str) {
        for _ in 0..4 {
            screen.handle_key(key(KeyCode::Tab), db);
        }
        screen.handle_key(ctrl_key('a'), db);
        screen.handle_key(ctrl_key('k'), db);
        for character in note.chars() {
            screen.handle_key(key(KeyCode::Char(character)), db);
        }
    }

    #[test]
    fn returning_from_filtered_nonzero_log_edit_refreshes_same_entry_before_second_edit() {
        let (db, mut history, target_id) = history_with_filtered_nonzero_target();
        let open_action = history.handle_key(key(KeyCode::Char('e')), &db);
        let open_log_id = match open_action {
            Action::Navigate(NavigationIntent::EditHistoryLog { log_id }) => log_id,
            _ => panic!("History edit must navigate with the selected log ID"),
        };
        assert_eq!(open_log_id, target_id);
        let entry = db.get_log_by_id(open_log_id).unwrap().unwrap();
        let mut first_edit = LogEntryScreen::from_existing(&db, &entry).unwrap();
        replace_note(&mut first_edit, &db, "updated target");

        let save_action = first_edit.handle_key(ctrl_key('s'), &db);
        let target = match save_action {
            Action::Navigate(NavigationIntent::Open(ModuleId::History)) => ScreenId::History,
            _ => panic!("successful History edit must navigate back to History"),
        };
        history.refresh_preserving_context(&db).unwrap();
        let current_screen = target;

        assert_eq!(current_screen, ScreenId::History);
        assert_eq!(history.selected_entry().unwrap().log.id, target_id);
        assert_eq!(
            history.selected_entry().unwrap().log.note.as_deref(),
            Some("updated target")
        );

        let reopen_action = history.handle_key(key(KeyCode::Char('e')), &db);
        let reopen_log_id = match reopen_action {
            Action::Navigate(NavigationIntent::EditHistoryLog { log_id }) => log_id,
            _ => panic!("History re-edit must navigate with the selected log ID"),
        };
        assert_eq!(reopen_log_id, target_id);
        let entry = db.get_log_by_id(reopen_log_id).unwrap().unwrap();
        let mut second_edit = LogEntryScreen::from_existing(&db, &entry).unwrap();
        let second_save = second_edit.handle_key(ctrl_key('s'), &db);
        assert!(matches!(
            second_save,
            Action::Navigate(NavigationIntent::Open(ModuleId::History))
        ));
        assert_eq!(
            db.list_logs_all()
                .unwrap()
                .into_iter()
                .find(|entry| entry.log.id == target_id)
                .unwrap()
                .log
                .note
                .as_deref(),
            Some("updated target")
        );
    }

    #[test]
    fn canceling_filtered_nonzero_log_edit_preserves_history_context() {
        let (db, mut history, target_id) = history_with_filtered_nonzero_target();
        let entry = db.get_log_by_id(target_id).unwrap().unwrap();
        let mut edit = LogEntryScreen::from_existing(&db, &entry).unwrap();
        replace_note(&mut edit, &db, "discarded target");

        assert!(matches!(
            edit.handle_key(key(KeyCode::Esc), &db),
            Action::None
        ));
        let cancel_action = edit.handle_key(key(KeyCode::Char('y')), &db);
        assert!(matches!(
            cancel_action,
            Action::Navigate(NavigationIntent::Open(ModuleId::History))
        ));
        history.refresh_preserving_context(&db).unwrap();

        assert_eq!(history.selected_entry().unwrap().log.id, target_id);
        assert_eq!(
            history.selected_entry().unwrap().log.note.as_deref(),
            Some("original target")
        );
        assert!(matches!(
            history.handle_key(key(KeyCode::Esc), &db),
            Action::None
        ));
    }
}
