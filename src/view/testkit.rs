//! Rendering a frame without a terminal.
//!
//! An interface that can only be checked by looking at it will not be checked,
//! and this is the item that gets left out. It is here first, before the views
//! it tests, so that every one of them arrives with a way to assert what it
//! draws.

use ratatui::Terminal;
use ratatui::backend::TestBackend;

use crate::view::app::App;

/// Draw `app` at this size and read the cells back as lines of text.
///
/// Trailing spaces are kept: a line that is not the full width is a layout bug,
/// and a test that trimmed them would hide it.
///
/// # Panics
///
/// Panics if the in-memory backend fails, which it cannot: there is no
/// terminal to be wrong about.
#[must_use]
pub fn render(app: &mut App, width: u16, height: u16) -> Vec<String> {
    let mut terminal =
        Terminal::new(TestBackend::new(width, height)).expect("a test backend cannot fail");
    terminal
        .draw(|frame| super::ui::draw(frame, app))
        .expect("drawing cannot fail");

    let buffer = terminal.backend().buffer();
    (0..height)
        .map(|y| {
            (0..width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::render;
    use crate::model::graph::Graph;
    use crate::view::app::App;

    #[test]
    fn the_harness_needs_no_terminal_and_no_package_manager() {
        let mut app = App::new(Graph::from_facts([]));
        let frame = render(&mut app, 40, 5);
        assert_eq!(frame.len(), 5);
        assert!(frame.iter().all(|line| line.chars().count() == 40));
    }
}
