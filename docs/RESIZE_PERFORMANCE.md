# Resize responsiveness probe

The reported Shift-resize lag was dominated by CPU preview generation and
software rendering in unoptimized development artifacts. Aspect-lock geometry
itself took less than a microsecond. Geometry semantics are unchanged.

A synthetic docked UI probe uses a 512×512 RGBA document, an opaque rectangle,
a 1120×760 logical window, four warm-up updates, and 28 measured drag updates.
Before/after measurements were taken on the same managed Linux host. Values are
rounded milliseconds, and are not physical pointer-to-photon measurements.

| Aspect-locked operation | Before p50 / p95 | After p50 / p95 |
| --- | --- | --- |
| Controller action and UI property/image update | 62.1 / 72.5 | 5.2 / 7.0 |
| Above plus full software-rendered snapshot/readback | 227.1 / 241.5 | 24.2 / 27.4 |

The controller-only probe confirms that locked/unlocked geometry has negligible
cost compared with full-image preview generation. It also exercises a 33°
rotated rectangle. Current full CPU previews are roughly 5 ms on this host,
compared with roughly 50 ms in the previous unoptimized development build.

Changes:

- Optimize the document, controller, Slint core and software renderer at level 3,
  and generated application code at level 2 in the development profile. Debug
  symbols and assertions remain available. Packaging uses this same profile.
- Use exact transparent/opaque source-over identity paths. A reference test
  checks 1,310,720 alpha/opacity/channel combinations, including noncanonical
  transparent colors and low-opacity rounding boundaries.
- Avoid alpha arithmetic for already opaque checkerboard-preview pixels.
- Skip unchanged color-control conversion/synchronization during geometry events;
  history UI models are rebuilt only when their rows or pending-shape state change.

Reproduce the current measurements from the repository root:

```sh
cargo run -p hoshi-controller --example controller_resize_profile --locked
cargo run -p hoshi-canvas-spike --example ui_resize_profile --locked
```

The UI probe uses the headless software renderer and synthetic controller
callbacks; it does not require a display server. These probes print timings,
not pass/fail timing thresholds, because shared runner load and hardware vary.
The software snapshot includes readback/copies that differ from normal display
presentation. Large-document, multi-layer, GPU and real Windows/macOS input
latency measurements remain release acceptance work.
