# Test strategy

One runner, `cargo xtask`, drives every check, locally and in CI. CI only calls
`cargo xtask test <suite>`. It never lists individual tests, so a test added in
the right place is picked up without editing a workflow.

```sh
cargo xtask test full      # everything CI runs on Linux
cargo xtask test unit      # fast inner loop
cargo xtask test smoke panels_smoke cursor_smoke   # selected native smokes
cargo xtask list           # the smoke table
```

## Suites

| Suite | Runs | When |
| --- | --- | --- |
| `lint` | `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings` | every PR, all OSes |
| `unit` | every crate's unit, integration and doc tests; cargo-nextest when installed (one process per test, JUnit under CI), else `cargo test` | every PR, all OSes |
| `scripts` | `python3 -m unittest discover -s scripts` | every PR |
| `smoke [NAME..]` | builds the desktop binary and examples, then runs every native smoke in [`xtask/src/smoke.rs`](../xtask/src/smoke.rs), or only the named ones | every PR, all OSes |
| `full` | `lint` + `unit` + `scripts` + `smoke` | before pushing |
| `changed [BASE]` | `lint`, then only what the diff since `BASE` (default `origin/main`) needs | while iterating |
| `coverage` | instrumented tests of `hoshi-core` and `hoshi-controller`; fails below a floor | every PR (Linux) |

### Selective runs (`changed`)

[`xtask/src/changed.rs`](../xtask/src/changed.rs) maps each changed path
(committed since the merge-base, uncommitted, or untracked) to work:

| Changed | Runs |
| --- | --- |
| `crates/hoshi-core/` | core, controller and spike tests, plus smoke |
| `crates/hoshi-controller/` | controller and spike tests, plus smoke |
| `spikes/desktop-canvas/` | spike tests, plus smoke |
| `scripts/` | `scripts` (`check_x11_cursor.py` also runs smoke) |
| `docs/`, top-level `*.md`, licences | nothing beyond `lint` |
| manifests, `Cargo.lock`, toolchain, `.cargo/`, `.config/`, `.github/workflows/`, `xtask/`, anything unknown | everything |

The package dependency table is checked against each `Cargo.toml` by a unit
test, so a new path dependency cannot be skipped silently. CI always runs the
full suites; `changed` is a local shortcut.

### Platforms

Native smokes run under Xvfb (and a private D-Bus session for the portal) on
Linux, and directly in the runner's desktop session on Windows and macOS.
Linux-only smokes say why in the table (`linux_only`) and print that reason
when skipped:

- `portal_smoke`: the xdg-desktop-portal screen picker exists only on Linux.
- `cursor_smoke`: reads the cursor image through X11.

## Test layers

Put each test in the lowest layer that can observe the behavior.

| Layer | Where | Proves | Tools |
| --- | --- | --- | --- |
| Core unit | `crates/hoshi-core/src/**` `#[cfg(test)]` | private invariants: tiles, masks, memory budget, codecs | `cargo test` |
| Core API | `crates/hoshi-core/tests/*.rs` | document behavior through the public API: atomic edits, one history entry per action, preview isolation, file round-trips | `cargo test` |
| Controller | `crates/hoshi-controller/tests/*.rs` | tool and gesture policy, viewport math, colors, without UI | `cargo test` |
| Adapter interaction | `spikes/desktop-canvas/tests/*.rs` | real Slint UI on the headless software renderer: synthetic pointer, key and focus events, rendered pixels, focus and modal rules | `i-slint-backend-testing` |
| Native smoke | `spikes/desktop-canvas/examples/*_smoke.rs` | the real winit window, OS event loop, D-Bus portal and cursor; saves snapshots | `cargo xtask test smoke` (Xvfb on Linux) |
| Reference | fixtures under `crates/*/tests/fixtures/` | output matches a pinned reference (blend modes vs Paint.NET 5.2, [BLEND_REFERENCE.md](BLEND_REFERENCE.md)) | `cargo test` |

Rules:

- **Behavior, not implementation.** Assert what a user or caller can observe:
  pixels, history entries, returned errors, emitted models. Don't assert
  private field values or exact intermediate coordinates.
- **Every bug fix starts with a failing test.** Record the failure message in
  the PR.
- **Every edit command covers four things:** the success result, one history
  entry, exact undo/redo, and a failure that leaves the document unchanged.
- **Previews:** assert that committed pixels are unchanged before commit, and
  that the committed result equals the last preview.
- **Determinism:** no sleeps for correctness, no wall-clock or random input
  without a fixed seed, and no network. Native smokes may wait on Slint timers
  to let layout settle.
- **Never** `#[ignore]` or delete a test to get green (AGENTS.md).

## Adding a native smoke

1. Write `spikes/desktop-canvas/examples/<name>_smoke.rs`. It must exit by
   itself, panic on failure, and read snapshot paths from environment
   variables.
2. Add one entry to `SMOKES` in `xtask/src/smoke.rs` with its purpose, screen
   size, snapshot variables and display scales. `cargo xtask test smoke`
   refuses to run while an example is missing from the table.
3. Snapshots land in `target/smoke/`; CI uploads that directory.

Each smoke is killed after 300 s. The suite runs every selected smoke and then
lists all failures, rather than stopping at the first.

## Coverage

`cargo xtask test coverage` instruments the tests of the domain crates with
cargo-llvm-cov and writes `target/coverage/lcov.info` and
`target/coverage/html/`. CI uploads both and prints the summary on the run
page. Each crate has a line-coverage floor in
[`xtask/src/coverage.rs`](../xtask/src/coverage.rs), set to the measured
baseline rounded down. Raise a floor when coverage rises. Never lower one to
get green: add tests instead.

Baseline (2026-11-12, measured by the crates' own tests):

| Crate | Lines | Floor | Weakest files |
| --- | --- | --- | --- |
| `hoshi-core` | 95.4% (3,704) | 95% | `lib.rs` 90.6%, `history.rs` 91.0% |
| `hoshi-controller` | 88.2% (1,418) | 88% | `rendering.rs` 43.2%, `editing.rs` 81.1%, `lib.rs` 86.9% |

The desktop spike is not instrumented. Its code is a disposable adapter, and
instrumenting it would rebuild Slint.

## Local prerequisites

- Rust from `rust-toolchain.toml` (installed by `rustup` on first use).
- Optional: `cargo install --locked cargo-nextest --version 0.9.151`.
- Coverage: `rustup component add llvm-tools-preview` and
  `cargo install --locked cargo-llvm-cov --version 0.9.1`.
- Linux smoke: `xvfb`, `dbus`, `libxkbcommon-x11-0`, `libxfixes3`,
  `libfontconfig-dev` (Debian/Ubuntu package names), plus `python3` for the
  cursor check and `scripts` suite.
