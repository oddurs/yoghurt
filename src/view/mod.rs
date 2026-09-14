//! How the machine is shown.
//!
//! Views project the graph and hold no rules of their own. If something here
//! has to decide what a package *is*, it belongs in [`crate::model`] instead.

pub mod app;
pub mod plain;
pub mod row;
#[path = "loop_.rs"]
pub mod run;
pub mod term;
pub mod testkit;
pub mod ui;
