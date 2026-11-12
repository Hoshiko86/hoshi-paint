# 0002. Effect and adjustment framework

- Status: Accepted (owner, 2026-11-12, #49)
- Date: 2026-11-12
- Related: ADR 0001 (clipping), 0004 (precision), ROADMAP 0.3.0/0.4.0,
  ARCHITECTURE.md "versioned project container and Wasm effects"

## Context

Roadmap 0.3.0 requires adjustments and effects with "preview matches apply;
cancellation preserves document"; 0.4.0 adds Wasm effects. Paint.NET effects
"apply special effects to the active layer or selection"; Repeat (Ctrl+F)
reapplies the last effect with the same parameters and no dialog
([Effects menu][fx], rev. 15 Aug 2026). Paint.NET 5.2 ships 13 adjustments and
about 50 effects in nine submenus (Appendix A). Effects run at FP32 since 5.0
([5.0 notes][pdn50]). Since 5.1, GPU effects default to the
`WorkingSpaceLinear` colour context: the image is linearized with its own
profile, rendered, and converted back. Individual effects expose their own
gamma choice; for example, Gaussian Blur defaults to sRGB 2.2 gamma
([5.1 notes][pdn51], [5.0 notes][pdn50]).

Hoshi constraints that shape this: core must stay free of threads and clocks
(AGENTS.md); layers are sparse COW `PixelStore`s whose `Arc<Tile>` are `Send +
Sync`, so an immutable snapshot is a cheap `PixelStore` clone; every tile
allocation already reserves the memory budget.

## Decision

**1. Effects are pure per-tile functions in core; scheduling is elsewhere.**

```rust
// hoshi-core/src/effects/mod.rs
pub struct EffectId(&'static str);              // stable, e.g. "adjust.hue-saturation"
pub enum ParamKind {
    Int { min: i32, max: i32, default: i32 },
    Float { min: f64, max: f64, default: f64, step: f64 },
    Bool { default: bool },
    Choice { options: &'static [&'static str], default: usize },
    Color { default: Color }, Angle { default: f64 }, Point { default: (f64, f64) },
    Seed,                                       // explicit; "Reseed" = new value
}
pub struct ParamSpec { pub id: &'static str, pub kind: ParamKind }
pub struct Params(Vec<ParamValue>);             // only built by Schema::validate

pub trait Effect: Sync {
    fn id(&self) -> EffectId;
    fn category(&self) -> Category;              // Adjustment | Effect(Submenu)
    fn schema(&self) -> &'static [ParamSpec];
    /// Extra source pixels needed around each output tile (0 for per-pixel ops).
    fn halo(&self, params: &Params) -> u32;
    /// Pure: same inputs → same bytes, on any thread, in any tile order.
    fn render(&self, params: &Params, src: &SourceView<'_>, out: &mut TileBuf)
        -> Result<(), EffectError>;
}
/// Per-pixel adjustments get a blanket `Effect` impl plus a sparse fast path:
/// the layer's implicit background is mapped once and uniform tiles stay uniform.
pub trait PixelOp: Sync { fn apply(&self, params: &Params, c: Color) -> Color; }
```

- Labels and parameter names are **not** in core: the adapter localizes
  `"{effect_id}.{param_id}"` keys (CLAUDE.md decision 1).
- `Schema::validate(&[ParamValue]) -> Result<Params, EffectError>` is the only
  constructor, so `render` never sees out-of-range values.
- `SourceView` exposes read-only sampling of an immutable `PixelStore` snapshot
  with edge clamping; `TileBuf` is a budget-reserved 128×128 output.
- Errors: `#[derive(thiserror::Error)] enum EffectError { InvalidParameter {
  param: &'static str }, Cancelled, MemoryLimit, Core(#[from] Error) }`.

**2. Source is the active layer**, clipped to the selection, as in Paint.NET.
No flattened-image source is planned. Parity does not need one.

**2a. Working space is declared per effect and follows Paint.NET's default for
that effect** (owner decision). `fn working_space(&self) -> WorkingSpace
{ Encoded | Linear }`. Where Paint.NET exposes a gamma choice, it is a normal
`Choice` parameter whose default matches Paint.NET. Linearization uses the
document profile (ADR 0005), or sRGB when there is none. The default for each
effect is recorded in its slice and checked against the docs.

**3. Jobs run in the controller with std threads; no shared mutable state.**

```rust
// hoshi-controller/src/effects/job.rs
pub struct EffectJob {
    generation: u64,                        // bumped on every param change
    cancel: Arc<AtomicBool>,
    results: mpsc::Receiver<TileResult>,    // (generation, tile pos, Arc<Tile>)
    progress: Progress,                     // done / total tiles, owned by UI thread
}
```

- On start, the controller takes `Document::effect_snapshot()` (layer pixel
  clone, selection mask, bounds) and moves it into one background thread that
  renders tiles with `std::thread::scope` workers over **disjoint** tile ranges.
  Workers only read the snapshot and send owned tiles through the channel.
- Tile order: visible viewport tiles first (centre-out), then the rest within
  the selection bounds. Tiles fully outside the selection are never rendered.
- The UI thread drains the channel on a timer/`invoke_from_event_loop`, drops
  results whose `generation` is stale, and hands fresh tiles to the document.
- Cancel = set the flag and drop the receiver; workers check the flag between
  tiles (and every N rows for heavy effects). Thread count never changes output.

**4. Preview is an active edit; Apply is one entry.**
`ActiveEdit::Effect(EffectPreview { layer, id, params, tiles: HashMap<pos,
Arc<Tile>> })` (ADR 0001 §5). The sampler shows rendered tiles composited
through `clip(src, rendered, mask)` and the source elsewhere. Committed pixels
are never written during preview.

`Document::commit_effect(preview)` requires every tile in the selection bounds;
the controller completes rendering (with progress) first. Commit builds one new
`PixelStore` from the source plus the clipped tiles and checkpoints once, label
= effect id. Failure (budget, history I/O) leaves the layer and history intact
and the preview retained. Cancel drops the preview; no entry.

**5. Determinism.** Reference results come from the CPU path only. Integer and
`f32` arithmetic are IEEE-deterministic in Rust (no implicit FMA contraction);
transcendental functions (`exp`, `sin`, `powf`) may differ across libms, so
effects use lookup tables or the `libm` crate for anything per-pixel. Random
effects take an explicit `Seed` parameter and a per-tile counter-based RNG
(seed, tile x, tile y), so tiles are order-independent. Tests: golden hashes for
integer-only effects; ±1 per channel for float effects across OSes.

**6. Wasm boundary = the same per-tile contract.** A future
`crates/hoshi-effects-wasm` implements `Effect` by calling a guest export
`render(params, src_tile_with_halo, out_tile)`. The guest receives only that
tile memory and its schema — no filesystem, network or clock (least authority).
The host owns scheduling, cancellation (epoch/fuel interruption) and a per-call
memory cap. **The runtime is decided at 0.4.0** (owner decision; iPad comes
after desktop parity). Candidates: `wasmtime` (Apache-2.0 WITH LLVM-exception,
GPLv3-compatible, JIT) for desktop, with `wasmi` (MIT/Apache interpreter)
noted as the fallback for platforms that forbid JIT, such as iOS.

## Alternatives considered

- **Threads inside core** (rayon in `Document`). Violates the core determinism
  rule and hides scheduling from the controller that owns cancellation.
- **`Arc<RwLock<Document>>` shared with workers.** Forbidden; snapshots are free
  because tiles are already COW.
- **Separate adjustment subsystem.** Duplicates preview/apply/history logic.
  Adjustments are effects with `halo = 0` and the `PixelOp` fast path.
- **GPU-first effects (wgpu).** Deferred per ARCHITECTURE.md; a GPU path must
  match the CPU reference within tolerance before it can be enabled.

## Consequences

- One lifecycle for ~50 built-in effects and later plugins; each new effect is
  one file plus a registry line.
- Preview memory = rendered tiles, bounded by the budget; a full-layer preview
  on a huge dense layer can fail with `MemoryLimit`, leaving the document intact.
- Adds `thiserror` (core/controller) and later `libm` (core).

## Implementation slices

1. **Schema + params** — `core/src/effects/params.rs` (+ tests) · — ·
   `ParamSpec`, `Params`, `validate`; invalid values rejected.
2. **Effect trait + SourceView** — `core/src/effects/mod.rs` · 1 · trait,
   `PixelOp` blanket impl, halo-aware `SourceView`.
3. **Invert Colors + Brightness/Contrast** — `core/src/effects/adjust.rs` · 2 ·
   reference adjustments; sparse background fast path tested.
4. **Effect preview/commit** — `core/src/effect_edit.rs`, `core/src/lib.rs` ·
   2, ADR 0001 slices 5–6 · `ActiveEdit::Effect`, `commit_effect`, one entry.
5. **Job runner** — `controller/src/effects/job.rs` (+ tests) · 4 · scoped
   workers, generation, cancel, progress; output identical for 1 vs N threads.
6. **Controller API** — `controller/src/effects/mod.rs`, `controller/src/lib.rs` ·
   5 · `begin_effect`, `set_effect_params`, `apply_effect`, `cancel_effect`.
7. **Schema-driven dialog** — desktop `effect_dialog.rs` + one `.slint` file ·
   6 · renders controls from `ParamSpec`; labels from the locale catalog.
8. **Gaussian Blur** — `core/src/effects/blur.rs` · 2 · first halo effect;
   Gamma choice parameter, default sRGB 2.2 as in Paint.NET 5.0+.

## Parity notes

Unblocks every item in Appendix A. Repeat Effect (Ctrl+F) re-applies the
stored `Params` without a dialog. Also unblocks preview, progress and cancel,
and later plugins. Paint.NET's plugins are .NET DLLs and are not binary
compatible (ROADMAP.md).

## Open questions

None that block the framework. Each effect slice must record Paint.NET's
parameter list and gamma default from its doc page.

## Appendix A — Paint.NET 5.2 menus (docs checked 2026-11-12)

- **Adjustments** ([doc][adj]): Auto-Level, Black and White,
  Brightness/Contrast, Curves, Exposure, Highlights/Shadows, Hue/Saturation,
  Invert Alpha, Invert Colors, Levels, Posterize, Sepia, Temperature and Tint.
- **Effects** (submenu pages rev. Aug–Sep 2026):
  - Artistic: Ink Sketch, Linocut, Mosaic, Oil Painting, Pencil Sketch,
    Pointillism, Stained Glass.
  - Blurs: Bokeh, Fragment, Gaussian, Median, Motion, Radial, Sketch, Square,
    Surface, Unfocus, Zoom.
  - Color: Ordered Dither, Quantize.
  - Distort: Bulge, Crystalize, Dents, Frosted Glass, Morphology, Pixelate,
    Polar Inversion, Spherize, Tile Reflection, Twist, Waves.
  - Noise: Add Noise, Reduce Noise.
  - Object: Drop Shadow.
  - Photo: Glow, Red Eye Removal, Sharpen, Soften Portrait, Straighten,
    Vignette.
  - Render: Clouds, Julia Fractal, Mandelbrot Fractal, Turbulence.
  - Stylize: Edge Detect, Emboss, Halftone, Outline, Relief.
- New in 5.2: Linocut, Mosaic, Pointillism, Stained Glass, Ordered Dither,
  Spherize, Waves, Halftone ([5.2 alpha 9719][a9719]). This list comes from
  the menu images on each page and may omit items that have no image.

[fx]: https://paint.net/doc/latest/EffectsMenu.html
[adj]: https://paint.net/doc/latest/AdjustmentsMenu.html
[pdn50]: https://blog.paint.net/2023/01/10/paint-net-5-0-is-now-available/
[pdn51]: https://blog.paint.net/2024/11/12/paint-net-5-1-is-now-available/
[a9719]: https://blog.paint.net/2026/08/10/paint-net-5-2-alpha-build-9719/
