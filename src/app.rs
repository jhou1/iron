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

    fn handle_key(&mut self, key: KeyEvent, db: &Database) -> anyhow::Result<ShellControl> {
        let action = self.screens.handle_key(self.current, key, db);
        self.apply_action(action, db)
    }

    fn apply_action(&mut self, action: Action, db: &Database) -> anyhow::Result<ShellControl> {
        match action {
            Action::Quit => Ok(ShellControl::Quit),
            Action::Navigate(intent) => {
                self.router
                    .transition(&mut self.current, &mut self.screens, intent, db)?;
                Ok(ShellControl::Continue)
            }
            Action::None => Ok(ShellControl::Continue),
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

            match shell.handle_key(key, db)? {
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
            shell.handle_key(key(KeyCode::Char('h')), db).unwrap(),
            ShellControl::Continue
        );
        assert_eq!(
            shell.handle_key(key(KeyCode::Char('/')), db).unwrap(),
            ShellControl::Continue
        );
        for character in "press".chars() {
            assert_eq!(
                shell.handle_key(key(KeyCode::Char(character)), db).unwrap(),
                ShellControl::Continue
            );
        }
        shell.handle_key(key(KeyCode::Enter), db).unwrap();
        shell.handle_key(key(KeyCode::Char('j')), db).unwrap();
        shell.handle_key(key(KeyCode::Char('e')), db).unwrap();
    }

    #[test]
    fn app_shell_routes_dashboard_to_goals_and_back() {
        let db = Database::open_in_memory().unwrap();
        let mut shell = AppShell::new(&db, false).unwrap();

        assert_eq!(shell.current_for_test(), ScreenId::Dashboard);
        assert_eq!(
            shell.handle_key(key(KeyCode::Char('g')), &db).unwrap(),
            ShellControl::Continue
        );
        assert_eq!(shell.current_for_test(), ScreenId::Goals);
        assert_eq!(
            shell.handle_key(key(KeyCode::Esc), &db).unwrap(),
            ShellControl::Continue
        );
        assert_eq!(shell.current_for_test(), ScreenId::Dashboard);
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
            shell.handle_key(key(KeyCode::Tab), &db).unwrap();
        }
        shell.handle_key(ctrl_key('a'), &db).unwrap();
        shell.handle_key(ctrl_key('k'), &db).unwrap();
        for character in "updated target".chars() {
            shell
                .handle_key(key(KeyCode::Char(character)), &db)
                .unwrap();
        }
        shell.handle_key(ctrl_key('s'), &db).unwrap();

        assert_eq!(shell.current_for_test(), ScreenId::History);
        assert_eq!(
            shell.screens.history_context_for_test(),
            (
                Some(target_id),
                Some("updated target".into()),
                "press".into()
            )
        );
        shell.handle_key(key(KeyCode::Char('e')), &db).unwrap();
        assert_eq!(
            shell.screens.log_entry_for_test().identity_for_test().0,
            Some(target_id)
        );
    }

    #[test]
    fn shell_history_cancel_returns_to_same_unmodified_log() {
        let (db, mut shell, target_id) = lifecycle_shell_fixture();
        open_filtered_second_history_log(&mut shell, &db);
        shell.handle_key(key(KeyCode::Esc), &db).unwrap();
        shell.handle_key(key(KeyCode::Char('y')), &db).unwrap();

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
