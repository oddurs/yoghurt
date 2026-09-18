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
//! keg, so the walk in [`crate::source::walk`] produces that fact once it resolves
//! symlinks, and the graph joins the two by path prefix.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, SystemTime};

use serde::Deserialize;

use crate::model::fact::{Fact, PackageId, ScanError, Source};
use crate::source::{children, size_of_each};

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
    desc: Option<String>,
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
    desc: Option<String>,
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

/// One version directory, and what the Cellar itself says about it.
struct Keg {
    path: PathBuf,
    bytes: u64,
    /// Whether this was installed because somebody asked for it by name.
    ///
    /// Read from the keg's own `INSTALL_RECEIPT.json` rather than from
    /// `brew info --json`, which omits formulae from untrusted taps entirely
    /// — so four things installed deliberately looked like residue.
    requested: bool,
}

/// One installed thing, and what it costs on disk.
struct Measured {
    name: String,
    /// Each version directory under it.
    versions: Vec<Keg>,
}

/// The two fields of an install receipt that say why a keg is here.
#[derive(Debug, Default, Deserialize)]
struct Receipt {
    #[serde(default)]
    installed_on_request: bool,
}

impl Receipt {
    /// Every keg Homebrew has written since 2016 carries one; a keg without
    /// one is simply not known to have been requested.
    fn read(keg: &Path) -> Self {
        fs::read_to_string(keg.join("INSTALL_RECEIPT.json"))
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }
}

impl Homebrew {
    /// Walk a Cellar or Caskroom, measuring every version directory.
    ///
    /// This is the expensive half — 160,000 entries and 1.8 seconds on the
    /// machine this was written against — so it runs while `brew info` does.
    fn measure(root: &Path) -> Vec<Measured> {
        // Every version directory of every package, flattened, so all 262 kegs
        // are measured at once rather than one package at a time. Sequentially
        // this walk was 1.8 seconds over 160,000 files and the single largest
        // part of the whole scan.
        let named: Vec<(String, Vec<PathBuf>)> = children(root)
            .into_iter()
            .filter_map(|dir| {
                let name = dir.file_name()?.to_str()?.to_owned();
                let versions = children(&dir);
                (!versions.is_empty()).then_some((name, versions))
            })
            .collect();

        let flat: Vec<PathBuf> = named.iter().flat_map(|(_, v)| v.iter().cloned()).collect();
        let mut sizes = size_of_each(&flat).into_iter();

        named
            .into_iter()
            .map(|(name, versions)| Measured {
                name,
                versions: versions
                    .into_iter()
                    .map(|path| {
                        let bytes = sizes.next().unwrap_or(0);
                        let requested = Receipt::read(&path).installed_on_request;
                        Keg {
                            path,
                            bytes,
                            requested,
                        }
                    })
                    .collect(),
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

        for keg in &self.versions {
            facts.push(Fact::Owns {
                package: id.clone(),
                artifact: keg.path.clone(),
            });
            facts.push(Fact::Size {
                artifact: keg.path.clone(),
                bytes: keg.bytes,
            });
        }

        // The Cellar knows this, so it holds for an untrusted tap too.
        if self.versions.iter().any(|keg| keg.requested) {
            facts.push(Fact::Wanted {
                package: id.clone(),
            });
        }

        let Some(formula) = known else {
            return;
        };
        if let Some(desc) = formula.desc.clone() {
            facts.push(Fact::Describes {
                package: id.clone(),
                text: desc,
            });
        }
        if formula.outdated {
            facts.push(Fact::Outdated {
                package: id.clone(),
                latest: formula.versions.stable.clone(),
            });
        } else {
            // Homebrew reports this for everything it installed, so it is known
            // rather than assumed.
            facts.push(Fact::UpToDate {
                package: id.clone(),
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
        for keg in &self.versions {
            facts.push(Fact::Owns {
                package: id.clone(),
                artifact: keg.path.clone(),
            });
            facts.push(Fact::Size {
                artifact: keg.path.clone(),
                bytes: keg.bytes,
            });
        }

        let Some(cask) = known else {
            return;
        };
        if let Some(desc) = cask.desc.clone() {
            facts.push(Fact::Describes {
                package: id.clone(),
                text: desc,
            });
        }
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
            .filter_map(|keg| Some(keg.path.file_name()?.to_str()?.to_owned()))
            .collect()
    }

    fn newest(&self) -> Option<String> {
        self.version_names().last().cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::Homebrew;
    use crate::model::fact::{Fact, PackageId, Source as _};
    use crate::model::graph::Graph;
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

            // Every real keg carries one of these, and it — not the JSON — is
            // what says whether somebody asked for the thing. `supabase` is
            // the untrusted tap: requested, and absent from the JSON entirely.
            // `fmt` deliberately gets none: a keg without a receipt is not
            // known to have been requested, and its size stays exactly the
            // bytes written above.
            for (keg, requested) in [
                ("Cellar/ripgrep/15.2.0", true),
                ("Cellar/pcre2/10.48", false),
                ("Cellar/supabase/2.72.7", true),
            ] {
                fs::write(
                    root.join(keg).join("INSTALL_RECEIPT.json"),
                    format!("{{\"installed_on_request\":{requested}}}"),
                )
                .expect("write receipt");
            }
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
            graph.package(&brew("ripgrep")).unwrap().wanted(),
            "you typed this"
        );
        assert!(
            !graph.package(&brew("pcre2")).unwrap().wanted(),
            "this came with it"
        );
    }

    #[test]
    fn a_formula_from_an_untrusted_tap_is_still_wanted() {
        let prefix = Prefix::new("tap-wanted");
        let graph = Graph::from_facts(prefix.homebrew().scan().unwrap());
        // `brew info --json` omits untrusted taps, so reading wanted from it
        // made four deliberately installed formulae look like residue.
        assert!(
            graph.package(&brew("supabase")).unwrap().wanted(),
            "the receipt says it was asked for, and the JSON never mentions it"
        );
    }

    #[test]
    fn a_cask_is_wanted_unconditionally() {
        let prefix = Prefix::new("cask");
        let graph = Graph::from_facts(prefix.homebrew().scan().unwrap());
        assert!(
            graph.package(&brew("inkscape")).unwrap().wanted(),
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
            graph.package(&brew("ripgrep")).unwrap().wanted(),
            "the keg's own receipt says so, with no JSON in sight"
        );
        assert!(
            !graph.package(&brew("fmt")).unwrap().wanted(),
            "no receipt is not the same as a receipt saying yes"
        );
    }
}
