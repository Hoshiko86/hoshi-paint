# Desktop canvas integration spike

An experimental Slint shell, not the final editor. The framework can still be
replaced; editing behavior lives in `hoshi-core` and `hoshi-controller`.

## Build and run

Install rustup and the platform's native linker/toolchain. The repository pins
Rust **1.99.1** and Slint **1.18.1**, with resolved dependencies in Cargo.lock.
Linux needs pkg-config and fontconfig development files plus the X11/Wayland,
fontconfig and xkbcommon runtime libraries. On Ubuntu, start with:

```sh
sudo apt-get install build-essential pkg-config libfontconfig-dev libxkbcommon-x11-0
```

On macOS install Xcode command-line tools. On Windows use the MSVC Rust toolchain
with Visual Studio C++ build tools. Then, from the repository root:

```sh
cargo run --package hoshi-canvas-spike --locked
```

The spike selects software rendering; it does not enable the Qt, Skia or GPU
renderer. To explicitly select the desktop backend, set `SLINT_BACKEND` to
`winit-software` in the shell. No server, credentials or network connection is
needed after dependencies have been downloaded.

For the managed Debian 13 cloud machine, `bash scripts/setup-cloud-linux.sh`
prepares rootless packages using signed Debian indexes. Source
`scripts/cloud-env.sh` in each new shell. It activates the retained toolchain,
local pkg-config sysroot, runtime libraries and four-job build limit. Each task
is already isolated; use the existing checkout rather than creating a worktree.

## Try the interaction

- Drag on the checkerboard to draw a filled rectangle. Release keeps it editable.
- Drag any of the eight square control points: corners resize both dimensions;
  edge midpoints resize one. The opposite handle stays fixed, within integer-pixel
  rounding, even after rotation. Crossed edges clamp at one pixel.
- Hold Shift while dragging a corner to preserve the aspect ratio from the start
  of that drag, within pixel rounding. Press/release Shift mid-drag to toggle it
  live; edge handles remain single-axis. Works after rotation and zoom.
- Drag the round control point above the top edge to rotate about the center
  in one-degree increments. Handles keep their screen size when zoomed.
- Drag inside the active rectangle to move it, including after rotation.
  Handles take precedence. Move shape and nudge/size buttons also remain available.
- Enter commits, Escape cancels. The toolbar also exposes both actions.
- Undo/redo use the toolbar or Ctrl/Cmd+Z, Ctrl/Cmd+Shift+Z; Ctrl+Y also redoes.
  Shortcuts apply only when the canvas has keyboard focus.
- Open Colors… before drawing or while a shape is active. The picker supplies
  RGB (0–255), HSV (degrees/percentages), a saturation/value area, hue slider,
  alpha (0 transparent,255 opaque), presets, and hex RGB/RGBA. Changes apply live.
  Hex uses #RGB,#RGBA,#RRGGBB or #RRGGBBAA; press Enter or Apply.
- The picker says **Editing the active shape** while resize handles are visible.
  After Enter or starting another shape it says **Next drawing color**: existing
  pixels remain unchanged, and the chosen color applies to the next drawing.
- Pick / Eyedropper samples merged canvas pixels with alpha, not checkerboard
  colors. It preserves the active shape and applies sampled colors immediately.
  Sampling uses a frozen visible composite per gesture to avoid alpha feedback.
  Returning to Rectangle preserves the shape for further adjustment.
- **Pick from screen…** in Colors opens a native screen sampler for pixels in
  other applications. Screen colors are opaque RGB; Escape cancels without
  changing the color or shape. Platform support is described below.
- Pan canvas changes the viewport; scroll zooms about the pointer. Zoom buttons
  use a fixed logical anchor. Reset view restores 1:1 coordinates.
- Edit the layer name. This commits an active shape before renaming it.
- Help → About Hoshi Paint opens an input-blocking overlay with Slint attribution.
  Opening it cancels an unfinished drag before blocking controller callbacks.
- Losing window activation cancels an unfinished drag; a released active shape
  survives. Clicks and zero-area drags do not paint stray edge pixels.

Only one layer is exposed in this shell. There is no brush, file import/save,
outline styling, rounded rectangle, ellipse tool or selection UI
yet. Core support does not imply that a corresponding UI has shipped.

## Validation commands

```sh
cargo test --workspace --locked
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
python3 -m unittest discover -s scripts -v
```

The interaction test installs a real **headless software rasterizer**, dispatches
synthetic Slint pointer/key/focus events, compares rendered pixel samples, checks
preview/commit/cancel/undo, text editing, focus policy, and modal input isolation.
It is not a physical mouse/keyboard or OS input-capture test.

For an actual Linux winit window under Xvfb:

```sh
SLINT_BACKEND=winit-software HOSHI_SPIKE_SNAPSHOT=target/spike-desktop.png \
  xvfb-run -a cargo run --package hoshi-canvas-spike --example desktop_smoke --locked
```

This starts the native desktop event loop and software renderer, dispatches
synthetic drawing/key events, verifies visible pixels, optionally saves a PNG,
and exits. The managed command sandbox may require escalation for X11 sockets.

CI builds/tests the workspace on Windows, macOS and Ubuntu and runs the real
Xvfb smoke test on Linux. It uploads experimental, unsigned, **debug binaries**
as `canvas-spike-*` artifacts. They require the documented runtime libraries;
they are not installers or signed application bundles.

## Evaluation boundaries

This is a CPU reference upload path: every refresh creates a core composite,
a checkerboard RGBA buffer and a Slint SharedPixelBuffer, then copies the RGBA
bytes into the latter. It currently does this even for viewport changes. Slint's
internal copies/uploads are not measured. No GPU/zero-copy/latency claim is made.

Continue evaluating Slint; do not select it as the production framework yet.
Pending evidence: real OS pointer capture outside the window, native IME,
mixed-monitor DPI, X11/Wayland manual input, screen readers, release-build
performance budgets, GPU texture integration/device loss, and iOS. See
[the Apple-host checklist](../../docs/IOS_SPIKE_CHECKLIST.md). The software desktop
renderer does not establish iOS support; the official iOS path requires Skia.

Slint's royalty-free 2.0 license is selected for this experiment, with the
top-level Help → About attribution. See THIRD_PARTY_NOTICES.md. A complete
transitive license inventory remains a packaged-release gate.

The widget palette is explicitly light to match the fixed light shell, even on
dark-themed Windows. Rendered tests check enabled-label contrast and a second
scenario exercises resize/rotation through synthetic window events.

A separate `color_smoke` example opens the actual desktop color window, edits
HSV/hex, samples alpha-preserving canvas color and exits. Use the same Xvfb
command with `--example color_smoke` and optional `HOSHI_COLOR_SNAPSHOT`.

## Native cursors and icon attribution

The canvas eyedropper uses a real native cursor with an outlined Font Awesome
Free SVG and tip hotspot, tested under X11 at 1x and 2x scale. Slint 1.18.1's
`MouseCursor.custom` is experimental; `.cargo/config.toml` enables its compiler
flag and the exact toolkit version remains pinned. Recheck native cursors on
toolkit upgrades. Font Awesome Free 6.7.2 SVG icons use CC BY 4.0; see
[third-party notices](../../THIRD_PARTY_NOTICES.md) for attribution and changes.

## Screen sampling

Open **Colors… → Pick from screen…**, select a visible pixel on any connected
display, or press Escape to cancel. The sampled opaque RGB color updates the
active shape without committing it; if no shape is active it becomes the next
drawing color. Sampling does not add an undo entry.

- Windows: a temporary desktop overlay catches the selection click and uses
  an eyedropper cursor. Sampling uses physical virtual-desktop coordinates,
  including monitors to the left or above the primary display.
- macOS: AppKit's native `NSColorSampler` supplies its loupe and selection cursor.
  The operating system controls access and may prompt for screen permissions.
- Ubuntu/Linux: the desktop portal's `Screenshot.PickColor` request provides
  screen selection, including on Wayland. A running session bus and a portal
  backend implementing PickColor (such as GNOME/KDE) are required. Missing
  support or denied access produces an error in Colors; canvas sampling remains
  available. A bare Xvfb session does not provide this desktop service.

Protected content and displays the operating system cannot expose may not be
sampleable. Screen sampling returns a displayed color, rather than an original
image's uncomposited pixel/alpha. No screenshot is saved or uploaded. This
desktop feature does not imply screen capture support on iOS.
