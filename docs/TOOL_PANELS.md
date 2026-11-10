# Tool panels and the compact editor shell

The main window keeps an icon tool rail, document tabs, canvas actions and the drawing viewport. The layer-name field is in the footer. Shape position, dimensions and rendering settings belong to **Shape Properties**, rather than being repeated beside and above the canvas.

Click **Shapes** in the rail to select the shape tool and open its properties. The sliders icon in the action bar also opens Shape Properties without changing the selected tool. The shape-kind selector currently offers Rectangle; future shape tools can reuse the same properties host.

Shape Properties opens as a floating window. Its header can place it on the left, right, top or bottom, independently of Colors and History. Short docks scroll their content. Closing a panel hides it; reopening restores current values. Docking, floating and closing preserve the editable preview, rendering defaults, numeric aspect lock and undo history. Pending numeric field edits apply before a host is hidden.

History, commit/cancel, zoom, fit, reset-view, Colors and the Shapes tool use icons with hover tooltips and accessible names. Document names, field labels, dropdown choices and menu entries retain text because those convey values or choices. The UI uses Lucide 1.71.0 outline icons, including Undo/Redo; see THIRD_PARTY_NOTICES.md for the ISC and Feather MIT notices.

## Implementation boundaries

- `ui/shape-properties.slint` defines one properties body and its floating host, reusing the numeric bounds and rendering components.
- `src/shape_properties.rs` synchronizes both hosts and forwards actions to existing controller bindings. It does not duplicate document-editing logic.
- `src/panel_layout.rs` calculates dock rectangles without a UI or document dependency. Stable panel ordering, spacing, adaptive horizontal bands and minimum canvas dimensions work for all three panels.
- `src/panel_hosts.rs` applies those rectangles to the main view. Dock location and window size determine layout; document edits do not.

Adding another adjustment panel should extend the panel descriptions and host bindings, rather than copying pairwise positioning expressions or adding platform-specific windows. The core/controller remain shared across platforms.
