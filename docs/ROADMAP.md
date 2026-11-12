# Release roadmap

## Principles

Fast everyday editing, native shape tools, predictable interactions, and portable
document operations. Recreate the valued workflow with original implementation
and assets. Existing Paint.NET plugins are not binary compatible; useful effects
may become built-in features or be recreated through a future Wasm API.

Goal: **as much behavioral parity with Paint.NET 5.2 as possible** on Windows,
macOS and Linux, with original implementation and assets. The feature-by-feature
inventory is [PARITY.md](PARITY.md); cross-cutting designs are in [adr/](adr/README.md).
Desktop parity comes first; iPad/iOS follows it (the core stays platform-neutral).

Versions describe scope, not dates. Only v0.0.0 is a committed implementation
target; later scopes can change after usability and technical validation.

| Version | Outcome (Paint.NET parity areas) | Key ADRs | Release gate |
| --- | --- | --- | --- |
| 0.0.0 | Foundation: canvas, Paintbrush/Eraser, canvas Color Picker + Screen Picker, colors, layer panel, Shapes, undo, PNG and `.hoshi` save | 0003 | Full acceptance scenario on Windows, macOS, Ubuntu |
| 0.0.1 | Stabilization: tech-debt issues, packaging, no data-loss defects | — | Reliable foundation scenario |
| 0.1.0 | Selections (Rectangle, Ellipse, Lasso, Magic Wand; Replace/Add/Subtract/Intersect/Invert), Move Pixels / Move Selection, Crop to Selection, Edit menu and clipboard (paste into new layer / new image, Copy Flattened) | 0001, 0005 | A basic select–move–crop–copy workflow passes |
| 0.2.0 | Painting tools: Pencil, Paint Bucket, Gradient, Clone Stamp, Recolor, Line/Curve, all Shapes presets; tool blend/Overwrite; pen pressure (Windows, macOS; Linux best effort) | 0003, 0005 | Drawing and tool interaction specifications pass |
| 0.3.0 | Effect framework + all Adjustments (live preview, cancel, progress, selection clipping, Repeat Ctrl+F) | 0002 | Preview matches apply; cancellation preserves document |
| 0.4.0 | All built-in Effects categories (Artistic, Blurs, Distort, Noise, Object, Photo, Render, Stylize, Color) | 0002 | Every effect has a deterministic reference test |
| 0.5.0 | Layers parity: 14 blend modes, FP32 compositing, Layer Properties, merge down, import from file, Rotate / Zoom, `.hoshi` v3 | 0004 | Blend-mode references within tolerance |
| 0.6.0 | Text tool (fonts, shaping, IME); View parity (rulers, units, grid); remaining file formats and color profiles | 0005 | Representative artwork workflows pass |
| 0.7.0 | Wasm effect plugins (versioned API, limits, sample plugin); performance budgets, autosave/recovery | 0002 | External effect works on all desktop platforms |
| 0.8.0 | Accessibility, scaling, platform conventions, settings and shortcut parity | — | Platform accessibility and usability review |
| 0.9.0 | Beta: compatibility, signing, installers, documentation | — | Feature freeze and release checklist pass |
| 1.0.0 | Stable desktop editor and file format | — | Upgrade, support and compatibility policies published |
| 1.x | iPad/iOS: touch and Pencil interface, files, lifecycle recovery | 0005 | Real-device edit, background/resume, save/reopen pass |

## Delivery order

1. Write interaction specifications and record architecture decisions.
2. Build the document/history core and shape lifecycle, without a GUI dependency.
3. Evaluate UI candidates and validate an iOS build path before choosing a shell.
4. Implement canvas, colors, brush/eraser, layers, shapes, then file operations.
5. Validate the integrated workflow, package each desktop target, release.

## Already delivered ahead of plan

Implemented in the core or desktop spike before their planned version:

- JPEG load and save, with quality setting and density metadata (0.1.0).
- Image Resize, Canvas Size, rotate 90°/180°, flip, and Flatten (0.1.0); see
  [IMAGE_MENU.md](IMAGE_MENU.md).
- Shape line styles (dash/dot variants) and fill patterns (0.2.0).
- Sparse 128×128 copy-on-write tiles, canvases up to 262,144 px per axis, and
  disk-backed compressed undo (0.6.0); see [LARGE_IMAGES.md](LARGE_IMAGES.md)
  and [DISK_HISTORY.md](DISK_HISTORY.md).
- Multi-document tabs ([DOCUMENT_MANAGEMENT.md](DOCUMENT_MANAGEMENT.md)),
  palettes ([PALETTES.md](../PALETTES.md)), and a localization foundation
  (en-US and es-ES catalogs).

## Current focus

v0.0.0 is blocked on Paintbrush and Eraser strokes
([#6](https://github.com/Hoshiko86/hoshi-paint/issues/6)), the layer panel
([#7](https://github.com/Hoshiko86/hoshi-paint/issues/7)), and the remaining
shapes, line and rounded rectangle
([#8](https://github.com/Hoshiko86/hoshi-paint/issues/8)). No new feature scope
until those land.

## GitHub management

Status lives in GitHub issues and
[Project #2](https://github.com/users/Hoshiko86/projects/2); these docs do not
track it. Milestones per release; issues for reviewable deliverables. Fields:
Status, Priority, Area, Platform, Target version. Statuses: Backlog, Ready, In
progress, In review, Done. Link PRs to issues and include validation evidence.

Labels in use: `codex-task`, `tech-debt`, `area:core`, `area:controller`,
`area:desktop`. Tech-debt issues (#36, #37, #39) carry the `tech-debt` label in
the v0.0.1 (stabilization) milestone.

Each issue needs user behavior, exclusions, acceptance criteria, dependencies,
and validation. [BACKLOG.md](BACKLOG.md) maps planning IDs to issues.

Workflow: Claude designs, splits tasks, and reviews PRs; Codex implements them
(see [CLAUDE.md](../CLAUDE.md) and [AGENTS.md](../AGENTS.md)).
