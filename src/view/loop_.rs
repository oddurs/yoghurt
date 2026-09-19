//! Running the interface.
//!
//! Poll, draw, repeat. The poll has a timeout rather than blocking forever so
//! that a signal — which only sets a flag — is noticed within a frame, and so
//! that the freshness in the header keeps counting up while nobody types.

use std::io;
use std::time::Duration;

use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent,
    MouseEventKind,
};

use crate::view::hit::Hit;

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
        screen.draw(|frame| ui::draw(frame, &mut app))?;

        if screen.interrupted() {
            break;
        }
        if event::poll(TICK)? {
            match event::read()? {
                Event::Key(key) => handle(&mut app, key, height),
                Event::Mouse(mouse) => point(&mut app, mouse, height),
                _ => {}
            }
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
            // Detail is the narrowest thing open, so it closes first.
            if app.detail.is_some() {
                app.detail = None;
            } else if !app.clear_one() {
                app.quit = true;
            }
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => app.quit = true,
        KeyCode::Char('/') => app.start_typing(),
        KeyCode::Char('!') => app.cycle_facet(),
        KeyCode::Down | KeyCode::Char('j') => app.move_by(1),
        KeyCode::Up | KeyCode::Char('k') => app.move_by(-1),
        // While detail is open the pages belong to it, because that is the
        // thing with more in it than fits.
        KeyCode::PageDown | KeyCode::Char('f') if app.detail.is_some() => app.scroll_detail(page),
        KeyCode::PageUp | KeyCode::Char('b') if app.detail.is_some() => app.scroll_detail(-page),
        KeyCode::PageDown | KeyCode::Char('f') => app.move_by(page),
        KeyCode::PageUp | KeyCode::Char('b') => app.move_by(-page),
        KeyCode::Home => app.selected = 0,
        KeyCode::End => app.move_by(isize::MAX),
        KeyCode::Char('g') => app.cycle_axis(),
        KeyCode::Char('s') => app.cycle_sort(),
        KeyCode::Char('r') => app.rescan(crate::survey::survey),
        // Shifted, because it reaches the network and the unshifted key must
        // stay the one that cannot surprise you.
        KeyCode::Char('R') => app.rescan(crate::survey::survey_checking_updates),
        KeyCode::Char('S') => app.reverse_sort(),
        KeyCode::Char(' ') => app.toggle_group(),
        KeyCode::Enter => app.toggle_detail(),
        _ => {}
    }
}

/// What the pointer does.
///
/// Every gesture here has a key that does the same thing, and every key that
/// matters has something on screen to point at. Neither half is an afterthought
/// bolted onto the other.
pub fn point(app: &mut App, mouse: MouseEvent, page: usize) {
    let what = app.hits.at(mouse.column, mouse.row).cloned();

    match mouse.kind {
        // Hover is tracked, not just clicks. Without it the pointer gives no
        // feedback until it commits to something.
        MouseEventKind::Moved => {
            app.hovered = what;
        }
        MouseEventKind::Down(MouseButton::Left) => match what {
            Some(Hit::Row(index)) => app.click_row(index),
            Some(Hit::Facet(label)) => {
                if let Some(facet) = crate::view::row::Facet::from_label(&label) {
                    app.toggle_facet(facet);
                }
            }
            Some(Hit::Axis) => app.cycle_axis(),
            Some(Hit::Sort) => app.cycle_sort(),
            // A key in the footer does exactly what pressing it does, rather
            // than a second implementation that can drift from the first.
            Some(Hit::Key(c)) => {
                handle(
                    app,
                    KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
                    page,
                );
            }
            Some(Hit::Detail) | None => {}
        },
        // Double-click opens what a single click selected.
        MouseEventKind::Down(MouseButton::Right) => {
            if let Some(Hit::Row(index)) = what {
                app.click_row(index);
                app.toggle_detail();
            }
        }
        MouseEventKind::ScrollDown => scroll(app, 3, what.as_ref()),
        MouseEventKind::ScrollUp => scroll(app, -3, what.as_ref()),
        _ => {}
    }
}

/// Scrolling belongs to whatever the pointer is over.
fn scroll(app: &mut App, delta: isize, what: Option<&Hit>) {
    if matches!(what, Some(Hit::Detail)) {
        app.scroll_detail(delta);
    } else {
        app.move_by(delta);
    }
}

#[cfg(test)]
mod tests {
    use super::{handle, point};
    use crate::model::fact::{Fact, PackageId};
    use crate::model::graph::Graph;
    use crate::view::app::App;
    use crate::view::hit::Hit;
    use crossterm::event::{
        KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
    };

    fn machine() -> App {
        let mut facts = Vec::new();
        for name in ["a", "b", "c", "d"] {
            let id = PackageId::new("homebrew", name);
            facts.push(Fact::Package {
                id: id.clone(),
                version: Some("1".to_owned()),
            });
            // Wanted, so that a facet filtering on it has something to show.
            facts.push(Fact::Wanted { package: id });
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
    fn enter_opens_detail_on_a_row_and_folds_on_a_heading() {
        let mut app = machine();
        assert_eq!(app.selected, 0, "the first row is the heading");
        press(&mut app, KeyCode::Enter);
        assert!(app.detail.is_none(), "a heading has no detail, so it folds");
        assert_eq!(app.rows.len(), 1);

        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Down);
        press(&mut app, KeyCode::Enter);
        assert!(app.detail.is_some(), "a row does");
    }

    #[test]
    fn escape_closes_detail_before_it_touches_the_filter() {
        let mut app = machine();
        press(&mut app, KeyCode::Char('!'));
        press(&mut app, KeyCode::Down);
        press(&mut app, KeyCode::Enter);
        assert!(app.detail.is_some());

        press(&mut app, KeyCode::Esc);
        assert!(
            app.detail.is_none(),
            "the narrowest thing open closes first"
        );
        assert!(app.filter.facet.is_some(), "and the filter survives");
    }

    #[test]
    fn detail_follows_the_cursor_rather_than_stranding_itself() {
        let mut app = machine();
        press(&mut app, KeyCode::Down);
        press(&mut app, KeyCode::Enter);
        app.scroll_detail(5);
        press(&mut app, KeyCode::Down);
        assert_eq!(
            app.detail,
            Some(0),
            "a new subject starts at the top of itself"
        );
    }

    #[test]
    fn paging_belongs_to_detail_while_it_is_open() {
        let mut app = machine();
        press(&mut app, KeyCode::Down);
        let row = app.selected;
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::PageDown);
        assert_eq!(app.selected, row, "the cursor stays put");
        assert_eq!(app.detail, Some(2), "the pane scrolls instead");
    }

    use crate::view::testkit::render;

    /// A frame has to have been drawn before anything can be pointed at, which
    /// is the whole design: if it was not drawn, it cannot be clicked.
    fn drawn(app: &mut App, width: u16, height: u16) {
        let _ = render(app, width, height);
    }

    fn at(kind: MouseEventKind, column: u16, row: u16) -> MouseEvent {
        MouseEvent {
            kind,
            column,
            row,
            modifiers: KeyModifiers::NONE,
        }
    }

    fn click(app: &mut App, column: u16, row: u16) {
        point(
            app,
            at(MouseEventKind::Down(MouseButton::Left), column, row),
            4,
        );
    }

    #[test]
    fn nothing_can_be_clicked_before_it_has_been_drawn() {
        let mut app = machine();
        click(&mut app, 10, 5);
        assert_eq!(app.selected, 0, "no frame, no regions, no targets");
    }

    #[test]
    fn clicking_a_row_selects_it() {
        let mut app = machine();
        drawn(&mut app, 100, 12);
        // Header, strip, rule, then the list: the heading, then the first item.
        click(&mut app, 10, 4);
        assert_eq!(app.selected, 1);
    }

    #[test]
    fn clicking_a_heading_folds_it_the_way_its_arrow_says_it_will() {
        let mut app = machine();
        drawn(&mut app, 100, 12);
        let before = app.rows.len();
        click(&mut app, 10, 3);
        assert!(app.rows.len() < before, "the arrow on it says fold");
    }

    #[test]
    fn clicking_a_count_in_the_strip_filters_by_it() {
        let mut app = machine();
        drawn(&mut app, 100, 12);
        click(&mut app, 2, 1);
        assert_eq!(
            app.filter.facet,
            Some(crate::view::row::Facet::Wanted),
            "the summary is the navigation"
        );
    }

    #[test]
    fn clicking_the_rule_changes_the_grouping() {
        use crate::view::row::Axis;
        let mut app = machine();
        drawn(&mut app, 100, 12);
        click(&mut app, 4, 2);
        assert_ne!(app.axis, Axis::Source);
    }

    #[test]
    fn clicking_a_key_in_the_footer_does_what_pressing_it_does() {
        let mut app = machine();
        let mut pressed = machine();
        drawn(&mut app, 100, 12);

        // `g` in the footer, and `g` on the keyboard.
        let footer = 11;
        click(&mut app, 30, footer);
        press(&mut pressed, KeyCode::Char('g'));
        assert_eq!(app.axis, pressed.axis, "one implementation, not two");
    }

    #[test]
    fn hovering_a_row_marks_it_without_selecting_it() {
        let mut app = machine();
        drawn(&mut app, 100, 12);
        point(&mut app, at(MouseEventKind::Moved, 10, 5), 4);
        assert_eq!(app.hovered, Some(Hit::Row(2)));
        assert_eq!(app.selected, 0, "pointing at something is not choosing it");
    }

    #[test]
    fn the_pointer_leaving_the_list_clears_the_mark() {
        let mut app = machine();
        drawn(&mut app, 100, 12);
        point(&mut app, at(MouseEventKind::Moved, 10, 5), 4);
        // The header is the one band that is not clickable.
        point(&mut app, at(MouseEventKind::Moved, 10, 0), 4);
        assert_eq!(app.hovered, None);
    }

    #[test]
    fn the_pointer_is_answered_by_everything_it_can_click() {
        let mut app = machine();
        drawn(&mut app, 100, 12);
        // A facet in the strip, the rule below it, and a key in the footer are
        // all clickable, and each used to stay silent under the pointer.
        point(&mut app, at(MouseEventKind::Moved, 2, 1), 4);
        assert!(
            matches!(app.hovered, Some(Hit::Facet(_))),
            "the strip: {:?}",
            app.hovered
        );
        point(&mut app, at(MouseEventKind::Moved, 3, 2), 4);
        assert!(
            matches!(app.hovered, Some(Hit::Axis | Hit::Sort)),
            "the rule: {:?}",
            app.hovered
        );
        point(&mut app, at(MouseEventKind::Moved, 24, 11), 4);
        assert!(
            matches!(app.hovered, Some(Hit::Key(_))),
            "the footer: {:?}",
            app.hovered
        );
    }

    #[test]
    fn the_wheel_moves_three_rows() {
        let mut app = machine();
        drawn(&mut app, 100, 12);
        point(&mut app, at(MouseEventKind::ScrollDown, 10, 5), 4);
        assert_eq!(app.selected, 3);
        point(&mut app, at(MouseEventKind::ScrollUp, 10, 5), 4);
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn the_wheel_over_detail_scrolls_detail_rather_than_the_list() {
        let mut app = machine();
        app.move_by(1);
        app.toggle_detail();
        drawn(&mut app, 120, 12);
        let row = app.selected;

        point(&mut app, at(MouseEventKind::ScrollDown, 100, 6), 4);
        assert_eq!(app.selected, row, "the list stays put");
        assert_eq!(app.detail, Some(3), "the pane moves");
    }

    #[test]
    fn an_unbound_key_is_ignored_rather_than_doing_something_surprising() {
        let mut app = machine();
        press(&mut app, KeyCode::Char('z'));
        assert_eq!(app.selected, 0);
        assert!(!app.quit);
    }
}
