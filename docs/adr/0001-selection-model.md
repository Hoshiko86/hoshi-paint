# 0001. Selection model

- Status: Proposed
- Date: 2026-11-12
- Related: #42 (CoverageMask), #43 (stroke lifecycle), ADR 0002, 0003, 0005

## Context

Roadmap 0.1.0 needs selections, pixel movement, clipboard and crop. Paint.NET
5.2 behaviour is summarized in Appendix A (all cited). The points that shape
the design: selections are polygon geometry (Copy Selection exports a JSON
polygon list); aliased vs antialiased is a toolbar **clipping mode**, not part of
the selection; selection changes appear in History; Move Pixels and Move
Selection rotate/scale/move freely. Hoshi has no selection yet. `State` holds
layers. The only previews are the active `Shape` and the planned (#43)
`ActiveStroke`, which uses a sparse `u8` `CoverageMask`.

## Decision

**1. Canonical form is vector geometry. Masks are derived caches.**

```rust
// hoshi-core/src/selection.rs — immutable, shared as Arc in State
pub struct Selection { geometry: PolygonSet /* f64 doc px, non-zero */, bounds: RectI }
pub enum CombineMode { Replace, Union, Exclude, Intersect, Xor } // UI: Add/Subtract/Invert
pub enum SelectionShape { Rect(RectF), Ellipse(Frame), Lasso(Vec<Point>), Pixels(PolygonSet) }
pub enum ClipQuality { Aliased, Antialiased }
impl Selection {
    pub fn combine(prev: Option<&Selection>, s: SelectionShape, m: CombineMode,
                   canvas: RectI) -> Result<Option<Selection>, Error>; // None = empty
    pub fn transformed(&self, t: &Affine) -> Result<Selection, Error>;
    pub fn mask(&self, q: ClipQuality) -> Result<MaskStore, Error>;  // budgeted
    pub fn to_polygon_list(&self) -> Vec<Vec<(f64, f64)>>;         // Copy Selection
}
```

- Boolean ops use **`i_overlay`** (pure Rust, `MIT OR Apache-2.0`; [lib.rs][iov])
  behind a private `polygon.rs` wrapper. Ellipses are flattened at 0.1 px.
- Magic Wand builds a `u8` flood mask (Contiguous/Global, tolerance, Layer/Image
  sampling, premultiplied/straight tolerance alpha). It then traces pixel-edge
  contours into a `PolygonSet` and reserves `edges × 16 B` before tracing.
- `None` = no selection = everything editable. Select All = canvas rect;
  Invert = `Xor` with the canvas rect; Deselect = `None`.

**2. One sparse mask type.** #42's `CoverageMask` becomes `pub(crate) MaskStore`
(`mask.rs`): 128×128 `u8` tiles plus an implicit `background: u8`, like
`PixelStore`. Aliased = pixel-centre test; Antialiased = area coverage via the
existing 4×4 supersampler. `Document` caches the mask keyed by
(selection generation, quality). The cache is not in `State`.

**3. One clipping function for preview and commit.** `clip(dst, edited, cov)
= lerp(dst, edited, cov)`. Brush coverage multiplies the selection coverage
and never adds to it. Every pixel writer (shape, stroke, fill, gradient,
effect, erase, paste) takes a `Clip { mask, bounds }`. Tiles outside `bounds`
or with zero coverage are skipped.

**4. Selection is in committed `State`; entries follow Paint.NET's History.**
`State.selection: Option<Arc<Selection>>`; `history_store` adds a checksummed
contour record. `.hoshi` does not store selections.

| Action | History |
| --- | --- |
| Rect/Ellipse/Lasso drag | one entry at pointer-up ("release … fixes the selection") |
| Magic Wand click | one entry. Tolerance changes and click-point drags rewrite that entry's result until Finish/Enter. Finish adds no entry |
| Move Selection drag / nudge | one entry per drag or key repeat burst |
| Select All, Deselect, Invert, Paste Selection | one entry each |

**5. Move Pixels: a floating layer in committed state, rendered from the original.**

```rust
pub(crate) struct Floating {        // State.floating: Option<Arc<Floating>>
    layer: usize, source: PixelStore /* lifted tiles only, COW */,
    mask: MaskStore, origin: Arc<Selection>, transform: Affine, keep_original: bool,
}
```

- The first drag lifts. With no selection, the whole layer is lifted. The
  layer itself is untouched. The sampler draws the layer with
  `clip(layer, TRANSPARENT, mask)` (a hole, skipped when Ctrl-copy sets
  `keep_original`) plus `source` resampled through `transform` and clipped to
  `origin.transformed(transform)`.
- **Each drag is one entry, "Move Pixels".** It changes only `transform`,
  sharing `source` by `Arc`, so it is cheap to checkpoint. Every render starts
  from `source`, so repeated moves never lose quality.
- **Finish** (Enter, Finish, tool switch, layer change, save, or any other
  pixel edit) writes the hole and the placement, then clears `floating` →
  entry "Finish Pixels". Undo steps back through drags one at a time.
- Resampling follows the tool setting (default Bicubic) and is applied only at
  preview and finish.
- At most one live preview edit exists (`ActiveEdit`: shape, stroke, effect,
  text). Starting one finishes any floating pixels first.

**6. Image/Edit commands** (one entry each):
- **Crop to Selection** crops the canvas to the selection's bounding rectangle
  **and** sets pixels outside a non-rectangular selection to transparent black.
- **Erase Selection** writes transparent black and deselects.
- **Cut** copies, writes transparent white `#FFFFFF00`, and deselects.
- **Fill Selection** uses Primary and keeps the selection.

## Alternatives considered

- **Mask as the canonical form.** Rotating or scaling a selection would
  resample the mask (lossy), the AA mode would be baked in, and Copy Selection
  could not export polygons. Rejected; the mask stays as the cache.
- **A CSG op list evaluated per pixel.** Cost grows with every combine, and the
  outline still needs a contour.
- **Floating pixels as a preview outside history.** Simpler, but it cannot show
  per-drag "Move Pixels" entries the way Paint.NET's History does.
- **Lifting by erasing the layer at once.** Breaks preview isolation and the
  Ctrl-copy behaviour.

## Consequences

- Positive: one clipping path, cost proportional to the boundary, lossless
  repeated moves, and paste (ADR 0005) reuses `Floating`.
- Negative: `history_store` must serialize `Floating` (its tiles reuse
  existing tile records). Noisy wand results can create millions of edges,
  which fail with `MemoryLimit` and are never silently truncated.
- New core dependency: `i_overlay`.

## Implementation slices

1. **MaskStore** — `core/src/mask.rs` (+tests) · after #42 · implicit background.
2. **Polygon wrapper** — `core/src/polygon.rs`, `Cargo.toml` · — · boolean ops.
3. **Selection** — `core/src/selection.rs` (+tests) · 1, 2 · `combine`,
   `transformed`, `mask`; checked against a per-pixel oracle.
4. **State + history** — `core/src/lib.rs`, `core/src/history_store.rs` · 3 ·
   commands of §4.
5. **Clip in commits** — `core/src/clip.rs`, `core/src/lib.rs` · 4 · shapes and
   strokes clipped; existing tests unchanged.
6. **ActiveEdit enum** — `core/src/lib.rs`, `core/src/painting.rs` · #43.
7. **Floating** — `core/src/floating.rs` (+tests) · 5, 6 · lift, drag entries,
   Finish Pixels, Ctrl-copy.
8. **Floating in history** — `core/src/history_store.rs` · 7.
9. **Magic Wand flood** — `core/src/flood.rs` · 3 · also used by Paint Bucket.
10. **Crop/Erase/Cut/Fill** — `core/src/canvas.rs` · 5.
11. **Selection tools** — `controller/src/tools/select.rs` · ADR 0003 s3 ·
    modifiers per Appendix A.
12. **Marching ants + Copy/Paste Selection JSON** — `controller/src/preview.rs`,
    `controller/src/clipboard.rs` · 4.

## Parity notes

This unblocks Rectangle/Ellipse/Lasso Select, Magic Wand, the five modes and
their modifiers, Selection Clipping Mode, Move Selection, Move Pixels
(including Ctrl-copy), Select All/Deselect/Invert, Erase/Fill Selection, Cut,
Crop to Selection, Copy/Paste Selection, and clipping for every tool and
effect. Marching-ants rendering is our own.

## Open questions

1. The "Finish Pixels" entry comes from Pinta, a fork of Paint.NET 3.36
   (`Document.FinishSelection` pushes `FinishPixelsHistoryItem`), not from the
   5.2 docs. Confirm in Paint.NET 5.2's History panel when a Windows machine is
   available.

## Appendix A — Paint.NET 5.2 facts (docs checked 2026-11-12)

- Modes: Replace, Add, Subtract, Intersect, Invert ("xor") ([Selection
  Tools][sel], rev. 4 Mar 2026). Modifiers ([Keyboard & Mouse][kb], rev. 14 Oct
  2026): Ctrl+left = Add, Alt+left = Subtract, Ctrl+right = Invert,
  Alt+right = Intersect. Plain left or right = Replace. Shift = square/circle.
  Left+right together = move the selection while dragging. Wand: Shift =
  Global.
- Rect Select styles: Any Size, Fixed Ratio, Fixed Size ([Rect Select][rs]).
  Releasing the mouse fixes the selection; use Move Selection to adjust it.
  Enter, Ctrl+D, or clicking off-canvas with a selection tool deselects.
- Magic Wand ([doc][wand]): Contiguous/Global flood, Tolerance, Image/Layer
  sampling, Tolerance Alpha (Premultiplied default / Straight). Tolerance edits
  and click-point drags reinterpret the selection "as if the user had undone the
  original and reapplied it". Enter or Finish ends editing.
- Move tools ([Move Tools][mv], rev. 24 Oct 2026): with no selection, the whole
  layer moves; Ctrl moves a copy; right-drag rotates; Shift snaps rotation to
  15°; Shift/Alt on nubs keep the ratio or scale about the centre; arrow keys
  move 1 px, Ctrl+arrow 10 px. Resampling: Nearest Neighbor, Nearest Neighbor
  (Antialiased), Bilinear, Multisample Bilinear, Anisotropic, Bicubic (default);
  gamma-corrected option. 5.2 renamed "Move Selected Pixels" to "Move Pixels"
  ([change log][road]).
- History lists "creation and modification of a selection" ([History][hist]).
- Edit commands, Crop, Cut, Erase and Paste Selection JSON: [Edit menu][edit]
  (rev. 9 Sep 2026), [Image menu][img] (rev. 4 Mar 2026).
- Pinta's Move Selected tool pushes one history item per transform and a Finish
  Pixels item on commit ([Pinta source][pinta]).

[sel]: https://paint.net/doc/latest/SelectionTools.html
[kb]: https://paint.net/doc/latest/KeyboardMouseCommands.html
[rs]: https://paint.net/doc/latest/RectangleSelectionTool.html
[wand]: https://paint.net/doc/latest/MagicWand.html
[mv]: https://paint.net/doc/latest/MoveTools.html
[hist]: https://paint.net/doc/latest/HistoryWindow.html
[edit]: https://paint.net/doc/latest/EditMenu.html
[img]: https://paint.net/doc/latest/ImageMenu.html
[road]: https://paint.net/roadmap.html
[pinta]: https://github.com/PintaProject/Pinta/blob/master/Pinta.Tools/Tools/MoveSelectedTool.cs
[iov]: https://lib.rs/crates/i_overlay
