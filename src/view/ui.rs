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
use crate::view::detail;
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
    draw_footer(frame, app, footer);
}

/// Identity, totals, and how fresh this is.
///
/// The timestamp is how a reader knows whether they are looking at the machine
/// or at a memory of it, so it is reserved first and never dropped. What gives
/// way as the header narrows is the totals, then the hostname.
fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let width = usize::from(area.width);
    // A failed rescan says so where the freshness would be. Showing a time that
    // is quietly older than it looks is the one thing this line must never do.
    let freshness = match &app.failure {
        Some(_) => "scan failed ".to_owned(),
        None => format!("{} ", app.freshness()),
    };
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
        // The active facet is marked by shape as well as colour, so the strip
        // still says which one is on under `mono`.
        let active = app.filter.facet.is_some_and(|facet| facet.label() == label);
        let text = format!("{count} {label}{}   ", if active { " ◂" } else { "" });
        let width = text.chars().count();
        // Drop a facet whole rather than cutting it in half. "2 bro" is worse
        // than not saying it.
        if used + width > usize::from(area.width) {
            break;
        }
        used += width;

        let style = Style::new()
            .fg(facet_colour(label))
            .add_modifier(Modifier::BOLD);
        spans.push(Span::styled(
            count.to_string(),
            if active {
                style.add_modifier(Modifier::REVERSED)
            } else {
                style
            },
        ));
        spans.push(Span::styled(
            format!(" {label}{}   ", if active { " ◂" } else { "" }),
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
    // Descending by default, flipped by `S`. The arrow says which, because the
    // default differs per column and nobody should have to remember that.
    let descending = app.reversed ^ app.sort.descends_by_default();
    let arrow = if descending { "↓" } else { "↑" };
    let narrowing = app.filter.describe();
    let title = if narrowing.is_empty() {
        format!("─ by {} · {}{arrow} ", app.axis.label(), app.sort.label())
    } else {
        // What is being hidden matters more than how what is left is ordered.
        format!(
            "─ {narrowing}{} · by {} ",
            if app.is_typing() { "▌" } else { "" },
            app.axis.label()
        )
    };
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

/// Below this a detail pane costs the list more than it gives.
///
/// A pane beside an editor is usually sixty columns, and two panes in sixty is
/// two unreadable panes — so below this, detail takes the whole body instead.
const SPLIT: u16 = 96;

/// The least width detail is worth showing in.
const DETAIL_MIN: u16 = 38;

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

/// The inventory, and the detail beside it when there is room.
fn draw_body(frame: &mut Frame, app: &App, area: Rect) {
    let Some((item, offset)) = app.selected_item().zip(app.detail) else {
        draw_list(frame, app, area);
        return;
    };
    if area.width < SPLIT {
        // Too narrow to split: detail takes the body rather than halving
        // something that is already hard to read.
        draw_detail(frame, app, item, offset, area);
        return;
    }
    // Two fifths, but never less than readable. At a fixed width the line that
    // matters most — the one naming what you installed — was the first to be
    // cut, on exactly the wide terminals that had room to spare.
    let share = (area.width * 2 / 5).max(DETAIL_MIN);
    let [list, detail] =
        Layout::horizontal([Constraint::Min(0), Constraint::Length(share)]).areas(area);
    draw_list(frame, app, list);
    draw_detail(frame, app, item, offset, detail);
}

/// What one thing is, and why it is here.
fn draw_detail(frame: &mut Frame, app: &App, item: &Item, offset: usize, area: Rect) {
    let width = usize::from(area.width).saturating_sub(2);
    let mut lines: Vec<Line<'_>> = vec![Line::from(Span::styled(
        format!(" {}", trim(&item.name, width)),
        Style::new().fg(Color::White).add_modifier(Modifier::BOLD),
    ))];

    for line in crate::view::detail::lines(&app.graph, item, app.now) {
        lines.push(match line {
            detail::Line::Section(name) => {
                // A heading with a rule out to the edge: cheaper to scan than a
                // column of capitals, and it gives the pane a rhythm.
                let rule = "─".repeat(width.saturating_sub(name.chars().count() + 2));
                Line::from(Span::styled(
                    format!(" {name} {rule}"),
                    Style::new().fg(Color::DarkGray),
                ))
            }
            detail::Line::Text(text) => Line::from(Span::raw(trim(&text, width))),
            detail::Line::Field(key, value) => Line::from(vec![
                Span::styled(format!("  {key:<13} "), Style::new().fg(Color::DarkGray)),
                Span::raw(trim(&value, width.saturating_sub(16))),
            ]),
            detail::Line::Command(command) => Line::from(Span::styled(
                format!("  {}", trim(&command, width.saturating_sub(2))),
                Style::new().fg(Color::Cyan),
            )),
            detail::Line::Blank => Line::from(""),
        });
    }

    let height = usize::from(area.height).max(1);
    let offset = offset.min(lines.len().saturating_sub(height));
    frame.render_widget(
        Paragraph::new(
            lines
                .into_iter()
                .skip(offset)
                .take(height)
                .collect::<Vec<_>>(),
        ),
        area,
    );
}

/// The inventory.
fn draw_list(frame: &mut Frame, app: &App, area: Rect) {
    if app.rows.is_empty() {
        let message = if app.graph.packages().count() == 0 {
            "  Nothing found. No package manager on this machine reported anything.".to_owned()
        } else {
            format!(
                "  Nothing matches {}. Press esc to widen.",
                app.filter.describe()
            )
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
        Span::styled(glyph(item), Style::new().fg(glyph_colour(item))),
        Span::raw(" "),
        Span::raw(name),
        Span::raw(" ".repeat(gap)),
        Span::styled(right, Style::new().fg(Color::DarkGray)),
    ]
}

/// The glyph a row wears.
///
/// Being out of date is the more urgent thing to say, so it wins over the
/// provenance glyph — but only for the glyph. The row still belongs to whatever
/// it belonged to.
fn glyph(item: &Item) -> &'static str {
    if item.outdated {
        "↑"
    } else {
        item.state.glyph()
    }
}

/// The colour that goes with it.
fn glyph_colour(item: &Item) -> Color {
    if item.outdated {
        Color::Yellow
    } else {
        state_colour(item.state)
    }
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
fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    // The footer says what the keys do *now*, so typing shows a different set.
    let keys: &[(&str, &str)] = if app.is_typing() {
        &[("type", "filter"), ("↵", "keep"), ("esc", "clear")]
    } else {
        &[
            ("↑↓", "move"),
            ("space", "fold"),
            ("g", "group"),
            ("s", "sort"),
            ("/", "find"),
            ("!", "facet"),
            ("↵", "detail"),
            ("r", "rescan"),
            ("q", "quit"),
        ]
    };
    let mut spans = vec![Span::raw(" ")];
    for (key, what) in keys {
        spans.push(Span::styled(
            *key,
            Style::new().add_modifier(Modifier::BOLD),
        ));
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
    fn an_outdated_row_wears_the_arrow_rather_than_its_provenance_glyph() {
        use super::glyph;
        use crate::view::row::{Item, State};
        let mut item = Item {
            name: "pandoc".to_owned(),
            source: "homebrew".to_owned(),
            version: None,
            state: State::Fine,
            bytes: None,
            package: None,
            path: None,
            installed: None,
            outdated: true,
            checked: true,
            latest: None,
            provides: Vec::new(),
            describes: None,
            labelled: None,
        };
        assert_eq!(
            glyph(&item),
            "↑",
            "being stale is the more urgent thing to say"
        );
        item.outdated = false;
        assert_eq!(glyph(&item), "●");
    }

    #[test]
    fn the_active_facet_is_marked_by_shape_not_only_by_colour() {
        use crate::view::row::Facet;
        let mut app = machine();
        assert!(!render(&app, 90, 4)[1].contains('◂'));
        app.toggle_facet(Facet::Wanted);
        assert!(
            render(&app, 90, 4)[1].contains("wanted ◂"),
            "under mono, colour says nothing: {:?}",
            render(&app, 90, 4)[1]
        );
    }

    #[test]
    fn a_failed_scan_says_so_where_the_freshness_would_be() {
        let mut app = machine();
        app.failure = Some("brew fell over".to_owned());
        let header = render(&app, 90, 4)[0].clone();
        assert!(header.contains("scan failed"), "{header}");
        assert!(
            !header.contains("scanned"),
            "a time that is quietly older than it looks is the worst thing this line can say"
        );
    }

    fn with_detail(width: u16, height: u16) -> Vec<String> {
        let mut app = machine();
        app.move_by(1);
        app.toggle_detail();
        render(&app, width, height)
    }

    #[test]
    fn detail_sits_beside_the_list_when_there_is_room() {
        let frame = with_detail(120, 8).join("\n");
        assert!(
            frame.contains("▾ homebrew"),
            "the list is still there:\n{frame}"
        );
        assert!(frame.contains("WHY"), "and so is the pane:\n{frame}");
    }

    #[test]
    fn detail_takes_the_whole_body_when_the_split_would_be_unreadable() {
        let frame = with_detail(80, 8).join("\n");
        assert!(frame.contains("WHY"), "{frame}");
        assert!(
            !frame.contains("▾ homebrew"),
            "two panes in eighty columns is two unreadable panes:\n{frame}"
        );
    }

    #[test]
    fn every_line_is_still_exactly_the_width_with_detail_open() {
        for width in [80_u16, 100, 120, 200] {
            for line in with_detail(width, 10) {
                assert_eq!(
                    line.chars().count(),
                    usize::from(width),
                    "at {width} columns"
                );
            }
        }
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
