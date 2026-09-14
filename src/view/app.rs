//! What the interface is looking at.
//!
//! State only: what the machine is, where the cursor is, and what the person
//! has asked to see. No drawing and no terminal, so every question the
//! interface can answer is answerable in a test without one.

use crate::model::graph::Graph;

/// The interface's whole state.
pub struct App {
    /// The machine.
    pub graph: Graph,
    /// The hostname, for the header.
    pub host: String,
    /// How long ago the scan finished, in seconds.
    pub scanned_ago: u64,
    /// Whether a scan is still running.
    pub scanning: bool,
    /// Set when the person has asked to leave.
    pub quit: bool,
}

impl App {
    /// Look at this machine.
    #[must_use]
    pub fn new(graph: Graph) -> Self {
        Self {
            graph,
            host: hostname(),
            scanned_ago: 0,
            scanning: false,
            quit: false,
        }
    }

    /// Totals for the header: packages, sources, bytes.
    #[must_use]
    pub fn totals(&self) -> Totals {
        let packages = self.graph.packages().count();
        let mut sources: Vec<&str> = self
            .graph
            .packages()
            .map(|(id, _)| id.source.as_str())
            .collect();
        sources.sort_unstable();
        sources.dedup();
        let bytes = self.graph.artifacts().filter_map(|(_, a)| a.bytes).sum();
        Totals {
            packages,
            sources: sources.len(),
            bytes,
        }
    }

    /// The counts the status strip shows, in the order it shows them.
    #[must_use]
    pub fn facets(&self) -> Vec<(&'static str, usize)> {
        use crate::model::question::Provenance;
        let (mut wanted, mut pulled, mut unexplained) = (0, 0, 0);
        for (id, _) in self.graph.packages() {
            match self.graph.why(id) {
                Provenance::Wanted => wanted += 1,
                Provenance::PulledIn(_) => pulled += 1,
                Provenance::Unexplained => unexplained += 1,
            }
        }
        let outdated = self.graph.packages().filter(|(_, p)| p.outdated).count();
        let broken = self
            .graph
            .artifacts()
            .filter(|(path, _)| self.graph.is_broken(path))
            .count();

        vec![
            ("wanted", wanted),
            ("pulled in", pulled),
            ("outdated", outdated),
            ("unexplained", unexplained),
            ("broken", broken),
        ]
    }

    /// Freshness, in the words a person uses.
    #[must_use]
    pub fn freshness(&self) -> String {
        if self.scanning {
            return "scanning".to_owned();
        }
        match self.scanned_ago {
            0..=5 => "scanned just now".to_owned(),
            s @ 6..=89 => format!("scanned {s}s ago"),
            s @ 90..=5399 => format!("scanned {}m ago", s / 60),
            s => format!("scanned {}h ago", s / 3600),
        }
    }
}

/// What the header counts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Totals {
    /// Packages every source knows about.
    pub packages: usize,
    /// How many sources reported anything.
    pub sources: usize,
    /// Bytes on disk, where anybody measured them.
    pub bytes: u64,
}

/// This machine's name, or a usable stand-in.
fn hostname() -> String {
    std::process::Command::new("hostname")
        .arg("-s")
        .output()
        .ok()
        .filter(|out| out.status.success())
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|name| name.trim().to_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "this machine".to_owned())
}

#[cfg(test)]
mod tests {
    use super::{App, Totals};
    use crate::model::fact::{Fact, PackageId};
    use crate::model::graph::Graph;
    use std::path::PathBuf;

    fn machine() -> App {
        let rg = PackageId::new("homebrew", "ripgrep");
        let pcre = PackageId::new("homebrew", "pcre2");
        App::new(Graph::from_facts([
            Fact::Package {
                id: rg.clone(),
                version: Some("15.2.0".to_owned()),
            },
            Fact::Wanted {
                package: rg.clone(),
            },
            Fact::Owns {
                package: rg.clone(),
                artifact: PathBuf::from("/opt/homebrew/Cellar/ripgrep/15.2.0"),
            },
            Fact::Size {
                artifact: PathBuf::from("/opt/homebrew/Cellar/ripgrep/15.2.0"),
                bytes: 6_500_000,
            },
            Fact::DependsOn {
                package: rg,
                on: pcre.clone(),
                declared_directly: true,
            },
            Fact::Package {
                id: pcre.clone(),
                version: Some("10.48".to_owned()),
            },
            Fact::Outdated {
                package: pcre,
                latest: Some("10.49".to_owned()),
            },
            Fact::Missing {
                artifact: PathBuf::from("/opt/homebrew/bin/ghost"),
            },
        ]))
    }

    #[test]
    fn totals_count_packages_sources_and_bytes() {
        assert_eq!(
            machine().totals(),
            Totals {
                packages: 2,
                sources: 1,
                bytes: 6_500_000
            }
        );
    }

    #[test]
    fn the_facets_are_the_questions_worth_asking() {
        assert_eq!(
            machine().facets(),
            vec![
                ("wanted", 1),
                ("pulled in", 1),
                ("outdated", 1),
                ("unexplained", 0),
                ("broken", 1),
            ]
        );
    }

    #[test]
    fn freshness_is_said_the_way_a_person_says_it() {
        let mut app = machine();
        for (seconds, expected) in [
            (0, "scanned just now"),
            (30, "scanned 30s ago"),
            (120, "scanned 2m ago"),
            (7200, "scanned 2h ago"),
        ] {
            app.scanned_ago = seconds;
            assert_eq!(app.freshness(), expected);
        }
    }

    #[test]
    fn a_scan_in_flight_says_so_rather_than_reporting_a_stale_time() {
        let mut app = machine();
        app.scanned_ago = 600;
        app.scanning = true;
        assert_eq!(app.freshness(), "scanning");
    }
}
