# Paint.NET parity inventory

As of **2026-11-12**. Compares Paint.NET **5.2** (released 2026-11-10) with Hoshi
Paint at current `main` (desktop spike + `hoshi-core` + `hoshi-controller`).

Hoshi Paint is *inspired by* Paint.NET's workflow. It is an original
implementation with original assets (Lucide icons, own palettes, own algorithms)
and uses no Paint.NET branding, artwork or trade dress. "Parity" here means
comparable user-facing capability, not identical UI or bit-identical pixels.
Paint.NET .NET plugins are **not** binary compatible; a future Wasm effect API is
planned instead ([ROADMAP.md](ROADMAP.md)). iPad/iOS work starts after desktop parity.

## Legend

| Mark | Meaning |
| --- | --- |
| ✅ | Done in Hoshi (shipped in the desktop spike or core) |
| 🟡 | Partial: what is missing is stated |
| 📋 | Planned with an open GitHub issue |
| ❌ | Missing; a roadmap version is given when [ROADMAP.md](ROADMAP.md) names one |
| (5.x) | Paint.NET version that introduced or renamed the feature, per the release posts |
| (unverified) | Not confirmed by any fetched first-party page |

## Sources (Paint.NET side)

All fetched on 2026-11-12. The official docs live at
[getpaint.net/doc/latest](https://www.getpaint.net/doc/latest/), which redirects to
`paint.net/doc/latest`. Page revisions date from 4 March 2026 to 24 October 2026. A
few pages still have older text: for example, the Menu Bar page quotes "33 effects
in seven sub menus", and the Adjustments count on that page is also stale. The
per-category Effects pages are used instead.

- Doc index and Features: [index.html](https://www.getpaint.net/doc/latest/index.html)
- Release posts: [5.0](https://blog.paint.net/2023/01/10/paint-net-5-0-is-now-available/) (2023-01-10),
  [5.1](https://blog.paint.net/2024/11/12/paint-net-5-1-is-now-available/) (2024-11-12),
  [5.2](https://blog.paint.net/2026/11/10/paint-net-5-2-is-now-available/) (2026-11-10),
  [5.2 alpha build 9719](https://blog.paint.net/2026/08/10/paint-net-5-2-alpha-build-9719/) (new effects)
- System requirements and downloads: [GitHub releases](https://github.com/paintdotnet/release/releases)
- `www.paint.net/roadmap.html` returned HTTP 403 through the session proxy, so its change log was not read.
  The release posts cover the same features.

Key release facts:

- **5.0**: pen pressure (Windows Ink); path smoothing and spacing for all brush
  tools; GPU rendering for almost all effects and adjustments; new GPU effect
  plugin system; Exposure, Highlights / Shadows, Temperature / Tint; Straighten,
  Drop Shadow, Bokeh Blur (replaces Unfocus); Adaptive and Lanczos resampling;
  Line/Curve Straight mode.
- **5.1**: full color management (Image → Color Profile, Windows Advanced
  Color on HDR/WCG displays); Sketch Blur, Square Blur, and Median Blur (moved from
  Noise → Median); Settings → Canvas customization. Canvas Size no longer fills
  with the Secondary color by default.
- **5.2**: FP32 layer rendering engine; new versioned FileType plugin system
  (RGBA64, CMYK, RGBA FP16); Plugin SDK on NuGet; CICP (AVIF/JPEG XL/PNG); HDR
  tone-mapped to SDR on load; 8 new effects; Ctrl selects Image sampling for Paint
  Bucket and Color Picker; Eraser fill pattern. Renames: "Move Selected Pixels" →
  **Move Pixels** and "Copy Merged" → **Copy Flattened**. Also new: Nearest
  Neighbor (Antialiased) for Move Pixels, PNG tEXt/iTXt metadata preservation, an
  Uzbek translation, .NET 11, experimental Wine (Linux/Mac).
- **6.0 (announced in the 5.2 post)**: a new .PDN format; first up are high bit-depth
  formats, new blend modes and layer folders, then adjustment layers, text layers and HDR.

---

## Tools

Docs: [ToolsWindow](https://www.getpaint.net/doc/latest/ToolsWindow.html),
[Toolbar](https://www.getpaint.net/doc/latest/Toolbar.html),
[KeyboardMouseCommands](https://www.getpaint.net/doc/latest/KeyboardMouseCommands.html),
plus each tool page that is linked in the table.

Hoshi currently exposes one drawing tool (Shapes → Rectangle). The controller
`Tool` enum has `Rectangle`, `MoveShape`, `Pan` and `Eyedropper`. The desktop
does not use `Tool::Eyedropper` yet.

| Feature | Paint.NET notes | Hoshi status | Issue/notes |
| --- | --- | --- | --- |
| Tools window | Icon grid, F5 toggle, Ctrl+Shift+F5 reset; tool drop-down Alt+T | 🟡 | Icon rail with Shapes only |
| Tool keys | S (cycles Rect/Lasso/Ellipse/Wand), M, Z, H, F, G, B, E, P, K, L, R, T, O (Line/Curve, then Shapes) | ❌ | |
| [Rectangle Select](https://www.getpaint.net/doc/latest/RectangleSelectionTool.html) | Normal / Fixed Ratio / Fixed Size; Shift = square | ❌ | Roadmap 0.1.0 |
| [Ellipse Select](https://www.getpaint.net/doc/latest/EllipseSelectionTool.html) | Shift = circle | ❌ | Roadmap 0.1.0 |
| [Lasso Select](https://www.getpaint.net/doc/latest/LassoSelectionTool.html) | Freeform outline | ❌ | Roadmap 0.1.0 |
| [Magic Wand](https://www.getpaint.net/doc/latest/MagicWand.html) | Tolerance; Contiguous/Global flood (Shift = global); Layer/Image sampling | ❌ | Roadmap 0.1.0 |
| [Move Pixels](https://www.getpaint.net/doc/latest/MoveTools.html) (renamed 5.2) | Move/resize/rotate pixels, Ctrl = copy; resampling incl. Nearest Neighbor (Antialiased) (5.2) | ❌ | Roadmap 0.1.0; shapes have their own move/rotate |
| [Move Selection](https://www.getpaint.net/doc/latest/MoveTools.html) | Edits the outline only | ❌ | Roadmap 0.1.0 |
| [Zoom tool](https://www.getpaint.net/doc/latest/ViewTools.html) | Left click in, right click out | 🟡 | Zoom buttons, wheel zoom at pointer, Fit and reset; no Zoom tool |
| [Pan tool](https://www.getpaint.net/doc/latest/ViewTools.html) | H; Space+drag from any tool; middle drag | 🟡 | Middle-drag and Space+drag; no H tool |
| [Paint Bucket](https://www.getpaint.net/doc/latest/PaintBucket.html) | Tolerance, flood mode, Layer/Image sampling, Ctrl = Image (5.2); fill style, AA, blend mode | ❌ | Roadmap 0.2.0 |
| [Gradient](https://www.getpaint.net/doc/latest/GradientTool.html) | 7 types: Linear, Linear (Reflected), Diamond, Radial, Conical, Spiral (Clockwise), Spiral (Anticlockwise); Color/Transparency mode; repeat modes; editable nubs | 📋 | #24 (multi-stop gradient, enhancement) |
| [Paintbrush](https://www.getpaint.net/doc/latest/Paintbrush.html) | Decimal size, pressure (5.0), hardness, spacing (5.0), smoothing (5.0), fill style, AA, blend mode | 📋 | #6 → #42, #43, #44, #47 |
| [Eraser](https://www.getpaint.net/doc/latest/EraserTool.html) | Size, pressure, hardness; fill pattern (5.2) | 📋 | #6 → #42–#44, #47 (icons: #46) |
| [Pencil](https://www.getpaint.net/doc/latest/Pencil.html) | 1-px aliased; blend mode | ❌ | Not in an issue |
| [Color Picker](https://www.getpaint.net/doc/latest/ColorPicker.html) | Layer/Image sampling (Ctrl = Image, 5.2); six sampling sizes; after click: stay / previous tool / Pencil | 📋 | #48, #47. Owner decision: samples the **canvas** (parity) |
| [Clone Stamp](https://www.getpaint.net/doc/latest/CloneStamp.html) | Ctrl+click source; size, pressure, hardness, spacing, smoothing, AA, blend mode | ❌ | Not in an issue |
| [Recolor](https://www.getpaint.net/doc/latest/RecolorTool.html) | Replaces Secondary with Primary; tolerance, sampling, brush options | ❌ | Not in an issue |
| [Text](https://www.getpaint.net/doc/latest/TextTool.html) | Font, size (points or Fixed 96 DPI), Bold/Italic/Underline/Strikeout, rendering mode, justification, AA; movable until commit | ❌ | Roadmap 0.7.0 |
| [Line/Curve](https://www.getpaint.net/doc/latest/LineCurveTool.html) | Straight (5.0) / Cubic Spline / Bézier; start/end caps; line and fill styles; AA; blend; right-drag rotate | 📋 | #8 (line) |
| [Shapes](https://www.getpaint.net/doc/latest/ShapeTools.html) | Presets: rectangle, rounded rectangle, ellipse, diamond, trapezoid, parallelogram, triangles, polygons/stars, arrows, callouts, symbols (lightning, check, multiply, gear, heart); A / Shift+A cycle | 🟡 | Rectangle exposed. Ellipse exists in core but has no UI. Rounded rect: #8. Others: roadmap 0.2.0 |
| Shape: draw mode | Outline / Fill / Outline+Fill, Primary/Secondary roles | ✅ | Roles follow the mouse button |
| Shape: brush width | Decimal; `[` `]`, Ctrl = ±5 | ✅ | 0.01–65,536 px; same keys |
| Shape: line style | Dash styles in the toolbar | 🟡 | Solid, Dash, Dot, Dash-dot, Dash-dot-dot; no caps |
| Shape: fill style | Hatch patterns with both colors | 🟡 | 8 procedural patterns; full catalog missing |
| Shape: handles, move, rotate | Nubs; Shift aspect, Alt center; drag inside; right-drag rotates, Shift = 15°; arrows ±1, Ctrl ±10 | ✅ | All of these, plus a movable pivot and numeric X/Y/W/H |
| Toolbar: Antialiasing | Split button per tool | ✅ | Shapes only |
| Toolbar: tool blend mode | Layer modes plus Overwrite | ❌ | SHAPE_RENDERING.md remaining work |
| Toolbar: Finish | Enter commits | ✅ | Enter or the commit icon |

## Selection

Docs: [SelectionTools](https://www.getpaint.net/doc/latest/SelectionTools.html),
[EditMenu](https://www.getpaint.net/doc/latest/EditMenu.html),
[KeyboardMouseCommands](https://www.getpaint.net/doc/latest/KeyboardMouseCommands.html).

| Feature | Paint.NET notes | Hoshi status | Issue/notes |
| --- | --- | --- | --- |
| Rect / ellipse / lasso / wand selections | See Tools | ❌ | Roadmap 0.1.0 |
| Selection modes | Replace, Add (union), Subtract, Intersect, Invert; Ctrl/Alt with left or right drag | ❌ | |
| Selection clipping | Pixelated / Antialiased | ❌ | |
| Select All / Deselect | Ctrl+A / Ctrl+D (or Enter) | ❌ | |
| Invert Selection | Ctrl+I | ❌ | |
| Erase Selection / Fill Selection | Delete / Backspace | ❌ | |
| Copy Selection / Paste Selection | Ctrl+Alt+Shift+C / V; polygon JSON; Paste combine modes | ❌ | |
| Crop to Selection | Image menu, Ctrl+Shift+X | ❌ | Menu item present but disabled |
| Zoom to Selection | Ctrl+Shift+B | ❌ | |
| Selection-limited adjustments/effects | Applies only inside the selection | ❌ | |

## Edit & clipboard

Docs: [EditMenu](https://www.getpaint.net/doc/latest/EditMenu.html),
[KeyboardMouseCommands](https://www.getpaint.net/doc/latest/KeyboardMouseCommands.html).

| Feature | Paint.NET notes | Hoshi status | Issue/notes |
| --- | --- | --- | --- |
| Edit menu | Present | ❌ | Hoshi menus: File, Image, Help |
| Undo / Redo | Ctrl+Z / Ctrl+Y | ✅ | Plus toolbar and History. **Note:** Paint.NET binds Ctrl+Shift+Z to Layers → Rotate / Zoom, not Redo. Hoshi follows Paint.NET: Ctrl+Shift+Z is reserved for Rotate / Zoom (#41) |
| Cut / Copy / Paste | Ctrl+X/C/V; Copy with no selection copies the layer; Paste selects and activates Move Pixels | ❌ | Roadmap 0.1.0 |
| Copy Flattened (renamed 5.2) | Ctrl+Shift+C, all layers | ❌ | |
| Paste into New Layer | Ctrl+Shift+V | ❌ | |
| Paste into New Image | Ctrl+Alt+V, sized to the clipboard | ❌ | |
| Paste canvas-resize prompt | Offered when the paste is larger than the canvas | ❌ | |
| New image sized from clipboard | File → New defaults to the clipboard image size | ❌ | IMAGE_MENU.md "clipboard-based defaults" |

## View

Docs: [ViewMenu](https://www.getpaint.net/doc/latest/ViewMenu.html),
[ViewTools](https://www.getpaint.net/doc/latest/ViewTools.html),
[StatusBar](https://www.getpaint.net/doc/latest/StatusBar.html),
[ImageList](https://www.getpaint.net/doc/latest/ImageList.html),
[SettingsDialog](https://www.getpaint.net/doc/latest/SettingsDialog.html).

| Feature | Paint.NET notes | Hoshi status | Issue/notes |
| --- | --- | --- | --- |
| View menu | Present | ❌ | Toolbar icons only |
| Zoom In / Out | Ctrl++ / Ctrl+−, Ctrl+wheel; fixed steps up to 6400% | 🟡 | Buttons and plain wheel at the pointer (×1.2); no keyboard shortcuts |
| Zoom to Window | Ctrl+B (pressing twice restores) | 🟡 | Fit button; no shortcut |
| Actual Size | Ctrl+0 | 🟡 | Reset-view button; no shortcut |
| Status-bar zoom slider/box | Slider and percent box | 🟡 | Zoom label only |
| Pixel Grid | View menu and toolbar; shown at ≥200% | ❌ | |
| Rulers | Highlights the selection bounds and cursor | ❌ | |
| Units | Pixels / Inches / Centimeters (View menu or status bar) | 🟡 | Only in the New/Resize dialogs |
| Keyboard scrolling | PgUp/PgDn, Home/End, Space+arrows | ❌ | |
| Canvas customization (5.1) | Settings → Canvas: drop shadow, border color, checkerboard brightness | ❌ | |
| Image list (tabs) | Live thumbnails, drag reorder, unsaved asterisk, context menu | 🟡 | Thumbnails, dirty marker, close, tooltip; no drag reorder |
| Image switching | Ctrl+Tab / Ctrl+PgUp/PgDn | 🟡 | Ctrl+Tab and Ctrl+Shift+Tab |
| Center new/opened image | Centered in the window | 📋 | #31 |
| High-DPI-correct input | | ✅ | Physical-to-document mapping uses the scale factor |

## Image menu

Docs: [ImageMenu](https://www.getpaint.net/doc/latest/ImageMenu.html),
[KeyboardMouseCommands](https://www.getpaint.net/doc/latest/KeyboardMouseCommands.html).

| Feature | Paint.NET notes | Hoshi status | Issue/notes |
| --- | --- | --- | --- |
| Crop to Selection | Ctrl+Shift+X | ❌ | Disabled until selections exist |
| Resize | Ctrl+R; percent or absolute; aspect lock; resolution; print size | ✅ | [IMAGE_MENU.md](IMAGE_MENU.md) |
| Resampling | Bicubic (default), Bicubic (Smooth), Bilinear, Bilinear (Low Quality), Adaptive (Sharp), Lanczos, Fant, Nearest Neighbor; gamma correction | 🟡 | All 8 plus gamma. No golden-image comparison yet. Hoshi's own Adaptive (Sharp) policy |
| Canvas Size | Ctrl+Shift+R; anchor; Fill: Transparent, Primary, Secondary, White, Black | 🟡 | Anchor works; fill is Transparent or Secondary only |
| Flip Horizontal / Vertical | | ✅ | |
| Rotate 90° CW / 90° CCW / 180° | Ctrl+H / Ctrl+G / pressing either twice | 🟡 | Commands done; no shortcuts |
| Color Profile (5.1) | Assign/Convert; sRGB, Adobe RGB, Display P3, ProPhoto RGB, .icc/.icm import/export | ❌ | sRGB only |
| Flatten | Ctrl+Shift+F | ✅ | |
| Resolution units | Per the New/Resize dialogs | 🟡 | Pixels/inch only (physical sizes accept cm) |

## Layers (incl. blend modes)

Docs: [LayersMenu](https://www.getpaint.net/doc/latest/LayersMenu.html),
[LayersWindow](https://www.getpaint.net/doc/latest/LayersWindow.html),
[WorkingWithLayers](https://www.getpaint.net/doc/latest/WorkingWithLayers.html),
[BlendModes](https://www.getpaint.net/doc/latest/BlendModes.html),
[RotateZoom](https://www.getpaint.net/doc/latest/RotateZoom.html).

`hoshi-core` already implements add/delete/rename/reorder/visibility/opacity with
undo (max 32 layers). The desktop only exposes renaming the active layer.

| Feature | Paint.NET notes | Hoshi status | Issue/notes |
| --- | --- | --- | --- |
| Layers window | F7; thumbnails, visibility checkboxes, drag reorder | 📋 | #7 |
| Add New Layer | Ctrl+Shift+N; transparent, above the active layer | 📋 | #7 (core done) |
| Delete Layer | Ctrl+Shift+Delete; the last layer can't be deleted | 📋 | #7 (core done) |
| Duplicate Layer | Ctrl+Shift+D | ❌ | Not in core or #7 |
| Merge Layer Down | Ctrl+M | ❌ | |
| Toggle Layer Visibility | Ctrl+, | 📋 | #7 (core done) |
| Import From File | One or more files as layers; grows the canvas | ❌ | |
| Layer Flip H/V, Rotate 180° | Active layer only | ❌ | Image-level only |
| Rotate / Zoom | Ctrl+Shift+Z; roll/tilt, pan, zoom, quality, tiling (Transparent/Wrap/Mirror), sampling | ❌ | |
| Go to Top/Above/Below/Bottom | Alt/Ctrl+Alt+PgUp/PgDn | ❌ | |
| Move Layer Up/Down/Top/Bottom | Alt+Shift / Ctrl+Alt+Shift + PgUp/PgDn | 📋 | #7 (core `move_layer`) |
| Layer Properties | F4 or double-click: name, visible, blend mode, opacity | 🟡 | Footer rename only; dialog: #7 |
| Layer opacity | Slider in Properties | 📋 | #7 (core 0–255) |
| Blend modes (14) | Normal, Multiply, Additive, Color Burn, Color Dodge, Reflect, Glow, Overlay, Difference, Negation, Lighten, Darken, Screen, Xor | ❌ | Source-over only; roadmap 0.7.0 |
| Layer count | Limited by memory | 🟡 | Hard cap of 32 |
| FP32 layer rendering (5.2) | Avoids multi-layer rounding artifacts | ❌ | RGBA8 compositing |
| Layer folders, new blend modes (6.0 plan) | Announced, not shipped | ❌ | Paint.NET hasn't shipped these either |

## Adjustments

Docs: [AdjustmentsMenu](https://www.getpaint.net/doc/latest/AdjustmentsMenu.html),
[Curves](https://www.getpaint.net/doc/latest/Curves.html),
[Levels](https://www.getpaint.net/doc/latest/Levels.html).

Hoshi has no Adjustments menu and no shared live preview. Brightness/contrast,
hue/saturation and levels are scoped for roadmap 0.3.0, which has no issues yet.

| Feature | Paint.NET notes | Hoshi status | Issue/notes |
| --- | --- | --- | --- |
| Live preview + selection clipping | Dialog preview | ❌ | Roadmap 0.3.0 |
| Auto-Level | Ctrl+Shift+L; no dialog; equivalent to Levels → Auto | ❌ | |
| Black and White | Ctrl+Shift+G; no dialog | ❌ | |
| Brightness / Contrast | Ctrl+Shift+T | ❌ | Roadmap 0.3.0 |
| Curves | Ctrl+Shift+M; Luminosity or RGB | ❌ | |
| Exposure (5.0) | | ❌ | |
| Highlights / Shadows (5.0) | | ❌ | |
| Hue / Saturation | Ctrl+Shift+U; hue, saturation, lightness | ❌ | Roadmap 0.3.0 |
| Invert Alpha | Ctrl+Alt+I | ❌ | |
| Invert Colors | Ctrl+Shift+I | ❌ | |
| Levels | Ctrl+L; per-channel input/output/gamma, Auto | ❌ | Roadmap 0.3.0 |
| Posterize | Ctrl+Shift+P; 2–64 levels; alpha (5.0) | ❌ | |
| Sepia | Ctrl+Shift+E; intensity (5.0) | ❌ | |
| Temperature and Tint (5.0) | | ❌ | |

## Effects

Docs: [EffectsMenu](https://www.getpaint.net/doc/latest/EffectsMenu.html): nine
submenus (Artistic, Blurs, Color, Distort, Noise, Object, Photo, Render,
Stylize). **Hoshi has no effects.** Blur and sharpen are scoped for roadmap
0.3.0, and external effects through Wasm for 0.4.0. None has an issue yet.

| Feature | Paint.NET notes | Hoshi status | Issue/notes |
| --- | --- | --- | --- |
| Repeat last effect | Ctrl+F; same parameters, no dialog | ❌ | |
| GPU effect rendering | All but Quantize and Red Eye Removal (5.1) | ❌ | CPU-only core |

### Artistic

Doc: [EffectsArtisticMenu](https://www.getpaint.net/doc/latest/EffectsArtisticMenu.html).

| Feature | Paint.NET notes | Hoshi status | Issue/notes |
| --- | --- | --- | --- |
| Ink Sketch | Ink outline, coloring | ❌ | |
| Linocut (5.2) | Parallel / Contour / Cross-hatch | ❌ | |
| Mosaic (5.2) | 10 tile shapes, grout, bevel, lighting | ❌ | |
| Oil Painting | | ❌ | |
| Pencil Sketch | | ❌ | |
| Pointillism (5.2) | | ❌ | |
| Stained Glass (5.2) | | ❌ | |

### Blurs

Doc: [EffectsBlursMenu](https://www.getpaint.net/doc/latest/EffectsBlursMenu.html) (ten effects; Bokeh replaced Unfocus in 5.0).

| Feature | Paint.NET notes | Hoshi status | Issue/notes |
| --- | --- | --- | --- |
| Bokeh Blur (5.0) | Radius, gamma, quality | ❌ | |
| Fragment Blur | | ❌ | |
| Gaussian Blur | Radius in 0.1 px steps; gamma boost; quality 1–4 | ❌ | Roadmap 0.3.0 |
| Median Blur (5.1) | Formerly Noise → Median | ❌ | |
| Motion Blur | Gaussian kernel (5.1) | ❌ | |
| Radial Blur | | ❌ | |
| Sketch Blur (5.1) | | ❌ | |
| Square Blur (5.1) | | ❌ | |
| Surface Blur | | ❌ | |
| Zoom Blur | | ❌ | |

### Color

Doc: [EffectsColorMenu](https://www.getpaint.net/doc/latest/EffectsColorMenu.html).

| Feature | Paint.NET notes | Hoshi status | Issue/notes |
| --- | --- | --- | --- |
| Ordered Dither (5.2) | Bayer, spiral, dual spiral, blue/white noise | ❌ | |
| Quantize / Dither | Median Cut or Octree; ≤256 colors; nine dither levels | ❌ | |

### Distort

Doc: [EffectsDistortMenu](https://www.getpaint.net/doc/latest/EffectsDistortMenu.html). The page intro still says "nine"; 11 are documented.

| Feature | Paint.NET notes | Hoshi status | Issue/notes |
| --- | --- | --- | --- |
| Bulge | Edge behavior: Clamp/Wrap/Mirror/Transparent | ❌ | |
| Crystalize | | ❌ | |
| Dents | | ❌ | |
| Frosted Glass | Diffusion, Randomize | ❌ | |
| Morphology | | ❌ | |
| Pixelate | Scale-down/up modes | ❌ | |
| Polar Inversion | | ❌ | |
| Spherize (5.2) | | ❌ | |
| Tile Reflection | Edge behavior | ❌ | |
| Twist | | ❌ | |
| Waves (5.2) | Waves, ripples, spirals, flowers | ❌ | |

### Noise

Doc: [EffectsNoiseMenu](https://www.getpaint.net/doc/latest/EffectsNoiseMenu.html).

| Feature | Paint.NET notes | Hoshi status | Issue/notes |
| --- | --- | --- | --- |
| Add Noise | Randomize button | ❌ | |
| Reduce Noise | | ❌ | |

### Object

Doc: [EffectsObjectMenu](https://www.getpaint.net/doc/latest/EffectsObjectMenu.html). Works on pixels surrounded by transparency.

| Feature | Paint.NET notes | Hoshi status | Issue/notes |
| --- | --- | --- | --- |
| Drop Shadow (5.0) | Radius, distance, angle, opacity, color, shadow-only | ❌ | |

### Photo

Doc: [EffectsPhotoMenu](https://www.getpaint.net/doc/latest/EffectsPhotoMenu.html). The intro says "five"; six are documented.

| Feature | Paint.NET notes | Hoshi status | Issue/notes |
| --- | --- | --- | --- |
| Glow | | ❌ | |
| Red Eye Removal | | ❌ | |
| Sharpen | Threshold (5.0) | ❌ | Roadmap 0.3.0 |
| Soften Portrait | | ❌ | |
| Straighten (5.0) | ±45°, auto-scale, sampling | ❌ | |
| Vignette | | ❌ | |

### Render

Doc: [EffectsRenderMenu](https://www.getpaint.net/doc/latest/EffectsRenderMenu.html).

| Feature | Paint.NET notes | Hoshi status | Issue/notes |
| --- | --- | --- | --- |
| Clouds | Blend modes; Colors tab (5.1) | ❌ | |
| Julia Fractal | Blend mode (5.0) | ❌ | |
| Mandelbrot Fractal | Blend mode (5.0) | ❌ | |
| Turbulence | Blend mode | ❌ | |

### Stylize

Doc: [EffectsStylizeMenu](https://www.getpaint.net/doc/latest/EffectsStylizeMenu.html). The page has a second "Emboss" heading whose text describes an outline effect, which looks like a doc error.

| Feature | Paint.NET notes | Hoshi status | Issue/notes |
| --- | --- | --- | --- |
| Edge Detect | Sobel/Prewitt, strength, blurring (5.0) | ❌ | |
| Emboss | Angle | ❌ | |
| Halftone (5.2) | CMYK or B/W dots | ❌ | |
| Outline | Radius | ❌ | |
| Relief | | ❌ | |

## Colors & palettes

Docs: [ColorsWindow](https://www.getpaint.net/doc/latest/ColorsWindow.html),
[WorkingWithPalettes](https://www.getpaint.net/doc/latest/WorkingWithPalettes.html).

| Feature | Paint.NET notes | Hoshi status | Issue/notes |
| --- | --- | --- | --- |
| Colors window | F8; minimized/expanded (More/Less) | ✅ | Float or dock on any edge |
| Primary / Secondary | Active slot; left/right click roles | ✅ | |
| Swap colors | X | ✅ | Canvas focus only |
| Switch active slot | C | 🟡 | Selectable in the panel; no C key |
| Reset to black/white | Not documented as a key | ✅ | Hoshi uses D (a Hoshi convention) |
| Color wheel | Left = active slot, right = inactive; Ctrl/Alt/Shift modifiers | 🟡 | Saturation-value field and hue slider; no wheel |
| RGB / HSV / Opacity sliders | | ✅ | |
| Hex entry | Editable; pasted values accepted | ✅ | Hex RGB or RGBA |
| 96-color palette | Default palette | ✅ | 96 entries; Hoshi's own default colors |
| Palette files | Open palettes folder, reset to default, .txt palette files | ✅ | Paint.NET text format import/export, Save As, dirty state |
| Add color to palette | Add icon, then click a slot | ✅ | "Add active color" works the same way |
| Color Picker tool (canvas) | See Tools | 📋 | #48 |

## History

Docs: [HistoryWindow](https://www.getpaint.net/doc/latest/HistoryWindow.html),
[index (Unlimited History)](https://www.getpaint.net/doc/latest/index.html).

| Feature | Paint.NET notes | Hoshi status | Issue/notes |
| --- | --- | --- | --- |
| History window | F6; click a row to jump; undone rows grey | ✅ | Virtualized list; float/dock |
| Undo / Redo icons | In the window | ✅ | |
| Length limited by disk and memory | Per the features page | ✅ | [DISK_HISTORY.md](DISK_HISTORY.md) |
| A new edit after undo discards redo | | ✅ | |
| Undo to save point clears the asterisk | | ✅ | |
| One entry per committed edit | | ✅ | Shapes and image ops; strokes: #43 |
| Shortcuts Ctrl+Z / Ctrl+Y | | ✅ | Caps Lock bug: #41 |
| History robustness | | 🟡 | Remove `.expect` in `checkpoint`: #37 |

## File formats

Docs: [FileMenu](https://www.getpaint.net/doc/latest/FileMenu.html),
[index (formats)](https://www.getpaint.net/doc/latest/index.html).
Hoshi details: [DOCUMENT_MANAGEMENT.md](DOCUMENT_MANAGEMENT.md), [FILE_FORMAT.md](FILE_FORMAT.md).
The docs list the supported formats without saying which ones are open-only or save-only.
Per-format save support is therefore (unverified), except where Save Options names a format.

| Feature | Paint.NET notes | Hoshi status | Issue/notes |
| --- | --- | --- | --- |
| New | Default 800×600 scaled by DPI, or the clipboard size; aspect lock; resolution; print size | 🟡 | Units, resolution, aspect lock and background done; no clipboard default |
| Open / Open Recent | Ctrl+O; open from a URL | 🟡 | Multi-select open; 10 recent files; no URL |
| Save / Save As / Save All | Ctrl+S / Ctrl+Shift+S / Ctrl+Alt+S; Save Options dialog with Auto-detect bit depth | 🟡 | Atomic writes; JPEG quality; no bit-depth options |
| Close / Exit prompts | Ctrl+W; unsaved-changes prompt | ✅ | Save / Don't Save / Cancel |
| Print | Ctrl+P | ❌ | |
| Acquire | From a scanner | ❌ | |
| Native layered format | `.pdn`, lossless; new format planned for 6.0 | 🟡 | Own `.hoshi` (V2 sparse tiles). `.pdn` not supported. Spec hardening: #9 |
| PNG | Listed; Save Options | 🟡 | Open/save with density. Decode limits and acceptance: #10 |
| JPEG (.jpg .jpeg .jpe .jfif .exif) | Listed; Save Options | ✅ | Quality 1–100; flattens on white |
| BMP (.bmp .dib .rle) | Listed; Save Options | 🟡 | Open only |
| GIF | Listed; Save Options | 🟡 | Open only (first frame) |
| TGA | Listed; Save Options | 🟡 | Open only |
| TIFF | Listed | 🟡 | Open only (first page) |
| WebP | Listed; max 16383 px | 🟡 | Open only |
| DDS | Listed; DDS FileType Plus bundled (5.2) | ❌ | |
| HEIC/HEIF | Requires the Microsoft HEVC extension | ❌ | |
| AVIF | Listed; max 65536 px | ❌ | |
| JPEG XL | Listed | ❌ | |
| JPEG XR (.jxr .wdp .wmp) | Listed | ❌ | |
| Metadata preservation | EXIF/XMP/IPTC; PNG tEXt/iTXt (5.2) | ❌ | Not preserved on save |
| Embedded color profiles (5.1) | Applied in real time | ❌ | ICC images are rejected on open |
| CICP / HDR tone mapping (5.2) | AVIF, JPEG XL, PNG (CICP); also HEIC, JPEG XR (HDR) | ❌ | Channels above 8 bits are rejected |
| CMYK import | CMYK32/40/64/80 (5.1–5.2) | ❌ | |
| Command-line open | `paintdotnet:` protocol | ✅ | Paths open as tabs |
| Drag-and-drop open, file associations | Not covered by the fetched pages | ❌ | DOCUMENT_MANAGEMENT.md future work |

## Settings, shortcuts, plugins

Docs: [SettingsDialog](https://www.getpaint.net/doc/latest/SettingsDialog.html),
[KeyboardMouseCommands](https://www.getpaint.net/doc/latest/KeyboardMouseCommands.html),
[Translations](https://www.getpaint.net/doc/latest/Translations.html),
[InstallPlugins](https://www.getpaint.net/doc/latest/InstallPlugins.html).

| Feature | Paint.NET notes | Hoshi status | Issue/notes |
| --- | --- | --- | --- |
| Settings dialog | Tabs: User Interface, Canvas, Tools, Pen & Tablet, Graphics, Color Management, Updates, Plugin Errors, Diagnostics | ❌ | Only config files (recent files, palettes) |
| User Interface options | Tab exists; individual options not listed in the doc text (unverified) | ❌ | |
| Languages | 37 languages listed; Uzbek added in 5.2 | 🟡 | en-US and es-ES; migration: #32, #39, #45 |
| Regional number/unit formatting | | 📋 | #32 |
| Tool defaults | Settings → Tools: Load from Toolbar, Reset | ❌ | Shape defaults last for the session only |
| Updates | Settings → Updates → Check Now | ❌ | Packaging: #13 |
| File shortcuts | Ctrl+N/O/S/Shift+S/Alt+S/W/P | 🟡 | All except Print; adds Ctrl+Shift+W (Close All) |
| Image shortcuts | Ctrl+R, Ctrl+Shift+R, Ctrl+Shift+F, Ctrl+H/G, Ctrl+Shift+X | 🟡 | Resize, Canvas Size and Flatten; canvas focus only |
| Layer/Adjustment/Effect shortcuts | See those sections | ❌ | |
| Window toggles | F5–F8; Ctrl+Shift+F5–F8 reset | ❌ | Toolbar buttons only |
| Brush width `[` `]` | Ctrl = ±5 | ✅ | Shape outline width |
| Help | F1 opens the online docs | ❌ | About dialog only |
| Global shortcut routing | Shortcuts work app-wide | 🟡 | Canvas focus required; floating panels get file keys only. #11 |
| Effect plugins | .dll in Documents\Paint.NET App Files\Effects; GPU system (5.0); classic system deprecated (5.2) | ❌ | Not binary compatible. Wasm effect API: roadmap 0.4.0 |
| FileType plugins | New versioned system (5.2); legacy system being deprecated | ❌ | No format plugin plan yet |
| Shape plugins | App Files\Shapes folder | ❌ | |
| Plugin SDK | Paint.NET.Plugin.SDK on NuGet (5.2) | ❌ | |

## Platform

Sources: [GitHub releases](https://github.com/paintdotnet/release/releases),
[SystemRequirements](https://www.getpaint.net/doc/latest/SystemRequirements.html),
[SettingsDialog](https://www.getpaint.net/doc/latest/SettingsDialog.html), and the release posts.

| Feature | Paint.NET notes | Hoshi status | Issue/notes |
| --- | --- | --- | --- |
| OS support | Windows 10 21H2+/11/Server 2022; x64 (AVX2; SSE4.2 required) or ARM64; experimental Wine package (5.2) | ✅ | Windows, macOS and Linux on CI. Packaging: #13 |
| Pen pressure (5.0) | Windows Ink; Settings → Pen & Tablet | ❌ | Roadmap 0.2.0 |
| Pointer capture / focus loss | | 📋 | #44: commits the active stroke |
| High DPI | New-image size scales with system DPI | ✅ | Logical/physical mapping; viewport texture bounded at HiDPI |
| Color management (5.1) | Windows Advanced Color on HDR/WCG; sRGB mode on SDR | ❌ | sRGB only; ICC rejected |
| HDR editing | Tone-mapped on load (5.2); native editing planned | ❌ | |
| GPU acceleration | Direct3D 11; Settings → Graphics | 🟡 | Slint renderer; pixel work on the CPU |
| Max canvas | 262,144 px per axis ([LARGE_IMAGES.md](LARGE_IMAGES.md) source) | ✅ | Sparse tiles |
| Performance budgets | | 📋 | #12 |
| Dirty tracking / prompts | Asterisk and unsaved prompt | 🟡 | Done; keyboard routing: #11 |
| iPad / iOS | Not offered | ❌ | Roadmap 0.5.0, after desktop parity |
| Accessibility | High-contrast canvas border (5.1) | 🟡 | Accessible names and labels; full review is roadmap 0.8.0 |

---

## Hoshi-only extras

| Feature | Notes |
| --- | --- |
| Screen Picker | Samples any pixel on the OS screen with the native picker (macOS `NSColorSampler`; Linux XDG portal PickColor on GNOME/KDE; Windows native). The result is opaque sRGB. This is separate from the canvas Color Picker. Tool rail: #47, specification/tests: #48 |
| Cross-platform | One core/controller on Windows, macOS and Linux. Paint.NET is Windows-only, apart from its experimental Wine package. iPad is planned after desktop parity |
| 262,144 px sparse canvases | 128×128 copy-on-write tiles, so a blank maximum-size image stores no pixel array. Global memory budget |
| Disk-backed unlimited undo | Compressed, checksummed, shared tiles; distant History rows load directly |
| Localization | Runtime locale files (en-US, es-ES); #32 |
| Shape pivot and numeric bounds | Movable rotation pivot; X/Y/W/H fields with ratio lock. Paint.NET's documented shape editing has nub/drag/rotate but no numeric fields |
| Dockable panels | Colors, History and Shape Properties float or dock on any edge without losing state |
| Zoom down to 1/4096 | Fits maximum-size images in small viewports |
| ICO open | Not in Paint.NET's built-in format list |

## Summary

Counts are per table row in each area. Hoshi-only extras are not counted.

| Area | ✅ | 🟡 | 📋 | ❌ | Rows |
| --- | --- | --- | --- | --- | --- |
| Tools | 5 | 6 | 5 | 13 | 29 |
| Selection | 0 | 0 | 0 | 10 | 10 |
| Edit & clipboard | 1 | 0 | 0 | 7 | 8 |
| View | 1 | 7 | 1 | 5 | 14 |
| Image menu | 3 | 4 | 0 | 2 | 9 |
| Layers (incl. blend modes) | 0 | 2 | 6 | 9 | 17 |
| Adjustments | 0 | 0 | 0 | 14 | 14 |
| Effects | 0 | 0 | 0 | 50 | 50 |
| Colors & palettes | 9 | 2 | 1 | 0 | 12 |
| History | 7 | 1 | 0 | 0 | 8 |
| File formats | 3 | 10 | 0 | 12 | 25 |
| Settings, shortcuts, plugins | 1 | 4 | 1 | 11 | 17 |
| Platform | 3 | 3 | 2 | 4 | 12 |
| **Total** | **33** | **39** | **16** | **137** | **225** |

The largest gaps are selections, clipboard, painting tools (in progress under
#6), the layer UI (#7), and all adjustments and effects. File I/O, the Image
menu, history and the Colors panel are the most complete areas.
