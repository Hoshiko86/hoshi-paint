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

## GitHub management

One GitHub Project; milestones per release; issues for reviewable deliverables.
Fields: Status, Priority, Area, Platform, Target version. Statuses: Backlog,
Ready, In progress, In review, Done. Views: current release, roadmap, defects,
architecture. Link PRs to issues and include validation evidence.

Each issue needs user behavior, exclusions, acceptance criteria, dependencies,
and validation. `BACKLOG.md` is the source for initial issue creation. The 12 release milestones and 13 initial issues have been created; see the
[GitHub milestones](https://github.com/Hoshiko86/hoshi-paint/milestones) and
BACKLOG.md for issue links. Project creation requires additional integration
permissions; the attempted `createProjectV2` mutation was rejected.
