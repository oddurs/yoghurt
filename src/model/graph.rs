//! The machine, as one graph.
//!
//! Three kinds of node — a package a manager records, an artifact on disk, and
//! a command resolvable on `PATH` — and four kinds of edge between them. This
//! is the only place with rules: sources assert [`Fact`]s and views project
//! what is here, but neither decides anything.
//!
//! Assembly is deterministic. Facts arrive unordered from threads that finish
//! in whatever order they finish, so every collection here is sorted and the
//! same set of facts always builds the same graph.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::model::fact::{Fact, PackageId};

/// What a package manager records about one package.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Package {
    /// The installed version, where the source reports one.
    pub version: Option<String>,
    /// Whether somebody asked for this deliberately.
    pub wanted: bool,
    /// What it is for, where a source says.
    pub describes: Option<String>,
    /// When it arrived.
    pub installed_at: Option<SystemTime>,
    /// Whether a newer version is published.
    pub outdated: bool,
    /// That newer version, where the source names one.
    pub latest: Option<String>,
    /// Paths this package put on disk.
    pub owns: BTreeSet<PathBuf>,
    /// Everything it needs, direct and inherited.
    pub depends_on: BTreeSet<PackageId>,
    /// The subset of `depends_on` the package asked for itself.
    pub declared: BTreeSet<PackageId>,
}

/// Something occupying space on disk.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Artifact {
    /// Its size, where anybody measured it.
    pub bytes: Option<u64>,
    /// The command names it can be invoked by.
    pub provides: BTreeSet<String>,
    /// Whether it is referred to but not there.
    pub missing: bool,
}

/// The assembled machine.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Graph {
    packages: BTreeMap<PackageId, Package>,
    artifacts: BTreeMap<PathBuf, Artifact>,
    /// Command name to the artifacts providing it.
    providers: BTreeMap<String, BTreeSet<PathBuf>>,
    /// `$PATH`, in order. Earlier wins.
    search_path: Vec<PathBuf>,
    /// Reverse of `Package::owns`, so ownership is a lookup rather than a scan.
    /// 0013 asks who owns a path once per artifact on the machine.
    owners: BTreeMap<PathBuf, BTreeSet<PackageId>>,
    /// Symlink to target, so ownership can follow a link the way the filesystem
    /// does.
    resolves: BTreeMap<PathBuf, PathBuf>,
    /// Reverse of `Package::depends_on`. The why-chain walks this: it asks who
    /// needed a package, not what a package needs.
    dependents: BTreeMap<PackageId, BTreeSet<PackageId>>,
}

impl Graph {
    /// Build the machine from everything the sources said.
    ///
    /// Order-independent: facts are folded into sorted collections, so threads
    /// finishing in a different order cannot produce a different graph.
    ///
    /// Contradictions are kept rather than resolved. Two managers claiming one
    /// artifact is a thing that happens, and losing one of them would be a
    /// worse answer than showing both.
    #[must_use]
    pub fn from_facts(facts: impl IntoIterator<Item = Fact>) -> Self {
        let mut graph = Self::default();
        // Sparse until the end: `$PATH` entries arrive with their index, and
        // nothing guarantees they arrive in it.
        let mut path_entries: BTreeMap<usize, PathBuf> = BTreeMap::new();

        for fact in facts {
            match fact {
                Fact::Package { id, version } => {
                    let package = graph.packages.entry(id).or_default();
                    // A later fact may know a version an earlier one did not.
                    if version.is_some() {
                        package.version = version;
                    }
                }
                Fact::Wanted { package } => {
                    graph.packages.entry(package).or_default().wanted = true;
                }
                Fact::Owns { package, artifact } => {
                    graph.artifacts.entry(artifact.clone()).or_default();
                    graph
                        .owners
                        .entry(artifact.clone())
                        .or_default()
                        .insert(package.clone());
                    graph
                        .packages
                        .entry(package)
                        .or_default()
                        .owns
                        .insert(artifact);
                }
                Fact::DependsOn {
                    package,
                    on,
                    declared_directly,
                } => {
                    graph.packages.entry(on.clone()).or_default();
                    graph
                        .dependents
                        .entry(on.clone())
                        .or_default()
                        .insert(package.clone());
                    let entry = graph.packages.entry(package).or_default();
                    entry.depends_on.insert(on.clone());
                    if declared_directly {
                        entry.declared.insert(on);
                    }
                }
                Fact::Artifact { path } => {
                    graph.artifacts.entry(path).or_default();
                }
                Fact::Resolves { link, target } => {
                    graph.artifacts.entry(link.clone()).or_default();
                    graph.artifacts.entry(target.clone()).or_default();
                    graph.resolves.insert(link, target);
                }
                Fact::Missing { artifact } => {
                    graph.artifacts.entry(artifact).or_default().missing = true;
                }
                Fact::Provides { artifact, command } => {
                    graph
                        .artifacts
                        .entry(artifact.clone())
                        .or_default()
                        .provides
                        .insert(command.clone());
                    graph.providers.entry(command).or_default().insert(artifact);
                }
                Fact::Size { artifact, bytes } => {
                    graph.artifacts.entry(artifact).or_default().bytes = Some(bytes);
                }
                Fact::InstalledAt { package, at } => {
                    graph.packages.entry(package).or_default().installed_at = Some(at);
                }
                Fact::Describes { package, text } => {
                    graph.packages.entry(package).or_default().describes = Some(text);
                }
                Fact::Outdated { package, latest } => {
                    let entry = graph.packages.entry(package).or_default();
                    entry.outdated = true;
                    if latest.is_some() {
                        entry.latest = latest;
                    }
                }
                Fact::SearchPath { index, directory } => {
                    path_entries.insert(index, directory);
                }
            }
        }

        graph.search_path = path_entries.into_values().collect();
        graph
    }

    /// Every package, in a stable order.
    pub fn packages(&self) -> impl Iterator<Item = (&PackageId, &Package)> {
        self.packages.iter()
    }

    /// Every artifact, in a stable order.
    pub fn artifacts(&self) -> impl Iterator<Item = (&Path, &Artifact)> {
        self.artifacts
            .iter()
            .map(|(path, artifact)| (path.as_path(), artifact))
    }

    /// Every command name anything provides, in a stable order.
    pub fn commands(&self) -> impl Iterator<Item = (&str, &BTreeSet<PathBuf>)> {
        self.providers
            .iter()
            .map(|(name, paths)| (name.as_str(), paths))
    }

    /// `$PATH`, in the order the shell searches it.
    #[must_use]
    pub fn search_path(&self) -> &[PathBuf] {
        &self.search_path
    }

    /// One package, if any source recorded it.
    #[must_use]
    pub fn package(&self, id: &PackageId) -> Option<&Package> {
        self.packages.get(id)
    }

    /// One artifact, if anything put it on disk.
    #[must_use]
    pub fn artifact(&self, path: &Path) -> Option<&Artifact> {
        self.artifacts.get(path)
    }

    /// Which packages own this path, by the longest prefix any of them claims.
    ///
    /// Ownership is a prefix relation, not equality. Homebrew owns a keg
    /// directory; the command inside it is `Cellar/ripgrep/15.2.0/bin/rg`, which
    /// the `PATH` walk found by resolving a symlink and which no source ever
    /// names. Matching on equality would make every linked binary an orphan.
    ///
    /// Returns every owner at the longest matching depth, because two managers
    /// claiming one path is representable rather than resolved.
    #[must_use]
    pub fn owners_of(&self, path: &Path) -> Vec<&PackageId> {
        let direct = Self::claim(&self.owners, path);
        if direct.is_empty() {
            // Follow the link the way the filesystem would: `/opt/homebrew/bin/rg`
            // is owned by whoever owns the keg it points into.
            if let Some(target) = self.resolves.get(path) {
                return Self::claim(&self.owners, target);
            }
        }
        direct
    }

    /// The owners claiming the longest prefix of this path.
    fn claim<'a>(
        owners: &'a BTreeMap<PathBuf, BTreeSet<PackageId>>,
        path: &Path,
    ) -> Vec<&'a PackageId> {
        path.ancestors()
            .find_map(|ancestor| owners.get(ancestor))
            .map(|found| found.iter().collect())
            .unwrap_or_default()
    }

    /// Everything that needs this package directly.
    #[must_use]
    pub fn dependents_of(&self, id: &PackageId) -> &BTreeSet<PackageId> {
        static NONE: std::sync::LazyLock<BTreeSet<PackageId>> =
            std::sync::LazyLock::new(BTreeSet::new);
        self.dependents.get(id).unwrap_or(&NONE)
    }

    /// What this path points at, if it is a symlink anything resolved.
    #[must_use]
    pub fn target_of(&self, path: &Path) -> Option<&Path> {
        self.resolves.get(path).map(PathBuf::as_path)
    }

    /// Everything this package needs, following dependencies all the way down.
    ///
    /// Cycle-safe: a package that depends on something that depends back on it
    /// is a broken formula, not a reason to hang.
    #[must_use]
    pub fn transitive_dependencies(&self, id: &PackageId) -> BTreeSet<PackageId> {
        let mut seen = BTreeSet::new();
        let mut queue = vec![id.clone()];
        while let Some(current) = queue.pop() {
            let Some(package) = self.packages.get(&current) else {
                continue;
            };
            for next in &package.depends_on {
                if seen.insert(next.clone()) {
                    queue.push(next.clone());
                }
            }
        }
        seen
    }

    /// How many packages there are, how many artifacts, how many commands.
    #[must_use]
    pub fn counts(&self) -> Counts {
        Counts {
            packages: self.packages.len(),
            artifacts: self.artifacts.len(),
            commands: self.providers.len(),
            owns: self.packages.values().map(|p| p.owns.len()).sum(),
            depends: self.packages.values().map(|p| p.depends_on.len()).sum(),
        }
    }
}

/// The size of the graph, for tests and for the header.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    /// Package nodes.
    pub packages: usize,
    /// Artifact nodes.
    pub artifacts: usize,
    /// Distinct command names.
    pub commands: usize,
    /// `owns` edges.
    pub owns: usize,
    /// `depends` edges.
    pub depends: usize,
}

#[cfg(test)]
mod tests {
    use super::{Counts, Graph};
    use crate::model::fact::{Fact, PackageId};
    use std::path::{Path, PathBuf};
    use std::time::{Duration, SystemTime};

    fn brew(name: &str) -> PackageId {
        PackageId::new("homebrew", name)
    }

    /// A small machine: ripgrep wants itself, pulls in pcre2, and the command
    /// `rg` lives inside ripgrep's keg rather than at the path anybody named.
    fn facts() -> Vec<Fact> {
        vec![
            Fact::Package {
                id: brew("ripgrep"),
                version: Some("15.2.0".to_owned()),
            },
            Fact::Wanted {
                package: brew("ripgrep"),
            },
            Fact::Owns {
                package: brew("ripgrep"),
                artifact: PathBuf::from("/opt/homebrew/Cellar/ripgrep/15.2.0"),
            },
            Fact::Size {
                artifact: PathBuf::from("/opt/homebrew/Cellar/ripgrep/15.2.0"),
                bytes: 6_500_000,
            },
            Fact::DependsOn {
                package: brew("ripgrep"),
                on: brew("pcre2"),
                declared_directly: true,
            },
            Fact::Package {
                id: brew("pcre2"),
                version: Some("10.48".to_owned()),
            },
            Fact::Owns {
                package: brew("pcre2"),
                artifact: PathBuf::from("/opt/homebrew/Cellar/pcre2/10.48"),
            },
            Fact::Provides {
                artifact: PathBuf::from("/opt/homebrew/Cellar/ripgrep/15.2.0/bin/rg"),
                command: "rg".to_owned(),
            },
            Fact::SearchPath {
                index: 1,
                directory: PathBuf::from("/usr/bin"),
            },
            Fact::SearchPath {
                index: 0,
                directory: PathBuf::from("/opt/homebrew/bin"),
            },
        ]
    }

    #[test]
    fn node_and_edge_counts_match_the_facts() {
        assert_eq!(
            Graph::from_facts(facts()).counts(),
            Counts {
                // ripgrep and pcre2.
                packages: 2,
                // Two kegs, plus the binary inside one of them.
                artifacts: 3,
                commands: 1,
                owns: 2,
                depends: 1,
            }
        );
    }

    #[test]
    fn assembly_does_not_depend_on_the_order_facts_arrive_in() {
        let forward = Graph::from_facts(facts());
        let mut backward = facts();
        backward.reverse();
        assert_eq!(forward, Graph::from_facts(backward));
    }

    #[test]
    fn the_search_path_is_ordered_by_index_not_by_arrival() {
        let graph = Graph::from_facts(facts());
        assert_eq!(
            graph.search_path(),
            [
                PathBuf::from("/opt/homebrew/bin"),
                PathBuf::from("/usr/bin")
            ]
        );
    }

    #[test]
    fn ownership_is_a_prefix_relation_so_a_linked_binary_is_not_an_orphan() {
        let graph = Graph::from_facts(facts());
        let binary = Path::new("/opt/homebrew/Cellar/ripgrep/15.2.0/bin/rg");
        assert_eq!(graph.owners_of(binary), vec![&brew("ripgrep")]);
    }

    #[test]
    fn the_longest_claim_wins_a_nested_one() {
        let outer = PathBuf::from("/opt/homebrew/Cellar/python/3.13");
        let inner = outer.join("libexec");
        let graph = Graph::from_facts([
            Fact::Owns {
                package: brew("python"),
                artifact: outer,
            },
            Fact::Owns {
                package: brew("python-libexec"),
                artifact: inner.clone(),
            },
        ]);
        assert_eq!(
            graph.owners_of(&inner.join("bin/pip")),
            vec![&brew("python-libexec")]
        );
    }

    #[test]
    fn a_path_nobody_claims_has_no_owner() {
        let graph = Graph::from_facts(facts());
        assert!(
            graph
                .owners_of(Path::new("/Applications/Xcode.app"))
                .is_empty()
        );
    }

    #[test]
    fn two_sources_claiming_one_artifact_are_both_kept() {
        let artifact = PathBuf::from("/opt/homebrew/bin/node");
        let graph = Graph::from_facts([
            Fact::Owns {
                package: brew("node"),
                artifact: artifact.clone(),
            },
            Fact::Owns {
                package: PackageId::new("nvm", "node"),
                artifact: artifact.clone(),
            },
        ]);
        assert_eq!(
            graph.owners_of(&artifact),
            vec![&brew("node"), &PackageId::new("nvm", "node")]
        );
    }

    #[test]
    fn a_dependency_cycle_does_not_hang() {
        let graph = Graph::from_facts([
            Fact::DependsOn {
                package: brew("a"),
                on: brew("b"),
                declared_directly: true,
            },
            Fact::DependsOn {
                package: brew("b"),
                on: brew("c"),
                declared_directly: true,
            },
            Fact::DependsOn {
                package: brew("c"),
                on: brew("a"),
                declared_directly: true,
            },
        ]);
        assert_eq!(
            graph.transitive_dependencies(&brew("a")),
            [brew("a"), brew("b"), brew("c")].into_iter().collect()
        );
    }

    #[test]
    fn a_package_mentioned_only_as_a_dependency_still_exists() {
        let graph = Graph::from_facts([Fact::DependsOn {
            package: brew("ripgrep"),
            on: brew("pcre2"),
            declared_directly: true,
        }]);
        assert!(
            graph.package(&brew("pcre2")).is_some(),
            "pcre2 is installed, we just learned of it late"
        );
    }

    #[test]
    fn a_later_fact_does_not_erase_a_version_an_earlier_one_knew() {
        let graph = Graph::from_facts([
            Fact::Package {
                id: brew("rg"),
                version: Some("15.2.0".to_owned()),
            },
            Fact::Package {
                id: brew("rg"),
                version: None,
            },
        ]);
        assert_eq!(
            graph.package(&brew("rg")).unwrap().version.as_deref(),
            Some("15.2.0")
        );
    }

    #[test]
    fn declared_dependencies_are_a_subset_of_all_of_them() {
        let graph = Graph::from_facts([
            Fact::DependsOn {
                package: brew("a"),
                on: brew("b"),
                declared_directly: true,
            },
            Fact::DependsOn {
                package: brew("a"),
                on: brew("c"),
                declared_directly: false,
            },
        ]);
        let a = graph.package(&brew("a")).unwrap();
        assert_eq!(a.depends_on.len(), 2);
        assert_eq!(a.declared, [brew("b")].into_iter().collect());
    }

    #[test]
    fn outdated_is_the_presence_of_the_fact_not_the_presence_of_a_version() {
        let named = Graph::from_facts([Fact::Outdated {
            package: brew("neovim"),
            latest: Some("0.12.0".to_owned()),
        }]);
        let anonymous = Graph::from_facts([Fact::Outdated {
            package: brew("neovim"),
            latest: None,
        }]);

        let named = named.package(&brew("neovim")).unwrap();
        assert!(named.outdated);
        assert_eq!(named.latest.as_deref(), Some("0.12.0"));

        let anonymous = anonymous.package(&brew("neovim")).unwrap();
        assert!(
            anonymous.outdated,
            "a source may know it is stale without knowing what is newer"
        );
        assert_eq!(anonymous.latest, None);
    }

    #[test]
    fn a_package_nobody_called_outdated_is_current() {
        let graph = Graph::from_facts(facts());
        assert!(!graph.package(&brew("ripgrep")).unwrap().outdated);
    }

    #[test]
    fn install_time_survives_assembly() {
        let at = SystemTime::UNIX_EPOCH + Duration::from_secs(1_789_355_634);
        let graph = Graph::from_facts([Fact::InstalledAt {
            package: brew("rg"),
            at,
        }]);
        assert_eq!(graph.package(&brew("rg")).unwrap().installed_at, Some(at));
    }
}
