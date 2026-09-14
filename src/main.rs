//! `yoghurt` — see what is installed on this machine, and where it came from.
//!
//! Argument parsing and the entry point. Everything else lives in the library:
//! [`yoghurt::source`] reads the machine, [`yoghurt::model`] holds it, and
//! [`yoghurt::view`] shows it.

use std::io::{self, IsTerminal as _, Write as _};
use std::process::ExitCode;

use yoghurt::model::fact::Source as _;
use yoghurt::view::{app::App, plain, run};
use yoghurt::{Graph, Homebrew, Walk};

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

/// What the arguments asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    /// Read the machine and print it as a table.
    Survey,
    /// Read the machine and open the interface.
    Interface,
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
        // A terminal gets the interface; a pipe gets the table. `--plain`
        // forces the table either way.
        [] if io::stdout().is_terminal() => Ok(Action::Interface),
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
        Action::Survey => Ok(plain::table(&survey()?)),
        Action::Interface => {
            run::run(App::new(survey()?)).map_err(|e| e.to_string())?;
            Ok(String::new())
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

#[cfg(test)]
mod tests {
    use super::{Action, VERSION, parse, run};

    fn args(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_owned()).collect()
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
    fn plain_always_prints_the_table() {
        assert_eq!(parse(&args(&["--plain"])).unwrap(), Action::Survey);
    }

    #[test]
    fn a_bare_invocation_follows_the_terminal() {
        // The test harness has no tty, so this is the pipe case.
        assert_eq!(parse(&args(&[])).unwrap(), Action::Survey);
    }
}
