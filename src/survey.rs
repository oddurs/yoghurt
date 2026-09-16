//! Reading the machine.
//!
//! Every source, laid against the walk that says what is actually there.
//!
//! Sources run at once. Eight of them in sequence is the sum of eight waits —
//! and two of those are subprocesses that spend their time blocked rather than
//! working. Concurrently it is the slowest one, and one slow package manager
//! stops holding up the other seven.
//!
//! A source that fails does not fail the scan. It is named, its error is kept,
//! and everything else still arrives, because seven eighths of a machine
//! reported honestly is worth far more than nothing reported at all.

use std::thread;
use std::time::SystemTime;

use crate::config::Config;
use crate::model::fact::{ScanError, Source};
use crate::source::taxonomy::{self, Subject};
use crate::{Applications, Cargo, Gem, Go, Graph, Homebrew, Node, PythonTools, Rustup, Walk};

/// What a scan produced, including what it could not.
#[derive(Debug)]
pub struct Survey {
    /// The machine, as far as anybody could tell.
    pub graph: Graph,
    /// Sources that were asked and could not answer.
    pub failures: Vec<ScanError>,
    /// When this was read. Older than now when it came from the cache.
    pub scanned: SystemTime,
}

impl Default for Survey {
    fn default() -> Self {
        Self {
            graph: Graph::default(),
            failures: Vec::new(),
            scanned: SystemTime::now(),
        }
    }
}

impl Survey {
    /// Whether anything went wrong.
    #[must_use]
    pub fn partial(&self) -> bool {
        !self.failures.is_empty()
    }

    /// What to say about it, in one line.
    #[must_use]
    pub fn trouble(&self) -> Option<String> {
        let names: Vec<&str> = self.failures.iter().map(|f| f.source_name).collect();
        match names.as_slice() {
            [] => None,
            [one] => Some(format!("{one} could not be read")),
            many => Some(format!("{} sources could not be read", many.len())),
        }
    }
}

/// The last scan, if there is one, without reading anything.
///
/// This is what the first frame is drawn from. Three seconds is long enough
/// that a tool which spends it before showing anything is a tool people stop
/// opening.
#[must_use]
pub fn cached() -> Option<Survey> {
    let cached = crate::cache::read()?;
    Some(Survey {
        graph: Graph::from_facts(cached.facts),
        failures: Vec::new(),
        scanned: cached.scanned,
    })
}

/// Read every source and assemble the machine.
///
/// # Errors
///
/// Returns a message only when the walk itself fails, because without ground
/// truth there is nothing to lay claims against. Every other failure is carried
/// in [`Survey::failures`] rather than thrown away.
pub fn survey() -> Result<Survey, String> {
    read(false)
}

/// Read every source, and ask each what is newer.
///
/// Separate because this is the only path that touches the network, and it runs
/// only when somebody asked for it.
///
/// # Errors
///
/// As [`survey`].
pub fn survey_checking_updates() -> Result<Survey, String> {
    read(true)
}

/// Every source this machine might have.
fn sources() -> Vec<Box<dyn Source + Send>> {
    let mut found: Vec<Box<dyn Source + Send>> = Vec::new();
    if let Some(brew) = Homebrew::from_environment() {
        found.push(Box::new(brew));
    }
    if let Some(cargo) = Cargo::from_environment() {
        found.push(Box::new(cargo));
    }
    if let Some(rustup) = Rustup::from_environment() {
        found.push(Box::new(rustup));
    }
    if let Some(go) = Go::from_environment() {
        found.push(Box::new(go));
    }
    found.push(Box::new(Applications::from_environment()));
    found.push(Box::new(Node::from_environment()));
    found.push(Box::new(Gem::from_environment()));
    found.push(Box::new(PythonTools::from_environment()));
    found
}

fn read(check_updates: bool) -> Result<Survey, String> {
    // The walk is ground truth. Without it there is nothing for the adapters to
    // lay their claims against, so this one failure is fatal where none of the
    // others are.
    let walk = Walk::from_environment();
    let mut facts = walk.scan().map_err(|e| e.to_string())?;
    let mut failures = Vec::new();

    // Scoped threads, so nothing has to be `'static` and no handle can outlive
    // the scan it belongs to.
    thread::scope(|scope| {
        let running: Vec<_> = sources()
            .into_iter()
            .map(|source| {
                scope.spawn(move || {
                    let mut found = source.scan()?;
                    if check_updates {
                        // Being unable to check what is newer never invalidates
                        // what was read.
                        if let Ok(newer) = source.updates() {
                            found.extend(newer);
                        }
                    }
                    Ok(found)
                })
            })
            .collect();

        for handle in running {
            match handle.join() {
                Ok(Ok(found)) => facts.extend(found),
                Ok(Err(error)) => failures.push(error),
                // A source that panicked took itself down and nothing else.
                Err(_) => failures.push(ScanError::new("unknown", "the source panicked")),
            }
        }
    });

    // Saved before the taxonomy runs, so a machine read without the network is
    // still worth keeping.
    let scanned = SystemTime::now();
    crate::cache::write(&facts, scanned);
    let graph = Graph::from_facts(facts.clone());

    // Only if somebody switched it on. Nothing above this line touches the
    // network, and this is the only thing that ever would.
    let config = Config::load()?;
    if !config.taxonomy.available() {
        return Ok(Survey {
            graph,
            failures,
            scanned,
        });
    }
    let subjects: Vec<Subject> = graph
        .packages()
        .map(|(id, package)| Subject {
            id: id.clone(),
            name: id.name.clone(),
            describes: package.describes.clone(),
        })
        .collect();
    match taxonomy::classify(&config.taxonomy, &subjects) {
        Ok(labels) => {
            facts.extend(labels);
            Ok(Survey {
                graph: Graph::from_facts(facts),
                failures,
                scanned,
            })
        }
        // A classification that fails is a missing label, never a missing
        // machine.
        Err(error) => {
            failures.push(error);
            Ok(Survey {
                graph,
                failures,
                scanned,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Survey;
    use crate::model::fact::ScanError;

    fn failed(names: &[&'static str]) -> Survey {
        Survey {
            graph: crate::Graph::default(),
            failures: names.iter().map(|n| ScanError::new(n, "no")).collect(),
            scanned: std::time::SystemTime::now(),
        }
    }

    #[test]
    fn a_scan_with_nothing_wrong_says_nothing() {
        assert!(!failed(&[]).partial());
        assert_eq!(failed(&[]).trouble(), None);
    }

    #[test]
    fn one_failed_source_is_named() {
        assert_eq!(
            failed(&["homebrew"]).trouble().as_deref(),
            Some("homebrew could not be read")
        );
    }

    #[test]
    fn several_failures_are_counted_rather_than_listed() {
        assert_eq!(
            failed(&["homebrew", "cargo", "gem"]).trouble().as_deref(),
            Some("3 sources could not be read"),
            "a header has no room for a list"
        );
    }

    #[test]
    fn a_partial_scan_is_still_a_scan() {
        let survey = failed(&["cargo"]);
        assert!(survey.partial(), "and it says so");
        // The graph is whatever the other sources managed, not nothing.
        assert_eq!(
            survey.graph.packages().count(),
            0,
            "empty here only because the fixture is"
        );
    }
}
