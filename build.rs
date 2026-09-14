//! Stamp the binary with the commit it was built from.
//!
//! Without this, "is the thing on my PATH the thing I just merged" is a
//! question you can only answer by rebuilding and seeing whether anything
//! changes.

use std::process::Command;

fn main() {
    // Rebuild when HEAD moves, so the stamp cannot go stale while the source
    // stays the same.
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs");

    let commit = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|out| out.status.success())
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|sha| sha.trim().to_owned())
        .unwrap_or_default();

    // Absent from a source tarball with no git history, which is fine: the
    // version alone is the answer there.
    println!("cargo:rustc-env=YOGHURT_COMMIT={commit}");
}
