//! Repository task runner. `cargo xtask test <suite>` runs the same checks
//! locally and in CI; suites are documented in docs/TESTING.md.

mod changed;
mod coverage;
mod run;
mod smoke;

use run::{Error, Step, Workspace};
use std::process::ExitCode;

const USAGE: &str = "\
usage: cargo xtask <command>

commands:
  test lint            rustfmt check + clippy -D warnings
  test unit            workspace unit, integration and doc tests
                       (cargo-nextest when installed, else cargo test)
  test scripts         Python tests for scripts/
  test smoke [NAME..]  native desktop smoke examples (all, or the named ones)
  test full            lint + unit + scripts + smoke (what CI runs)
  test changed [BASE]  lint, then only what the diff since BASE needs
                       (default origin/main; see docs/TESTING.md)
  test coverage        line coverage of the domain crates with floors
                       (needs cargo-llvm-cov and llvm-tools-preview)
  list                 print the smoke table";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match dispatch(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("xtask: {error}");
            if matches!(error, Error::Usage(_)) {
                eprintln!("{USAGE}");
            }
            ExitCode::FAILURE
        }
    }
}

fn dispatch(args: &[String]) -> Result<(), Error> {
    let workspace = Workspace::locate()?;
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    match words.as_slice() {
        ["test", "lint"] => lint(&workspace),
        ["test", "unit"] => unit(&workspace, &[]),
        ["test", "scripts"] => scripts(&workspace),
        ["test", "smoke", names @ ..] => smoke(&workspace, names),
        ["test", "full"] => {
            lint(&workspace)?;
            unit(&workspace, &[])?;
            scripts(&workspace)?;
            smoke(&workspace, &[])
        }
        ["test", "coverage"] => coverage(&workspace),
        ["test", "changed"] => changed(&workspace, "origin/main"),
        ["test", "changed", base] => changed(&workspace, base),
        ["list"] => {
            for entry in smoke::SMOKES {
                println!("{:<26} {}", entry.name, entry.purpose);
            }
            Ok(())
        }
        ["help" | "--help" | "-h"] => {
            println!("{USAGE}");
            Ok(())
        }
        _ => Err(Error::Usage(format!(
            "unknown command `{}`",
            words.join(" ")
        ))),
    }
}

fn cargo(args: &[&str]) -> Step {
    Step::new("cargo").args(args)
}

fn lint(workspace: &Workspace) -> Result<(), Error> {
    workspace.run(cargo(&["fmt", "--all", "--check"]))?;
    workspace.run(cargo(&[
        "clippy",
        "--workspace",
        "--all-targets",
        "--locked",
        "--",
        "-D",
        "warnings",
    ]))
}

/// Tests of `packages`, or of the whole workspace when empty. nextest runs
/// each test in its own process (the core memory budget is a process-wide
/// static) and writes JUnit under CI; it does not run doc tests, so those run
/// separately.
fn unit(workspace: &Workspace, packages: &[&str]) -> Result<(), Error> {
    let mut scope: Vec<&str> = Vec::new();
    if packages.is_empty() {
        scope.push("--workspace");
    }
    for package in packages {
        scope.extend(["--package", package]);
    }
    let nextest = workspace
        .capture(&cargo(&["nextest", "--version"]))
        .is_some();
    if !nextest {
        println!("xtask: cargo-nextest not installed; using cargo test");
        return workspace.run(cargo(&["test", "--locked"]).args(&scope));
    }
    let profile = if std::env::var_os("CI").is_some() {
        "ci"
    } else {
        "default"
    };
    workspace.run(cargo(&["nextest", "run", "--locked", "--profile", profile]).args(&scope))?;
    let libraries: Vec<&str> = if packages.is_empty() {
        changed::PACKAGES
            .iter()
            .map(|package| package.name)
            .collect()
    } else {
        packages.to_vec()
    };
    let mut doc = cargo(&["test", "--doc", "--locked"]);
    for package in libraries {
        doc = doc.args(&["--package", package]);
    }
    workspace.run(doc)
}

/// Instrumented test run of the domain crates; writes `target/coverage/`
/// (lcov.info and html/) and fails if a crate drops below its floor.
fn coverage(workspace: &Workspace) -> Result<(), Error> {
    if workspace
        .capture(&cargo(&["llvm-cov", "--version"]))
        .is_none()
    {
        return Err(Error::Usage(format!(
            "cargo-llvm-cov is not installed: `rustup component add llvm-tools-preview` \
             and `cargo install --locked cargo-llvm-cov --version {}`",
            coverage::LLVM_COV_VERSION
        )));
    }
    let mut measure = cargo(&["llvm-cov", "--no-report", "--locked"]);
    for floor in coverage::FLOORS {
        measure = measure.args(&["--package", floor.package]);
    }
    workspace.run(measure)?;
    let out = workspace.coverage_output_dir()?;
    workspace.run(
        cargo(&["llvm-cov", "report", "--lcov", "--output-path"]).arg(out.join("lcov.info")),
    )?;
    workspace.run(cargo(&["llvm-cov", "report", "--html", "--output-dir"]).arg(&out))?;
    let mut below = Vec::new();
    for floor in coverage::FLOORS {
        let gate = cargo(&[
            "llvm-cov",
            "report",
            "--summary-only",
            "--ignore-filename-regex",
        ])
        .arg(coverage::ignore_all_but(floor))
        .args(&["--fail-under-lines"])
        .arg(floor.lines.to_string());
        if let Err(error) = workspace.run(gate) {
            eprintln!("xtask: {error}");
            below.push(floor.package);
        }
    }
    println!("xtask: coverage report in {}", out.display());
    if below.is_empty() {
        Ok(())
    } else {
        Err(Error::BelowCoverageFloor(below))
    }
}

fn changed(workspace: &Workspace, base: &str) -> Result<(), Error> {
    let merge_base = workspace
        .capture(&Step::new("git").args(&["merge-base", base, "HEAD"]))
        .ok_or_else(|| {
            Error::Usage(format!(
                "no merge-base with `{base}`; fetch it or pass another base"
            ))
        })?;
    let merge_base = merge_base.trim();
    let tracked = workspace
        .capture(&Step::new("git").args(&["diff", "--name-only", merge_base]))
        .unwrap_or_default();
    let untracked = workspace
        .capture(&Step::new("git").args(&["ls-files", "--others", "--exclude-standard"]))
        .unwrap_or_default();
    let selection = changed::select(tracked.lines().chain(untracked.lines()));
    println!("xtask: changes since {base} ({merge_base}) need {selection:?}");
    lint(workspace)?;
    if selection.everything {
        unit(workspace, &[])?;
        scripts(workspace)?;
        return smoke(workspace, &[]);
    }
    if selection.is_empty() {
        println!("xtask: only documentation changed; lint was enough");
        return Ok(());
    }
    if !selection.packages.is_empty() {
        let packages: Vec<&str> = selection.packages.iter().copied().collect();
        unit(workspace, &packages)?;
    }
    if selection.scripts {
        scripts(workspace)?;
    }
    if selection.smoke {
        smoke(workspace, &[])?;
    }
    Ok(())
}

fn scripts(workspace: &Workspace) -> Result<(), Error> {
    workspace.run(Step::new("python3").args(&["-m", "unittest", "discover", "-s", "scripts", "-v"]))
}

fn smoke(workspace: &Workspace, names: &[&str]) -> Result<(), Error> {
    let unlisted = workspace.unlisted_smoke_examples()?;
    if !unlisted.is_empty() {
        return Err(Error::UnlistedSmoke(unlisted));
    }
    let selected: Vec<&smoke::Smoke> = if names.is_empty() {
        smoke::SMOKES.iter().collect()
    } else {
        names
            .iter()
            .map(|name| {
                smoke::SMOKES
                    .iter()
                    .find(|entry| entry.name == *name)
                    .ok_or_else(|| {
                        Error::Usage(format!("no smoke named `{name}`; see `cargo xtask list`"))
                    })
            })
            .collect::<Result<_, _>>()?
    };
    // Every OS builds the desktop binary and examples (CI uploads the binary).
    workspace.run(cargo(&[
        "build",
        "--package",
        "hoshi-canvas-spike",
        "--bins",
        "--examples",
        "--locked",
    ]))?;
    let output = workspace.smoke_output_dir()?;
    let mut failures = Vec::new();
    for entry in selected {
        if let (Some(reason), false) = (entry.linux_only, cfg!(target_os = "linux")) {
            println!("\n--- {} not run on this OS: {reason}", entry.name);
            continue;
        }
        for step in workspace.smoke_steps(entry, &output) {
            if let Err(error) = workspace.run(step) {
                eprintln!("xtask: {error}");
                failures.push(entry.name);
            }
        }
    }
    println!("xtask: snapshots in {}", output.display());
    if failures.is_empty() {
        Ok(())
    } else {
        Err(Error::SmokeFailed(failures))
    }
}
