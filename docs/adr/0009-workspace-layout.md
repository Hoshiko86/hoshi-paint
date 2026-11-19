# 0009. Workspace layout: context bar, panel rail, four panels

- Status: Proposed
- Date: 2026-11-19
- Deciders: owner (approve), Claude (architect)
- Related: ADR 0003 (tools), ADR 0007 (theme), ADR 0008 (floating panels),
  ADR 0001 (selection quality), ADR 0004 (blend modes)

## Context

The desktop layout grew one feature at a time:

- A left rail mixes tools (Shapes, Brush, Eraser, Color Picker, Screen
  Picker), a brush-size field and the Colors button.
- The command bar's first row always shows a shape summary.
- Shape options live in a separate Shape Properties panel.
- The pickers were about to move into the Colors window (#235).

**Owner decisions (2026-11-19):**

- **A context-aware tool options bar** across the top of the canvas, under
  the document tabs. Its contents follow the active tool:
  - **Brush:** size, hardness, spacing, brush type (as in GIMP or
    Photoshop).
  - **Eraser:** the same options as Brush.
  - **Shapes:** shape type (rectangle, rounded rectangle, ellipse, triangle,
    right triangle, trapezoid, parallelogram, …), draw mode (fill, outline,
    fill + outline), size, line style, fill style, antialiasing, blending mode,
    selection quality, and a Finish button.
  - **Color Picker:** sampling (layer, canvas/image, screen), sample size,
    after-click behavior, and selection quality.
- **Both pickers become one tool.** Screen is a Color Picker sampling source.
  With Screen selected, the bar shows a **Pick from screen** button that starts
  the OS screen pick (ADR 0005). Clicking the canvas still samples the canvas.
  Both pickers leave the Colors panel.
- **A panel rail at the top of the window** toggles four panels: **Colors**,
  **Tools**, **Layers** and **History**. The Colors button leaves the left
  rail.
- **The tool rail becomes the Tools panel,** docked left by default, so it
  looks like today's rail. It can float or dock elsewhere like any panel
  (ADR 0008).
- **Layout first:** the bar ships with the options that already work. New
  features each get their own epic and appear in the bar when they land.

"Selection quality" is Paint.NET's *Antialiased* / *Pixelated* choice for how
a selection's edge clips drawing *(Paint.NET behavior unverified; owner listed
it for parity)*. It maps directly to ADR 0001's `ClipQuality { Antialiased,
Aliased }`.

## Decision

1. **Window structure, top to bottom:**
   1. Main command bar: undo/redo, commit and cancel, zoom, and on the right
      the **panel rail**.
   2. Document tabs.
   3. **Tool options bar** (fixed height).
   4. Workspace: canvas plus docked panels.
   5. Status bar.

   Every row has a fixed height, so switching tools never moves the canvas.

2. **Tool options bar.** One component, `ToolOptionsBar`
   (`ui/tool-options-bar.slint`), with one section per tool family (Brush and
   Eraser, Shapes, Color Picker), chosen by `selected-tool`.
   - **Sources:** each control reads and writes `ToolSettings` through one
     adapter module (`src/tool_options.rs`), like `shape_rendering.rs` today.
   - **Shared settings:** brush size, antialiasing and selection quality are
     one shared value across tools (CLAUDE.md: brush size is one shared
     toolbar value).
   - **Missing features:** a control whose feature doesn't exist yet isn't
     shown. No disabled placeholders.
   - **Overflow:** when the bar is narrower than its section, the extra
     controls go into a trailing `»` menu, so the bar never wraps.
   - **Text:** every label comes from the catalogs.

3. **Shape Properties panel is retired.** Its style controls move to the
   Shapes section of the bar.
   - The numeric Bounds fields (X, Y, Width, Height, Lock), a Hoshi extra,
     move to the end of the Shapes section and go into the overflow menu
     first.
   - After that, the panel, its `PanelId` and its window are removed.

4. **Panel rail.** Four toggle `IconButton`s (#244): Colors, Tools, Layers,
   History. Each shows its panel's open state and opens or closes it,
   wherever it is placed.
   - The panel set is `PanelId { Tools, Colors, Layers, History }`.
   - Every panel uses `ToolPanelFrame` and ADR 0008's floating modes.
   - Defaults: **Tools** docked left; **Colors** floating; **History** and
     **Layers** closed.

5. **Tools panel.** It holds the tool buttons only (Shapes, Brush, Eraser,
   Color Picker; more as ADR 0003 adds tools).
   - Docked left or right, it is a single column, as narrow as today's rail.
   - Floating, or docked top or bottom, it is a grid that wraps.
   - Tool selection is unchanged (`ToolId` keys).

6. **Layers panel.** A new panel over the core's existing `Layer` API:
   - a list, top layer first, with name, visibility toggle and thumbnail;
   - select;
   - add, delete, duplicate, move up and down;
   - opacity, and properties (rename).

   Every edit is one history entry, through the controller. Blend mode joins it
   when ADR 0004's blend modes land.

## Feature epics (shown in the bar when done)

- **Brush engine:** hardness, spacing, brush types (round, square, textured,
  …).
- **Shape types:** rounded rectangle, triangle, right triangle, trapezoid,
  parallelogram, … (core `ShapeKind` today has only Rectangle and Ellipse).
- **Blending mode:** ADR 0004.
- **Selection quality:** wired once selections clip drawing (ADR 0001
  slices).

## Consequences

- #235's Colors-window picker placement and #238's Brush panel are dropped
  (both superseded). The Colors window redesign (#239–#241) keeps its sliders,
  wheel and More/Less, but loses the picker block.
- The left rail, the command bar's shape-summary row and the Shape Properties
  panel go away, so there is less chrome and one place for options.
- Bar width becomes a constraint as features land, which is why it has the
  overflow menu.

## Slices (filed as each prerequisite merges)

1. **Bar shell + existing options:**
   - `ToolOptionsBar` under the tabs replaces the command bar's shape row.
   - Brush/Eraser section: size, antialiasing.
   - Shapes section: type (rectangle, ellipse), draw mode, size, line style,
     fill style, antialiasing, Finish.
   - Picker section: sampling Layer/Image/Screen with Pick from screen, sample
     size, after click.
   - Remove the brush-size field from the rail, and both pickers from the rail
     and the Colors panel.
2. **Panel rail + Tools panel:**
   - the panel rail in the command bar;
   - the rail becomes the docked-left Tools panel (`PanelId::Tools`);
   - the Colors button leaves the rail.
3. **Bounds into the bar + overflow menu;** retire Shape Properties.
4. **Layers panel** (likely two slices: list and select, then edit
   operations).
5. **Feature epics** as listed above.
