//! `yoghurt` — see what is installed on this machine, and where it came from.

use std::fmt::Write as _;
use std::io::{self, IsTerminal as _, Write as _};
use std::process::ExitCode;

use yoghurt::fact::Source as _;
use yoghurt::{Graph, Homebrew, Provenance, Walk};

const VERSION: &str = env!("CARGO_PKG_VERSION");

const USAGE: &str = "\
yoghurt — see what is installed on this machine, and where it came from

Usage:
  yoghurt            Survey the machine
  yoghurt --plain    Print the table even when attached to a terminal
  yoghurt --help
  yoghurt --version

Columns:
  NAME  SOURCE  VERSION  ORIGIN  SIZE  PATH

  ORIGIN is why it is here: `wanted` if you asked for it, `pulled-in` if it
  came with something else, `unexplained` if nothing you installed needs it,
  and `orphan` if no package manager claims it at all.
";

/// Prefixes macOS itself owns.
///
/// Nothing claims `/usr/bin/awk`, which makes it an orphan by the letter of the
/// model and noise by any useful measure — there are over a thousand of them
/// against thirty-five real ones. The honest fix is a system source that claims
/// these the way Homebrew claims the Cellar; until then they are filtered here.
const SYSTEM_PREFIXES: &[&str] = &[
    "/usr/bin",
    "/usr/sbin",
    "/usr/libexec",
    "/bin",
    "/sbin",
    "/System",
    "/Library",
];

/// What the arguments asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    /// Read the machine and print it.
    Survey,
    /// Explain the arguments.
    Help,
    /// Say which version this is.
    Version,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let text = match parse(&args).and_then(run) {
        Ok(text) => text,
        Err(message) => {
            eprintln!("yoghurt: {message}\n\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };

    // A closed pipe (`yoghurt | head`) is the reader's decision, not an error.
    let _ = write!(io::stdout().lock(), "{text}");
    ExitCode::SUCCESS
}

/// Read the arguments, touching nothing.
///
/// Separate from [`run`] so the argument surface can be tested without reading
/// the machine the tests happen to be running on.
///
/// # Errors
///
/// Returns a human-readable message when the arguments are not understood.
fn parse(args: &[String]) -> Result<Action, String> {
    match args {
        [] => Ok(Action::Survey),
        [one] if one == "--plain" => Ok(Action::Survey),
        [one] if one == "-h" || one == "--help" => Ok(Action::Help),
        [one] if one == "-V" || one == "--version" => Ok(Action::Version),
        [one] => Err(format!("unrecognised argument `{one}`")),
        _ => Err(format!("expected at most one argument, got {}", args.len())),
    }
}

/// Do it.
///
/// # Errors
///
/// Returns a human-readable message when a source could not be read.
fn run(action: Action) -> Result<String, String> {
    match action {
        Action::Help => Ok(USAGE.to_owned()),
        Action::Version => Ok(format!("yoghurt {VERSION}\n")),
        Action::Survey => {
            // Until the interface exists in 0017, both paths lead to the table.
            let _tty = io::stdout().is_terminal();
            Ok(table(&survey()?))
        }
    }
}

/// Read every source and assemble the machine.
fn survey() -> Result<Graph, String> {
    let walk = Walk::from_environment();
    let mut facts = walk.scan().map_err(|e| e.to_string())?;
    if let Some(brew) = Homebrew::from_environment() {
        facts.extend(brew.scan().map_err(|e| e.to_string())?);
    }
    Ok(Graph::from_facts(facts))
}

/// One row per package, then anything on disk nobody claims.
///
/// Tab-separated and free of escapes, so `yoghurt | awk` works.
fn table(graph: &Graph) -> String {
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
        // The keg that matches the version shown, not whichever sorts first:
        // a formula with two kegs would otherwise report one version beside
        // the other one's path.
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

/// Whether macOS itself put this here.
fn is_system(path: &std::path::Path) -> bool {
    SYSTEM_PREFIXES
        .iter()
        .any(|prefix| path.starts_with(prefix))
}

const UNITS: [&str; 5] = ["B", "K", "M", "G", "T"];

/// Bytes, at the precision a person reads rather than the one a computer holds.
///
/// The cast loses precision above 2^53 bytes, which is eight petabytes in a
/// keg. It is not a real machine.
#[allow(clippy::cast_precision_loss)]
fn size(bytes: Option<u64>) -> String {
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

#[cfg(test)]
mod tests {
    use super::{Action, VERSION, is_system, parse, run, size, table};
    use std::path::{Path, PathBuf};
    use yoghurt::Graph;
    use yoghurt::fact::{Fact, PackageId};

    fn args(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_owned()).collect()
    }

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
    fn reports_the_version() {
        assert_eq!(
            run(Action::Version).unwrap(),
            format!("yoghurt {VERSION}\n")
        );
    }

    #[test]
    fn prints_usage_for_help() {
        assert!(run(Action::Help).unwrap().starts_with("yoghurt \u{2014}"));
    }

    #[test]
    fn rejects_unknown_arguments() {
        assert!(parse(&args(&["--nope"])).is_err());
        assert!(parse(&args(&["a", "b"])).is_err());
    }

    #[test]
    fn bare_and_plain_both_survey() {
        assert_eq!(parse(&args(&[])).unwrap(), Action::Survey);
        assert_eq!(parse(&args(&["--plain"])).unwrap(), Action::Survey);
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
