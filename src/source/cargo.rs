//! Cargo, and the toolchains underneath it.
//!
//! Two sources in one file because they share a directory and are otherwise
//! impossible to tell apart: `~/.cargo/bin` holds both the binaries you asked
//! `cargo install` for and the shims rustup puts there, and counting the shims
//! as installed crates would overstate every machine with Rust on it by a
//! dozen.
//!
//! Neither needs a subprocess. Cargo keeps a manifest of what it installed and
//! rustup keeps a directory per toolchain, so both are a file read.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use serde::Deserialize;

use crate::model::fact::{Fact, PackageId, ScanError, Source};
use crate::source::{children, size_of};

/// Binaries rustup puts in `~/.cargo/bin`.
///
/// Fixed by rustup rather than guessed: these are the shims it installs, and
/// they are the toolchain rather than anything anybody chose.
const RUSTUP_SHIMS: &[&str] = &[
    "cargo",
    "cargo-clippy",
    "cargo-fmt",
    "cargo-miri",
    "clippy-driver",
    "rls",
    "rust-analyzer",
    "rust-gdb",
    "rust-gdbgui",
    "rust-lldb",
    "rustc",
    "rustdoc",
    "rustfmt",
    "rustup",
];

/// What `cargo install` has put on this machine.
pub struct Cargo {
    home: PathBuf,
}

impl Cargo {
    /// The name this source reports itself under.
    pub const NAME: &'static str = "cargo";

    /// Cargo as installed for this user, if it is.
    #[must_use]
    pub fn from_environment() -> Option<Self> {
        let home = cargo_home()?;
        home.join("bin").is_dir().then_some(Self { home })
    }

    /// Cargo rooted somewhere else. Tests point this at a fixture tree.
    #[must_use]
    pub fn new(home: PathBuf) -> Self {
        Self { home }
    }
}

/// `~/.cargo/.crates2.json`, reduced to what matters.
#[derive(Debug, Default, Deserialize)]
struct Crates2 {
    #[serde(default)]
    installs: std::collections::BTreeMap<String, Install>,
}

#[derive(Debug, Deserialize)]
struct Install {
    #[serde(default)]
    bins: Vec<String>,
}

/// `ripgrep 14.1.1 (registry+https://...)` into its parts.
///
/// The source in brackets says where it came from, and a crate installed from a
/// local path is a different thing from one off crates.io — you can edit it.
fn split_key(key: &str) -> Option<(String, String, Origin)> {
    let (name, rest) = key.split_once(' ')?;
    let (version, source) = rest.split_once(' ')?;
    let origin = if source.contains("registry+") {
        Origin::Registry
    } else if source.contains("git+") {
        Origin::Git
    } else {
        Origin::Path
    };
    Some((name.to_owned(), version.to_owned(), origin))
}

/// Where a crate came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Origin {
    /// crates.io, or another registry.
    Registry,
    /// A git repository.
    Git,
    /// A directory on this machine.
    Path,
}

impl Origin {
    fn describe(self) -> &'static str {
        match self {
            Self::Registry => "installed from a registry",
            Self::Git => "installed from a git repository",
            Self::Path => "installed from a local checkout",
        }
    }
}

impl Cargo {
    /// What the manifest says is installed.
    fn installed(&self) -> Result<Vec<(PackageId, String, Origin)>, ScanError> {
        let manifest = self.home.join(".crates2.json");
        let Ok(text) = fs::read_to_string(&manifest) else {
            return Ok(Vec::new());
        };
        let parsed: Crates2 = serde_json::from_str(&text).map_err(|e| {
            ScanError::new(Self::NAME, format!("parsing {}: {e}", manifest.display()))
        })?;
        Ok(parsed
            .installs
            .keys()
            .filter_map(|key| split_key(key))
            .map(|(name, version, origin)| (PackageId::new(Self::NAME, name), version, origin))
            .collect())
    }
}

/// The newest stable version crates.io has.
///
/// Through `curl`, for the same reason the taxonomy is: no TLS stack in the
/// binary for something most runs never do.
fn newest_on_crates_io(name: &str) -> Option<String> {
    let output = Command::new("curl")
        .args(["--silent", "--max-time", "10", "-H", "User-Agent: yoghurt"])
        .arg(format!("https://crates.io/api/v1/crates/{name}"))
        .output()
        .ok()?;
    let body: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    body["crate"]["max_stable_version"]
        .as_str()
        .map(ToOwned::to_owned)
}

impl Source for Cargo {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    /// Ask crates.io what the newest published version is.
    ///
    /// One request per crate, because crates.io has no way to ask about
    /// several at once. Twenty-one crates is twenty-one round trips and a few
    /// seconds, which is why this never runs unless somebody asked for it.
    ///
    /// A crate installed from a path or a git repository is skipped: there is
    /// no registry version to compare against, and reporting it as current
    /// would be a guess dressed as a fact.
    fn updates(&self) -> Result<Vec<Fact>, ScanError> {
        let installed = self.installed()?;
        let mut facts = Vec::new();
        for (id, version, origin) in installed {
            if origin != Origin::Registry {
                continue;
            }
            let Some(newest) = newest_on_crates_io(&id.name) else {
                // Unreachable or unknown: leave it reading as unchecked rather
                // than claiming it is current.
                continue;
            };
            if newest == version {
                facts.push(Fact::UpToDate { package: id });
            } else {
                facts.push(Fact::Outdated {
                    package: id,
                    latest: Some(newest),
                });
            }
        }
        Ok(facts)
    }

    fn scan(&self) -> Result<Vec<Fact>, ScanError> {
        let manifest = self.home.join(".crates2.json");
        let Ok(text) = fs::read_to_string(&manifest) else {
            // Cargo is present but has installed nothing. Not an error.
            return Ok(Vec::new());
        };
        let parsed: Crates2 = serde_json::from_str(&text).map_err(|e| {
            ScanError::new(Self::NAME, format!("parsing {}: {e}", manifest.display()))
        })?;

        let bin = self.home.join("bin");
        let mut facts = Vec::new();
        for (key, install) in &parsed.installs {
            let Some((name, version, origin)) = split_key(key) else {
                continue;
            };
            let id = PackageId::new(Self::NAME, &name);
            facts.push(Fact::Package {
                id: id.clone(),
                version: Some(version),
            });
            // You typed `cargo install`. Nothing arrives here as somebody
            // else's dependency.
            facts.push(Fact::Wanted {
                package: id.clone(),
            });
            facts.push(Fact::Describes {
                package: id.clone(),
                text: origin.describe().to_owned(),
            });

            for binary in &install.bins {
                let path = bin.join(binary);
                if path.exists() {
                    facts.push(Fact::Size {
                        artifact: path.clone(),
                        bytes: size_of(&path),
                    });
                    facts.push(Fact::Owns {
                        package: id.clone(),
                        artifact: path,
                    });
                }
            }
        }
        Ok(facts)
    }
}

/// The Rust toolchains rustup manages.
pub struct Rustup {
    home: PathBuf,
    cargo_bin: PathBuf,
    /// How to ask rustup what is newer. Injected so tests never run it: the
    /// answer depends on the machine, which would make the suite depend on it
    /// too.
    check: fn() -> Option<String>,
}

impl Rustup {
    /// The name this source reports itself under.
    pub const NAME: &'static str = "rustup";

    /// Rustup as installed for this user, if it is.
    #[must_use]
    pub fn from_environment() -> Option<Self> {
        let home = std::env::var_os("RUSTUP_HOME")
            .map(PathBuf::from)
            .or_else(|| home_dir().map(|h| h.join(".rustup")))?;
        let cargo_bin = cargo_home()?.join("bin");
        home.join("toolchains").is_dir().then_some(Self {
            home,
            cargo_bin,
            check: rustup_check,
        })
    }

    /// Rustup rooted somewhere else. Tests point this at a fixture tree.
    #[must_use]
    pub fn new(home: PathBuf, cargo_bin: PathBuf) -> Self {
        Self {
            home,
            cargo_bin,
            check: rustup_check,
        }
    }

    /// Rustup with the check answered by a stub.
    #[must_use]
    pub fn with_check(home: PathBuf, cargo_bin: PathBuf, check: fn() -> Option<String>) -> Self {
        Self {
            home,
            cargo_bin,
            check,
        }
    }
}

impl Source for Rustup {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    /// `rustup check` says which toolchains have something newer.
    ///
    /// It reaches the network itself, which is why this lives here rather than
    /// in `scan`.
    ///
    /// The report names **channels** — `stable-aarch64-apple-darwin` — while
    /// the directories on a machine may be pinned to versions. A fact about a
    /// name that is not installed would not update a package, it would invent
    /// one, so only what exists on disk is reported.
    fn updates(&self) -> Result<Vec<Fact>, ScanError> {
        let Some(report) = (self.check)() else {
            return Ok(Vec::new());
        };
        let installed: Vec<String> = children(&self.home.join("toolchains"))
            .iter()
            .filter_map(|dir| Some(dir.file_name()?.to_str()?.to_owned()))
            .collect();

        Ok(report
            .lines()
            .filter_map(parse_check)
            .filter(|fact| match fact {
                Fact::UpToDate { package } | Fact::Outdated { package, .. } => {
                    installed.contains(&package.name)
                }
                _ => false,
            })
            .collect())
    }

    fn scan(&self) -> Result<Vec<Fact>, ScanError> {
        let mut facts = Vec::new();

        for toolchain in children(&self.home.join("toolchains")) {
            let Some(name) = toolchain.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            let id = PackageId::new(Self::NAME, name);
            facts.push(Fact::Package {
                id: id.clone(),
                version: None,
            });
            facts.push(Fact::Wanted {
                package: id.clone(),
            });
            // Toolchains are gigabytes and there are usually several, so they
            // are not measured here. 0031 makes that affordable.
            facts.push(Fact::Owns {
                package: id,
                artifact: toolchain,
            });
        }

        // The shims belong to rustup, not to cargo, so they stop being counted
        // twice and stop reading as orphans.
        let shims = PackageId::new(Self::NAME, "shims");
        let mut any = false;
        for shim in RUSTUP_SHIMS {
            let path = self.cargo_bin.join(shim);
            if path.exists() {
                any = true;
                facts.push(Fact::Owns {
                    package: shims.clone(),
                    artifact: path,
                });
            }
        }
        if any {
            facts.push(Fact::Package {
                id: shims.clone(),
                version: None,
            });
            facts.push(Fact::Wanted {
                package: shims.clone(),
            });
            facts.push(Fact::Describes {
                package: shims,
                text: "the rustup shims that dispatch to the active toolchain".to_owned(),
            });
        }
        Ok(facts)
    }
}

/// Ask rustup what is newer.
fn rustup_check() -> Option<String> {
    let output = Command::new("rustup").arg("check").output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

/// One line of `rustup check`.
///
/// `stable-aarch64-apple-darwin - up to date : 1.98.1 (…)` or
/// `nightly-… - Update available : 1.100.0-nightly (…) -> 1.100.0-nightly (…)`
fn parse_check(line: &str) -> Option<Fact> {
    let (name, rest) = line.split_once(" - ")?;
    let package = PackageId::new(Rustup::NAME, name.trim());
    let rest = rest.to_lowercase();
    if rest.starts_with("up to date") {
        return Some(Fact::UpToDate { package });
    }
    if rest.starts_with("update available") {
        // Everything after the arrow is the version being offered.
        let latest = line
            .rsplit_once("-> ")
            .map(|(_, newer)| newer.trim().to_owned());
        return Some(Fact::Outdated { package, latest });
    }
    None
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

fn cargo_home() -> Option<PathBuf> {
    std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| home_dir().map(|h| h.join(".cargo")))
}

#[cfg(test)]
mod tests {
    use super::{Cargo, Rustup};
    use crate::Graph;
    use crate::model::fact::{Fact, PackageId, Source as _};
    use std::fs;
    use std::path::PathBuf;

    /// A fixture `~/.cargo` and `~/.rustup` that removes itself.
    struct Home(PathBuf);

    impl Home {
        fn new(tag: &str) -> Self {
            let root =
                std::env::temp_dir().join(format!("yoghurt-cargo-{tag}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            let bin = root.join(".cargo/bin");
            fs::create_dir_all(&bin).expect("create bin");
            fs::create_dir_all(root.join(".rustup/toolchains/stable-aarch64-apple-darwin"))
                .expect("create toolchain");

            // Two real crates, a rustup shim, and a binary cargo never recorded.
            for name in ["rg", "cargo-nextest", "cargo", "rustc", "wasm-pack"] {
                fs::write(bin.join(name), "0123456789").expect("write binary");
            }
            fs::write(
                root.join(".cargo/.crates2.json"),
                r#"{"installs":{
                    "ripgrep 14.1.1 (registry+https://github.com/rust-lang/crates.io-index)":
                        {"bins":["rg"]},
                    "cargo-nextest 0.9.143 (registry+https://github.com/rust-lang/crates.io-index)":
                        {"bins":["cargo-nextest"]},
                    "backpage 0.1.0 (path+file:///Users/x/Code/backpage)":
                        {"bins":["backpage"]}
                }}"#,
            )
            .expect("write manifest");
            Self(root)
        }

        fn cargo(&self) -> Cargo {
            Cargo::new(self.0.join(".cargo"))
        }

        fn rustup(&self) -> Rustup {
            Rustup::new(self.0.join(".rustup"), self.0.join(".cargo/bin"))
        }

        fn graph(&self) -> Graph {
            let mut facts = self.cargo().scan().expect("cargo");
            facts.extend(self.rustup().scan().expect("rustup"));
            Graph::from_facts(facts)
        }
    }

    impl Drop for Home {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn every_recorded_crate_becomes_a_package() {
        let home = Home::new("crates");
        let graph = home.graph();
        for name in ["ripgrep", "cargo-nextest", "backpage"] {
            assert!(
                graph.package(&PackageId::new("cargo", name)).is_some(),
                "{name}"
            );
        }
    }

    #[test]
    fn a_crate_is_always_wanted_because_you_typed_cargo_install() {
        let home = Home::new("wanted");
        let graph = home.graph();
        assert!(
            graph
                .package(&PackageId::new("cargo", "ripgrep"))
                .unwrap()
                .wanted
        );
    }

    #[test]
    fn the_version_comes_off_the_manifest_key() {
        let home = Home::new("version");
        let graph = home.graph();
        assert_eq!(
            graph
                .package(&PackageId::new("cargo", "cargo-nextest"))
                .unwrap()
                .version
                .as_deref(),
            Some("0.9.143")
        );
    }

    #[test]
    fn a_local_checkout_is_described_differently_from_a_registry_crate() {
        let home = Home::new("origin");
        let graph = home.graph();
        let local = graph.package(&PackageId::new("cargo", "backpage")).unwrap();
        let registry = graph.package(&PackageId::new("cargo", "ripgrep")).unwrap();
        assert!(
            local
                .describes
                .as_deref()
                .unwrap()
                .contains("local checkout")
        );
        assert!(registry.describes.as_deref().unwrap().contains("registry"));
    }

    #[test]
    fn a_binary_the_manifest_does_not_mention_stays_unowned() {
        let home = Home::new("unrecorded");
        let graph = home.graph();
        assert!(
            graph
                .owners_of(&home.0.join(".cargo/bin/wasm-pack"))
                .is_empty(),
            "cargo did not install it, so cargo must not claim it"
        );
    }

    #[test]
    fn a_binary_the_manifest_names_but_disk_does_not_have_is_not_invented() {
        let home = Home::new("missing");
        let facts = home.cargo().scan().unwrap();
        assert!(
            !facts.iter().any(|f| matches!(f, Fact::Owns { artifact, .. }
                if artifact.ends_with("backpage"))),
            "backpage is in the manifest but not on disk"
        );
    }

    #[test]
    fn the_shims_belong_to_rustup_rather_than_being_counted_as_crates() {
        let home = Home::new("shims");
        let graph = home.graph();
        let shims = PackageId::new("rustup", "shims");
        assert_eq!(
            graph.owners_of(&home.0.join(".cargo/bin/cargo")),
            vec![&shims]
        );
        assert_eq!(
            graph.owners_of(&home.0.join(".cargo/bin/rustc")),
            vec![&shims]
        );
        assert!(
            graph.package(&PackageId::new("cargo", "cargo")).is_none(),
            "a shim is the toolchain, not something anybody installed"
        );
    }

    #[test]
    fn each_toolchain_is_a_package() {
        let home = Home::new("toolchains");
        let graph = home.graph();
        assert!(
            graph
                .package(&PackageId::new("rustup", "stable-aarch64-apple-darwin"))
                .is_some()
        );
    }

    #[test]
    fn rustup_check_is_read_line_by_line() {
        use super::parse_check;
        use crate::model::fact::Fact;

        let current = parse_check("stable-aarch64-apple-darwin - Up to date : 1.98.1 (48a229cea)");
        assert!(matches!(current, Some(Fact::UpToDate { .. })));

        let newer = parse_check(
            "nightly-aarch64-apple-darwin - Update available : 1.100.0-nightly (a69a63265) -> 1.100.0-nightly (574ff7d98)",
        );
        let Some(Fact::Outdated { package, latest }) = newer else {
            panic!("expected an update");
        };
        assert_eq!(package.name, "nightly-aarch64-apple-darwin");
        assert_eq!(latest.as_deref(), Some("1.100.0-nightly (574ff7d98)"));
    }

    /// Stands in for `rustup check`, so no test depends on the machine it runs
    /// on. Reports one toolchain the fixture has and one it does not.
    #[allow(clippy::unnecessary_wraps)]
    fn stub_check() -> Option<String> {
        Some(
            "stable-aarch64-apple-darwin - Up to date : 1.98.1 (48a229cea)\n\
             1.88-aarch64-apple-darwin - Update available : 1.88.0 (abc) -> 1.99.0 (def)\n"
                .to_owned(),
        )
    }

    #[test]
    fn a_toolchain_rustup_reports_but_disk_does_not_have_is_not_invented() {
        let home = Home::new("phantom");
        let rustup = Rustup::with_check(
            home.0.join(".rustup"),
            home.0.join(".cargo/bin"),
            stub_check,
        );

        let facts = rustup.updates().unwrap();
        assert_eq!(
            facts.len(),
            1,
            "only the toolchain that is installed: {facts:?}"
        );
        assert!(
            matches!(&facts[0], Fact::UpToDate { package } if package.name == "stable-aarch64-apple-darwin")
        );
    }

    #[test]
    fn a_toolchain_that_is_installed_and_stale_is_reported_with_the_newer_version() {
        let home = Home::new("stale");
        fs::create_dir_all(home.0.join(".rustup/toolchains/1.88-aarch64-apple-darwin"))
            .expect("create toolchain");
        let rustup = Rustup::with_check(
            home.0.join(".rustup"),
            home.0.join(".cargo/bin"),
            stub_check,
        );

        let facts = rustup.updates().unwrap();
        let newer = facts.iter().find_map(|f| match f {
            Fact::Outdated { package, latest } if package.name.starts_with("1.88") => {
                latest.clone()
            }
            _ => None,
        });
        assert_eq!(newer.as_deref(), Some("1.99.0 (def)"));
    }

    #[test]
    fn a_line_rustup_did_not_write_is_ignored() {
        use super::parse_check;
        assert!(parse_check("info: syncing channel updates").is_none());
        assert!(parse_check("").is_none());
    }

    #[test]
    fn checking_updates_never_runs_during_an_ordinary_scan() {
        // The default `updates` says nothing, so a source that has not
        // implemented it leaves its packages reading as unchecked rather than
        // as current.
        let home = Home::new("noupdates");
        let facts = home.rustup().scan().unwrap();
        assert!(
            !facts.iter().any(|f| matches!(
                f,
                crate::model::fact::Fact::UpToDate { .. }
                    | crate::model::fact::Fact::Outdated { .. }
            )),
            "scan must not claim to know what only the network can say"
        );
    }

    #[test]
    fn neither_being_installed_yields_no_facts_and_no_error() {
        let absent = PathBuf::from("/nonexistent/for/sure");
        assert_eq!(Cargo::new(absent.clone()).scan().unwrap(), Vec::new());
        assert_eq!(
            Rustup::new(absent.clone(), absent).scan().unwrap(),
            Vec::new()
        );
    }

    #[test]
    fn an_unparseable_manifest_is_reported_rather_than_swallowed() {
        let home = Home::new("badjson");
        fs::write(home.0.join(".cargo/.crates2.json"), "{not json").expect("write");
        let error = home.cargo().scan().unwrap_err();
        assert_eq!(error.source_name, "cargo");
        assert!(error.to_string().contains("parsing"), "{error}");
    }
}
