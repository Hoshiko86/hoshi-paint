# AGENTS.md — rules for coding agents (Codex)

You are the implementer. Architecture is decided elsewhere (see `CLAUDE.md`,
`docs/ARCHITECTURE.md`). Do the task you were given, in the files you were
given, and stop.

## 0. Scope (most important rule)

- Touch **only the files named in the task**. `Cargo.toml`/`Cargo.lock` edits
  are allowed only when the task says to add a dependency.
- If the task can't be done without editing another file, **stop and report**
  which file and why. Do not "fix it while you're there".
- No drive-by refactors, renames, reformatting of untouched code, or new
  public APIs the task did not ask for.
- Keep public signatures stable unless the task explicitly changes them.

## 1. Workspace layout and ownership

Dependencies point inward only: `spikes/desktop-canvas` → `hoshi-controller` → `hoshi-core`.

| Crate / path | Owns | Must NOT depend on |
|---|---|---|
| `crates/hoshi-core` | Document, layers, pixels (`pixels.rs`), geometry, shape rasterizing (`rendering/`), resampling, history + disk store, `.hoshi` format (`project.rs`), memory budget | Slint, controller, OS/dialog/filesystem-picker APIs, UI text |
| `crates/hoshi-controller` | Tools, gestures, viewport, logical-coordinate input, color parsing, palettes, named colors, preview image | Slint, platform crates, `std::fs` dialogs |
| `spikes/desktop-canvas/src` | Slint adapter: `lib.rs` wiring, `workspace.rs` tabs, `documents.rs` open/save flow, `files.rs` codecs + atomic save, `colors.rs`/`color_library.rs` panels, `screen_pick.rs` OS pickers, `localization.rs` | — (outermost layer) |
| `spikes/desktop-canvas/ui/*.slint` | Presentation only. Colors/sizes come from `theme.slint`; text from `localization.slint` | Editing policy |

- Editing policy lives in core/controller, never in Slint callbacks.
- Platform `cfg(target_os)` code only inside `screen_pick.rs` (or a new narrow
  adapter module the task names).

## 2. Rust guardrails

**Errors**
- Library crates (`hoshi-core`, `hoshi-controller`): typed error enums with
  `thiserror`. Variants carry data (paths, sizes, source errors via `#[source]`/
  `#[from]`), not pre-formatted `String`s.
- The spike crate may use `anyhow` only in `main.rs` and `examples/`.
- **No new `Result<_, String>`** and no `.map_err(|e| e.to_string())`. Convert
  to text only at the UI boundary (status bar / dialog).
- No user-facing English inside core/controller errors in new code; that copy
  belongs to the adapter/localization layer.

**Panics**
- No `.unwrap()`, `.expect()`, `panic!`, `unreachable!`, `todo!` or raw slice
  indexing that can go out of bounds in non-test code. Use `?`, `let … else`,
  `.get()`, or restructure so the invariant is held by a type.
- If you believe an invariant truly can't fail, encode it (pass the value in,
  use an enum variant that carries it) instead of `.expect()`.
- Tests (`#[cfg(test)]`, `tests/`, `examples/`) may unwrap.

**Ownership**
- No `.clone()` to silence the borrow checker. Borrow (`&T`, `&mut T`), split
  borrows, `std::mem::take`/`replace`, or return references. A clone is OK only
  for `Copy`-cheap types, `Rc`/`slint::Weak` handles captured by callbacks, or
  when an owned value is genuinely required — say so in a comment if non-obvious.
- **No `Arc<Mutex<T>>` / `Arc<RwLock<T>>`** for new state. The app is
  single-threaded UI: use owned state + `&mut`. For background work use
  `std::sync::mpsc` channels or `slint::invoke_from_event_loop`.
- Don't add new `Rc<RefCell<…>>`. The existing `SharedController`
  (`Rc<RefCell<Workspace>>`) is legacy: keep `borrow_mut()` scopes short, never
  hold a borrow across a call that may re-enter a Slint callback.
- No `unsafe` outside `screen_pick.rs`. Every `unsafe` block gets a `// SAFETY:` comment.

**Types**
- Make invalid states unrepresentable: enums over `i32`/`bool` mode flags,
  newtypes for coordinates/dimensions, private fields with validating constructors.
- Keep `hoshi-core` deterministic: no clocks, threads, or global mutable state
  added in new code.

**Style**
- Functions ≤ ~60 lines, files ≤ ~600 lines for new code. Prefer a new focused
  module over growing `lib.rs`.
- Doc-comment every new `pub` item with its invariant/contract (one line is fine).
- Tests: behavioral, at public API boundaries. Don't assert pixel coordinates of
  UI layout or literal theme colors.

## 3. Verification (required before you finish)

Run from the repo root, all must pass:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

To save time while iterating, test only the crate you touched
(`cargo test -p hoshi-core`, `-p hoshi-controller`, `-p hoshi-canvas-spike`),
then run the full set once at the end. If adding a dependency, drop `--locked`
for one `cargo build`, then confirm the `--locked` commands pass.

Do not `#[allow(...)]` a clippy lint, `#[ignore]` a test, or delete a test to
get green. If something won't pass, stop and report the exact error.

## 4. Finish report (keep it short)

1. Files changed (should match the task).
2. One-line summary per change.
3. Output tail of the three verification commands.
4. Anything you could not do and why.
