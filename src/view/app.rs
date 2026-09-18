//! What the interface is looking at.
//!
//! State only: what the machine is, where the cursor is, and what the person
//! has asked to see. No drawing and no terminal, so every question the
//! interface can answer is answerable in a test without one.

use std::collections::BTreeSet;

use crate::model::graph::Graph;
use crate::view::row::{Axis, Facet, Filter, Row, Sort, build};

/// The interface's whole state.
pub struct App {
    /// The machine.
    pub graph: Graph,
    /// The hostname, for the header.
    pub host: String,
    /// When the scan finished.
    pub scanned: std::time::SystemTime,
    /// What went wrong last time, if anything did.
    pub failure: Option<String>,
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
    /// What the list is grouped by.
    pub axis: Axis,
    /// What counts as now, for grouping by age. Injected so tests are stable.
    pub now: std::time::SystemTime,
    /// What the list is ordered by, within each group.
    pub sort: Sort,
    /// Whether that order is inverted.
    pub reversed: bool,
    /// What the list is narrowed to.
    pub filter: Filter,
    /// What keystrokes currently mean.
    pub mode: Mode,
    /// Whether detail is showing, and how far down it is scrolled.
    pub detail: Option<usize>,
    /// What was drawn where, for the frame now on screen.
    pub hits: crate::view::hit::Hits,
    /// Which row the pointer is over, if any.
    pub hovered: Option<usize>,
    /// Which palette is in force.
    pub theme: crate::view::theme::Theme,
}

/// What a keypress does right now.
///
/// While typing, letters go into the query rather than being commands — `s`
/// means the letter s, not sort.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    /// Keys are commands.
    #[default]
    Browsing,
    /// Keys are a query.
    Typing,
}

impl App {
    /// Look at this machine.
    #[must_use]
    pub fn new(graph: Graph) -> Self {
        Self::at(graph, std::time::SystemTime::now())
    }

    /// Look at a machine read at a particular time.
    ///
    /// The cache is older than now, and saying "scanned just now" over a
    /// two-hour-old answer is the one thing the header must never do.
    #[must_use]
    pub fn at(graph: Graph, scanned: std::time::SystemTime) -> Self {
        let now = std::time::SystemTime::now();
        let rows = build(
            &graph,
            &BTreeSet::new(),
            Axis::default(),
            Sort::default(),
            false,
            &Filter::default(),
            now,
        );
        Self {
            graph,
            host: hostname(),
            scanned,
            failure: None,
            scanning: false,
            quit: false,
            rows,
            collapsed: BTreeSet::new(),
            selected: 0,
            offset: 0,
            axis: Axis::default(),
            now,
            sort: Sort::default(),
            reversed: false,
            filter: Filter::default(),
            mode: Mode::default(),
            detail: None,
            hits: crate::view::hit::Hits::default(),
            hovered: None,
            theme: crate::view::theme::Theme::from_environment(),
        }
    }

    /// Read the machine again, keeping everything the person set up.
    ///
    /// The grouping, the sort, the filter, the folds and the cursor all survive:
    /// a refresh that resets the view is a refresh nobody presses twice.
    ///
    /// A source that fails leaves the previous answer standing. A worse machine
    /// is not an improvement on a stale one.
    pub fn rescan(&mut self, read: impl FnOnce() -> Result<crate::survey::Survey, String>) {
        self.scanning = true;
        match read() {
            Ok(survey) => {
                // A partial answer replaces a whole one, and says so.
                self.failure = survey.trouble();
                self.scanned = survey.scanned;
                self.graph = survey.graph;
            }
            // Only the walk failing gets here, and then there is no machine to
            // show at all, so the previous answer stands.
            Err(message) => self.failure = Some(message),
        }
        self.scanning = false;
        self.rebuild();
    }

    /// Rebuild the list, keeping the cursor on whatever it was pointing at.
    ///
    /// Collapsing a group moves every row after it, and a cursor that jumped to
    /// a different package each time would make the list unusable.
    pub fn rebuild(&mut self) {
        let anchor = self.rows.get(self.selected).cloned();
        self.rows = build(
            &self.graph,
            &self.collapsed,
            self.axis,
            self.sort,
            self.reversed,
            &self.filter,
            self.now,
        );
        self.selected = anchor
            .and_then(|was| self.rows.iter().position(|row| same_thing(row, &was)))
            .unwrap_or(self.selected)
            .min(self.rows.len().saturating_sub(1));
    }

    /// Group by the next axis round.
    ///
    /// Folds are per axis: a group folded under `source` means nothing under
    /// `size`, and carrying them across would hide rows for no reason.
    pub fn cycle_axis(&mut self) {
        self.axis = self.axis.next();
        self.collapsed.clear();
        self.rebuild();
    }

    /// Order by the next column round.
    ///
    /// Reversal is dropped, because a column's default direction is the one
    /// that reads best and carrying an inversion across columns surprises.
    pub fn cycle_sort(&mut self) {
        self.sort = self.sort.next();
        self.reversed = false;
        self.rebuild();
    }

    /// Turn the current order upside down.
    pub fn reverse_sort(&mut self) {
        self.reversed = !self.reversed;
        self.rebuild();
    }

    /// Turn a facet on, or off if it is already on.
    ///
    /// The cursor goes to the top: after narrowing to thirty rows from four
    /// hundred, wherever it was is not where you are looking.
    pub fn toggle_facet(&mut self, facet: Facet) {
        self.filter.facet = if self.filter.facet == Some(facet) {
            None
        } else {
            Some(facet)
        };
        self.selected = 0;
        self.offset = 0;
        self.rebuild();
    }

    /// Cycle through the facets from the keyboard, ending back at none.
    pub fn cycle_facet(&mut self) {
        let next = match self.filter.facet {
            None => Some(Facet::ALL[0]),
            Some(current) => {
                let index = Facet::ALL.iter().position(|f| *f == current).unwrap_or(0);
                Facet::ALL.get(index + 1).copied()
            }
        };
        self.filter.facet = next;
        self.selected = 0;
        self.offset = 0;
        self.rebuild();
    }

    /// The item the cursor is on, if it is on one.
    #[must_use]
    pub fn selected_item(&self) -> Option<&crate::view::row::Item> {
        match self.rows.get(self.selected)? {
            Row::Item(item) => Some(item),
            Row::Group { .. } => None,
        }
    }

    /// Show or hide the detail for whatever the cursor is on.
    ///
    /// A heading has no detail, so `↵` on one folds it instead — which is what
    /// it already did, and what a reader expects.
    pub fn toggle_detail(&mut self) {
        if self.detail.is_some() {
            self.detail = None;
        } else if self.selected_item().is_some() {
            self.detail = Some(0);
        } else {
            self.toggle_group();
        }
    }

    /// Put the cursor on a row the pointer chose.
    ///
    /// Clicking a heading folds it, the same as `space`, because that is what
    /// the arrow on it says it will do.
    pub fn click_row(&mut self, index: usize) {
        if index >= self.rows.len() {
            return;
        }
        self.selected = index;
        if matches!(self.rows.get(index), Some(Row::Group { .. })) {
            self.toggle_group();
        } else if self.detail.is_some() {
            // Opening detail on a click would cover the list somebody is
            // pointing at. Selecting is what a single click means; detail that
            // is already open simply follows.
            self.detail = Some(0);
        }
    }

    /// Scroll the detail, if it is showing.
    pub fn scroll_detail(&mut self, delta: isize) {
        if let Some(offset) = self.detail {
            self.detail = Some(offset.saturating_add_signed(delta));
        }
    }

    /// Start typing a query.
    pub fn start_typing(&mut self) {
        self.mode = Mode::Typing;
    }

    /// Stop typing, keeping whatever was typed.
    pub fn stop_typing(&mut self) {
        self.mode = Mode::Browsing;
    }

    /// Whether keystrokes are going into the query.
    #[must_use]
    pub fn is_typing(&self) -> bool {
        self.mode == Mode::Typing
    }

    /// Add a character to the query.
    pub fn push_query(&mut self, c: char) {
        self.filter.query.push(c);
        self.selected = 0;
        self.offset = 0;
        self.rebuild();
    }

    /// Remove the last character of the query.
    pub fn pop_query(&mut self) {
        self.filter.query.pop();
        self.selected = 0;
        self.offset = 0;
        self.rebuild();
    }

    /// Undo one narrowing, narrowest first.
    ///
    /// Returns whether anything was cleared, so `esc` can fall through to
    /// leaving once there is nothing left to undo.
    pub fn clear_one(&mut self) -> bool {
        if self.mode == Mode::Typing {
            self.mode = Mode::Browsing;
            return true;
        }
        if !self.filter.query.is_empty() {
            self.filter.query.clear();
        } else if self.filter.facet.is_some() {
            self.filter.facet = None;
        } else {
            return false;
        }
        self.selected = 0;
        self.offset = 0;
        self.rebuild();
        true
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
        // Detail follows the cursor rather than staying on what it was opened
        // for, and starts at the top of whatever it now describes.
        if self.detail.is_some() {
            self.detail = Some(0);
        }
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
        let (mut wanted, mut pulled, mut system, mut unexplained) = (0, 0, 0, 0);
        for (id, _) in self.graph.packages() {
            match self.graph.why(id) {
                Provenance::Wanted => wanted += 1,
                Provenance::PulledIn(_) => pulled += 1,
                Provenance::System => system += 1,
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
            // Last on purpose. A narrow strip drops facets from the end, and
            // the one nobody needs to act on is the one to lose first.
            ("system", system),
        ]
    }

    /// Freshness, in the words a person uses.
    ///
    /// Takes `now` rather than reading the clock, so the interface can render
    /// the same frame twice and get the same answer.
    #[must_use]
    pub fn freshness_at(&self, now: std::time::SystemTime) -> String {
        if self.scanning {
            return "scanning".to_owned();
        }
        let seconds = now.duration_since(self.scanned).map_or(0, |d| d.as_secs());
        match seconds {
            0..=5 => "scanned just now".to_owned(),
            s @ 6..=89 => format!("scanned {s}s ago"),
            s @ 90..=5399 => format!("scanned {}m ago", s / 60),
            s => format!("scanned {}h ago", s / 3600),
        }
    }

    /// Freshness now.
    #[must_use]
    pub fn freshness(&self) -> String {
        self.freshness_at(std::time::SystemTime::now())
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
                ("system", 0),
            ]
        );
    }

    #[test]
    fn freshness_is_said_the_way_a_person_says_it() {
        let app = machine();
        for (seconds, expected) in [
            (0, "scanned just now"),
            (30, "scanned 30s ago"),
            (120, "scanned 2m ago"),
            (7200, "scanned 2h ago"),
        ] {
            let later = app.scanned + std::time::Duration::from_secs(seconds);
            assert_eq!(app.freshness_at(later), expected);
        }
    }

    #[test]
    fn a_rescan_keeps_everything_the_person_set_up() {
        use crate::view::row::{Axis, Facet, Sort};
        let mut app = machine();
        app.axis = Axis::Role;
        app.sort = Sort::Size;
        app.filter.facet = Some(Facet::Wanted);
        app.collapsed.insert("wanted".to_owned());
        app.rebuild();

        app.rescan(|| {
            Ok(crate::survey::Survey {
                graph: machine().graph,
                ..Default::default()
            })
        });

        assert_eq!(
            app.axis,
            Axis::Role,
            "a refresh that resets the view is not pressed twice"
        );
        assert_eq!(app.sort, Sort::Size);
        assert_eq!(app.filter.facet, Some(Facet::Wanted));
        assert!(app.collapsed.contains("wanted"));
    }

    #[test]
    fn a_failed_rescan_leaves_the_previous_answer_standing() {
        let mut app = machine();
        let before = app.rows.len();
        app.rescan(|| Err("brew fell over".to_owned()));
        assert_eq!(
            app.rows.len(),
            before,
            "a worse machine is not better than a stale one"
        );
        assert_eq!(app.failure.as_deref(), Some("brew fell over"));
        assert!(!app.scanning, "it must not be left looking busy");
    }

    #[test]
    fn a_machine_read_two_hours_ago_does_not_claim_to_be_fresh() {
        let graph = machine().graph;
        let earlier = std::time::SystemTime::now() - std::time::Duration::from_secs(7200);
        let app = App::at(graph, earlier);
        assert_eq!(
            app.freshness(),
            "scanned 2h ago",
            "the one thing the header must never do is say `just now` over a memory"
        );
    }

    #[test]
    fn a_partial_rescan_replaces_the_machine_and_says_what_was_missed() {
        let mut app = machine();
        app.rescan(|| {
            Ok(crate::survey::Survey {
                graph: machine().graph,
                failures: vec![crate::model::fact::ScanError::new("cargo", "no")],
                ..Default::default()
            })
        });
        assert_eq!(
            app.failure.as_deref(),
            Some("cargo could not be read"),
            "seven eighths of a machine is worth having, and worth labelling"
        );
        assert!(!app.rows.is_empty(), "the rest of it still arrived");
    }

    #[test]
    fn a_rescan_that_works_clears_the_previous_failure() {
        let mut app = machine();
        app.rescan(|| Err("transient".to_owned()));
        app.rescan(|| {
            Ok(crate::survey::Survey {
                graph: machine().graph,
                ..Default::default()
            })
        });
        assert!(app.failure.is_none());
    }

    #[test]
    fn a_rescan_moves_the_clock_forward() {
        let mut app = machine();
        let before = app.scanned;
        std::thread::sleep(std::time::Duration::from_millis(5));
        app.rescan(|| {
            Ok(crate::survey::Survey {
                graph: machine().graph,
                ..Default::default()
            })
        });
        assert!(
            app.scanned > before,
            "otherwise it still reports the old freshness"
        );
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
        app.scanning = true;
        assert_eq!(app.freshness(), "scanning");
    }
}
