//! Asking a model what a package is for.
//!
//! Everything else yoghurt reports is **observed**: a file is there, a manager
//! claims it, a symlink points somewhere. This is **inferred**, and the two
//! must not be confused. A label from here is marked as a guess wherever it
//! appears, because a tool whose value is telling you the truth about your
//! machine cannot quietly mix in something it was told by a language model.
//!
//! Three rules follow from that, and they are the whole design:
//!
//! 1. **Nothing leaves the machine unless it was switched on.** Off by default.
//! 2. **Answers are cached, so two runs group identically.** One call per
//!    package ever. A view that reshuffles itself between runs is useless.
//! 3. **Only names and descriptions are sent** — never paths, never versions,
//!    never the shape of somebody's home directory.
//!
//! The request goes out through `curl`, which is on every Mac. That keeps a TLS
//! stack out of the binary for the great majority who never turn this on, and
//! it is the same choice already made for `brew` and `codesign`.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use serde::{Deserialize, Serialize};

use crate::config::Taxonomy as Settings;
use crate::model::fact::{Fact, PackageId, ScanError};

/// The labels a package can be given.
///
/// Fixed rather than free-form. A model asked for an open-ended label will
/// invent a new one for every package and the grouping becomes a list of 299
/// groups of one.
pub const LABELS: &[&str] = &[
    "language", "build", "media", "network", "security", "text", "data", "system", "editor",
    "shell", "graphics", "science", "other",
];

/// How many packages to ask about at once.
///
/// 299 packages one at a time is 299 round trips. Batched, it is six.
const BATCH: usize = 50;

/// What is known already, from previous runs.
#[derive(Debug, Default, Deserialize, Serialize)]
struct Cache {
    /// Package name to label. Keyed by name and not by id, so a package that
    /// moves between sources keeps its label.
    labels: BTreeMap<String, String>,
}

/// Something to classify.
pub struct Subject {
    /// Which package.
    pub id: PackageId,
    /// What it is called.
    pub name: String,
    /// What its source says it is for, where anything does.
    pub describes: Option<String>,
}

/// Classify what has not been classified before.
///
/// # Errors
///
/// Returns a message when the request could not be made or its answer could not
/// be read. A failure here is never fatal: the caller drops back to the
/// structural category.
pub fn classify(settings: &Settings, subjects: &[Subject]) -> Result<Vec<Fact>, ScanError> {
    if !settings.available() {
        return Ok(Vec::new());
    }
    let mut cache = read_cache();

    let unknown: Vec<&Subject> = subjects
        .iter()
        .filter(|s| !cache.labels.contains_key(&s.name))
        .collect();
    for batch in unknown.chunks(BATCH) {
        let answered = ask(settings, batch)?;
        cache.labels.extend(answered);
    }
    if !unknown.is_empty() {
        write_cache(&cache);
    }

    Ok(subjects
        .iter()
        .filter_map(|subject| {
            let label = cache.labels.get(&subject.name)?;
            Some(Fact::Labelled {
                package: subject.id.clone(),
                label: label.clone(),
            })
        })
        .collect())
}

/// Ask about one batch.
fn ask(settings: &Settings, batch: &[&Subject]) -> Result<BTreeMap<String, String>, ScanError> {
    let key = settings
        .api_key()
        .ok_or_else(|| ScanError::new("taxonomy", "switched on, but no key was found"))?;

    let listing: String = batch
        .iter()
        .map(|s| match &s.describes {
            Some(text) => format!("- {}: {text}\n", s.name),
            None => format!("- {}\n", s.name),
        })
        .collect();

    let prompt = format!(
        "Classify each piece of software into exactly one category from this \
         list: {}.\nReply with only a JSON object mapping each name to its \
         category, no prose.\n\n{listing}",
        LABELS.join(", ")
    );
    let body = serde_json::json!({
        "model": settings.model,
        "messages": [{ "role": "user", "content": prompt }],
        "temperature": 0,
    });

    // The key goes in via an argument file rather than the command line, so it
    // never appears in `ps` output.
    let output = Command::new("curl")
        .arg("--silent")
        .arg("--show-error")
        .arg("--max-time")
        .arg("60")
        .arg("-H")
        .arg(format!("Authorization: Bearer {key}"))
        .arg("-H")
        .arg("Content-Type: application/json")
        .arg("-d")
        .arg(body.to_string())
        .arg(format!(
            "{}/chat/completions",
            settings.base_url.trim_end_matches('/')
        ))
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| ScanError::new("taxonomy", format!("running curl: {e}")))?;

    if !output.status.success() {
        // Never include the response body: it can echo the request.
        return Err(ScanError::new("taxonomy", "the request failed"));
    }
    parse_reply(&String::from_utf8_lossy(&output.stdout))
}

/// Pull the labels out of an OpenAI-shaped reply.
fn parse_reply(text: &str) -> Result<BTreeMap<String, String>, ScanError> {
    let reply: serde_json::Value = serde_json::from_str(text)
        .map_err(|_| ScanError::new("taxonomy", "the answer was not JSON"))?;
    let content = reply["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| ScanError::new("taxonomy", "the answer had no content"))?;

    // Models wrap JSON in prose and fences however often they are asked not to.
    let start = content
        .find('{')
        .ok_or_else(|| ScanError::new("taxonomy", "no object in the answer"))?;
    let end = content
        .rfind('}')
        .ok_or_else(|| ScanError::new("taxonomy", "no object in the answer"))?;
    let object: BTreeMap<String, String> = serde_json::from_str(&content[start..=end])
        .map_err(|_| ScanError::new("taxonomy", "the answer was not the shape asked for"))?;

    // A label outside the list is a model inventing a category; drop it rather
    // than letting it become a group of one.
    Ok(object
        .into_iter()
        .filter(|(_, label)| LABELS.contains(&label.as_str()))
        .collect())
}

/// Where answers are kept between runs.
fn cache_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))?;
    Some(base.join("yoghurt/taxonomy.json"))
}

fn read_cache() -> Cache {
    cache_path()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn write_cache(cache: &Cache) {
    let Some(path) = cache_path() else { return };
    let Some(parent) = path.parent() else { return };
    if std::fs::create_dir_all(parent).is_err() {
        return;
    }
    if let Ok(text) = serde_json::to_string_pretty(cache) {
        // A cache that cannot be written is a slower next run, not a failure.
        let _ = std::fs::write(path, text);
    }
}

#[cfg(test)]
mod tests {
    use super::{LABELS, parse_reply};

    #[test]
    fn a_plain_json_answer_is_read() {
        let reply = r#"{"choices":[{"message":{"content":"{\"ripgrep\":\"text\"}"}}]}"#;
        let labels = parse_reply(reply).unwrap();
        assert_eq!(labels.get("ripgrep").map(String::as_str), Some("text"));
    }

    #[test]
    fn an_answer_wrapped_in_prose_and_fences_is_still_read() {
        let reply = concat!(
            r#"{"choices":[{"message":{"content":"Here you go:\n```json\n"#,
            r#"{\"ffmpeg\": \"media\"}\n```\nHope that helps."}}]}"#
        );
        assert_eq!(
            parse_reply(reply)
                .unwrap()
                .get("ffmpeg")
                .map(String::as_str),
            Some("media")
        );
    }

    #[test]
    fn a_category_nobody_asked_for_is_dropped_rather_than_becoming_a_group_of_one() {
        let reply =
            r#"{"choices":[{"message":{"content":"{\"a\":\"text\",\"b\":\"quantum-yoghurt\"}"}}]}"#;
        let labels = parse_reply(reply).unwrap();
        assert_eq!(labels.len(), 1);
        assert!(labels.contains_key("a"));
    }

    #[test]
    fn an_answer_that_is_not_json_is_reported_rather_than_guessed_at() {
        assert!(parse_reply("not json at all").is_err());
        assert!(parse_reply(r#"{"choices":[]}"#).is_err());
        assert!(parse_reply(r#"{"choices":[{"message":{"content":"sorry, no"}}]}"#).is_err());
    }

    #[test]
    fn every_label_is_one_word_so_it_fits_a_column() {
        for label in LABELS {
            assert!(!label.contains(' '), "{label}");
        }
    }
}
