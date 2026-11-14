//! Selective testing: which suites and packages a set of changed paths needs.

use std::collections::BTreeSet;

/// A workspace package, its directory, and the workspace packages it uses.
pub struct Package {
    pub name: &'static str,
    pub dir: &'static str,
    pub depends_on: &'static [&'static str],
}

/// Workspace members with tests. `xtask` itself is the harness: a change to it
/// runs everything. A test checks `depends_on` against each Cargo.toml.
pub const PACKAGES: &[Package] = &[
    Package {
        name: "hoshi-core",
        dir: "crates/hoshi-core",
        depends_on: &[],
    },
    Package {
        name: "hoshi-controller",
        dir: "crates/hoshi-controller",
        depends_on: &["hoshi-core"],
    },
    Package {
        name: "hoshi-canvas-spike",
        dir: "spikes/desktop-canvas",
        depends_on: &["hoshi-controller", "hoshi-core"],
    },
];

const SPIKE: &str = "hoshi-canvas-spike";

/// Paths that never affect a build or test result.
fn is_inert(path: &str) -> bool {
    let top_level_doc =
        !path.contains('/') && (path.ends_with(".md") || path == "LICENSE" || path == ".gitignore");
    top_level_doc
        || path.starts_with("docs/")
        || path.starts_with("licenses/")
        || path.starts_with(".github/ISSUE_TEMPLATE/")
        || path == ".github/pull_request_template.md"
        || path == ".github/copilot-instructions.md"
}

/// What a change needs to run.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Selection {
    /// Run every suite: workspace manifests, toolchain, CI or the harness changed,
    /// or a path is not recognised.
    pub everything: bool,
    /// Packages whose tests must run (changed ones plus their dependents).
    pub packages: BTreeSet<&'static str>,
    pub smoke: bool,
    pub scripts: bool,
}

impl Selection {
    pub fn is_empty(&self) -> bool {
        !self.everything && self.packages.is_empty() && !self.smoke && !self.scripts
    }
}

pub fn select<'a>(paths: impl IntoIterator<Item = &'a str>) -> Selection {
    let mut selection = Selection::default();
    for path in paths {
        if is_inert(path) {
            continue;
        }
        if let Some(package) = PACKAGES
            .iter()
            .find(|package| path.starts_with(&format!("{}/", package.dir)))
        {
            add_with_dependents(&mut selection.packages, package.name);
        } else if path.starts_with("scripts/") {
            selection.scripts = true;
            // The native cursor smoke drives this script.
            if path == "scripts/check_x11_cursor.py" {
                selection.smoke = true;
            }
        } else {
            selection.everything = true;
        }
    }
    if selection.packages.contains(SPIKE) {
        selection.smoke = true;
    }
    selection
}

fn add_with_dependents(selected: &mut BTreeSet<&'static str>, name: &'static str) {
    if !selected.insert(name) {
        return;
    }
    for package in PACKAGES {
        if package.depends_on.contains(&name) {
            add_with_dependents(selected, package.name);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn packages(selection: &Selection) -> Vec<&str> {
        selection.packages.iter().copied().collect()
    }

    #[test]
    fn core_change_runs_core_and_every_dependent_plus_smoke() {
        let selection = select(["crates/hoshi-core/src/painting.rs"]);
        assert_eq!(
            packages(&selection),
            ["hoshi-canvas-spike", "hoshi-controller", "hoshi-core"]
        );
        assert!(selection.smoke && !selection.everything && !selection.scripts);
    }

    #[test]
    fn controller_change_skips_core() {
        let selection = select(["crates/hoshi-controller/tests/eyedropper.rs"]);
        assert_eq!(
            packages(&selection),
            ["hoshi-canvas-spike", "hoshi-controller"]
        );
    }

    #[test]
    fn spike_change_runs_spike_and_smoke_only() {
        let selection = select(["spikes/desktop-canvas/ui/app-chrome.slint"]);
        assert_eq!(packages(&selection), ["hoshi-canvas-spike"]);
        assert!(selection.smoke);
    }

    #[test]
    fn docs_only_change_selects_nothing() {
        let selection = select(["docs/TESTING.md", "README.md", "CLAUDE.md"]);
        assert!(selection.is_empty(), "{selection:?}");
    }

    #[test]
    fn scripts_change_runs_scripts_and_cursor_script_adds_smoke() {
        let selection = select(["scripts/sync_project.py"]);
        assert!(selection.scripts && !selection.smoke && selection.packages.is_empty());
        assert!(select(["scripts/check_x11_cursor.py"]).smoke);
    }

    #[test]
    fn manifests_ci_harness_and_unknown_paths_run_everything() {
        for path in [
            "Cargo.lock",
            "Cargo.toml",
            "rust-toolchain.toml",
            ".cargo/config.toml",
            ".config/nextest.toml",
            ".github/workflows/ci.yml",
            "xtask/src/main.rs",
            "something/new.rs",
        ] {
            assert!(select([path]).everything, "{path}");
        }
    }

    #[test]
    fn a_crate_directory_prefix_is_not_a_match() {
        assert!(select(["crates/hoshi-core-extra/src/lib.rs"]).everything);
    }

    /// `depends_on` must match the path dependencies in each Cargo.toml, so the
    /// selector cannot silently skip a dependent.
    #[test]
    fn dependency_table_matches_manifests() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        for package in PACKAGES {
            let manifest = std::fs::read_to_string(root.join(package.dir).join("Cargo.toml"))
                .unwrap_or_else(|error| panic!("{}: {error}", package.dir));
            let mut found: Vec<&str> = PACKAGES
                .iter()
                .filter(|other| {
                    manifest.lines().any(|line| {
                        line.starts_with(&format!("{} ", other.name)) && line.contains("path =")
                    })
                })
                .map(|other| other.name)
                .collect();
            found.sort_unstable();
            let mut declared = package.depends_on.to_vec();
            declared.sort_unstable();
            assert_eq!(found, declared, "{} depends_on", package.name);
        }
    }

    /// Each package builds its integration tests as one binary
    /// (`tests/main.rs`, `autotests = false`), so a `tests/*.rs` file that is
    /// neither a module of it nor its own `[[test]]` would never run.
    #[test]
    fn every_integration_test_file_is_built() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        for package in PACKAGES {
            let dir = root.join(package.dir);
            let manifest = std::fs::read_to_string(dir.join("Cargo.toml")).unwrap();
            assert!(
                manifest.contains("autotests = false"),
                "{}: integration tests must be one binary",
                package.dir
            );
            let main = std::fs::read_to_string(dir.join("tests/main.rs")).unwrap();
            for entry in std::fs::read_dir(dir.join("tests")).unwrap() {
                let file = entry.unwrap().file_name().into_string().unwrap();
                let Some(module) = file.strip_suffix(".rs") else {
                    continue;
                };
                let built = module == "main"
                    || main.lines().any(|line| line == format!("mod {module};"))
                    || manifest.contains(&format!("path = \"tests/{file}\""));
                assert!(
                    built,
                    "{}/tests/{file} is not in tests/main.rs",
                    package.dir
                );
            }
        }
    }

    #[test]
    fn every_workspace_member_is_known() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let manifest = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
        let members = manifest
            .lines()
            .find(|line| line.starts_with("members"))
            .unwrap();
        for member in members.split('"').skip(1).step_by(2) {
            assert!(
                member == "xtask" || PACKAGES.iter().any(|package| package.dir == member),
                "workspace member {member} missing from PACKAGES"
            );
        }
    }
}
