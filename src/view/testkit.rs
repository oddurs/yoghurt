//! Rendering a frame without a terminal.
//!
//! An interface that can only be checked by looking at it will not be checked,
//! and this is the item that gets left out. It is here first, before the views
//! it tests, so that every one of them arrives with a way to assert what it
//! draws.

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::style::{Color, Modifier};

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

/// What one cell was drawn with, reduced to what a reader can see.
///
/// Bold is deliberately not here: a heading keeps its weight inside a
/// highlighted bar, and that is not a difference in the bar itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Paint {
    /// The foreground it was given.
    pub fg: Color,
    /// The background it was given.
    pub bg: Color,
    /// Whether the two are swapped, which turns a foreground into a
    /// background and is how this interface highlights.
    pub reversed: bool,
}

/// Draw `app` at this size and read back how each cell of one row is painted.
///
/// The text harness above cannot see colour, so the bug where a highlighted
/// row was a bar of three different backgrounds — white under the name, dark
/// grey under the size, green under the glyph — rendered a perfect fixture.
///
/// # Panics
///
/// Panics if the in-memory backend fails, which it cannot.
#[must_use]
pub fn paint(app: &mut App, width: u16, height: u16, row: u16) -> Vec<Paint> {
    let mut terminal =
        Terminal::new(TestBackend::new(width, height)).expect("a test backend cannot fail");
    terminal
        .draw(|frame| super::ui::draw(frame, app))
        .expect("drawing cannot fail");

    let buffer = terminal.backend().buffer();
    (0..width)
        .map(|x| {
            let cell = &buffer[(x, row)];
            Paint {
                fg: cell.fg,
                bg: cell.bg,
                reversed: cell.modifier.contains(Modifier::REVERSED),
            }
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
