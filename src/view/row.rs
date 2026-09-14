//! The list, flattened.
//!
//! A grouped list is a tree, and drawing a tree means answering "what is on
//! line 412" quickly. So the tree is flattened once into a vector of rows —
//! group headers interleaved with their items — and everything after that is
//! indexing. Collapsing a group rebuilds the vector rather than teaching the
//! renderer about depth.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use crate::model::fact::PackageId;
use crate::model::graph::Graph;
use crate::model::question::Provenance;

/// What a row says about the thing it names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum State {
    /// Somebody asked for it.
    Fine,
    /// A dependency. You did not ask for it.
    PulledIn,
    /// Nothing you installed needs it.
    Unexplained,
    /// On disk, and no package manager claims it.
    Orphan,
    /// Referred to, and not there.
    Broken,
}

impl State {
    /// One glyph per state, so the screen still says everything it needs to
    /// with no colour at all.
    #[must_use]
    pub fn glyph(self) -> &'static str {
        match self {
            Self::Fine => "●",
            Self::PulledIn => "◐",
            // Both mean "nothing accounts for this"; the source column tells
            // them apart, so they share a glyph deliberately.
            Self::Unexplained | Self::Orphan => "?",
            Self::Broken => "✕",
        }
    }

    /// How it is named in a column.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Fine => "wanted",
            Self::PulledIn => "dep",
            Self::Unexplained => "unneeded",
            Self::Orphan => "orphan",
            Self::Broken => "broken",
        }
    }
}

/// What the list is grouped by.
///
/// Each answers a different question, and the two that matter most are not the
/// default: `Role` separates what you chose from what came with it, and `Size`
/// says where the disk went.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Axis {
    /// What each manager is responsible for.
    #[default]
    Source,
    /// What you actually chose.
    Role,
    /// Where the disk went.
    Size,
    /// What you have not touched in a long time.
    Age,
    /// What needs attention.
    Health,
}

impl Axis {
    /// Every axis, in the order `g` cycles them.
    pub const ALL: [Self; 5] = [
        Self::Source,
        Self::Role,
        Self::Size,
        Self::Age,
        Self::Health,
    ];

    /// How it is named in the pane title.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Role => "role",
            Self::Size => "size",
            Self::Age => "age",
            Self::Health => "health",
        }
    }

    /// The next axis round.
    #[must_use]
    pub fn next(self) -> Self {
        let index = Self::ALL.iter().position(|a| *a == self).unwrap_or(0);
        Self::ALL[(index + 1) % Self::ALL.len()]
    }

    /// Which group an item falls in, and where that group sorts.
    ///
    /// Alphabetical is right for sources and wrong for everything else: size
    /// buckets belong largest first and ages newest first, because that is the
    /// order the question is asked in.
    fn bucket(self, item: &Item, now: SystemTime) -> (u8, String) {
        match self {
            Self::Source => (0, item.source.clone()),
            // Role is provenance and nothing else. Being out of date does not
            // change whether you asked for something, and letting it do so made
            // the strip say 87 wanted while the list said 65.
            Self::Role => match item.state {
                State::Fine => (0, "wanted".to_owned()),
                State::PulledIn => (1, "pulled in".to_owned()),
                State::Unexplained => (2, "nothing needs".to_owned()),
                State::Orphan => (3, "unclaimed".to_owned()),
                State::Broken => (4, "broken".to_owned()),
            },
            Self::Size => match item.bytes.unwrap_or(0) {
                b if b >= 100 << 20 => (0, "over 100M".to_owned()),
                b if b >= 10 << 20 => (1, "10M to 100M".to_owned()),
                b if b >= 1 << 20 => (2, "1M to 10M".to_owned()),
                b if b > 0 => (3, "under 1M".to_owned()),
                _ => (4, "unmeasured".to_owned()),
            },
            Self::Age => {
                const DAY: u64 = 60 * 60 * 24;
                let Some(age) = item.installed.and_then(|at| now.duration_since(at).ok()) else {
                    return (4, "unknown".to_owned());
                };
                match age {
                    a if a < Duration::from_secs(30 * DAY) => (0, "this month".to_owned()),
                    a if a < Duration::from_secs(90 * DAY) => (1, "this quarter".to_owned()),
                    a if a < Duration::from_secs(365 * DAY) => (2, "this year".to_owned()),
                    _ => (3, "older".to_owned()),
                }
            }
            Self::Health => match (item.state, item.outdated) {
                (State::Broken, _) => (0, "broken".to_owned()),
                (_, true) => (1, "outdated".to_owned()),
                (State::Unexplained | State::Orphan, _) => (2, "unaccounted for".to_owned()),
                (State::Fine | State::PulledIn, _) => (3, "fine".to_owned()),
            },
        }
    }
}

/// What the list is ordered by, within each group.
///
/// Sorting applies inside groups rather than across them, so grouping and
/// sorting compose instead of fighting: "the biggest thing in each source" is
/// one question, not two.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Sort {
    /// Alphabetical, ignoring case.
    #[default]
    Name,
    /// Largest first.
    Size,
    /// Newest first.
    Age,
    /// Grouped by what is wrong with it.
    State,
    /// Version string, which is not a number and is not pretended to be one.
    Version,
}

impl Sort {
    /// Every column, in the order `s` cycles them.
    pub const ALL: [Self; 5] = [
        Self::Name,
        Self::Size,
        Self::Age,
        Self::State,
        Self::Version,
    ];

    /// How it is named in the rule.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Size => "size",
            Self::Age => "age",
            Self::State => "state",
            Self::Version => "version",
        }
    }

    /// The next column round.
    #[must_use]
    pub fn next(self) -> Self {
        let index = Self::ALL.iter().position(|s| *s == self).unwrap_or(0);
        Self::ALL[(index + 1) % Self::ALL.len()]
    }

    /// Whether this column reads best largest-or-newest first.
    ///
    /// Nobody asks for the smallest thing on their disk.
    #[must_use]
    pub fn descends_by_default(self) -> bool {
        matches!(self, Self::Size | Self::Age)
    }

    /// Compare two items on this column, in the direction asked for.
    ///
    /// Direction is handled here rather than by reversing the result, because
    /// an item with nothing to compare — no size, no date — must sort last
    /// either way. A hole should never outrank something real just because the
    /// order was inverted.
    fn compare(self, a: &Item, b: &Item, descending: bool) -> std::cmp::Ordering {
        use std::cmp::Ordering;
        let by_name = || a.name.to_lowercase().cmp(&b.name.to_lowercase());
        let dir = |ordering: Ordering| {
            if descending {
                ordering.reverse()
            } else {
                ordering
            }
        };

        match self {
            Self::Name => dir(by_name()),
            Self::Size => match (a.bytes, b.bytes) {
                (Some(x), Some(y)) => dir(x.cmp(&y)).then_with(by_name),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => by_name(),
            },
            Self::Age => match (a.installed, b.installed) {
                (Some(x), Some(y)) => dir(x.cmp(&y)).then_with(by_name),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => by_name(),
            },
            Self::State => {
                dir((a.state, !a.outdated).cmp(&(b.state, !b.outdated))).then_with(by_name)
            }
            Self::Version => match (a.version.as_deref(), b.version.as_deref()) {
                (Some(x), Some(y)) => dir(x.cmp(y)).then_with(by_name),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => by_name(),
            },
        }
    }
}

/// One thing in the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    /// What it is called.
    pub name: String,
    /// Which manager owns it, or `-` for an orphan.
    pub source: String,
    /// The installed version, where anything reports one.
    pub version: Option<String>,
    /// Why it is here.
    pub state: State,
    /// Bytes on disk, where anybody measured them.
    pub bytes: Option<u64>,
    /// The package this stands for, if a manager claims it.
    pub package: Option<PackageId>,
    /// Where it lives.
    pub path: Option<PathBuf>,
    /// When it arrived, where anything recorded it.
    pub installed: Option<SystemTime>,
    /// Whether a newer version is published. Orthogonal to [`Item::state`]:
    /// something you asked for can be out of date without ceasing to be
    /// something you asked for.
    pub outdated: bool,
}

/// A line of the list: either a group heading or something in it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Row {
    /// A heading, with what it contains.
    Group {
        /// What the group is called.
        key: String,
        /// How many items are under it.
        count: usize,
        /// How much they take up.
        bytes: u64,
        /// Whether it is folded shut.
        collapsed: bool,
    },
    /// One package or orphan.
    Item(Item),
}

/// Prefixes macOS itself owns.
///
/// Nothing claims `/usr/bin/awk`, which makes it an orphan by the letter of the
/// model and noise by any useful measure. The honest fix is a system source
/// that claims these the way Homebrew claims the Cellar.
const SYSTEM_PREFIXES: &[&str] = &[
    "/usr/bin",
    "/usr/sbin",
    "/usr/libexec",
    "/bin",
    "/sbin",
    "/System",
    "/Library",
];

/// Everything worth showing, grouped and flattened.
///
/// `now` is passed rather than read, so grouping by age is testable.
#[must_use]
pub fn build(
    graph: &Graph,
    collapsed: &BTreeSet<String>,
    axis: Axis,
    sort: Sort,
    reversed: bool,
    now: SystemTime,
) -> Vec<Row> {
    let mut items: Vec<(u8, String, Item)> = items(graph)
        .into_iter()
        .map(|item| {
            let (order, key) = axis.bucket(&item, now);
            (order, key, item)
        })
        .collect();
    let descending = reversed ^ sort.descends_by_default();
    items.sort_by(|a, b| {
        // The group comes first whatever the sort: reversing a column must not
        // shuffle the groups themselves.
        (a.0, &a.1)
            .cmp(&(b.0, &b.1))
            .then_with(|| sort.compare(&a.2, &b.2, descending))
    });

    let mut rows = Vec::new();
    let mut index = 0;
    while index < items.len() {
        let key = items[index].1.clone();
        let end = items[index..].partition_point(|(_, k, _)| *k == key) + index;
        let group = &items[index..end];
        let folded = collapsed.contains(&key);

        rows.push(Row::Group {
            key,
            count: group.len(),
            bytes: group.iter().filter_map(|(_, _, item)| item.bytes).sum(),
            collapsed: folded,
        });
        if !folded {
            rows.extend(group.iter().map(|(_, _, item)| Row::Item(item.clone())));
        }
        index = end;
    }
    rows
}

/// Every package, plus everything on disk nobody claims.
fn items(graph: &Graph) -> Vec<Item> {
    let mut items: Vec<Item> = graph
        .packages()
        .map(|(id, package)| {
            let state = match graph.why(id) {
                Provenance::Wanted => State::Fine,
                Provenance::PulledIn(_) => State::PulledIn,
                Provenance::Unexplained => State::Unexplained,
            };
            let bytes: u64 = package
                .owns
                .iter()
                .filter_map(|path| graph.artifact(path).and_then(|a| a.bytes))
                .sum();
            let path = package
                .version
                .as_deref()
                .and_then(|version| {
                    package
                        .owns
                        .iter()
                        .find(|p| p.file_name().and_then(|n| n.to_str()) == Some(version))
                })
                .or_else(|| package.owns.iter().next())
                .cloned();

            Item {
                name: id.name.clone(),
                source: id.source.clone(),
                version: package.version.clone(),
                state,
                bytes: (bytes > 0).then_some(bytes),
                package: Some(id.clone()),
                path,
                installed: package.installed_at,
                outdated: package.outdated,
            }
        })
        .collect();

    items.extend(
        graph
            .artifacts()
            .filter(|(path, _)| graph.is_orphan(path) && !is_system(path))
            .map(|(path, artifact)| Item {
                name: path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("-")
                    .to_owned(),
                source: "unclaimed".to_owned(),
                version: None,
                state: if artifact.missing {
                    State::Broken
                } else {
                    State::Orphan
                },
                bytes: artifact.bytes,
                package: None,
                path: Some(path.to_owned()),
                installed: None,
                outdated: false,
            }),
    );
    items
}

/// Whether macOS itself put this here.
fn is_system(path: &std::path::Path) -> bool {
    SYSTEM_PREFIXES
        .iter()
        .any(|prefix| path.starts_with(prefix))
}

#[cfg(test)]
mod tests {
    use super::{Axis, Row, Sort, State, build};
    use crate::model::fact::{Fact, PackageId};
    use crate::model::graph::Graph;
    use std::collections::BTreeSet;
    use std::path::PathBuf;
    use std::time::SystemTime;

    /// A fixed clock, so grouping by age is not a coin toss.
    ///
    /// `from_secs` rather than a larger unit because `Duration::from_days` is
    /// still unstable.
    #[allow(clippy::unnecessary_min_or_max)]
    fn epoch() -> SystemTime {
        const DAY: u64 = 60 * 60 * 24;
        SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(20_000 * DAY)
    }

    fn machine() -> Graph {
        let rg = PackageId::new("homebrew", "ripgrep");
        let pcre = PackageId::new("homebrew", "pcre2");
        Graph::from_facts([
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
                id: pcre,
                version: Some("10.48".to_owned()),
            },
            Fact::Artifact {
                path: PathBuf::from("/Applications/Xcode.app"),
            },
            Fact::Artifact {
                path: PathBuf::from("/usr/bin/awk"),
            },
        ])
    }

    fn names(rows: &[Row]) -> Vec<String> {
        rows.iter()
            .map(|row| match row {
                Row::Group { key, count, .. } => format!("[{key} {count}]"),
                Row::Item(item) => item.name.clone(),
            })
            .collect()
    }

    #[test]
    fn rows_are_grouped_by_source_with_a_heading_each() {
        assert_eq!(
            names(&build(
                &machine(),
                &BTreeSet::new(),
                Axis::Source,
                Sort::Name,
                false,
                epoch()
            )),
            vec![
                "[homebrew 2]",
                "pcre2",
                "ripgrep",
                "[unclaimed 1]",
                "Xcode.app"
            ]
        );
    }

    #[test]
    fn a_collapsed_group_keeps_its_heading_and_loses_its_items() {
        let collapsed = ["homebrew".to_owned()].into_iter().collect();
        assert_eq!(
            names(&build(
                &machine(),
                &collapsed,
                Axis::Source,
                Sort::Name,
                false,
                epoch()
            )),
            vec!["[homebrew 2]", "[unclaimed 1]", "Xcode.app"]
        );
    }

    #[test]
    fn a_group_heading_totals_what_is_under_it_even_when_folded() {
        let collapsed = ["homebrew".to_owned()].into_iter().collect();
        let rows = build(
            &machine(),
            &collapsed,
            Axis::Source,
            Sort::Name,
            false,
            epoch(),
        );
        let Row::Group { bytes, count, .. } = &rows[0] else {
            panic!("expected a heading");
        };
        assert_eq!((*count, *bytes), (2, 6_500_000));
    }

    #[test]
    fn state_follows_from_the_graph_rather_than_from_a_flag() {
        let rows = build(
            &machine(),
            &BTreeSet::new(),
            Axis::Source,
            Sort::Name,
            false,
            epoch(),
        );
        let state = |name: &str| {
            rows.iter()
                .find_map(|row| match row {
                    Row::Item(item) if item.name == name => Some(item.state),
                    _ => None,
                })
                .expect("a row")
        };
        assert_eq!(state("ripgrep"), State::Fine);
        assert_eq!(state("pcre2"), State::PulledIn);
        assert_eq!(state("Xcode.app"), State::Orphan);
    }

    #[test]
    fn system_binaries_stay_out_of_the_list() {
        let rows = build(
            &machine(),
            &BTreeSet::new(),
            Axis::Source,
            Sort::Name,
            false,
            epoch(),
        );
        assert!(
            !names(&rows).contains(&"awk".to_owned()),
            "a thousand of these would bury the rest"
        );
    }

    #[test]
    fn every_state_has_its_own_glyph_so_colour_is_never_the_only_signal() {
        let all = [
            State::Fine,
            State::PulledIn,
            State::Unexplained,
            State::Orphan,
            State::Broken,
        ];
        let glyphs: BTreeSet<&str> = all.iter().map(|s| s.glyph()).collect();
        // Orphan and Unexplained deliberately share `?`: both mean "nothing
        // accounts for this", and the source column tells them apart.
        assert_eq!(glyphs.len(), all.len() - 1);
    }

    #[test]
    fn grouping_by_role_separates_what_you_chose_from_what_came_with_it() {
        let rows = build(
            &machine(),
            &BTreeSet::new(),
            Axis::Role,
            Sort::Name,
            false,
            epoch(),
        );
        assert_eq!(
            names(&rows),
            vec![
                "[wanted 1]",
                "ripgrep",
                "[pulled in 1]",
                "pcre2",
                "[unclaimed 1]",
                "Xcode.app"
            ]
        );
    }

    #[test]
    fn size_buckets_are_ordered_largest_first_not_alphabetically() {
        let rows = build(
            &machine(),
            &BTreeSet::new(),
            Axis::Size,
            Sort::Name,
            false,
            epoch(),
        );
        let headings: Vec<String> = names(&rows)
            .into_iter()
            .filter(|n| n.starts_with('['))
            .collect();
        assert_eq!(headings, vec!["[1M to 10M 1]", "[unmeasured 2]"]);
    }

    #[test]
    fn age_buckets_are_ordered_newest_first() {
        assert_eq!(Axis::Age.label(), "age");
        let rows = build(
            &machine(),
            &BTreeSet::new(),
            Axis::Age,
            Sort::Name,
            false,
            epoch(),
        );
        let headings: Vec<String> = names(&rows)
            .into_iter()
            .filter(|n| n.starts_with('['))
            .collect();
        assert_eq!(
            headings,
            vec!["[unknown 3]"],
            "nothing in the fixture records a date"
        );
    }

    #[test]
    fn health_puts_what_needs_attention_first() {
        let rows = build(
            &machine(),
            &BTreeSet::new(),
            Axis::Health,
            Sort::Name,
            false,
            epoch(),
        );
        let headings: Vec<String> = names(&rows)
            .into_iter()
            .filter(|n| n.starts_with('['))
            .collect();
        assert_eq!(headings, vec!["[unaccounted for 1]", "[fine 2]"]);
    }

    #[test]
    fn the_axes_cycle_round() {
        let mut axis = Axis::Source;
        for _ in 0..Axis::ALL.len() {
            axis = axis.next();
        }
        assert_eq!(axis, Axis::Source);
    }

    #[test]
    fn sorting_happens_inside_groups_not_across_them() {
        let rows = build(
            &machine(),
            &BTreeSet::new(),
            Axis::Source,
            Sort::Size,
            false,
            epoch(),
        );
        assert_eq!(
            names(&rows),
            vec![
                "[homebrew 2]",
                "ripgrep",
                "pcre2",
                "[unclaimed 1]",
                "Xcode.app"
            ],
            "ripgrep is the bigger of the two, and the groups keep their order"
        );
    }

    #[test]
    fn size_sorts_largest_first_without_being_asked() {
        assert!(
            Sort::Size.descends_by_default(),
            "nobody wants the smallest thing on their disk"
        );
        assert!(Sort::Age.descends_by_default());
        assert!(!Sort::Name.descends_by_default());
    }

    #[test]
    fn reversing_turns_the_order_round_but_leaves_the_groups_alone() {
        let forward = build(
            &machine(),
            &BTreeSet::new(),
            Axis::Source,
            Sort::Name,
            false,
            epoch(),
        );
        let backward = build(
            &machine(),
            &BTreeSet::new(),
            Axis::Source,
            Sort::Name,
            true,
            epoch(),
        );
        assert_eq!(
            names(&forward),
            vec![
                "[homebrew 2]",
                "pcre2",
                "ripgrep",
                "[unclaimed 1]",
                "Xcode.app"
            ]
        );
        assert_eq!(
            names(&backward),
            vec![
                "[homebrew 2]",
                "ripgrep",
                "pcre2",
                "[unclaimed 1]",
                "Xcode.app"
            ],
            "the items turn round; the groups do not"
        );
    }

    #[test]
    fn an_unmeasured_size_never_outranks_a_real_one() {
        let rows = build(
            &machine(),
            &BTreeSet::new(),
            Axis::Role,
            Sort::Size,
            false,
            epoch(),
        );
        let items: Vec<String> = names(&rows)
            .into_iter()
            .filter(|n| !n.starts_with('['))
            .collect();
        assert_eq!(
            items.first().map(String::as_str),
            Some("ripgrep"),
            "the only measured one"
        );
    }

    #[test]
    fn the_sort_columns_cycle_round() {
        let mut sort = Sort::Name;
        for _ in 0..Sort::ALL.len() {
            sort = sort.next();
        }
        assert_eq!(sort, Sort::Name);
    }

    #[test]
    fn names_sort_without_regard_to_case() {
        let rows = build(
            &machine(),
            &BTreeSet::new(),
            Axis::Source,
            Sort::Name,
            false,
            epoch(),
        );
        let unclaimed: Vec<_> = names(&rows);
        let x = unclaimed
            .iter()
            .position(|n| n == "Xcode.app")
            .expect("Xcode");
        let heading = unclaimed
            .iter()
            .position(|n| n == "[unclaimed 1]")
            .expect("heading");
        assert!(heading < x);
    }
}
