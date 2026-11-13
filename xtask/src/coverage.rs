//! Line-coverage measurement and floors (cargo-llvm-cov).

/// A crate whose own sources must stay at or above a line-coverage floor.
pub struct Floor {
    pub package: &'static str,
    /// Matches this crate's source paths in coverage reports.
    pub path: &'static str,
    /// Minimum line coverage in percent: the CI-measured baseline rounded
    /// down. CI is the reference; local runs can differ by a fraction.
    /// Raise it when coverage improves; never lower it to get green.
    pub lines: u32,
}

/// The domain crates, measured by their own tests. The desktop spike is a
/// disposable adapter and is not instrumented (it would rebuild Slint).
pub const FLOORS: &[Floor] = &[
    Floor {
        package: "hoshi-core",
        path: "crates/hoshi-core/",
        // CI baseline 96.03% (2026-11-13).
        lines: 96,
    },
    Floor {
        package: "hoshi-controller",
        path: "crates/hoshi-controller/",
        // CI baseline 87.73% (2026-11-12).
        lines: 87,
    },
];

/// The version CI installs and the runner expects.
pub const LLVM_COV_VERSION: &str = "0.9.1";

/// An `--ignore-filename-regex` that keeps only `floor`'s sources.
pub fn ignore_all_but(floor: &Floor) -> String {
    let mut others: Vec<String> = FLOORS
        .iter()
        .filter(|other| other.package != floor.package)
        .map(|other| regex_path(other.path))
        .collect();
    others.extend(["spikes/".into(), "xtask/".into()].map(|path: String| regex_path(&path)));
    format!("({})", others.join("|"))
}

/// A path prefix as a regex that matches either separator.
fn regex_path(path: &str) -> String {
    path.trim_end_matches('/')
        .split('/')
        .map(|part| part.replace('-', "\\-"))
        .collect::<Vec<_>>()
        .join("[/\\\\]")
        + "[/\\\\]"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignore_regex_names_every_other_crate() {
        let core = &FLOORS[0];
        let regex = ignore_all_but(core);
        assert!(regex.contains("hoshi\\-controller"), "{regex}");
        assert!(!regex.contains("hoshi\\-core"), "{regex}");
        assert!(
            regex.contains("spikes") && regex.contains("xtask"),
            "{regex}"
        );
    }

    #[test]
    fn floors_are_percentages_for_known_packages() {
        for floor in FLOORS {
            assert!(floor.lines <= 100);
            assert!(
                crate::changed::PACKAGES
                    .iter()
                    .any(|package| package.name == floor.package
                        && floor.path == format!("{}/", package.dir)),
                "{}",
                floor.package
            );
        }
    }
}
