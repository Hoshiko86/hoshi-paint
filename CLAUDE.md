# CLAUDE.md — architect notes

Role split: **Claude** = architecture, type design, task breakdown, PR review.
**Codex** = implementation in a cloud sandbox, governed by `AGENTS.md`.
The owner works from a phone: keep replies short and scannable.

Deeper references: `docs/ARCHITECTURE.md` (quality gate — binding),
`.github/copilot-instructions.md`, `.github/pull_request_template.md`.

## System shape

```
spikes/desktop-canvas (Slint 1.18.1 adapter, disposable)
   └─> hoshi-controller (tools, gestures, viewport, colors; no UI deps)
          └─> hoshi-core (document, sparse COW tiles, history-on-disk, .hoshi I/O)
```

- Toolchain pinned 1.99.1, edition 2024. `--locked` everywhere. CI runs on
  Linux/Windows/macOS plus xvfb smoke examples.
- Core: sparse 128×128 `Arc` copy-on-write tiles, ≤262,144 px/axis, global
  memory budget (`memory.rs`, static atomics), disk-backed compressed undo.
- Shapes: draw → adjust → commit; one history entry per commit.
- Spike state: `SharedController = Rc<RefCell<Workspace>>`; `Workspace` holds
  tabs, each with its own `Controller`.

## Decisions (keep consistent)

1. Errors: `thiserror` enums in libraries; text produced at the adapter/locale
   boundary. Migrating away from `Result<_, String>` file by file.
2. No `unwrap/expect` in non-test code; encode invariants in types instead.
3. No `Arc<Mutex>`; single-threaded UI with `&mut`; channels for background work.
4. Tasks for Codex: **1–2 files each**, one concern, behavior-preserving unless
   stated, with explicit "do not touch" lists and a stop condition.

## Known debt (audit 2026-11)

1. **Stringly errors in the adapter** — ~48 `Result<_, String>` in `files.rs`/
   `workspace.rs`; `.map_err(|e| e.to_string())` loses kind. Core `Error` is a
   fieldless enum whose `Display` holds UI English (conflicts with i18n boundary);
   `ProjectError::Io(String)` drops the io source.
2. **`Rc<RefCell<Workspace>>` + `Deref<Target = Controller>`** — every callback
   reaches through the workspace to the active controller; with no tab open,
   mutations silently hit a hidden `empty` controller. RefCell re-entrancy risk.
3. **Invariants by `.expect()` / primitives** — `editing.rs` `.expect("active
   shape")` ×5; `HandlePoint.kind: i32` with magic 8/9; hub files too big
   (spike `lib.rs` 1153 lines, core `lib.rs` 723 with `Document` 41 methods).

## Task queue

Status lives in GitHub issues (`codex-task` label); this list is the plan.

- [x] T1 #35 controller `editing.rs` expects (landed in #33).
- [ ] T2 #36 spike `files.rs`: `FileError` (thiserror) replaces `String`; 4 bridge
      sites in `workspace.rs` stay `to_string()` until `WorkspaceError` (T4).
- [ ] T3 #37 core `history.rs`: remove `.expect`/`records[0]` in `checkpoint`.
- [ ] #39 localization `Localizer` panics → debug_assert + fallback.
- [ ] #41 Ctrl+Shift+Z / Caps Lock undo-redo shortcut bug.
- [ ] DRAW-1 (#6) → #42 core tip/mask → #43 Document stroke lifecycle →
      #44 controller tools → #47 desktop rail. #45 i18n keys, #46 icons,
      #48 eyedropper spec/tests are independent.
- Decisions (owner): **Paint.NET 5.2 parity** is the goal (`docs/PARITY.md`,
  ADRs in `docs/adr/`); iPad/iOS after desktop parity. Color Picker tool samples
  the canvas (parity) + separate Screen Picker action (Hoshi extra). Lost
  pointer capture / focus loss **commits** an active stroke. Brush size is one
  shared toolbar value. Ctrl+Shift+Z = Layer Rotate/Zoom (not Redo).
  Esc commits live edits (Paint.NET). Fonts: all system fonts + a bundled
  curated open-font pack (OFL/Apache/UFL). License is not fixed to GPL-3.0;
  revisit if a dependency needs it. Open: blend reference renders (owner).
- Workflow: Codex gets "Implement issue #N"; fixes via `@codex` on the PR.
- Next: T4 `WorkspaceError`; `HandleKind` enum (replaces `i32` 8/9);
  replace `Workspace`'s `empty` controller with a `Drawing` settings value,
  then drop `Deref` caller file by file (note: `empty` currently carries
  drawing settings while no tab is open, so a plain no-op is a regression);
  move core error copy to adapter; split spike `lib.rs` (slint! macro → `.slint`
  file, `refresh` → `view_model.rs`); split `Color`/`Layer` out of core `lib.rs`.

## PR review checklist

Blocking:
- [ ] Only the task's files changed (plus manifest if a dep was requested).
- [ ] Dependency direction intact; no Slint/OS types in core/controller.
- [ ] No new `unwrap/expect/panic/unreachable` outside tests.
- [ ] No new `Result<_, String>`, `.to_string()` error flattening, or UI text in core.
- [ ] No lazy `.clone()` of non-`Copy` data; no `Arc<Mutex>`; no new `Rc<RefCell>`.
- [ ] No `RefCell` borrow held across a call that can re-enter a callback.
- [ ] Document atomicity: failure leaves prior state; one history entry per edit.
- [ ] Behavior preserved (tests unchanged or strengthened, none ignored/deleted).
- [ ] fmt / clippy `-D warnings` / tests green on CI (all 3 OSes).

Follow-up (non-blocking): naming, fn length, missing doc comments, test gaps.

Review output: findings only, `file:line`, scenario → fix, label blocking/follow-up.
