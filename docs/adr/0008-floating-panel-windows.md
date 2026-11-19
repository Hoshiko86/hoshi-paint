# 0008. Floating panels are owned tool windows

- Status: Proposed
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

**Owner decision (2026-11-19):** panels stay real OS windows (they can move to
a second monitor), but they are owned by the main window. The alternative,
panels drawn inside the main window, was declined.

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
  parent's client area, so we can't use it. `with_x11_window_type(Utility)`
  marks it as a utility window. winit has no `WM_TRANSIENT_FOR`.
- **Wayland:** no parent or owner support in winit 0.30. Compositors also
  ignore client window positioning.
- **All platforms:** `Window::drag_window()` and `drag_resize_window()` start
  an OS move or resize from our own UI.

## Decision

1. **Ownership through one attributes hook.** The desktop app builds its winit
   backend with an attributes hook. A thread-local "pending panel" value names
   the owner for the next window created. `apps/desktop/src/panel_windows.rs`
   sets it just before showing a panel window and clears it afterwards. Windows
   created without it, such as the main window and dialogs, are unchanged.
   - **Windows:** `with_owner_window(main HWND)` plus `with_skip_taskbar(true)`.
   - **macOS:** `with_parent_window(main handle)`, which makes it a child
     window.
   - **X11:** `with_x11_window_type(Utility)`, plus raise-on-focus (item 3).
   - **Wayland:** no ownership; raise-on-focus only (item 3). This is a known
     limitation, documented in `docs/PARITY.md`.

2. **Panel chrome is ours.** Floating panel windows use
   `with_decorations(false)`.
   - `ToolPanelFrame`'s header (title, placement menu, dock and close buttons)
     becomes the drag handle and calls `drag_window()` on press.
   - Its edges call `drag_resize_window()` on press.
   - Panels then look like Hoshi panels and follow the theme (ADR 0007),
     docked or floating.

3. **Raise-on-focus fallback.** On X11 and Wayland, when the main window gains
   focus (`WindowEvent::Focused(true)` via `on_winit_window_event`), visible
   floating panels are shown again so the compositor raises them. They are not
   focused, so focus stays on the canvas.

4. **Tests.**
   - The hook and thread-local are unit-tested with fake attributes: the main
     window is untouched, panels get the owner settings, and the pending value
     is cleared after one window, including on error.
   - OS behavior (staying above, minimizing together, no taskbar entry) is
     checked by native smoke on Windows and macOS CI, by reading back the
     window's owner or parent through the platform API. Where CI can't observe
     it, it is a manual checklist item in `docs/TESTING.md`.
   - The headless testing backend skips the hook.

## Consequences

- Panels behave like Paint.NET's tool windows on Windows and macOS. On Linux
  they behave reasonably: utility type plus raise-on-focus.
- We depend on Slint's `unstable-winit-030` API. A Slint upgrade must re-check
  the hook and accessor, which is listed in ADR 0006's upgrade notes.
- Platform `cfg` code lives only in `panel_windows.rs`, behind one small
  interface (`show_owned(panel, owner)`), in line with ADR 0005's platform
  boundary.
- On macOS, child windows move with the main window. That matches macOS
  palette conventions.

## Slices

1. Backend builder with the hook plus `panel_windows.rs`: ownership on
   Windows/macOS, the X11 utility type, raise-on-focus, unit tests and smoke.
2. Panel chrome: undecorated floating windows, `ToolPanelFrame` drag and
   resize handles.
