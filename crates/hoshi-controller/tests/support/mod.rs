//! Shared helpers for integration tests (#82).

use std::process::Command;

/// Set in the child process to the name of the test it must run.
const ISOLATED_TEST: &str = "HOSHI_ISOLATED_TEST";

/// Runs `body` in a fresh process, for tests that exhaust or inspect the
/// process-wide memory budget (`hoshi_core::memory_budget`). `cargo test` runs
/// tests as threads of one process, so they would otherwise interfere.
///
/// Call it as the whole test body, with `test_name` equal to the test's own
/// function name. The parent re-runs this test binary with `--exact
/// test_name` and asserts the child ran exactly one test, so a renamed test
/// or a typo fails instead of passing without running `body`.
pub fn run_isolated(test_name: &str, body: impl FnOnce()) {
    if std::env::var(ISOLATED_TEST).as_deref() == Ok(test_name) {
        body();
        return;
    }
    let exe = std::env::current_exe().expect("test binary path");
    let output = Command::new(exe)
        .args(["--exact", test_name, "--test-threads=1"])
        .env(ISOLATED_TEST, test_name)
        .output()
        .expect("spawn isolated test");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "isolated test `{test_name}` failed:\n{stdout}\n{stderr}"
    );
    assert!(
        stdout.lines().any(|line| line.trim() == "running 1 test"),
        "isolated run of `{test_name}` ran no test; is the name wrong?\n{stdout}"
    );
}
