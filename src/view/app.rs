//! What the interface is looking at.
//!
//! State only: what the machine is, where the cursor is, and what the person
//! has asked to see. No drawing and no terminal, so every question the
//! interface can answer is answerable in a test without one.

use std::collections::BTreeSet;

use crate::model::graph::Graph;
use crate::view::row::{Row, build};

/// The interface's whole state.
pub struct App {
    /// The machine.
    pub graph: Graph,
    /// The hostname, for the header.
    pub host: String,
    /// How long ago the scan finished, in seconds.
    pub scanned_ago: u64,
    /// Whether a scan is still running.
    pub scanning: bool,
    /// Set when the person has asked to leave.
    pub quit: bool,
    /// The list, flattened. Rebuilt whenever what it shows changes.
    pub rows: Vec<Row>,
    /// Which group headings are folded shut.
    pub collapsed: BTreeSet<String>,
    /// Where the cursor is, as an index into `rows`.
    pub selected: usize,
    /// The first visible row.
    pub offset: usize,
}

impl App {
    /// Look at this machine.
    #[must_use]
    pub fn new(graph: Graph) -> Self {
        let rows = build(&graph, &BTreeSet::new());
        Self {
            graph,
            host: hostname(),
            scanned_ago: 0,
            scanning: false,
            quit: false,
            rows,
            collapsed: BTreeSet::new(),
            selected: 0,
            offset: 0,
        }
    }

    /// Rebuild the list, keeping the cursor on whatever it was pointing at.
    ///
    /// Collapsing a group moves every row after it, and a cursor that jumped to
    /// a different package each time would make the list unusable.
    pub fn rebuild(&mut self) {
        let anchor = self.rows.get(self.selected).cloned();
        self.rows = build(&self.graph, &self.collapsed);
        self.selected = anchor
            .and_then(|was| self.rows.iter().position(|row| same_thing(row, &was)))
            .unwrap_or(self.selected)
            .min(self.rows.len().saturating_sub(1));
    }

    /// Move the cursor, stopping at both ends rather than wrapping.
    ///
    /// Wrapping in a list of four hundred means a keypress can take you a long
    /// way from where you were looking.
    pub fn move_by(&mut self, delta: isize) {
        if self.rows.is_empty() {
            return;
        }
        let last = self.rows.len() - 1;
        self.selected = self.selected.saturating_add_signed(delta).min(last);
    }

    /// Fold or unfold the group the cursor is on.
    ///
    /// On an item, folds the group that contains it, so the cursor does not
    /// have to travel to the heading first.
    pub fn toggle_group(&mut self) {
        let Some(key) = self.group_at(self.selected) else {
            return;
        };
        if !self.collapsed.remove(&key) {
            self.collapsed.insert(key.clone());
            // Folding from inside means the heading is where you end up.
            if let Some(index) = self
                .rows
                .iter()
                .position(|row| matches!(row, Row::Group { key: k, .. } if *k == key))
            {
                self.selected = index;
            }
        }
        self.rebuild();
    }

    /// Which group a row belongs to.
    fn group_at(&self, index: usize) -> Option<String> {
        match self.rows.get(index)? {
            Row::Group { key, .. } => Some(key.clone()),
            Row::Item(item) => Some(item.source.clone()),
        }
    }

    /// Keep the cursor inside a window of `height` rows, moving as little as
    /// possible — the list should not jump when the cursor is already visible.
    pub fn scroll_into_view(&mut self, height: usize) {
        if height == 0 {
            return;
        }
        if self.selected < self.offset {
            self.offset = self.selected;
        } else if self.selected >= self.offset + height {
            self.offset = self.selected + 1 - height;
        }
        let max = self.rows.len().saturating_sub(height);
        self.offset = self.offset.min(max);
    }

    /// Totals for the header: packages, sources, bytes.
    #[must_use]
    pub fn totals(&self) -> Totals {
        let packages = self.graph.packages().count();
        let mut sources: Vec<&str> = self
            .graph
            .packages()
            .map(|(id, _)| id.source.as_str())
            .collect();
        sources.sort_unstable();
        sources.dedup();
        let bytes = self.graph.artifacts().filter_map(|(_, a)| a.bytes).sum();
        Totals {
            packages,
            sources: sources.len(),
            bytes,
        }
    }

    /// The counts the status strip shows, in the order it shows them.
    #[must_use]
    pub fn facets(&self) -> Vec<(&'static str, usize)> {
        use crate::model::question::Provenance;
        let (mut wanted, mut pulled, mut unexplained) = (0, 0, 0);
        for (id, _) in self.graph.packages() {
            match self.graph.why(id) {
                Provenance::Wanted => wanted += 1,
                Provenance::PulledIn(_) => pulled += 1,
                Provenance::Unexplained => unexplained += 1,
            }
        }
        let outdated = self.graph.packages().filter(|(_, p)| p.outdated).count();
        let broken = self
            .graph
            .artifacts()
            .filter(|(path, _)| self.graph.is_broken(path))
            .count();

        vec![
            ("wanted", wanted),
            ("pulled in", pulled),
            ("outdated", outdated),
            ("unexplained", unexplained),
            ("broken", broken),
        ]
    }

    /// Freshness, in the words a person uses.
    #[must_use]
    pub fn freshness(&self) -> String {
        if self.scanning {
            return "scanning".to_owned();
        }
        match self.scanned_ago {
            0..=5 => "scanned just now".to_owned(),
            s @ 6..=89 => format!("scanned {s}s ago"),
            s @ 90..=5399 => format!("scanned {}m ago", s / 60),
            s => format!("scanned {}h ago", s / 3600),
        }
    }
}

/// Whether two rows name the same thing, for keeping the cursor still.
fn same_thing(a: &Row, b: &Row) -> bool {
    match (a, b) {
        (Row::Group { key: x, .. }, Row::Group { key: y, .. }) => x == y,
        (Row::Item(x), Row::Item(y)) => x.name == y.name && x.source == y.source,
        _ => false,
    }
}

/// What the header counts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Totals {
    /// Packages every source knows about.
    pub packages: usize,
    /// How many sources reported anything.
    pub sources: usize,
    /// Bytes on disk, where anybody measured them.
    pub bytes: u64,
}

/// This machine's name, or a usable stand-in.
fn hostname() -> String {
    std::process::Command::new("hostname")
        .arg("-s")
        .output()
        .ok()
        .filter(|out| out.status.success())
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|name| name.trim().to_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "this machine".to_owned())
}

#[cfg(test)]
mod tests {
    use super::{App, Totals};
    use crate::model::fact::{Fact, PackageId};
    use crate::model::graph::Graph;
    use crate::view::row::Row;
    use std::path::PathBuf;

    fn machine() -> App {
        let rg = PackageId::new("homebrew", "ripgrep");
        let pcre = PackageId::new("homebrew", "pcre2");
        App::new(Graph::from_facts([
            Fact::Package {
                id: rg.clone(),
                version: Some("15.2.0".to_owned()),
            },
            Fact::Wanted {
                package: rg.clone(),
            },
            Fact::Owns {
                package: rg.clone(),
                artifact: PathBuf::from("/opt/homebrew/Cellar/ripgrep/15.2.0"),
            },
            Fact::Size {
                artifact: PathBuf::from("/opt/homebrew/Cellar/ripgrep/15.2.0"),
                bytes: 6_500_000,
            },
            Fact::DependsOn {
                package: rg,
                on: pcre.clone(),
                declared_directly: true,
            },
            Fact::Package {
                id: pcre.clone(),
                version: Some("10.48".to_owned()),
            },
            Fact::Outdated {
                package: pcre,
                latest: Some("10.49".to_owned()),
            },
            Fact::Missing {
                artifact: PathBuf::from("/opt/homebrew/bin/ghost"),
            },
        ]))
    }

    #[test]
    fn totals_count_packages_sources_and_bytes() {
        assert_eq!(
            machine().totals(),
            Totals {
                packages: 2,
                sources: 1,
                bytes: 6_500_000
            }
        );
    }

    #[test]
    fn the_facets_are_the_questions_worth_asking() {
        assert_eq!(
            machine().facets(),
            vec![
                ("wanted", 1),
                ("pulled in", 1),
                ("outdated", 1),
                ("unexplained", 0),
                ("broken", 1),
            ]
        );
    }

    #[test]
    fn freshness_is_said_the_way_a_person_says_it() {
        let mut app = machine();
        for (seconds, expected) in [
            (0, "scanned just now"),
            (30, "scanned 30s ago"),
            (120, "scanned 2m ago"),
            (7200, "scanned 2h ago"),
        ] {
            app.scanned_ago = seconds;
            assert_eq!(app.freshness(), expected);
        }
    }

    fn names(app: &App) -> Vec<String> {
        app.rows
            .iter()
            .map(|row| match row {
                Row::Group { key, .. } => format!("[{key}]"),
                Row::Item(item) => item.name.clone(),
            })
            .collect()
    }

    #[test]
    fn the_list_is_built_as_soon_as_there_is_a_machine() {
        assert_eq!(
            names(&machine()),
            vec!["[homebrew]", "pcre2", "ripgrep", "[unclaimed]", "ghost"],
            "the dangling symlink is unclaimed, and that is the point of showing it"
        );
    }

    #[test]
    fn the_cursor_stops_at_both_ends_rather_than_wrapping() {
        let mut app = machine();
        app.move_by(-1);
        assert_eq!(app.selected, 0, "already at the top");
        app.move_by(500);
        assert_eq!(
            app.selected,
            app.rows.len() - 1,
            "a keypress must not travel far"
        );
    }

    #[test]
    fn folding_a_group_from_inside_it_leaves_the_cursor_on_the_heading() {
        let mut app = machine();
        app.selected = 2; // ripgrep
        app.toggle_group();
        assert_eq!(names(&app), vec!["[homebrew]", "[unclaimed]", "ghost"]);
        assert_eq!(app.selected, 0, "the cursor follows the rows that vanished");
    }

    #[test]
    fn unfolding_puts_the_items_back() {
        let mut app = machine();
        app.toggle_group();
        app.toggle_group();
        assert_eq!(
            names(&app),
            vec!["[homebrew]", "pcre2", "ripgrep", "[unclaimed]", "ghost"]
        );
    }

    #[test]
    fn the_cursor_stays_on_the_same_package_when_the_list_is_rebuilt() {
        let mut app = machine();
        app.selected = 2;
        let before = names(&app)[2].clone();
        app.rebuild();
        assert_eq!(names(&app)[app.selected], before);
    }

    #[test]
    fn the_window_does_not_move_when_the_cursor_is_already_inside_it() {
        let mut app = machine();
        app.selected = 1;
        app.scroll_into_view(3);
        assert_eq!(app.offset, 0, "nothing should jump");
    }

    #[test]
    fn the_window_follows_the_cursor_off_either_end() {
        let mut app = machine();
        app.selected = 2;
        app.scroll_into_view(2);
        assert_eq!(app.offset, 1);
        app.selected = 0;
        app.scroll_into_view(2);
        assert_eq!(app.offset, 0);
    }

    #[test]
    fn a_scan_in_flight_says_so_rather_than_reporting_a_stale_time() {
        let mut app = machine();
        app.scanned_ago = 600;
        app.scanning = true;
        assert_eq!(app.freshness(), "scanning");
    }
}
