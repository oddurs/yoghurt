//! How the machine is shown.
//!
//! Views project the graph and hold no rules of their own. If something here
//! has to decide what a package *is*, it belongs in [`crate::model`] instead.

pub mod plain;
pub mod term;
