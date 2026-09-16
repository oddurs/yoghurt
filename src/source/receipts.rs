//! Who installed the things nothing claims.
//!
//! macOS keeps a receipt for every `.pkg` ever installed — a hundred of them on
//! an ordinary machine — and `pkgutil` will say which one put a given file
//! there. That is how 36 TeX Live files stop being orphans and become what they
//! are: a cask's installer, which put them somewhere Homebrew does not look.
//!
//! This is not a [`crate::model::fact::Source`]. A source says what a manager
//! installed; this asks a question *about paths the graph already knows are
//! unclaimed*, so it runs after assembly and only over what is left.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;

use crate::model::fact::{Fact, PackageId};
use crate::model::graph::Graph;

/// The name these are reported under.
pub const NAME: &str = "installer";

/// How many paths to ask about at most.
///
/// Each answer is a subprocess. A machine with thousands of unclaimed files
/// should not spend a minute learning that most of them have no receipt either.
const LIMIT: usize = 200;

/// Ask who installed each unclaimed path, and claim what can be claimed.
///
/// Only paths outside the home directory are asked about: receipts record
/// system installers, and nothing in `~` was put there by one.
#[must_use]
pub fn claim(graph: &Graph) -> Vec<Fact> {
    claim_with(graph, owner_of)
}

/// The same, with the lookup supplied. Tests never run `pkgutil`.
#[must_use]
pub fn claim_with(graph: &Graph, ask: fn(&Path) -> Option<String>) -> Vec<Fact> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let candidates: Vec<PathBuf> = graph
        .unclaimed()
        .into_iter()
        .filter(|path| home.as_ref().is_none_or(|home| !path.starts_with(home)))
        .map(Path::to_owned)
        .take(LIMIT)
        .collect();

    if candidates.is_empty() {
        return Vec::new();
    }

    // Each answer is a subprocess that spends its time blocked, so they run at
    // once. Forty in sequence is a second nobody should wait for.
    let answers: Vec<(PathBuf, Option<String>)> = thread::scope(|scope| {
        let running: Vec<_> = candidates
            .into_iter()
            .map(|path| scope.spawn(move || (path.clone(), ask(&path))))
            .collect();
        running
            .into_iter()
            .filter_map(|handle| handle.join().ok())
            .collect()
    });

    let mut facts = Vec::new();
    let mut named: Vec<String> = Vec::new();
    for (path, owner) in answers {
        let Some(pkgid) = owner else {
            continue;
        };
        let id = PackageId::new(NAME, &pkgid);
        if !named.contains(&pkgid) {
            named.push(pkgid);
            facts.push(Fact::Package {
                id: id.clone(),
                version: None,
            });
            // An installer package was run deliberately, by somebody.
            facts.push(Fact::Wanted {
                package: id.clone(),
            });
            facts.push(Fact::Describes {
                package: id.clone(),
                text: "installed by a macOS installer package".to_owned(),
            });
        }
        facts.push(Fact::Owns {
            package: id,
            artifact: path,
        });
    }
    facts
}

/// Which receipt claims this path, if any.
fn owner_of(path: &Path) -> Option<String> {
    let output = Command::new("pkgutil")
        .arg("--file-info")
        .arg(path)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.strip_prefix("pkgid: "))
        .map(|id| id.trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::{NAME, claim_with};
    use crate::Graph;
    use crate::model::fact::{Fact, PackageId};
    use std::path::{Path, PathBuf};

    /// Only what is left over is worth a subprocess.
    fn explode_owned(_: &Path) -> Option<String> {
        panic!("something already owned must not be asked about");
    }

    /// No receipt ever put anything in a home directory.
    fn explode_home(_: &Path) -> Option<String> {
        panic!("a path under HOME must not be asked about");
    }

    /// Stands in for `pkgutil`, so no test runs it.
    fn stub(path: &Path) -> Option<String> {
        path.starts_with("/usr/local/texlive")
            .then(|| "org.tug.mactex.basictex2025".to_owned())
    }

    fn machine() -> Graph {
        Graph::from_facts([
            Fact::Artifact {
                path: PathBuf::from("/usr/local/texlive/bin/tex"),
            },
            Fact::Artifact {
                path: PathBuf::from("/usr/local/texlive/bin/pdftex"),
            },
            Fact::Artifact {
                path: PathBuf::from("/opt/mystery/thing"),
            },
        ])
    }

    #[test]
    fn a_receipt_claims_what_it_installed() {
        let facts = claim_with(&machine(), stub);
        let graph = Graph::from_facts(facts);
        let id = PackageId::new(NAME, "org.tug.mactex.basictex2025");

        assert_eq!(
            graph.owners_of(Path::new("/usr/local/texlive/bin/tex")),
            vec![&id],
            "a cask's installer put it somewhere Homebrew does not look"
        );
        assert!(
            graph.package(&id).unwrap().wanted,
            "somebody ran the installer"
        );
    }

    #[test]
    fn one_receipt_owning_many_files_is_one_package() {
        let facts = claim_with(&machine(), stub);
        let declared = facts
            .iter()
            .filter(|f| matches!(f, Fact::Package { .. }))
            .count();
        assert_eq!(declared, 1, "two files, one installer");
        let owns = facts
            .iter()
            .filter(|f| matches!(f, Fact::Owns { .. }))
            .count();
        assert_eq!(owns, 2);
    }

    #[test]
    fn a_path_no_receipt_claims_stays_unclaimed() {
        let facts = claim_with(&machine(), stub);
        let graph = Graph::from_facts(facts);
        assert!(
            graph.owners_of(Path::new("/opt/mystery/thing")).is_empty(),
            "honestly unclaimed is better than wrongly claimed"
        );
    }

    #[test]
    fn something_already_owned_is_not_asked_about() {
        let owned = Graph::from_facts([Fact::Owns {
            package: PackageId::new("homebrew", "ripgrep"),
            artifact: PathBuf::from("/usr/local/texlive/bin/tex"),
        }]);
        assert_eq!(claim_with(&owned, explode_owned), Vec::new());
    }

    #[test]
    fn nothing_in_the_home_directory_is_asked_about() {
        let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
            return;
        };
        let at_home = Graph::from_facts([Fact::Artifact {
            path: home.join(".local/bin/something"),
        }]);
        assert_eq!(claim_with(&at_home, explode_home), Vec::new());
    }
}
