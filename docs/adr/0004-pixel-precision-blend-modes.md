# 0004. Pixel precision and blend modes

- Status: Proposed
- Date: 2026-11-12
- Related: `hoshi-core::Color::over`, `CompositeSampler`, FILE_FORMAT.md,
  LARGE_IMAGES.md, ADR 0002

## Context

Hoshi stores straight-alpha RGBA8 in sparse 128×128 COW tiles. `Color::over`
is exact integer source-over in encoded sRGB; `CompositeSampler::pixel` folds
every visible layer per pixel with it, quantizing to 8 bits **after every
layer**. Layers have only `visible` and `opacity`; `.hoshi` is version 2.

Paint.NET 5.2 (released 2026-11-10) ships a rewritten layer rendering engine
that composites at **FP32 per channel**, removing artifacts and wrong colours
the 8-bit renderer produced with multiple layers; the engine also supports
UI16/FP16/FP32 pixel formats, but the rest of the app still uses 8-bit layers.
New blend modes and layer folders are planned for 6.0; linear/gamma-correct
blending "for the entire workflow" is a stated future goal, not a 5.2 feature
([5.2 alpha][a52], [roadmap][road], [5.2 release][r52]). The layer Blend Mode
list has 14 modes: Normal, Multiply, Additive, Color Burn, Color Dodge,
Reflect, Glow, Overlay, Difference, Negation, Lighten, Darken, Screen and Xor.
Tools add **Overwrite** (copy). A tool blend acts "as if the tool was being used
on a new layer immediately above the active layer and the results merged down"
([Blend Modes][bm], rev. 4 Mar 2026). 6.0, planned for 2028, brings new blend
modes and layer folders ([roadmap][road]).

## Decision

**1. Storage stays RGBA8 straight alpha.** Paint.NET 5.2 layers are still 8-bit,
so parity does not need more; FP32 storage would quadruple memory (16 B/px) and
undo bytes against a budget that already limits dense images. High bit-depth
storage is revisited only with a 16-bit import requirement.

**2. Compositing is FP32 premultiplied, in encoded (gamma) sRGB, quantized once.**

```rust
// hoshi-core/src/composite.rs
#[derive(Clone, Copy)] struct Px { r: f32, g: f32, b: f32, a: f32 } // premultiplied, 0..=1
const DECODE: [f32; 256] = /* i / 255 */;
fn quantize(v: f32) -> u8 { (v.clamp(0., 1.) * 255. + 0.5) as u8 }  // round-half-up
pub(crate) fn composite_span(layers: &[LayerView<'_>], y: u32, x: Range<u32>,
                             out: &mut [Color]);
```

- Accumulate bottom-to-top in `Px`; convert to straight RGBA8 only at the
  output (viewport, flatten, export, Color Picker "Image" sampling).
- **No linearization** for layer blending: Paint.NET 5.2 still blends encoded
  values, and matching its observable output is the goal. A future
  `BlendSpace::Linear` document option is allowed but not built.
- Layer-internal writes (brush dabs, shape commit) keep the exact integer
  `Color::over` into RGBA8 storage; this ADR changes **only** cross-layer
  compositing, so existing single-layer tests stay bit-exact.

**3. Blend modes.** `Layer` gains `blend_mode: BlendMode`.

```rust
#[repr(u8)] pub enum BlendMode { Normal = 0, Multiply, Additive, ColorBurn,
    ColorDodge, Reflect, Glow, Overlay, Difference, Negation, Lighten, Darken,
    Screen, Xor }  // codes are file-format values; append-only
```

Separable modes use the W3C compositing structure on straight colours
`Cb` (backdrop), `Cs` (source) with alphas `ab`, `as` (after opacity):
`ao = as + ab·(1−as)`;
`Co·ao = as·(1−ab)·Cs + ab·(1−as)·Cb + as·ab·B(Cb, Cs)`.

| Mode | B(b, s) (reference formulas; pixel parity checked by slice 6) |
| --- | --- |
| Normal | s |
| Multiply | b·s |
| Additive | min(1, b+s) |
| Color Burn | s = 0 ? 0 : max(0, 1 − (1−b)/s) |
| Color Dodge | s = 1 ? 1 : min(1, b/(1−s)) |
| Reflect | s = 1 ? 1 : min(1, b²/(1−s)) |
| Glow | b = 1 ? 1 : min(1, s²/(1−b)) |
| Overlay | b < ½ ? 2bs : 1 − 2(1−b)(1−s) |
| Difference | \|b − s\| |
| Negation | 1 − \|1 − b − s\| |
| Lighten / Darken | max(b, s) / min(b, s) |
| Screen | 1 − (1−b)(1−s) |
| Xor | bitwise XOR of the 8-bit channel values: identical pixels → black, as the docs describe |

The list and Xor semantics come from Paint.NET's long-standing `UserBlendOps`
as preserved in the MIT-licensed [Pinta][pinta] port (not re-read for this revision; its source path has moved); we use them as a **specification
only** and confirm each mode against Paint.NET 5.2 reference renders
(slice 6) before claiming parity. The documented behaviour of each mode
(e.g. "white is transparent" for Multiply, "black is ignored" for Reflect, Glow
= Reflect with the layers swapped) is encoded as unit tests in slice 1. Tool
blending (Paint Bucket, Paintbrush, Pencil, Clone Stamp, Text, Shapes, Gradient,
Line/Curve) uses `ToolBlend { Mode(BlendMode), Overwrite }`. It is implemented
literally as "render the tool to a scratch tile, blend it onto the layer".

**4. Performance budget and caching.** Target from V0_0_0.md: 60 Hz navigation
on 2048² × 8 layers. A naive FP32 fold is ~33 M blend ops per 4 MP viewport
frame — too slow on one core. Therefore:

- **Composite tile cache** in the controller preview path: composited 128×128
  RGBA8 tiles keyed by `(tile pos, layer tile ids, layer props generation)`;
  only dirty tiles recompose. Cache bytes reserve the memory budget and are
  evicted LRU, never failing an edit.
- **Sparse short-circuits:** runs where every layer is its implicit background
  are composited once per run; an opaque Normal layer at opacity 255 hides
  everything below it.
- Hot loops are SoA over a span, written for auto-vectorization (stable Rust,
  no `std::simd`); data-parallel over disjoint tiles via `thread::scope` in the
  controller (ADR 0002 §3). Release-build p95 is measured before merge of
  slice 4.

**5. Format and history migration.**
- `.hoshi` **version 3**: after `opacity` each layer stores `u8 blend_mode`.
  Reader accepts V1/V2 (blend = Normal) and V3; unknown codes → `InvalidData`.
  Writer always writes V3. FILE_FORMAT.md updated in the same slice.
- `history_store` layer metadata gains the same byte (scratch only, no compat).
- `set_layer_properties(layer, visible, opacity)` becomes
  `set_layer_properties(layer, LayerProperties { visible, opacity, blend_mode })`;
  one entry "Layer Properties" as today.
- `CompositeSampler::pixel` keeps its signature and becomes a thin wrapper over
  `composite_span` so `project.rs`, export and thumbnails need no change.

## Alternatives considered

- **Keep 8-bit per-layer quantization.** Exactly the artifact class Paint.NET
  5.2 removed; multi-layer parity would drift.
- **Linear-light blending now.** Looks better for some modes, but diverges from
  Paint.NET 5.2 output; kept as a future document option.
- **FP16/FP32 storage.** 2–4× memory, larger undo, no parity gain today.
- **Fixed-point u16 compositing.** Deterministic and fast, but Paint.NET's FP32
  rounding behaviour is easier to match in `f32`; `f32` is deterministic in Rust
  for `+ − × ÷` without transcendental functions.

## Consequences

- Multi-layer composites may change by ±1 per channel versus today; tests that
  assert multi-layer exact values are updated with documented expectations
  (single-layer tests are unaffected).
- Older Hoshi builds cannot open V3 files (as V1 builds could not open V2).
- The composite cache becomes the main lever for large-document performance.

## Implementation slices

1. **BlendMode enum + formulas** — `core/src/blend.rs` (+ tests) · — · pure
   per-mode functions with edge-case tests (s=0, s=1, alpha 0).
2. **FP32 composite span** — `core/src/composite.rs` (+ tests) · 1 · Normal
   matches `Color::over` chains within ±1; exact for 0/255 alpha.
3. **Sampler on spans** — `core/src/lib.rs` · 2 · `CompositeSampler` delegates;
   single-layer tests unchanged.
4. **Composite tile cache** — `controller/src/preview.rs`, `controller/src/composite_cache.rs` ·
   3 · dirty-tile invalidation; budgeted; benchmark numbers in PR.
5. **Layer blend property** — `core/src/lib.rs`, `core/src/history_store.rs` · 1 ·
   `LayerProperties`; undo restores mode.
6. **Reference renders** — `core/tests/blend_reference.rs` + fixture PNGs · 5 ·
   **the owner provides the fixtures later** (they need Paint.NET 5.2 on
   Windows). Until then, tolerance-based tests cover the documented properties
   of each mode; the fixtures switch them to ±1 against Paint.NET.
7. **.hoshi V3** — `core/src/project.rs`, `docs/FILE_FORMAT.md` · 5 · V1/V2/V3
   read, V3 write, unknown-mode rejection.
8. **Layer Properties dialog** — desktop layer properties file + `.slint` · 5 ·
   blend dropdown with localized labels.

## Parity notes

Unblocks the Layer Properties blend mode list (14 modes), Paint.NET 5.2-class
FP32 multi-layer compositing, tool blend modes, and import of layered formats
that carry blend modes (e.g. ORA/PSD importers later). Deferred with Paint.NET:
layer folders and new 6.0 blend modes.

## Open questions

1. **(Owner)** Paint.NET 5.2 reference exports for slice 6 need a Windows
   machine. They will be provided later; until then, tests are tolerance-based.
2. Paint.NET's exact FP32→8-bit rounding (half-up, half-even, dithering) is
   undocumented. The reference renders will settle it.

[a52]: https://blog.paint.net/2026/05/09/paint-net-5-2-alpha-build-9625/
[road]: https://paint.net/roadmap.html
[r52]: https://forums.paint.net/topic/134628-paintnet-52-is-now-available/
[bm]: https://paint.net/doc/latest/BlendModes.html
[pinta]: https://github.com/PintaProject/Pinta
