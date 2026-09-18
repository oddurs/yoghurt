//! What one thing is, and why it is here.
//!
//! The list says what is on the machine. This says why — and for a dependency
//! that means naming the thing you actually chose, which is the one question no
//! package manager answers in a single step.
//!
//! Composed as lines of text rather than handed to a wrapping widget, because a
//! wrapped paragraph that does not know about the indent it started with loses
//! its left edge, and a pane that loses its left edge stops being a pane.

use std::time::SystemTime;

use crate::model::graph::Graph;
use crate::model::question::Provenance;
use crate::view::row::Item;

/// One line of the pane.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Line {
    /// A heading with a rule running out to the edge.
    Section(String),
    /// Ordinary text, already indented.
    Text(String),
    /// A label and its value, aligned into a column.
    Field(String, String),
    /// Something the reader is meant to copy rather than read.
    Command(String),
    /// Nothing, for rhythm.
    Blank,
}

/// Everything worth saying about one item.
///
/// `now` is passed rather than read so the same item renders the same way twice.
#[must_use]
pub fn lines(graph: &Graph, item: &Item, now: SystemTime) -> Vec<Line> {
    let mut out = vec![Line::Text(headline(item))];

    if let Some(text) = &item.describes {
        out.push(Line::Text(text.clone()));
    }
    out.push(Line::Blank);
    out.extend(why(graph, item));
    out.extend(facts(item, now));
    out.extend(provides(item));
    out.extend(uninstall(item));
    out
}

/// The first line: what it is, at a glance.
fn headline(item: &Item) -> String {
    let version = item.version.as_deref().unwrap_or("-");
    let size = crate::view::plain::size(item.bytes);
    format!("{} {version} · {} · {size}", glyph(item), item.source)
}

fn glyph(item: &Item) -> &'static str {
    if item.outdated {
        "↑"
    } else {
        item.state.glyph()
    }
}

/// Why this is on the machine.
///
/// For a dependency this is the chain back to the nearest thing somebody chose,
/// which the `pulled in` query already computed — so this is layout rather than
/// logic.
fn why(graph: &Graph, item: &Item) -> Vec<Line> {
    let Some(id) = &item.package else {
        return vec![
            Line::Section("WHY".to_owned()),
            Line::Text("  Nothing on this machine claims it.".to_owned()),
            Line::Text("  It is on disk, and no package manager put it there.".to_owned()),
            Line::Blank,
        ];
    };

    let mut out = vec![Line::Section("WHY".to_owned())];
    match graph.why(id) {
        Provenance::Wanted => out.push(Line::Text("  You asked for this.".to_owned())),
        Provenance::System => {
            out.push(Line::Text("  It came with macOS.".to_owned()));
            out.push(Line::Text(
                "  Nobody installed it and it cannot be removed.".to_owned(),
            ));
        }
        Provenance::Unexplained => {
            out.push(Line::Text("  Nothing you installed needs it.".to_owned()));
            out.push(Line::Text(
                "  Usually what an uninstall left behind.".to_owned(),
            ));
        }
        Provenance::PulledIn(chain) => {
            // `glib`, then `└ gtk+3`, then `  └ inkscape  ← you installed this`.
            for (depth, step) in chain.iter().enumerate() {
                let indent = "  ".repeat(depth);
                let last = depth + 1 == chain.len();
                let line = if depth == 0 {
                    format!("  {}", step.name)
                } else if last {
                    format!("  {indent}└ {}  ← you installed this", step.name)
                } else {
                    format!("  {indent}└ {}", step.name)
                };
                out.push(Line::Text(line));
            }
        }
    }
    out.push(Line::Blank);
    out
}

/// The things that are simply true.
fn facts(item: &Item, now: SystemTime) -> Vec<Line> {
    let mut out = vec![Line::Section("FACTS".to_owned())];
    out.push(Line::Field(
        "version".to_owned(),
        item.version.clone().unwrap_or_else(|| "-".to_owned()),
    ));
    out.push(Line::Field("source".to_owned(), item.source.clone()));
    out.push(Line::Field(
        "size".to_owned(),
        crate::view::plain::size(item.bytes),
    ));

    out.push(Line::Field(
        "installed".to_owned(),
        item.installed
            .map_or_else(|| "not recorded".to_owned(), |at| ago(at, now)),
    ));

    let update = if item.outdated {
        item.latest
            .clone()
            .map_or_else(|| "yes".to_owned(), |v| format!("yes, {v}"))
    } else if item.checked {
        "no, this is the newest".to_owned()
    } else {
        // Saying "current" here would be a claim nobody has checked.
        "not checked".to_owned()
    };
    out.push(Line::Field("newer version".to_owned(), update));

    if let Some(path) = &item.path {
        out.push(Line::Field("where".to_owned(), path.display().to_string()));
    }
    if let Some(label) = &item.labelled {
        // Marked as a guess, the way it is everywhere else.
        out.push(Line::Field(
            "purpose".to_owned(),
            format!("~ {label} (inferred)"),
        ));
    }
    out.push(Line::Blank);
    out
}

/// What it puts on the path.
fn provides(item: &Item) -> Vec<Line> {
    if item.provides.is_empty() {
        return Vec::new();
    }
    let mut out = vec![Line::Section("PROVIDES".to_owned())];
    // Wrapped into columns rather than one per line: a package with 45 commands
    // would otherwise be a pane you scroll past rather than read.
    for chunk in item.provides.chunks(4) {
        out.push(Line::Text(format!("  {}", chunk.join("  "))));
    }
    out.push(Line::Blank);
    out
}

/// How to get rid of it, as text.
///
/// Shown rather than run. Composing the command is the useful part — knowing
/// which manager owns a thing is exactly what yoghurt knows and a shell does
/// not.
fn uninstall(item: &Item) -> Vec<Line> {
    let Some(id) = &item.package else {
        return Vec::new();
    };
    let command = match id.source.as_str() {
        "homebrew" => format!("brew uninstall {}", id.name),
        "cargo" => format!("cargo uninstall {}", id.name),
        "rustup" => format!("rustup toolchain uninstall {}", id.name),
        // No command is better than a wrong one: an application is dragged
        // to the bin and the App Store has no uninstaller at all.
        _ => return Vec::new(),
    };
    vec![
        Line::Section("TO REMOVE".to_owned()),
        Line::Command(command),
    ]
}

/// How long ago, in the words a person uses.
fn ago(at: SystemTime, now: SystemTime) -> String {
    const DAY: u64 = 60 * 60 * 24;
    let Ok(elapsed) = now.duration_since(at) else {
        return "in the future, apparently".to_owned();
    };
    match elapsed.as_secs() / DAY {
        0 => "today".to_owned(),
        1 => "yesterday".to_owned(),
        d @ 2..=60 => format!("{d} days ago"),
        d @ 61..=729 => format!("{} months ago", d / 30),
        d => format!("{} years ago", d / 365),
    }
}

#[cfg(test)]
mod tests {
    use super::{Line, lines};
    use crate::model::fact::{Fact, PackageId};
    use crate::model::graph::Graph;
    use crate::view::row::{Item, State};
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime};

    /// A fixed clock. `from_secs` because `Duration::from_days` is unstable.
    fn now() -> SystemTime {
        const DAY: u64 = 60 * 60 * 24;
        SystemTime::UNIX_EPOCH + Duration::from_secs(20_000 * DAY)
    }

    fn item(name: &str, source: &str) -> Item {
        Item {
            name: name.to_owned(),
            source: source.to_owned(),
            version: Some("1.2.3".to_owned()),
            state: State::Fine,
            bytes: Some(6_500_000),
            package: Some(PackageId::new(source, name)),
            path: Some(PathBuf::from("/opt/homebrew/Cellar/x/1.2.3")),
            installed: None,
            outdated: false,
            checked: false,
            latest: None,
            provides: Vec::new(),
            describes: None,
            labelled: None,
        }
    }

    fn text(lines: &[Line]) -> String {
        lines
            .iter()
            .map(|line| match line {
                Line::Section(name) => format!("{name}\n"),
                Line::Text(t) => format!("{t}\n"),
                Line::Field(k, v) => format!("  {k}  {v}\n"),
                Line::Command(c) => format!("  {c}\n"),
                Line::Blank => "\n".to_owned(),
            })
            .collect()
    }

    /// inkscape was chosen; gtk+3 came with it; glib came with gtk+3.
    fn chain() -> Graph {
        Graph::from_facts([
            Fact::Wanted {
                package: PackageId::new("homebrew", "inkscape"),
            },
            Fact::DependsOn {
                package: PackageId::new("homebrew", "inkscape"),
                on: PackageId::new("homebrew", "gtk+3"),
                declared_directly: true,
            },
            Fact::DependsOn {
                package: PackageId::new("homebrew", "gtk+3"),
                on: PackageId::new("homebrew", "glib"),
                declared_directly: true,
            },
        ])
    }

    #[test]
    fn a_dependency_names_the_thing_you_actually_chose() {
        let rendered = text(&lines(&chain(), &item("glib", "homebrew"), now()));
        assert!(rendered.contains("  glib\n"), "{rendered}");
        assert!(rendered.contains("  └ gtk+3\n"), "{rendered}");
        assert!(rendered.contains("← you installed this"), "{rendered}");
        assert!(rendered.contains("inkscape"), "{rendered}");
    }

    #[test]
    fn something_you_chose_says_so_in_one_line() {
        let rendered = text(&lines(&chain(), &item("inkscape", "homebrew"), now()));
        assert!(rendered.contains("You asked for this."), "{rendered}");
    }

    #[test]
    fn something_nothing_needs_says_what_it_probably_is() {
        let graph = Graph::from_facts([Fact::Package {
            id: PackageId::new("homebrew", "residue"),
            version: None,
        }]);
        let rendered = text(&lines(&graph, &item("residue", "homebrew"), now()));
        assert!(
            rendered.contains("Nothing you installed needs it."),
            "{rendered}"
        );
        assert!(rendered.contains("uninstall left behind"), "{rendered}");
    }

    #[test]
    fn something_nobody_claims_says_that_rather_than_guessing() {
        let mut orphan = item("Xcode.app", "-");
        orphan.package = None;
        orphan.state = State::Orphan;
        let rendered = text(&lines(&Graph::from_facts([]), &orphan, now()));
        assert!(
            rendered.contains("Nothing on this machine claims it."),
            "{rendered}"
        );
    }

    #[test]
    fn the_removal_command_is_the_one_for_the_manager_that_owns_it() {
        let brew = text(&lines(&chain(), &item("glib", "homebrew"), now()));
        assert!(brew.contains("brew uninstall glib"), "{brew}");

        let cargo = text(&lines(
            &Graph::from_facts([]),
            &item("ripgrep", "cargo"),
            now(),
        ));
        assert!(cargo.contains("cargo uninstall ripgrep"), "{cargo}");
    }

    #[test]
    fn a_source_with_no_uninstaller_offers_no_command_rather_than_a_wrong_one() {
        let rendered = text(&lines(
            &Graph::from_facts([]),
            &item("Things3", "app store"),
            now(),
        ));
        assert!(
            !rendered.contains("TO REMOVE"),
            "the App Store has no uninstaller: {rendered}"
        );
    }

    #[test]
    fn not_checked_is_never_reported_as_current() {
        let rendered = text(&lines(&chain(), &item("glib", "homebrew"), now()));
        assert!(rendered.contains("not checked"), "{rendered}");
        assert!(!rendered.contains("this is the newest"), "{rendered}");
    }

    #[test]
    fn a_checked_and_current_package_says_so() {
        let mut current = item("glib", "homebrew");
        current.checked = true;
        let rendered = text(&lines(&chain(), &current, now()));
        assert!(rendered.contains("no, this is the newest"), "{rendered}");
    }

    #[test]
    fn an_available_update_names_the_version() {
        let mut stale = item("pandoc", "homebrew");
        stale.outdated = true;
        stale.checked = true;
        stale.latest = Some("3.10".to_owned());
        let rendered = text(&lines(&chain(), &stale, now()));
        assert!(rendered.contains("yes, 3.10"), "{rendered}");
    }

    #[test]
    fn an_inferred_label_is_still_marked_as_inferred_here() {
        let mut labelled = item("ffmpeg", "homebrew");
        labelled.labelled = Some("media".to_owned());
        let rendered = text(&lines(&chain(), &labelled, now()));
        assert!(rendered.contains("~ media (inferred)"), "{rendered}");
    }

    #[test]
    fn many_commands_are_columns_rather_than_a_pane_you_scroll_past() {
        let mut many = item("ffmpeg", "homebrew");
        many.provides = (0..9).map(|n| format!("cmd{n}")).collect();
        let rendered = lines(&chain(), &many, now());
        let command_lines = rendered
            .iter()
            .filter(|l| matches!(l, Line::Text(t) if t.contains("cmd")))
            .count();
        assert_eq!(
            command_lines, 3,
            "nine commands in four columns is three lines"
        );
    }

    #[test]
    fn install_dates_are_said_the_way_a_person_says_them() {
        use super::ago;
        const DAY: u64 = 60 * 60 * 24;
        for (days, expected) in [
            (0, "today"),
            (1, "yesterday"),
            (5, "5 days ago"),
            (90, "3 months ago"),
            (800, "2 years ago"),
        ] {
            let at = now() - Duration::from_secs(days * DAY);
            assert_eq!(ago(at, now()), expected, "{days} days");
        }
    }
}
