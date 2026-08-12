use std::fmt::Write;

use ratatui::{backend::TestBackend, buffer::Buffer, Terminal};

pub fn render_to_buffer(
    width: u16,
    height: u16,
    mut render: impl FnMut(&mut ratatui::Frame<'_>),
) -> Buffer {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("test terminal should be created");
    terminal
        .draw(|frame| render(frame))
        .expect("test frame should render");
    terminal.backend().buffer().clone()
}

pub fn buffer_snapshot(buffer: &Buffer) -> String {
    let area = buffer.area;
    let mut output = format!(
        "area\tx={}\ty={}\twidth={}\theight={}\n",
        area.x, area.y, area.width, area.height
    );
    for y in area.y..area.bottom() {
        for x in area.x..area.right() {
            let cell = &buffer[(x, y)];
            writeln!(
                output,
                "{x},{y}\t{:?}\t{:?}\t{:?}\t{:?}\t{}",
                cell.symbol(),
                cell.fg,
                cell.bg,
                cell.modifier,
                cell.skip
            )
            .expect("writing to a String should succeed");
        }
    }
    output
}

pub fn render_to_snapshot(
    width: u16,
    height: u16,
    render: impl FnMut(&mut ratatui::Frame<'_>),
) -> String {
    buffer_snapshot(&render_to_buffer(width, height, render))
}

pub fn render_to_string(
    width: u16,
    height: u16,
    render: impl FnMut(&mut ratatui::Frame<'_>),
) -> String {
    let buffer = render_to_buffer(width, height, render);
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
