# Image and canvas setup

The desktop spike exposes File → New and an Image menu patterned after Paint.NET.
This is an implementation milestone, not a claim of exact current Paint.NET parity.
The official documentation could not be fetched under the environment's network policy.

| Command | Implemented behavior |
| --- | --- |
| File → New… | Pixel/inch/centimeter dimensions, aspect lock, resolution in pixels/inch, white/transparent/Secondary background; replaces this single-document spike's current image and clears its history after Create |
| Resize… | Absolute dimensions or percentage of the original image, aspect lock, physical units, resolution, Nearest Neighbor/Bilinear/Bicubic/Lanczos 3; resamples every layer |
| Canvas Size… | Changes bounds without resampling; nine-point anchor, transparent padding or Secondary-color padding on the bottom layer, upper layers padded transparently; shrinking crops pixels |
| Flip Horizontal / Vertical | Flips all layers without changing layer names, visibility, opacity, or selection |
| Rotate 90° Clockwise / Counter-Clockwise / 180° | Rotates all layers; quarter-turns swap document dimensions |
| Flatten | Replaces the visible composite with one visible layer at 100% layer opacity, preserving actual pixel transparency; undo restores original layers and properties |
| Crop to Selection | Disabled: the spike has no selection tool yet; an active rectangle is not a selection |

Ctrl/Cmd+N opens New; Ctrl/Cmd+R opens Resize; Ctrl/Cmd+Shift+R opens Canvas Size;
Ctrl/Cmd+Shift+F flattens. These shortcuts currently apply while the canvas has focus.
Escape or Cancel dismisses settings without changing the document or committing an
active shape. Dialogs block drawing and history commands. Opening a dialog ends a
pointer gesture but retains the shape preview. Accepting settings validates first,
then commits a shape before the image edit. That produces a shape history step and
an image-edit history step. New clears history and preserves workspace colors.

Resolution is stored to 0.001 pixels/inch. Editing resolution preserves pixel size
and updates the physical-size display. Canvas Size preserves resolution. Dimensions,
resolution, all layer pixels and properties participate in undo/redo and History
navigation. Resampling uses premultiplied RGBA to avoid dark transparency fringes;
it operates in encoded RGB space. Image dimensions and the view are no longer fixed
to 512×512. The initial startup document remains 512×512 transparent.

## Remaining parity work

- Selection tools and crop-to-selection semantics, including nonrectangular masks.
- Additional Paint.NET resampling modes and pixel-level algorithm comparison.
- New in another document tab, clipboard-based defaults, save/dirty prompts and file I/O.
- Separate resolution unit selection (pixels/cm); physical width/height already support cm.
- Global menu shortcuts outside canvas focus and persisted dialog defaults.
- Larger documents and memory budgeting: this spike retains the 1,048,576-pixel limit,
  32-layer limit, and 16 retained history entries. Resolution is 1–100,000 pixels/inch.
- Verify detailed dialog defaults and canvas-padding semantics against a current Paint.NET installation.

## Validation

Core tests check exact flip/rotation pixel maps, every anchor's padding/cropping,
layer metadata, premultiplied resize behavior, flattening, resolution history and
validation without mutation. Controller tests cover preview commit boundaries and
workspace colors surviving New. A rendered UI test covers real keyboard shortcuts,
Escape, invalid sizes, unit conversion, percentage scaling, tiny high-resolution
physical sizes, and resized canvas display bounds. The native
`image_settings_smoke` example exercises all three dialogs and dimension history;
Linux CI includes its screenshots in the artifact.
