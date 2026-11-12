# Shape rendering

Rendering settings apply immediately to the active shape preview and become defaults for the next shape. Changing a setting creates no history entry. Enter commits the final raster result as one undoable action. Saved documents contain those pixels; previews and tool defaults are not saved as vector objects.

Open **Shape Properties** using the Shapes tool or sliders icon. It can float or dock on any edge; see [tool panels](TOOL_PANELS.md).

## Controls and color roles

- Antialiasing is on by default and can be disabled for hard pixel edges.
- Outline width accepts decimals from 0.01 to 65,536 canvas pixels. Outlines remain inset inside the bounds. A sufficiently wide outline consumes the fill.
- Line styles: Solid, Dash, Dot, Dash dot, Dash dot dot. Dash lengths scale with outline width and are measured along the stroke centerline for every shape.
- Fill patterns: Solid, Horizontal, Vertical, Forward diagonal, Backward diagonal, Cross, Diagonal cross, Checkerboard.
- `[` / `]` change width by one pixel; Ctrl/Cmd changes the step to five. These shortcuts operate with canvas focus; text fields retain ordinary typing.

Fill-only and outline-only use Primary for left-button creation, Secondary for right-button creation. With both enabled, left-button shapes use Primary outlines and Secondary fills; right-button shapes reverse the roles. Patterned fills use the other workspace color for their background. Both color slots, including alpha, update the editable shape live. Gaps in a dashed outline show the fill in Both mode and remain transparent in Outline mode.

Patterns use an eight-pixel local tile (checkerboard uses four-pixel squares). They translate, rotate and mirror with the shape. This is a procedural starter palette, not a claim that all Paint.NET hatch presets have shipped.

## Implementation

`hoshi-core::rendering` owns the shape paint model, dash/pattern sampling and prepared rasterizer. Shape dimensions and editing transforms remain separate in `geometry`. `hoshi-controller::rendering` owns workspace settings and color-role mapping. The desktop's `shape-rendering.slint` component and `shape_rendering.rs` adapter present controls without implementing geometry or compositing. All platforms use these same modules.

Preview and commit use the same prepared rasterizer. Solid rectangle edges use exact polygon/pixel intersections, including rotated and fractional-width outlines. Other textured/curved boundaries use 4×4 supersampling. Samples are averaged in premultiplied encoded RGBA and unpremultiplied once, so translucent edges retain their hue. Interior samples bypass antialias work when a conservative boundary-distance test proves uniform coverage. Ellipse dashed strokes prepare an arc-length table once per operation rather than integrating the perimeter for every pixel.

Uniform full-canvas fills keep their constant-size representation. Patterned fills and dashed outlines cannot use solid-fill shortcuts merely because their corner colors match. Large axis-aligned outlines still visit border strips rather than their entire bounding-box interior. Memory-budget failures preserve the active preview and prior committed state.

## Paint.NET comparison

The [current Shapes tool documentation](https://www.getpaint.net/doc/latest/ShapeTools.html) describes decimal brush sizes, line styles, two-color fills and an antialiasing switch. These control categories are now available here. The rendering algorithms are our own implementation; exact pixel equivalence has not been established against Paint.NET reference images.

Remaining work includes the full hatch preset catalog, shape blend-mode selection, selection-boundary quality, additional shape tools and custom shape definitions. Curved/textured subpixel coverage uses a finite sample grid, so extremely thin strokes can lose coverage; solid rectangles use analytic coverage to avoid this limitation. Existing inset strokes also remain a deliberate model constraint pending reference-image comparison of stroke placement and joins.
