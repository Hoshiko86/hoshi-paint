# CLAUDE.md — architect notes

Role split: **Claude** = architecture, type design, task breakdown, PR review.
**Codex** = implementation in a cloud sandbox, governed by `AGENTS.md`.
The owner works from a phone: keep replies short and scannable.

Deeper references: `docs/ARCHITECTURE.md` (quality gate — binding),
`.github/copilot-instructions.md`, `.github/pull_request_template.md`.

## System shape

```
apps/desktop (Slint 1.18.1 desktop app, production — ADR 0006)
   └─> hoshi-controller (tools, gestures, viewport, colors; no UI deps)
          └─> hoshi-core (document, sparse COW tiles, history-on-disk, .hoshi I/O)
```

- Toolchain pinned 1.100.0, edition 2024. `--locked` everywhere. CI runs on
  Linux/Windows/macOS via `cargo xtask test <suite>` (`docs/TESTING.md`;
  smoke table in `xtask/src/smoke.rs`; coverage floors in
  `xtask/src/coverage.rs`: core 96%, controller 97%, desktop 82%). Test epic #63.
- One integration-test binary per crate (`tests/main.rs`, `autotests = false`):
  a task adding `tests/<area>.rs` must also list `tests/main.rs` in scope.
- Core: sparse 128×128 `Arc` copy-on-write tiles, ≤262,144 px/axis, global
  memory budget (`memory.rs`, static atomics), disk-backed compressed undo.
- Shapes: draw → adjust → commit; one history entry per commit.
- Desktop state: `SharedController = Rc<RefCell<Workspace>>`; `Workspace` holds
  tabs, each with its own `Controller`.

## Decisions (keep consistent)

1. Errors: `thiserror` enums in libraries; text produced at the adapter/locale
   boundary. Migrating away from `Result<_, String>` file by file.
2. No `unwrap/expect` in non-test code; encode invariants in types instead.
3. No `Arc<Mutex>`; single-threaded UI with `&mut`; channels for background work.
4. Tasks for Codex: **1–2 files each**, one concern, behavior-preserving unless
   stated, with explicit "do not touch" lists and a stop condition.

## Known debt (audit 2026-11, refreshed 2026-11-16)

Fixed: adapter errors typed (`FileError`, `WorkspaceError`, no
`Result<_, String>` left in `apps/desktop/src`); core `Error` `Display` is
developer text, adapters map variants to catalogs; `editing.rs` expects gone.

1. **`ProjectError::Io(String)`** (core `project.rs`) still drops the io source.
2. **`Rc<RefCell<Workspace>>` + `Deref<Target = Controller>`** — every callback
   reaches through the workspace to the active controller; with no tab open,
   mutations silently hit a hidden `empty` controller. RefCell re-entrancy
   risk. Plan: #146.
3. **Primitives / big hubs** — `HandlePoint.kind: i32` with magic 8/9 (#153);
   `apps/desktop/src/lib.rs` ~480 lines (markup now in `ui/main-window.slint`);
   core `lib.rs` ~776 lines, ~42 `pub fn` (`Color`/`Layer`/`Document`).

## Task queue

Status lives in GitHub issues (`codex-task` label); this list is the plan.

- Done: T1 #35, T2 #36 (#83), T3 #37 (#105), T4 `WorkspaceError` (#93), #39
  (#107), #41, #50, #74 (#128), i18n epic #119 + #125, DRAW-1 chain #42–#48.
- [ ] TOOL epic #51 (ADR 0003). Done: #53 ToolId, trait/ToolCx + Pan (#108),
      Brush/Eraser (#117), Shapes slice 6 (#127 via #150).
      Now: #147 rail ToolId string keys (Codex, PR #154) → #153 `HandleKind`
      slice 7 (blocked on #147) → #146 retire `empty`/`Deref` (phase 1 =
      slice 8, after #153) → settings → Color Picker options.
      Desktop: #148 `FileAction` enum replaces `file-action(int)` (after #147).
- [ ] SEL epic #52 (ADR 0001). Done: #54 polygon (#122), #55 MaskStore (#129).
      Next: Selection type → State+history → clip → ActiveEdit → floating → …
- [ ] #137 canvas render into Slint buffer (no per-frame Vec + copy). Later:
      dirty-tile redraw (needs core change tracking) → GPU Slint renderer ADR
      (femtovg/skia) → wgpu eval.
- [ ] Core follow-ups: #131 ellipse radii once, #132 fingerprint regen doc,
      #133 `MaskStore::len` rename.
- [ ] Newer refactors (enums, `view_model.rs` split, core `lib.rs` split,
      desktop coverage floor, test helpers, renames): see open `codex-task`
      issues.
- Decisions (owner): **Paint.NET 5.2 parity** is the goal (`docs/PARITY.md`,
  ADRs in `docs/adr/`); iPad/iOS after desktop parity. Color Picker tool samples
  the canvas like Paint.NET: left click → Primary, right → Secondary; default
  samples the current layer, Ctrl = Image (#166). Separate Screen Picker action
  (Hoshi extra) still writes the selected slot. Lost
  pointer capture / focus loss **commits** an active stroke; a failed stroke commit **discards** it (#77). Brush size is one
  shared toolbar value. Ctrl+Shift+Z = Layer Rotate/Zoom (not Redo).
  Esc commits live edits (Paint.NET). Fonts: all system fonts + a bundled
  curated open-font pack (OFL/Apache/UFL). **Slint is the production
  desktop toolkit, GPL-3.0 (ADR 0006)**; iPad toolkit open until the Apple-host
  checklist runs. Open: blend reference renders (owner).
- File later slices only when their prerequisites merge (keeps file refs true).
  If one must be filed early, add the `blocked` label plus a "⛔ Blocked until
  #N merges" banner, and don't assign it to Codex. On every PR refresh: when
  #N has merged, re-check the issue's file refs, then drop the label and banner.
- Workflow: Codex gets "Implement issue #N"; review and fixes run
  automatically on the PR (below; `.github/workflows/agent-*.yml`).
- Issues never quote coverage numbers: write "coverage floors pass
  (`cargo xtask test coverage`)"; floors live in `xtask/src/coverage.rs`.
- Next (after #146): drop `Deref` caller file by file (`empty` carries
  drawing settings while no tab is open, so a plain no-op is a regression).

## PR review checklist

Blocking:
- [ ] Only the task's files changed (plus manifest if a dep was requested).
- [ ] Dependency direction intact; no Slint/OS types in core/controller.
- [ ] No new `unwrap/expect/panic/unreachable` outside tests.
- [ ] No new `Result<_, String>` or `.to_string()` error flattening.
- [ ] No new user-facing text in Rust (any crate): copy comes from both i18n catalogs.
- [ ] No lazy `.clone()` of non-`Copy` data; no `Arc<Mutex>`; no new `Rc<RefCell>`.
- [ ] No `RefCell` borrow held across a call that can re-enter a callback.
- [ ] Document atomicity: failure leaves prior state; one history entry per edit.
- [ ] Behavior preserved (tests unchanged or strengthened, none ignored/deleted).
- [ ] fmt / clippy `-D warnings` / tests green on CI (all 3 OSes).

Follow-up (non-blocking): naming, fn length, missing doc comments, test gaps.

Nits are fixed in the PR, not filed (owner, 2026-11-18):
- Review loop (`agent-claude.yml`, no copy and paste): when a PR opens or
  leaves draft, or the owner or Codex comments `@claude review`, Claude posts
  one review and returns a verdict. In-scope fixes (files the PR already
  changes, no owner decision) go straight to Codex (`agent-codex.yml`), which
  pushes to the PR branch and starts CI; Claude then posts a re-check, and
  anything still open goes back to Codex (`agent-round.yml`). At most three
  rounds per request; after that, and for scope exceptions or design
  questions, it stops at "Needs the owner". Manual fix requests: a PR comment
  starting `/codex-fix`.
- Auto-merge (owner, 2026-11-24; `agent-merge.yml`): a `codex/*` PR merges
  (squash) once Claude's review or re-check says ready (an `agent-review`
  commit status on that exact commit; the `agent-ready` label mirrors it) and
  CI succeeded on the same commit with no failed checks. A new push has no
  status, so it needs a new review. Never auto-merged: other branches, PRs touching
  `.github/` or `docs/adr/`, PRs labelled `hold`, or conflicts. Everything
  else still merges only on the owner's ask.
- Owner inbox: the `needs-owner` label (issues and PRs). Bots add it, with an
  @-mention, on "Needs the owner", a failed Codex run or merge conflicts;
  "ready" removes it. Claude adds it to any issue waiting on an owner
  decision. Saved search: `is:open label:needs-owner`.
- Never write `@codex…` in PR text or comments: ChatGPT's connector answers
  any such mention except `@codex review` from legacy Codex Cloud, which
  can't see our environment.
- Other owner `@claude` comments are architecture work (docs, ADRs, issues
  only; no product code).
- Findings outside the PR's files → an issue only if it is a real defect or
  blocks planned work; otherwise add a line to that area's open `Polish:` issue.

Review output: findings only, `file:line`, scenario → fix, label blocking/follow-up.
