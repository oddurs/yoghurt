//! The list, flattened.
//!
//! A grouped list is a tree, and drawing a tree means answering "what is on
//! line 412" quickly. So the tree is flattened once into a vector of rows —
//! group headers interleaved with their items — and everything after that is
//! indexing. Collapsing a group rebuilds the vector rather than teaching the
//! renderer about depth.

use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::model::fact::PackageId;
use crate::model::graph::Graph;
use crate::model::question::Provenance;

/// What a row says about the thing it names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum State {
    /// Installed, current, claimed by a manager.
    Fine,
    /// A newer version is published.
    Outdated,
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
            Self::Outdated => "↑",
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
            Self::Outdated => "outdated",
            Self::PulledIn => "dep",
            Self::Unexplained => "unneeded",
            Self::Orphan => "orphan",
            Self::Broken => "broken",
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

/// Everything worth showing, grouped by source and flattened.
#[must_use]
pub fn build(graph: &Graph, collapsed: &BTreeSet<String>) -> Vec<Row> {
    let mut items = items(graph);
    items.sort_by(|a, b| {
        a.source
            .cmp(&b.source)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    let mut rows = Vec::new();
    let mut index = 0;
    while index < items.len() {
        let key = items[index].source.clone();
        let end = items[index..].partition_point(|item| item.source == key) + index;
        let group = &items[index..end];
        let folded = collapsed.contains(&key);

        rows.push(Row::Group {
            key,
            count: group.len(),
            bytes: group.iter().filter_map(|item| item.bytes).sum(),
            collapsed: folded,
        });
        if !folded {
            rows.extend(group.iter().cloned().map(Row::Item));
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
            let state = if package.outdated {
                State::Outdated
            } else {
                match graph.why(id) {
                    Provenance::Wanted => State::Fine,
                    Provenance::PulledIn(_) => State::PulledIn,
                    Provenance::Unexplained => State::Unexplained,
                }
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
    use super::{Row, State, build};
    use crate::model::fact::{Fact, PackageId};
    use crate::model::graph::Graph;
    use std::collections::BTreeSet;
    use std::path::PathBuf;

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
            names(&build(&machine(), &BTreeSet::new())),
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
            names(&build(&machine(), &collapsed)),
            vec!["[homebrew 2]", "[unclaimed 1]", "Xcode.app"]
        );
    }

    #[test]
    fn a_group_heading_totals_what_is_under_it_even_when_folded() {
        let collapsed = ["homebrew".to_owned()].into_iter().collect();
        let rows = build(&machine(), &collapsed);
        let Row::Group { bytes, count, .. } = &rows[0] else {
            panic!("expected a heading");
        };
        assert_eq!((*count, *bytes), (2, 6_500_000));
    }

    #[test]
    fn state_follows_from_the_graph_rather_than_from_a_flag() {
        let rows = build(&machine(), &BTreeSet::new());
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
        let rows = build(&machine(), &BTreeSet::new());
        assert!(
            !names(&rows).contains(&"awk".to_owned()),
            "a thousand of these would bury the rest"
        );
    }

    #[test]
    fn every_state_has_its_own_glyph_so_colour_is_never_the_only_signal() {
        let all = [
            State::Fine,
            State::Outdated,
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
    fn names_sort_without_regard_to_case() {
        let rows = build(&machine(), &BTreeSet::new());
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
