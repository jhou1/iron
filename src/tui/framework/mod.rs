pub mod action_panel;
pub mod bounded_panel;
pub mod components;
pub mod input;
pub mod list_panel;
pub mod screen_frame;
pub mod static_panel;
#[cfg(test)]
pub mod test_support;
pub mod viewport;

pub use action_panel::render_action_panel;
#[allow(unused_imports)]
pub use components::{
    fit_rect, render_detail_panel, render_detail_panel_with_header, render_scrollable_lines,
    render_shortcuts, render_status_line, shortcut_height, truncate_to_width, DetailPanelState,
    Shortcut, StatusMessage,
};
pub use input::TextInput;
#[allow(unused_imports)]
pub use list_panel::{
    render_scrollable_list_panel, render_scrollable_list_panel_spanned, ListPanelAreas,
    ListPanelContent, SpannedListPanelContent,
};
pub use screen_frame::{render_screen_frame, render_screen_frame_sections, ScreenFrameSections};
#[allow(unused_imports)]
pub use static_panel::{render_static_panel, StaticPanelAreas};
#[allow(unused_imports)]
pub use viewport::{
    combined_span, move_selection, visual_spans, EdgeBehavior, Overflow, ViewportState, VisualSpan,
};
