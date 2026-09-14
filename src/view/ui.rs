//! Drawing.
//!
//! Two fixed lines at the top and one at the bottom, and the view between them.
//! The header says what this is and how fresh it is; the strip says what is
//! happening before a single row is read; the footer says what the keys do
//! right now.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use std::fmt::Write as _;

use crate::view::app::App;
use crate::view::row::{Axis, Item, Row, State};

/// Below this the header drops to the identity and the counts.
const NARROW: u16 = 80;

/// Draw one frame.
pub fn draw(frame: &mut Frame, app: &App) {
    let [header, strip, rule, body, footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    draw_header(frame, app, header);
    draw_strip(frame, app, strip);
    draw_rule(frame, app, rule);
    draw_body(frame, app, body);
    draw_footer(frame, footer);
}

/// Identity, totals, and how fresh this is.
///
/// The timestamp is how a reader knows whether they are looking at the machine
/// or at a memory of it, so it is reserved first and never dropped. What gives
/// way as the header narrows is the totals, then the hostname.
fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let width = usize::from(area.width);
    let freshness = format!("{} ", app.freshness());
    let reserved = freshness.chars().count();

    let totals = app.totals();
    let mut left = vec![
        Span::styled(" yoghurt", Style::new().add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(app.host.clone(), Style::new().fg(Color::Cyan)),
    ];
    if area.width >= NARROW {
        left.push(Span::raw("   "));
        left.push(Span::styled(
            format!(
                "{} · {} · {}",
                plural(totals.packages, "package"),
                plural(totals.sources, "source"),
                super::plain::human(totals.bytes)
            ),
            Style::new().fg(Color::DarkGray),
        ));
    }

    truncate(&mut left, width.saturating_sub(reserved));
    let used: usize = left.iter().map(|s| s.content.chars().count()).sum();
    left.push(Span::raw(" ".repeat(width.saturating_sub(used + reserved))));
    left.push(Span::styled(freshness, Style::new().fg(Color::DarkGray)));

    frame.render_widget(Paragraph::new(Line::from(left)), area);
}

/// Cut spans down to `budget` cells, dropping whole spans from the end and
/// clipping the one that straddles the limit.
fn truncate(spans: &mut Vec<Span<'_>>, budget: usize) {
    let mut used = 0;
    for index in 0..spans.len() {
        let len = spans[index].content.chars().count();
        if used + len <= budget {
            used += len;
            continue;
        }
        let keep: String = spans[index].content.chars().take(budget - used).collect();
        spans[index].content = keep.into();
        spans.truncate(index + 1);
        return;
    }
}

/// What is happening, in one line.
///
/// Every count here is a filter in 0020. Until then it is still the fastest
/// answer to "is anything wrong", which is what most people open this for.
fn draw_strip(frame: &mut Frame, app: &App, area: Rect) {
    let mut spans = vec![Span::raw(" ")];
    let mut used = 1;

    for (label, count) in app.facets() {
        let text = format!("{count} {label}   ");
        let width = text.chars().count();
        // Drop a facet whole rather than cutting it in half. "2 bro" is worse
        // than not saying it.
        if used + width > usize::from(area.width) {
            break;
        }
        used += width;
        spans.push(Span::styled(
            count.to_string(),
            Style::new()
                .fg(facet_colour(label))
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            format!(" {label}   "),
            Style::new().fg(Color::DarkGray),
        ));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

/// One hue per facet, used identically wherever the facet appears.
fn facet_colour(label: &str) -> Color {
    match label {
        "wanted" => Color::Green,
        "pulled in" => Color::Blue,
        "outdated" => Color::Yellow,
        "broken" => Color::Red,
        _ => Color::Magenta,
    }
}

fn draw_rule(frame: &mut Frame, app: &App, area: Rect) {
    let title = format!("─ by {} ", app.axis.label());
    let rule = format!(
        "{title}{}",
        "─".repeat(usize::from(area.width).saturating_sub(title.chars().count()))
    );
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            rule,
            Style::new().fg(Color::DarkGray),
        ))),
        area,
    );
}

/// Widths at which a column stops paying for itself.
///
/// They drop in order of how little they answer: role, then size, then version.
/// The name and the glyph never drop.
///
/// There is no source column. The list is grouped by source, so the heading
/// above every row already says it, and repeating it costs twelve columns for
/// nothing. When 0033 adds the other grouping axes it comes back for those.
const SHOW_SOURCE: u16 = 96;
const SHOW_ROLE: u16 = 80;
const SHOW_SIZE: u16 = 68;
const SHOW_VERSION: u16 = 56;

/// The inventory.
fn draw_body(frame: &mut Frame, app: &App, area: Rect) {
    if app.rows.is_empty() {
        let message = if app.graph.packages().count() == 0 {
            "  Nothing found. No package manager on this machine reported anything."
        } else {
            "  Nothing matches."
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                message,
                Style::new().fg(Color::DarkGray),
            ))),
            area,
        );
        return;
    }

    let height = usize::from(area.height);
    // Grouping by source makes a source column redundant; any other axis does
    // not, so it comes back.
    let show_source = app.axis != Axis::Source;
    let lines: Vec<Line<'_>> = app
        .rows
        .iter()
        .enumerate()
        .skip(app.offset)
        .take(height)
        .map(|(index, row)| line_for(row, index == app.selected, area.width, show_source))
        .collect();

    frame.render_widget(Paragraph::new(lines), area);
}

/// One line of the list.
fn line_for(row: &Row, selected: bool, width: u16, show_source: bool) -> Line<'_> {
    let spans = match row {
        Row::Group {
            key,
            count,
            bytes,
            collapsed,
        } => group_line(key, *count, *bytes, *collapsed, width),
        Row::Item(item) => item_line(item, width, show_source),
    };
    let line = Line::from(spans);
    if selected {
        line.style(Style::new().add_modifier(Modifier::REVERSED))
    } else {
        line
    }
}

/// A heading, with what is under it.
fn group_line(key: &str, count: usize, bytes: u64, collapsed: bool, width: u16) -> Vec<Span<'_>> {
    let arrow = if collapsed { "▸" } else { "▾" };
    let right = format!("{count}  {}  ", super::plain::human(bytes));
    let left = format!(" {arrow} {key}");
    let gap = usize::from(width).saturating_sub(left.chars().count() + right.chars().count());

    vec![
        Span::styled(
            left,
            Style::new().fg(Color::White).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" ".repeat(gap)),
        Span::styled(right, Style::new().fg(Color::DarkGray)),
    ]
}

/// One package or orphan.
///
/// Everything right of the name is fixed width and right-aligned, so the eye
/// runs down a column instead of hunting along each row.
fn item_line(item: &Item, width: u16, show_source: bool) -> Vec<Span<'_>> {
    let mut right = String::new();
    if width >= SHOW_VERSION {
        let _ = write!(
            right,
            "{:>12}  ",
            trim(item.version.as_deref().unwrap_or("-"), 12)
        );
    }
    if width >= SHOW_ROLE {
        let label = if item.outdated {
            "outdated"
        } else {
            item.state.label()
        };
        let _ = write!(right, "{label:>8}  ");
    }
    if show_source && width >= SHOW_SOURCE {
        let _ = write!(right, "{:>10}  ", trim(&item.source, 10));
    }
    if width >= SHOW_SIZE {
        let _ = write!(right, "{:>6}  ", super::plain::size(item.bytes));
    }
    let used = 4 + right.chars().count();
    let room = usize::from(width).saturating_sub(used).max(8);
    let name = trim(&item.name, room);
    let gap = room.saturating_sub(name.chars().count());

    vec![
        Span::raw("  "),
        Span::styled(
            item.state.glyph(),
            Style::new().fg(state_colour(item.state)),
        ),
        Span::raw(" "),
        Span::raw(name),
        Span::raw(" ".repeat(gap)),
        Span::styled(right, Style::new().fg(Color::DarkGray)),
    ]
}

/// One hue per state, matching the strip above it.
fn state_colour(state: State) -> Color {
    match state {
        State::Fine => Color::Green,
        State::PulledIn => Color::Blue,
        State::Unexplained | State::Orphan => Color::Magenta,
        State::Broken => Color::Red,
    }
}

/// `1 package`, `2 packages`. Every noun here pluralises by adding an s.
fn plural(count: usize, noun: &str) -> String {
    if count == 1 {
        format!("{count} {noun}")
    } else {
        format!("{count} {noun}s")
    }
}

/// Cut a string to fit, marking that something was lost.
fn trim(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        return text.to_owned();
    }
    let kept: String = text.chars().take(width.saturating_sub(1)).collect();
    format!("{kept}…")
}

/// The keys that do something right now.
fn draw_footer(frame: &mut Frame, area: Rect) {
    let keys = [
        ("↑↓", "move"),
        ("space", "fold"),
        ("g", "group"),
        ("q", "quit"),
    ];
    let mut spans = vec![Span::raw(" ")];
    for (key, what) in keys {
        spans.push(Span::styled(key, Style::new().add_modifier(Modifier::BOLD)));
        spans.push(Span::styled(
            format!(" {what}  "),
            Style::new().fg(Color::DarkGray),
        ));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

#[cfg(test)]
mod tests {
    use crate::model::fact::{Fact, PackageId};
    use crate::model::graph::Graph;
    use crate::view::app::App;
    use crate::view::testkit::render;
    use std::path::PathBuf;

    fn machine() -> App {
        let rg = PackageId::new("homebrew", "ripgrep");
        App::new(Graph::from_facts([
            Fact::Package {
                id: rg.clone(),
                version: Some("15.2.0".to_owned()),
            },
            Fact::Wanted {
                package: rg.clone(),
            },
            Fact::Owns {
                package: rg,
                artifact: PathBuf::from("/opt/homebrew/Cellar/ripgrep/15.2.0"),
            },
            Fact::Size {
                artifact: PathBuf::from("/opt/homebrew/Cellar/ripgrep/15.2.0"),
                bytes: 6_500_000,
            },
        ]))
    }

    #[test]
    fn a_full_frame_matches_the_fixture() {
        let mut app = machine();
        app.host = "mba-oddur".to_owned();
        let rendered = render(&app, 78, 6).join("\n");
        let fixture = include_str!("../../tests/fixtures/chrome-78x6.txt");

        // Regenerate with: YOGHURT_BLESS=1 cargo test chrome
        if std::env::var_os("YOGHURT_BLESS").is_some() {
            std::fs::write("tests/fixtures/chrome-78x6.txt", format!("{rendered}\n"))
                .expect("write fixture");
            return;
        }
        assert_eq!(rendered, fixture.trim_end_matches('\n'), "\n{rendered}");
    }

    #[test]
    fn the_totals_appear_once_there_is_room_for_them() {
        let mut app = machine();
        app.host = "mba".to_owned();
        assert!(
            render(&app, 100, 4)[0].contains("1 package · 1 source · 6.2M"),
            "{:?}",
            render(&app, 100, 4)[0]
        );
        assert!(
            !render(&app, 60, 4)[0].contains("packages ·"),
            "no room at 60 columns"
        );
    }

    #[test]
    fn freshness_survives_even_the_narrowest_header() {
        let app = machine();
        assert!(
            render(&app, 40, 4)[0].contains("just now"),
            "a reader must know how stale this is"
        );
    }

    #[test]
    fn an_empty_machine_says_so_rather_than_showing_a_blank_pane() {
        let app = App::new(Graph::from_facts([]));
        assert!(render(&app, 80, 6).join("\n").contains("Nothing found"));
    }

    #[test]
    fn a_facet_that_does_not_fit_is_dropped_whole() {
        let frame = render(&machine(), 30, 4);
        assert!(
            !frame[1].contains("bro"),
            "a half-written word is worse than silence: {:?}",
            frame[1]
        );
        assert!(frame[1].starts_with(" 1 wanted"), "{:?}", frame[1]);
    }

    #[test]
    fn counts_are_pluralised() {
        use super::plural;
        assert_eq!(plural(1, "package"), "1 package");
        assert_eq!(plural(0, "source"), "0 sources");
        assert_eq!(plural(224, "package"), "224 packages");
    }

    #[test]
    fn the_source_is_not_repeated_on_every_row_under_its_own_heading() {
        let frame = render(&machine(), 120, 6).join("\n");
        assert_eq!(
            frame.matches("homebrew").count(),
            1,
            "the group heading says it once; the rows must not say it again:\n{frame}"
        );
    }

    #[test]
    fn every_line_is_exactly_as_wide_as_the_terminal() {
        for width in [40_u16, 60, 80, 100, 200] {
            for line in render(&machine(), width, 8) {
                assert_eq!(
                    line.chars().count(),
                    usize::from(width),
                    "at {width} columns"
                );
            }
        }
    }
}
