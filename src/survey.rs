//! Reading the machine.
//!
//! Every source, laid against the walk that says what is actually there. Lives
//! here rather than in `main` because the interface asks for another one every
//! time somebody presses `r`, and a survey that only the entry point can run is
//! a survey you can only have once.

use crate::config::Config;
use crate::model::fact::Source as _;
use crate::source::taxonomy::{self, Subject};
use crate::{Applications, Cargo, Gem, Go, Graph, Homebrew, Node, PythonTools, Rustup, Walk};

/// Read every source and assemble the machine.
///
/// # Errors
///
/// Returns a message when a source exists but could not be read.
pub fn survey() -> Result<Graph, String> {
    read(false)
}

/// Read every source, and ask each what is newer.
///
/// Separate because this is the only path that touches the network, and it runs
/// only when somebody asked for it.
///
/// # Errors
///
/// Returns a message when a source exists but could not be read. A source that
/// cannot be *checked* is not an error: its packages keep reading as unchecked.
pub fn survey_checking_updates() -> Result<Graph, String> {
    read(true)
}

fn read(check_updates: bool) -> Result<Graph, String> {
    // The walk is ground truth and runs first; the adapters lay their claims
    // against it. A source that is not installed contributes nothing, which is
    // not a failure.
    let walk = Walk::from_environment();
    let mut facts = walk.scan().map_err(|e| e.to_string())?;

    let sources: Vec<Box<dyn crate::model::fact::Source>> = [
        Homebrew::from_environment().map(|s| Box::new(s) as Box<dyn crate::model::fact::Source>),
        Cargo::from_environment().map(|s| Box::new(s) as Box<dyn crate::model::fact::Source>),
        Rustup::from_environment().map(|s| Box::new(s) as Box<dyn crate::model::fact::Source>),
        Some(Box::new(Applications::from_environment()) as Box<dyn crate::model::fact::Source>),
        Some(Box::new(Node::from_environment()) as Box<dyn crate::model::fact::Source>),
        Go::from_environment().map(|s| Box::new(s) as Box<dyn crate::model::fact::Source>),
        Some(Box::new(Gem::from_environment()) as Box<dyn crate::model::fact::Source>),
        Some(Box::new(PythonTools::from_environment()) as Box<dyn crate::model::fact::Source>),
    ]
    .into_iter()
    .flatten()
    .collect();

    for source in &sources {
        facts.extend(source.scan().map_err(|e| e.to_string())?);
    }
    if check_updates {
        for source in &sources {
            // Being unable to check is never a reason to change what is already
            // known, so a failure here is dropped rather than propagated.
            if let Ok(newer) = source.updates() {
                facts.extend(newer);
            }
        }
    }
    let graph = Graph::from_facts(facts.clone());

    // Only if somebody switched it on. Nothing above this line touches the
    // network, and this is the only thing that ever would.
    let config = Config::load()?;
    if !config.taxonomy.available() {
        return Ok(graph);
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
            Ok(Graph::from_facts(facts))
        }
        // A classification that fails is a missing label, never a missing
        // machine. Fall back to what was observed.
        Err(error) => {
            eprintln!("yoghurt: {error}");
            Ok(graph)
        }
    }
}
