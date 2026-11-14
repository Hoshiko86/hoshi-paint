//! Workspace paths and child-process steps with logging and timeouts.

use crate::smoke::{EnvValue, Host, Smoke};
use std::ffi::OsString;
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::time::{Duration, Instant};

/// A native smoke program that runs longer than this is hung.
const SMOKE_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Debug)]
pub enum Error {
    Usage(String),
    Io {
        context: String,
        source: std::io::Error,
    },
    Spawn {
        program: String,
        source: std::io::Error,
    },
    Failed {
        step: String,
        status: ExitStatus,
    },
    TimedOut {
        step: String,
        after: Duration,
    },
    UnlistedSmoke(Vec<String>),
    SmokeFailed(Vec<&'static str>),
    BelowCoverageFloor(Vec<&'static str>),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(message) => write!(f, "{message}"),
            Self::Io { context, source } => write!(f, "{context}: {source}"),
            Self::Spawn { program, source } => {
                write!(f, "cannot start `{program}`: {source}")?;
                if matches!(program.as_str(), "xvfb-run" | "dbus-run-session") {
                    write!(f, " (install xvfb and dbus; see docs/TESTING.md)")?;
                }
                Ok(())
            }
            Self::Failed { step, status } => write!(f, "`{step}` failed ({status})"),
            Self::TimedOut { step, after } => {
                write!(f, "`{step}` killed after {}s", after.as_secs())
            }
            Self::UnlistedSmoke(names) => write!(
                f,
                "smoke examples missing from xtask/src/smoke.rs: {}",
                names.join(", ")
            ),
            Self::SmokeFailed(names) => write!(f, "smoke failures: {}", names.join(", ")),
            Self::BelowCoverageFloor(names) => write!(
                f,
                "line coverage below the floor in xtask/src/coverage.rs: {}",
                names.join(", ")
            ),
        }
    }
}

impl std::error::Error for Error {}

/// One child process: program, arguments, extra environment and an optional
/// timeout. Runs from the workspace root.
pub struct Step {
    program: OsString,
    args: Vec<OsString>,
    env: Vec<(String, OsString)>,
    timeout: Option<Duration>,
}

impl Step {
    pub fn new(program: impl Into<OsString>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            env: Vec::new(),
            timeout: None,
        }
    }

    pub fn args(mut self, args: &[&str]) -> Self {
        self.args.extend(args.iter().map(OsString::from));
        self
    }

    pub fn arg(mut self, arg: impl Into<OsString>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn env(mut self, name: &str, value: impl Into<OsString>) -> Self {
        self.env.push((name.to_owned(), value.into()));
        self
    }

    fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    fn describe(&self) -> String {
        let mut text = String::new();
        for (name, value) in &self.env {
            text.push_str(&format!("{name}={} ", value.to_string_lossy()));
        }
        text.push_str(&self.program.to_string_lossy());
        for arg in &self.args {
            text.push(' ');
            text.push_str(&arg.to_string_lossy());
        }
        text
    }
}

pub struct Workspace {
    root: PathBuf,
    target: PathBuf,
}

impl Workspace {
    pub fn locate() -> Result<Self, Error> {
        // xtask/ sits directly under the workspace root.
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let root = manifest.parent().unwrap_or(manifest).to_path_buf();
        let target = match std::env::var_os("CARGO_TARGET_DIR") {
            Some(dir) => root.join(dir),
            None => root.join("target"),
        };
        Ok(Self { root, target })
    }

    /// Runs `step` quietly and returns its standard output, or `None` if it
    /// cannot start or fails. For probes such as `git` and `cargo nextest`.
    pub fn capture(&self, step: &Step) -> Option<String> {
        let output = Command::new(&step.program)
            .args(&step.args)
            .current_dir(&self.root)
            .stderr(std::process::Stdio::null())
            .output()
            .ok()?;
        if output.status.success() {
            String::from_utf8(output.stdout).ok()
        } else {
            None
        }
    }

    pub fn run(&self, step: Step) -> Result<(), Error> {
        let description = step.describe();
        println!("\n==> {description}");
        let started = Instant::now();
        let mut command = Command::new(&step.program);
        command.args(&step.args).current_dir(&self.root);
        for (name, value) in &step.env {
            command.env(name, value);
        }
        let mut child = command.spawn().map_err(|source| Error::Spawn {
            program: step.program.to_string_lossy().into_owned(),
            source,
        })?;
        let status = match step.timeout {
            None => child.wait(),
            Some(limit) => loop {
                if let Some(status) = child.try_wait().map_err(|source| Error::Io {
                    context: format!("waiting for `{description}`"),
                    source,
                })? {
                    break Ok(status);
                }
                if started.elapsed() > limit {
                    // Best effort: the step has already failed.
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(Error::TimedOut {
                        step: description,
                        after: limit,
                    });
                }
                std::thread::sleep(Duration::from_millis(100));
            },
        }
        .map_err(|source| Error::Io {
            context: format!("waiting for `{description}`"),
            source,
        })?;
        println!("<== {:.1}s", started.elapsed().as_secs_f64());
        if status.success() {
            Ok(())
        } else {
            Err(Error::Failed {
                step: description,
                status,
            })
        }
    }

    /// `examples/*_smoke.rs` files the smoke table does not list.
    pub fn unlisted_smoke_examples(&self) -> Result<Vec<String>, Error> {
        let dir = self.root.join("spikes/desktop-canvas/examples");
        let entries = std::fs::read_dir(&dir).map_err(|source| Error::Io {
            context: format!("reading {}", dir.display()),
            source,
        })?;
        let mut names = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|source| Error::Io {
                context: format!("reading {}", dir.display()),
                source,
            })?;
            let file = entry.file_name();
            if let Some(name) = file.to_str().and_then(|file| file.strip_suffix(".rs"))
                && name.ends_with("_smoke")
            {
                names.push(name.to_owned());
            }
        }
        names.sort();
        Ok(crate::smoke::unlisted(names.iter().map(String::as_str))
            .into_iter()
            .map(str::to_owned)
            .collect())
    }

    /// `target/smoke/`, created if needed. CI uploads its contents.
    pub fn smoke_output_dir(&self) -> Result<PathBuf, Error> {
        let dir = self.target.join("smoke");
        std::fs::create_dir_all(&dir).map_err(|source| Error::Io {
            context: format!("creating {}", dir.display()),
            source,
        })?;
        Ok(dir)
    }

    /// `target/coverage/`, created if needed. CI uploads its contents.
    pub fn coverage_output_dir(&self) -> Result<PathBuf, Error> {
        let dir = self.target.join("coverage");
        std::fs::create_dir_all(&dir).map_err(|source| Error::Io {
            context: format!("creating {}", dir.display()),
            source,
        })?;
        Ok(dir)
    }

    /// The process steps for one smoke entry: one per display scale.
    pub fn smoke_steps(&self, smoke: &Smoke, output: &Path) -> Vec<Step> {
        // Every smoke is a subcommand of one example binary (examples/smoke.rs).
        let exe = self
            .target
            .join("debug/examples")
            .join(format!("smoke{}", std::env::consts::EXE_SUFFIX));
        let runs: Vec<(Option<&str>, Option<&str>)> = if smoke.scales.is_empty() {
            vec![(None, smoke.screen)]
        } else {
            smoke
                .scales
                .iter()
                .map(|scale| (Some(scale.factor), scale.screen))
                .collect()
        };
        runs.into_iter()
            .map(|(scale, screen)| {
                // Linux CI has no display or session bus, so the host provides
                // one; Windows and macOS runners already have a desktop session.
                let mut step = match (smoke.host, cfg!(target_os = "linux")) {
                    (Host::Xvfb, true) => {
                        let mut step = Step::new("xvfb-run").arg("-a");
                        if let Some(screen) = screen {
                            step = step.arg("-s").arg(format!("-screen 0 {screen}x24"));
                        }
                        step.arg(&exe).arg(smoke.name)
                    }
                    (Host::DbusSession, true) => Step::new("dbus-run-session")
                        .arg("--")
                        .arg(&exe)
                        .arg(smoke.name),
                    (_, false) => Step::new(&exe).arg(smoke.name),
                };
                if smoke.host == Host::Xvfb {
                    step = step.env("SLINT_BACKEND", "winit-software");
                }
                step = step.timeout(SMOKE_TIMEOUT);
                if let Some(scale) = scale {
                    step = step.env("SLINT_SCALE_FACTOR", scale);
                }
                for (name, value) in smoke.env {
                    step = match value {
                        EnvValue::Snapshot(file) => step.env(name, output.join(file)),
                        EnvValue::OutputDir => step.env(name, output),
                    };
                }
                step
            })
            .collect()
    }
}
