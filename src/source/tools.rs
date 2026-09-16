//! Four small sources that keep their inventories in a directory.
//!
//! Go, Ruby gems, pipx and uv. None is big enough to deserve its own file and
//! each is the same shape: look where the ecosystem puts things, read what is
//! there, emit facts. Together they are the difference between "unclaimed" and
//! a name for another few dozen things on a machine.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::model::fact::{Fact, PackageId, ScanError, Source};
use crate::source::{children, size_of};

/// Binaries `go install` left behind.
///
/// Go writes no manifest, but the module path and version are recorded *inside*
/// the binary, and `go version -m` reads them back out — which is the only way
/// to tell `gopls` from a file somebody happened to name `gopls`.
pub struct Go {
    bin: PathBuf,
    /// How to read module metadata. Injected so tests never run the toolchain.
    read: fn(&[PathBuf]) -> String,
}

impl Go {
    /// The name this source reports itself under.
    pub const NAME: &'static str = "go";

    /// Go binaries on this machine, if there are any.
    #[must_use]
    pub fn from_environment() -> Option<Self> {
        let bin = std::env::var_os("GOBIN").map(PathBuf::from).or_else(|| {
            std::env::var_os("GOPATH")
                .map(|p| PathBuf::from(p).join("bin"))
                .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join("go/bin")))
        })?;
        bin.is_dir().then_some(Self {
            bin,
            read: go_version,
        })
    }

    /// Go binaries somewhere else, with the metadata supplied.
    #[must_use]
    pub fn new(bin: PathBuf, read: fn(&[PathBuf]) -> String) -> Self {
        Self { bin, read }
    }
}

/// `go version -m` over every binary at once.
///
/// One subprocess rather than one per file: the command takes a list, and a
/// machine with forty go tools would otherwise pay forty process spawns.
fn go_version(binaries: &[PathBuf]) -> String {
    Command::new("go")
        .arg("version")
        .arg("-m")
        .args(binaries)
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).into_owned())
        .unwrap_or_default()
}

impl Source for Go {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn scan(&self) -> Result<Vec<Fact>, ScanError> {
        let binaries = children(&self.bin);
        if binaries.is_empty() {
            return Ok(Vec::new());
        }

        let mut facts = Vec::new();
        // Output is one block per binary: a path line, then `\tmod\t<module>\t<version>`.
        let report = (self.read)(&binaries);
        let mut current: Option<PathBuf> = None;

        for line in report.lines() {
            if let Some(rest) = line
                .strip_suffix(": go1.0")
                .or_else(|| line.split(':').next())
            {
                let candidate = PathBuf::from(rest.trim());
                if binaries.contains(&candidate) {
                    current = Some(candidate);
                    continue;
                }
            }
            let Some(binary) = current.clone() else {
                continue;
            };
            let mut parts = line.split('\t').filter(|p| !p.is_empty());
            if parts.next() != Some("mod") {
                continue;
            }
            let (Some(module), Some(version)) = (parts.next(), parts.next()) else {
                continue;
            };

            let id = PackageId::new(Self::NAME, module);
            facts.push(Fact::Package {
                id: id.clone(),
                version: Some(version.to_owned()),
            });
            // `go install` is always something somebody typed.
            facts.push(Fact::Wanted {
                package: id.clone(),
            });
            facts.push(Fact::Size {
                artifact: binary.clone(),
                bytes: size_of(&binary),
            });
            facts.push(Fact::Owns {
                package: id,
                artifact: binary,
            });
            current = None;
        }
        Ok(facts)
    }
}

/// Installed Ruby gems.
pub struct Gem {
    roots: Vec<PathBuf>,
}

impl Gem {
    /// The name this source reports itself under.
    pub const NAME: &'static str = "gem";

    /// The gem directories on this machine.
    #[must_use]
    pub fn from_environment() -> Self {
        let mut roots = vec![PathBuf::from("/Library/Ruby/Gems")];
        if let Some(home) = std::env::var_os("HOME") {
            roots.push(PathBuf::from(&home).join(".gem/ruby"));
        }
        roots.push(PathBuf::from("/opt/homebrew/lib/ruby/gems"));
        Self { roots }
    }

    /// Gems somewhere else. Tests point this at a fixture tree.
    #[must_use]
    pub fn new(roots: Vec<PathBuf>) -> Self {
        Self { roots }
    }
}

/// `rails-7.1.3` into `("rails", "7.1.3")`.
///
/// A gem name may contain hyphens, so the version is the last segment that
/// begins with a digit — splitting on the first hyphen would turn
/// `net-http-persistent` into `net`.
#[must_use]
pub fn split_gem(directory: &str) -> Option<(String, String)> {
    let at = directory
        .char_indices()
        .rfind(|(i, c)| *c == '-' && directory[i + 1..].starts_with(|n: char| n.is_ascii_digit()))
        .map(|(i, _)| i)?;
    Some((directory[..at].to_owned(), directory[at + 1..].to_owned()))
}

impl Source for Gem {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn scan(&self) -> Result<Vec<Fact>, ScanError> {
        let mut facts = Vec::new();
        for root in &self.roots {
            // A root holds one directory per ruby version, each with `gems/`.
            for version_dir in children(root) {
                for gem in children(&version_dir.join("gems")) {
                    let Some(name) = gem.file_name().and_then(|n| n.to_str()) else {
                        continue;
                    };
                    let Some((name, version)) = split_gem(name) else {
                        continue;
                    };
                    let id = PackageId::new(Self::NAME, name);
                    facts.push(Fact::Package {
                        id: id.clone(),
                        version: Some(version),
                    });
                    facts.push(Fact::Size {
                        artifact: gem.clone(),
                        bytes: size_of(&gem),
                    });
                    facts.push(Fact::Owns {
                        package: id,
                        artifact: gem,
                    });
                }
            }
        }
        Ok(facts)
    }
}

/// Python command-line tools, from pipx or uv.
///
/// Both keep one virtual environment per tool in a directory named after it,
/// which is all this needs — the metadata files they also write differ in
/// format and say nothing the directory name does not.
pub struct PythonTools {
    /// Directory, and the name to report it under.
    roots: Vec<(PathBuf, &'static str)>,
}

impl PythonTools {
    /// pipx and uv, wherever they keep their tools.
    #[must_use]
    pub fn from_environment() -> Self {
        let mut roots = Vec::new();
        if let Some(home) = std::env::var_os("HOME") {
            let home = PathBuf::from(home);
            roots.push((home.join(".local/share/pipx/venvs"), "pipx"));
            roots.push((home.join(".local/share/uv/tools"), "uv"));
        }
        Self { roots }
    }

    /// Tools somewhere else. Tests point this at a fixture tree.
    #[must_use]
    pub fn new(roots: Vec<(PathBuf, &'static str)>) -> Self {
        Self { roots }
    }
}

impl Source for PythonTools {
    fn name(&self) -> &'static str {
        "python tools"
    }

    fn scan(&self) -> Result<Vec<Fact>, ScanError> {
        let mut facts = Vec::new();
        for (root, source) in &self.roots {
            for tool in children(root) {
                let Some(name) = tool.file_name().and_then(|n| n.to_str()) else {
                    continue;
                };
                let id = PackageId::new(*source, name);
                facts.push(Fact::Package {
                    id: id.clone(),
                    version: version_of(&tool),
                });
                // Nothing installs a pipx or uv tool as a dependency.
                facts.push(Fact::Wanted {
                    package: id.clone(),
                });
                facts.push(Fact::Size {
                    artifact: tool.clone(),
                    bytes: size_of(&tool),
                });
                facts.push(Fact::Owns {
                    package: id,
                    artifact: tool,
                });
            }
        }
        Ok(facts)
    }
}

/// The version a tool's environment records, where one is findable.
///
/// Both managers write a metadata file, in different formats, so this looks for
/// the installed distribution instead — `lib/pythonX/site-packages/<name>-<v>.dist-info`
/// is where the answer actually is.
fn version_of(tool: &Path) -> Option<String> {
    let name = tool.file_name()?.to_str()?;
    for python in children(&tool.join("lib")) {
        for entry in children(&python.join("site-packages")) {
            let file = entry.file_name()?.to_str()?;
            if let Some(rest) = file.strip_suffix(".dist-info")
                && let Some((found, version)) = split_gem(rest)
                && found.replace('_', "-") == name.replace('_', "-")
            {
                return Some(version);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{Gem, Go, PythonTools, split_gem};
    use crate::Graph;
    use crate::model::fact::{PackageId, Source as _};
    use std::fs;
    use std::path::PathBuf;

    struct Tree(PathBuf);

    impl Tree {
        fn new(tag: &str) -> Self {
            let root =
                std::env::temp_dir().join(format!("yoghurt-tools-{tag}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).expect("create");
            Self(root)
        }

        fn file(&self, path: &str, contents: &str) -> PathBuf {
            let full = self.0.join(path);
            fs::create_dir_all(full.parent().expect("parent")).expect("create parent");
            fs::write(&full, contents).expect("write");
            full
        }

        fn dir(&self, path: &str) -> PathBuf {
            let full = self.0.join(path);
            fs::create_dir_all(&full).expect("create dir");
            full
        }
    }

    impl Drop for Tree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// Stands in for `go version -m`, so no test runs the toolchain.
    fn stub_go(binaries: &[PathBuf]) -> String {
        use std::fmt::Write as _;
        let mut out = String::new();
        for binary in binaries {
            let _ = writeln!(out, "{}: go1.24.0", binary.display());
            if binary.ends_with("gopls") {
                out.push_str("\tpath\tgolang.org/x/tools/gopls\n");
                out.push_str("\tmod\tgolang.org/x/tools/gopls\tv0.23.0\th1:abc=\n");
            }
        }
        out
    }

    #[test]
    fn a_go_binary_is_named_by_the_module_inside_it() {
        let tree = Tree::new("go");
        tree.file("bin/gopls", "binary");
        let go = Go::new(tree.0.join("bin"), stub_go);

        let graph = Graph::from_facts(go.scan().unwrap());
        let id = PackageId::new("go", "golang.org/x/tools/gopls");
        assert_eq!(
            graph.package(&id).unwrap().version.as_deref(),
            Some("v0.23.0"),
            "the only way to tell gopls from a file somebody named gopls"
        );
        assert!(
            graph.package(&id).unwrap().wanted,
            "go install is always deliberate"
        );
    }

    #[test]
    fn a_binary_with_no_module_metadata_is_not_invented() {
        let tree = Tree::new("gostray");
        tree.file("bin/something", "not a go binary");
        let go = Go::new(tree.0.join("bin"), stub_go);
        assert_eq!(
            go.scan().unwrap(),
            Vec::new(),
            "it stays unclaimed, honestly"
        );
    }

    /// Asking the toolchain about nothing is a subprocess for no reason.
    fn explode(_: &[PathBuf]) -> String {
        panic!("must not ask the toolchain about nothing");
    }

    #[test]
    fn an_empty_go_bin_costs_no_subprocess() {
        let tree = Tree::new("goempty");
        tree.dir("bin");
        assert_eq!(
            Go::new(tree.0.join("bin"), explode).scan().unwrap(),
            Vec::new()
        );
    }

    #[test]
    fn a_gem_name_may_contain_hyphens() {
        assert_eq!(
            split_gem("net-http-persistent-4.0.2"),
            Some(("net-http-persistent".to_owned(), "4.0.2".to_owned())),
            "splitting on the first hyphen would call this `net`"
        );
        assert_eq!(
            split_gem("rails-7.1.3"),
            Some(("rails".to_owned(), "7.1.3".to_owned()))
        );
        assert_eq!(split_gem("nothing-here"), None);
    }

    #[test]
    fn every_gem_directory_becomes_a_package() {
        let tree = Tree::new("gems");
        tree.dir("Gems/3.4.0/gems/rails-7.1.3");
        tree.dir("Gems/3.4.0/gems/net-http-persistent-4.0.2");
        let gem = Gem::new(vec![tree.0.join("Gems")]);

        let graph = Graph::from_facts(gem.scan().unwrap());
        assert!(graph.package(&PackageId::new("gem", "rails")).is_some());
        assert!(
            graph
                .package(&PackageId::new("gem", "net-http-persistent"))
                .is_some()
        );
    }

    #[test]
    fn a_python_tool_is_reported_under_the_manager_that_installed_it() {
        let tree = Tree::new("python");
        tree.dir("pipx/venvs/black/lib/python3.12/site-packages/black-24.10.0.dist-info");
        tree.dir("uv/tools/ruff/lib/python3.12/site-packages/ruff-0.8.4.dist-info");
        let tools = PythonTools::new(vec![
            (tree.0.join("pipx/venvs"), "pipx"),
            (tree.0.join("uv/tools"), "uv"),
        ]);

        let graph = Graph::from_facts(tools.scan().unwrap());
        assert_eq!(
            graph
                .package(&PackageId::new("pipx", "black"))
                .unwrap()
                .version
                .as_deref(),
            Some("24.10.0")
        );
        assert_eq!(
            graph
                .package(&PackageId::new("uv", "ruff"))
                .unwrap()
                .version
                .as_deref(),
            Some("0.8.4")
        );
    }

    #[test]
    fn a_tool_whose_version_cannot_be_found_is_still_a_package() {
        let tree = Tree::new("noversion");
        tree.dir("venvs/mystery");
        let tools = PythonTools::new(vec![(tree.0.join("venvs"), "pipx")]);
        let graph = Graph::from_facts(tools.scan().unwrap());
        let mystery = graph.package(&PackageId::new("pipx", "mystery")).unwrap();
        assert!(mystery.version.is_none(), "unknown, rather than absent");
    }

    #[test]
    fn none_of_them_being_installed_yields_no_facts_and_no_error() {
        let absent = PathBuf::from("/nonexistent/for/sure");
        assert_eq!(Gem::new(vec![absent.clone()]).scan().unwrap(), Vec::new());
        assert_eq!(
            PythonTools::new(vec![(absent, "pipx")]).scan().unwrap(),
            Vec::new()
        );
    }
}
