//! Repository task runner. `cargo xtask test <suite>` runs the same checks
//! locally and in CI; suites are documented in docs/TESTING.md.

mod run;
mod smoke;

use run::{Error, Step, Workspace};
use std::process::ExitCode;

const USAGE: &str = "\
usage: cargo xtask <command>

commands:
  test lint            rustfmt check + clippy -D warnings
  test unit            workspace unit, integration and doc tests
  test scripts         Python tests for scripts/
  test smoke [NAME..]  native desktop smoke examples (all, or the named ones)
  test full            lint + unit + scripts + smoke (what CI runs)
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
        ["test", "unit"] => unit(&workspace),
        ["test", "scripts"] => scripts(&workspace),
        ["test", "smoke", names @ ..] => smoke(&workspace, names),
        ["test", "full"] => {
            lint(&workspace)?;
            unit(&workspace)?;
            scripts(&workspace)?;
            smoke(&workspace, &[])
        }
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

fn unit(workspace: &Workspace) -> Result<(), Error> {
    workspace.run(cargo(&["test", "--workspace", "--locked"]))
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
    if !cfg!(target_os = "linux") {
        println!("xtask: native smoke runs on Linux only for now (TEST-3, #66); built, not run");
        return Ok(());
    }
    let output = workspace.smoke_output_dir()?;
    let mut failures = Vec::new();
    for entry in selected {
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
