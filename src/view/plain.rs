//! The machine as a table.
//!
//! One row per package, then everything on disk nobody claims. Tab-separated
//! and free of escapes, so `yoghurt | awk` works.

use std::fmt::Write as _;

use crate::model::graph::Graph;
use crate::model::question::{Provenance, is_system};

const UNITS: [&str; 5] = ["B", "K", "M", "G", "T"];

/// One row per package, then anything on disk nobody claims.
#[must_use]
pub fn table(graph: &Graph) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "NAME\tSOURCE\tVERSION\tORIGIN\tSIZE\tPATH");

    for (id, package) in graph.packages() {
        let origin = match graph.why(id) {
            Provenance::Wanted => "wanted",
            Provenance::PulledIn(_) => "pulled-in",
            Provenance::Unexplained => "unexplained",
        };
        let bytes: u64 = package
            .owns
            .iter()
            .filter_map(|path| graph.artifact(path).and_then(|a| a.bytes))
            .sum();
        // The keg that matches the version shown, not whichever sorts first: a
        // formula with two kegs would otherwise report one version beside the
        // other one's path.
        let home = package
            .version
            .as_deref()
            .and_then(|version| {
                package
                    .owns
                    .iter()
                    .find(|path| path.file_name().and_then(|n| n.to_str()) == Some(version))
            })
            .or_else(|| package.owns.iter().next())
            .map(|path| path.display().to_string());

        let _ = writeln!(
            out,
            "{}\t{}\t{}\t{}\t{}\t{}",
            id.name,
            id.source,
            package.version.as_deref().unwrap_or("-"),
            if package.outdated {
                format!("{origin},outdated")
            } else {
                origin.to_owned()
            },
            size(Some(bytes)),
            home.as_deref().unwrap_or("-"),
        );
    }

    for (path, artifact) in graph.artifacts() {
        if !graph.is_orphan(path) || is_system(path) {
            continue;
        }
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("-");
        let origin = if artifact.missing {
            "orphan,broken"
        } else {
            "orphan"
        };
        let _ = writeln!(
            out,
            "{name}\t-\t-\t{origin}\t{}\t{}",
            size(artifact.bytes),
            path.display()
        );
    }
    out
}

/// Bytes, at the precision a person reads rather than the one a computer holds.
///
/// The cast loses precision above 2^53 bytes, which is eight petabytes in a
/// keg. It is not a real machine.
#[allow(clippy::cast_precision_loss)]
pub(crate) fn size(bytes: Option<u64>) -> String {
    let Some(bytes) = bytes.filter(|b| *b > 0) else {
        return "-".to_owned();
    };
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes}B")
    } else if value < 10.0 {
        format!("{value:.1}{}", UNITS[unit])
    } else {
        format!("{value:.0}{}", UNITS[unit])
    }
}

/// Bytes for a header, where zero is a number rather than a dash.
#[must_use]
pub fn human(bytes: u64) -> String {
    if bytes == 0 {
        "0B".to_owned()
    } else {
        size(Some(bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::{is_system, size, table};
    use crate::model::fact::{Fact, PackageId};
    use crate::model::graph::Graph;
    use std::path::{Path, PathBuf};

    fn machine() -> Graph {
        Graph::from_facts([
            Fact::Package {
                id: PackageId::new("homebrew", "ripgrep"),
                version: Some("15.2.0".to_owned()),
            },
            Fact::Wanted {
                package: PackageId::new("homebrew", "ripgrep"),
            },
            Fact::Owns {
                package: PackageId::new("homebrew", "ripgrep"),
                artifact: PathBuf::from("/opt/homebrew/Cellar/ripgrep/15.2.0"),
            },
            Fact::Size {
                artifact: PathBuf::from("/opt/homebrew/Cellar/ripgrep/15.2.0"),
                bytes: 6_500_000,
            },
            Fact::Artifact {
                path: PathBuf::from("/Applications/Xcode.app"),
            },
            Fact::Artifact {
                path: PathBuf::from("/usr/bin/awk"),
            },
        ])
    }

    #[test]
    fn a_package_row_says_why_it_is_here() {
        let rendered = table(&machine());
        let row = rendered
            .lines()
            .find(|l| l.starts_with("ripgrep\t"))
            .expect("a row");
        assert_eq!(
            row,
            "ripgrep\thomebrew\t15.2.0\twanted\t6.2M\t/opt/homebrew/Cellar/ripgrep/15.2.0"
        );
    }

    #[test]
    fn a_multi_keg_package_shows_the_path_of_the_version_it_names() {
        let id = PackageId::new("homebrew", "fmt");
        let graph = Graph::from_facts([
            Fact::Package {
                id: id.clone(),
                version: Some("12.2.0".to_owned()),
            },
            Fact::Owns {
                package: id.clone(),
                artifact: PathBuf::from("/opt/homebrew/Cellar/fmt/12.1.0"),
            },
            Fact::Owns {
                package: id,
                artifact: PathBuf::from("/opt/homebrew/Cellar/fmt/12.2.0"),
            },
        ]);
        let rendered = table(&graph);
        let row = rendered
            .lines()
            .find(|l| l.starts_with("fmt\t"))
            .expect("a row");
        assert!(row.ends_with("/opt/homebrew/Cellar/fmt/12.2.0"), "{row}");
    }

    #[test]
    fn an_application_nobody_claims_is_listed_as_an_orphan() {
        let rendered = table(&machine());
        assert!(
            rendered
                .lines()
                .any(|l| l.starts_with("Xcode.app\t-\t-\torphan\t")),
            "{rendered}"
        );
    }

    #[test]
    fn system_binaries_are_not_reported_as_orphans() {
        let rendered = table(&machine());
        assert!(
            !rendered.contains("/usr/bin/awk"),
            "there are a thousand of these: {rendered}"
        );
    }

    #[test]
    fn the_table_carries_no_escape_sequences() {
        assert!(!table(&machine()).contains('\u{1b}'));
    }

    #[test]
    fn every_row_has_the_same_number_of_columns_as_the_header() {
        let rendered = table(&machine());
        let mut lines = rendered.lines();
        let columns = lines.next().expect("header").split('\t').count();
        for line in lines {
            assert_eq!(line.split('\t').count(), columns, "{line}");
        }
    }

    #[test]
    fn system_prefixes_are_recognised_and_others_are_not() {
        assert!(is_system(Path::new("/usr/bin/awk")));
        assert!(is_system(Path::new(
            "/System/Library/CoreServices/Finder.app"
        )));
        assert!(!is_system(Path::new("/Applications/Xcode.app")));
        assert!(!is_system(Path::new("/opt/homebrew/bin/rg")));
    }

    #[test]
    fn sizes_read_the_way_a_person_reads_them() {
        assert_eq!(size(None), "-");
        assert_eq!(size(Some(0)), "-");
        assert_eq!(size(Some(512)), "512B");
        assert_eq!(size(Some(6_500_000)), "6.2M");
        assert_eq!(size(Some(1_617_282_179)), "1.5G");
    }
}
