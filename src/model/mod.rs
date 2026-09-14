//! What the machine is.
//!
//! One graph over three kinds of node — a package a manager records, an
//! artifact on disk, and a command resolvable on `PATH` — and the questions
//! that can be asked of it. All the rules live here: a source asserts facts and
//! a view projects what is here, but neither decides anything.

pub mod fact;
pub mod graph;
pub mod question;
