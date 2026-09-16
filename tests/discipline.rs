//! Rules about the tests themselves.
//!
//! Every adapter reads a directory, and the one directory it must never read
//! during a test is the developer's own. A suite that passes because of what
//! happens to be installed on one machine is a suite that tells you nothing
//! about anybody else's — and it fails in CI, weeks later, for reasons nobody
//! can reproduce.
//!
//! This is checked rather than trusted, because it is the kind of thing that
//! holds until somebody is in a hurry.

use std::fs;
use std::path::{Path, PathBuf};

/// Every `.rs` file under `src`.
fn sources() -> Vec<PathBuf> {
    fn walk(dir: &Path, found: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, found);
            } else if path.extension().is_some_and(|e| e == "rs") {
                found.push(path);
            }
        }
    }
    let mut found = Vec::new();
    walk(Path::new("src"), &mut found);
    found.sort();
    found
}

/// Whatever follows `#[cfg(test)]`, which is near enough the test module.
fn test_code(text: &str) -> Option<&str> {
    let at = text.find("#[cfg(test)]")?;
    Some(&text[at..])
}

#[test]
fn no_test_reads_the_machine_it_is_running_on() {
    let mut offenders = Vec::new();
    for path in sources() {
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let Some(tests) = test_code(&text) else {
            continue;
        };
        for (number, line) in tests.lines().enumerate() {
            // `from_environment` is how an adapter finds the real machine.
            // Inside a test it is the developer's own, which is the one machine
            // a result must never depend on.
            if line.contains("from_environment()") && !line.trim_start().starts_with("///") {
                offenders.push(format!("{}: {}", path.display(), number));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "a test must take a root and be given a fixture:\n  {}",
        offenders.join("\n  ")
    );
}

#[test]
fn every_adapter_can_be_pointed_somewhere_else() {
    // An adapter that can only read the real machine cannot be tested at all,
    // so each one has a constructor taking the root it should read.
    let adapters = [
        "src/source/homebrew.rs",
        "src/source/cargo.rs",
        "src/source/macos.rs",
        "src/source/node.rs",
        "src/source/tools.rs",
        "src/source/walk.rs",
    ];
    for path in adapters {
        let text = fs::read_to_string(path).unwrap_or_default();
        assert!(
            text.contains("pub fn new("),
            "{path} has no way to be pointed at a fixture"
        );
    }
}

#[test]
fn every_adapter_has_tests_of_its_own() {
    for path in sources() {
        let name = path.to_string_lossy().to_string();
        if !name.starts_with("src/source/") || name.ends_with("mod.rs") {
            continue;
        }
        let text = fs::read_to_string(&path).unwrap_or_default();
        assert!(test_code(&text).is_some(), "{name} has no tests");
    }
}

#[test]
fn nothing_shells_out_from_a_test() {
    // A subprocess in a test is the machine again, wearing a hat: `brew`,
    // `codesign` and `pkgutil` all answer differently per machine, so every one
    // of them is injected rather than called.
    let mut offenders = Vec::new();
    for path in sources() {
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let Some(tests) = test_code(&text) else {
            continue;
        };
        if tests.contains("Command::new(") {
            offenders.push(path.display().to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "inject the answer instead:\n  {}",
        offenders.join("\n  ")
    );
}
