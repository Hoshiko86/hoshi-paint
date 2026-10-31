# Architecture decisions

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

The first core slice uses dense RGBA8 buffers, caps canvases at 1,048,576 pixels,
limits documents to 32 layers, and keeps at most 16 history entries. It prioritizes correctness and is NOT the
final large-document architecture. History snapshots share immutable pixel
buffers; editing copies the affected layer. Before raising this limit for the
2048×2048 workload, replace snapshots with bounded tile deltas or measure and
justify another approach. Allocation/decode limits and history memory budgets
must also cover hostile input and iOS memory pressure.

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
