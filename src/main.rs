//! `yoghurt` — see what is installed on this machine, and where it came from.
//!
//! Argument parsing and the entry point. Everything else lives in the library:
//! [`yoghurt::source`] reads the machine, [`yoghurt::model`] holds it, and
//! [`yoghurt::view`] shows it.

use std::io::{self, IsTerminal as _, Write as _};
use std::process::ExitCode;

use yoghurt::survey::survey;
use yoghurt::view::row::{Axis, Facet, Sort};
use yoghurt::view::{app::App, plain, run, testkit};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The commit this was built from, stamped by `build.rs`.
///
/// Empty when built outside a git checkout.
const COMMIT: &str = env!("YOGHURT_COMMIT");

const USAGE: &str = "\
yoghurt — see what is installed on this machine, and where it came from

Usage:
  yoghurt                    Open the interface, or print the table when piped
  yoghurt --plain            Print the table even when attached to a terminal
  yoghurt --screenshot WxH   Render one frame to stdout and exit
  yoghurt --help
  yoghurt --version

Choosing a view (with --screenshot):
  --group AXIS     source, role, category, size, age, health
  --sort COLUMN    name, size, age, state, version
  --facet NAME     wanted, pulled in, outdated, unexplained, broken
  --find TEXT      narrow to what matches
  --detail         open the detail pane on the first row

Columns:
  NAME  SOURCE  VERSION  ORIGIN  SIZE  PATH

  ORIGIN is why it is here: `wanted` if you asked for it, `pulled-in` if it
  came with something else, `unexplained` if nothing you installed needs it,
  and `orphan` if no package manager claims it at all.
";

/// What the arguments asked for.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Action {
    /// Read the machine and print it as a table.
    Survey,
    /// Read the machine and open the interface.
    Interface,
    /// Render one frame and exit.
    Screenshot(Shot),
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
/// Which view to render, and at what size.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Shot {
    width: u16,
    height: u16,
    group: Option<Axis>,
    sort: Option<Sort>,
    facet: Option<Facet>,
    find: Option<String>,
    detail: bool,
}

/// `96x30` into a size.
fn size_of(text: &str) -> Result<(u16, u16), String> {
    let (w, h) = text
        .split_once(['x', 'X'])
        .ok_or_else(|| format!("expected a size like 96x30, got `{text}`"))?;
    let parse = |n: &str, what: &str| {
        n.parse::<u16>()
            .ok()
            .filter(|v| *v > 0)
            .ok_or_else(|| format!("{what} must be a positive number, got `{n}`"))
    };
    Ok((parse(w, "width")?, parse(h, "height")?))
}

fn parse(args: &[String]) -> Result<Action, String> {
    match args {
        [] if io::stdout().is_terminal() => return Ok(Action::Interface),
        [] => return Ok(Action::Survey),
        [one] if one == "--plain" => return Ok(Action::Survey),
        [one] if one == "-h" || one == "--help" => return Ok(Action::Help),
        [one] if one == "-V" || one == "--version" => return Ok(Action::Version),
        _ => {}
    }

    // Anything longer is a screenshot and its view. Parsed as a loop rather
    // than a slice pattern, because the options may arrive in any order.
    let mut shot = Shot::default();
    let mut seen_size = false;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        let mut value = |what: &str| {
            rest.next()
                .cloned()
                .ok_or_else(|| format!("{what} needs a value"))
        };
        match arg.as_str() {
            "--screenshot" => {
                let (w, h) = size_of(&value("--screenshot")?)?;
                shot.width = w;
                shot.height = h;
                seen_size = true;
            }
            "--group" => {
                let name = value("--group")?;
                shot.group = Some(
                    Axis::from_label(&name).ok_or_else(|| format!("no such grouping `{name}`"))?,
                );
            }
            "--sort" => {
                let name = value("--sort")?;
                shot.sort = Some(
                    Sort::from_label(&name).ok_or_else(|| format!("no such column `{name}`"))?,
                );
            }
            "--facet" => {
                let name = value("--facet")?;
                shot.facet = Some(
                    Facet::from_label(&name).ok_or_else(|| format!("no such facet `{name}`"))?,
                );
            }
            "--find" => shot.find = Some(value("--find")?),
            "--detail" => shot.detail = true,
            other => return Err(format!("unrecognised argument `{other}`")),
        }
    }

    if !seen_size {
        return Err("--group, --sort, --facet and --find need --screenshot".to_owned());
    }
    Ok(Action::Screenshot(shot))
}

/// Do it.
///
/// # Errors
///
/// Returns a human-readable message when a source could not be read.
fn run(action: Action) -> Result<String, String> {
    match action {
        Action::Help => Ok(USAGE.to_owned()),
        Action::Version => Ok(version()),
        Action::Survey => Ok(plain::table(&survey()?)),
        Action::Interface => {
            run::run(App::new(survey()?)).map_err(|e| e.to_string())?;
            Ok(String::new())
        }
        Action::Screenshot(shot) => Ok(screenshot(&shot)?),
    }
}

/// One frame of the interface, as text.
///
/// The harness this uses is the one the tests use, so a screenshot is exactly
/// what the interface draws rather than an approximation of it. No terminal is
/// involved, so this works over a pipe and in CI.
fn screenshot(shot: &Shot) -> Result<String, String> {
    let mut app = App::new(survey()?);
    if let Some(axis) = shot.group {
        app.axis = axis;
    }
    if let Some(sort) = shot.sort {
        app.sort = sort;
    }
    app.filter.facet = shot.facet;
    if let Some(find) = &shot.find {
        app.filter.query.clone_from(find);
    }
    app.rebuild();
    if shot.detail {
        // The first row is a heading, so step onto what it contains.
        app.move_by(1);
        app.toggle_detail();
    }
    app.scroll_into_view(usize::from(shot.height).saturating_sub(4));
    Ok(testkit::render(&mut app, shot.width, shot.height).join("\n") + "\n")
}

/// The version, and the commit it came from when there is one.
fn version() -> String {
    if COMMIT.is_empty() {
        format!("yoghurt {VERSION}\n")
    } else {
        format!("yoghurt {VERSION} ({COMMIT})\n")
    }
}

#[cfg(test)]
mod tests {
    use super::{Action, VERSION, parse, run};
    use yoghurt::view::row::{Axis, Facet, Sort};

    fn args(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn reports_the_version_and_the_commit_it_was_built_from() {
        let reported = run(Action::Version).unwrap();
        assert!(
            reported.starts_with(&format!("yoghurt {VERSION}")),
            "{reported}"
        );
        if !super::COMMIT.is_empty() {
            assert!(
                reported.contains(super::COMMIT),
                "a binary must be able to say which commit it is: {reported}"
            );
        }
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
    fn a_screenshot_carries_the_size_and_the_view() {
        let parsed = parse(&args(&[
            "--screenshot",
            "96x30",
            "--group",
            "role",
            "--sort",
            "size",
            "--facet",
            "outdated",
            "--find",
            "py",
        ]))
        .unwrap();
        let Action::Screenshot(shot) = parsed else {
            panic!("expected a screenshot")
        };
        assert_eq!((shot.width, shot.height), (96, 30));
        assert_eq!(shot.group, Some(Axis::Role));
        assert_eq!(shot.sort, Some(Sort::Size));
        assert_eq!(shot.facet, Some(Facet::Outdated));
        assert_eq!(shot.find.as_deref(), Some("py"));
    }

    #[test]
    fn the_options_may_arrive_in_any_order() {
        let a = parse(&args(&["--group", "size", "--screenshot", "80x24"])).unwrap();
        let b = parse(&args(&["--screenshot", "80x24", "--group", "size"])).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn a_bad_size_says_what_it_wanted() {
        for bad in ["96", "96by30", "0x30", "axb"] {
            let error = parse(&args(&["--screenshot", bad])).unwrap_err();
            assert!(!error.is_empty(), "{bad}");
        }
    }

    #[test]
    fn an_unknown_view_name_is_rejected_rather_than_ignored() {
        assert!(parse(&args(&["--screenshot", "80x24", "--group", "colour"])).is_err());
        assert!(parse(&args(&["--screenshot", "80x24", "--sort", "vibes"])).is_err());
        assert!(parse(&args(&["--screenshot", "80x24", "--facet", "lovely"])).is_err());
    }

    #[test]
    fn a_view_option_without_a_screenshot_says_so() {
        let error = parse(&args(&["--group", "role"])).unwrap_err();
        assert!(error.contains("--screenshot"), "{error}");
    }

    #[test]
    fn an_option_missing_its_value_says_which_one() {
        let error = parse(&args(&["--screenshot", "80x24", "--find"])).unwrap_err();
        assert!(error.contains("--find"), "{error}");
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
