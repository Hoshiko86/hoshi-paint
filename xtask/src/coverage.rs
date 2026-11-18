//! Line-coverage measurement and floors (cargo-llvm-cov).

/// A crate whose own sources must stay at or above a line-coverage floor.
pub struct Floor {
    pub package: &'static str,
    /// Matches this crate's source paths in coverage reports.
    pub path: &'static str,
    /// Minimum line coverage in percent; baselines and margins are recorded below.
    /// CI is the reference; local runs can differ by a fraction.
    /// Raise it when coverage improves; never lower it to get green.
    pub lines: u32,
}

/// The domain crates and desktop adapter, measured by their own tests.
pub const FLOORS: &[Floor] = &[
    Floor {
        package: "hoshi-core",
        path: "crates/hoshi-core/",
        // CI baseline 96.73% (2026-11-16).
        lines: 96,
    },
    Floor {
        package: "hoshi-controller",
        path: "crates/hoshi-controller/",
        // CI baseline 97.77% (2026-11-16).
        lines: 97,
    },
    Floor {
        package: "hoshi-desktop",
        path: "apps/desktop/",
        // Local baseline 84.81% after entry-point exclusion (2026-11-17).
        // Native-only screen picking is carried by smoke, keeping a 2-point margin.
        lines: 82,
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
    others.push(regex_path("xtask/"));
    if !floor.path.starts_with("apps/") {
        others.push(regex_path("apps/"));
    } else {
        others.push(format!(
            r"(^|[/\\]){}main\.rs$",
            regex_path("apps/desktop/src/")
        ));
    }
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
        assert!(regex.contains("apps") && regex.contains("xtask"), "{regex}");
    }

    #[test]
    fn desktop_gate_keeps_apps_while_domain_gates_ignore_them() {
        let entry_point = r"(^|[/\\])apps[/\\]desktop[/\\]src[/\\]main\.rs$";
        for floor in FLOORS {
            let regex = ignore_all_but(floor);
            if floor.package == "hoshi-desktop" {
                assert!(regex.contains(entry_point), "{regex}");
                assert!(regex.contains("hoshi\\-core"), "{regex}");
                assert!(regex.contains("hoshi\\-controller"), "{regex}");
            } else {
                assert!(regex.contains(&regex_path("apps/")), "{regex}");
                assert!(!regex.contains(entry_point), "{regex}");
            }
        }
        assert_eq!(
            ignore_all_but(&FLOORS[0]),
            r"(crates[/\\]hoshi\-controller[/\\]|apps[/\\]desktop[/\\]|xtask[/\\]|apps[/\\])"
        );
        assert_eq!(
            ignore_all_but(&FLOORS[1]),
            r"(crates[/\\]hoshi\-core[/\\]|apps[/\\]desktop[/\\]|xtask[/\\]|apps[/\\])"
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
