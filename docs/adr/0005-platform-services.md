# 0005. Platform services: clipboard, text, pen input, colour management

- Status: Proposed
- Date: 2026-11-12
- Related: ADR 0001 (floating paste), 0003 (`PointerInput`), 0004, `screen_pick.rs`

## Context

Four parity features need the OS: clipboard (0.1.0), pen pressure (0.2.0),
Text tool (0.7.0) and colour management. Facts checked 2026-11-12:

- **Slint 1.18.1** `PointerEvent` has only `button`, `kind`, `modifiers`,
  `touch_finger_id` — no pressure or tilt (local source,
  `i-slint-common-1.18.1/builtin_structs.rs`). The winit backend exposes
  `WinitWindowAccessor::on_winit_window_event` and the raw window handle (our
  `raw-window-handle-06` feature is already on).
- **winit 0.30.13** (our lockfile) reports `Touch.force` but no pen tilt; pen
  support is an open request ([winit #99][w99]).
- Already in `Cargo.lock` via Slint, so no new licence surface: **arboard 3.7.0**
  (MIT OR Apache-2.0, Slint enables `wayland-data-control`), **parley 0.11.1**
  (Apache-2.0 OR MIT), **harfrust 0.12.0** (MIT), **fontdb 0.24**, **swash 0.2.10**.
- Paint.NET pressure input uses the Windows Ink "pointer" API. Pressure maps
  to tip **size** on Paintbrush, Eraser, Clone Stamp and Recolor. The toggle
  appears only when a pen is detected ([Paintbrush][pb], [Eraser][er]).
- Paint.NET 5.1 colour management ([5.1 notes][cm51], [Image menu][img]):
  embedded profiles are kept and displayed through real-time conversion.
  Image → Color Profile offers **Assign** (relabel) and **Convert** (transform),
  built-in sRGB/Adobe RGB/Display P3/ProPhoto, ICC import, and export of the
  image's profile. A bad ICC is ignored and the image treated as sRGB; a CMYK
  profile on an RGB image is ignored and removed ([change log][road]).
- Paint.NET clipboard ([Edit menu][edit], rev. 9 Sep 2026):
  - Copy copies the selection, or the whole layer when there is none. Copy
    Flattened copies all layers (renamed from Copy Merged in 5.2).
  - Paste overwrites the active layer as selected pixels and activates Move
    Pixels. When zoomed in, pixels are "placed within the current viewport".
    An oversized image prompts to resize the canvas.
  - Paste into New Layer creates the layer above the active one. Paste into New
    Image makes an image the clipboard's size.
  - Copy/Paste Selection exchanges a JSON polygon list as text.
- All candidates below are permissive and GPL-3.0-only compatible (MIT,
  Apache-2.0, BSD-3-Clause).

## Decision

**Ports live in the desktop crate under `src/platform/`, free of Slint types.**
Each port is a small trait over core/controller value types, so the iOS shell
can implement the same trait. When the iOS adapter starts, the trait files move
verbatim into a new `crates/hoshi-platform` (ports only, depends on core). All
`cfg(target_os)` code stays inside `platform/*` (extends the `screen_pick.rs`
rule). Errors are `thiserror` enums per port; text is produced by localization.

### Clipboard

```rust
pub trait Clipboard {
    fn read(&mut self) -> Result<ClipContent, ClipboardError>;   // Image | Text | Empty
    fn write_image(&mut self, img: &ClipImage, text: Option<&str>) -> Result<(), ClipboardError>;
}
pub struct ClipImage { width: u32, height: u32, pixels: Vec<Color> } // straight RGBA8, budgeted
```

- Desktop implementation: **arboard** (`image-data` feature). Copy writes the
  selection's pixels (ADR 0001 mask applied, outside = transparent) cropped to
  its bounds, or the whole layer without a selection. Copy Flattened samples
  the composite.
- PNG on the clipboard: arboard's image API converts to the platform bitmap
  format; a PNG-format flavour (for alpha interop with browsers/Paint.NET) is a
  follow-up via the per-OS APIs already in our dependency set (`windows-sys`
  `RegisterClipboardFormat("PNG")`, `NSPasteboard` `public.png`, Wayland/X11
  `image/png`). Read prefers PNG > bitmap > text-as-path.
- Paste → `Controller::paste(ClipImage, PasteTarget::{Layer, NewLayer, NewImage})`.
  `Layer`/`NewLayer` create a `Floating` (ADR 0001 §5), set the selection to its
  rectangle, and switch to Move Pixels. Position: the canvas origin when it is
  visible, otherwise the top-left of the visible viewport. The docs confirm
  only the viewport case; the origin default is inferred. Larger-than-canvas
  images first ask *Expand canvas / Keep size*. `NewLayer` inserts above the
  active layer. `NewImage` opens a tab sized to the image.
  History entries follow ADR 0001 (paste, each drag, Finish Pixels).
- Clipboard reads run on the UI thread with a size check before allocation;
  an oversized image fails with `MemoryLimit` before decoding.

### Text tool

- **Font discovery** (OS) in the adapter: `trait FontSource { fn families(&self)
  -> &[FamilyInfo]; fn load(&self, id: FontId) -> Result<Arc<[u8]>, FontError>; }`
  implemented with **fontdb** system scan (Windows/macOS/Linux; iOS later via
  CoreText).
- **Shaping, layout and rasterization** are pure and live in a new crate
  `crates/hoshi-text` (depends on core): **parley** with system-font features
  disabled, fed font bytes; glyph outlines via **swash**/skrifa → a
  `MaskStore` (ADR 0001) composited by core like a stroke. Deterministic tests
  use a bundled OFL test font.
- Text is an `ActiveEdit::Text`, editable and movable by its nub until **Esc or
  Finish** (Esc commits in Paint.NET; Enter inserts a new line) or a tool
  switch. One entry, "Text" ([Text tool][txt]). Options: font, size (points at
  image DPI, or fixed 96 DPI), bold/italic/underline/strikethrough, rendering
  mode, justification, antialiasing, blend. Colour is Primary and there are no
  fill styles. Committed text
  is pixels, as in Paint.NET.
- **IME:** an invisible Slint `TextInput` overlay positioned at the caret owns
  focus while editing, so Slint/winit deliver preedit/commit on all three OSes.
  Its text and preedit range are mirrored to the controller as `TextCommand`s;
  the canvas renders preedit underlined. No custom IME code per OS.

### Pen pressure and tilt

```rust
pub struct PenSample { pub pos: (f64, f64) /*logical*/, pub pressure: f32 /*0..=1*/,
                       pub tilt: Option<(f32, f32)>, pub eraser_end: bool }
pub trait PenSource { fn drain(&mut self, out: &mut Vec<PenSample>); }
```

- The adapter correlates pen samples with Slint pointer events by position and
  fills `PointerInput.pressure` (ADR 0003). Absent pen → `None` → full size.
- Core stroke dabs take an optional pressure; Paintbrush, Eraser, Clone Stamp
  and Recolor map it to tip size
  (Paint.NET parity). Pressure → opacity is a Hoshi extra, off by default.
- Backends: **Windows** `WM_POINTER` / `GetPointerPenInfo` via `windows-sys`
  (window subclass on the winit HWND); **macOS** `NSEvent` local monitor for
  `pressure`/`tilt` via `objc2-app-kit`; **Linux Wayland** `zwp_tablet_v2`.
  Evaluate **octotablet 0.1.0** (MIT; Wayland and Windows Ink complete, macOS
  missing, single unstable release — [lib.rs][otab]) in a spike before writing
  our own. **Scope (owner decision):** Windows and macOS are required for
  0.2.0. Linux (Wayland, then X11 XInput2) is best effort and tracked as a
  follow-up if the crate falls short.

### Colour management

- Document metadata: `State.profile: Option<Arc<IccProfile>>` (bytes + parsed
  description); `None` = sRGB. Not pixels, so changing it is a cheap entry.
- **Assign** changes the profile only; **Convert** transforms pixels (an
  ADR 0002 `PixelOp`) and sets the profile — one entry each. Export Profile
  writes the ICC bytes.
- Engine: **moxcms** (pure Rust, BSD-3-Clause OR Apache-2.0; builds for iOS
  with no C toolchain). Fallback **lcms2** crate (Little CMS, MIT C library)
  if accuracy or rendering intents fall short ([moxcms][mox]).
- Display: the adapter converts the preview image from document profile to the
  monitor profile (Windows `GetICMProfileW`, macOS `NSScreen.colorSpace`
  ICC data, Linux colord over `zbus`, else sRGB). Core never sees the monitor.
- **Open = assign, never convert** (Paint.NET 5.1 parity). The embedded ICC from
  PNG/JPEG (`image` 0.25 decoders) becomes `State.profile` with pixels
  unchanged, and it is re-embedded on export. Unparseable profiles → sRGB. A
  CMYK profile on RGB data → dropped. `.hoshi` V3/V4 gets an optional profile
  chunk (coordinate with ADR 0004 to land in one version bump).

## Alternatives considered

- **Traits in `hoshi-controller`.** Correct dependency-wise, but these are
  adapter concerns the controller never calls; moving them to a ports crate
  later is cheaper than undoing a controller API.
- **Slint's built-in clipboard only.** Text-only API; no image support.
- **cosmic-text** (MIT OR Apache-2.0, harfrust) instead of parley: comparable,
  but parley is already compiled for Slint; one text stack is cheaper.
- **Native text engines (DirectWrite/CoreText/Pango).** Best platform fidelity,
  three implementations, non-deterministic tests. Rejected.
- **Wintab on Windows.** Legacy; Windows Ink/`WM_POINTER` is what Paint.NET uses.

## Consequences

- Wayland clipboard depends on compositor support for `wlr-data-control` /
  `ext-data-control` *(unverified for GNOME/Mutter)*; arboard falls back to X11
  under XWayland.
- Pen backends add `unsafe` in `platform/pen_*.rs`, each with `// SAFETY:`.
- `hoshi-text` and `moxcms` add build time; parley/swash are already compiled.

## Implementation slices

1. **Platform module + clipboard port** — desktop `platform/mod.rs`,
   `platform/clipboard.rs` · — · arboard text/image; budgeted read.
2. **Copy/Copy Flattened/Cut** — `controller/src/clipboard.rs` · ADR 0001 slice 5 ·
   `ClipImage` from selection; Cut = copy + Erase Selection (one entry).
3. **Paste targets** — `controller/src/clipboard.rs`, desktop `documents.rs` ·
   ADR 0001 slice 7 · Layer/NewLayer/NewImage; expand-canvas prompt.
4. **PNG clipboard flavour** — desktop `platform/clipboard_png.rs` · 1 · per-OS.
5. **PointerInput.pressure plumbing** — `controller/src/input.rs`, core
   `stroke.rs` · ADR 0003 slice 3, #42 · pressure → dab width; tests with
   synthetic pressure.
6. **Pen spike (Windows, Wayland)** — desktop `platform/pen.rs` · 5 · octotablet
   evaluation report; adopt or replace. Linux is part of this spike only if
  octotablet covers it; otherwise it becomes a follow-up issue.
7. **macOS pen** — desktop `platform/pen_macos.rs` · 5 · NSEvent monitor.
8. **hoshi-text crate** — `crates/hoshi-text/src/lib.rs`, its `Cargo.toml` ·
   ADR 0001 slice 1 · layout + mask from font bytes; golden tests.
9. **Text tool + IME overlay** — `controller/src/tools/text.rs`, desktop
   text overlay `.slint` · 8, ADR 0003 slice 3.
10. **Colour profile metadata** — `core/src/color_profile.rs`, `core/src/lib.rs` ·
    — · Assign/Convert entries; moxcms dependency.
11. **Display transform** — desktop `platform/display_profile.rs` · 10.

## Parity notes

Unblocks Edit → Copy, Copy Flattened, Cut, Paste, Paste into New Layer, Paste
into New Image (with the canvas-expand prompt) and Copy/Paste Selection;
pressure on Paintbrush, Eraser, Clone Stamp and Recolor; the Text tool with
IME; Image → Color Profile (Assign, Convert, import/export, built-ins);
colour-managed display. Hoshi extras: pressure on macOS/Linux, pen tilt.

## Open questions

None for the owner. Decided 2026-11-12: Windows and macOS pen support for
0.2.0, Linux best effort; a bundled OFL font for text tests; Paint.NET's
colour-profile and paste behaviour.

Unverified: the canvas-origin paste default (§Clipboard); compositor support
for Wayland data-control on GNOME.

[w99]: https://github.com/rust-windowing/winit/issues/99
[pb]: https://paint.net/doc/latest/Paintbrush.html
[cm51]: https://blog.paint.net/2024/11/12/paint-net-5-1-is-now-available/
[otab]: https://lib.rs/crates/octotablet
[mox]: https://docs.rs/crate/moxcms/0.8.0/source/README.md
[er]: https://paint.net/doc/latest/EraserTool.html
[img]: https://paint.net/doc/latest/ImageMenu.html
[road]: https://paint.net/roadmap.html
[edit]: https://paint.net/doc/latest/EditMenu.html
[txt]: https://paint.net/doc/latest/TextTool.html
