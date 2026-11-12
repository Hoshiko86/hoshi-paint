# 0003. Tool framework and stable tool IDs

- Status: Accepted (owner, 2026-11-12, #49)
- Date: 2026-11-12
- Related: #44 (Brush/Eraser), #48 (eyedropper), CLAUDE.md debt 2–3, ADR 0001

## Context

`hoshi-controller::Tool` is an enum (`Rectangle`, `MoveShape`, `Pan`,
`Eyedropper`) and every pointer handler in `editing.rs` matches on it. The
desktop maps it to Slint magic ints (`0..=3` in `lib.rs` `refresh` and
`on_choose_tool`); #44 adds `4`/`5`. `HandlePoint.kind: i32` uses magic `8`/`9`.
Paint.NET 5.2 has 19 tools ([Tools window][tw]), each configured from a shared
toolbar ([Tool Bar][tb]); adding them by
growing two matches and an int table will not scale and violates the
"make invalid states unrepresentable" gate.

Current shape behaviour is specified in V0_0_0.md: tool changes preserve the
editable shape; Pan, zoom, colour selection and colour sampling never commit it.

## Decision

**1. `ToolId`: a stable, fieldless enum with string keys shared with the UI.**

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ToolId { MovePixels, MoveSelection, RectangleSelect, LassoSelect,
    EllipseSelect, MagicWand, Zoom, Pan, PaintBucket, Gradient, Paintbrush, Eraser,
    Pencil, ColorPicker, CloneStamp, Recolor, Text, LineCurve, Shapes }
impl ToolId {
    pub const ALL: [ToolId; 19];
    pub const fn key(self) -> &'static str;      // "rectangle-select", never renamed
    pub fn from_key(key: &str) -> Option<Self>;
}
```

Slint uses `in property <string> selected-tool` and `callback choose-tool(string)`.
Keys are also the localization and icon keys. The adapter has no tool table.

**2. Behaviour is a trait; dispatch is a closed enum (no `Box<dyn>`).**

```rust
// hoshi-controller/src/tools/mod.rs
pub(crate) trait ToolBehavior {
    fn pointer_down(&mut self, cx: &mut ToolCx<'_>, p: PointerInput) -> ToolResult;
    fn pointer_move(&mut self, cx: &mut ToolCx<'_>, p: PointerInput) -> ToolResult;
    fn pointer_up(&mut self, cx: &mut ToolCx<'_>, p: PointerInput) -> ToolResult;
    fn key(&mut self, cx: &mut ToolCx<'_>, k: KeyInput) -> Result<Handled, ControllerError>;
    fn cursor(&self, q: &ToolQuery<'_>, at: Point) -> EditCursor;
    /// Pointer capture lost / window focus lost. Default: same as pointer_up.
    fn pointer_cancel(&mut self, cx: &mut ToolCx<'_>);
}
pub(crate) enum ToolState { Paintbrush(BrushTool), Eraser(BrushTool), Shapes(ShapeTool), ... }
pub struct PointerInput { pub pos: Point /*logical*/, pub button: PointerButton,
    pub modifiers: InputModifiers, pub pressure: Option<f32> /* ADR 0005 */ }
```

- `ToolCx` is the **narrow** capability set: `&mut Document`, `&Viewport`,
  `&ColorState` (primary/secondary/slot), `&mut ToolSettings`. Tools never reach
  through the controller or the workspace (Law of Demeter).
- Tool-local state (current gesture, anchor, last point) lives in the tool
  struct, replacing the shared `Gesture` enum piece by piece.
- `ToolState::from(ToolId)` is the registry; adding a tool = one file + one arm.

**3. Active edits belong to the document, not the tool.** Shape, stroke,
floating pixels, text and effect previews are `Document::ActiveEdit` (ADR 0001
§5). On `set_tool(new)`:

- If `new.preserves_active_edit()` (Pan, Zoom, Color Picker, and the tool that
  owns the current edit) nothing is committed. This keeps V0_0_0.md behaviour.
- Otherwise the controller calls `document.finish_active_edit()` (commit, one
  entry) before switching. A failed commit aborts the switch and keeps the edit.
- Enter = Finish for every tool. With a selection tool active and no live
  edit, Enter deselects ([Selection Tools][sel]). Escape also finishes (Paint.NET; owner decision).

**4. Settings: one shared toolbar state, viewed through typed groups.**
Paint.NET's toolbar values belong to the toolbar, not to a tool: "there is no
such thing as per-tool anything" ([forum, staff reply][share]). So one Brush
size is shared by Paintbrush, Eraser, Clone Stamp, Recolor, Line/Curve and
Shapes.

```rust
pub struct ToolSettings { pub brush: BrushSettings /* size, pressure, hardness,
    spacing, smoothing */, pub fill: FillStyle, pub antialias: bool,
    pub blend: ToolBlend /* BlendMode | Overwrite */, pub selection_clip: ClipQuality,
    pub selection: SelectionSettings, pub flood: FloodSettings, pub shape: ShapeSettings,
    pub gradient: GradientSettings, pub text: TextSettings, pub picker: PickerSettings,
    pub move_pixels: MoveSettings }
impl ToolId { pub const fn settings(self) -> &'static [SettingsGroup];
              pub const fn shortcut(self) -> (char, u8 /* press count */); }
```

The UI builds the toolbar from `settings()`. Group setters validate values
(`set_brush_width` → `BrushTip::new`). Settings are workspace state, not
history, and they survive tab switches; this replaces the hidden `empty`
controller. The verified table is in Appendix A.

**5. Colour sampling: one canvas tool, one OS action.**
- **Color Picker** (`ToolId::ColorPicker`, parity, [doc][cp]) samples the
  document: left click → Primary, right click → Secondary. Sampling is Layer
  or Image, with Ctrl as a temporary Image mode. Sizes are 1, 3×3, 5×5, 11×11,
  31×31 and 51×51, averaged, alpha included. After-click actions: do not
  switch / previous tool / Pencil. It replaces `Tool::Eyedropper`.
- **Screen Picker** (Hoshi extra) is an adapter `Command`, not a tool: it calls
  `screen_pick.rs`, and the opaque result goes to `controller.set_selected_color`.
  It never changes `ToolId` or active edits. **Owner decision (2026-11-12):
  both exist.** This complements the CLAUDE.md note; it does not reverse it.

**6. Handles get a type.** `HandleKind { Resize(ResizeHandle), Rotate, Pivot }`
replaces `HandlePoint.kind: i32` and the `ROTATION_HANDLE`/`PIVOT_HANDLE`
constants; Slint receives `string` kinds.

## Alternatives considered

- **Keep the enum and add arms.** Each new tool touches `editing.rs` (500+
  lines) and `lib.rs`; conflicts between Codex tasks are guaranteed.
- **`Box<dyn ToolBehavior>` registry.** Open for plugins we do not plan, costs
  exhaustiveness checks. Enum dispatch keeps "add a tool" a compile error until
  every match is updated.
- **`u16` IDs for Slint.** Cheaper than strings but recreates magic numbers.
- **Settings per tool instance.** Not how Paint.NET works (see §4). Users ask
  for it on the forum, so it could become an opt-in Hoshi extra later.

## Consequences

- `editing.rs` and the spike `lib.rs` tool match shrink; `Gesture` is retired
  gradually, not in one PR.
- #44 can land first using `Tool`; slice 5 below migrates Brush/Eraser.
- The canvas Color Picker returns as a parity tool next to the Screen Picker
  action (owner decision). The architect updates CLAUDE.md.

## Implementation slices

1. **ToolId** — `controller/src/tool_id.rs` (+ tests) · — · enum, keys,
   `from_key`, round-trip test over `ALL`.
2. **Slint string IDs** — desktop `lib.rs` (tool match only) + rail `.slint` ·
   1 · magic ints removed; behaviour unchanged.
3. **Trait + ToolCx** — `controller/src/tools/mod.rs` · 1 · trait, `ToolState`,
   `PointerInput`; no tool migrated yet.
4. **Pan + Zoom tools** — `controller/src/tools/navigate.rs`, `controller/src/lib.rs` ·
   3 · first migration; existing pan tests unchanged.
5. **Brush/Eraser migration** — `controller/src/tools/brush.rs`, `lib.rs` · 3, #44 ·
   `Gesture::Stroke` removed.
6. **Shapes migration** — `controller/src/tools/shapes.rs`, `editing.rs` · 3 ·
   the largest slice; all `tests/` unchanged and green.
7. **HandleKind** — `controller/src/lib.rs`, desktop `shape_bounds.rs` · — ·
   replaces `i32` 8/9.
8. **ToolSettings groups** — `controller/src/tools/settings.rs` · 3 · moves
   width/antialias/mode fields off `Controller`.
9. **Color Picker tool** — `controller/src/tools/color_picker.rs` · 3, 8 ·
   sampling size, Ctrl = Image, after-click action; #48 specs reused.

## Parity notes

Unblocks every tool in Appendix A, Finish semantics on tool switch, the shared
toolbar, Paint.NET's shortcut cycling (pressing S repeatedly), and later
shortcut customization (0.8.0) through stable keys.

## Open questions

1. ~~**Escape.**~~ **Decided (owner, 2026-11-12): mirror Paint.NET.** Esc,
   Enter, Ctrl+D or Finish commit live text and tool drawing
   ([Keyboard & Mouse][kb]). V0_0_0.md and SHAPE_EDITING.md are updated; the
   desktop change is tracked as a `codex-task` issue.
2. The toolbar-sharing evidence is an undated forum reply from the 3.x era.
   Confirm it still holds in 5.2 (low risk).

## Appendix A — tools, shortcuts, toolbar (docs checked 2026-11-12)

Shortcuts are from [Keyboard & Mouse][kb] (rev. 14 Oct 2026). Repeating a key
cycles through tools that share it. Toolbar options come from each tool's page.

| Tool (key) | Toolbar options |
| --- | --- |
| Move Pixels (M), Move Selection (M×2) | resampling (6 modes, Bicubic default), gamma corrected/ignore |
| Rectangle (S), Lasso (S×2), Ellipse (S×3) Select | selection mode; rect: Any/Fixed Ratio/Fixed Size |
| Magic Wand (S×4) | mode, flood, tolerance, tolerance alpha, sampling |
| Zoom (Z), Pan (H; Space = temporary) | — |
| Paint Bucket (F) | flood, tolerance, tolerance alpha, fill style, sampling (Ctrl = Image), AA, blend |
| Gradient (G) | 7 types (Linear, Reflected, Diamond, Radial, Conical, Spiral CW/CCW), colour/transparency mode, repeat, blend |
| Paintbrush (B) | size, pressure, hardness, spacing, smoothing, fill style, AA, blend, clip |
| Eraser (E) | size, pressure, hardness, spacing, fill style (5.2), smoothing, AA, clip |
| Pencil (P) | blend (incl. Overwrite); 1 px, no AA |
| Color Picker (K) | sampling layer/image, size, after click |
| Clone Stamp (L; Ctrl+click = source) | size, pressure, hardness, spacing, smoothing, AA, blend, clip |
| Recolor (R) | size, pressure, hardness, spacing, tolerance, sampling, smoothing, AA, clip |
| Text (T) | font, size (pt or fixed 96 DPI), formatting, rendering mode, justification, AA, blend |
| Line/Curve (O), Shapes (O×2; A cycles shapes) | shape, draw mode, size, line/outline/fill styles, AA, blend |

[tw]: https://paint.net/doc/latest/ToolsWindow.html
[tb]: https://paint.net/doc/latest/Toolbar.html
[sel]: https://paint.net/doc/latest/SelectionTools.html
[cp]: https://paint.net/doc/latest/ColorPicker.html
[kb]: https://paint.net/doc/latest/KeyboardMouseCommands.html
[share]: https://forums.getpaint.net/topic/24131-can-paintn-only-save-preferences-for-one-tool/
