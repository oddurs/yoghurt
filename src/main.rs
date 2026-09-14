//! `yoghurt` — see what is installed on this machine.

use std::fmt::Write as _;
use std::io::{self, Write as _};
use std::process::ExitCode;

use yoghurt::{SourceSummary, abbreviate, survey};

const VERSION: &str = env!("CARGO_PKG_VERSION");

const USAGE: &str = "\
yoghurt — see what is installed on this machine

Usage:
  yoghurt            Summarise every package source it can find
  yoghurt --help
  yoghurt --version
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let text = match resolve(&args) {
        Ok(text) => text,
        Err(message) => {
            eprintln!("yoghurt: {message}\n\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };

    // A closed pipe (`yoghurt | head`) is not an error worth reporting.
    let _ = writeln!(io::stdout().lock(), "{text}");
    ExitCode::SUCCESS
}

/// Turn arguments into the text to print.
///
/// # Errors
///
/// Returns a human-readable message when the arguments are not understood.
fn resolve(args: &[String]) -> Result<String, String> {
    match args {
        [] => Ok(report(&survey())),
        [one] if one == "-h" || one == "--help" => Ok(USAGE.trim_end().to_owned()),
        [one] if one == "-V" || one == "--version" => Ok(format!("yoghurt {VERSION}")),
        [one] => Err(format!("unrecognised argument `{one}`")),
        _ => Err(format!("expected no arguments, got {}", args.len())),
    }
}

/// The survey as a table: what was found, how much of it, and where from.
///
/// Counts are right-aligned in a fixed column so the eye can run down them
/// rather than hunt along each row.
fn report(sources: &[SourceSummary]) -> String {
    if sources.is_empty() {
        return "yoghurt found no package sources on this machine.".to_owned();
    }

    // Bound rather than inlined: a width specifier needs a named argument.
    let (source_head, items_head, total_head) = ("SOURCE", "ITEMS", "total");
    let name = sources
        .iter()
        .map(|s| s.name.len())
        .max()
        .unwrap_or(0)
        .max(total_head.len());
    let total: usize = sources.iter().map(|s| s.count).sum();
    let num = total.to_string().len().max(items_head.len());

    let mut out = String::new();
    let _ = writeln!(out, "  {source_head:<name$}  {items_head:>num$}  WHERE");
    for found in sources {
        let (label, count, root) = (found.name, found.count, abbreviate(&found.root));
        let _ = writeln!(out, "  {label:<name$}  {count:>num$}  {root}");
    }
    let _ = writeln!(out, "  {total_head:<name$}  {total:>num$}");
    out
}

#[cfg(test)]
mod tests {
    use super::{VERSION, report, resolve};
    use std::path::PathBuf;
    use yoghurt::SourceSummary;

    fn args(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_owned()).collect()
    }

    fn source(name: &'static str, count: usize) -> SourceSummary {
        SourceSummary {
            name,
            root: PathBuf::from("/opt/homebrew/Cellar"),
            count,
        }
    }

    #[test]
    fn reports_the_version() {
        assert_eq!(
            resolve(&args(&["--version"])).unwrap(),
            format!("yoghurt {VERSION}")
        );
    }

    #[test]
    fn prints_usage_for_help() {
        assert!(
            resolve(&args(&["--help"]))
                .unwrap()
                .starts_with("yoghurt —")
        );
    }

    #[test]
    fn rejects_unknown_arguments() {
        assert!(resolve(&args(&["--nope"])).is_err());
        assert!(resolve(&args(&["a", "b"])).is_err());
    }

    #[test]
    fn a_bare_invocation_surveys_the_machine() {
        assert!(resolve(&args(&[])).is_ok());
    }

    #[test]
    fn the_table_totals_every_source() {
        let table = report(&[source("homebrew", 169), source("cargo", 18)]);
        assert!(table.contains("homebrew"), "{table}");
        assert!(table.contains("187"), "expected the total 187 in:\n{table}");
    }

    #[test]
    fn an_empty_survey_says_so_rather_than_printing_a_bare_header() {
        assert!(report(&[]).contains("no package sources"));
    }
}
