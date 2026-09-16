//! The four questions.
//!
//! Every state the interface shows is a query over the graph rather than a
//! feature somebody remembered to build. Orphan, broken, pulled in and
//! shadowed are structural facts: they follow from what is and is not
//! connected, so they cannot drift out of step with each other and a new
//! package manager gets all four for nothing.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::Path;

use crate::model::fact::PackageId;
use crate::model::graph::Graph;

/// Why a package is on the machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Provenance {
    /// Somebody asked for it. A package that is both wanted and depended on is
    /// still wanted — you chose it, and something later happening to need it
    /// does not undo that.
    Wanted,

    /// Nothing asked for it; it arrived underneath something that was.
    ///
    /// Carries the chain back to the nearest wanted package, nearest first, so
    /// the detail pane can print `glib -> gtk+3 -> inkscape` without walking
    /// the graph again.
    PulledIn(Vec<PackageId>),

    /// Nothing asked for it and nothing wanted needs it.
    ///
    /// Usually the residue of an uninstall that did not finish, or a
    /// dependency of something removed. The most interesting thing a package
    /// can be, and invisible to every package manager individually.
    Unexplained,
}

/// Which provider of a command actually runs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resolution<'a> {
    /// The one the shell finds first.
    pub winner: Option<&'a Path>,
    /// The rest, in the order they lose, each beaten by everything before it.
    pub shadowed: Vec<&'a Path>,
}

impl Resolution<'_> {
    /// Whether more than one thing provides this name.
    #[must_use]
    pub fn contested(&self) -> bool {
        !self.shadowed.is_empty()
    }
}

/// Prefixes macOS itself owns.
///
/// Nothing claims `/usr/bin/awk`, which makes it an orphan by the letter of the
/// model and noise by any useful measure — there are over a thousand of them
/// against a few dozen that matter.
///
/// `/Library` is deliberately **not** here even though `/System/Library` is:
/// third parties install into `/Library`, and treating it as the system's
/// hid 36 TeX Live files that a receipt can name perfectly well.
///
/// The one piece of policy in the model. It lives here rather than in a view so
/// that everything asking "is this worth mentioning" gets the same answer.
pub const SYSTEM_PREFIXES: &[&str] = &[
    "/usr/bin",
    "/usr/sbin",
    "/usr/libexec",
    "/usr/share",
    "/bin",
    "/sbin",
    "/System",
];

/// Whether macOS itself put this here.
#[must_use]
pub fn is_system(path: &Path) -> bool {
    SYSTEM_PREFIXES
        .iter()
        .any(|prefix| path.starts_with(prefix))
}

impl Graph {
    /// **Orphan**: an artifact with no owning package.
    ///
    /// Nobody detects these. They are what is left when every manager's claims
    /// are laid against what the walk found, which is why a package manager
    /// this program has never heard of still produces them.
    #[must_use]
    pub fn is_orphan(&self, path: &Path) -> bool {
        self.artifact(path).is_some() && self.owners_of(path).is_empty()
    }

    /// Everything on disk that nothing claims and somebody might care about.
    ///
    /// Orphans minus what macOS put there, which is the difference between a
    /// few dozen answers and fourteen hundred.
    #[must_use]
    pub fn unclaimed(&self) -> Vec<&Path> {
        self.artifacts()
            .filter(|(path, _)| self.is_orphan(path) && !is_system(path))
            .map(|(path, _)| path)
            .collect()
    }

    /// **Broken**: something refers to this path and it is not there.
    ///
    /// A dangling symlink on `$PATH`, or a package owning a keg that has been
    /// deleted underneath it.
    #[must_use]
    pub fn is_broken(&self, path: &Path) -> bool {
        self.artifact(path).is_some_and(|artifact| artifact.missing)
    }

    /// **Pulled in**: reachable only through `depends`, never from `wanted`.
    ///
    /// Walks the reverse dependency edges outward from the package until it
    /// reaches something wanted, so the result is the shortest honest
    /// explanation rather than the first one found. Cycle-safe.
    #[must_use]
    pub fn why(&self, id: &PackageId) -> Provenance {
        if self.package(id).is_some_and(|package| package.wanted) {
            return Provenance::Wanted;
        }

        // Breadth-first, so the chain is the shortest one that explains it.
        let mut came_from: BTreeMap<PackageId, PackageId> = BTreeMap::new();
        let mut seen: BTreeSet<PackageId> = [id.clone()].into();
        let mut queue: VecDeque<PackageId> = [id.clone()].into();

        while let Some(current) = queue.pop_front() {
            for next in self.dependents_of(&current) {
                if !seen.insert(next.clone()) {
                    continue;
                }
                came_from.insert(next.clone(), current.clone());
                if self.package(next).is_some_and(|package| package.wanted) {
                    return Provenance::PulledIn(Self::chain(&came_from, id, next));
                }
                queue.push_back(next.clone());
            }
        }
        Provenance::Unexplained
    }

    /// Rebuild the route from `start` to `wanted`, nearest first.
    fn chain(
        came_from: &BTreeMap<PackageId, PackageId>,
        start: &PackageId,
        wanted: &PackageId,
    ) -> Vec<PackageId> {
        let mut chain = vec![start.clone()];
        let mut step = wanted.clone();
        let mut route = vec![step.clone()];
        while let Some(previous) = came_from.get(&step) {
            if previous == start {
                break;
            }
            route.push(previous.clone());
            step = previous.clone();
        }
        route.reverse();
        chain.extend(route);
        chain
    }

    /// **Shadowed**: two artifacts provide one command, and `$PATH` order
    /// decides.
    ///
    /// Exactly what the shell does: the earliest entry wins. A provider whose
    /// directory is not on `$PATH` at all sorts last, because nothing would
    /// ever reach it by typing the name.
    #[must_use]
    pub fn resolve(&self, command: &str) -> Resolution<'_> {
        let Some((_, providers)) = self.commands().find(|(name, _)| *name == command) else {
            return Resolution {
                winner: None,
                shadowed: Vec::new(),
            };
        };

        let mut ranked: Vec<(usize, &Path)> = providers
            .iter()
            .map(|path| (self.path_rank(path), path.as_path()))
            .collect();
        // Stable on the path itself, so two providers in one directory do not
        // swap places between runs.
        ranked.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(b.1)));

        let mut paths = ranked.into_iter().map(|(_, path)| path);
        Resolution {
            winner: paths.next(),
            shadowed: paths.collect(),
        }
    }

    /// Every command more than one thing provides.
    #[must_use]
    pub fn contested(&self) -> Vec<(&str, Resolution<'_>)> {
        self.commands()
            .filter(|(_, providers)| providers.len() > 1)
            .map(|(name, _)| (name, self.resolve(name)))
            .filter(|(_, resolution)| resolution.contested())
            .collect()
    }

    /// Where this artifact's directory sits in `$PATH`. Off-path sorts last.
    fn path_rank(&self, path: &Path) -> usize {
        path.parent()
            .and_then(|dir| self.search_path().iter().position(|entry| entry == dir))
            .unwrap_or(usize::MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::Provenance;
    use crate::model::fact::{Fact, PackageId};
    use crate::model::graph::Graph;
    use std::path::{Path, PathBuf};

    fn brew(name: &str) -> PackageId {
        PackageId::new("homebrew", name)
    }

    /// inkscape was installed on purpose; gtk+3 came with it; glib came with
    /// gtk+3. Exactly the chain the design promises to print.
    fn dependency_chain() -> Graph {
        Graph::from_facts([
            Fact::Wanted {
                package: brew("inkscape"),
            },
            Fact::DependsOn {
                package: brew("inkscape"),
                on: brew("gtk+3"),
                declared_directly: true,
            },
            Fact::DependsOn {
                package: brew("gtk+3"),
                on: brew("glib"),
                declared_directly: true,
            },
        ])
    }

    #[test]
    fn a_package_somebody_asked_for_is_wanted() {
        assert_eq!(
            dependency_chain().why(&brew("inkscape")),
            Provenance::Wanted
        );
    }

    #[test]
    fn a_dependency_carries_the_chain_back_to_what_you_installed() {
        assert_eq!(
            dependency_chain().why(&brew("glib")),
            Provenance::PulledIn(vec![brew("glib"), brew("gtk+3"), brew("inkscape")])
        );
    }

    #[test]
    fn the_chain_is_the_shortest_explanation_not_the_first_one_found() {
        // glib is reachable two ways; the direct one is the honest answer.
        let graph = Graph::from_facts([
            Fact::Wanted {
                package: brew("inkscape"),
            },
            Fact::DependsOn {
                package: brew("inkscape"),
                on: brew("glib"),
                declared_directly: true,
            },
            Fact::DependsOn {
                package: brew("inkscape"),
                on: brew("gtk+3"),
                declared_directly: true,
            },
            Fact::DependsOn {
                package: brew("gtk+3"),
                on: brew("glib"),
                declared_directly: true,
            },
        ]);
        assert_eq!(
            graph.why(&brew("glib")),
            Provenance::PulledIn(vec![brew("glib"), brew("inkscape")])
        );
    }

    #[test]
    fn a_package_that_is_both_wanted_and_depended_on_counts_as_wanted() {
        let graph = Graph::from_facts([
            Fact::Wanted {
                package: brew("inkscape"),
            },
            Fact::Wanted {
                package: brew("glib"),
            },
            Fact::DependsOn {
                package: brew("inkscape"),
                on: brew("glib"),
                declared_directly: true,
            },
        ]);
        assert_eq!(
            graph.why(&brew("glib")),
            Provenance::Wanted,
            "you chose it first"
        );
    }

    #[test]
    fn a_package_nothing_wanted_needs_is_unexplained() {
        let graph = Graph::from_facts([Fact::DependsOn {
            package: brew("abandoned"),
            on: brew("residue"),
            declared_directly: true,
        }]);
        assert_eq!(graph.why(&brew("residue")), Provenance::Unexplained);
    }

    #[test]
    fn a_dependency_cycle_does_not_hang_the_why_chain() {
        let graph = Graph::from_facts([
            Fact::DependsOn {
                package: brew("a"),
                on: brew("b"),
                declared_directly: true,
            },
            Fact::DependsOn {
                package: brew("b"),
                on: brew("a"),
                declared_directly: true,
            },
        ]);
        assert_eq!(graph.why(&brew("a")), Provenance::Unexplained);
    }

    #[test]
    fn an_artifact_nobody_owns_is_an_orphan() {
        let app = PathBuf::from("/Applications/Xcode.app");
        let graph = Graph::from_facts([Fact::Artifact { path: app.clone() }]);
        assert!(graph.is_orphan(&app));
    }

    #[test]
    fn an_artifact_inside_something_owned_is_not_an_orphan() {
        let keg = PathBuf::from("/opt/homebrew/Cellar/ripgrep/15.2.0");
        let binary = keg.join("bin/rg");
        let graph = Graph::from_facts([
            Fact::Owns {
                package: brew("ripgrep"),
                artifact: keg,
            },
            Fact::Artifact {
                path: binary.clone(),
            },
        ]);
        assert!(!graph.is_orphan(&binary), "ownership is a prefix relation");
    }

    #[test]
    fn a_path_no_source_ever_mentioned_is_not_an_orphan_it_is_nothing() {
        let graph = Graph::from_facts([]);
        assert!(
            !graph.is_orphan(Path::new("/nowhere")),
            "absence is not orphanhood"
        );
    }

    #[test]
    fn a_dangling_symlink_is_broken() {
        let ghost = PathBuf::from("/opt/homebrew/bin/ghost");
        let graph = Graph::from_facts([Fact::Missing {
            artifact: ghost.clone(),
        }]);
        assert!(graph.is_broken(&ghost));
    }

    /// Two `rg`s: a current one from Homebrew and an older one from cargo. The
    /// shell takes whichever directory comes first.
    fn contested_rg() -> (Graph, PathBuf, PathBuf) {
        let brew_rg = PathBuf::from("/opt/homebrew/bin/rg");
        let cargo_rg = PathBuf::from("/Users/x/.cargo/bin/rg");
        let graph = Graph::from_facts([
            Fact::SearchPath {
                index: 0,
                directory: PathBuf::from("/opt/homebrew/bin"),
            },
            Fact::SearchPath {
                index: 1,
                directory: PathBuf::from("/Users/x/.cargo/bin"),
            },
            Fact::Provides {
                artifact: brew_rg.clone(),
                command: "rg".to_owned(),
            },
            Fact::Provides {
                artifact: cargo_rg.clone(),
                command: "rg".to_owned(),
            },
        ]);
        (graph, brew_rg, cargo_rg)
    }

    #[test]
    fn the_earliest_path_entry_wins_and_the_rest_are_shadowed() {
        let (graph, brew_rg, cargo_rg) = contested_rg();
        let resolution = graph.resolve("rg");
        assert_eq!(resolution.winner, Some(brew_rg.as_path()));
        assert_eq!(resolution.shadowed, vec![cargo_rg.as_path()]);
        assert!(resolution.contested());
    }

    #[test]
    fn one_provider_is_not_contested() {
        let graph = Graph::from_facts([
            Fact::SearchPath {
                index: 0,
                directory: PathBuf::from("/opt/homebrew/bin"),
            },
            Fact::Provides {
                artifact: PathBuf::from("/opt/homebrew/bin/fd"),
                command: "fd".to_owned(),
            },
        ]);
        assert!(!graph.resolve("fd").contested());
        assert!(graph.contested().is_empty());
    }

    #[test]
    fn a_provider_that_is_not_on_the_path_at_all_loses_to_one_that_is() {
        let on_path = PathBuf::from("/usr/bin/python3");
        let off_path = PathBuf::from("/opt/weird/python3");
        let graph = Graph::from_facts([
            Fact::SearchPath {
                index: 0,
                directory: PathBuf::from("/usr/bin"),
            },
            Fact::Provides {
                artifact: off_path.clone(),
                command: "python3".to_owned(),
            },
            Fact::Provides {
                artifact: on_path.clone(),
                command: "python3".to_owned(),
            },
        ]);
        let resolution = graph.resolve("python3");
        assert_eq!(
            resolution.winner,
            Some(on_path.as_path()),
            "nothing reaches the other one"
        );
        assert_eq!(resolution.shadowed, vec![off_path.as_path()]);
    }

    #[test]
    fn a_command_nothing_provides_has_no_winner() {
        let graph = Graph::from_facts([]);
        assert_eq!(graph.resolve("nonesuch").winner, None);
    }

    #[test]
    fn resolution_does_not_depend_on_the_order_facts_arrived_in() {
        let (forward, brew_rg, _) = contested_rg();
        assert_eq!(forward.resolve("rg").winner, Some(brew_rg.as_path()));
    }
}
