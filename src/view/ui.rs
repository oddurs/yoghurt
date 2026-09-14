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

use crate::view::app::App;

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
    draw_rule(frame, rule);
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
                "{} packages · {} sources · {}",
                totals.packages,
                totals.sources,
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
    for (label, count) in app.facets() {
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

fn draw_rule(frame: &mut Frame, area: Rect) {
    let rule = "─".repeat(usize::from(area.width));
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            rule,
            Style::new().fg(Color::DarkGray),
        ))),
        area,
    );
}

/// The view itself. The list lands here in 0017.
fn draw_body(frame: &mut Frame, app: &App, area: Rect) {
    let totals = app.totals();
    let message = if totals.packages == 0 {
        "  Nothing found. No package manager on this machine reported anything."
    } else {
        "  The inventory lands here."
    };
    let _ = app;
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            message,
            Style::new().fg(Color::DarkGray),
        ))),
        area,
    );
}

/// The keys that do something right now.
fn draw_footer(frame: &mut Frame, area: Rect) {
    let keys = [("q", "quit"), ("?", "help")];
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
        assert!(render(&app, 100, 4)[0].contains("1 packages · 1 sources · 6.2M"));
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
