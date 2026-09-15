//! What a source is allowed to say about the machine.
//!
//! An adapter's entire job is to emit [`Fact`]s. It knows nothing about the
//! interface, nothing about the other adapters, and nothing about how any of
//! this is scored or displayed — so adding a package manager is one file that
//! touches nothing else, and every adapter is testable as a pure function from
//! a directory tree to a list of assertions.
//!
//! Facts are additive and order-independent. Two sources claiming the same
//! artifact is a thing that happens on a real machine, so it is representable
//! here rather than an error; reconciling them is the graph's job.

use std::error::Error;
use std::fmt;
use std::path::PathBuf;
use std::time::SystemTime;

/// What a package manager calls a package.
///
/// Identity is the pair, not the name: Homebrew's `node` and npm's `node` are
/// different things that happen to share a word.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PackageId {
    /// The source that records it, as reported by [`Source::name`].
    pub source: String,
    /// The name that source knows it by.
    pub name: String,
}

impl PackageId {
    /// Name a package within a source.
    pub fn new(source: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            name: name.into(),
        }
    }
}

impl fmt::Display for PackageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.source, self.name)
    }
}

/// One assertion about the machine.
///
/// Every variant states something a source observed. Nothing here is a
/// conclusion — "orphan", "shadowed" and "pulled in" are questions answered by
/// the graph, not facts anybody emits.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Fact {
    /// This package exists, at this version if the source knows one.
    ///
    /// A package that appears in no other fact is still installed, so sources
    /// emit this for everything they record.
    Package {
        /// Which package.
        id: PackageId,
        /// The installed version, where the source reports one.
        version: Option<String>,
    },

    /// Somebody asked for this package deliberately.
    ///
    /// Its absence is what makes a package "pulled in": present on the machine,
    /// but only ever as a consequence of something else.
    Wanted {
        /// Which package.
        package: PackageId,
    },

    /// This package put this path on disk.
    ///
    /// An artifact with no `Owns` fact from anyone is an orphan.
    Owns {
        /// Which package.
        package: PackageId,
        /// The path it is responsible for.
        artifact: PathBuf,
    },

    /// This package needs another in order to work.
    DependsOn {
        /// The dependent.
        package: PackageId,
        /// What it needs.
        on: PackageId,
        /// Whether the package asked for this itself, rather than inheriting it
        /// through one of its own dependencies. Keeps a why-chain short.
        declared_directly: bool,
    },

    /// This path is on disk.
    ///
    /// Most artifacts arrive implicitly, named by whoever owns or provides
    /// them. An application bundle is owned by nobody and provides no command,
    /// so without this it would not exist — which is exactly backwards, since
    /// unowned things are the most interesting on the machine.
    Artifact {
        /// The path.
        path: PathBuf,
    },

    /// This path is a symlink to that one.
    ///
    /// The shell finds `/opt/homebrew/bin/rg`; Homebrew owns the keg the link
    /// points into. Ownership follows the link, while `$PATH` position belongs
    /// to the link itself, so both paths have to survive.
    Resolves {
        /// The path as found.
        link: PathBuf,
        /// What it points at, fully resolved.
        target: PathBuf,
    },

    /// This path is referred to but is not there.
    ///
    /// A dangling symlink, or a package claiming something that has been
    /// deleted underneath it.
    Missing {
        /// The path that is absent.
        artifact: PathBuf,
    },

    /// This artifact can be run under this command name.
    ///
    /// Two artifacts providing the same name is a contest; which one wins is
    /// decided by [`Fact::SearchPath`] order, in the graph.
    Provides {
        /// The executable on disk.
        artifact: PathBuf,
        /// The bare name it is invoked by.
        command: String,
    },

    /// This artifact occupies this many bytes.
    Size {
        /// The path measured.
        artifact: PathBuf,
        /// Its size, including everything beneath it if it is a directory.
        bytes: u64,
    },

    /// This package arrived at this time.
    InstalledAt {
        /// Which package.
        package: PackageId,
        /// When the source says it was installed.
        at: SystemTime,
    },

    /// What this package is for, in a sentence somebody wrote.
    ///
    /// Homebrew has one for every formula, npm calls it `description`, and
    /// cargo knows where a crate came from. It is what makes the difference
    /// between a list of names and a list you can read.
    Describes {
        /// Which package.
        package: PackageId,
        /// The sentence.
        text: String,
    },

    /// A label somebody inferred, rather than observed.
    ///
    /// Distinct from [`Fact::Describes`], which is a sentence a source wrote
    /// down. This is a guess, and it stays marked as one everywhere it appears.
    Labelled {
        /// Which package.
        package: PackageId,
        /// The category it was put in.
        label: String,
    },

    /// Somebody looked, and this is the newest there is.
    ///
    /// Distinct from the absence of [`Fact::Outdated`], which only means nobody
    /// asked. "Current" and "not checked" look identical without this, and 75
    /// packages on a real machine were in the second state while appearing to
    /// be in the first.
    UpToDate {
        /// Which package.
        package: PackageId,
    },

    /// A newer version than the installed one is published.
    Outdated {
        /// Which package.
        package: PackageId,
        /// The newer version, where the source names one.
        latest: Option<String>,
    },

    /// This directory is entry `index` of the command search path.
    ///
    /// Emitted by the walk rather than by any package manager: it is what makes
    /// the contest between two providers of one command resolvable.
    SearchPath {
        /// Position in `$PATH`, counting from zero. Earlier wins.
        index: usize,
        /// The directory itself.
        directory: PathBuf,
    },
}

/// A source that could not be read.
///
/// A failing source must never take the others down with it, so this carries
/// enough to say which one failed and why without unwinding anything.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScanError {
    /// The source that failed, as reported by [`Source::name`]. Named for what
    /// it holds rather than `source`, which on an [`Error`] means the cause.
    pub source_name: &'static str,
    /// What went wrong, in a sentence fit to show a person.
    pub message: String,
}

impl ScanError {
    /// Record a failure against a named source.
    pub fn new(source_name: &'static str, message: impl Into<String>) -> Self {
        Self {
            source_name,
            message: message.into(),
        }
    }
}

impl fmt::Display for ScanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.source_name, self.message)
    }
}

impl Error for ScanError {}

/// One package manager, reduced to the only thing the rest of the program wants
/// from it.
///
/// `name` is identity, not knowledge: it labels the facts and lets a partial
/// scan say which source failed. It is deliberately the only other method.
pub trait Source {
    /// How this source is named, in facts and in the interface.
    fn name(&self) -> &'static str;

    /// Everything this source can say about the machine.
    ///
    /// # Errors
    ///
    /// Returns [`ScanError`] when the source exists but could not be read. A
    /// source that is simply not installed is not an error: it returns no
    /// facts.
    fn scan(&self) -> Result<Vec<Fact>, ScanError>;

    /// What is newer than what is installed.
    ///
    /// Separate from [`Source::scan`] because for most sources this is a
    /// network round trip, and yoghurt does not touch the network unless it was
    /// asked to. Called only when somebody asks for it.
    ///
    /// The default is to say nothing, which leaves a package reading as "not
    /// checked" rather than falsely as "current".
    ///
    /// # Errors
    ///
    /// Returns [`ScanError`] when the check was attempted and failed. Being
    /// unable to check is never a reason to change what is already known.
    fn updates(&self) -> Result<Vec<Fact>, ScanError> {
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::{Fact, PackageId, ScanError, Source};
    use std::collections::HashSet;
    use std::fs;
    use std::path::{Path, PathBuf};

    /// A scratch directory that removes itself.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(tag: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("yoghurt-{tag}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).expect("create scratch directory");
            Self(dir)
        }

        fn file(&self, name: &str, contents: &str) -> PathBuf {
            let path = self.0.join(name);
            fs::write(&path, contents).expect("write fixture file");
            path
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// The shape every real adapter has: a root goes in, facts come out, and
    /// nothing in between touches the machine this is running on.
    struct DirectorySource {
        root: PathBuf,
    }

    impl DirectorySource {
        const NAME: &'static str = "fixture";
    }

    impl Source for DirectorySource {
        fn name(&self) -> &'static str {
            Self::NAME
        }

        fn scan(&self) -> Result<Vec<Fact>, ScanError> {
            let entries = fs::read_dir(&self.root).map_err(|e| {
                ScanError::new("fixture", format!("reading {}: {e}", self.root.display()))
            })?;

            let mut names: Vec<String> = entries
                .filter_map(Result::ok)
                .filter_map(|e| e.file_name().into_string().ok())
                .collect();
            names.sort_unstable();

            let mut facts = Vec::new();
            for name in names {
                let id = PackageId::new(Self::NAME, &name);
                let artifact = self.root.join(&name);
                let bytes = fs::metadata(&artifact).map_or(0, |m| m.len());
                facts.push(Fact::Package {
                    id: id.clone(),
                    version: Some("1.0.0".to_owned()),
                });
                facts.push(Fact::Owns {
                    package: id.clone(),
                    artifact: artifact.clone(),
                });
                facts.push(Fact::Size { artifact, bytes });
                facts.push(Fact::Wanted { package: id });
            }
            Ok(facts)
        }
    }

    #[test]
    fn a_fixture_directory_yields_exactly_the_expected_facts() {
        let scratch = Scratch::new("fact-fixture");
        let rg = scratch.file("ripgrep", "xxxx");
        let source = DirectorySource {
            root: scratch.path().to_owned(),
        };

        let id = PackageId::new("fixture", "ripgrep");
        assert_eq!(
            source.scan().unwrap(),
            vec![
                Fact::Package {
                    id: id.clone(),
                    version: Some("1.0.0".to_owned())
                },
                Fact::Owns {
                    package: id.clone(),
                    artifact: rg.clone()
                },
                Fact::Size {
                    artifact: rg,
                    bytes: 4
                },
                Fact::Wanted { package: id },
            ]
        );
    }

    #[test]
    fn a_source_that_cannot_be_read_names_itself() {
        let source = DirectorySource {
            root: PathBuf::from("/nonexistent/for/sure"),
        };
        let err = source.scan().unwrap_err();
        assert_eq!(err.source_name, "fixture");
        assert!(
            err.to_string().starts_with("fixture: reading /nonexistent"),
            "{err}"
        );
    }

    #[test]
    fn the_same_fact_twice_is_representable_rather_than_an_error() {
        let fact = Fact::Wanted {
            package: PackageId::new("homebrew", "ripgrep"),
        };
        let facts = [fact.clone(), fact.clone()];
        assert_eq!(facts.len(), 2, "duplicates survive the vec");
        assert_eq!(
            facts.iter().collect::<HashSet<_>>().len(),
            1,
            "and collapse on identity"
        );
    }

    #[test]
    fn a_package_is_identified_by_its_source_as_well_as_its_name() {
        assert_ne!(
            PackageId::new("homebrew", "node"),
            PackageId::new("npm", "node")
        );
        assert_eq!(
            PackageId::new("homebrew", "node").to_string(),
            "homebrew:node"
        );
    }

    #[test]
    fn two_sources_may_claim_one_artifact() {
        let artifact = PathBuf::from("/opt/homebrew/bin/node");
        let facts = [
            Fact::Owns {
                package: PackageId::new("homebrew", "node"),
                artifact: artifact.clone(),
            },
            Fact::Owns {
                package: PackageId::new("nvm", "node"),
                artifact,
            },
        ];
        assert_eq!(facts.len(), 2, "disagreement is data, not a panic");
    }
}
