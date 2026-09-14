//! See what is installed on this machine, and where it came from.
//!
//! The model is one graph over three kinds of node — a package a manager
//! records, an artifact on disk, and a command resolvable on `PATH` — and every
//! state the interface shows is a query over it rather than a feature somebody
//! remembered to build.
//!
//! [`fact`] is the boundary that makes that work: an adapter emits [`Fact`]s
//! and knows nothing else about the program.

pub mod fact;

pub use fact::{Fact, PackageId, ScanError, Source};

use std::fs;
use std::path::{Path, PathBuf};

/// One package manager's inventory, reduced to a count.
///
/// The summary the bare `yoghurt` table is built from, until the graph replaces
/// it. Distinct from [`fact::Source`], which is the adapter trait.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceSummary {
    /// How this source is named in the interface.
    pub name: &'static str,
    /// The directory the count came from.
    pub root: PathBuf,
    /// How many packages it holds.
    pub count: usize,
}

/// Shims that `rustup` puts in `~/.cargo/bin`. They are the toolchain, not
/// things anyone installed, and counting them would overstate every machine
/// with Rust on it by a dozen.
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

/// Every source present on this machine, in the order the interface shows them.
///
/// A source that is not installed is absent from the result rather than
/// present with a count of zero: an empty row invites the question "should I
/// install that?", which is not what this tool is for.
#[must_use]
pub fn survey() -> Vec<SourceSummary> {
    let home = home();
    let mut found = Vec::new();

    if let Some(prefix) = homebrew_prefix() {
        push(&mut found, "homebrew", prefix.join("Cellar"), &|_| true);
        push(
            &mut found,
            "homebrew casks",
            prefix.join("Caskroom"),
            &|_| true,
        );
    }

    push(&mut found, "cargo", home.join(".cargo/bin"), &|name| {
        !RUSTUP_SHIMS.contains(&name)
    });
    push(
        &mut found,
        "rustup",
        home.join(".rustup/toolchains"),
        &|_| true,
    );
    push(&mut found, "go", go_bin(&home), &|_| true);
    push(
        &mut found,
        "pipx",
        home.join(".local/share/pipx/venvs"),
        &|_| true,
    );
    push(
        &mut found,
        "uv tools",
        home.join(".local/share/uv/tools"),
        &|_| true,
    );

    if let Some(root) = first_existing(&node_module_roots(&home)) {
        // A scoped package is `@scope/name`, one directory deeper.
        let count = entries(&root, &|name| name != ".bin" && !name.starts_with('@')).len()
            + entries(&root, &|name| name.starts_with('@'))
                .iter()
                .map(|scope| entries(&root.join(scope), &|_| true).len())
                .sum::<usize>();
        if count > 0 {
            found.push(SourceSummary {
                name: "npm global",
                root,
                count,
            });
        }
    }

    for dir in application_dirs(&home) {
        push(&mut found, "applications", dir, &|name| is_bundle(name));
    }

    found
}

/// Count `root` and record it when it holds anything.
fn push(
    into: &mut Vec<SourceSummary>,
    name: &'static str,
    root: PathBuf,
    keep: &dyn Fn(&str) -> bool,
) {
    let count = entries(&root, keep).len();
    if count > 0 {
        into.push(SourceSummary { name, root, count });
    }
}

/// Names directly inside `dir` that `keep` accepts, ignoring dotfiles and any
/// directory that cannot be read.
#[must_use]
pub fn entries(dir: &Path, keep: &dyn Fn(&str) -> bool) -> Vec<String> {
    let Ok(read) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = read
        .filter_map(Result::ok)
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|name| !name.starts_with('.') && keep(name))
        .collect();
    names.sort_unstable();
    names
}

/// Where Homebrew lives: what the environment says, else the two prefixes
/// Homebrew itself installs to.
#[must_use]
pub fn homebrew_prefix() -> Option<PathBuf> {
    if let Some(prefix) = std::env::var_os("HOMEBREW_PREFIX").map(PathBuf::from)
        && prefix.join("Cellar").is_dir()
    {
        return Some(prefix);
    }
    first_existing(&[PathBuf::from("/opt/homebrew"), PathBuf::from("/usr/local")])
        .filter(|prefix| prefix.join("Cellar").is_dir())
}

/// macOS application bundles. The filesystem is usually case-insensitive, so
/// the extension comparison is too.
fn is_bundle(name: &str) -> bool {
    Path::new(name)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("app"))
}

fn go_bin(home: &Path) -> PathBuf {
    if let Some(bin) = std::env::var_os("GOBIN") {
        return PathBuf::from(bin);
    }
    std::env::var_os("GOPATH").map_or_else(|| home.join("go/bin"), |p| PathBuf::from(p).join("bin"))
}

fn node_module_roots(home: &Path) -> Vec<PathBuf> {
    vec![
        home.join(".npm-global/lib/node_modules"),
        PathBuf::from("/opt/homebrew/lib/node_modules"),
        PathBuf::from("/usr/local/lib/node_modules"),
        home.join(".local/lib/node_modules"),
    ]
}

fn application_dirs(home: &Path) -> Vec<PathBuf> {
    vec![PathBuf::from("/Applications"), home.join("Applications")]
}

fn first_existing(candidates: &[PathBuf]) -> Option<PathBuf> {
    candidates.iter().find(|p| p.is_dir()).cloned()
}

fn home() -> PathBuf {
    std::env::var_os("HOME").map_or_else(|| PathBuf::from("/"), PathBuf::from)
}

/// Replace the home directory with `~`, so a path fits a terminal column.
#[must_use]
pub fn abbreviate(path: &Path) -> String {
    let home = home();
    path.strip_prefix(&home).map_or_else(
        |_| path.display().to_string(),
        |rest| format!("~/{}", rest.display()),
    )
}

#[cfg(test)]
mod tests {
    use super::{SourceSummary, abbreviate, entries, is_bundle, survey};
    use std::fs;
    use std::path::{Path, PathBuf};

    /// A scratch directory that removes itself. Cheaper than a dependency for
    /// the two tests that need one.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(tag: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("yoghurt-{tag}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).expect("create scratch directory");
            Self(dir)
        }

        fn touch(&self, names: &[&str]) -> &Path {
            for name in names {
                fs::create_dir_all(self.0.join(name)).expect("create entry");
            }
            &self.0
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn counts_entries_and_skips_dotfiles() {
        let scratch = Scratch::new("entries");
        let dir = scratch.touch(&["ripgrep", "fd", ".DS_Store", ".keep"]);
        assert_eq!(entries(dir, &|_| true), vec!["fd", "ripgrep"]);
    }

    #[test]
    fn applies_the_filter() {
        let scratch = Scratch::new("filter");
        let dir = scratch.touch(&["Alfred.app", "notes.txt"]);
        assert_eq!(entries(dir, &is_bundle), vec!["Alfred.app"]);
    }

    #[test]
    fn an_unreadable_directory_is_empty_rather_than_an_error() {
        assert!(entries(Path::new("/nonexistent/for/sure"), &|_| true).is_empty());
    }

    #[test]
    fn abbreviates_the_home_directory() {
        let home = PathBuf::from(std::env::var_os("HOME").unwrap_or_else(|| "/".into()));
        assert_eq!(abbreviate(&home.join(".cargo/bin")), "~/.cargo/bin");
        assert_eq!(abbreviate(Path::new("/usr/local/bin")), "/usr/local/bin");
    }

    #[test]
    fn a_survey_never_reports_an_empty_source() {
        assert!(
            survey()
                .iter()
                .all(|SourceSummary { count, .. }| *count > 0)
        );
    }
}
