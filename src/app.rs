use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use fluent_bundle::FluentValue;
use ratatui::prelude::*;
use ratatui::widgets::Paragraph;
use std::io::stdout;

use crate::db::Database;
use crate::i18n::tr_args;
use crate::tui::{Action, ScreenId};

mod router;
mod screens;
use router::Router;
use screens::ScreenStore;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShellControl {
    Continue,
    Quit,
}

struct AppShell {
    current: ScreenId,
    screens: ScreenStore,
    router: Router,
}

impl AppShell {
    fn new(db: &Database, no_color: bool) -> anyhow::Result<Self> {
        Ok(Self {
            current: ScreenId::Dashboard,
            screens: ScreenStore::new(db, no_color)?,
            router: Router,
        })
    }

    fn render(&mut self, frame: &mut Frame) {
        self.screens.render(self.current, frame);
    }

    fn handle_key(&mut self, key: KeyEvent, db: &Database) -> ShellControl {
        let action = self.screens.handle_key(self.current, key, db);
        self.apply_action(action, db)
    }

    fn apply_action(&mut self, action: Action, db: &Database) -> ShellControl {
        match action {
            Action::Quit => ShellControl::Quit,
            Action::Navigate(intent) => {
                let source = self.current;
                if let Err(error) =
                    self.router
                        .transition(&mut self.current, &mut self.screens, intent, db)
                {
                    self.screens.set_navigation_error(
                        source,
                        tr_args(
                            "navigation-refresh-error",
                            &[("msg", FluentValue::from(error.to_string()))],
                        ),
                    );
                }
                ShellControl::Continue
            }
            Action::None => ShellControl::Continue,
        }
    }

    #[cfg(test)]
    fn current_for_test(&self) -> ScreenId {
        self.current
    }
}

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
    let mut shell = AppShell::new(db, no_color)?;

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
            shell.render(frame);
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind != event::KeyEventKind::Press {
                continue;
            }
            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                break;
            }

            match shell.handle_key(key, db) {
                ShellControl::Quit => break,
                ShellControl::Continue => {}
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{PracticeType, SetData};
    use crate::tui::{ModuleId, NavigationIntent};
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

    fn render_shell(shell: &mut AppShell, width: u16, height: u16) -> String {
        crate::tui::framework::test_support::render_to_string(width, height, |frame| {
            shell.render(frame)
        })
    }

    fn ctrl_key(character: char) -> KeyEvent {
        KeyEvent {
            code: KeyCode::Char(character),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn lifecycle_shell_fixture() -> (Database, AppShell, i64) {
        let db = Database::open_in_memory().unwrap();
        let bench = db
            .create_practice("Bench Press", PracticeType::Bodyweight)
            .unwrap();
        let leg = db
            .create_practice("Leg Press", PracticeType::Bodyweight)
            .unwrap();
        let pull = db
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
            bench.id,
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
                leg.id,
                &logged_at(11),
                &sets,
                Some("original target"),
                None,
                None,
                Some(7),
            )
            .unwrap();
        db.create_log_at(
            pull.id,
            &logged_at(10),
            &sets,
            Some("outside filter"),
            None,
            None,
            Some(7),
        )
        .unwrap();
        let shell = AppShell::new(&db, false).unwrap();
        (db, shell, target_id)
    }

    fn open_filtered_second_history_log(shell: &mut AppShell, db: &Database) {
        assert_eq!(
            shell.handle_key(key(KeyCode::Char('h')), db),
            ShellControl::Continue
        );
        assert_eq!(
            shell.handle_key(key(KeyCode::Char('/')), db),
            ShellControl::Continue
        );
        for character in "press".chars() {
            assert_eq!(
                shell.handle_key(key(KeyCode::Char(character)), db),
                ShellControl::Continue
            );
        }
        shell.handle_key(key(KeyCode::Enter), db);
        shell.handle_key(key(KeyCode::Char('j')), db);
        shell.handle_key(key(KeyCode::Char('e')), db);
    }

    #[test]
    fn app_shell_routes_dashboard_to_goals_and_back() {
        let db = Database::open_in_memory().unwrap();
        let mut shell = AppShell::new(&db, false).unwrap();

        assert_eq!(shell.current_for_test(), ScreenId::Dashboard);
        assert_eq!(
            shell.handle_key(key(KeyCode::Char('g')), &db),
            ShellControl::Continue
        );
        assert_eq!(shell.current_for_test(), ScreenId::Goals);
        assert_eq!(
            shell.handle_key(key(KeyCode::Esc), &db),
            ShellControl::Continue
        );
        assert_eq!(shell.current_for_test(), ScreenId::Dashboard);
    }

    #[test]
    fn dashboard_navigation_failure_stays_put_and_preserves_cached_target() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        let db = Database::open(temp.path()).unwrap();
        let mut shell = AppShell::new(&db, false).unwrap();
        let target_before = shell.screens.practices_identity_for_test();
        rusqlite::Connection::open(temp.path())
            .unwrap()
            .execute("DROP TABLE practices", [])
            .unwrap();

        assert_eq!(
            shell.handle_key(key(KeyCode::Char('e')), &db),
            ShellControl::Continue
        );
        assert_eq!(shell.current_for_test(), ScreenId::Dashboard);
        assert_eq!(shell.screens.practices_identity_for_test(), target_before);
        assert!(render_shell(&mut shell, 80, 35).contains("Unable to open screen"));
    }

    #[test]
    fn goals_navigation_failure_preserves_source_browse_context() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        let db = Database::open(temp.path()).unwrap();
        db.create_goal("Keep selected").unwrap();
        let mut shell = AppShell::new(&db, false).unwrap();
        assert_eq!(
            shell.handle_key(key(KeyCode::Char('g')), &db),
            ShellControl::Continue
        );
        let source_before = shell.screens.goals_context_for_test();
        rusqlite::Connection::open(temp.path())
            .unwrap()
            .execute("DROP TABLE daily_metrics", [])
            .unwrap();

        assert_eq!(
            shell.handle_key(key(KeyCode::Esc), &db),
            ShellControl::Continue
        );

        assert_eq!(shell.current_for_test(), ScreenId::Goals);
        assert_eq!(shell.screens.goals_context_for_test(), source_before);
        assert!(render_shell(&mut shell, 80, 24).contains("Unable to open screen"));
    }

    #[test]
    fn log_entry_return_failure_preserves_request_scoped_form() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        let db = Database::open(temp.path()).unwrap();
        let practice = db
            .create_practice("Pull-up", PracticeType::Bodyweight)
            .unwrap();
        db.create_log(
            practice.id,
            &[SetData::Bodyweight { reps: 10 }],
            Some("keep form"),
            None,
            None,
            Some(7),
        )
        .unwrap();
        let mut shell = AppShell::new(&db, false).unwrap();
        assert_eq!(
            shell.handle_key(key(KeyCode::Char('h')), &db),
            ShellControl::Continue
        );
        assert_eq!(
            shell.handle_key(key(KeyCode::Char('e')), &db),
            ShellControl::Continue
        );
        let form_before = shell.screens.log_entry_context_for_test();
        rusqlite::Connection::open(temp.path())
            .unwrap()
            .execute("DROP TABLE training_sessions", [])
            .unwrap();

        assert_eq!(
            shell.apply_action(
                Action::Navigate(NavigationIntent::Open(ModuleId::History)),
                &db,
            ),
            ShellControl::Continue
        );

        assert_eq!(shell.current_for_test(), ScreenId::LogEntry);
        assert_eq!(shell.screens.log_entry_context_for_test(), form_before);
        assert!(render_shell(&mut shell, 80, 24).contains("Unable to open screen"));
    }

    #[test]
    fn confirmed_cancel_refresh_failure_keeps_form_and_renders_navigation_error() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        let db = Database::open(temp.path()).unwrap();
        let practice = db
            .create_practice("Pull-up", PracticeType::Bodyweight)
            .unwrap();
        let log_id = db
            .create_log(
                practice.id,
                &[SetData::Bodyweight { reps: 10 }],
                Some("keep form"),
                None,
                None,
                Some(7),
            )
            .unwrap();
        let mut shell = AppShell::new(&db, false).unwrap();
        shell.handle_key(key(KeyCode::Char('h')), &db);
        let history_before = shell.screens.history_context_for_test();
        shell.handle_key(key(KeyCode::Char('e')), &db);
        let form_before = shell.screens.log_entry_context_for_test();
        assert_eq!(
            shell.screens.log_entry_for_test().identity_for_test().0,
            Some(log_id)
        );

        shell.handle_key(key(KeyCode::Esc), &db);
        rusqlite::Connection::open(temp.path())
            .unwrap()
            .execute("DROP TABLE training_sessions", [])
            .unwrap();
        shell.handle_key(key(KeyCode::Char('y')), &db);

        assert_eq!(shell.current_for_test(), ScreenId::LogEntry);
        assert_eq!(shell.screens.log_entry_context_for_test(), form_before);
        assert_eq!(shell.screens.history_context_for_test(), history_before);
        let rendered = render_shell(&mut shell, 80, 24);
        assert!(rendered.contains("Unable to open screen"), "{rendered}");
        assert!(rendered.contains("[y/Enter]"), "{rendered}");
        assert!(rendered.contains("[n/Esc]"), "{rendered}");
    }

    #[test]
    fn shell_history_save_returns_to_same_refreshed_log() {
        let (db, mut shell, target_id) = lifecycle_shell_fixture();
        open_filtered_second_history_log(&mut shell, &db);
        assert_eq!(shell.current_for_test(), ScreenId::LogEntry);
        assert_eq!(
            shell.screens.log_entry_for_test().identity_for_test().0,
            Some(target_id)
        );
        for _ in 0..4 {
            shell.handle_key(key(KeyCode::Tab), &db);
        }
        shell.handle_key(ctrl_key('a'), &db);
        shell.handle_key(ctrl_key('k'), &db);
        for character in "updated target".chars() {
            shell.handle_key(key(KeyCode::Char(character)), &db);
        }
        shell.handle_key(ctrl_key('s'), &db);

        assert_eq!(shell.current_for_test(), ScreenId::History);
        assert_eq!(
            shell.screens.history_context_for_test(),
            (
                Some(target_id),
                Some("updated target".into()),
                "press".into()
            )
        );
        shell.handle_key(key(KeyCode::Char('e')), &db);
        assert_eq!(
            shell.screens.log_entry_for_test().identity_for_test().0,
            Some(target_id)
        );
    }

    #[test]
    fn shell_history_cancel_returns_to_same_unmodified_log() {
        let (db, mut shell, target_id) = lifecycle_shell_fixture();
        open_filtered_second_history_log(&mut shell, &db);
        shell.handle_key(key(KeyCode::Esc), &db);
        shell.handle_key(key(KeyCode::Char('y')), &db);

        assert_eq!(shell.current_for_test(), ScreenId::History);
        assert_eq!(
            shell.screens.history_context_for_test(),
            (
                Some(target_id),
                Some("original target".into()),
                "press".into()
            )
        );
    }
}
