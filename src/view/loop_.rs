//! Running the interface.
//!
//! Poll, draw, repeat. The poll has a timeout rather than blocking forever so
//! that a signal — which only sets a flag — is noticed within a frame, and so
//! that the freshness in the header keeps counting up while nobody types.

use std::io;
use std::time::Duration;

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

    while !app.quit {
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

    // While typing, letters are the query rather than commands. Only the keys
    // that cannot be part of a name stay commands.
    if app.is_typing() {
        match key.code {
            KeyCode::Esc => {
                app.clear_one();
            }
            KeyCode::Enter => app.stop_typing(),
            KeyCode::Backspace => app.pop_query(),
            KeyCode::Up => app.move_by(-1),
            KeyCode::Down => app.move_by(1),
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.push_query(c);
            }
            KeyCode::Char('c') => app.quit = true,
            _ => {}
        }
        return;
    }

    let page = isize::try_from(page.max(1)).unwrap_or(1);
    match key.code {
        KeyCode::Char('q') => app.quit = true,
        // Undo one narrowing at a time, and leave only when there is nothing
        // left to undo. Escape should not discard a filter and quit at once.
        KeyCode::Esc => {
            if !app.clear_one() {
                app.quit = true;
            }
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => app.quit = true,
        KeyCode::Char('/') => app.start_typing(),
        KeyCode::Char('!') => app.cycle_facet(),
        KeyCode::Down | KeyCode::Char('j') => app.move_by(1),
        KeyCode::Up | KeyCode::Char('k') => app.move_by(-1),
        KeyCode::PageDown | KeyCode::Char('f') => app.move_by(page),
        KeyCode::PageUp | KeyCode::Char('b') => app.move_by(-page),
        KeyCode::Home => app.selected = 0,
        KeyCode::End => app.move_by(isize::MAX),
        KeyCode::Char('g') => app.cycle_axis(),
        KeyCode::Char('s') => app.cycle_sort(),
        KeyCode::Char('r') => app.rescan(crate::survey::survey),
        KeyCode::Char('S') => app.reverse_sort(),
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
    fn q_and_escape_both_leave_when_there_is_nothing_to_undo() {
        for code in [KeyCode::Char('q'), KeyCode::Esc] {
            let mut app = machine();
            press(&mut app, code);
            assert!(app.quit);
        }
    }

    #[test]
    fn escape_undoes_one_narrowing_at_a_time_before_it_leaves() {
        let mut app = machine();
        press(&mut app, KeyCode::Char('!'));
        press(&mut app, KeyCode::Char('/'));
        press(&mut app, KeyCode::Char('a'));

        press(&mut app, KeyCode::Esc);
        assert!(!app.is_typing(), "first escape stops typing");
        assert!(!app.quit);

        press(&mut app, KeyCode::Esc);
        assert!(app.filter.query.is_empty(), "then clears the query");
        assert!(!app.quit);

        press(&mut app, KeyCode::Esc);
        assert!(app.filter.facet.is_none(), "then the facet");
        assert!(!app.quit);

        press(&mut app, KeyCode::Esc);
        assert!(app.quit, "and only then leaves");
    }

    #[test]
    fn while_typing_letters_are_the_query_rather_than_commands() {
        let mut app = machine();
        press(&mut app, KeyCode::Char('/'));
        for c in "sq".chars() {
            press(&mut app, KeyCode::Char(c));
        }
        assert_eq!(
            app.filter.query, "sq",
            "s must not sort and q must not quit"
        );
        assert!(!app.quit);
    }

    #[test]
    fn backspace_takes_the_query_back_a_character() {
        let mut app = machine();
        press(&mut app, KeyCode::Char('/'));
        press(&mut app, KeyCode::Char('a'));
        press(&mut app, KeyCode::Char('b'));
        press(&mut app, KeyCode::Backspace);
        assert_eq!(app.filter.query, "a");
    }

    #[test]
    fn the_arrows_still_move_while_typing() {
        let mut app = machine();
        press(&mut app, KeyCode::Char('/'));
        press(&mut app, KeyCode::Down);
        assert!(app.is_typing(), "moving must not end the query");
    }

    #[test]
    fn bang_cycles_the_facets_and_comes_back_to_none() {
        use crate::view::row::Facet;
        let mut app = machine();
        for _ in 0..Facet::ALL.len() {
            press(&mut app, KeyCode::Char('!'));
        }
        assert!(app.filter.facet.is_some());
        press(&mut app, KeyCode::Char('!'));
        assert!(
            app.filter.facet.is_none(),
            "the last step is back to everything"
        );
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
    fn home_and_end_go_to_the_ends() {
        let mut app = machine();
        press(&mut app, KeyCode::End);
        assert_eq!(app.selected, app.rows.len() - 1);
        press(&mut app, KeyCode::Home);
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn g_cycles_the_grouping_axis() {
        use crate::view::row::Axis;
        let mut app = machine();
        assert_eq!(app.axis, Axis::Source);
        press(&mut app, KeyCode::Char('g'));
        assert_eq!(app.axis, Axis::Role);
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
