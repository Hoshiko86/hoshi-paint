//! Disposable Slint adapter around the UI-independent controller.

use hoshi_controller::{ColorSlot, Controller, ControllerError, ShapeMode, Tool};
use slint::ComponentHandle;
use std::{cell::RefCell, rc::Rc};
mod colors;
mod history;
mod screen_pick;

slint::slint! {
    import { ColorPanel, PaletteSwatch, WorkspaceColors } from "../ui/colors.slint";
    import { HistoryPanel, HistoryRow } from "../ui/history.slint";
    export { HistoryWindow, HistoryRow } from "../ui/history.slint";
    export { ColorPickerWindow, PaletteSwatch } from "../ui/colors.slint";
    import { Button, LineEdit, AboutSlint, Palette, ComboBox, SpinBox } from "std-widgets.slint";
    export struct ControlPoint { x: float, y: float, kind: int }
    export component AppWindow inherits Window {
        title: "Hoshi Paint · Canvas study";
        preferred-width: 1120px;
        preferred-height: 760px;
        min-width: max(960px, 236px + root.dock-left-width + root.dock-right-width + 240px);
        min-height: 680px;
        background: #f2f4f8;
        default-font-family: "DejaVu Sans";
        default-font-size: 14px;
        // Match widgets to the shell's fixed light surfaces on dark-themed Windows.
        init => { Palette.color-scheme = ColorScheme.light; }
        in property <image> canvas-image;
        in property <[ControlPoint]> control-points;
        in property <float> zoom: 1;
        in property <float> offset-x;
        in property <float> offset-y;
        in property <string> status;
        in property <string> layer-name;
        in property <bool> has-shape;
        in property <bool> can-undo;
        in property <bool> can-redo;
        in property <int> selected-tool;
        in property <int> shape-mode;
        in property <int> stroke-width: 2;
        in property <color> workspace-primary;
        in property <color> workspace-secondary;
        in property <string> workspace-primary-description;
        in property <string> workspace-secondary-description;
        callback swap-colors();
        callback reset-colors();
        in property <int> color-slot;
        changed shape-mode => { shape-mode-choice.current-index = root.shape-mode; }
        callback select-shape-mode(int);
        callback edit-stroke-width(int);
        callback select-color-slot(int);
        in property <color> selected-color;
        in property <string> selected-hex;
        private property <bool> about-open;
        out property <bool> modal-visible: root.about-open;
        out property <float> canvas-left: viewport.absolute-position.x / 1px;
        out property <float> canvas-top: viewport.absolute-position.y / 1px;
        out property <float> layer-input-left: layer-input.absolute-position.x / 1px;
        out property <float> layer-input-top: layer-input.absolute-position.y / 1px;
        out property <string> layer-edit-text: layer-input.text;
        out property <float> rectangle-tool-left: rectangle-tool.absolute-position.x / 1px;
        out property <float> rectangle-tool-top: rectangle-tool.absolute-position.y / 1px;
        out property <float> rectangle-tool-width: rectangle-tool.width / 1px;
        out property <float> rectangle-tool-height: rectangle-tool.height / 1px;
        callback pointer-down(float, float);
        callback pointer-move(float, float);
        callback pointer-up(float, float);
        callback pointer-cancel();
        callback begin-pan(float, float);
        callback end-pan(float, float);
        callback pan-by(float, float);
        private property <bool> navigation-space;
        private property <PointerEventButton> navigation-owner: PointerEventButton.other;
        private property <bool> navigation-panning;
        private property <bool> navigation-space-pan;
        callback clear-navigation-state();
        clear-navigation-state => {
            root.navigation-space = false;
            root.navigation-owner = PointerEventButton.other;
            root.navigation-panning = false;
            root.navigation-space-pan = false;
        }
        callback action(int);
        callback choose-tool(int);
        callback shift-modifier(bool);
        out property <float> docked-red-left: docked-colors.red-slider-left;
        out property <float> docked-red-top: docked-colors.red-slider-top;
        out property <float> docked-red-width: docked-colors.red-slider-width;
        out property <float> docked-red-height: docked-colors.red-slider-height;
        out property <float> docked-swatches-left: docked-colors.swatches-left;
        out property <float> docked-swatches-top: docked-colors.swatches-top;
        in property <[PaletteSwatch]> color-swatches;
        in property <int> colors-placement;
        in property <bool> colors-docked;
        in property <bool> colors-visible;
        in property <int> color-r;
        in property <int> color-g;
        in property <int> color-b;
        in property <int> color-a: 255;
        in property <float> color-h;
        in property <float> color-s;
        in property <float> color-v;
        in property <color> color-original;
        in property <string> color-original-description;
        in property <string> color-current-description;
        callback restore-original-color();
        in property <string> color-hex;
        in property <string> color-error;
        in property <image> color-sv;
        // Screen picking is temporary; the selected drawing tool stays active.
        in property <bool> screen-picking;
        callback edit-rgba(int, int, int, int);
        callback edit-hsv(float, float, float);
        callback edit-hex(string);
        callback pick-screen();
        callback set-colors-placement(int);
        callback toggle-color-dock();
        callback close-colors();
        callback show-colors();
        callback hide-colors();
        callback sync-color(int, int, int, int, bool);
        callback choose-color(int);
        callback rename(string);
        callback zoom-at(float, float, float);
        // Independent placement for each adjustment panel. Same-edge panels
        // share a row/column without covering the drawing viewport.
        in property <int> history-placement;
        in property <bool> history-visible;
        in property <int> history-position;
        in property <[HistoryRow]> history-rows;
        in property <bool> history-can-jump;
        in property <string> history-status;
        callback show-history();
        callback close-history();
        callback set-history-placement(int);
        callback history-jump(int);
        callback history-undo();
        callback history-redo();
        callback sync-history();
        private property <bool> color-host-visible: root.colors-visible && root.colors-docked && root.colors-placement > 0;
        private property <bool> history-host-visible: root.history-visible && root.history-placement > 0;
        private property <length> dock-left-width: (root.color-host-visible && root.colors-placement == 1 ? 434px : 0px) + (root.history-host-visible && root.history-placement == 1 ? 314px : 0px);
        private property <length> dock-right-width: (root.color-host-visible && root.colors-placement == 2 ? 434px : 0px) + (root.history-host-visible && root.history-placement == 2 ? 314px : 0px);
        private property <bool> has-top-dock: (root.color-host-visible && root.colors-placement == 3) || (root.history-host-visible && root.history-placement == 3);
        private property <bool> has-bottom-dock: (root.color-host-visible && root.colors-placement == 4) || (root.history-host-visible && root.history-placement == 4);
        private property <int> dock-row-count: (root.has-top-dock ? 1 : 0) + (root.has-bottom-dock ? 1 : 0);
        private property <length> dock-row-height: min(264px, (workspace.height - 200px) / max(1, root.dock-row-count));
        private property <length> dock-top-height: root.has-top-dock ? root.dock-row-height : 0px;
        private property <length> dock-bottom-height: root.has-bottom-dock ? root.dock-row-height : 0px;
        out property <float> canvas-width: viewport.width / 1px;
        out property <float> canvas-height: viewport.height / 1px;
        out property <float> colors-host-left: docked-colors.absolute-position.x / 1px;
        out property <float> colors-host-top: docked-colors.absolute-position.y / 1px;
        out property <float> colors-host-width: docked-colors.width / 1px;
        out property <float> colors-host-height: docked-colors.height / 1px;
        out property <float> history-host-left: docked-history.absolute-position.x / 1px;
        out property <float> history-host-top: docked-history.absolute-position.y / 1px;
        out property <float> history-host-width: docked-history.width / 1px;
        out property <float> history-host-height: docked-history.height / 1px;
        callback show-about();
        show-about => { root.hide-colors(); root.close-history(); root.pointer-cancel(); root.about-open = true; about-focus.focus(); }
        function close-about() { root.about-open = false; canvas-focus.focus(); }
        MenuBar {
            Menu {
                title: "Help";
                MenuItem {
                    title: "About Hoshi Paint";
                    activated => { root.show-about(); }
                }
            }
        }
        VerticalLayout {
            padding: 16px;
            spacing: 12px;
            HorizontalLayout {
                height: 44px;
                spacing: 16px;
                Text { text: "HOSHI PAINT"; font-size: 22px; font-weight: 700; color: #18253c; }
                Text { text: "Canvas study / 01"; color: #6c778b; vertical-alignment: center; }
                Rectangle { horizontal-stretch: 1; }
                Text { text: "Shape workflow"; color: #6c778b; vertical-alignment: center; }
            }
            HorizontalLayout {
                height: 40px;
                spacing: 8px;
                Button { icon: @image-url("../icons/undo.svg"); width: 40px; accessible-label: "Undo"; enabled: root.can-undo || root.has-shape; opacity: self.enabled ? 1 : 0.4;
                    Tooltip { text: @markdown("Undo · Ctrl/Cmd+Z"); } clicked => { root.action(0); } }
                Button { icon: @image-url("../icons/redo.svg"); width: 40px; accessible-label: "Redo"; enabled: root.can-redo && !root.has-shape; opacity: self.enabled ? 1 : 0.4;
                    Tooltip { text: @markdown("Redo · Ctrl/Cmd+Shift+Z"); } clicked => { root.action(1); } }
                Button { text: "History…"; clicked => { root.show-history(); } }
                Button { text: "Commit · Enter"; enabled: root.has-shape; clicked => { root.action(2); } }
                Button { text: "Cancel · Esc"; enabled: root.has-shape; clicked => { root.action(3); } }
                Rectangle { horizontal-stretch: 1; }
                Button { text: "−"; accessible-label: "Zoom out"; Tooltip { text: @markdown("Zoom out"); } clicked => { root.zoom-at(256, 256, -1); } }
                Button { text: "Reset view"; clicked => { root.action(4); } }
                Button { text: "+"; accessible-label: "Zoom in"; Tooltip { text: @markdown("Zoom in"); } clicked => { root.zoom-at(256, 256, 1); } }
            }
            workspace := Rectangle {
                vertical-stretch: 1;
                VerticalLayout {
                    x: 0px; y: 0px; width: 190px; height: parent.height;
                    spacing: 8px;
                    Text { text: "TOOLS"; color: #6c778b; font-size: 12px; font-weight: 700; }
                    rectangle-tool := Button { text: root.selected-tool == 0 ? "● Rectangle" : "Rectangle"; clicked => { root.choose-tool(0); } }
                    Button { text: root.selected-tool == 1 ? "● Move shape" : "Move shape"; enabled: root.has-shape; clicked => { root.choose-tool(1); } }
                    Button { text: root.selected-tool == 2 ? "● Pan canvas" : "Pan canvas"; clicked => { root.choose-tool(2); } }
                    Rectangle { height: 12px; }
                    Text { text: "SHAPE STYLE"; color: #6c778b; font-size: 12px; font-weight: 700; }
                    shape-mode-choice := ComboBox { model: ["Fill", "Outline", "Fill and outline"]; current-index: root.shape-mode;
                        accessible-label: "Shape style";
                        Tooltip { text: @markdown("Fill or outline alone uses Primary. Both uses Primary outline and Secondary fill."); }
                        selected(value) => { root.select-shape-mode(self.current-index); } }
                    Text { text: "Outline width (px)"; color: #526079; font-size: 12px; }
                    SpinBox { minimum: 1; maximum: 65536; value: root.stroke-width; enabled: root.shape-mode != 0;
                        accessible-label: "Outline width in canvas pixels";
                        edited(value) => { root.edit-stroke-width(value); } }
                    Text { text: "COLOR"; color: #6c778b; font-size: 12px; font-weight: 700; }
                    WorkspaceColors {
                        primary: root.workspace-primary; secondary: root.workspace-secondary;
                        primary-description: root.workspace-primary-description; secondary-description: root.workspace-secondary-description;
                        slot: root.color-slot; picking: root.screen-picking;
                        select(value) => { root.select-color-slot(value); }
                        swap => { root.swap-colors(); } reset => { root.reset-colors(); }
                    }
                    Button { text: root.color-slot == 0 ? "Primary · Colors…" : "Secondary · Colors…"; clicked => { root.show-colors(); } }
                    Text { text: root.selected-hex; color: #526079; font-size: 12px; }
                    Rectangle { height: 12px; }
                    Text { text: "LAYER NAME"; color: #6c778b; font-size: 12px; font-weight: 700; }
                    layer-input := LineEdit { text: root.layer-name; placeholder-text: "Layer name"; edited(value) => { root.rename(value); self.text = root.layer-name; } }
                    Rectangle { vertical-stretch: 1; }
                    Text { text: "Drag handles to resize.\nRound handle rotates.\nEnter commits. Esc cancels."; wrap: word-wrap; color: #6c778b; font-size: 12px; }
                }
                viewport := Rectangle {
                    x: 204px + root.dock-left-width;
                    y: root.dock-top-height;
                    width: parent.width - 204px - root.dock-left-width - root.dock-right-width;
                    height: parent.height - root.dock-top-height - root.dock-bottom-height;
                    background: #dce2eb;
                    clip: true;
                    border-width: 1px;
                    border-color: #c6cfdd;
                    accessible-role: image;
                    accessible-label: "Drawing canvas. Drag to draw; Enter commits; Escape cancels.";
                    Image {
                        source: root.canvas-image;
                        x: root.offset-x * 1px;
                        y: root.offset-y * 1px;
                        width: 512px * root.zoom;
                        height: 512px * root.zoom;
                        image-fit: fill;
                    }
                    if root.has-shape : Path {
                        width: parent.width;
                        height: parent.height;
                        viewbox-width: self.width / 1px;
                        viewbox-height: self.height / 1px;
                        stroke: #315cae;
                        stroke-width: 1px;
                        fill: transparent;
                        MoveTo { x: root.control-points[0].x; y: root.control-points[0].y; }
                        LineTo { x: root.control-points[2].x; y: root.control-points[2].y; }
                        LineTo { x: root.control-points[4].x; y: root.control-points[4].y; }
                        LineTo { x: root.control-points[6].x; y: root.control-points[6].y; }
                        Close { }
                        MoveTo { x: root.control-points[1].x; y: root.control-points[1].y; }
                        LineTo { x: root.control-points[8].x; y: root.control-points[8].y; }
                    }
                    for point in root.control-points : Rectangle {
                        x: point.x * 1px - 5px;
                        y: point.y * 1px - 5px;
                        width: 10px;
                        height: 10px;
                        border-width: 1.5px;
                        border-color: #315cae;
                        border-radius: point.kind == 8 ? 5px : 1px;
                        background: point.kind == 8 ? #c8ddff : white;
                    }
                    canvas-focus := FocusScope {
                        width: parent.width;
                        height: parent.height;
                        focus-lost(reason) => {
                            root.navigation-space = false;
                            if reason == FocusReason.window-activation || root.navigation-panning { root.pointer-cancel(); }
                        }
                        key-pressed(event) => {
                            if event.text == Key.Space && !event.modifiers.control && !event.modifiers.meta && !event.modifiers.alt {
                                root.navigation-space = true; return accept;
                            }
                            if !event.modifiers.control && !event.modifiers.meta && !event.modifiers.alt {
                                if event.text == "x" || event.text == "X" { root.swap-colors(); return accept; }
                                if event.text == "d" || event.text == "D" { root.reset-colors(); return accept; }
                            }
                            if event.text == Key.Shift { root.shift-modifier(true); return accept; }
                            if event.text == Key.Return { root.action(2); return accept; }
                            if event.text == Key.Escape {
                                if root.navigation-panning {
                                    root.end-pan(touch.mouse-x / 1px, touch.mouse-y / 1px);
                                    root.clear-navigation-state();
                                } else { root.action(3); }
                                return accept;
                            }
                            if event.modifiers.control || event.modifiers.meta {
                                if event.text == "z" { root.action(event.modifiers.shift ? 1 : 0); return accept; }
                                if event.text == "y" { root.action(1); return accept; }
                            }
                            reject
                        }
                        key-released(event) => {
                            if event.text == Key.Space {
                                root.navigation-space = false;
                                if root.navigation-space-pan && root.navigation-owner == PointerEventButton.left {
                                    root.end-pan(touch.mouse-x / 1px, touch.mouse-y / 1px);
                                    root.clear-navigation-state();
                                }
                                return accept;
                            }
                            if event.text == Key.Shift { root.shift-modifier(false); return accept; }
                            reject
                        }
                        touch := TouchArea {
                            enabled: !root.about-open;
                            mouse-cursor: root.screen-picking ? MouseCursor.custom(@image-url("../icons/eye-dropper-cursor.svg"), 3, 29) : MouseCursor.default;
                            pointer-event(event) => {
                                if event.kind == PointerEventKind.cancel {
                                    root.pointer-cancel();
                                } else if event.kind == PointerEventKind.down && root.navigation-owner == PointerEventButton.other {
                                    if event.button == PointerEventButton.left || event.button == PointerEventButton.middle || event.button == PointerEventButton.right {
                                        canvas-focus.focus();
                                        root.navigation-owner = event.button;
                                        root.navigation-space-pan = event.button == PointerEventButton.left && root.navigation-space;
                                        root.navigation-panning = event.button != PointerEventButton.left || root.navigation-space || root.selected-tool == 2;
                                        if root.navigation-panning {
                                            root.begin-pan(self.mouse-x / 1px, self.mouse-y / 1px);
                                        } else {
                                            root.shift-modifier(event.modifiers.shift);
                                            root.pointer-down(self.mouse-x / 1px, self.mouse-y / 1px);
                                        }
                                    }
                                } else if event.kind == PointerEventKind.move && root.navigation-owner != PointerEventButton.other {
                                    root.pointer-move(self.mouse-x / 1px, self.mouse-y / 1px);
                                } else if event.kind == PointerEventKind.up && event.button == root.navigation-owner {
                                    if root.navigation-panning {
                                        root.end-pan(self.mouse-x / 1px, self.mouse-y / 1px);
                                    } else {
                                        root.shift-modifier(event.modifiers.shift);
                                        root.pointer-up(self.mouse-x / 1px, self.mouse-y / 1px);
                                    }
                                    root.navigation-owner = PointerEventButton.other;
                                    root.navigation-panning = false;
                                    root.navigation-space-pan = false;
                                }
                            }
                            scroll-event(event) => {
                                if event.modifiers.control || event.modifiers.meta {
                                    root.zoom-at(self.mouse-x / 1px, self.mouse-y / 1px, event.delta-y / 1px / 120);
                                } else if event.modifiers.shift {
                                    root.pan-by((event.delta-x + event.delta-y) / 1px, 0);
                                } else {
                                    root.pan-by(event.delta-x / 1px, event.delta-y / 1px);
                                }
                                accept
                            }
                        }
                    }
                }
                docked-colors := ColorPanel {
                    workspace-primary: root.workspace-primary; workspace-secondary: root.workspace-secondary;
                    primary-description: root.workspace-primary-description; secondary-description: root.workspace-secondary-description;
                    swap-colors => { root.swap-colors(); } reset-colors => { root.reset-colors(); }
                    color-slot: root.color-slot;
                    select-color-slot(value) => { root.select-color-slot(value); }
                    visible: root.color-host-visible;
                    x: root.colors-placement == 2 ? parent.width - 420px : 204px;
                    y: root.colors-placement == 4 ? parent.height - root.dock-row-height + 14px : (root.colors-placement == 3 ? 0px : root.dock-top-height);
                    width: (root.colors-placement == 3 || root.colors-placement == 4) ? ((parent.width - 204px) - (root.history-host-visible && root.history-placement == root.colors-placement ? 14px : 0px)) / (root.history-host-visible && root.history-placement == root.colors-placement ? 2 : 1) : 420px;
                    height: (root.colors-placement == 3 || root.colors-placement == 4) ? root.dock-row-height - 14px : parent.height - root.dock-top-height - root.dock-bottom-height;
                    docked: true;
                    placement: root.colors-placement;
                    placement-selected(value) => { root.set-colors-placement(value); }
                    swatches: root.color-swatches;
                    red: root.color-r; green: root.color-g; blue: root.color-b; alpha: root.color-a;
                    hue: root.color-h; saturation: root.color-s; brightness: root.color-v;
                    hex-value: root.color-hex; error: root.color-error;
                    sv-image: root.color-sv; current-color: root.selected-color;
                    original-color: root.color-original;
                    original-description: root.color-original-description;
                    current-description: root.color-current-description;
                    restore-original => { root.restore-original-color(); }
                    has-active-shape: root.has-shape; screen-picking: root.screen-picking;
                    set-rgba(r,g,b,a) => { root.edit-rgba(r,g,b,a); }
                    set-hsv(h,s,v) => { root.edit-hsv(h,s,v); }
                    set-hex(text) => { root.edit-hex(text); }
                    pick-screen => { root.pick-screen(); }
                    toggle-dock => { root.toggle-color-dock(); }
                    close-picker => { root.close-colors(); }
                }
                docked-history := HistoryPanel {
                    visible: root.history-host-visible;
                    x: root.history-placement == 2 ? parent.width - 300px - (root.color-host-visible && root.colors-placement == 2 ? 434px : 0px) : 204px + (root.color-host-visible && root.colors-placement == root.history-placement ? docked-colors.width + 14px : 0px);
                    y: root.history-placement == 4 ? parent.height - root.dock-row-height + 14px : (root.history-placement == 3 ? 0px : root.dock-top-height);
                    width: (root.history-placement == 3 || root.history-placement == 4) ? ((parent.width - 204px) - (root.color-host-visible && root.colors-placement == root.history-placement ? 14px : 0px)) / (root.color-host-visible && root.colors-placement == root.history-placement ? 2 : 1) : 300px;
                    height: (root.history-placement == 3 || root.history-placement == 4) ? root.dock-row-height - 14px : parent.height - root.dock-top-height - root.dock-bottom-height;
                    placement: root.history-placement;
                    rows: root.history-rows;
                    current-index: root.history-position;
                    can-jump: root.history-can-jump;
                    status-text: root.history-status;
                    can-undo: root.can-undo || root.has-shape;
                    can-redo: root.can-redo && !root.has-shape;
                    has-shape: root.has-shape;
                    placement-selected(value) => { root.set-history-placement(value); }
                    close-panel => { root.close-history(); }
                    jump(value) => { root.history-jump(value); }
                    undo => { root.history-undo(); }
                    redo => { root.history-redo(); }
                }
            }
            Text { height: 22px; text: root.status; color: #526079; font-size: 12px; }
        }
        Rectangle {
            visible: root.about-open;
            width: parent.width;
            height: parent.height;
            background: #18253c99;
            about-focus := FocusScope {
                width: parent.width;
                height: parent.height;
                TouchArea {
                    pointer-event(event) => { about-focus.focus(); }
                    clicked => { about-focus.focus(); }
                }
                key-pressed(event) => {
                    if event.text == Key.Escape || event.text == Key.Return { root.close-about(); }
                    accept
                }
                Rectangle {
                    x: (parent.width - self.width) / 2;
                    y: (parent.height - self.height) / 2;
                    width: 460px;
                    height: 360px;
                    background: white;
                    border-radius: 12px;
                    VerticalLayout {
                        padding: 24px;
                        spacing: 16px;
                        Text { text: "Hoshi Paint"; color: #18253c; font-size: 24px; font-weight: 700; }
                        Text { text: "Copyright 2026 Hoshi Paint contributors\nGNU GPL version 3 only · GPL-3.0-only"; wrap: word-wrap; color: #526079; }
                        Text { text: "Icons: Font Awesome Free 6.7.2, Fonticons, Inc.\nCC BY 4.0 · fontawesome.com\nCursor adapted with contrast outline. See THIRD_PARTY_NOTICES.md."; wrap: word-wrap; color: #526079; font-size: 11px; }
                        AboutSlint { }
                        Button { text: "Close"; clicked => { root.close-about(); } }
                    }
                }
            }
        }
    }
}

pub type SharedController = Rc<RefCell<Controller>>;

fn refresh(ui: &AppWindow, controller: &Controller) {
    let document = controller.document();
    let rgba = controller.preview_rgba();
    let mut pixels =
        slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(document.width(), document.height());
    pixels.make_mut_bytes().copy_from_slice(&rgba);
    ui.set_canvas_image(slint::Image::from_rgba8(pixels));
    let viewport = controller.viewport();
    ui.set_zoom(viewport.zoom as f32);
    ui.set_offset_x(viewport.offset_x as f32);
    ui.set_offset_y(viewport.offset_y as f32);
    let points = controller
        .handle_positions()
        .into_iter()
        .map(|p| ControlPoint {
            x: p.x as f32,
            y: p.y as f32,
            kind: p.kind,
        })
        .collect::<Vec<_>>();
    ui.set_control_points(Rc::new(slint::VecModel::from(points)).into());
    ui.set_has_shape(document.active_shape().is_some());
    ui.set_can_undo(document.undo_len() > 0);
    ui.set_can_redo(document.redo_len() > 0);
    ui.set_selected_tool(match controller.tool() {
        Tool::Rectangle => 0,
        Tool::MoveShape => 1,
        Tool::Pan => 2,
        Tool::Eyedropper => 3,
    });
    history::refresh(ui, controller);
    ui.set_shape_mode(match controller.shape_mode() {
        ShapeMode::Fill => 0,
        ShapeMode::Outline => 1,
        ShapeMode::Both => 2,
    });
    ui.set_stroke_width(controller.stroke_width() as i32);
    ui.set_color_slot(if controller.color_slot() == ColorSlot::Primary {
        0
    } else {
        1
    });
    let primary = controller.primary_color();
    let secondary = controller.secondary_color();
    let primary_ui = slint::Color::from_argb_u8(primary.a, primary.r, primary.g, primary.b);
    let secondary_ui =
        slint::Color::from_argb_u8(secondary.a, secondary.r, secondary.g, secondary.b);
    if ui.get_workspace_primary() != primary_ui {
        ui.set_workspace_primary(primary_ui);
        ui.set_workspace_primary_description(
            colors::describe_workspace_color(primary, "Primary").into(),
        );
    }
    if ui.get_workspace_secondary() != secondary_ui {
        ui.set_workspace_secondary(secondary_ui);
        ui.set_workspace_secondary_description(
            colors::describe_workspace_color(secondary, "Secondary").into(),
        );
    }
    let color = controller.selected_color();
    ui.set_selected_color(slint::Color::from_argb_u8(
        color.a, color.r, color.g, color.b,
    ));
    ui.set_selected_hex(hoshi_controller::color::format_hex(color).into());
    ui.invoke_sync_color(
        color.r.into(),
        color.g.into(),
        color.b.into(),
        color.a.into(),
        document.active_shape().is_some(),
    );
    ui.set_layer_name(document.layers()[document.selected_layer()].name().into());
    let shape = document
        .active_shape()
        .map(|s| {
            format!(
                " · Active shape: {} × {} at {}, {} · {}°",
                s.width,
                s.height,
                s.x,
                s.y,
                s.rotation_degrees.rem_euclid(360)
            )
        })
        .unwrap_or_default();
    ui.set_status(
        format!(
            "512 × 512 · {:.0}% · Undo: {}{}",
            viewport.zoom * 100.0,
            document.undo_len(),
            shape
        )
        .into(),
    );
}

fn update(
    ui: &slint::Weak<AppWindow>,
    controller: &SharedController,
    action: impl FnOnce(&mut Controller) -> Result<(), ControllerError>,
) {
    if let Some(ui) = ui.upgrade() {
        if ui.get_modal_visible() {
            return;
        }
        let mut controller = controller.borrow_mut();
        let result = action(&mut controller);
        refresh(&ui, &controller);
        if let Err(error) = result {
            ui.set_status(format!("Action unavailable: {error:?}").into());
        }
    }
}

pub fn create_app() -> Result<(AppWindow, SharedController), slint::PlatformError> {
    let (ui, controller, _) = create_app_with_colors()?;
    Ok((ui, controller))
}

pub fn create_app_with_colors()
-> Result<(AppWindow, SharedController, Rc<ColorPickerWindow>), slint::PlatformError> {
    let (ui, controller, colors, _) = create_app_with_panels()?;
    Ok((ui, controller, colors))
}

pub fn create_app_with_panels() -> Result<
    (
        AppWindow,
        SharedController,
        Rc<ColorPickerWindow>,
        Rc<HistoryWindow>,
    ),
    slint::PlatformError,
> {
    let ui = AppWindow::new()?;
    let controller = Rc::new(RefCell::new(Controller::default()));
    refresh(&ui, &controller.borrow());
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_shift_modifier(move |pressed| {
        if state.borrow().aspect_lock() == pressed {
            return;
        }
        update(&weak, &state, |c| c.set_aspect_lock(pressed));
    });
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_pointer_down(move |x, y| update(&weak, &state, |c| c.pointer_down(x.into(), y.into())));
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_pointer_move(move |x, y| update(&weak, &state, |c| c.pointer_move(x.into(), y.into())));
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_pointer_up(move |x, y| {
        update(&weak, &state, |c| {
            let movement = c.pointer_move(x.into(), y.into());
            let release = c.pointer_up();
            movement.and(release)
        })
    });
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_pointer_cancel(move || {
        if let Some(ui) = weak.upgrade() {
            ui.invoke_clear_navigation_state();
        }
        update(&weak, &state, |c| {
            if c.is_dragging() {
                c.cancel_pointer();
            }
            c.set_aspect_lock(false)
        })
    });
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_begin_pan(move |x, y| {
        update(&weak, &state, |c| c.begin_temporary_pan(x.into(), y.into()))
    });
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_end_pan(move |x, y| {
        update(&weak, &state, |c| {
            let movement = c.pointer_move(x.into(), y.into());
            let release = c.end_temporary_pan();
            movement.and(release)
        })
    });
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_pan_by(move |dx, dy| update(&weak, &state, |c| c.pan(dx.into(), dy.into())));
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_choose_tool(move |tool| {
        if tool == 3 {
            if let Some(ui) = weak.upgrade() {
                ui.invoke_pick_screen();
            }
            return;
        }
        update(&weak, &state, |c| {
            c.set_tool(match tool {
                1 => Tool::MoveShape,
                2 => Tool::Pan,
                3 => Tool::Eyedropper,
                _ => Tool::Rectangle,
            })
        })
    });
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_choose_color(move |color| {
        update(&weak, &state, |c| {
            c.set_selected_color(match color {
                1 => hoshi_controller::Color::rgba(232, 104, 92, 255),
                2 => hoshi_controller::Color::rgba(54, 167, 140, 255),
                _ => hoshi_controller::Color::rgba(72, 136, 240, 255),
            })
        })
    });
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_rename(move |name| update(&weak, &state, |c| c.rename_layer(&name)));
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_select_shape_mode(move |mode| {
        update(&weak, &state, |c| {
            c.set_shape_mode(match mode {
                1 => ShapeMode::Outline,
                2 => ShapeMode::Both,
                _ => ShapeMode::Fill,
            })
        });
    });
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_edit_stroke_width(move |width| {
        update(&weak, &state, |c| c.set_stroke_width(width.max(0) as u32))
    });

    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_zoom_at(move |x, y, delta| {
        update(&weak, &state, |c| {
            c.zoom_at(x.into(), y.into(), 1.2_f64.powf(delta.into()))
        })
    });
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_action(move |action| {
        update(&weak, &state, |c| match action {
            0 => c.undo().map(|_| ()),
            1 => c.redo().map(|_| ()),
            2 => c.commit().map(|_| ()),
            3 => {
                c.cancel();
                Ok(())
            }
            4 => {
                c.reset_view();
                Ok(())
            }
            5 => c.nudge_shape(-1, 0),
            6 => c.nudge_shape(1, 0),
            7 => c.nudge_shape(0, -1),
            8 => c.nudge_shape(0, 1),
            9 | 10 => {
                let s = c
                    .document()
                    .active_shape()
                    .ok_or(ControllerError::NoActiveShape)?;
                let size = |n: u32| {
                    if action == 9 {
                        n.saturating_sub(8).max(1)
                    } else {
                        n.saturating_add(8)
                    }
                };
                c.set_shape_bounds(s.x, s.y, size(s.width), size(s.height))
            }
            _ => Ok(()),
        })
    });
    let picker = colors::bind_picker(&ui, controller.clone())?;
    let history = history::bind_window(&ui, controller.clone())?;
    Ok((ui, controller, picker, history))
}
