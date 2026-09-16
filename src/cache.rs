//! What the last scan found.
//!
//! Reading eight package managers takes three seconds, and a tool people wait
//! three seconds for is a tool they stop opening. So the first frame comes from
//! here and says how old it is, and the machine is read again only when asked.
//!
//! Freshness is shown, never guessed at. A person must always know whether they
//! are looking at the machine or at a memory of it, which is why the age is in
//! the header rather than implied by the absence of a warning.

use std::path::PathBuf;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::model::fact::Fact;

/// The on-disk format.
///
/// Versioned so that a cache written by an older yoghurt is discarded rather
/// than misread. Silently misreading a cache would produce a machine that never
/// existed, which is worse than a slow start.
const FORMAT: u32 = 1;

/// A scan, saved.
#[derive(Debug, Deserialize, Serialize)]
pub struct Cached {
    format: u32,
    /// When the scan finished.
    pub scanned: SystemTime,
    /// Everything it found.
    pub facts: Vec<Fact>,
}

/// Where it lives.
#[must_use]
pub fn path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))?;
    Some(base.join("yoghurt/scan.json"))
}

/// The last scan, if there is one this version understands.
///
/// Returns `None` for anything unreadable — missing, corrupt, or written by a
/// different format. All three mean the same thing to a caller: read the
/// machine instead.
#[must_use]
pub fn read() -> Option<Cached> {
    read_from(&path()?)
}

/// Read one specific file. Tests point this at a fixture.
#[must_use]
pub fn read_from(path: &std::path::Path) -> Option<Cached> {
    let text = std::fs::read_to_string(path).ok()?;
    let cached: Cached = serde_json::from_str(&text).ok()?;
    (cached.format == FORMAT).then_some(cached)
}

/// Save a scan for next time.
///
/// Failing to write is a slower next run, not a failure: nothing here is worth
/// interrupting somebody over.
pub fn write(facts: &[Fact], scanned: SystemTime) {
    let Some(path) = path() else { return };
    write_to(&path, facts, scanned);
}

/// Write to one specific file. Tests point this at a fixture.
pub fn write_to(path: &std::path::Path, facts: &[Fact], scanned: SystemTime) {
    let Some(parent) = path.parent() else { return };
    if std::fs::create_dir_all(parent).is_err() {
        return;
    }
    let cached = Cached {
        format: FORMAT,
        scanned,
        facts: facts.to_vec(),
    };
    if let Ok(text) = serde_json::to_string(&cached) {
        let _ = std::fs::write(path, text);
    }
}

#[cfg(test)]
mod tests {
    use super::{Cached, FORMAT, read_from, write_to};
    use crate::model::fact::{Fact, PackageId};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime};

    struct Scratch(PathBuf);

    impl Scratch {
        fn new(tag: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("yoghurt-cache-{tag}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).expect("create");
            Self(dir)
        }

        fn file(&self) -> PathBuf {
            self.0.join("scan.json")
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn facts() -> Vec<Fact> {
        vec![
            Fact::Package {
                id: PackageId::new("homebrew", "ripgrep"),
                version: Some("15.2.0".to_owned()),
            },
            Fact::Wanted {
                package: PackageId::new("homebrew", "ripgrep"),
            },
            Fact::Size {
                artifact: PathBuf::from("/opt/homebrew/Cellar/ripgrep"),
                bytes: 6_500_000,
            },
        ]
    }

    #[test]
    fn what_goes_in_comes_back_out() {
        const DAY: u64 = 60 * 60 * 24;
        let scratch = Scratch::new("roundtrip");
        let at = SystemTime::UNIX_EPOCH + Duration::from_secs(20_000 * DAY);
        write_to(&scratch.file(), &facts(), at);

        let read = read_from(&scratch.file()).expect("a cache");
        assert_eq!(
            read.facts,
            facts(),
            "a fact must survive the disk unchanged"
        );
        assert_eq!(read.scanned, at);
    }

    #[test]
    fn a_missing_cache_is_not_an_error() {
        assert!(read_from(&PathBuf::from("/nonexistent/for/sure")).is_none());
    }

    #[test]
    fn a_corrupt_cache_is_discarded_rather_than_half_read() {
        let scratch = Scratch::new("corrupt");
        fs::write(scratch.file(), "{not json").expect("write");
        assert!(read_from(&scratch.file()).is_none());
    }

    #[test]
    fn a_cache_from_another_format_is_refused_rather_than_misread() {
        let scratch = Scratch::new("oldformat");
        let stale = format!(
            r#"{{"format":{},"scanned":{{"secs_since_epoch":0,"nanos_since_epoch":0}},"facts":[]}}"#,
            FORMAT + 1
        );
        fs::write(scratch.file(), stale).expect("write");
        assert!(
            read_from(&scratch.file()).is_none(),
            "misreading one would produce a machine that never existed"
        );
    }

    #[test]
    fn writing_somewhere_impossible_is_survivable() {
        // A cache that cannot be written is a slower next run, not a failure.
        write_to(
            &PathBuf::from("/nonexistent/for/sure/scan.json"),
            &facts(),
            SystemTime::now(),
        );
    }

    #[test]
    fn the_format_is_recorded_so_it_can_be_refused_later() {
        let scratch = Scratch::new("format");
        write_to(&scratch.file(), &facts(), SystemTime::now());
        let text = fs::read_to_string(scratch.file()).expect("read");
        let parsed: Cached = serde_json::from_str(&text).expect("parse");
        assert_eq!(parsed.format, FORMAT);
    }
}
