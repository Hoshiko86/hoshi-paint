# Architecture decisions

## Required: architecture quality gate

Every major feature or structural change includes a short review of these
principles in its pull request:

- **SOLID:** each module and component has one clear reason to change; public
  interfaces are small; extensions do not require modifying unrelated tools;
  implementations preserve their interface contracts; callers depend on the
  narrowest useful interface; orchestration depends on core abstractions.
- **DRY:** rules and semantic design values have one source of truth. Similar
  code may remain separate when it represents different behavior and would
  otherwise create a misleading abstraction.
- **Atomicity:** a user edit either commits one complete history operation or
  leaves the document unchanged. File replacement and persistent settings must
  not expose partial writes.
- **Consistency, isolation, durability:** document invariants are checked at
  command boundaries; previews do not corrupt committed state; independent
  documents and jobs do not leak state; successful saves and history records
  survive the lifetime promised by their storage interface. These ACID ideas
  apply to state transitions and persistence rather than every UI interaction.
- **Inversion of control:** UI and platform adapters emit commands and receive
  models through callbacks. Editing policy belongs in `hoshi-controller` or
  `hoshi-core`; operating-system details stay behind adapter interfaces.
- **Law of Demeter:** a module communicates with its direct collaborators through
  intentional APIs. Callers do not reach through controllers, documents, layers,
  storage engines, or UI children to manipulate their internals. Repeated access
  chains indicate a missing command, query, or boundary abstraction.
- **Composition over inheritance:** behavior is assembled from focused Rust
  types, traits, Slint components, and injected callbacks. Inheritance is limited
  to framework-required visual primitives and must not become the mechanism for
  sharing editing policy or application state.

The review records evidence, exceptions, and any follow-up issue. Passing tests
alone does not satisfy this gate.

Semantic UI colors and dimensions live in `ui/theme.slint`. Shell components
consume those roles rather than duplicating literal values in application
composition or tests.

## Accepted: Rust core independent of UI

`hoshi-core` owns raster documents, layers, compositing, tool geometry and history.
It must not depend on windowing, GPU handles, filesystem dialogs, or OS APIs.
Coordinates and pixel conversion are explicit. UI input becomes document actions.
This enables deterministic tests and shared desktop/mobile editing behavior.

## Accepted: temporary editable raster shapes

An active shape is separate from layer pixels. Preview is derived from document
plus active shape; cancel discards it, commit changes pixels once. A committed
shape has no persistent vector identity. See the interaction specification.

## Initial implementation: bounded dense pixels and snapshot history

The core now uses sparse RGBA8 layers with implicit backgrounds and shared 128×128
copy-on-write tiles. Canvas dimensions support 262,144 pixels per axis. Pixel/scratch
allocations share a memory budget; viewport rendering, scanline resampling and native/
PNG streaming avoid whole-image working buffers. See [large images](LARGE_IMAGES.md)
for resource policy and evidence. [Undo history](DISK_HISTORY.md) stores compressed,
shared tile records on disk without a fixed entry cap.

Initial source-over blending operates in encoded RGBA8 space, consistently for
preview and commit. Color-space handling, antialiasing, and linear-light rendering
need decisions before visual-fidelity acceptance; current raster edges are hard.

## Proposed: wgpu rendering

Evaluate accelerated presentation/compositing while preserving deterministic
document semantics. Core CPU reference results must remain available for tests.
No GPU dependency is required by the current slice.

## Open: UI framework

The [source-backed evaluation](UI_EVALUATION.md) recommends a bounded Slint-first
spike, with egui as a desktop fallback. This is not a production stack selection;
toolchain compatibility, licensing, desktop input and Apple-host validation remain
gates before adopting a framework.

The first [desktop integration spike](SPIKE_RESULTS.md) uses Slint 1.18.1,
software rendering and a separate UI-independent `hoshi-controller` crate.
Rust is now pinned to 1.99.1. This establishes a desktop evaluation baseline,
not production framework selection, GPU validation or iOS support.

Compare Slint, egui, and a native-shell approach. Evaluate: desktop pointer
capture, pen/pressure, IME and text, accessibility, menus/dialogs, high DPI,
iOS compilation/distribution, custom canvas integration with wgpu, licensing,
maintenance, and build complexity. Record supported versions and evidence.
Do not commit the product to a framework based solely on advertised portability.

## Proposed: versioned project container and Wasm effects

Project format must preserve layer order/properties/pixels, validate limits,
reject unsupported versions clearly, and use atomic replacement on save.
Specify recovery and migrations before implementation. No arbitrary executable
content belongs in project files.

Future Wasm plugins receive explicit image/effect interfaces, bounded memory,
execution/cancellation limits, and no ambient filesystem/network access. iOS
distribution policy and runtime feasibility require separate investigation.
