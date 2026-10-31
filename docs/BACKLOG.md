# Initial GitHub issue backlog

Local planning IDs map to the GitHub issues below. All v0.0.0 issues and the
release milestones are now created. Later releases are in ROADMAP.md; split them
into issues when their scope is ready. The GitHub Project board is blocked by
integration permissions (`createProjectV2`: resource not accessible).

| ID | Deliverable | Priority | Dependencies | Status |
| --- | --- | --- | --- | --- |
| PLAN-1 | Release/interaction/architecture specifications | P0 | — | Written; user review pending |
| CORE-1 | Bounded documents, layers, compositing | P0 | PLAN-1 | Completed; core checks pass on desktop CI |
| CORE-2 | Rectangle/ellipse preview, adjust, commit/cancel, undo/redo | P0 | CORE-1 | Completed; core checks pass on desktop CI |
| ARCH-1 | UI framework decision and iOS feasibility evidence | P0 | PLAN-1 | Research and first desktop spike delivered; hardware/GPU/iOS evidence pending |
| UI-1 | Desktop window, canvas zoom/pan, colors and tool settings | P0 | ARCH-1, CORE-1 | First rectangle/canvas shell implemented; remaining UI scope open |
| DRAW-1 | Brush/eraser strokes and eyedropper | P0 | UI-1, CORE-1 | Backlog |
| LAYER-1 | Layer panel and add/delete/rename/reorder/visibility/opacity actions | P0 | UI-1, CORE-1 | Backlog |
| SHAPE-1 | Handles, rotation, constraints, line and rounded rectangle | P0 | UI-1, CORE-2 | Backlog |
| FILE-1 | Specify and implement versioned layered project format and safe saves | P0 | CORE-1 | Backlog |
| FILE-2 | PNG import/export with decode/allocation limits | P0 | CORE-1 | Backlog |
| UX-1 | Dirty tracking, prompts, keyboard/event routing and save-active-shape behavior | P0 | FILE-1, SHAPE-1 | Backlog |
| PERF-1 | Reference hardware, budgets, bounded tile/delta history | P0 | CORE-1 | Backlog |
| RELEASE-1 | Desktop builds, integrated acceptance and packaging | P0 | all above | Backlog |

## CORE-1 — document foundation

Behavior: callers create bounded transparent documents and manipulate raster
layers. Criteria: reject invalid dimensions, preserve layer metadata/history,
composite source-over in order, apply visibility/opacity, prevent deleting the
last layer, reject invalid indices without mutation. Validation: core tests,
format and lint checks. Excludes UI and serialization.

## CORE-2 — shape lifecycle

Behavior: preview rectangle/ellipse fill and outline, adjust parameters, commit
once or cancel. Criteria: preview does not mutate pixels/history; cancel leaves
history unchanged; commit is one undoable action; redo restores exact pixels;
new edits invalidate redo; off-canvas geometry clips; invalid geometry is rejected;
no-op commits create no history. Validation: deterministic core tests. Excludes
rotation, handles, line, rounded rectangles, and UI shortcut dispatch.

## ARCH-1 — UI and iOS gate

Deliver a comparison against ARCHITECTURE.md criteria, one recorded decision,
and a reproducible iOS compile procedure/result. A real Apple host/device may be
required; report missing evidence rather than claiming success from Linux.
Acceptance: desktop canvas/input integration and accessibility tradeoffs are
understood, licensing checked, iOS blockers documented. No product UI implementation
until the decision is recorded. A bounded evaluation spike is allowed.

## Remaining issue acceptance templates

- UI-1: open a window, pan/zoom anchored to pointer, preserve input coordinates at
  high DPI, expose primary/secondary color and tool settings accessibly.
- DRAW-1: a complete stroke is one undo; pointer capture handles leaving canvas;
  eraser removes alpha; eyedropper samples the documented source.
- LAYER-1: every listed layer action has undo; active-shape transitions follow spec.
- SHAPE-1: all four shapes support applicable styles and adjustment; Enter/Escape,
  constraints, rotation, and transitions match the interaction contract.
- FILE-1: round-trip layers/properties/pixels; detect corruption/unknown versions;
  failed writes preserve prior file and dirty document.
- FILE-2: decoded export equals composite; malformed/oversized imports fail safely.
- UX-1: prompts never silently discard edits; shortcuts and active-shape save pass.
- PERF-1: publish measured results; enforce total history memory and decode bounds.
- RELEASE-1: all required checks and the full scenario pass on three desktop OSes;
  publish build instructions, artifacts, limitations, and dependency licenses.

## GitHub tracking

| Planning ID | GitHub issue |
| --- | --- |
| PLAN-1 | [#1](https://github.com/Hoshiko86/hoshi-paint/issues/1) |
| CORE-1 | [#2](https://github.com/Hoshiko86/hoshi-paint/issues/2) |
| CORE-2 | [#3](https://github.com/Hoshiko86/hoshi-paint/issues/3) |
| ARCH-1 | [#4](https://github.com/Hoshiko86/hoshi-paint/issues/4) |
| UI-1 | [#5](https://github.com/Hoshiko86/hoshi-paint/issues/5) |
| DRAW-1 | [#6](https://github.com/Hoshiko86/hoshi-paint/issues/6) |
| LAYER-1 | [#7](https://github.com/Hoshiko86/hoshi-paint/issues/7) |
| SHAPE-1 | [#8](https://github.com/Hoshiko86/hoshi-paint/issues/8) |
| FILE-1 | [#9](https://github.com/Hoshiko86/hoshi-paint/issues/9) |
| FILE-2 | [#10](https://github.com/Hoshiko86/hoshi-paint/issues/10) |
| UX-1 | [#11](https://github.com/Hoshiko86/hoshi-paint/issues/11) |
| PERF-1 | [#12](https://github.com/Hoshiko86/hoshi-paint/issues/12) |
| RELEASE-1 | [#13](https://github.com/Hoshiko86/hoshi-paint/issues/13) |

## Current validation

See [SPIKE_RESULTS.md](SPIKE_RESULTS.md) for the newer Rust 1.99.1 desktop
experiment: 28 Rust integration tests pass locally, including a rendered UI
scenario; real Linux desktop software-rendering smoke passes. The earlier core
baseline below is retained as historical cross-platform evidence.

The reviewed core passes 15 integration tests, formatting, and Clippy locally on
Linux with pinned Rust 1.85.1. The same checks passed on GitHub-hosted Windows,
macOS and Ubuntu in [foundation CI](https://github.com/Hoshiko86/hoshi-paint/actions/runs/41249496579).
This validates the core, not graphical application behavior or iOS. CORE-1 and
CORE-2 are complete; remaining release work is open.
