use ratatui::text::Line;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VisualSpan {
    pub start: usize,
    pub height: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Overflow {
    pub above: bool,
    pub below: bool,
}

impl Overflow {
    pub fn suffix(self) -> &'static str {
        match (self.above, self.below) {
            (true, true) => " ↑↓",
            (true, false) => " ↑",
            (false, true) => " ↓",
            (false, false) => "",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeBehavior {
    Clamp,
    Wrap,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ViewportState {
    offset: usize,
    height: usize,
}

impl ViewportState {
    pub fn new(height: usize) -> Self {
        Self { offset: 0, height }
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn set_height(&mut self, height: usize) {
        self.height = height;
    }

    pub fn set_offset(&mut self, offset: usize) {
        self.offset = offset;
    }

    pub fn reveal(&mut self, span: VisualSpan) {
        if self.height == 0 || span.start < self.offset || span.height > self.height {
            self.offset = span.start;
            return;
        }

        let span_end = span.start.saturating_add(span.height);
        let viewport_end = self.offset.saturating_add(self.height);
        if span_end > viewport_end {
            self.offset = span_end.saturating_sub(self.height);
        }
    }

    pub fn clamp(&mut self, content_height: usize) {
        self.offset = self.offset.min(content_height.saturating_sub(self.height));
    }

    pub fn overflow(&self, content_height: usize) -> Overflow {
        Overflow {
            above: self.offset > 0,
            below: self.offset.saturating_add(self.height) < content_height,
        }
    }
}

pub fn visual_spans(lines: &[Line<'_>], width: u16) -> Vec<VisualSpan> {
    let width = width.max(1) as usize;
    let mut start = 0;

    lines
        .iter()
        .map(|line| {
            let height = line.width().div_ceil(width).max(1);
            let span = VisualSpan { start, height };
            start += height;
            span
        })
        .collect()
}

pub fn combined_span(spans: &[VisualSpan], start: usize, count: usize) -> Option<VisualSpan> {
    if count == 0 {
        return None;
    }
    let first = spans.get(start)?;
    let last_index = start.checked_add(count - 1)?;
    let last = spans.get(last_index)?;
    let end = last.start.saturating_add(last.height);
    Some(VisualSpan {
        start: first.start,
        height: end.saturating_sub(first.start),
    })
}

pub fn move_selection(current: usize, len: usize, delta: isize, edge: EdgeBehavior) -> usize {
    if len == 0 {
        return 0;
    }

    let current = current.min(len - 1);
    match edge {
        EdgeBehavior::Clamp => current.saturating_add_signed(delta).min(len - 1),
        EdgeBehavior::Wrap => (current as isize + delta).rem_euclid(len as isize) as usize,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::text::Line;

    #[test]
    fn visual_spans_count_ascii_cjk_and_empty_rows() {
        let lines = vec![
            Line::from("123456789"),
            Line::from("中文测试"),
            Line::from(""),
        ];

        assert_eq!(
            visual_spans(&lines, 4),
            vec![
                VisualSpan {
                    start: 0,
                    height: 3,
                },
                VisualSpan {
                    start: 3,
                    height: 2,
                },
                VisualSpan {
                    start: 5,
                    height: 1,
                },
            ]
        );
    }

    #[test]
    fn reveal_keeps_selected_content_visible_and_reports_both_overflows() {
        let mut viewport = ViewportState::new(4);

        viewport.reveal(VisualSpan {
            start: 7,
            height: 2,
        });

        assert_eq!(viewport.offset(), 5);
        assert_eq!(
            viewport.overflow(12),
            Overflow {
                above: true,
                below: true,
            }
        );

        viewport.reveal(VisualSpan {
            start: 0,
            height: 1,
        });
        assert_eq!(viewport.offset(), 0);
    }

    #[test]
    fn selection_navigation_clamps_or_wraps_at_edges() {
        assert_eq!(move_selection(0, 4, -1, EdgeBehavior::Clamp), 0);
        assert_eq!(move_selection(3, 4, 1, EdgeBehavior::Clamp), 3);
        assert_eq!(move_selection(0, 4, -1, EdgeBehavior::Wrap), 3);
        assert_eq!(move_selection(3, 4, 1, EdgeBehavior::Wrap), 0);
        assert_eq!(move_selection(0, 0, 1, EdgeBehavior::Wrap), 0);
    }
}
