//! Native desktop smoke suite: the single source of truth for which
//! `spikes/desktop-canvas/examples/*_smoke.rs` programs run, how, and which
//! snapshots they write.

/// How an environment variable is filled in for a smoke run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnvValue {
    /// A snapshot file written inside the smoke output directory.
    Snapshot(&'static str),
    /// The smoke output directory itself (examples that write several files).
    OutputDir,
}

/// What hosts the example on Linux.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Host {
    /// A real winit window on a virtual X server (`xvfb-run`).
    Xvfb,
    /// A private D-Bus session bus (`dbus-run-session`); no window.
    DbusSession,
}

/// One native smoke program.
#[derive(Debug)]
pub struct Smoke {
    /// Example name, matching `examples/<name>.rs`.
    pub name: &'static str,
    /// What the run proves, printed in the log.
    pub purpose: &'static str,
    pub host: Host,
    /// Virtual screen `WIDTHxHEIGHT`; `None` keeps the xvfb-run default.
    pub screen: Option<&'static str>,
    pub env: &'static [(&'static str, EnvValue)],
    /// Display scale factors to run at; empty runs once at the default.
    pub scales: &'static [Scale],
    /// Why the smoke cannot run on Windows or macOS; `None` runs everywhere.
    pub linux_only: Option<&'static str>,
}

/// A display scale and the virtual screen large enough for it.
#[derive(Debug)]
pub struct Scale {
    pub factor: &'static str,
    pub screen: Option<&'static str>,
}

use EnvValue::{OutputDir, Snapshot};

/// Every smoke example. `cargo xtask test smoke` fails if an
/// `examples/*_smoke.rs` file is missing here.
pub const SMOKES: &[Smoke] = &[
    Smoke {
        name: "desktop_smoke",
        purpose: "real winit/software backend draws and edits a shape",
        host: Host::Xvfb,
        screen: None,
        env: &[
            ("HOSHI_SPIKE_SNAPSHOT", Snapshot("spike-desktop.png")),
            (
                "HOSHI_SPIKE_EDITABLE_SNAPSHOT",
                Snapshot("spike-handles.png"),
            ),
        ],
        scales: &[],
        linux_only: None,
    },
    Smoke {
        name: "color_smoke",
        purpose: "color picker window, floating and docked",
        host: Host::Xvfb,
        screen: None,
        env: &[
            ("HOSHI_COLOR_SNAPSHOT", Snapshot("spike-colors.png")),
            ("HOSHI_DOCK_SNAPSHOT", Snapshot("spike-colors-docked.png")),
        ],
        scales: &[],
        linux_only: None,
    },
    Smoke {
        name: "color_library_smoke",
        purpose: "native named colors and palettes",
        host: Host::Xvfb,
        screen: Some("1600x1200"),
        env: &[("HOSHI_SMOKE_SNAPSHOT_DIR", OutputDir)],
        scales: &[],
        linux_only: None,
    },
    Smoke {
        name: "rectangle_editing_smoke",
        purpose: "shared shape editing controls",
        host: Host::Xvfb,
        screen: Some("1280x960"),
        env: &[(
            "HOSHI_EDITING_SNAPSHOT",
            Snapshot("spike-rectangle-editing.png"),
        )],
        scales: &[],
        linux_only: None,
    },
    Smoke {
        name: "rendering_smoke",
        purpose: "shared shape rendering",
        host: Host::Xvfb,
        screen: Some("1280x960"),
        env: &[(
            "HOSHI_RENDERING_SNAPSHOT",
            Snapshot("spike-shape-rendering.png"),
        )],
        scales: &[],
        linux_only: None,
    },
    Smoke {
        name: "bounds_smoke",
        purpose: "numeric shape bounds",
        host: Host::Xvfb,
        screen: Some("1280x960"),
        env: &[("HOSHI_SMOKE_SNAPSHOT_DIR", OutputDir)],
        scales: &[],
        linux_only: None,
    },
    Smoke {
        name: "shape_properties_smoke",
        purpose: "floating and docked shape properties",
        host: Host::Xvfb,
        screen: Some("1600x1100"),
        env: &[("HOSHI_SMOKE_SNAPSHOT_DIR", OutputDir)],
        scales: &[],
        linux_only: None,
    },
    Smoke {
        name: "documents_smoke",
        purpose: "document tabs and save prompts",
        host: Host::Xvfb,
        screen: Some("1280x960"),
        env: &[("HOSHI_DOCUMENT_SNAPSHOT_DIR", OutputDir)],
        scales: &[],
        linux_only: None,
    },
    Smoke {
        name: "large_canvas_smoke",
        purpose: "large sparse canvas and bounded viewport",
        host: Host::Xvfb,
        screen: Some("1440x1080"),
        env: &[("HOSHI_LARGE_SNAPSHOT_DIR", OutputDir)],
        scales: &[],
        linux_only: None,
    },
    Smoke {
        name: "image_settings_smoke",
        purpose: "image and canvas settings",
        host: Host::Xvfb,
        screen: None,
        env: &[("HOSHI_IMAGE_SNAPSHOT_DIR", OutputDir)],
        scales: &[],
        linux_only: None,
    },
    Smoke {
        name: "disk_history_smoke",
        purpose: "disk-backed long history",
        host: Host::Xvfb,
        screen: Some("1440x1080"),
        env: &[("HOSHI_HISTORY_SNAPSHOT_DIR", OutputDir)],
        scales: &[],
        linux_only: None,
    },
    Smoke {
        name: "panels_smoke",
        purpose: "docked panels and history",
        host: Host::Xvfb,
        screen: Some("1600x1000"),
        env: &[
            ("HOSHI_PANELS_SNAPSHOT", Snapshot("spike-panels.png")),
            ("HOSHI_HISTORY_SNAPSHOT", Snapshot("spike-history.png")),
        ],
        scales: &[],
        linux_only: None,
    },
    Smoke {
        name: "portal_smoke",
        purpose: "screen picker D-Bus portal contract",
        host: Host::DbusSession,
        screen: None,
        env: &[],
        scales: &[],
        linux_only: Some("the xdg-desktop-portal screen picker exists only on Linux"),
    },
    Smoke {
        name: "cursor_smoke",
        purpose: "eyedropper cursor at 1x and 2x",
        host: Host::Xvfb,
        screen: None,
        env: &[],
        scales: &[
            Scale {
                factor: "1",
                screen: None,
            },
            Scale {
                factor: "2",
                screen: Some("2560x1800"),
            },
        ],
        linux_only: Some("reads the cursor image through X11 (scripts/check_x11_cursor.py)"),
    },
];

/// Smoke example names found on disk that the table does not list.
pub fn unlisted<'a>(on_disk: impl IntoIterator<Item = &'a str>) -> Vec<&'a str> {
    on_disk
        .into_iter()
        .filter(|name| !SMOKES.iter().any(|smoke| smoke.name == *name))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::path::Path;

    #[test]
    fn every_table_entry_has_an_example_file() {
        let examples =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../spikes/desktop-canvas/examples");
        for smoke in SMOKES {
            assert!(
                examples.join(format!("{}.rs", smoke.name)).is_file(),
                "{} has no example file",
                smoke.name
            );
        }
    }

    #[test]
    fn names_snapshots_and_variables_are_unique() {
        let mut names = HashSet::new();
        let mut snapshots = HashSet::new();
        for smoke in SMOKES {
            assert!(names.insert(smoke.name), "duplicate {}", smoke.name);
            let mut variables = HashSet::new();
            for (variable, value) in smoke.env {
                assert!(
                    variables.insert(*variable),
                    "{} sets {variable} twice",
                    smoke.name
                );
                if let Snapshot(file) = value {
                    assert!(snapshots.insert(*file), "two smokes write {file}");
                }
            }
        }
    }

    #[test]
    fn unlisted_reports_only_missing_names() {
        assert_eq!(unlisted(["desktop_smoke", "new_smoke"]), ["new_smoke"]);
    }
}
