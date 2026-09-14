//! Homebrew.
//!
//! The Cellar is the inventory and the JSON enriches it, which is the opposite
//! of how this started. Spike 0008 found that `brew info --json=v2 --installed`
//! silently omits packages from untrusted taps — three of them on the machine
//! this was written against — while those packages remain installed, linked
//! into `bin`, and on the user's `PATH`. An adapter built on the JSON alone
//! would lose them and then show them as orphans, which is precisely the wrong
//! answer, because Homebrew does own them.
//!
//! So this walks `Cellar/<name>/<version>` for what exists and uses the JSON
//! for what Homebrew knows about it. A keg with no JSON entry is a package with
//! fewer facts, never a missing one.
//!
//! It emits no `Provides` facts. `/opt/homebrew/bin/rg` is a symlink into the
//! keg, so the walk in [`crate::walk`] produces that fact once it resolves
//! symlinks, and the graph joins the two by path prefix.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, SystemTime};

use serde::Deserialize;

use crate::fact::{Fact, PackageId, ScanError, Source};

/// Homebrew, as a source of facts.
pub struct Homebrew {
    prefix: PathBuf,
    info: RefCell<InfoHandle>,
}

/// `brew info` is started at construction and collected after the Cellar has
/// been measured, because the two cost about the same and neither needs the
/// other. Doing them in sequence wastes 1.4 seconds of the first frame.
enum InfoHandle {
    Running(Child),
    Done(Option<String>),
}

impl InfoHandle {
    /// Wait for the subprocess, once.
    fn collect(&mut self) -> Option<&str> {
        if let Self::Running(_) = self {
            let Self::Running(child) = std::mem::replace(self, Self::Done(None)) else {
                unreachable!("just matched");
            };
            let text = child
                .wait_with_output()
                .ok()
                .filter(|out| out.status.success())
                .and_then(|out| String::from_utf8(out.stdout).ok());
            *self = Self::Done(text);
        }
        match self {
            Self::Done(text) => text.as_deref(),
            Self::Running(_) => unreachable!("resolved above"),
        }
    }
}

impl Homebrew {
    /// The name this source reports itself under.
    pub const NAME: &'static str = "homebrew";

    /// Homebrew as installed on this machine, if it is.
    ///
    /// Returns `None` when there is no Cellar to read, which is not an error:
    /// a machine without Homebrew simply has no Homebrew packages.
    #[must_use]
    pub fn from_environment() -> Option<Self> {
        let prefix = Self::prefix()?;
        // Canonical, because the walk resolves symlinks and ownership is
        // matched by path prefix. If the prefix is reached through a link, the
        // two sides would never meet and every linked binary would look like an
        // orphan.
        let prefix = fs::canonicalize(&prefix).unwrap_or(prefix);
        let started = Command::new("brew")
            .args(["info", "--json=v2", "--installed"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn();
        let info = match started {
            Ok(child) => InfoHandle::Running(child),
            Err(_) => InfoHandle::Done(None),
        };
        Some(Self {
            prefix,
            info: RefCell::new(info),
        })
    }

    /// Homebrew rooted at a given prefix, with the JSON supplied rather than
    /// fetched. Tests point this at a fixture tree.
    #[must_use]
    pub fn new(prefix: PathBuf, info: Option<String>) -> Self {
        Self {
            prefix,
            info: RefCell::new(InfoHandle::Done(info)),
        }
    }

    /// Where Homebrew lives: what the environment says, else the two prefixes
    /// Homebrew itself installs to.
    fn prefix() -> Option<PathBuf> {
        let candidates = std::env::var_os("HOMEBREW_PREFIX")
            .map(PathBuf::from)
            .into_iter()
            .chain([PathBuf::from("/opt/homebrew"), PathBuf::from("/usr/local")]);
        candidates
            .into_iter()
            .find(|prefix| prefix.join("Cellar").is_dir())
    }
}

/// Only the fields this needs. Serde ignores the rest of the megabyte.
#[derive(Debug, Default, Deserialize)]
struct Info {
    #[serde(default)]
    formulae: Vec<InfoFormula>,
    #[serde(default)]
    casks: Vec<InfoCask>,
}

#[derive(Debug, Deserialize)]
struct InfoFormula {
    name: String,
    #[serde(default)]
    outdated: bool,
    #[serde(default)]
    linked_keg: Option<String>,
    #[serde(default)]
    versions: Versions,
    #[serde(default)]
    installed: Vec<InstalledKeg>,
}

#[derive(Debug, Default, Deserialize)]
struct Versions {
    #[serde(default)]
    stable: Option<String>,
}

#[derive(Debug, Deserialize)]
struct InstalledKeg {
    version: String,
    #[serde(default)]
    installed_on_request: bool,
    #[serde(default)]
    time: Option<i64>,
    #[serde(default)]
    runtime_dependencies: Vec<RuntimeDependency>,
}

#[derive(Debug, Deserialize)]
struct RuntimeDependency {
    full_name: String,
    #[serde(default)]
    declared_directly: bool,
}

/// Casks name everything differently, and a cask is never somebody else's
/// dependency — so every one of them is wanted, unconditionally.
#[derive(Debug, Deserialize)]
struct InfoCask {
    token: String,
    #[serde(default)]
    installed: Option<String>,
    #[serde(default)]
    installed_time: Option<i64>,
    #[serde(default)]
    outdated: bool,
}

fn unix(seconds: i64) -> Option<SystemTime> {
    u64::try_from(seconds)
        .ok()
        .map(|s| SystemTime::UNIX_EPOCH + Duration::from_secs(s))
}

/// Every directory entry that is not a dotfile.
fn children(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut found: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
        .map(|e| e.path())
        .collect();
    found.sort();
    found
}

/// Bytes under a directory, following nothing.
///
/// A keg is a few hundred files, so this is a plain recursive walk rather than
/// anything clever. Symlinks are counted as their own size, never followed —
/// a keg that links into another keg must not be charged for it twice.
fn size_of(path: &Path) -> u64 {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return 0;
    };
    if metadata.is_dir() {
        children(path).iter().map(|child| size_of(child)).sum()
    } else {
        metadata.len()
    }
}

impl Source for Homebrew {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn scan(&self) -> Result<Vec<Fact>, ScanError> {
        // Measure the disk first. `brew info` is running while this happens,
        // and the two cost about the same.
        let kegs = Self::measure(&self.prefix.join("Cellar"));
        let casks = Self::measure(&self.prefix.join("Caskroom"));

        let mut handle = self.info.borrow_mut();
        let info: Info = match handle.collect() {
            Some(text) => serde_json::from_str(text)
                .map_err(|e| ScanError::new(Self::NAME, format!("parsing brew info: {e}")))?,
            None => Info::default(),
        };

        let formulae: BTreeMap<&str, &InfoFormula> =
            info.formulae.iter().map(|f| (f.name.as_str(), f)).collect();
        let by_token: BTreeMap<&str, &InfoCask> =
            info.casks.iter().map(|c| (c.token.as_str(), c)).collect();

        let mut facts = Vec::new();
        for entry in &kegs {
            entry.emit_formula(formulae.get(entry.name.as_str()).copied(), &mut facts);
        }
        for entry in &casks {
            entry.emit_cask(by_token.get(entry.name.as_str()).copied(), &mut facts);
        }
        Ok(facts)
    }
}

/// One installed thing, and what it costs on disk.
struct Measured {
    name: String,
    /// Each version directory under it, with its size.
    versions: Vec<(PathBuf, u64)>,
}

impl Homebrew {
    /// Walk a Cellar or Caskroom, measuring every version directory.
    ///
    /// This is the expensive half — 160,000 entries and 1.8 seconds on the
    /// machine this was written against — so it runs while `brew info` does.
    fn measure(root: &Path) -> Vec<Measured> {
        children(root)
            .into_iter()
            .filter_map(|dir| {
                let name = dir.file_name()?.to_str()?.to_owned();
                let versions: Vec<(PathBuf, u64)> = children(&dir)
                    .into_iter()
                    .map(|v| (v.clone(), size_of(&v)))
                    .collect();
                (!versions.is_empty()).then_some(Measured { name, versions })
            })
            .collect()
    }
}

impl Measured {
    fn id(&self) -> PackageId {
        PackageId::new(Homebrew::NAME, &self.name)
    }

    /// Facts for one formula. `known` is absent for a keg from an untrusted
    /// tap, which is installed all the same.
    fn emit_formula(&self, known: Option<&InfoFormula>, facts: &mut Vec<Fact>) {
        let id = self.id();

        // The version is the one you get when you run it: the linked keg. The
        // others stay visible as owned artifacts with their own sizes.
        let linked = known
            .and_then(|f| f.linked_keg.clone())
            .or_else(|| self.newest());
        facts.push(Fact::Package {
            id: id.clone(),
            version: linked,
        });

        for (path, bytes) in &self.versions {
            facts.push(Fact::Owns {
                package: id.clone(),
                artifact: path.clone(),
            });
            facts.push(Fact::Size {
                artifact: path.clone(),
                bytes: *bytes,
            });
        }

        let Some(formula) = known else {
            return;
        };
        if formula.outdated {
            facts.push(Fact::Outdated {
                package: id.clone(),
                latest: formula.versions.stable.clone(),
            });
        }

        // The JSON may describe a keg that is no longer on disk, and the disk
        // may hold one the JSON never mentioned. Only believe it about kegs
        // that are actually there.
        let on_disk = self.version_names();
        for installed in formula
            .installed
            .iter()
            .filter(|i| on_disk.iter().any(|v| v == &i.version))
        {
            if installed.installed_on_request {
                facts.push(Fact::Wanted {
                    package: id.clone(),
                });
            }
            if let Some(at) = installed.time.and_then(unix) {
                facts.push(Fact::InstalledAt {
                    package: id.clone(),
                    at,
                });
            }
            for dependency in &installed.runtime_dependencies {
                facts.push(Fact::DependsOn {
                    package: id.clone(),
                    on: PackageId::new(Homebrew::NAME, &dependency.full_name),
                    declared_directly: dependency.declared_directly,
                });
            }
        }
    }

    /// Facts for one cask. Casks share almost no field names with formulae, and
    /// a cask is never somebody else's dependency.
    fn emit_cask(&self, known: Option<&InfoCask>, facts: &mut Vec<Fact>) {
        let id = self.id();
        facts.push(Fact::Package {
            id: id.clone(),
            version: known
                .and_then(|c| c.installed.clone())
                .or_else(|| self.newest()),
        });
        facts.push(Fact::Wanted {
            package: id.clone(),
        });
        for (path, bytes) in &self.versions {
            facts.push(Fact::Owns {
                package: id.clone(),
                artifact: path.clone(),
            });
            facts.push(Fact::Size {
                artifact: path.clone(),
                bytes: *bytes,
            });
        }

        let Some(cask) = known else {
            return;
        };
        if cask.outdated {
            facts.push(Fact::Outdated {
                package: id.clone(),
                latest: None,
            });
        }
        if let Some(at) = cask.installed_time.and_then(unix) {
            facts.push(Fact::InstalledAt { package: id, at });
        }
    }

    fn version_names(&self) -> Vec<String> {
        self.versions
            .iter()
            .filter_map(|(path, _)| Some(path.file_name()?.to_str()?.to_owned()))
            .collect()
    }

    fn newest(&self) -> Option<String> {
        self.version_names().last().cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::Homebrew;
    use crate::fact::{Fact, PackageId, Source as _};
    use crate::graph::Graph;
    use std::fs;
    use std::path::PathBuf;

    /// A fixture prefix that removes itself.
    struct Prefix(PathBuf);

    impl Prefix {
        /// A Cellar shaped like the real one: a wanted formula, a formula with
        /// two kegs on disk, and a keg from an untrusted tap that the JSON
        /// refuses to mention at all.
        fn new(tag: &str) -> Self {
            let root =
                std::env::temp_dir().join(format!("yoghurt-brew-{tag}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            for keg in [
                "Cellar/ripgrep/15.2.0/bin",
                "Cellar/fmt/12.1.0/lib",
                "Cellar/fmt/12.2.0/lib",
                "Cellar/pcre2/10.48/lib",
                "Cellar/supabase/2.72.7/bin",
                "Caskroom/inkscape/1.4",
            ] {
                fs::create_dir_all(root.join(keg)).expect("create fixture keg");
            }
            fs::write(root.join("Cellar/ripgrep/15.2.0/bin/rg"), "0123456789")
                .expect("write binary");
            fs::write(root.join("Cellar/fmt/12.1.0/lib/libfmt.a"), "0123").expect("write lib");
            fs::write(root.join("Cellar/fmt/12.2.0/lib/libfmt.a"), "01234567").expect("write lib");
            Self(root)
        }

        fn homebrew(&self) -> Homebrew {
            let json = fs::read_to_string("tests/fixtures/brew-info.json").expect("read fixture");
            Homebrew::new(self.0.clone(), Some(json))
        }

        fn keg(&self, path: &str) -> PathBuf {
            self.0.join(path)
        }
    }

    impl Drop for Prefix {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn brew(name: &str) -> PackageId {
        PackageId::new("homebrew", name)
    }

    #[test]
    fn every_keg_on_disk_becomes_a_package() {
        let prefix = Prefix::new("kegs");
        let graph = Graph::from_facts(prefix.homebrew().scan().unwrap());
        for name in ["ripgrep", "fmt", "pcre2", "supabase", "inkscape"] {
            assert!(graph.package(&brew(name)).is_some(), "{name} is installed");
        }
    }

    #[test]
    fn a_formula_from_an_untrusted_tap_is_owned_by_homebrew_not_orphaned() {
        let prefix = Prefix::new("untrusted");
        let graph = Graph::from_facts(prefix.homebrew().scan().unwrap());

        // The JSON fixture does not mention supabase at all, exactly as brew's
        // own output does not.
        assert!(graph.package(&brew("supabase")).is_some());
        assert_eq!(
            graph.owners_of(&prefix.keg("Cellar/supabase/2.72.7/bin")),
            vec![&brew("supabase")],
            "it is installed by Homebrew, so it must not read as an orphan"
        );
    }

    #[test]
    fn installed_on_request_becomes_wanted_and_nothing_else_does() {
        let prefix = Prefix::new("wanted");
        let graph = Graph::from_facts(prefix.homebrew().scan().unwrap());
        assert!(
            graph.package(&brew("ripgrep")).unwrap().wanted,
            "you typed this"
        );
        assert!(
            !graph.package(&brew("pcre2")).unwrap().wanted,
            "this came with it"
        );
    }

    #[test]
    fn a_cask_is_wanted_unconditionally() {
        let prefix = Prefix::new("cask");
        let graph = Graph::from_facts(prefix.homebrew().scan().unwrap());
        assert!(
            graph.package(&brew("inkscape")).unwrap().wanted,
            "a cask is never somebody else's dependency"
        );
    }

    #[test]
    fn two_kegs_on_disk_are_two_artifacts_with_their_own_sizes() {
        let prefix = Prefix::new("multikeg");
        let graph = Graph::from_facts(prefix.homebrew().scan().unwrap());

        let fmt = graph.package(&brew("fmt")).unwrap();
        assert_eq!(fmt.owns.len(), 2, "both versions are on disk");
        assert_eq!(
            graph
                .artifact(&prefix.keg("Cellar/fmt/12.1.0"))
                .unwrap()
                .bytes,
            Some(4)
        );
        assert_eq!(
            graph
                .artifact(&prefix.keg("Cellar/fmt/12.2.0"))
                .unwrap()
                .bytes,
            Some(8)
        );
    }

    #[test]
    fn the_version_is_the_linked_keg_because_that_is_the_one_you_run() {
        let prefix = Prefix::new("linked");
        let graph = Graph::from_facts(prefix.homebrew().scan().unwrap());
        assert_eq!(
            graph.package(&brew("ripgrep")).unwrap().version.as_deref(),
            Some("15.2.0")
        );
    }

    #[test]
    fn runtime_dependencies_become_edges_carrying_whether_they_were_declared() {
        let prefix = Prefix::new("deps");
        let graph = Graph::from_facts(prefix.homebrew().scan().unwrap());
        let ripgrep = graph.package(&brew("ripgrep")).unwrap();
        assert!(ripgrep.depends_on.contains(&brew("pcre2")));
        assert!(
            ripgrep.declared.contains(&brew("pcre2")),
            "ripgrep asked for pcre2 itself"
        );
    }

    #[test]
    fn the_adapter_emits_no_provides_facts_because_that_is_the_walks_job() {
        let prefix = Prefix::new("provides");
        let facts = prefix.homebrew().scan().unwrap();
        assert!(!facts.iter().any(|f| matches!(f, Fact::Provides { .. })));
    }

    #[test]
    fn sizes_are_emitted_for_every_keg() {
        let prefix = Prefix::new("sizes");
        let facts = prefix.homebrew().scan().unwrap();
        let sized: Vec<_> = facts
            .iter()
            .filter_map(|f| match f {
                Fact::Size { artifact, .. } => Some(artifact.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(sized.len(), 6, "five kegs and one cask: {sized:?}");
    }

    #[test]
    fn homebrew_not_installed_yields_no_facts_and_no_error() {
        let absent = Homebrew::new(PathBuf::from("/nonexistent/for/sure"), None);
        assert_eq!(absent.scan().unwrap(), Vec::new());
    }

    #[test]
    fn unparseable_json_is_reported_rather_than_swallowed() {
        let prefix = Prefix::new("badjson");
        let broken = Homebrew::new(prefix.0.clone(), Some("{not json".to_owned()));
        let error = broken.scan().unwrap_err();
        assert_eq!(error.source_name, "homebrew");
        assert!(error.to_string().contains("parsing brew info"), "{error}");
    }

    #[test]
    fn a_cellar_with_no_json_at_all_still_reports_every_keg() {
        let prefix = Prefix::new("nojson");
        let blind = Homebrew::new(prefix.0.clone(), None);
        let graph = Graph::from_facts(blind.scan().unwrap());
        assert!(graph.package(&brew("ripgrep")).is_some());
        assert!(
            !graph.package(&brew("ripgrep")).unwrap().wanted,
            "without the JSON we cannot know"
        );
    }
}
