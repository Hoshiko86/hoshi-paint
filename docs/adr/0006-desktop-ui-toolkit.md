# 0006. Desktop UI toolkit: Slint

- Status: Accepted (owner, 2026-11-15)
- Date: 2026-11-15
- Deciders: owner (approve), Claude (architect)
- Related: ARCH-1 ([UI_EVALUATION.md](../UI_EVALUATION.md)),
  [SPIKE_RESULTS.md](../SPIKE_RESULTS.md), ADR 0003, ADR 0005, #137

## Context

ARCH-1 evaluated Slint first, with egui as the desktop fallback, and left the
production choice open until desktop, licensing and iOS gates were met
([UI_EVALUATION.md](../UI_EVALUATION.md)). Since then the Slint 1.18.1 adapter
in `spikes/desktop-canvas` has grown into the working editor:

- Tabs, Open/Save, the Image menu, shapes, brush and eraser, history, colors,
  palettes and Screen Picker, docking panels, and two locales.
- Every user-facing string comes from the catalogs. Guards fail on English in
  Rust or `.slint` source (#126, #138).
- CI builds and tests it on Linux, Windows and macOS, including native smoke
  runs (`docs/TESTING.md`).
- The core and controller have no Slint types (`docs/ARCHITECTURE.md`).

No desktop must-have from ARCH-1 has failed so far, so the egui fallback was
never triggered. The iOS gate is still unrun: no Apple host has been available
([IOS_SPIKE_CHECKLIST.md](../IOS_SPIKE_CHECKLIST.md)).

Slint is offered under GPLv3, a royalty-free license with conditions, or a
commercial license. Hoshi Paint is `GPL-3.0-only` (workspace `Cargo.toml`).

## Decision

1. **Slint is the production UI toolkit for the desktop app** on Windows, macOS
   and Linux. The adapter is no longer a disposable spike: it is held to the
   same quality gate as the core and controller (`docs/ARCHITECTURE.md`,
   `AGENTS.md`, the CLAUDE.md review checklist).
2. **License: Slint's GPL-3.0 option.** The app stays `GPL-3.0-only`, so no
   Slint attribution badge is required. A future non-GPL distribution would need
   a new ADR, either for the royalty-free license and its conditions or for a
   commercial license.
3. **Scope: desktop only.** iPad/iOS stays open until the Apple-host checklist
   has run. That later decision may choose Slint, or a shared core with a native
   iPad shell. This ADR is the "explicit scope decision" that ARCH-1 requires.
4. **The boundary is unchanged.** Slint types stay in the desktop crate. Core and
   controller remain UI-independent, so an iPad shell or another toolkit can
   reuse them.
5. **The version stays pinned (`slint = "=1.18.1"`).** An upgrade is its own PR.
   It must pass the full suite and smoke runs on all three OSes, and record any
   behavior changes.
6. **Rendering:** the software renderer stays for now. Canvas performance work
   follows #137: render into Slint's buffer, then dirty tiles. A GPU renderer
   (femtovg or Skia) or a wgpu texture path needs its own ADR with
   measurements.

## Alternatives considered

- **egui / eframe:** this was the fallback, kept for the case where Slint failed
  a desktop must-have, and none has. It has no listed iOS target
  ([UI_EVALUATION.md](../UI_EVALUATION.md)), so it would not settle iPad either.
- **Native shells per OS:** strongest platform fit, but they multiply UI work
  before the editor reaches parity. That route stays open for iPad only.
- **Keep the spike label:** this keeps the "disposable" exemption alive for the
  code users run, which conflicts with Paint.NET parity work landing in it.

## Consequences

- **Positive:** a single toolkit for the desktop app, and parity work stops
  carrying spike caveats. Docs, CI and the crate name can say what the code is.
- **Negative:** Slint's pointer events carry no pressure or tilt (ADR 0005), so
  pen input needs the winit hook that ADR 0005 describes. The app also depends
  on Slint's release cadence.
- **Follow-up:**
  - rename and move the crate (slice 2)
  - retire the spike wording in docs
  - split the oversized adapter files (CLAUDE.md "Known debt")

## Implementation slices

1. **Record the decision.** Files: this ADR, the ADR index, and the status lines
   in `UI_EVALUATION.md`, `SPIKE_RESULTS.md`, `ARCHITECTURE.md` and `CLAUDE.md`.
   Done when no doc calls the desktop adapter disposable or the framework
   decision open.
2. **Move and rename.**
   - `spikes/desktop-canvas` → `apps/desktop`, crate `hoshi-canvas-spike` →
     `hoshi-desktop`.
   - Update the workspace manifest, CI, xtask paths (smoke, coverage, english
     guard, changed-path rules), README and docs.
   - Mechanical only, depends on 1. Done when CI is green on all three OSes and
     `rg 'spikes/desktop-canvas|hoshi-canvas-spike'` finds only history docs.
3. **Main window layout out of `lib.rs`.** Move the `slint!` macro body to
   `ui/main-window.slint`. Depends on 2. Done when `lib.rs` holds no Slint
   markup and the tests are unchanged.
4. **`refresh` → `view_model.rs`.** Depends on 3. Behavior-preserving.

## Parity notes

This ADR unblocks nothing new in Paint.NET terms. It removes the "spike"
qualifier from the parity work already landing in the desktop adapter.

## Open questions

1. **iPad/iOS toolkit:** decided after the Apple-host checklist (owner).
2. **GPU renderer choice:** decided after #137 measurements (architect, with
   owner approval).
