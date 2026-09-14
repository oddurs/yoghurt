//! What the person has asked for, between runs.
//!
//! Read from `~/.config/yoghurt/config.toml`, which is created only when
//! something is turned on. A machine that has never opted into anything has no
//! configuration file, and that is the point: the defaults are what the tool
//! promises, and the file is a record of departures from them.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Everything configurable.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct Config {
    /// Asking a model what packages are for.
    pub taxonomy: Taxonomy,
}

/// Optional classification through an OpenAI-shaped API.
///
/// Off unless somebody turns it on. yoghurt's promise is that it does not touch
/// the network unless asked, and this is the only thing that would.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct Taxonomy {
    /// Whether to ask at all. Nothing leaves the machine while this is false.
    pub enabled: bool,
    /// Which model to ask.
    pub model: String,
    /// Where to ask it.
    pub base_url: String,
    /// The environment variable holding the key.
    pub api_key_env: String,
    /// A file holding the key, for when it is not in the environment.
    ///
    /// Read in the shape `KEY=value`, optionally `export`-prefixed, so one file
    /// can serve several tools rather than each keeping its own copy of a
    /// secret.
    pub api_key_file: String,
}

impl Default for Taxonomy {
    fn default() -> Self {
        Self {
            enabled: false,
            model: "anthropic/claude-haiku-4.5".to_owned(),
            base_url: "https://openrouter.ai/api/v1".to_owned(),
            api_key_env: "OPENROUTER_API_KEY".to_owned(),
            api_key_file: String::new(),
        }
    }
}

impl Config {
    /// Where the configuration lives.
    #[must_use]
    pub fn path() -> Option<PathBuf> {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
        Some(base.join("yoghurt/config.toml"))
    }

    /// Read it, or the defaults when there is none.
    ///
    /// # Errors
    ///
    /// Returns a message when a file exists but cannot be understood. A missing
    /// file is not an error — it is the ordinary case.
    pub fn load() -> Result<Self, String> {
        let Some(path) = Self::path() else {
            return Ok(Self::default());
        };
        Self::read(&path)
    }

    /// Read one specific file. Tests point this at a fixture.
    ///
    /// # Errors
    ///
    /// Returns a message when the file exists but is not valid configuration.
    pub fn read(path: &Path) -> Result<Self, String> {
        let Ok(text) = std::fs::read_to_string(path) else {
            return Ok(Self::default());
        };
        toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
    }
}

impl Taxonomy {
    /// The key, without ever putting it somewhere it could be read back.
    ///
    /// The environment wins, so a key can be supplied for one run without being
    /// written down. Otherwise the key file, which is how one secret serves
    /// several tools instead of being copied into each of their configs.
    ///
    /// Returns `None` rather than an error: no key simply means this is not
    /// available, which is the same as not being switched on.
    #[must_use]
    pub fn api_key(&self) -> Option<String> {
        if let Some(key) = std::env::var(&self.api_key_env)
            .ok()
            .filter(|k| !k.is_empty())
        {
            return Some(key);
        }
        if self.api_key_file.is_empty() {
            return None;
        }
        key_from_file(&expand(&self.api_key_file), &self.api_key_env)
    }

    /// Whether this is switched on *and* usable.
    #[must_use]
    pub fn available(&self) -> bool {
        self.enabled && self.api_key().is_some()
    }
}

/// `~` for the home directory, and nothing else. Config paths are written by
/// people, and people write `~`.
fn expand(path: &str) -> PathBuf {
    let Some(rest) = path.strip_prefix("~/") else {
        return PathBuf::from(path);
    };
    std::env::var_os("HOME").map_or_else(
        || PathBuf::from(path),
        |home| PathBuf::from(home).join(rest),
    )
}

/// Pull one assignment out of a shell-style env file.
///
/// Accepts `KEY=value` and `export KEY=value`, ignores comments and blank
/// lines, and strips surrounding quotes. Deliberately narrow: this reads a
/// secret, and a lenient parser reading secrets is a way to pick up the wrong
/// one.
fn key_from_file(path: &Path, want: &str) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    for line in text.lines() {
        let line = line.trim().trim_start_matches("export ").trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((name, value)) = line.split_once('=') else {
            continue;
        };
        if name.trim() != want {
            continue;
        }
        let value = value.trim().trim_matches(['"', '\'']).to_owned();
        if !value.is_empty() {
            return Some(value);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{Config, Taxonomy, key_from_file};
    use std::fs;
    use std::path::PathBuf;

    struct Scratch(PathBuf);

    impl Scratch {
        fn new(tag: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("yoghurt-cfg-{tag}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).expect("create");
            Self(dir)
        }

        fn write(&self, name: &str, text: &str) -> PathBuf {
            let path = self.0.join(name);
            fs::write(&path, text).expect("write");
            path
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn a_machine_with_no_config_gets_the_defaults() {
        let config = Config::read(&PathBuf::from("/nonexistent/for/sure")).unwrap();
        assert_eq!(config, Config::default());
        assert!(
            !config.taxonomy.enabled,
            "nothing is switched on by default"
        );
    }

    #[test]
    fn the_taxonomy_is_off_until_somebody_says_otherwise() {
        let scratch = Scratch::new("off");
        let path = scratch.write("config.toml", "[taxonomy]\nmodel = \"something/else\"\n");
        let config = Config::read(&path).unwrap();
        assert_eq!(config.taxonomy.model, "something/else");
        assert!(
            !config.taxonomy.enabled,
            "setting a model must not switch it on"
        );
    }

    #[test]
    fn a_config_that_cannot_be_understood_says_which_file() {
        let scratch = Scratch::new("bad");
        let path = scratch.write("config.toml", "this is not toml {{{");
        let error = Config::read(&path).unwrap_err();
        assert!(error.contains("config.toml"), "{error}");
    }

    #[test]
    fn the_key_is_read_from_a_shared_file_rather_than_copied_into_the_config() {
        let scratch = Scratch::new("keyfile");
        let path = scratch.write(
            "env",
            "# a comment\n\nexport OTHER_KEY=nope\nexport OPENROUTER_API_KEY=\"sk-or-test\"\n",
        );
        assert_eq!(
            key_from_file(&path, "OPENROUTER_API_KEY").as_deref(),
            Some("sk-or-test")
        );
    }

    #[test]
    fn a_file_without_the_key_yields_nothing_rather_than_the_wrong_key() {
        let scratch = Scratch::new("wrongkey");
        let path = scratch.write("env", "export SOMETHING_ELSE=sk-or-wrong\n");
        assert_eq!(
            key_from_file(&path, "OPENROUTER_API_KEY"),
            None,
            "reading a secret leniently is how you pick up the wrong one"
        );
    }

    #[test]
    fn switched_on_without_a_key_is_not_available() {
        let taxonomy = Taxonomy {
            enabled: true,
            api_key_env: "YOGHURT_DEFINITELY_UNSET_KEY".to_owned(),
            api_key_file: String::new(),
            ..Taxonomy::default()
        };
        assert!(!taxonomy.available());
    }

    #[test]
    fn the_config_never_holds_the_key_itself() {
        let rendered = toml::to_string(&Config::default()).expect("serialise");
        assert!(
            !rendered.contains("api_key ="),
            "a key must not be writable into the config"
        );
        assert!(rendered.contains("api_key_env"), "only where to find it");
    }
}
