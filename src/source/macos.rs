//! Applications, and who put them there.
//!
//! An application bundle is the one thing on a Mac that usually arrives without
//! a package manager: dragged out of a disk image, installed by a vendor's own
//! updater, or bought from the App Store. Reported as orphans they are the
//! largest blind spot on the machine — 45 of them here — and that is a failure
//! of asking rather than a fact about the machine, because a bundle says a
//! great deal about itself.
//!
//! Three signals, in descending order of what they prove:
//!
//! 1. **A Mac App Store receipt.** The App Store installed it. That is a
//!    package manager and it should read as one.
//! 2. **A Developer ID signature.** Names the company that shipped it. Not an
//!    installer, so it does not make the app *owned* — but it answers "where did
//!    this come from", which is the question being asked.
//! 3. **Nothing.** Which is honest, and on this machine is one application.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::model::fact::{Fact, PackageId, ScanError, Source};
use crate::source::size_of_each;

/// Applications installed outside a package manager.
pub struct Applications {
    directories: Vec<PathBuf>,
    /// How to ask a bundle who signed it. Injected so tests never shell out.
    authority: fn(&Path) -> Option<String>,
}

impl Applications {
    /// The name this source reports itself under.
    pub const NAME: &'static str = "applications";

    /// The name the App Store's own installs are reported under.
    pub const APP_STORE: &'static str = "app store";

    /// The application directories on this machine.
    #[must_use]
    pub fn from_environment() -> Self {
        let mut directories = vec![PathBuf::from("/Applications")];
        if let Some(home) = std::env::var_os("HOME") {
            directories.push(PathBuf::from(home).join("Applications"));
        }
        Self {
            directories,
            authority: codesign_authority,
        }
    }

    /// Applications somewhere else, with signing answered by a stub.
    #[must_use]
    pub fn new(directories: Vec<PathBuf>, authority: fn(&Path) -> Option<String>) -> Self {
        Self {
            directories,
            authority,
        }
    }
}

/// Who signed this bundle, according to `codesign`.
///
/// Run once per bundle and never per file: there are 45 bundles and 3000
/// artifacts, and the difference is a tenth of a second against a minute.
fn codesign_authority(bundle: &Path) -> Option<String> {
    let output = Command::new("codesign")
        .arg("-dvvv")
        .arg(bundle)
        .output()
        .ok()?;
    // `codesign` writes its report to stderr even when it succeeds.
    let report = String::from_utf8_lossy(&output.stderr);
    report
        .lines()
        .find_map(|line| line.strip_prefix("Authority="))
        .map(|authority| authority.trim().to_owned())
}

/// `Developer ID Application: Figma, Inc. (T8RA8NE3B7)` into `Figma, Inc.`
///
/// The team identifier in brackets is stable and meaningless to a reader; the
/// name in front of it is the answer to "who made this".
#[must_use]
pub fn vendor(authority: &str) -> Option<String> {
    let name = authority.strip_prefix("Developer ID Application: ")?;
    let name = name.rsplit_once(" (").map_or(name, |(before, _)| before);
    Some(name.trim().to_owned())
}

/// What a bundle calls itself, and what version it says it is.
fn plist(bundle: &Path, key: &str) -> Option<String> {
    let contents = std::fs::read_to_string(bundle.join("Contents/Info.plist")).ok()?;
    // Info.plist is usually binary, in which case this finds nothing and the
    // caller does without. Parsing it properly is a dependency this does not
    // need for one optional field.
    let opening = format!("<key>{key}</key>");
    let after = contents.split_once(&opening)?.1;
    let value = after.split_once("<string>")?.1;
    let value = value.split_once("</string>")?.0;
    Some(value.trim().to_owned())
}

impl Source for Applications {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn scan(&self) -> Result<Vec<Fact>, ScanError> {
        // Measured together rather than one at a time: 44 bundles is 2.6
        // seconds in sequence and a fraction of that in parallel. Until this,
        // 23.7G of the machine reported nothing at all.
        let bundles: Vec<PathBuf> = self
            .directories
            .iter()
            .flat_map(|directory| crate::source::children(directory))
            .filter(|path| {
                path.extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("app"))
            })
            .collect();
        let sizes = size_of_each(&bundles);

        let mut facts = Vec::new();
        for (bundle, bytes) in bundles.into_iter().zip(sizes) {
            {
                let Some(name) = bundle.file_name().and_then(|n| n.to_str()) else {
                    continue;
                };
                let name = name.trim_end_matches(".app");

                // The App Store leaves a receipt, and that makes it the
                // installer rather than merely the shop.
                let from_store = bundle.join("Contents/_MASReceipt").exists();
                let source = if from_store {
                    Self::APP_STORE
                } else {
                    Self::NAME
                };
                let id = PackageId::new(source, name);

                facts.push(Fact::Package {
                    id: id.clone(),
                    version: plist(&bundle, "CFBundleShortVersionString"),
                });
                // Nothing installs an application as a dependency.
                facts.push(Fact::Wanted {
                    package: id.clone(),
                });
                facts.push(Fact::Size {
                    artifact: bundle.clone(),
                    bytes,
                });
                facts.push(Fact::Owns {
                    package: id.clone(),
                    artifact: bundle.clone(),
                });

                let description = if from_store {
                    Some("installed from the Mac App Store".to_owned())
                } else {
                    (self.authority)(&bundle)
                        .as_deref()
                        .and_then(vendor)
                        .map(|who| {
                            format!("signed by {who}, and installed outside any package manager")
                        })
                };
                if let Some(text) = description {
                    facts.push(Fact::Describes { package: id, text });
                }
            }
        }
        Ok(facts)
    }
}

#[cfg(test)]
mod tests {
    use super::{Applications, vendor};
    use crate::Graph;
    use crate::model::fact::{PackageId, Source as _};
    use std::fs;
    use std::path::{Path, PathBuf};

    /// A fixture `/Applications` that removes itself.
    struct Disk(PathBuf);

    impl Disk {
        fn new(tag: &str) -> Self {
            let root =
                std::env::temp_dir().join(format!("yoghurt-apps-{tag}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&root);

            // An App Store app, a signed one, an unsigned one, and something
            // that is not an application at all.
            for app in ["Things3.app", "Figma.app", "Mystery.app"] {
                fs::create_dir_all(root.join(app).join("Contents")).expect("create bundle");
            }
            fs::create_dir_all(root.join("Things3.app/Contents/_MASReceipt"))
                .expect("create receipt");
            fs::write(
                root.join("Figma.app/Contents/Info.plist"),
                "<plist><dict><key>CFBundleShortVersionString</key><string>126.1.2</string></dict></plist>",
            )
            .expect("write plist");
            fs::create_dir_all(root.join("not-an-app")).expect("create directory");
            Self(root)
        }

        fn source(&self) -> Applications {
            Applications::new(vec![self.0.clone()], stub_authority)
        }

        fn graph(&self) -> Graph {
            Graph::from_facts(self.source().scan().expect("scan"))
        }
    }

    impl Drop for Disk {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// Stands in for `codesign`, so no test shells out.
    fn stub_authority(bundle: &Path) -> Option<String> {
        bundle
            .file_name()
            .and_then(|n| n.to_str())
            .filter(|name| *name == "Figma.app")
            .map(|_| "Developer ID Application: Figma, Inc. (T8RA8NE3B7)".to_owned())
    }

    #[test]
    fn an_app_store_app_is_owned_by_the_app_store_rather_than_orphaned() {
        let disk = Disk::new("store");
        let graph = disk.graph();
        let id = PackageId::new("app store", "Things3");
        assert!(
            graph.package(&id).is_some(),
            "a receipt means the App Store installed it"
        );
        assert_eq!(graph.owners_of(&disk.0.join("Things3.app")), vec![&id]);
    }

    #[test]
    fn a_signed_app_names_the_company_that_shipped_it() {
        let disk = Disk::new("signed");
        let graph = disk.graph();
        let figma = graph
            .package(&PackageId::new("applications", "Figma"))
            .unwrap();
        assert_eq!(
            figma.describes.as_deref(),
            Some("signed by Figma, Inc., and installed outside any package manager")
        );
    }

    #[test]
    fn the_version_comes_off_the_bundle() {
        let disk = Disk::new("version");
        let graph = disk.graph();
        assert_eq!(
            graph
                .package(&PackageId::new("applications", "Figma"))
                .unwrap()
                .version
                .as_deref(),
            Some("126.1.2")
        );
    }

    #[test]
    fn an_unsigned_app_is_still_claimed_but_says_nothing_about_its_origin() {
        let disk = Disk::new("unsigned");
        let graph = disk.graph();
        let mystery = graph
            .package(&PackageId::new("applications", "Mystery"))
            .unwrap();
        assert!(
            mystery.describes.is_none(),
            "nothing is known, so nothing is claimed"
        );
        assert!(
            !graph.owners_of(&disk.0.join("Mystery.app")).is_empty(),
            "it is still an app"
        );
    }

    #[test]
    fn an_application_is_never_somebody_elses_dependency() {
        let disk = Disk::new("wanted");
        let graph = disk.graph();
        for (source, name) in [("app store", "Things3"), ("applications", "Figma")] {
            assert!(
                graph
                    .package(&PackageId::new(source, name))
                    .unwrap()
                    .wanted()
            );
        }
    }

    #[test]
    fn a_directory_that_is_not_a_bundle_is_ignored() {
        let disk = Disk::new("notanapp");
        let graph = disk.graph();
        assert!(
            graph
                .package(&PackageId::new("applications", "not-an-app"))
                .is_none()
        );
    }

    #[test]
    fn a_missing_applications_directory_is_not_an_error() {
        let absent =
            Applications::new(vec![PathBuf::from("/nonexistent/for/sure")], stub_authority);
        assert_eq!(absent.scan().unwrap(), Vec::new());
    }

    #[test]
    fn the_team_identifier_is_dropped_because_it_means_nothing_to_a_reader() {
        assert_eq!(
            vendor("Developer ID Application: Running with Crayons Ltd (XZZXE9SED4)").as_deref(),
            Some("Running with Crayons Ltd")
        );
        assert_eq!(
            vendor("Software Signing"),
            None,
            "Apple's own signing is not a vendor"
        );
        assert_eq!(vendor("Apple Mac OS Application Signing"), None);
    }
}
