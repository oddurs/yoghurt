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

impl Source for Cargo {
    fn name(&self) -> &'static str {
        Self::NAME
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
        home.join("toolchains")
            .is_dir()
            .then_some(Self { home, cargo_bin })
    }

    /// Rustup rooted somewhere else. Tests point this at a fixture tree.
    #[must_use]
    pub fn new(home: PathBuf, cargo_bin: PathBuf) -> Self {
        Self { home, cargo_bin }
    }
}

impl Source for Rustup {
    fn name(&self) -> &'static str {
        Self::NAME
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
