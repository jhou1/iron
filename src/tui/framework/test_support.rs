use ratatui::{backend::TestBackend, Terminal};

pub fn render_to_string(
    width: u16,
    height: u16,
    mut render: impl FnMut(&mut ratatui::Frame<'_>),
) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("test terminal should be created");
    terminal
        .draw(|frame| render(frame))
        .expect("test frame should render");

    let buffer = terminal.backend().buffer();
    (0..height)
        .map(|y| {
            (0..width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
                .trim_end()
                .to_owned()
        })
        .collect::<Vec<_>>()
        .join("\n")
}
