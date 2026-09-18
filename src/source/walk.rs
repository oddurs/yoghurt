//! What is actually on the machine.
//!
//! Every other source makes claims. This one says what is there, and the
//! difference between the two is where the interesting answers live: an
//! artifact nobody claims is an orphan, and a claim with no artifact is broken.
//! Neither is detected — both are what is left over once the claims are laid
//! against the ground.
//!
//! It is deliberately not an adapter for anything. It reads `$PATH` and the
//! application directories, resolves symlinks the way the shell would, and
//! emits what it finds.

use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};

use crate::model::fact::{Fact, ScanError, Source};

/// The ground-truth walk.
pub struct Walk {
    search_path: Vec<PathBuf>,
    application_dirs: Vec<PathBuf>,
    warnings: std::cell::RefCell<Vec<String>>,
}

impl Walk {
    /// The name this source reports itself under.
    pub const NAME: &'static str = "path";

    /// Walk the machine this process is running on.
    #[must_use]
    pub fn from_environment() -> Self {
        let search_path = std::env::var_os("PATH")
            .map(|path| std::env::split_paths(&path).collect())
            .unwrap_or_default();
        let home = std::env::var_os("HOME").map(PathBuf::from);
        let mut application_dirs = vec![PathBuf::from("/Applications")];
        application_dirs.extend(home.map(|home| home.join("Applications")));
        Self::new(search_path, application_dirs)
    }

    /// Walk directories given explicitly. Tests point this at a fixture tree.
    #[must_use]
    pub fn new(search_path: Vec<PathBuf>, application_dirs: Vec<PathBuf>) -> Self {
        Self {
            search_path,
            application_dirs,
            warnings: std::cell::RefCell::new(Vec::new()),
        }
    }

    /// Directories that could not be read. A machine always has a few `$PATH`
    /// entries pointing at nothing, and that is not worth failing over.
    #[must_use]
    pub fn warnings(&self) -> Vec<String> {
        self.warnings.borrow().clone()
    }

    fn warn(&self, message: String) {
        self.warnings.borrow_mut().push(message);
    }

    /// Everything runnable in one `$PATH` directory.
    fn walk_path_entry(&self, index: usize, directory: &Path, facts: &mut Vec<Fact>) {
        facts.push(Fact::SearchPath {
            index,
            directory: directory.to_owned(),
        });

        // A `$PATH` entry can itself be a link to somewhere else — Homebrew's
        // `opt/<name>` and `/Library/TeX/texbin` both are — and the files
        // inside it are then ordinary files, not links. Resolving the
        // directory once catches every entry under it for one `canonicalize`,
        // where resolving each entry cost 60ms of the 160 this walk used to
        // take. Without it `mise` appeared twice: once from its keg and once
        // as an orphan that was the very same file.
        let resolved = fs::canonicalize(directory)
            .ok()
            .filter(|canonical| canonical != directory);

        let entries = match fs::read_dir(directory) {
            Ok(entries) => entries,
            Err(error) => {
                // A `$PATH` entry that does not exist is ordinary, not a fault.
                if error.kind() != io::ErrorKind::NotFound {
                    self.warn(format!("{}: {error}", directory.display()));
                }
                return;
            }
        };

        for entry in entries.filter_map(Result::ok) {
            let link = entry.path();
            let Ok(command) = entry.file_name().into_string() else {
                continue;
            };

            // `file_type` comes off the directory read, so the common case —
            // a regular file that is not a link — costs one `lstat` and no
            // `canonicalize` at all. Resolving every entry instead of only the
            // links cost 60ms of the 160 this used to take.
            let symlink = entry.file_type().is_ok_and(|kind| kind.is_symlink());
            let metadata = if symlink {
                fs::metadata(&link)
            } else {
                entry.metadata()
            };

            match metadata {
                Ok(metadata) if is_executable(&metadata) => {
                    facts.push(Fact::Provides {
                        artifact: link.clone(),
                        command: command.clone(),
                    });
                    facts.push(Fact::Size {
                        artifact: link.clone(),
                        bytes: metadata.len(),
                    });
                    if symlink
                        && let Ok(target) = fs::canonicalize(&link)
                        && target != link
                    {
                        facts.push(Fact::Resolves { link, target });
                    } else if let Some(canonical) = &resolved {
                        facts.push(Fact::Resolves {
                            target: canonical.join(&command),
                            link,
                        });
                    }
                }
                Ok(_) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    // The entry exists in the directory but its target does
                    // not: a dangling symlink, left by an uninstall that did
                    // not finish. Still on `$PATH`, still shadowing whatever
                    // comes after it, and still broken.
                    facts.push(Fact::Provides {
                        artifact: link.clone(),
                        command,
                    });
                    facts.push(Fact::Missing { artifact: link });
                }
                Err(error) => self.warn(format!("{}: {error}", link.display())),
            }
        }
    }

    /// Application bundles in one directory.
    ///
    /// Bundles carry no size here on purpose: measuring one means walking every
    /// file inside it, and `Xcode.app` alone is fourteen gigabytes. That work
    /// belongs behind the cache in 0031, not in the first frame.
    fn walk_applications(&self, directory: &Path, facts: &mut Vec<Fact>) {
        let entries = match fs::read_dir(directory) {
            Ok(entries) => entries,
            Err(error) => {
                if error.kind() != io::ErrorKind::NotFound {
                    self.warn(format!("{}: {error}", directory.display()));
                }
                return;
            }
        };

        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if is_bundle(&path) {
                facts.push(Fact::Artifact { path });
            }
        }
    }
}

/// Runnable by somebody — owner, group or world.
fn is_executable(metadata: &fs::Metadata) -> bool {
    metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
}

/// A macOS application bundle. The filesystem is usually case-insensitive, so
/// the extension comparison is too.
fn is_bundle(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("app"))
}

impl Source for Walk {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn scan(&self) -> Result<Vec<Fact>, ScanError> {
        self.warnings.borrow_mut().clear();
        let mut facts = Vec::new();
        for (index, directory) in self.search_path.iter().enumerate() {
            self.walk_path_entry(index, directory, &mut facts);
        }
        for directory in &self.application_dirs {
            self.walk_applications(directory, &mut facts);
        }
        Ok(facts)
    }
}

#[cfg(test)]
mod tests {
    use super::Walk;
    use crate::model::fact::{Fact, Source as _};
    use crate::model::graph::Graph;
    use std::fs;
    use std::os::unix::fs::{PermissionsExt as _, symlink};
    use std::path::PathBuf;

    /// A fixture machine that removes itself.
    struct Machine(PathBuf);

    impl Machine {
        fn new(tag: &str) -> Self {
            let root =
                std::env::temp_dir().join(format!("yoghurt-walk-{tag}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).expect("create fixture root");
            Self(root)
        }

        fn dir(&self, path: &str) -> PathBuf {
            let dir = self.0.join(path);
            fs::create_dir_all(&dir).expect("create fixture directory");
            dir
        }

        fn executable(&self, path: &str, contents: &str) -> PathBuf {
            let file = self.0.join(path);
            fs::create_dir_all(file.parent().expect("has a parent")).expect("create parent");
            fs::write(&file, contents).expect("write executable");
            fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).expect("chmod");
            file
        }

        fn data(&self, path: &str, contents: &str) -> PathBuf {
            let file = self.0.join(path);
            fs::create_dir_all(file.parent().expect("has a parent")).expect("create parent");
            fs::write(&file, contents).expect("write file");
            fs::set_permissions(&file, fs::Permissions::from_mode(0o644)).expect("chmod");
            file
        }

        fn path(&self, path: &str) -> PathBuf {
            self.0.join(path)
        }
    }

    impl Drop for Machine {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// A machine shaped like the real one: a keg, a link into it from a
    /// directory on `$PATH`, a non-executable file, and an application.
    fn machine(tag: &str) -> (Machine, Walk) {
        let m = Machine::new(tag);
        let bin = m.dir("opt/bin");
        m.executable("opt/Cellar/ripgrep/15.2.0/bin/rg", "binary");
        symlink(m.path("opt/Cellar/ripgrep/15.2.0/bin/rg"), bin.join("rg")).expect("link rg");
        m.executable("usr/bin/rg", "older binary");
        m.data("opt/bin/README", "not executable");
        m.dir("Applications/Ghostty.app");
        m.dir("Applications/not-an-app");

        // A `$PATH` entry that is itself a link to a keg, holding a plain
        // file. Homebrew's `opt/<name>/bin` is exactly this shape.
        m.executable("opt/Cellar/mise/2026.9.6/bin/mise", "binary");
        symlink(
            m.path("opt/Cellar/mise/2026.9.6"),
            m.path("opt").join("mise"),
        )
        .expect("link the keg");

        let walk = Walk::new(
            vec![
                bin,
                m.path("usr/bin"),
                m.path("nothing/here"),
                m.path("opt/mise/bin"),
            ],
            vec![m.path("Applications")],
        );
        (m, walk)
    }

    #[test]
    fn every_path_directory_becomes_a_search_path_entry_in_order() {
        let (m, walk) = machine("order");
        let facts = walk.scan().unwrap();
        let entries: Vec<_> = facts
            .iter()
            .filter_map(|f| match f {
                Fact::SearchPath { index, directory } => Some((*index, directory.clone())),
                _ => None,
            })
            .collect();
        assert_eq!(
            entries,
            vec![
                (0, m.path("opt/bin")),
                (1, m.path("usr/bin")),
                (2, m.path("nothing/here")),
                (3, m.path("opt/mise/bin")),
            ]
        );
    }

    #[test]
    fn an_executable_provides_its_own_name() {
        let (m, walk) = machine("provides");
        let graph = Graph::from_facts(walk.scan().unwrap());
        let providers: Vec<_> = graph
            .commands()
            .find(|(name, _)| *name == "rg")
            .map(|(_, paths)| paths.iter().cloned().collect())
            .unwrap_or_default();
        assert_eq!(providers, vec![m.path("opt/bin/rg"), m.path("usr/bin/rg")]);
    }

    #[test]
    fn a_file_that_is_not_executable_provides_nothing() {
        let (_m, walk) = machine("nonexec");
        let graph = Graph::from_facts(walk.scan().unwrap());
        assert!(graph.commands().all(|(name, _)| name != "README"));
    }

    #[test]
    fn a_symlink_resolves_to_its_target_and_both_paths_survive() {
        let (m, walk) = machine("symlink");
        let graph = Graph::from_facts(walk.scan().unwrap());
        assert_eq!(
            graph.target_of(&m.path("opt/bin/rg")),
            Some(
                fs::canonicalize(m.path("opt/Cellar/ripgrep/15.2.0/bin/rg"))
                    .unwrap()
                    .as_path()
            ),
            "the link is what is on PATH; the target is what is owned"
        );
    }

    #[test]
    fn a_plain_file_inside_a_linked_directory_still_resolves_to_its_keg() {
        let (m, walk) = machine("linked-dir");
        let graph = Graph::from_facts(walk.scan().unwrap());
        // `opt/mise/bin/mise` is not a link; `opt/mise` is. Testing only the
        // leaf missed this, and `mise` showed up twice — once owned by its
        // keg, once as an orphan that was the very same file.
        assert_eq!(
            graph.target_of(&m.path("opt/mise/bin/mise")),
            Some(
                fs::canonicalize(m.path("opt/Cellar/mise/2026.9.6/bin/mise"))
                    .unwrap()
                    .as_path()
            ),
            "the directory is the link, and everything under it resolves"
        );
    }

    #[test]
    fn a_dangling_symlink_is_recorded_as_missing_rather_than_skipped() {
        let m = Machine::new("dangling");
        let bin = m.dir("bin");
        symlink(m.path("gone/binary"), bin.join("ghost")).expect("link ghost");
        let walk = Walk::new(vec![bin.clone()], Vec::new());

        let graph = Graph::from_facts(walk.scan().unwrap());
        let ghost = graph
            .artifact(&bin.join("ghost"))
            .expect("still an artifact");
        assert!(
            ghost.missing,
            "it is on PATH and it is broken; both are true"
        );
        assert!(
            graph.commands().any(|(name, _)| name == "ghost"),
            "it still shadows whatever comes after it"
        );
    }

    #[test]
    fn application_bundles_become_artifacts_and_other_directories_do_not() {
        let (m, walk) = machine("apps");
        let graph = Graph::from_facts(walk.scan().unwrap());
        assert!(
            graph
                .artifact(&m.path("Applications/Ghostty.app"))
                .is_some()
        );
        assert!(graph.artifact(&m.path("Applications/not-an-app")).is_none());
    }

    #[test]
    fn a_path_entry_that_does_not_exist_is_ordinary_and_silent() {
        let (_m, walk) = machine("absent");
        walk.scan().unwrap();
        assert!(walk.warnings().is_empty(), "{:?}", walk.warnings());
    }

    #[test]
    fn ownership_follows_the_link_into_the_keg() {
        let (m, walk) = machine("ownership");
        let keg = fs::canonicalize(m.path("opt/Cellar/ripgrep/15.2.0")).unwrap();
        let mut facts = walk.scan().unwrap();
        facts.push(Fact::Owns {
            package: crate::model::fact::PackageId::new("homebrew", "ripgrep"),
            artifact: keg,
        });

        let graph = Graph::from_facts(facts);
        assert_eq!(
            graph.owners_of(&m.path("opt/bin/rg")),
            vec![&crate::model::fact::PackageId::new("homebrew", "ripgrep")],
            "the thing on PATH is a link, and the keg behind it is what Homebrew owns"
        );
    }

    #[test]
    fn a_binary_nobody_owns_has_no_owner() {
        let (m, walk) = machine("orphan");
        let graph = Graph::from_facts(walk.scan().unwrap());
        assert!(
            graph.owners_of(&m.path("usr/bin/rg")).is_empty(),
            "this is what makes an orphan an orphan"
        );
    }

    #[test]
    fn the_walk_reads_nothing_outside_the_directories_it_was_given() {
        let m = Machine::new("scoped");
        let walk = Walk::new(vec![m.path("bin")], vec![m.path("Applications")]);
        assert!(walk.scan().unwrap().iter().all(|fact| match fact {
            Fact::SearchPath { directory, .. } => directory.starts_with(m.path("")),
            Fact::Artifact { path } | Fact::Missing { artifact: path } => {
                path.starts_with(m.path(""))
            }
            _ => true,
        }));
    }
}
