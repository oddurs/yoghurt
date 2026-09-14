//! The list, flattened.
//!
//! A grouped list is a tree, and drawing a tree means answering "what is on
//! line 412" quickly. So the tree is flattened once into a vector of rows —
//! group headers interleaved with their items — and everything after that is
//! indexing. Collapsing a group rebuilds the vector rather than teaching the
//! renderer about depth.

use std::collections::{BTreeMap, BTreeSet};
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
    /// What a thing actually is.
    Category,
}

impl Axis {
    /// Every axis, in the order `g` cycles them.
    pub const ALL: [Self; 6] = [
        Self::Source,
        Self::Role,
        Self::Category,
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
            Self::Category => "category",
        }
    }

    /// The axis with this name, if there is one.
    #[must_use]
    pub fn from_label(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|axis| axis.label() == name)
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
            Self::Category => match item.category() {
                Category::Application => (0, "applications".to_owned()),
                Category::Tool => (1, "tools".to_owned()),
                Category::Library => (2, "libraries".to_owned()),
                Category::Unclaimed => (3, "unclaimed".to_owned()),
            },
            Self::Health => match (item.state, item.outdated) {
                (State::Broken, _) => (0, "broken".to_owned()),
                (_, true) => (1, "outdated".to_owned()),
                (State::Unexplained | State::Orphan, _) => (2, "unaccounted for".to_owned()),
                (State::Fine | State::PulledIn, _) => (3, "fine".to_owned()),
            },
        }
    }
}

/// A narrowing of the list.
///
/// A facet and a typed query are the same kind of thing — both say "show me
/// fewer rows" — so they are one value that composes, rather than two that have
/// to be reconciled every time either changes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Filter {
    /// A named subset, chosen from the strip.
    pub facet: Option<Facet>,
    /// What the person typed.
    pub query: String,
}

impl Filter {
    /// Whether this narrows anything at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.facet.is_none() && self.query.is_empty()
    }

    /// Whether an item survives it.
    #[must_use]
    pub fn matches(&self, item: &Item) -> bool {
        self.facet.is_none_or(|facet| facet.matches(item)) && self.matches_query(item)
    }

    /// Match on name, source, and the commands it provides.
    ///
    /// Case-insensitive and substring rather than prefix: people remember the
    /// middle of a name as often as the start.
    fn matches_query(&self, item: &Item) -> bool {
        if self.query.is_empty() {
            return true;
        }
        let needle = self.query.to_lowercase();
        let has = |text: &str| text.to_lowercase().contains(&needle);

        has(&item.name)
            || has(&item.source)
            || item.provides.iter().any(|command| has(command))
            // So that `video` finds the codecs and `figma` finds what signed it.
            || item.describes.as_deref().is_some_and(has)
    }

    /// How the narrowing is described in the rule.
    #[must_use]
    pub fn describe(&self) -> String {
        match (self.facet, self.query.as_str()) {
            (None, "") => String::new(),
            (Some(facet), "") => facet.label().to_owned(),
            (None, query) => format!("/{query}"),
            (Some(facet), query) => format!("{} /{query}", facet.label()),
        }
    }
}

/// One of the named subsets in the status strip.
///
/// Every count on the strip is one of these, so the summary is the navigation:
/// the commonest question a person has costs one keystroke or one click.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Facet {
    /// Things you asked for.
    Wanted,
    /// Things that came with something else.
    PulledIn,
    /// Things with a newer version published.
    Outdated,
    /// Things nothing you installed needs.
    Unexplained,
    /// Things that are not there.
    Broken,
}

impl Facet {
    /// Every facet, in the order the strip shows them.
    pub const ALL: [Self; 5] = [
        Self::Wanted,
        Self::PulledIn,
        Self::Outdated,
        Self::Unexplained,
        Self::Broken,
    ];

    /// How it is named on the strip.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Wanted => "wanted",
            Self::PulledIn => "pulled in",
            Self::Outdated => "outdated",
            Self::Unexplained => "unexplained",
            Self::Broken => "broken",
        }
    }

    /// The facet with this name, if there is one.
    #[must_use]
    pub fn from_label(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|facet| facet.label() == name)
    }

    /// Whether an item belongs to it.
    #[must_use]
    pub fn matches(self, item: &Item) -> bool {
        match self {
            Self::Wanted => item.state == State::Fine,
            Self::PulledIn => item.state == State::PulledIn,
            Self::Outdated => item.outdated,
            Self::Unexplained => item.state == State::Unexplained,
            Self::Broken => item.state == State::Broken,
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

    /// The column with this name, if there is one.
    #[must_use]
    pub fn from_label(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|sort| sort.label() == name)
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

/// What a thing is, as opposed to where it came from.
///
/// Derived from the graph — what a package owns and what it puts on the path —
/// rather than from a list somebody has to keep up to date. A package manager
/// this program has never heard of is categorised correctly as soon as its
/// adapter lands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Category {
    /// An application bundle, whoever installed it.
    Application,
    /// Something you can run: it puts at least one command on your path.
    Tool,
    /// Something other things link against, and you never invoke.
    Library,
    /// On disk, and nothing claims it.
    Unclaimed,
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
    /// The command names it puts on the path, so a filter can find a package by
    /// what you actually type.
    pub provides: Vec<String>,
    /// What it is for, where a source says so.
    pub describes: Option<String>,
}

impl Item {
    /// What this is.
    ///
    /// Order matters: a cask that installs an app is an application even though
    /// it also puts a command on the path, because the app is the thing you
    /// think of it as.
    #[must_use]
    pub fn category(&self) -> Category {
        if self
            .path
            .as_ref()
            .is_some_and(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("app")))
        {
            return Category::Application;
        }
        if self.package.is_none() {
            return Category::Unclaimed;
        }
        if self.provides.is_empty() {
            Category::Library
        } else {
            Category::Tool
        }
    }
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
    filter: &Filter,
    now: SystemTime,
) -> Vec<Row> {
    let mut items: Vec<(u8, String, Item)> = items(graph)
        .into_iter()
        .filter(|item| filter.matches(item))
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

/// Which commands each package is responsible for.
///
/// Attribution runs from the command to the owner, not from the package
/// outward. A `Provides` fact sits on the path the walk found — the link at
/// `/opt/homebrew/bin/rg` — while the package owns the keg behind it, so asking
/// a package what it owns and reading `provides` off that finds nothing.
fn commands_by_package(graph: &Graph) -> BTreeMap<PackageId, Vec<String>> {
    let mut by_package: BTreeMap<PackageId, Vec<String>> = BTreeMap::new();
    for (path, artifact) in graph.artifacts() {
        if artifact.provides.is_empty() {
            continue;
        }
        for owner in graph.owners_of(path) {
            by_package
                .entry(owner.clone())
                .or_default()
                .extend(artifact.provides.iter().cloned());
        }
    }
    for commands in by_package.values_mut() {
        commands.sort_unstable();
        commands.dedup();
    }
    by_package
}

/// Every package, plus everything on disk nobody claims.
fn items(graph: &Graph) -> Vec<Item> {
    let commands = commands_by_package(graph);
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
                provides: commands.get(id).cloned().unwrap_or_default(),
                describes: package.describes.clone(),
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
                provides: artifact.provides.iter().cloned().collect(),
                describes: None,
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
    use super::{Axis, Category, Facet, Filter, Item, Row, Sort, State, build};
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
                &Filter::default(),
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
                &Filter::default(),
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
            &Filter::default(),
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
            &Filter::default(),
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
            &Filter::default(),
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
            &Filter::default(),
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
            &Filter::default(),
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
            &Filter::default(),
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
            &Filter::default(),
            epoch(),
        );
        let headings: Vec<String> = names(&rows)
            .into_iter()
            .filter(|n| n.starts_with('['))
            .collect();
        assert_eq!(headings, vec!["[unaccounted for 1]", "[fine 2]"]);
    }

    #[test]
    fn every_axis_is_reachable_by_cycling() {
        // Cycling ALL.len() times returns to the start whether or not an axis
        // is missing from ALL, so counting is not enough: this collects what is
        // actually reached. Without it, `Category` was unreachable from `g` and
        // every test still passed.
        let mut axis = Axis::Source;
        let mut seen = vec![axis];
        for _ in 0..Axis::ALL.len() {
            axis = axis.next();
            if !seen.contains(&axis) {
                seen.push(axis);
            }
        }
        assert_eq!(axis, Axis::Source, "and it comes back round");
        for expected in Axis::ALL {
            assert!(
                seen.contains(&expected),
                "{} is unreachable from g",
                expected.label()
            );
        }
    }

    #[test]
    fn every_sort_column_is_reachable_by_cycling() {
        let mut sort = Sort::Name;
        let mut seen = vec![sort];
        for _ in 0..Sort::ALL.len() {
            sort = sort.next();
            if !seen.contains(&sort) {
                seen.push(sort);
            }
        }
        assert_eq!(sort, Sort::Name);
        for expected in Sort::ALL {
            assert!(
                seen.contains(&expected),
                "{} is unreachable from s",
                expected.label()
            );
        }
    }

    #[test]
    fn sorting_happens_inside_groups_not_across_them() {
        let rows = build(
            &machine(),
            &BTreeSet::new(),
            Axis::Source,
            Sort::Size,
            false,
            &Filter::default(),
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
            &Filter::default(),
            epoch(),
        );
        let backward = build(
            &machine(),
            &BTreeSet::new(),
            Axis::Source,
            Sort::Name,
            true,
            &Filter::default(),
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
            &Filter::default(),
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
    fn a_facet_narrows_the_list_to_one_kind_of_thing() {
        let filter = Filter {
            facet: Some(Facet::PulledIn),
            query: String::new(),
        };
        let rows = build(
            &machine(),
            &BTreeSet::new(),
            Axis::Source,
            Sort::Name,
            false,
            &filter,
            epoch(),
        );
        assert_eq!(names(&rows), vec!["[homebrew 1]", "pcre2"]);
    }

    #[test]
    fn a_query_matches_the_middle_of_a_name_not_just_the_start() {
        let filter = Filter {
            facet: None,
            query: "grep".to_owned(),
        };
        let rows = build(
            &machine(),
            &BTreeSet::new(),
            Axis::Source,
            Sort::Name,
            false,
            &filter,
            epoch(),
        );
        assert_eq!(names(&rows), vec!["[homebrew 1]", "ripgrep"]);
    }

    #[test]
    fn a_facet_and_a_query_compose_rather_than_replacing_each_other() {
        let filter = Filter {
            facet: Some(Facet::Wanted),
            query: "grep".to_owned(),
        };
        let rows = build(
            &machine(),
            &BTreeSet::new(),
            Axis::Source,
            Sort::Name,
            false,
            &filter,
            epoch(),
        );
        assert_eq!(names(&rows), vec!["[homebrew 1]", "ripgrep"]);

        let missing = Filter {
            facet: Some(Facet::PulledIn),
            query: "grep".to_owned(),
        };
        let none = build(
            &machine(),
            &BTreeSet::new(),
            Axis::Source,
            Sort::Name,
            false,
            &missing,
            epoch(),
        );
        assert!(
            none.is_empty(),
            "ripgrep is wanted, so it is not also pulled in"
        );
    }

    #[test]
    fn a_query_matches_what_a_package_says_it_is_for() {
        let graph = Graph::from_facts([
            Fact::Package {
                id: PackageId::new("homebrew", "ripgrep"),
                version: Some("15.2.0".to_owned()),
            },
            Fact::Wanted {
                package: PackageId::new("homebrew", "ripgrep"),
            },
            Fact::Describes {
                package: PackageId::new("homebrew", "ripgrep"),
                text: "Search tool like grep and The Silver Searcher".to_owned(),
            },
        ]);
        let filter = Filter {
            facet: None,
            query: "silver searcher".to_owned(),
        };
        let rows = build(
            &graph,
            &BTreeSet::new(),
            Axis::Source,
            Sort::Name,
            false,
            &filter,
            epoch(),
        );
        assert_eq!(
            names(&rows),
            vec!["[homebrew 1]", "ripgrep"],
            "a description is how you find something whose name you do not know"
        );
    }

    #[test]
    fn a_query_is_case_insensitive() {
        for query in ["RIPGREP", "RipGrep", "ripgrep"] {
            let filter = Filter {
                facet: None,
                query: query.to_owned(),
            };
            let rows = build(
                &machine(),
                &BTreeSet::new(),
                Axis::Source,
                Sort::Name,
                false,
                &filter,
                epoch(),
            );
            assert_eq!(names(&rows), vec!["[homebrew 1]", "ripgrep"], "{query}");
        }
    }

    #[test]
    fn the_narrowing_describes_itself_for_the_rule() {
        assert_eq!(Filter::default().describe(), "");
        assert_eq!(
            Filter {
                facet: Some(Facet::Broken),
                query: String::new()
            }
            .describe(),
            "broken"
        );
        assert_eq!(
            Filter {
                facet: None,
                query: "rg".to_owned()
            }
            .describe(),
            "/rg"
        );
        assert_eq!(
            Filter {
                facet: Some(Facet::Wanted),
                query: "rg".to_owned()
            }
            .describe(),
            "wanted /rg"
        );
    }

    fn item(name: &str, provides: &[&str], path: &str, owned: bool) -> Item {
        Item {
            name: name.to_owned(),
            source: "homebrew".to_owned(),
            version: None,
            state: State::Fine,
            bytes: None,
            package: owned.then(|| PackageId::new("homebrew", name)),
            path: (!path.is_empty()).then(|| PathBuf::from(path)),
            installed: None,
            outdated: false,
            provides: provides.iter().map(|c| (*c).to_owned()).collect(),
            describes: None,
        }
    }

    #[test]
    fn something_you_can_run_is_a_tool_and_something_you_cannot_is_a_library() {
        assert_eq!(
            item("ripgrep", &["rg"], "/k/ripgrep", true).category(),
            Category::Tool
        );
        assert_eq!(
            item("pcre2", &[], "/k/pcre2", true).category(),
            Category::Library
        );
    }

    #[test]
    fn an_app_bundle_is_an_application_whoever_installed_it() {
        assert_eq!(
            item("Ghostty.app", &[], "/Applications/Ghostty.app", true).category(),
            Category::Application
        );
        assert_eq!(
            item("Xcode.app", &[], "/Applications/Xcode.app", false).category(),
            Category::Application,
            "nobody owning it does not stop it being an application"
        );
    }

    #[test]
    fn a_cask_that_installs_an_app_is_an_application_even_though_it_also_provides_a_command() {
        assert_eq!(
            item("orbstack", &["orb"], "/Applications/OrbStack.app", true).category(),
            Category::Application,
            "the app is the thing you think of it as"
        );
    }

    #[test]
    fn something_nothing_claims_is_unclaimed() {
        assert_eq!(
            item("mystery", &["mystery"], "/usr/local/bin/mystery", false).category(),
            Category::Unclaimed
        );
    }

    #[test]
    fn grouping_by_category_puts_applications_first_and_unclaimed_last() {
        let rows = build(
            &machine(),
            &BTreeSet::new(),
            Axis::Category,
            Sort::Name,
            false,
            &Filter::default(),
            epoch(),
        );
        let headings: Vec<String> = names(&rows)
            .into_iter()
            .filter(|n| n.starts_with('['))
            .collect();
        assert_eq!(headings, vec!["[applications 1]", "[libraries 2]"]);
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
            &Filter::default(),
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
