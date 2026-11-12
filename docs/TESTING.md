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
| `unit` | `cargo test --workspace --locked`: unit, integration and doc tests of every crate | every PR, all OSes |
| `scripts` | `python3 -m unittest discover -s scripts` | every PR |
| `smoke [NAME..]` | builds the desktop binary and examples, then runs every native smoke in [`xtask/src/smoke.rs`](../xtask/src/smoke.rs), or only the named ones | every PR (Linux; Windows/macOS in TEST-3) |
| `full` | `lint` + `unit` + `scripts` + `smoke` | before pushing; CI on Linux |

Planned in [#63](https://github.com/Hoshiko86/hoshi-paint/issues/63):
`changed` (only crates affected by the diff and their dependents; TEST-3) and
`coverage` (cargo-llvm-cov with a floor gate; TEST-4).

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

## Local prerequisites

- Rust from `rust-toolchain.toml` (installed by `rustup` on first use).
- Linux smoke: `xvfb`, `dbus`, `libxkbcommon-x11-0`, `libxfixes3`,
  `libfontconfig-dev` (Debian/Ubuntu package names), plus `python3` for the
  cursor check and `scripts` suite.
