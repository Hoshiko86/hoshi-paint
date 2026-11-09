# Shared shape editing

Shapes remain adjustable previews until Enter or an outside click commits them to the raster layer. Escape discards the preview. A commit is one history entry; changing the pivot alone creates none. This remains a raster editor: committed shapes are pixels, recoverable through undo rather than persistent vector objects.

## Controls

- Eight handles resize. Shift preserves the drag-start aspect ratio, including edge handles. Alt fixes the center; Shift+Alt combines both. Crossing the opposite handle mirrors local geometry.
- Shift creates squares; Alt creates from the center. Modifiers can change during a drag without accumulating rounding errors.
- Drag inside the bounds to move. Arrow keys nudge one document pixel; Ctrl+arrow nudges ten. Text fields keep their normal arrow keys.
- Drag the rotation knob, just outside the bounds, or with the right button to rotate. Shift snaps mouse rotation to 15 degrees. Right-held arrows rotate; Shift increases the step to 15 degrees.
- Drag the pivot crosshair to choose the rotation center. It follows translation and stays fixed during rotation and resizing. A new shape starts with a centered pivot.
- Right-button creation reverses Primary/Secondary color roles without switching the selected color slot. Middle-drag and Space+left-drag pan.

## Code boundaries

`hoshi-core::geometry` owns points, frames, local/document transforms, resize constraints and rotation around a pivot. It knows no windows, events or shape kinds. Floating-point gesture geometry is quantized only when applied to the integer raster shape. Prepared inverse transforms cache trigonometry for pixel sampling.

`hoshi-controller::editing` owns gesture lifetimes, hit testing, pivot state, color roles and document mutations. Drag operations start from an immutable original frame so toggling modifiers does not cause cumulative drift. `input` defines platform-independent command values; `viewport` handles logical/document coordinates; `preview` handles bounded rendering.

The desktop `input` adapter translates Slint events into those shared commands. `ui/shape-controls.slint` draws the handles. Operating-system event details belong in adapters, not the geometry or document model. The same crates and UI are compiled on Windows, macOS and Linux.

## Adding shapes

Add the primitive's geometry and raster coverage in core, then its creation/tool selection in the UI. Select the kind with `Controller::set_shape_kind`; the existing frame handles, movement, scaling, pivot, rotation, keyboard commands and history lifecycle can be reused. Controller regression tests exercise an ellipse through this same path even though an ellipse tool is not exposed yet.

A line or polygon still needs its own creation rules and coverage calculation. Vertex editing, line endpoint controls and other shape-specific controls should be separate extensions beside shared frame editing. Mirror flags are retained even for symmetric rectangles/ellipses so future asymmetric primitives can use the same transform.

Integer bounds and whole-degree rotations remain current model constraints. Dash patterns, antialiasing controls, fractional stroke widths and additional shape tools are separate rendering/tool work; this change does not claim those features.
