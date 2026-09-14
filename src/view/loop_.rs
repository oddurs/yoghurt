//! Running the interface.
//!
//! Poll, draw, repeat. The poll has a timeout rather than blocking forever so
//! that a signal — which only sets a flag — is noticed within a frame, and so
//! that the freshness in the header keeps counting up while nobody types.

use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::view::app::App;
use crate::view::term::Screen;
use crate::view::ui;

/// How long to wait for input before redrawing anyway.
const TICK: Duration = Duration::from_millis(250);

/// Take the terminal and run until the person leaves.
///
/// # Errors
///
/// Returns the underlying error if the terminal cannot be taken, drawn to, or
/// read from.
pub fn run(mut app: App) -> io::Result<()> {
    let mut screen = Screen::open()?;
    let started = Instant::now();

    while !app.quit {
        app.scanned_ago = started.elapsed().as_secs();

        // The list occupies everything between the two header lines, the rule
        // and the footer.
        let height = usize::from(screen.area()?.height).saturating_sub(4);
        app.scroll_into_view(height);
        screen.draw(|frame| ui::draw(frame, &app))?;

        if screen.interrupted() {
            break;
        }
        if event::poll(TICK)?
            && let Event::Key(key) = event::read()?
        {
            handle(&mut app, key, height);
        }
    }
    Ok(())
}

/// What a keypress does.
///
/// Separate from the loop so every binding is testable without a terminal.
pub fn handle(app: &mut App, key: KeyEvent, page: usize) {
    // Key *releases* arrive on some terminals, and acting on both would move
    // the cursor twice for one press.
    if key.kind == KeyEventKind::Release {
        return;
    }
    let page = isize::try_from(page.max(1)).unwrap_or(1);

    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => app.quit = true,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => app.quit = true,
        KeyCode::Down | KeyCode::Char('j') => app.move_by(1),
        KeyCode::Up | KeyCode::Char('k') => app.move_by(-1),
        KeyCode::PageDown | KeyCode::Char('f') => app.move_by(page),
        KeyCode::PageUp | KeyCode::Char('b') => app.move_by(-page),
        KeyCode::Char('g') | KeyCode::Home => app.selected = 0,
        KeyCode::Char('G') | KeyCode::End => app.move_by(isize::MAX),
        KeyCode::Char(' ') | KeyCode::Enter => app.toggle_group(),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::handle;
    use crate::model::fact::{Fact, PackageId};
    use crate::model::graph::Graph;
    use crate::view::app::App;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

    fn machine() -> App {
        let mut facts = Vec::new();
        for name in ["a", "b", "c", "d"] {
            facts.push(Fact::Package {
                id: PackageId::new("homebrew", name),
                version: Some("1".to_owned()),
            });
        }
        App::new(Graph::from_facts(facts))
    }

    fn press(app: &mut App, code: KeyCode) {
        handle(app, KeyEvent::new(code, KeyModifiers::NONE), 2);
    }

    #[test]
    fn q_and_escape_both_leave() {
        for code in [KeyCode::Char('q'), KeyCode::Esc] {
            let mut app = machine();
            press(&mut app, code);
            assert!(app.quit);
        }
    }

    #[test]
    fn ctrl_c_leaves_rather_than_killing_the_process() {
        let mut app = machine();
        handle(
            &mut app,
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
            2,
        );
        assert!(app.quit);
    }

    #[test]
    fn the_arrows_and_their_vim_twins_do_the_same_thing() {
        let mut arrows = machine();
        let mut vim = machine();
        press(&mut arrows, KeyCode::Down);
        press(&mut vim, KeyCode::Char('j'));
        assert_eq!(arrows.selected, vim.selected);
        assert_eq!(arrows.selected, 1);
    }

    #[test]
    fn g_and_shift_g_go_to_the_ends() {
        let mut app = machine();
        press(&mut app, KeyCode::Char('G'));
        assert_eq!(app.selected, app.rows.len() - 1);
        press(&mut app, KeyCode::Char('g'));
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn paging_moves_by_the_height_it_was_given() {
        let mut app = machine();
        press(&mut app, KeyCode::PageDown);
        assert_eq!(app.selected, 2);
    }

    #[test]
    fn space_folds_the_group_the_cursor_is_in() {
        let mut app = machine();
        app.selected = 2;
        press(&mut app, KeyCode::Char(' '));
        assert_eq!(app.rows.len(), 1, "only the heading is left");
    }

    #[test]
    fn a_key_release_does_nothing() {
        let mut app = machine();
        let mut release = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        release.kind = KeyEventKind::Release;
        handle(&mut app, release, 2);
        assert_eq!(app.selected, 0, "one press must not move the cursor twice");
    }

    #[test]
    fn an_unbound_key_is_ignored_rather_than_doing_something_surprising() {
        let mut app = machine();
        press(&mut app, KeyCode::Char('z'));
        assert_eq!(app.selected, 0);
        assert!(!app.quit);
    }
}
