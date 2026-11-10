//! Disposable Slint adapter around the UI-independent controller.

use hoshi_controller::{ColorSlot, Controller, ControllerError, ShapeMode, Tool};
use slint::ComponentHandle;
use std::{cell::RefCell, rc::Rc};
mod colors;
pub mod documents;
pub mod files;
mod history;
mod image_settings;
mod input;
mod resources;
mod screen_pick;
mod shape_bounds;
mod shape_rendering;
pub mod workspace;

slint::slint! {
    import { ShapeBounds } from "../ui/shape-bounds.slint";
    import { ShapeRendering } from "../ui/shape-rendering.slint";
    import { ShapeControls, ControlPoint } from "../ui/shape-controls.slint";
    export { ControlPoint } from "../ui/shape-controls.slint";
    import { FileShortcuts } from "../ui/file-shortcuts.slint";
    import { SavePrompt, SaveConfiguration, FileErrorPrompt } from "../ui/file-prompts.slint";
    import { ImageSettings } from "../ui/image-settings.slint";
    import { ColorPanel, PaletteSwatch, WorkspaceColors, SwatchDetails } from "../ui/colors.slint";
    import { HistoryPanel, HistoryRow } from "../ui/history.slint";
    export { HistoryWindow, HistoryRow } from "../ui/history.slint";
    export { ColorPickerWindow, PaletteSwatch } from "../ui/colors.slint";
    import { Button, LineEdit, AboutSlint, Palette, ComboBox, SpinBox, ScrollView } from "std-widgets.slint";
    export struct DocumentTabRow { id: int, title: string, detail: string, thumbnail: image, active: bool, dirty: bool, generation: int }
    export component AppWindow inherits Window {
        in property <string> document-title: "Untitled · Hoshi Paint";
        title: root.document-title;
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
        in property <bool> viewport-preview;
        callback render-viewport();
        callback fit-canvas();
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
        in property <string> shape-x-text;
        in property <string> shape-y-text;
        in property <string> shape-width-text;
        in property <string> shape-height-text;
        callback edit-shape-bound(int, string, bool);
        in-out property <bool> shape-bounds-locked;
        callback bounds-lock-changed(bool);
        changed shape-bounds-locked => { root.bounds-lock-changed(root.shape-bounds-locked); }
        in property <int> shape-mode;
        in property <int> stroke-width: 2;
        in property <float> stroke-width-pixels: 2;
        in property <bool> shape-antialiasing: true;
        in property <int> line-style;
        in property <int> fill-pattern;
        callback edit-stroke-width-pixels(float);
        callback select-line-style(int);
        callback select-fill-pattern(int);
        callback toggle-shape-antialiasing(bool);
        callback adjust-outline-width(int);
        out property <float> outline-input-left: rendering-settings.width-left;
        out property <float> outline-input-top: rendering-settings.width-top;
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
        in property <bool> has-document: true;
        in property <bool> any-dirty;
        in property <[DocumentTabRow]> document-tabs;
        in property <[string]> recent-files;
        in property <bool> file-prompt-visible;
        in property <string> file-prompt-name;
        in property <bool> save-config-visible;
        in property <bool> save-config-jpeg;
        in property <string> save-config-warning;
        in-out property <int> save-quality: 95;
        in-out property <string> file-error;
        in property <bool> file-busy;
        callback file-action(int);
        callback activate-document(int);
        callback close-document(int);
        callback cycle-document(bool);
        callback open-recent(int);
        callback clear-recent();
        callback open-document(string);
        callback save-document-as(string);
        callback file-prompt-response(int);
        callback confirm-save-configuration(bool);
        callback focus-canvas();
        focus-canvas => { canvas-focus.focus(); }
        callback dismiss-file-error();
        dismiss-file-error => { root.file-error = ""; canvas-focus.focus(); }
        in property <int> image-width: 512;
        in property <int> image-height: 512;
        in property <bool> can-flatten;
        in-out property <int> image-dialog-kind: -1;
        in-out property <string> image-width-text;
        in-out property <string> image-height-text;
        in-out property <string> image-resolution-text;
        in-out property <string> image-percent-text;
        in-out property <int> image-units;
        in-out property <bool> image-aspect;
        in-out property <bool> image-percentage;
        in-out property <int> image-anchor: 4;
        in-out property <int> image-background;
        in-out property <int> image-resampling: 0;
        in-out property <bool> image-gamma-correction: true;
        in property <string> image-summary;
        in property <string> image-error;
        callback show-image-settings(int);
        callback edit-image-setting(int, string);
        callback select-image-units(int);
        callback select-image-aspect(bool);
        callback select-image-percentage(bool);
        callback apply-image-settings();
        callback close-image-settings();
        close-image-settings => { root.image-dialog-kind = -1; canvas-focus.focus(); }
        callback image-command(int);
        out property <bool> modal-visible: root.about-open || root.image-dialog-kind >= 0 || root.file-prompt-visible || root.save-config-visible || root.file-error != "" || root.file-busy;
        out property <float> canvas-left: viewport.absolute-position.x / 1px;
        out property <float> canvas-top: viewport.absolute-position.y / 1px;
        out property <float> layer-input-left: layer-input.absolute-position.x / 1px;
        out property <float> layer-input-top: layer-input.absolute-position.y / 1px;
        out property <string> layer-edit-text: layer-input.text;
        out property <float> rectangle-tool-left: rectangle-tool.absolute-position.x / 1px;
        out property <float> rectangle-tool-top: rectangle-tool.absolute-position.y / 1px;
        out property <float> rectangle-tool-width: rectangle-tool.width / 1px;
        out property <float> rectangle-tool-height: rectangle-tool.height / 1px;
        // Button/modifier events are normalized once in the Rust input adapter.
        callback pointer-down(float, float);
        // Logical x/y, secondary button, then Shift/Alt/Control (or Command).
        callback pointer-pressed(float, float, bool, bool, bool, bool);
        callback pointer-motion(float, float, bool, bool, bool);
        callback pointer-released(float, float, bool, bool, bool);
        callback input-modifiers(bool, bool, bool);
        callback keyboard-transform(int, int, bool, bool);
        callback hover-canvas(float, float);
        // Presentation values match the named EditCursor mapping in input.rs:
        // draw, move, horizontal, vertical, diagonals, rotation, movable pivot.
        in-out property <int> edit-cursor;
        changed has-shape => { if !root.has-shape { root.edit-cursor = 0; } }
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
        private property <length> dock-row-height: min(264px, (workspace.height - bounds-bar.height - 200px) / max(1, root.dock-row-count));
        private property <length> dock-top-height: root.has-top-dock ? root.dock-row-height : 0px;
        private property <length> dock-bottom-height: root.has-bottom-dock ? root.dock-row-height : 0px;
        out property <float> canvas-width: viewport.width / 1px;
        out property <float> canvas-height: viewport.height / 1px;
        changed canvas-width => { root.render-viewport(); }
        changed canvas-height => { root.render-viewport(); }
        out property <float> colors-host-left: docked-colors.absolute-position.x / 1px;
        out property <float> colors-host-top: docked-colors.absolute-position.y / 1px;
        out property <float> colors-host-width: docked-colors.width / 1px;
        out property <float> colors-host-height: docked-colors.height / 1px;
        out property <float> history-host-left: docked-history.absolute-position.x / 1px;
        out property <float> history-host-top: docked-history.absolute-position.y / 1px;
        out property <float> history-host-width: docked-history.width / 1px;
        out property <float> history-host-height: docked-history.height / 1px;
        callback show-about();
        show-about => { if !root.modal-visible { root.hide-colors(); root.close-history(); root.pointer-cancel(); root.about-open = true; about-focus.focus(); } }
        function close-about() { root.about-open = false; canvas-focus.focus(); }
        MenuBar {
            Menu { title: "File";
                MenuItem { title: "New…"; enabled: !root.modal-visible; activated => { root.file-action(7); } }
                MenuItem { title: "Open…"; enabled: !root.modal-visible; activated => { root.file-action(0); } }
                Menu { title: "Open Recent";
                    for path[index] in root.recent-files: MenuItem { title: path; enabled: !root.modal-visible; activated => { root.open-recent(index); } }
                    MenuSeparator { }
                    MenuItem { title: "Clear this list"; enabled: !root.modal-visible; activated => { root.clear-recent(); } }
                }
                MenuSeparator { }
                MenuItem { title: "Save"; enabled: root.has-document && !root.modal-visible; activated => { root.file-action(1); } }
                MenuItem { title: "Save As…"; enabled: root.has-document && !root.modal-visible; activated => { root.file-action(2); } }
                MenuItem { title: "Save All"; enabled: root.any-dirty && !root.modal-visible; activated => { root.file-action(3); } }
                MenuSeparator { }
                MenuItem { title: "Close"; enabled: root.has-document && !root.modal-visible; activated => { root.file-action(4); } }
                MenuItem { title: "Close All"; enabled: root.has-document && !root.modal-visible; activated => { root.file-action(5); } }
                MenuItem { title: "Exit"; enabled: !root.modal-visible; activated => { root.file-action(6); } }
            }
            Menu { title: "Image";
                MenuItem { title: "Crop to Selection"; enabled: false; }
                MenuSeparator { }
                MenuItem { title: "Resize…"; enabled: root.has-document && !root.modal-visible; activated => { root.show-image-settings(1); } }
                MenuItem { title: "Canvas Size…"; enabled: root.has-document && !root.modal-visible; activated => { root.show-image-settings(2); } }
                MenuSeparator { }
                MenuItem { title: "Flip Horizontal"; enabled: root.has-document && !root.modal-visible; activated => { root.image-command(0); } }
                MenuItem { title: "Flip Vertical"; enabled: root.has-document && !root.modal-visible; activated => { root.image-command(1); } }
                MenuItem { title: "Rotate 90° Clockwise"; enabled: root.has-document && !root.modal-visible; activated => { root.image-command(2); } }
                MenuItem { title: "Rotate 90° Counter-Clockwise"; enabled: root.has-document && !root.modal-visible; activated => { root.image-command(3); } }
                MenuItem { title: "Rotate 180°"; enabled: root.has-document && !root.modal-visible; activated => { root.image-command(4); } }
                MenuSeparator { }
                MenuItem { title: "Flatten"; enabled: root.has-document && !root.modal-visible && root.can-flatten; activated => { root.image-command(5); } }
            }
            Menu {
                title: "Help";
                MenuItem {
                    title: "About Hoshi Paint";
                    activated => { root.show-about(); }
                }
            }
        }
        FileShortcuts {
            width: parent.width; height: parent.height;
            file-action(value) => { root.file-action(value); }
            cycle-document(backward) => { root.cycle-document(backward); }
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
                Button { text: "History…"; enabled: root.has-document; clicked => { root.show-history(); } }
                Button { text: "Commit · Enter"; enabled: root.has-shape; clicked => { root.action(2); } }
                Button { text: "Cancel · Esc"; enabled: root.has-shape; clicked => { root.action(3); } }
                Rectangle { horizontal-stretch: 1; }
                Button { text: "−"; accessible-label: "Zoom out"; Tooltip { text: @markdown("Zoom out"); } clicked => { root.zoom-at(root.canvas-width / 2, root.canvas-height / 2, -1); } }
                Button { text: "Fit"; enabled: root.has-document && !root.modal-visible; accessible-label: "Fit canvas in window"; Tooltip { text: @markdown("Fit the whole image in the canvas viewport"); } clicked => { root.fit-canvas(); } }
                Button { text: "Reset view"; clicked => { root.action(4); } }
                Button { text: "+"; accessible-label: "Zoom in"; Tooltip { text: @markdown("Zoom in"); } clicked => { root.zoom-at(root.canvas-width / 2, root.canvas-height / 2, 1); } }
            }
            ScrollView {
                height: 54px; content-height: 50px; content-width: max(self.width, root.document-tabs.length * 210px);
                HorizontalLayout { spacing: 6px; alignment: start;
                    for tab in root.document-tabs: Rectangle {
                        width: 204px; background: tab.active ? #dbe8ff : #e5e9f0;
                        border-width: tab.active ? 2px : 1px; border-color: tab.active ? #315cae : #b7c2d2; border-radius: 4px;
                        HorizontalLayout { padding: 5px; spacing: 5px;
                            Button { text: tab.title + (tab.dirty ? " *" : ""); icon: tab.thumbnail;
                                accessible-label: "Activate document " + tab.title;
                                enabled: !root.modal-visible; clicked => { root.activate-document(tab.id); }
                                Tooltip { SwatchDetails { text: tab.detail; } }
                            }
                            Button { text: "×"; width: 26px; accessible-label: "Close document " + tab.title;
                                enabled: !root.modal-visible; clicked => { root.close-document(tab.id); } }
                        }
                    }
                }
            }
            workspace := Rectangle {
                vertical-stretch: 1;
                VerticalLayout {
                    x: 0px; y: 0px; width: 190px; height: parent.height;
                    spacing: 8px;
                    Text { text: "TOOLS"; color: #6c778b; font-size: 12px; font-weight: 700; }
                    rectangle-tool := Button { enabled: root.has-document; text: root.selected-tool == 0 ? "● Rectangle" : "Rectangle"; clicked => { root.choose-tool(0); } }
                    Rectangle { height: 4px; }
                    Text { text: "SHAPE STYLE"; color: #6c778b; font-size: 12px; font-weight: 700; }
                    shape-mode-choice := ComboBox { model: ["Fill", "Outline", "Fill and outline"]; current-index: root.shape-mode;
                        accessible-label: "Shape style";
                        Tooltip { text: @markdown("Fill or outline alone uses Primary. Both uses Primary outline and Secondary fill."); }
                        selected(value) => { root.select-shape-mode(self.current-index); } }
                    rendering-settings := ShapeRendering {
                        outline-width: root.stroke-width-pixels;
                        outline-enabled: root.has-document && root.shape-mode != 0;
                        fill-enabled: root.has-document && root.shape-mode != 1;
                        antialiasing: root.shape-antialiasing;
                        line-style: root.line-style; fill-pattern: root.fill-pattern;
                        edit-width(value) => { root.edit-stroke-width-pixels(value); }
                        select-line(value) => { root.select-line-style(value); }
                        select-fill(value) => { root.select-fill-pattern(value); }
                        toggle-antialiasing(value) => { root.toggle-shape-antialiasing(value); }
                    }
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
                    Rectangle { height: 4px; }
                    Text { text: "LAYER NAME"; color: #6c778b; font-size: 12px; font-weight: 700; }
                    layer-input := LineEdit { enabled: root.has-document; text: root.layer-name; placeholder-text: "Layer name"; edited(value) => { root.rename(value); self.text = root.layer-name; } }
                    Rectangle { vertical-stretch: 1; }

                }
                bounds-bar := ShapeBounds {
                    x: 204px + root.dock-left-width;
                    y: root.dock-top-height;
                    width: parent.width - 204px - root.dock-left-width - root.dock-right-width;
                    active: root.has-shape; locked <=> root.shape-bounds-locked;
                    x-value: root.shape-x-text; y-value: root.shape-y-text;
                    width-value: root.shape-width-text; height-value: root.shape-height-text;
                    edit(field,value,locked) => { root.edit-shape-bound(field,value,locked); }
                }
                viewport := Rectangle {
                    x: 204px + root.dock-left-width;
                    y: root.dock-top-height + bounds-bar.height;
                    width: parent.width - 204px - root.dock-left-width - root.dock-right-width;
                    height: parent.height - root.dock-top-height - root.dock-bottom-height - bounds-bar.height;
                    background: #dce2eb;
                    clip: true;
                    border-width: 1px;
                    border-color: #c6cfdd;
                    accessible-role: image;
                    accessible-label: "Drawing canvas. Drag to draw; Enter commits; Escape cancels.";
                    Image {
                        visible: root.has-document;
                        source: root.canvas-image;
                        x: root.viewport-preview ? 0px : root.offset-x * 1px;
                        y: root.viewport-preview ? 0px : root.offset-y * 1px;
                        width: root.viewport-preview ? parent.width : root.image-width * 1px * root.zoom;
                        height: root.viewport-preview ? parent.height : root.image-height * 1px * root.zoom;
                        image-fit: fill;
                    }
                    if root.has-shape: ShapeControls {
                        width: parent.width; height: parent.height;
                        points: root.control-points;
                    }
                    canvas-focus := FocusScope {
                        width: parent.width;
                        height: parent.height;
                        focus-lost(reason) => {
                            root.navigation-space = false;
                            if reason == FocusReason.window-activation || root.navigation-panning { root.pointer-cancel(); }
                        }
                        key-pressed(event) => {
                            root.input-modifiers(event.text == Key.Shift || event.modifiers.shift, event.text == Key.Alt || event.modifiers.alt, event.text == Key.Control || event.text == Key.Meta || event.modifiers.control || event.modifiers.meta);
                            if !event.modifiers.alt && (event.text == "[" || event.text == "]") {
                                root.adjust-outline-width((event.text == "[" ? -1 : 1) * (event.modifiers.control || event.modifiers.meta ? 5 : 1));
                                return accept;
                            }
                            if root.has-shape && (event.text == Key.LeftArrow || event.text == Key.RightArrow || event.text == Key.UpArrow || event.text == Key.DownArrow) {
                                root.keyboard-transform(event.text == Key.LeftArrow ? -1 : event.text == Key.RightArrow ? 1 : 0, event.text == Key.UpArrow ? -1 : event.text == Key.DownArrow ? 1 : 0, root.navigation-owner == PointerEventButton.right, event.modifiers.control || event.modifiers.meta);
                                return accept;
                            }
                            if event.text == Key.Space && !event.modifiers.control && !event.modifiers.meta && !event.modifiers.alt {
                                root.navigation-space = true; return accept;
                            }
                            if !event.modifiers.control && !event.modifiers.meta && !event.modifiers.alt {
                                if event.text == "x" || event.text == "X" { root.swap-colors(); return accept; }
                                if event.text == "d" || event.text == "D" { root.reset-colors(); return accept; }
                            }
                            if event.text == Key.Shift || event.text == Key.Alt || event.text == Key.Control || event.text == Key.Meta { return accept; }
                            if event.text == Key.Return { root.action(2); return accept; }
                            if event.text == Key.Escape {
                                if root.navigation-panning {
                                    root.end-pan(touch.mouse-x / 1px, touch.mouse-y / 1px);
                                    root.clear-navigation-state();
                                } else { root.action(3); }
                                return accept;
                            }
                            if event.modifiers.control || event.modifiers.meta {
                                if root.has-document && (event.text == "r" || event.text == "R") { root.show-image-settings(event.modifiers.shift ? 2 : 1); return accept; }
                                if (event.text == "f" || event.text == "F") && event.modifiers.shift { root.image-command(5); return accept; }
                                if event.text == "z" { root.action(event.modifiers.shift ? 1 : 0); return accept; }
                                if event.text == "y" { root.action(1); return accept; }
                            }
                            reject
                        }
                        key-released(event) => {
                            root.input-modifiers(event.text != Key.Shift && event.modifiers.shift, event.text != Key.Alt && event.modifiers.alt, event.text != Key.Control && event.text != Key.Meta && (event.modifiers.control || event.modifiers.meta));
                            if event.text == Key.Space {
                                root.navigation-space = false;
                                if root.navigation-space-pan && root.navigation-owner == PointerEventButton.left {
                                    root.end-pan(touch.mouse-x / 1px, touch.mouse-y / 1px);
                                    root.clear-navigation-state();
                                }
                                return accept;
                            }
                            if event.text == Key.Shift || event.text == Key.Alt || event.text == Key.Control || event.text == Key.Meta { return accept; }
                            reject
                        }
                        touch := TouchArea {
                            enabled: root.has-document && !root.modal-visible;
                            mouse-cursor: root.screen-picking ? MouseCursor.custom(@image-url("../icons/eye-dropper-cursor.svg"), 3, 29) : root.navigation-panning ? MouseCursor.grabbing : root.edit-cursor == 1 ? MouseCursor.move : root.edit-cursor == 2 ? MouseCursor.ew-resize : root.edit-cursor == 3 ? MouseCursor.ns-resize : root.edit-cursor == 4 ? MouseCursor.nesw-resize : root.edit-cursor == 5 ? MouseCursor.nwse-resize : root.edit-cursor == 6 ? MouseCursor.custom(@image-url("../icons/rotate-shape-cursor.svg"), 16, 16) : root.edit-cursor == 7 ? MouseCursor.move : MouseCursor.default;
                            pointer-event(event) => {
                                if event.kind == PointerEventKind.cancel {
                                    root.pointer-cancel();
                                } else if event.kind == PointerEventKind.down && root.navigation-owner == PointerEventButton.other {
                                    if event.button == PointerEventButton.left || event.button == PointerEventButton.middle || event.button == PointerEventButton.right {
                                        canvas-focus.focus();
                                        root.navigation-owner = event.button;
                                        root.navigation-space-pan = event.button == PointerEventButton.left && root.navigation-space;
                                        root.navigation-panning = event.button == PointerEventButton.middle || (event.button == PointerEventButton.left && root.navigation-space) || root.selected-tool == 2;
                                        if root.navigation-panning {
                                            root.begin-pan(self.mouse-x / 1px, self.mouse-y / 1px);
                                        } else {
                                            root.pointer-pressed(self.mouse-x / 1px, self.mouse-y / 1px, event.button == PointerEventButton.right, event.modifiers.shift, event.modifiers.alt, event.modifiers.control || event.modifiers.meta);
                                        }
                                    }
                                } else if event.kind == PointerEventKind.move && root.navigation-owner != PointerEventButton.other {
                                    if root.navigation-panning { root.pointer-move(self.mouse-x / 1px, self.mouse-y / 1px); }
                                    else { root.pointer-motion(self.mouse-x / 1px, self.mouse-y / 1px, event.modifiers.shift, event.modifiers.alt, event.modifiers.control || event.modifiers.meta); }
                                } else if event.kind == PointerEventKind.move {
                                    root.hover-canvas(self.mouse-x / 1px, self.mouse-y / 1px);
                                } else if event.kind == PointerEventKind.up && event.button == root.navigation-owner {
                                    if root.navigation-panning {
                                        root.end-pan(self.mouse-x / 1px, self.mouse-y / 1px);
                                    } else {
                                        root.pointer-released(self.mouse-x / 1px, self.mouse-y / 1px, event.modifiers.shift, event.modifiers.alt, event.modifiers.control || event.modifiers.meta);
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
        if root.file-prompt-visible: Rectangle {
            background: #0008; TouchArea { }
            SavePrompt { x: (parent.width - self.width)/2; y: (parent.height - self.height)/2; width: 540px; height: 240px;
                name: root.file-prompt-name; response(value) => { root.file-prompt-response(value); } }
        }
        if root.save-config-visible: Rectangle {
            background: #0008; TouchArea { }
            SaveConfiguration { x: (parent.width - self.width)/2; y: (parent.height - self.height)/2; width: 540px; height: 330px;
                jpeg: root.save-config-jpeg; warning: root.save-config-warning; quality <=> root.save-quality;
                save => { root.confirm-save-configuration(true); } cancel => { root.confirm-save-configuration(false); } }
        }
        if root.file-error != "": Rectangle {
            background: #0008; TouchArea { }
            FileErrorPrompt { x: (parent.width - self.width)/2; y: (parent.height - self.height)/2; width: 540px; height: 300px;
                message: root.file-error; dismiss => { root.dismiss-file-error(); }
            }
        }
        if root.image-dialog-kind >= 0: Rectangle {
            background: #0008;
            TouchArea { }
            ImageSettings {
                x: (parent.width - self.width) / 2; y: (parent.height - self.height) / 2;
                width: 560px; height: min(parent.height - 24px, root.image-dialog-kind == 0 ? 610px : root.image-dialog-kind == 1 ? 700px : 680px);
                kind: root.image-dialog-kind;
                width-text <=> root.image-width-text; height-text <=> root.image-height-text;
                resolution-text <=> root.image-resolution-text; percent-text <=> root.image-percent-text;
                units: root.image-units; aspect: root.image-aspect; percentage: root.image-percentage;
                anchor <=> root.image-anchor; background-choice <=> root.image-background; resampling <=> root.image-resampling; gamma-correction <=> root.image-gamma-correction;
                summary: root.image-summary; error: root.image-error;
                edited(which, value) => { root.edit-image-setting(which, value); }
                units-selected(value) => { root.select-image-units(value); }
                aspect-selected(value) => { root.select-image-aspect(value); }
                percentage-selected(value) => { root.select-image-percentage(value); }
                apply => { root.apply-image-settings(); } cancel => { root.close-image-settings(); }
            }
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
}

pub type SharedController = Rc<RefCell<workspace::Workspace>>;

pub fn refresh_workspace(ui: &AppWindow, state: &SharedController) {
    refresh(ui, &state.borrow());
}

fn render_canvas(ui: &AppWindow, controller: &Controller) {
    if !ui.get_has_document() {
        ui.set_canvas_image(slint::Image::default());
        return;
    }
    let document = controller.document();
    let large = u64::from(document.width()) * u64::from(document.height()) > 1_048_576;
    ui.set_viewport_preview(large);
    let preview = if large {
        controller.render_viewport(
            ui.get_canvas_width().ceil().max(1.) as u32,
            ui.get_canvas_height().ceil().max(1.) as u32,
            f64::from(ui.window().scale_factor()),
        )
    } else {
        controller.thumbnail(document.width(), document.height())
    };
    let mut pixels =
        slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(preview.width, preview.height);
    pixels.make_mut_bytes().copy_from_slice(&preview.rgba);
    ui.set_canvas_image(slint::Image::from_rgba8(pixels));
}
fn bind_rendering(ui: &AppWindow, state: SharedController) {
    let weak = ui.as_weak();
    let fit_state = state.clone();
    ui.on_fit_canvas(move || {
        if let Some(ui) = weak.upgrade()
            && !ui.get_modal_visible()
            && let Ok(mut state) = fit_state.try_borrow_mut()
            && state.has_document()
        {
            let result = state.fit_viewport(
                f64::from(ui.get_canvas_width()),
                f64::from(ui.get_canvas_height()),
            );
            refresh(&ui, &state);
            if let Err(error) = result {
                ui.set_status(format!("Cannot fit image: {error:?}").into());
            }
        }
    });
    let weak = ui.as_weak();
    ui.on_render_viewport(move || {
        if let Some(ui) = weak.upgrade()
            && let Ok(state) = state.try_borrow()
        {
            render_canvas(&ui, &state);
        }
    });
}

fn refresh(ui: &AppWindow, controller: &workspace::Workspace) {
    ui.set_has_document(controller.has_document());
    let document = controller.document();
    ui.set_image_width(document.width() as i32);
    ui.set_image_height(document.height() as i32);
    ui.set_can_flatten(
        document.layers().len() > 1
            || !document.layers()[0].visible()
            || document.layers()[0].opacity() != 255,
    );
    let viewport = controller.viewport();
    ui.set_zoom(viewport.zoom as f32);
    ui.set_offset_x(viewport.offset_x as f32);
    ui.set_offset_y(viewport.offset_y as f32);
    render_canvas(ui, controller);
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
    ui.set_has_shape(controller.has_document() && document.active_shape().is_some());
    ui.set_can_undo(controller.has_document() && document.undo_len() > 0);
    ui.set_can_redo(controller.has_document() && document.redo_len() > 0);
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
    shape_rendering::refresh(ui, controller);
    shape_bounds::refresh(ui, controller);
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
    let percent = viewport.zoom * 100.;
    let zoom_label = if percent < 1. {
        format!("{percent:.3}")
    } else {
        format!("{percent:.0}")
    };
    ui.set_status(
        format!(
            "{} × {} · {}% · Undo: {}{}",
            document.width(),
            document.height(),
            zoom_label,
            document.undo_len(),
            shape
        )
        .into(),
    );
    documents::refresh(ui, controller);
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
        let result = {
            let mut controller = controller.borrow_mut();
            let result = action(&mut controller);
            refresh(&ui, &controller);
            result
        };
        if let Err(error) = result {
            if matches!(
                error,
                ControllerError::Core(
                    hoshi_core::Error::HistoryIo | hoshi_core::Error::HistoryCorrupt
                )
            ) {
                ui.invoke_hide_colors();
                ui.invoke_close_history();
                ui.set_file_error(error.to_string().into());
            }
            ui.set_status(match error {
                ControllerError::Core(hoshi_core::Error::MemoryLimit) => {
                    "Image memory budget exceeded. Close other images or use a smaller size.".into()
                }
                ControllerError::Core(
                    hoshi_core::Error::HistoryIo | hoshi_core::Error::HistoryCorrupt,
                ) => error.to_string().into(),
                _ => format!("Action unavailable: {error}").into(),
            });
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

pub fn create_app_with_dialogs(
    dialogs: Rc<dyn documents::FileDialogs>,
) -> Result<(AppWindow, SharedController), slint::PlatformError> {
    let (ui, state, _, _) = create_panels(Some(dialogs))?;
    Ok((ui, state))
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
    create_panels(None)
}
fn create_panels(
    dialogs: Option<Rc<dyn documents::FileDialogs>>,
) -> Result<
    (
        AppWindow,
        SharedController,
        Rc<ColorPickerWindow>,
        Rc<HistoryWindow>,
    ),
    slint::PlatformError,
> {
    resources::initialize();
    let ui = AppWindow::new()?;
    let recent_store = if dialogs.is_some() {
        None
    } else {
        documents::recent_store_path()
    };
    let controller = Rc::new(RefCell::new(workspace::Workspace::with_recent_store(
        recent_store,
    )));
    image_settings::bind(&ui, controller.clone());
    bind_rendering(&ui, controller.clone());
    refresh(&ui, &controller.borrow());
    input::bind(&ui, controller.clone());
    shape_rendering::bind(&ui, controller.clone());
    shape_bounds::bind(&ui, controller.clone());
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
    documents::bind(&ui, controller.clone(), dialogs);
    let main = ui.as_weak();
    picker.on_file_action(move |action| {
        if let Some(ui) = main.upgrade() {
            ui.invoke_file_action(action);
        }
    });
    let main = ui.as_weak();
    picker.on_cycle_document(move |backward| {
        if let Some(ui) = main.upgrade() {
            ui.invoke_cycle_document(backward);
        }
    });
    let main = ui.as_weak();
    history.on_file_action(move |action| {
        if let Some(ui) = main.upgrade() {
            ui.invoke_file_action(action);
        }
    });
    let main = ui.as_weak();
    history.on_cycle_document(move |backward| {
        if let Some(ui) = main.upgrade() {
            ui.invoke_cycle_document(backward);
        }
    });

    Ok((ui, controller, picker, history))
}
