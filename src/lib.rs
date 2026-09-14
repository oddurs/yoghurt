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
pub mod graph;
pub mod homebrew;
pub mod question;
pub mod walk;

pub use fact::{Fact, PackageId, ScanError, Source};
pub use graph::Graph;
pub use homebrew::Homebrew;
pub use question::{Provenance, Resolution};
pub use walk::Walk;
