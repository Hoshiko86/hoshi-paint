# Desktop canvas integration spike

An experimental Slint shell, not the final editor. The framework can still be
replaced; editing behavior lives in `hoshi-core` and `hoshi-controller`.

## Build and run

Install rustup and the platform's native linker/toolchain. The repository pins
Rust **1.99.1** and Slint **1.18.1**, with resolved dependencies in Cargo.lock.
Linux needs pkg-config and fontconfig development files plus the X11/Wayland,
fontconfig and xkbcommon runtime libraries. On Ubuntu, start with:

```sh
sudo apt-get install build-essential pkg-config libfontconfig1-dev libxkbcommon-x11-0
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
- Select Move shape and drag, or use nudge/size buttons to adjust it.
- Enter commits, Escape cancels. The toolbar also exposes both actions.
- Undo/redo use the toolbar or Ctrl/Cmd+Z, Ctrl/Cmd+Shift+Z; Ctrl+Y also redoes.
  Shortcuts apply only when the canvas has keyboard focus.
- Choose a color before drawing or while a shape is active.
- Pan canvas changes the viewport; scroll zooms about the pointer. Zoom buttons
  use a fixed logical anchor. Reset view restores 1:1 coordinates.
- Edit the layer name. This commits an active shape before renaming it.
- Help → About Hoshi Paint opens an input-blocking overlay with Slint attribution.
- Losing window activation cancels an unfinished drag; a released active shape
  survives. Clicks and zero-area drags do not paint stray edge pixels.

Only one layer is exposed in this shell. There is no brush, file import/save,
shape rotation, outline styling, rounded rectangle, ellipse tool or selection UI
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
