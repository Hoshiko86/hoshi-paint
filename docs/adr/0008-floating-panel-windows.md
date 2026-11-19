# 0008. Floating panels: owned tool windows (Windows/macOS), in-window panels (Linux/iPad)

- Status: Accepted (owner, 2026-11-19)
- Date: 2026-11-19
- Deciders: owner (approve), Claude (architect)
- Related: ADR 0006 (Slint), ADR 0007 (theme), ADR 0005 (platform services)

## Context

A floating panel (Colors, History, Shape Properties, and soon Brush) is a
separate Slint `Window`, which is a plain top-level OS window today. Owner
report (2026-11-19):

- Panels hide behind other apps' windows when Hoshi Paint has focus.
- They look like separate applications: their own OS title bar and taskbar
  entry.

**Owner decisions (2026-11-19):**
- On Windows and macOS, panels stay real OS windows (they can move to a second
  monitor), but they are owned by the main window.
- On Linux, and later iPad, floating panels live inside the main window.

What the pinned stack offers. The sources checked 2026-11-19 are Slint 1.18.1
and winit 0.30.13 from `Cargo.lock`:

- **Slint feature `unstable-winit-030`.**
  - `BackendSelector::with_winit_window_attributes_hook` runs on every window's
    `WindowAttributes` before the window is created.
  - `WinitWindowAccessor` (`with_winit_window`, `on_winit_window_event`) gives
    access to an existing window and its events.
  - The API is unstable across Slint minors, but the version is pinned
    (`=1.18.1`).
- **Windows:** `WindowAttributesExtWindows::with_owner_window(HWND)` makes an
  owned window: it stays above its owner, minimizes and restores with it, and is
  hidden with it. `with_skip_taskbar(true)` removes the taskbar button.
- **macOS:** `with_parent_window(handle)` calls
  `addChildWindow_ordered(…, NSWindowAbove)`. The panel stays above the main
  window and moves with it.
- **X11:** `with_parent_window` embeds the panel, which confines it to the
  parent's client area, and winit has no `WM_TRANSIENT_FOR`.
- **Wayland:** no parent or owner support in winit 0.30. Compositors also
  ignore client window positioning.
- So Linux has no reliable owned-window path.
- **All platforms:** `Window::drag_window()` and `drag_resize_window()` start
  an OS move or resize from our own UI.

## Decision

1. **Two floating modes, chosen per platform at compile time.** Docked panels
   are unchanged on every platform.
   - **Owned window (Windows, macOS):** the panel is its own OS window owned
     by the main window (items 2–3).
   - **In-window (Linux X11/Wayland, iPad later):** the panel is drawn inside
     the main window, above the canvas, and can be dragged within the main
     window (item 4). There is no OS window, so it can't hide behind other
     apps or appear in a taskbar, and it behaves the same on X11 and Wayland.
     iPad has a single window per scene, so it uses this mode too.
   - `apps/desktop/src/panel_windows.rs` exposes the mode as one
     `const FLOATING: FloatingMode` chosen with `cfg`. The rest of the app
     asks it, never `cfg` directly.

2. **Ownership through one attributes hook (owned-window mode).** The desktop
   app builds its winit backend with an attributes hook. A thread-local
   "pending panel" value names the owner for the next window created.
   `panel_windows.rs` sets it just before showing a panel window and clears it
   afterwards. Windows created without it, such as the main window and
   dialogs, are unchanged.
   - **Windows:** `with_owner_window(main HWND)` plus `with_skip_taskbar(true)`.
   - **macOS:** `with_parent_window(main handle)`, which makes it a child
     window.

3. **Panel chrome is ours (owned-window mode).** Floating panel windows use
   `with_decorations(false)`.
   - `ToolPanelFrame`'s header (title, placement menu, dock and close buttons)
     becomes the drag handle and calls `drag_window()` on press.
   - Its edges call `drag_resize_window()` on press.
   - Panels look like Hoshi panels and follow the theme (ADR 0007).

4. **In-window panels (Linux, iPad).** `main-window.slint` gets one floating
   host per panel, above the canvas and docked hosts. Each reuses the same
   panel body the docked and window hosts use, so there is one body component
   per panel and no duplicated UI.
   - Position and size are `AppWindow` properties, kept inside the main
     window's bounds when the window resizes.
   - `ToolPanelFrame`'s header drags the host and its edges resize it, both in
     Slint.
   - The panel layout (`panel_layout.rs`) treats floating panels as overlays:
     they don't change the canvas area.
   - In this mode the floating OS windows (`ColorPickerWindow`, …) are never
     shown.

5. **Translucent over the canvas** (owner, 2026-11-19). A floating panel that
   overlaps any part of the canvas viewport becomes translucent so the image
   shows through. It is fully opaque again while the pointer is over it or one
   of its controls has keyboard focus, so it stays usable, as with Paint.NET's
   translucent tool windows. A panel that doesn't overlap the canvas is always
   opaque. Docked panels never overlap the canvas.
   - **Opacity:** one compiled theme token, `floating-panel-opacity`
     (default 0.6), not themeable for now (ADR 0007 §1). The change animates
     over about 150 ms.
   - **Overlap:** the panel's rectangle against the canvas viewport's rectangle,
     both in screen coordinates. It is recomputed when the panel or the main
     window moves or resizes, and when the canvas layout changes.
   - **In-window mode (Linux, iPad):** Slint `opacity` on the floating host. It
     needs no platform code.
   - **Owned-window mode:** whole-window alpha through the OS, confined to
     `panel_windows.rs`. Windows: `WS_EX_LAYERED` +
     `SetLayeredWindowAttributes(LWA_ALPHA)` (`windows-sys` is already a
     dependency). macOS: `NSWindow::setAlphaValue` (`objc2-app-kit` is already
     a dependency; it adds the `NSWindow` feature). Rendering is unchanged.
     *(Unverified until slice 4's spike: layered-window alpha with Slint's
     software renderer on Windows.)*

6. **Tests.**
   - The hook and thread-local are unit-tested with fake attributes: the main
     window is untouched, panels get the owner settings, and the pending value
     is cleared after one window, including on error.
   - The in-window hosts are tested headless: drag, clamping to the window on
     resize, z-order above the canvas, and docking and undocking.
   - OS ownership (staying above, minimizing together, no taskbar entry) is
     checked by native smoke on Windows and macOS CI where it can be observed;
     otherwise it is a manual checklist item in `docs/TESTING.md`.
   - The headless testing backend skips the hook.
   - The overlap rule (rectangles, hover and focus → opaque) is a pure
     function with unit tests. In-window opacity is tested headless.

## Consequences

- Windows and macOS panels behave like Paint.NET's tool windows. Linux panels
  behave the same on X11 and Wayland, and iPad inherits the in-window mode.
- Linux users can't drag a floating panel onto a second monitor. This is
  accepted (owner), and docking still works.
- We depend on Slint's `unstable-winit-030` API on Windows and macOS. A Slint
  upgrade must re-check the hook, which is listed in ADR 0006's upgrade notes.
- Platform `cfg` code lives only in `panel_windows.rs`, in line with ADR 0005's
  platform boundary.
- On macOS, child windows move with the main window. That matches macOS
  palette conventions.

## Slices

1. **Owned windows (Windows/macOS):** backend builder with the hook,
   `panel_windows.rs` and `FloatingMode`, unit tests and smoke.
2. **Panel chrome:** undecorated floating windows, `ToolPanelFrame` drag and
   resize handles.
3. **In-window panels (Linux):** floating hosts in `main-window.slint`, drag,
   resize and clamping, `panel_layout` overlay handling, and the Linux
   `FloatingMode`.
4. **Translucent over the canvas:** the overlap rule, in-window opacity, and
   OS alpha on Windows/macOS (starting with a short spike on Windows
   layered-window alpha with the software renderer).
