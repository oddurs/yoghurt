//! Global node packages.
//!
//! Where these live depends on how node was installed, so there is no single
//! answer to "where are they" — Homebrew's prefix, `/usr/local`, a user prefix,
//! or inside whichever version manager is in charge. This looks in the places
//! they are, rather than asking a `node` that may not be the one that installed
//! them.
//!
//! No subprocess: every package carries its own `package.json`, and the `bin`
//! map in it is what makes the commands it provides a fact rather than a guess.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::model::fact::{Fact, PackageId, ScanError, Source};
use crate::source::{children, size_of};

/// Globally installed node packages.
pub struct Node {
    roots: Vec<PathBuf>,
}

impl Node {
    /// The name this source reports itself under.
    pub const NAME: &'static str = "npm";

    /// The node module directories on this machine.
    #[must_use]
    pub fn from_environment() -> Self {
        let home = std::env::var_os("HOME").map(PathBuf::from);
        let mut roots = vec![
            PathBuf::from("/opt/homebrew/lib/node_modules"),
            PathBuf::from("/usr/local/lib/node_modules"),
        ];
        if let Some(home) = home {
            roots.push(home.join(".npm-global/lib/node_modules"));
            roots.push(home.join(".local/lib/node_modules"));
            // Version managers keep a tree per installed runtime, so the
            // packages live under whichever one is current.
            for manager in [".local/state/fnm_multishells", ".volta/tools/image/node"] {
                roots.push(home.join(manager));
            }
        }
        Self { roots }
    }

    /// Node packages somewhere else. Tests point this at a fixture tree.
    #[must_use]
    pub fn new(roots: Vec<PathBuf>) -> Self {
        Self { roots }
    }
}

/// Only what this needs out of a `package.json`.
#[derive(Debug, Deserialize)]
struct Manifest {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    description: Option<String>,
    /// Either `{"rg": "bin/rg.js"}` or a bare string naming one command.
    #[serde(default)]
    bin: Option<Bin>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Bin {
    /// `"bin": "cli.js"` — one command, named after the package rather than
    /// after the path, which is why the path itself is not read.
    One(#[allow(dead_code)] String),
    /// `"bin": {"npm": "...", "npx": "..."}`
    Many(BTreeMap<String, String>),
}

impl Manifest {
    /// The command names this package puts on the path.
    fn commands(&self, package: &str) -> Vec<String> {
        match &self.bin {
            // A bare string is named after the package, minus any scope.
            Some(Bin::One(_)) => {
                vec![package.rsplit('/').next().unwrap_or(package).to_owned()]
            }
            Some(Bin::Many(map)) => map.keys().cloned().collect(),
            None => Vec::new(),
        }
    }
}

/// Every package directory under one root.
///
/// A scoped package is `@scope/name`, one directory deeper, and is the thing
/// most often missed — `.bin` is a directory of symlinks and not a package.
fn packages(root: &Path) -> Vec<(String, PathBuf)> {
    let mut found = Vec::new();
    for entry in children(root) {
        let Some(name) = entry.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if name == ".bin" {
            continue;
        }
        if name.starts_with('@') {
            for scoped in children(&entry) {
                if let Some(leaf) = scoped.file_name().and_then(|n| n.to_str()) {
                    found.push((format!("{name}/{leaf}"), scoped));
                }
            }
        } else {
            found.push((name.to_owned(), entry));
        }
    }
    found
}

impl Source for Node {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn scan(&self) -> Result<Vec<Fact>, ScanError> {
        let mut facts = Vec::new();
        let mut seen = Vec::new();

        for root in &self.roots {
            for (name, path) in packages(root) {
                // A version manager can expose the same tree at several paths.
                let Ok(real) = fs::canonicalize(&path) else {
                    continue;
                };
                if seen.contains(&real) {
                    continue;
                }
                seen.push(real.clone());

                let Ok(text) = fs::read_to_string(path.join("package.json")) else {
                    continue;
                };
                // A malformed manifest is one bad package, not a bad scan.
                let Ok(manifest) = serde_json::from_str::<Manifest>(&text) else {
                    continue;
                };

                let id = PackageId::new(Self::NAME, manifest.name.as_deref().unwrap_or(&name));
                facts.push(Fact::Package {
                    id: id.clone(),
                    version: manifest.version.clone(),
                });
                // Nothing lands in a global node_modules as a dependency of
                // something else global.
                facts.push(Fact::Wanted {
                    package: id.clone(),
                });
                facts.push(Fact::Owns {
                    package: id.clone(),
                    artifact: real.clone(),
                });
                facts.push(Fact::Size {
                    artifact: real,
                    bytes: size_of(&path),
                });

                if let Some(text) = manifest.description.clone() {
                    facts.push(Fact::Describes {
                        package: id.clone(),
                        text,
                    });
                }
                // The `bin` map is why a package is findable by the command you
                // type rather than only by its own name.
                for command in manifest.commands(&name) {
                    facts.push(Fact::Provides {
                        artifact: path.join("__bin__"),
                        command,
                    });
                }
            }
        }
        Ok(facts)
    }
}

#[cfg(test)]
mod tests {
    use super::Node;
    use crate::Graph;
    use crate::model::fact::{Fact, PackageId, Source as _};
    use std::fs;
    use std::path::PathBuf;

    struct Modules(PathBuf);

    impl Modules {
        fn new(tag: &str) -> Self {
            let root =
                std::env::temp_dir().join(format!("yoghurt-node-{tag}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            let modules = root.join("lib/node_modules");

            Self::package(
                &modules.join("npm"),
                r#"{"name":"npm","version":"11.19.1",
                "description":"a package manager for JavaScript",
                "bin":{"npm":"bin/npm-cli.js","npx":"bin/npx-cli.js"}}"#,
            );
            Self::package(
                &modules.join("@vendor/sdk"),
                r#"{"name":"@vendor/sdk",
                "version":"0.30.1","description":"A scoped package"}"#,
            );
            Self::package(
                &modules.join("prettier"),
                r#"{"name":"prettier","version":"3.4.2",
                "bin":"bin/prettier.cjs"}"#,
            );
            Self::package(&modules.join("broken"), "{not json");
            // `.bin` is symlinks, not a package.
            fs::create_dir_all(modules.join(".bin")).expect("create .bin");

            Self(root)
        }

        fn package(dir: &std::path::Path, manifest: &str) {
            fs::create_dir_all(dir).expect("create package");
            fs::write(dir.join("package.json"), manifest).expect("write manifest");
        }

        fn source(&self) -> Node {
            Node::new(vec![self.0.join("lib/node_modules")])
        }

        fn graph(&self) -> Graph {
            Graph::from_facts(self.source().scan().expect("scan"))
        }
    }

    impl Drop for Modules {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn npm(name: &str) -> PackageId {
        PackageId::new("npm", name)
    }

    #[test]
    fn every_package_becomes_a_package() {
        let modules = Modules::new("packages");
        let graph = modules.graph();
        assert!(graph.package(&npm("npm")).is_some());
        assert!(graph.package(&npm("prettier")).is_some());
    }

    #[test]
    fn a_scoped_package_is_found_one_directory_deeper() {
        let modules = Modules::new("scoped");
        let graph = modules.graph();
        assert!(
            graph.package(&npm("@vendor/sdk")).is_some(),
            "the thing most often missed"
        );
    }

    #[test]
    fn the_bin_map_becomes_the_commands_it_provides() {
        let modules = Modules::new("bin");
        let facts = modules.source().scan().unwrap();
        let commands: Vec<String> = facts
            .iter()
            .filter_map(|f| match f {
                Fact::Provides { command, .. } => Some(command.clone()),
                _ => None,
            })
            .collect();
        assert!(commands.contains(&"npm".to_owned()));
        assert!(
            commands.contains(&"npx".to_owned()),
            "both, not just the package name"
        );
    }

    #[test]
    fn a_bare_bin_string_is_named_after_its_package() {
        let modules = Modules::new("barebin");
        let facts = modules.source().scan().unwrap();
        assert!(
            facts
                .iter()
                .any(|f| matches!(f, Fact::Provides { command, .. }
            if command == "prettier"))
        );
    }

    #[test]
    fn dot_bin_is_symlinks_rather_than_a_package() {
        let modules = Modules::new("dotbin");
        assert!(modules.graph().package(&npm(".bin")).is_none());
    }

    #[test]
    fn a_malformed_manifest_is_one_bad_package_not_a_bad_scan() {
        let modules = Modules::new("broken");
        let graph = modules.graph();
        assert!(graph.package(&npm("broken")).is_none());
        assert!(
            graph.package(&npm("npm")).is_some(),
            "the rest still arrived"
        );
    }

    #[test]
    fn a_description_comes_through() {
        let modules = Modules::new("desc");
        assert_eq!(
            modules
                .graph()
                .package(&npm("npm"))
                .unwrap()
                .describes
                .as_deref(),
            Some("a package manager for JavaScript")
        );
    }

    #[test]
    fn a_package_is_always_wanted() {
        let modules = Modules::new("wanted");
        assert!(modules.graph().package(&npm("prettier")).unwrap().wanted());
    }

    #[test]
    fn the_same_tree_reached_by_two_roots_is_counted_once() {
        let modules = Modules::new("dedup");
        let root = modules.0.join("lib/node_modules");
        let twice = Node::new(vec![root.clone(), root]);
        let graph = Graph::from_facts(twice.scan().unwrap());
        assert_eq!(
            graph.packages().count(),
            3,
            "a version manager exposes one tree many ways"
        );
    }

    #[test]
    fn no_node_modules_anywhere_yields_no_facts_and_no_error() {
        let absent = Node::new(vec![PathBuf::from("/nonexistent/for/sure")]);
        assert_eq!(absent.scan().unwrap(), Vec::new());
    }
}
