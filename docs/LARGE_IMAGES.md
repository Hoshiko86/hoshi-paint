# Large images and memory policy

Hoshi now accepts **1–262,144 pixels on each axis**, matching the maximum recorded
in Paint.NET's [official release history](https://www.getpaint.net/roadmap.html)
(introduced in 4.2.2). This replaces the initial 1,048,576-total-pixel restriction.
The limit describes coordinates and dimensions; it does not guarantee that every
possible dense image can fit on every device. One fully materialized RGBA8 layer
at 262,144 × 262,144 would occupy 256 GiB before history and working buffers.

The initial limit protected a disposable implementation that stored dense layers,
copied full layers on edit, composited the entire image for every pointer update,
and allocated decoded-source and intermediate floating-point frames for resize.
Those choices were appropriate for testing the editing workflow but prevented
ordinary large photos and panoramic images from working efficiently.

## Changes

- **Sparse tiled layers.** Each layer has an implicit background color and allocated
  128×128 tiles. Blank/solid canvases do not allocate a full image. Edge tiles allocate
  only their valid pixels, so extremely thin images do not waste 128 times their RAM.
- **Copy-on-write history.** Snapshots share unchanged tiles. A small shape copies
  only touched tiles; rasterization visits its clipped bounds. Opaque full-canvas
  fills and uniform transforms/resizes retain constant-size storage.
- **Viewport rendering.** Large images are sampled directly into the visible viewport,
  bounded to four megapixels and 4096 pixels per texture edge, including HiDPI.
  Tabs use small direct-sampled thumbnails. Small images retain the original texture
  path. Pan/zoom no longer builds a full-image display buffer.
- **Scanline resampling.** All eight filters retain premultiplied color and gamma
  behavior, with checked axis tables and an 8 MiB row cache. Sparse regions are
  filtered without traversing implicit background. A Lanczos alpha-overshoot hue
  bug exposed by the new tests is also corrected.
- **Streaming files.** Native project V2 streams backgrounds and allocated tiles;
  it reads earlier V1 files too. PNG reads/writes rows, including Adam7 input. JPEG
  export samples the document without a second full RGB buffer. The old 128 MiB
  encoded-file ceiling is gone. Formats whose decoders retain an encoded/full-frame
  buffer must reserve that actual working memory before decoding.
- **A shared memory budget.** Unique pixel allocations and managed codec/resampling
  scratch are charged across all documents and retained undo states. The desktop
  defaults to one quarter of available RAM at startup (64 MiB fallback/minimum).
  Allocations reserve budget and use fallible vector allocation; resource failures
  leave the committed document intact. UI textures, Rust metadata and framework
  allocations are separately bounded, not a promise about total process RSS.
  Developers may override the budget with `HOSHI_MEMORY_LIMIT_MB` before launching.
- **Large-canvas navigation.** Fit shows the whole image; the minimum zoom is 1/4096
  (about 0.0244%) so even maximum-size images fit small docked viewports. This extends
  beyond Paint.NET's documented 1% minimum zoom. Fractional zoom values remain visible.

Native `.hoshi` and PNG support the canvas dimension range. JPEG has its own
**65,535-pixel-per-axis** format limit and is rejected before flattening/mutating
an oversized document. PNG/native saves retain atomic destination replacement.
A V2 project cannot be opened by an older Hoshi build; this build reads V1 and V2.

## What still differs from Paint.NET

Paint.NET's [official features](https://www.getpaint.net/features.html) say history
length is limited by available disk space. Hoshi still retains 16 in-memory history
entries, now sharing tiles; it does **not** yet page tiles/history to disk. Disk-backed
history, cold-tile eviction, background/cancellable image operations and compressed
native tile records are follow-up work. Arbitrary dense photographic content must
fit the memory budget, and expensive file/resampling work can still take time.
32-layer and 32-open-document limits remain. The current Paint.NET internals are
not fully public; these are Hoshi's own implementation choices, not a claim that
Paint.NET uses an identical sparse tile representation.

## Evidence

Tests create maximum-size sparse canvases, edit the far corner beyond 32-bit pixel
index ranges, undo/redo, rotate, resize, flatten, and round-trip sparse projects.
A budget-exhaustion test verifies a failed edit preserves both committed content
and its active preview. Codec tests cover max-width PNG (>2 megapixels), compact
max-size native files, Adam7 rows, and a valid JPEG larger than the old 128 MiB cap.
Resampling tests include max-size sparse/uniform images, anisotropic filters,
transparent RGB, and downsampling without full-frame float buffers.

The real winit `large_canvas_smoke` creates a maximum-size canvas, fits it, makes
small edits at 100%, navigates history and resizes the window. That sequence uses
less than 8 MiB of tracked pixel storage; this measures a sparse drawing, not a dense
256 GiB photograph. CI artifacts include its Fit/edit/resize screenshots.
