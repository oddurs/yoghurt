//! Where facts come from.
//!
//! One module per package manager, each implementing [`crate::model::fact::Source`]
//! and knowing nothing about the others, the graph, or the interface. Adding a
//! package manager is a file here and nothing else.
//!
//! [`walk`] is the exception and is deliberately not an adapter for anything:
//! it reads the filesystem and says what is actually there, so that a claim
//! with no artifact is broken and an artifact with no claim is an orphan.

pub mod homebrew;
pub mod walk;
