use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    style::{Color, Style},
    text::Span,
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextInput {
    value: String,
    cursor: usize,
}

impl TextInput {
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(dead_code)]
    pub fn from(value: impl Into<String>) -> Self {
        let value = value.into();
        let cursor = value.len();
        Self { value, cursor }
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    #[allow(dead_code)]
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn is_empty_trimmed(&self) -> bool {
        self.trimmed().is_empty()
    }

    pub fn trimmed(&self) -> &str {
        self.value.trim()
    }

    pub fn set(&mut self, value: impl Into<String>) {
        self.value = value.into();
        self.cursor = self.value.len();
    }

    pub fn clear(&mut self) {
        self.value.clear();
        self.cursor = 0;
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        let control = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Left | KeyCode::Char('b') if key.code == KeyCode::Left || control => {
                self.move_left();
                true
            }
            KeyCode::Right | KeyCode::Char('f') if key.code == KeyCode::Right || control => {
                self.move_right();
                true
            }
            KeyCode::Home | KeyCode::Char('a') if key.code == KeyCode::Home || control => {
                self.cursor = 0;
                true
            }
            KeyCode::End | KeyCode::Char('e') if key.code == KeyCode::End || control => {
                self.cursor = self.value.len();
                true
            }
            KeyCode::Backspace => {
                if self.cursor > 0 {
                    let previous = self.previous_boundary();
                    self.value.remove(previous);
                    self.cursor = previous;
                }
                true
            }
            KeyCode::Char('k') if control => {
                self.value.truncate(self.cursor);
                true
            }
            KeyCode::Char(character) if !control => {
                self.value.insert(self.cursor, character);
                self.cursor += character.len_utf8();
                true
            }
            _ => false,
        }
    }

    #[allow(dead_code)]
    pub fn handle_key_if<F>(&mut self, key: KeyEvent, accepts: F) -> bool
    where
        F: Fn(char) -> bool,
    {
        let control = key.modifiers.contains(KeyModifiers::CONTROL);
        if let KeyCode::Char(character) = key.code {
            if !control && !accepts(character) {
                return false;
            }
        }
        self.handle_key(key)
    }

    pub fn visible_spans(
        &self,
        max_width: u16,
        prefix_width: u16,
        color: Color,
    ) -> Vec<Span<'static>> {
        let available = max_width.saturating_sub(prefix_width).saturating_sub(1) as usize;
        let before = &self.value[..self.cursor];
        let after = &self.value[self.cursor..];
        let before_width = before.width();

        let (visible_before, visible_after) = if before_width + after.width() <= available {
            (before.to_owned(), after.to_owned())
        } else {
            let mut start = before.len();
            let mut used = 0;
            for (index, character) in before.char_indices().rev() {
                let width = character.width().unwrap_or(0);
                if used + width > available {
                    break;
                }
                used += width;
                start = index;
            }

            let remaining = available.saturating_sub(used);
            let mut end = 0;
            let mut after_width = 0;
            for (index, character) in after.char_indices() {
                let width = character.width().unwrap_or(0);
                if after_width + width > remaining {
                    break;
                }
                after_width += width;
                end = index + character.len_utf8();
            }
            (before[start..].to_owned(), after[..end].to_owned())
        };

        vec![
            Span::styled(visible_before, Style::default().fg(color)),
            Span::styled("_", Style::default().fg(color)),
            Span::styled(visible_after, Style::default().fg(color)),
        ]
    }

    fn previous_boundary(&self) -> usize {
        self.value[..self.cursor]
            .char_indices()
            .next_back()
            .map(|(index, _)| index)
            .unwrap_or(0)
    }

    fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor = self.previous_boundary();
        }
    }

    fn move_right(&mut self) {
        if self.cursor < self.value.len() {
            self.cursor = self.value[self.cursor..]
                .char_indices()
                .nth(1)
                .map(|(index, _)| self.cursor + index)
                .unwrap_or(self.value.len());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn unicode_editing_moves_and_deletes_on_char_boundaries() {
        let mut input = TextInput::from("目标A");
        input.handle_key(key(KeyCode::Left));
        input.handle_key(key(KeyCode::Backspace));
        assert_eq!(input.value(), "目A");
        assert_eq!(input.cursor(), "目".len());
    }

    #[test]
    fn emacs_navigation_and_kill_match_project_contract() {
        let mut input = TextInput::from("abcdef");
        input.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL));
        input.handle_key(key(KeyCode::Right));
        input.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::CONTROL));
        assert_eq!(input.value(), "a");
    }

    #[test]
    fn visible_spans_keep_long_cursor_inside_width() {
        let input = TextInput::from("X".repeat(100));
        let rendered = input.visible_spans(12, 2, ratatui::style::Color::Green);
        let text: String = rendered.iter().map(|span| span.content.as_ref()).collect();
        let line = ratatui::text::Line::from(rendered);
        assert!(line.width() <= 10);
        assert!(text.contains('_'));
    }

    #[test]
    fn filtered_input_rejects_invalid_characters_without_blocking_navigation() {
        let mut input = TextInput::from("12");
        input.handle_key_if(key(KeyCode::Char('x')), |character| {
            character.is_ascii_digit()
        });
        input.handle_key_if(key(KeyCode::Left), |character| character.is_ascii_digit());
        input.handle_key_if(key(KeyCode::Backspace), |character| {
            character.is_ascii_digit()
        });

        assert_eq!(input.value(), "2");
        assert_eq!(input.cursor(), 0);
    }
}
