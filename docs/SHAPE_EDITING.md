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

Integer bounds and whole-degree rotations remain current model constraints. Shared paint settings now support fractional strokes, antialiasing, dash styles and two-color patterns; see [SHAPE_RENDERING.md](SHAPE_RENDERING.md) for algorithms and remaining parity work. Additional shape tools still need their primitive geometry and creation UI.

## Numeric position and dimensions

The Shape Properties panel provides X, Y, Width and Height for the active shape. Values are whole canvas pixels, independent of zoom and screen DPI. X/Y locate the top-left of the **unrotated** bounds, measured from canvas (0,0); negative positions and shapes partly outside the canvas are allowed. Width and height range from 1 to 262,144 pixels. Rotation, mirrored geometry and rendering styles stay intact.

Type a complete value and press Enter or leave the field (for example with Tab) to apply it to the preview. Enter inside a field applies that field, rather than committing the shape. Invalid or incomplete input leaves the preview unchanged and the field returns to its last valid value. Enter with canvas focus still commits the shape as one undoable action; Escape on the canvas cancels it. The fields are disabled when there is no active shape, so this is not editing previously committed pixels as a vector object.

**Lock ratio** links numeric width and height. The ratio is captured when the lock is enabled and kept across successive entries to avoid rounding drift. The dependent dimension rounds to the nearest whole pixel, with a minimum of one; values that would exceed the size limit are rejected together. A new mouse resize establishes a fresh ratio for later numeric edits. This toggle controls numeric entries; Shift continues to constrain mouse resizing.

Changing X or Y moves a custom rotation pivot by the same offset. Changing dimensions keeps the entered X/Y origin and any custom pivot fixed; the default pivot follows the shape center. Mouse editing and numeric editing use the same controller geometry. The implementation lives in `hoshi-controller::bounds`, with a reusable `shape-bounds.slint` view and a small Rust adapter. It can be reused for additional shape tools.

Shape Properties can float or dock on any edge, with scrolling when space is limited. See [TOOL_PANELS.md](TOOL_PANELS.md).
