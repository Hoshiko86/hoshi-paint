//! Disposable Slint adapter around the UI-independent controller.

use hoshi_controller::{Controller, ControllerError, Tool};
use slint::ComponentHandle;
use std::{cell::RefCell, rc::Rc};

slint::slint! {
    import { Button, LineEdit, AboutSlint } from "std-widgets.slint";
    export component AppWindow inherits Window {
        title: "Hoshi Paint · Canvas study";
        preferred-width: 1120px;
        preferred-height: 760px;
        min-width: 960px;
        min-height: 680px;
        background: #f2f4f8;
        default-font-family: "DejaVu Sans";
        default-font-size: 14px;
        in property <image> canvas-image;
        in property <float> zoom: 1;
        in property <float> offset-x;
        in property <float> offset-y;
        in property <string> status;
        in property <string> layer-name;
        in property <bool> has-shape;
        in property <bool> can-undo;
        in property <bool> can-redo;
        in property <int> selected-tool;
        private property <bool> about-open;
        out property <bool> modal-visible: root.about-open;
        out property <float> canvas-left: viewport.absolute-position.x / 1px;
        out property <float> canvas-top: viewport.absolute-position.y / 1px;
        out property <float> layer-input-left: layer-input.absolute-position.x / 1px;
        out property <float> layer-input-top: layer-input.absolute-position.y / 1px;
        out property <string> layer-edit-text: layer-input.text;
        callback pointer-down(float, float);
        callback pointer-move(float, float);
        callback pointer-up(float, float);
        callback pointer-cancel();
        callback action(int);
        callback choose-tool(int);
        callback choose-color(int);
        callback rename(string);
        callback zoom-at(float, float, float);
        callback show-about();
        show-about => { root.pointer-cancel(); root.about-open = true; about-focus.focus(); }
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
                Button { text: "Undo"; enabled: root.can-undo || root.has-shape; clicked => { root.action(0); } }
                Button { text: "Redo"; enabled: root.can-redo && !root.has-shape; clicked => { root.action(1); } }
                Button { text: "Commit · Enter"; enabled: root.has-shape; clicked => { root.action(2); } }
                Button { text: "Cancel · Esc"; enabled: root.has-shape; clicked => { root.action(3); } }
                Rectangle { horizontal-stretch: 1; }
                Button { text: "−"; clicked => { root.zoom-at(256, 256, -1); } }
                Button { text: "Reset view"; clicked => { root.action(4); } }
                Button { text: "+"; clicked => { root.zoom-at(256, 256, 1); } }
            }
            HorizontalLayout {
                spacing: 14px;
                VerticalLayout {
                    width: 190px;
                    spacing: 8px;
                    Text { text: "TOOLS"; color: #6c778b; font-size: 12px; font-weight: 700; }
                    Button { text: root.selected-tool == 0 ? "● Rectangle" : "Rectangle"; clicked => { root.choose-tool(0); } }
                    Button { text: root.selected-tool == 1 ? "● Move shape" : "Move shape"; enabled: root.has-shape; clicked => { root.choose-tool(1); } }
                    Button { text: root.selected-tool == 2 ? "● Pan canvas" : "Pan canvas"; clicked => { root.choose-tool(2); } }
                    Rectangle { height: 12px; }
                    Text { text: "PRIMARY COLOR"; color: #6c778b; font-size: 12px; font-weight: 700; }
                    HorizontalLayout {
                        Button { text: "Blue"; clicked => { root.choose-color(0); } }
                        Button { text: "Coral"; clicked => { root.choose-color(1); } }
                        Button { text: "Mint"; clicked => { root.choose-color(2); } }
                    }
                    Rectangle { height: 12px; }
                    Text { text: "LAYER NAME"; color: #6c778b; font-size: 12px; font-weight: 700; }
                    layer-input := LineEdit { text: root.layer-name; placeholder-text: "Layer name"; edited(value) => { root.rename(value); self.text = root.layer-name; } }
                    Text { text: "ADJUST ACTIVE SHAPE"; color: #6c778b; font-size: 12px; font-weight: 700; }
                    HorizontalLayout {
                        Button { text: "←"; enabled: root.has-shape; clicked => { root.action(5); } }
                        Button { text: "→"; enabled: root.has-shape; clicked => { root.action(6); } }
                        Button { text: "↑"; enabled: root.has-shape; clicked => { root.action(7); } }
                        Button { text: "↓"; enabled: root.has-shape; clicked => { root.action(8); } }
                    }
                    HorizontalLayout {
                        Button { text: "Smaller"; enabled: root.has-shape; clicked => { root.action(9); } }
                        Button { text: "Larger"; enabled: root.has-shape; clicked => { root.action(10); } }
                    }
                    Rectangle { vertical-stretch: 1; }
                    Text { text: "Drag → adjust → Enter.\nEscape cancels. Scroll zooms."; wrap: word-wrap; color: #6c778b; font-size: 12px; }
                }
                viewport := Rectangle {
                    horizontal-stretch: 1;
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
                    canvas-focus := FocusScope {
                        width: parent.width;
                        height: parent.height;
                        focus-lost(reason) => {
                            if reason == FocusReason.window-activation { root.pointer-cancel(); }
                        }
                        key-pressed(event) => {
                            if event.text == Key.Return { root.action(2); return accept; }
                            if event.text == Key.Escape { root.action(3); return accept; }
                            if event.modifiers.control || event.modifiers.meta {
                                if event.text == "z" { root.action(event.modifiers.shift ? 1 : 0); return accept; }
                                if event.text == "y" { root.action(1); return accept; }
                            }
                            reject
                        }
                        touch := TouchArea {
                            enabled: !root.about-open;
                            pointer-event(event) => {
                                if event.button == PointerEventButton.left && event.kind == PointerEventKind.down {
                                    canvas-focus.focus();
                                    root.pointer-down(self.mouse-x / 1px, self.mouse-y / 1px);
                                } else if event.button == PointerEventButton.left && event.kind == PointerEventKind.up {
                                    root.pointer-up(self.mouse-x / 1px, self.mouse-y / 1px);
                                } else if event.kind == PointerEventKind.cancel { root.pointer-cancel(); }
                            }
                            moved => { if self.pressed { root.pointer-move(self.mouse-x / 1px, self.mouse-y / 1px); } }
                            scroll-event(event) => {
                                root.zoom-at(self.mouse-x / 1px, self.mouse-y / 1px, event.delta-y / 1px / 120);
                                accept
                            }
                        }
                    }
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
                    width: 380px;
                    height: 240px;
                    background: white;
                    border-radius: 12px;
                    VerticalLayout {
                        padding: 24px;
                        spacing: 16px;
                        Text { text: "Hoshi Paint"; color: #18253c; font-size: 24px; font-weight: 700; }
                        Text { text: "A study in drawing, shapes, and simple workflows."; wrap: word-wrap; color: #526079; }
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
    ui.set_has_shape(document.active_shape().is_some());
    ui.set_can_undo(document.undo_len() > 0);
    ui.set_can_redo(document.redo_len() > 0);
    ui.set_selected_tool(match controller.tool() {
        Tool::Rectangle => 0,
        Tool::MoveShape => 1,
        Tool::Pan => 2,
    });
    ui.set_layer_name(document.layers()[document.selected_layer()].name().into());
    let shape = document
        .active_shape()
        .map(|s| {
            format!(
                " · Active shape: {} × {} at {}, {}",
                s.width, s.height, s.x, s.y
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
    let ui = AppWindow::new()?;
    let controller = Rc::new(RefCell::new(Controller::default()));
    refresh(&ui, &controller.borrow());
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_pointer_down(move |x, y| update(&weak, &state, |c| c.pointer_down(x.into(), y.into())));
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_pointer_move(move |x, y| update(&weak, &state, |c| c.pointer_move(x.into(), y.into())));
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_pointer_up(move |x, y| {
        update(&weak, &state, |c| {
            c.pointer_move(x.into(), y.into())?;
            c.pointer_up()
        })
    });
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_pointer_cancel(move || {
        update(&weak, &state, |c| {
            if c.is_dragging() {
                c.cancel_pointer();
            }
            Ok(())
        })
    });
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_choose_tool(move |tool| {
        update(&weak, &state, |c| {
            c.set_tool(match tool {
                1 => Tool::MoveShape,
                2 => Tool::Pan,
                _ => Tool::Rectangle,
            })
        })
    });
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_choose_color(move |color| {
        update(&weak, &state, |c| {
            c.set_primary_color(match color {
                1 => hoshi_controller::Color::rgba(232, 104, 92, 255),
                2 => hoshi_controller::Color::rgba(54, 167, 140, 255),
                _ => hoshi_controller::Color::rgba(72, 136, 240, 255),
            })
        })
    });
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_rename(move |name| update(&weak, &state, |c| c.rename_layer(&name)));
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
    Ok((ui, controller))
}
