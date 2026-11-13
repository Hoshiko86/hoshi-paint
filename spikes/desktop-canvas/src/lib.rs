//! Disposable Slint adapter around the UI-independent controller.

use hoshi_controller::{ColorSlot, Controller, ControllerError, ShapeMode, Tool};
use slint::ComponentHandle;
use std::{cell::RefCell, rc::Rc};
mod color_library;
mod colors;
pub mod documents;
pub mod files;
mod history;
mod image_settings;
mod input;
mod localization;
pub mod palette_store;
mod panel_hosts;
mod panel_layout;
mod resources;
mod screen_pick;
mod shape_bounds;
mod shape_properties;
mod shape_rendering;
pub mod workspace;

slint::slint! {
    import { ShapePropertiesPanel } from "../ui/shape-properties.slint";
    export { ShapePropertiesWindow } from "../ui/shape-properties.slint";
    import { ShapeControls, ControlPoint } from "../ui/shape-controls.slint";
    export { ControlPoint } from "../ui/shape-controls.slint";
    import { FileShortcuts } from "../ui/file-shortcuts.slint";
    import { SavePrompt, SaveConfiguration, FileErrorPrompt } from "../ui/file-prompts.slint";
    import { ImageSettings } from "../ui/image-settings.slint";
    import { ColorPanel, NamedColorSuggestion, PaletteSwatch, WorkspaceColors, ColorSwatch } from "../ui/colors.slint";
    import { HistoryPanel, HistoryRow } from "../ui/history.slint";
    import { DesktopCommandBars, DocumentTabs, DesktopStatusBar, DocumentTabRow } from "../ui/app-chrome.slint";
    import { DesktopTheme } from "../ui/theme.slint";
    import { Translations } from "../ui/localization.slint";
    export { HistoryWindow, HistoryRow } from "../ui/history.slint";
    export { ColorPickerWindow, PaletteSwatch, NamedColorSuggestion } from "../ui/colors.slint";
    import { Button, AboutSlint, Palette, ComboBox, SpinBox } from "std-widgets.slint";
    export { DocumentTabRow } from "../ui/app-chrome.slint";
    export { Translations } from "../ui/localization.slint";
    export component AppWindow inherits Window {
        in property <string> document-title: "Untitled · Hoshi Paint";
        title: root.document-title;
        preferred-width: 1120px;
        preferred-height: 760px;
        min-width: max(960px, root.layout-min-width * 1px + 32px);
        min-height: 680px;
        background: DesktopTheme.window-background;
        default-font-family: "DejaVu Sans";
        default-font-size: 13px;
        init => { Palette.color-scheme = ColorScheme.dark; }
        in property <image> canvas-image;
        in property <bool> viewport-preview;
        callback render-viewport();
        callback initialize-canvas-view();
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
        out property <float> outline-input-left: docked-shapes.outline-input-left;
        out property <float> outline-input-top: docked-shapes.outline-input-top;
        in property <color> workspace-primary;
        in property <color> workspace-secondary;
        in property <string> workspace-primary-description;
        in property <string> workspace-secondary-description;
        callback swap-colors();
        callback reset-colors();
        in property <int> color-slot;
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
        out property <color> pasteboard-color: DesktopTheme.pasteboard-background;
        out property <float> layer-input-left: status-bar.input-left;
        out property <float> layer-input-top: status-bar.input-top;
        out property <string> layer-edit-text: status-bar.edit-text;
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
        in-out property <string> color-name-query;
        in property <[NamedColorSuggestion]> color-name-suggestions;
        in property <[string]> palette-names;
        in property <int> palette-selected;
        in property <string> palette-title;
        in property <bool> palette-inserting;
        in property <bool> palette-expanded;
        in property <bool> palette-dirty;
        in property <string> palette-error;
        callback search-color-name(string);
        callback choose-named-color(string);
        callback select-palette-color(int, bool);
        callback toggle-palette-insert();
        callback select-palette(int);
        callback save-palette-as();
        callback import-palette();
        callback export-palette();
        callback open-palettes-folder();
        callback reset-palette();
        callback set-palette-expanded(bool);
        callback import-palette-path(string);
        callback save-palette-path(string);
        callback export-palette-path(string);

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
        callback refresh-palette-library();
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
        in property <bool> shape-properties-visible;
        in property <int> shape-properties-placement;
        callback show-shape-properties();
        callback close-shape-properties();
        callback set-shape-properties-placement(int);
        callback sync-shape-properties();
        callback finish-shape-properties-inputs();
        public function finish-shape-properties-edits() { docked-shapes.finish-edits(); }
        callback layout-panels();
        out property <float> workspace-width: workspace.width / 1px;
        out property <float> workspace-height: workspace.height / 1px;
        changed workspace-width => { root.layout-panels(); }
        changed workspace-height => { root.layout-panels(); }
        changed colors-visible => { root.layout-panels(); }
        changed colors-placement => { root.layout-panels(); }
        changed colors-docked => { root.layout-panels(); }
        changed history-visible => { root.layout-panels(); }
        changed history-placement => { root.layout-panels(); }
        changed shape-properties-visible => { root.layout-panels(); }
        changed shape-properties-placement => { root.layout-panels(); }
        in property <float> layout-min-width: 318;
        in property <float> canvas-layout-x;
        in property <float> canvas-layout-y;
        in property <float> canvas-layout-width;
        in property <float> canvas-layout-height;
        in property <float> colors-layout-x;
        in property <float> colors-layout-y;
        in property <float> colors-layout-width;
        in property <float> colors-layout-height;
        in property <float> history-layout-x;
        in property <float> history-layout-y;
        in property <float> history-layout-width;
        in property <float> history-layout-height;
        in property <float> shapes-layout-x;
        in property <float> shapes-layout-y;
        in property <float> shapes-layout-width;
        in property <float> shapes-layout-height;
        private property <bool> shapes-host-visible: root.shape-properties-visible && root.shape-properties-placement > 0;
        out property <float> shapes-host-left: docked-shapes.absolute-position.x / 1px;
        out property <float> shapes-host-top: docked-shapes.absolute-position.y / 1px;
        out property <float> shapes-host-width: docked-shapes.width / 1px;
        out property <float> shapes-host-height: docked-shapes.height / 1px;
        out property <float> canvas-width: viewport.width / 1px;
        out property <float> canvas-height: viewport.height / 1px;
        private property <bool> canvas-view-initialized;
        changed canvas-width => {
            root.render-viewport();
            if !root.canvas-view-initialized && root.canvas-width > 0 && root.canvas-height > 0 {
                root.canvas-view-initialized = true;
                root.initialize-canvas-view();
            }
        }
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
        show-about => { if !root.modal-visible { root.close-shape-properties(); root.hide-colors(); root.close-history(); root.pointer-cancel(); root.about-open = true; about-focus.focus(); } }
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
            padding: 6px;
            spacing: 4px;
            DesktopCommandBars {
                has-document: root.has-document; has-shape: root.has-shape;
                can-undo: root.can-undo; can-redo: root.can-redo;
                modal-visible: root.modal-visible; shape-mode: root.shape-mode;
                stroke-width-pixels: root.stroke-width-pixels;
                action(value) => { root.action(value); }
                select-shapes => { root.choose-tool(0); root.show-shape-properties(); }
                show-history => { root.show-history(); }
                show-shape-properties => { root.show-shape-properties(); }
                zoom(delta) => { root.zoom-at(root.canvas-width / 2, root.canvas-height / 2, delta); }
                fit-canvas => { root.fit-canvas(); }
            }
            DocumentTabs {
                tabs: root.document-tabs; modal-visible: root.modal-visible;
                activate(id) => { root.activate-document(id); }
                close(id) => { root.close-document(id); }
            }
            workspace := Rectangle {
                vertical-stretch: 1;
                background: DesktopTheme.workspace-background;
                VerticalLayout {
                    x: 0px; y: 0px; width: DesktopTheme.tool-rail-width; height: parent.height;
                    spacing: 8px; alignment: start;
                    Text { text: "TOOLS"; color: DesktopTheme.muted-text; font-size: 11px; font-weight: 700; }
                    Rectangle { height: 44px; border-width: root.selected-tool == 0 ? 2px : 0px; border-color: DesktopTheme.selected-tool; border-radius: 4px; background: transparent;
                        rectangle-tool := Button { x: 2px; y: 2px; width: parent.width - 4px; height: parent.height - 4px;
                            icon: @image-url("../icons/shapes.svg"); accessible-label: "Shapes"; enabled: root.has-document; opacity: self.enabled ? 1 : 0.4;
                            Tooltip { text: @markdown("Shapes · Rectangle"); }
                            clicked => { root.choose-tool(0); root.show-shape-properties(); }
                        }
                    }
                    Button { icon: @image-url("../icons/palette.svg"); height: 40px; accessible-label: "Colors";
                        Tooltip { text: @markdown("Colors · Primary, Secondary and Color Picker"); } clicked => { root.show-colors(); } }
                    Text { text: "COLORS"; color: DesktopTheme.muted-text; font-size: 11px; font-weight: 700; }
                    ColorSwatch { value: root.workspace-primary; label: "Primary color"; description: root.workspace-primary-description; selected: root.color-slot == 0; clicked => { root.select-color-slot(0); } }
                    ColorSwatch { value: root.workspace-secondary; label: "Secondary color"; description: root.workspace-secondary-description; selected: root.color-slot == 1; clicked => { root.select-color-slot(1); } }
                    HorizontalLayout { spacing: 4px;
                        Button { icon: @image-url("../icons/swap-colors.svg"); width: 28px; accessible-label: "Swap colors"; Tooltip { text: @markdown("Swap colors · X"); } clicked => { root.swap-colors(); } }
                        Button { icon: @image-url("../icons/reset-colors.svg"); width: 28px; accessible-label: "Reset colors"; Tooltip { text: @markdown("Reset colors · D"); } clicked => { root.reset-colors(); } }
                    }
                    Rectangle { vertical-stretch: 1; }
                }
                viewport := Rectangle {
                    x: root.canvas-layout-x * 1px; y: root.canvas-layout-y * 1px;
                    width: root.canvas-layout-width * 1px; height: root.canvas-layout-height * 1px;
                    background: DesktopTheme.pasteboard-background;
                    clip: true;
                    border-width: 1px;
                    border-color: DesktopTheme.strong-border;
                    accessible-role: image;
                    accessible-label: "Drawing canvas. Drag to draw; Enter or Escape commits.";
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
                                } else { root.action(2); }
                                return accept;
                            }
                            if event.modifiers.control || event.modifiers.meta {
                                if root.has-document && (event.text == "r" || event.text == "R") { root.show-image-settings(event.modifiers.shift ? 2 : 1); return accept; }
                                if (event.text == "f" || event.text == "F") && event.modifiers.shift { root.image-command(5); return accept; }
                                if (event.text == "z" || event.text == "Z") && !event.modifiers.shift { root.action(0); return accept; }
                                if event.text == "y" || event.text == "Y" { root.action(1); return accept; }
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
                    x: root.colors-layout-x * 1px; y: root.colors-layout-y * 1px;
                    width: root.colors-layout-width * 1px; height: root.colors-layout-height * 1px;
                    docked: true;
                    placement: root.colors-placement;
                    placement-selected(value) => { root.set-colors-placement(value); }
                    swatches: root.color-swatches;
                    color-name-query <=> root.color-name-query;
                    color-name-suggestions: root.color-name-suggestions;
                    palette-names: root.palette-names; palette-selected: root.palette-selected;
                    palette-title: root.palette-title; palette-inserting: root.palette-inserting;
                    palette-expanded: root.palette-expanded; palette-dirty: root.palette-dirty;
                    palette-error: root.palette-error;
                    search-color-name(value) => { root.search-color-name(value); }
                    choose-named-color(value) => { root.choose-named-color(value); }
                    select-palette-color(index,inactive) => { root.select-palette-color(index,inactive); }
                    toggle-palette-insert => { root.toggle-palette-insert(); }
                    select-palette(index) => { root.select-palette(index); }
                    save-palette-as => { root.save-palette-as(); }
                    import-palette => { root.import-palette(); }
                    export-palette => { root.export-palette(); }
                    open-palettes-folder => { root.open-palettes-folder(); }
                    reset-palette => { root.reset-palette(); }
                    set-palette-expanded(value) => { root.set-palette-expanded(value); }

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
                    x: root.history-layout-x * 1px; y: root.history-layout-y * 1px;
                    width: root.history-layout-width * 1px; height: root.history-layout-height * 1px;
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
                docked-shapes := ShapePropertiesPanel {
                    visible: root.shapes-host-visible;
                    x: root.shapes-layout-x * 1px; y: root.shapes-layout-y * 1px;
                    width: root.shapes-layout-width * 1px; height: root.shapes-layout-height * 1px;
                    has-document: root.has-document; has-shape: root.has-shape;
                    shape-mode: root.shape-mode; stroke-width-pixels: root.stroke-width-pixels;
                    shape-antialiasing: root.shape-antialiasing; line-style: root.line-style; fill-pattern: root.fill-pattern;
                    shape-x-text: root.shape-x-text; shape-y-text: root.shape-y-text;
                    shape-width-text: root.shape-width-text; shape-height-text: root.shape-height-text;
                    shape-bounds-locked <=> root.shape-bounds-locked; placement: root.shape-properties-placement;
                    select-shape-mode(value) => { root.select-shape-mode(value); }
                    edit-stroke-width-pixels(value) => { root.edit-stroke-width-pixels(value); }
                    select-line-style(value) => { root.select-line-style(value); }
                    select-fill-pattern(value) => { root.select-fill-pattern(value); }
                    toggle-shape-antialiasing(value) => { root.toggle-shape-antialiasing(value); }
                    edit-shape-bound(field,value,locked) => { root.edit-shape-bound(field,value,locked); }
                    bounds-lock-changed(value) => { if root.shape-bounds-locked != value { root.shape-bounds-locked = value; } }
                    placement-selected(value) => { root.set-shape-properties-placement(value); }
                    close-panel => { root.close-shape-properties(); }
                }
            }
            status-bar := DesktopStatusBar {
                status: root.status; has-document: root.has-document;
                image-width: root.image-width; image-height: root.image-height;
                zoom: root.zoom; layer-name: root.layer-name;
                rename(value) => { root.rename(value); }
            }
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
                    background: #303338;
                    border-radius: 8px;
                    border-width: 1px;
                    border-color: #69778c;
                    VerticalLayout {
                        padding: 24px;
                        spacing: 16px;
                        Text { text: "Hoshi Paint"; color: #eef2f7; font-size: 24px; font-weight: 700; }
                        Text { text: "Copyright 2026 Hoshi Paint contributors\nGNU GPL version 3 only · GPL-3.0-only"; wrap: word-wrap; color: #b8c2d1; }
                        Text { text: "Icons: Lucide 1.71.0 · lucide.dev\nISC / MIT · Lucide and Feather contributors\nCursor adapted with contrast outline. See THIRD_PARTY_NOTICES.md."; wrap: word-wrap; color: #b8c2d1; font-size: 11px; }
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
    let center_state = state.clone();
    ui.on_initialize_canvas_view(move || {
        let weak = weak.clone();
        let center_state = center_state.clone();
        slint::Timer::single_shot(std::time::Duration::ZERO, move || {
            if let Some(ui) = weak.upgrade()
                && ui.window().is_visible()
                && let Ok(mut state) = center_state.try_borrow_mut()
                && state.has_document()
            {
                let _ = state.center_viewport(
                    f64::from(ui.get_canvas_width()),
                    f64::from(ui.get_canvas_height()),
                );
                refresh(&ui, &state);
            }
        });
    });
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
        Tool::Brush => 4,
        Tool::Eraser => 5,
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
    ui.invoke_sync_shape_properties();
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
            // Core errors carry no user copy; the locale catalogs supply it.
            let text = ui.global::<Translations>();
            let history = match error {
                ControllerError::Core(hoshi_core::Error::HistoryIo) => {
                    Some(text.get_error_history_io())
                }
                ControllerError::Core(hoshi_core::Error::HistoryCorrupt) => {
                    Some(text.get_error_history_corrupt())
                }
                _ => None,
            };
            if let Some(message) = &history {
                panel_hosts::close_adjustment_panels(&ui);
                ui.set_file_error(message.clone());
            }
            ui.set_status(match (error, history) {
                (_, Some(message)) => message,
                (ControllerError::Core(hoshi_core::Error::MemoryLimit), None) => {
                    text.get_error_memory_limit()
                }
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

/// Handles to the main window and its reusable adjustment-panel hosts.
pub struct AppPanels {
    pub ui: AppWindow,
    pub controller: SharedController,
    pub colors: Rc<ColorPickerWindow>,
    pub history: Rc<HistoryWindow>,
    pub shapes: Rc<ShapePropertiesWindow>,
}
pub fn create_app_with_dialogs(
    dialogs: Rc<dyn documents::FileDialogs>,
) -> Result<(AppWindow, SharedController), slint::PlatformError> {
    let app = create_panels(Some(dialogs), None)?;
    Ok((app.ui, app.controller))
}
pub fn create_app_with_shape_properties()
-> Result<(AppWindow, SharedController, Rc<ShapePropertiesWindow>), slint::PlatformError> {
    let app = create_panels(None, default_palette_root())?;
    Ok((app.ui, app.controller, app.shapes))
}
pub fn create_app_with_all_panels() -> Result<AppPanels, slint::PlatformError> {
    create_panels(None, default_palette_root())
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
    let app = create_panels(None, default_palette_root())?;
    Ok((app.ui, app.controller, app.colors, app.history))
}
pub fn create_app_with_palette_root(
    root: Option<std::path::PathBuf>,
) -> Result<AppPanels, slint::PlatformError> {
    create_panels(None, root)
}
fn default_palette_root() -> Option<std::path::PathBuf> {
    documents::recent_store_path().and_then(|path| path.parent().map(std::path::Path::to_path_buf))
}
fn create_panels(
    dialogs: Option<Rc<dyn documents::FileDialogs>>,
    palette_root: Option<std::path::PathBuf>,
) -> Result<AppPanels, slint::PlatformError> {
    resources::initialize();
    let ui = AppWindow::new()?;
    let locale = localization::system_selection();
    let text = localization::Localizer::new(&locale);
    localization::apply(&ui, &text);
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
    panel_hosts::bind(&ui);
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
        let canvas_size = weak.upgrade().map(|ui| {
            (
                f64::from(ui.get_canvas_width()),
                f64::from(ui.get_canvas_height()),
            )
        });
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
                canvas_size.map_or(Ok(()), |(width, height)| c.center_viewport(width, height))
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
    let picker = colors::bind_picker(&ui, controller.clone(), palette_root)?;
    localization::apply_colors(&picker, &text);
    let history = history::bind_window(&ui, controller.clone(), &text)?;
    let shapes = shape_properties::create(&ui, controller.clone())?;
    documents::bind(&ui, controller.clone(), dialogs, text);
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

    Ok(AppPanels {
        ui,
        controller,
        colors: picker,
        history,
        shapes,
    })
}
