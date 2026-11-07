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

- Drag on the checkerboard to draw a rectangle. Release keeps it editable.
  Hold Shift while drawing to create a square; press/release Shift during the drag
  to toggle the constraint live. Squares stop at the canvas boundary.
- Choose Fill, Outline, or Fill and outline in Shape style. Outline width is in
  canvas pixels (1–65,536), inset inside the bounds; a wide stroke can consume
  the interior. Settings apply live to the active preview and to future shapes.
- Primary and Secondary are shared workspace colors, initially black and white.
  Click either swatch in the sidebar or Colors panel to choose the slot being edited.
  Fill-only and outline-only shapes use Primary. With both enabled, outlines use
  Primary and fills use Secondary. Changing tools/styles does not select a slot.
- RGB, HSV, hex, palette swatches, alpha, Restore and Color Picker edit the selected
  slot. Each slot keeps its original comparison color until Colors closes;
  switching slots, swapping colors, or docking does not reset it.
- Swap colors with the arrow icon or X; reset to black/white with the overlapping
  squares icon or D. Shortcuts apply while the canvas has focus, preserving text
  input in color/layer fields. Active shapes update live; committed pixels stay fixed.
  Right-drag remains canvas panning; reversed right-button painting is not implemented.
- Cancel discards the active preview while retaining workspace colors and settings.
- Drag any of the eight square control points: corners resize both dimensions;
  edge midpoints resize one. The opposite handle stays fixed, within integer-pixel
  rounding, even after rotation. Crossed edges clamp at one pixel.
- Hold Shift while dragging a corner to preserve the aspect ratio from the start
  of that drag, within pixel rounding. Press/release Shift mid-drag to toggle it
  live; edge handles remain single-axis. Works after rotation and zoom.
- Drag the round control point above the top edge to rotate about the center
  in one-degree increments. Handles keep their screen size when zoomed.
- Drag inside the active rectangle to move it, including after rotation.
  Handles take precedence. Shape movement is direct; canvas panning uses the
  shortcuts below. Separate Move shape, Pan canvas and nudge/size buttons are no
  longer needed.
- Enter commits, Escape cancels. The toolbar also exposes both actions.
- Undo/redo use the toolbar or Ctrl/Cmd+Z, Ctrl/Cmd+Shift+Z; Ctrl+Y also redoes.
  Shortcuts apply only when the canvas has keyboard focus.
- Open Colors… before drawing or while a shape is active. The picker supplies
  RGB (0–255), HSV (degrees/percentages), a saturation/value area, hue slider,
  alpha (0 transparent,255 opaque), square color swatches, and hex RGB/RGBA. Changes apply live.
  Hex uses #RGB,#RGBA,#RRGGBB or #RRGGBBAA; press Enter to apply a typed hex value.
- **Original / Current** swatches compare the starting RGBA color with live edits,
  including transparency on a checkerboard. **Restore** returns to the original
  color without committing/cancelling the shape or adding history. The reference
  is captured when Colors opens, survives Dock/Float/location changes, and starts
  fresh after closing/reopening. Unaccepted hex drafts are discarded on reopening.
- Narrow Colors panels stack RGB and HSV vertically, preserving full-width slider
  tracks and aligned labels/numeric fields. Wide panels retain two columns.
  The scrolling body keeps all channels, alpha, hex and palette controls reachable.
- The picker says **Editing the active shape** while resize handles are visible.
  After Enter or starting another shape it says **Next drawing color**: existing
  pixels remain unchanged, and the chosen color applies to the next drawing.
- **Colors…** is the single main-window entry to the color panel. Its **Color
  Picker** icon starts the native screen sampler. Select a pixel anywhere the desktop allows, including the
  canvas. The result is opaque RGB; Escape cancels without changing the color.
  Picking is one-shot and keeps the current drawing tool and editable shape.
  RGB/HSV/hex edits can immediately refine the result, repeatedly.
- Colors and History offer **Float / Left / Right / Top / Bottom** in their
  header location selectors. Dock/Float shortcuts remember the last selected edge.
  Panels can choose different edges, or share one edge without covering each other
  or the canvas. Headers remain visible while compact bodies scroll. Placement
  is remembered during the current app session.
- Color preset buttons show only square swatches. Hover shows the palette name
  when known, hex, RGB and HSV. Accessible labels/descriptions remain available;
  clicking a swatch changes RGB while preserving current alpha. Favorite-color
  persistence and palette files remain future work.
- Open **History…** for labeled committed actions, the current state, grayed redo
  entries and available undo/redo counts. Click a retained step to undo/redo to it.
  Commit or cancel an active shape before selecting a history row. Toolbar and
  History Undo/Redo use icons with tooltips. At most 16 actions are retained; the
  first row means **Oldest available state**, not necessarily the original image.
- **Ctrl/Cmd + wheel** zooms about the pointer over the canvas. A plain wheel pans
  vertically (trackpad horizontal movement pans horizontally); Shift + wheel pans
  horizontally. **Space + left-drag**, **middle-drag**, or **right-drag** temporarily
  pan without changing the drawing tool, shape or history. Right-drag is a
  convenience alias; this is not a claim of complete Paint.NET shortcut parity.
  Release, Escape, capture loss or focus loss end temporary navigation. Space in
  text fields remains text input.
- Edit the layer name. This commits an active shape before renaming it.
- Help → About Hoshi Paint opens an input-blocking overlay with Slint attribution.
  Opening it cancels an unfinished drag before blocking controller callbacks.
- Losing window activation cancels an unfinished drag; a released active shape
  survives. Clicks and zero-area drags do not paint stray edge pixels.

Use **File → New…** to set up an image. The **Image** menu provides Resize,
Canvas Size, horizontal/vertical flips, 90°/180° rotations and Flatten. Size dialogs
support pixels/inches/centimeters and aspect locking; Resize also supports percentage
scaling, resolution and resampling. Canvas Size uses a nine-point anchor and crops
or pads without scaling pixels. [Behavior and remaining Paint.NET parity gaps](../../docs/IMAGE_MENU.md).

Only one layer is exposed in this shell. There is no brush, file import/save,
dash/pattern styles, antialiasing controls, rounded rectangle, ellipse tool or selection UI
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

Hoshi Paint and the selected Slint license option are GPL-3.0-only, with
the top-level Help → About attribution retained. See THIRD_PARTY_NOTICES.md. A complete
transitive license inventory remains a packaged-release gate.

The widget palette is explicitly light to match the fixed light shell, even on
dark-themed Windows. Rendered tests check enabled-label contrast and a second
scenario exercises resize/rotation through synthetic window events.

A separate `color_smoke` example opens the actual desktop color window, edits
HSV/hex, docks/floats the same panel, refines RGB and exits. Use the same Xvfb
command with `--example color_smoke` and optional `HOSHI_COLOR_SNAPSHOT` and
`HOSHI_DOCK_SNAPSHOT`.

## Native cursors and icon attribution

The application shows a real native cursor while awaiting screen selection with an outlined Font Awesome
Free SVG and tip hotspot, tested under X11 at 1x and 2x scale. Slint 1.18.1's
`MouseCursor.custom` is experimental; `.cargo/config.toml` enables its compiler
flag and the exact toolkit version remains pinned. CI builds once without a
scale override, then tests that same binary at runtime scales 1 and 2; setting
SLINT_SCALE_FACTOR during compilation can pin the generated UI scale. Recheck native cursors on
toolkit upgrades. Font Awesome Free 6.7.2 SVG icons use CC BY 4.0; see
[third-party notices](../../THIRD_PARTY_NOTICES.md) for attribution and changes.

## Screen sampling

Open **Colors… → Color Picker**, select a visible pixel on any connected
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
  support or denied access produces an error in Colors; RGB/HSV/hex editing remains
  available. A bare Xvfb session does not provide this desktop service.

Protected content and displays the operating system cannot expose may not be
sampleable. Screen sampling returns a displayed color, rather than an original
image's uncomposited pixel/alpha. No screenshot is saved or uploaded. This
desktop feature does not imply screen capture support on iOS.

## Reusable adjustment panels

`ui/colors.slint` defines ToolPanelFrame with a fixed title/placement header and
a scrolling body, plus a reusable ColorPanel rendered by both hosts. Future
adjustment windows must offer independent Dock/Float placement using that frame and keep their state outside the
window so Dock/Float never resets an in-progress edit. Colors and History share this frame; About remains a modal dialog. Placement is currently a session preference,
not a saved workspace layout.

## Resize responsiveness

Development binaries now optimize the document/controller, software renderer,
Slint core and generated application code while retaining debug information and
assertions. The source-over fast paths retain exact RGBA output; geometry-only
updates skip unnecessary color-control synchronization. See
[the measured resize probe](../../docs/RESIZE_PERFORMANCE.md) for before/after
results and reproducible commands. These are development measurements, not
physical display-latency or release acceptance results.
