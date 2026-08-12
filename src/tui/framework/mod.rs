#[allow(dead_code)]
pub mod action_panel;
pub mod components;
pub mod input;
#[allow(dead_code)]
pub mod list_panel;
#[allow(dead_code)]
pub mod screen_frame;
#[cfg(test)]
pub mod test_support;
pub mod viewport;

#[allow(unused_imports)]
pub use action_panel::render_action_panel;
#[allow(unused_imports)]
pub use components::{
    fit_rect, render_detail_panel, render_scrollable_lines, render_shortcuts, render_status_line,
    shortcut_height, truncate_to_width, DetailPanelState, Shortcut, StatusMessage,
};
pub use input::TextInput;
#[allow(unused_imports)]
pub use list_panel::{render_scrollable_list_panel, ListPanelContent};
#[allow(unused_imports)]
pub use screen_frame::{render_screen_frame, ScreenFrameAreas};
#[allow(unused_imports)]
pub use viewport::{
    combined_span, move_selection, visual_spans, EdgeBehavior, Overflow, ViewportState, VisualSpan,
};
