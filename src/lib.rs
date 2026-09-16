//! See what is installed on this machine, and where it came from.
//!
//! The model is one graph over three kinds of node — a package a manager
//! records, an artifact on disk, and a command resolvable on `PATH` — and every
//! state the interface shows is a query over it rather than a feature somebody
//! remembered to build.
//!
//! Three layers, in the order data moves through them:
//!
//! - [`source`] — adapters, each emitting facts and knowing nothing else
//! - [`model`] — the graph, and the questions that can be asked of it
//! - [`view`] — projections of the graph, holding no rules of their own

pub mod config;
pub mod model;
pub mod source;
pub mod survey;
pub mod view;

pub use config::Config;
pub use model::fact::{Fact, PackageId, ScanError, Source};
pub use model::graph::Graph;
pub use model::question::{Provenance, Resolution};
pub use source::cargo::{Cargo, Rustup};
pub use source::homebrew::Homebrew;
pub use source::macos::Applications;
pub use source::node::Node;
pub use source::tools::{Gem, Go, PythonTools};
pub use source::walk::Walk;
