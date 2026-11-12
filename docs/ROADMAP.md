# Release roadmap

## Principles

Fast everyday editing, native shape tools, predictable interactions, and portable
document operations. Recreate the valued workflow with original implementation
and assets. Existing Paint.NET plugins are not binary compatible; useful effects
may become built-in features or be recreated through a future Wasm API.

Versions describe scope, not dates. Only v0.0.0 is a committed implementation
target; later scopes can change after usability and technical validation.

| Version | Outcome | Release gate |
| --- | --- | --- |
| 0.0.0 | Desktop foundation: canvas, brush/eraser/eyedropper, colors, raster layers, shapes, undo, PNG and layered save | Full acceptance scenario on Windows, macOS, Ubuntu |
| 0.0.1 | Stabilization and packaging fixes | No known data-loss defects; reliable foundation scenario |
| 0.1.0 | Selections, pixel movement, clipboard, crop/resize, rotate/flip, JPEG | Complete a basic image-editing workflow |
| 0.2.0 | Fill bucket, polygons/freeform shapes, line styles, improved brushes, pressure where available | Drawing and shape interaction specifications pass |
| 0.3.0 | Brightness/contrast, hue/saturation, levels, blur, sharpen, shared effect preview | Preview matches apply; cancellation preserves document |
| 0.4.0 | Versioned Wasm effects, installation interface, limits, sample plugin | External effect works on all supported desktop platforms |
| 0.5.0 | Usable iPad/iOS interface, Pencil, gestures, files, lifecycle recovery | Real-device edit, background/resume, and save/reopen pass |
| 0.6.0 | Large-document performance, tiles, autosave/recovery, cancellation | Published memory, latency, and recovery budgets pass |
| 0.7.0 | Text, more selections/blend modes, prioritized native effects | Representative artwork workflows pass |
| 0.8.0 | Accessibility, scaling, platform conventions, shortcut customization | Platform accessibility and usability review |
| 0.9.0 | Beta: compatibility, signing, installers, documentation | Feature freeze and release checklist pass |
| 1.0.0 | Stable supported editor and file format | Upgrade, support, and compatibility policies published |

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

v0.0.0 is blocked on brush and eraser strokes (the eyedropper tool exists)
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
