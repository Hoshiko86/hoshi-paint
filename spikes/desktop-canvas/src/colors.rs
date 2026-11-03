use crate::{AppWindow, SharedController, refresh, update};
use hoshi_controller::{
    Color, Tool,
    color::{self, Hsv},
};
use slint::ComponentHandle;
use std::rc::Rc;

slint::slint! {
    import { Button, Slider, SpinBox, LineEdit, Palette } from "std-widgets.slint";
    component Channel inherits Rectangle {
        preferred-height: 34px;
        in property <string> label;
        in property <float> value;
        in property <int> maximum: 255;
        out property <float> slider-left: slider.absolute-position.x / 1px;
        out property <float> slider-top: slider.absolute-position.y / 1px;
        out property <float> slider-width: slider.width / 1px;
        out property <float> slider-height: slider.height / 1px;
        callback edited(float);
        changed value => { slider.value = root.value; spin.value = round(root.value); }
        HorizontalLayout {
            spacing: 4px;
            Text { text: root.label; width: 40px; color: #273650; vertical-alignment: center; }
            slider := Slider { minimum: 0; maximum: root.maximum; value: root.value; accessible-label: root.label;
                changed(v) => { root.edited(v); }
            }
            spin := SpinBox { minimum: 0; maximum: root.maximum; value: round(root.value); width: 128px; accessible-label: root.label;
                edited(v) => { root.edited(v); }
            }
        }
    }
    export component ColorPickerWindow inherits Window {
        title: "Colors · Hoshi Paint";
        preferred-width: 640px;
        preferred-height: 680px;
        min-width: 640px;
        min-height: 660px;
        background: #f2f4f8;
        default-font-size: 13px;
        init => { Palette.color-scheme = ColorScheme.light; }
        in property <int> red;
        in property <int> green;
        in property <int> blue;
        in property <int> alpha: 255;
        in property <float> hue;
        in property <float> saturation;
        in property <float> brightness;
        in-out property <string> hex-value;
        in property <string> error;
        in property <image> sv-image;
        in property <color> current-color;
        in property <bool> has-active-shape;
        out property <float> red-slider-left: red-channel.slider-left;
        out property <float> red-slider-top: red-channel.slider-top;
        out property <float> red-slider-width: red-channel.slider-width;
        out property <float> red-slider-height: red-channel.slider-height;
        out property <float> sv-left: sv.absolute-position.x / 1px;
        out property <float> sv-top: sv.absolute-position.y / 1px;
        out property <float> sv-width: sv.width / 1px;
        out property <float> sv-height: sv.height / 1px;
        callback set-rgba(int, int, int, int);
        callback set-hsv(float, float, float);
        callback set-hex(string);
        callback pick();
        callback close-picker();
        VerticalLayout {
            padding: 16px;
            spacing: 8px;
            HorizontalLayout {
                height: 32px;
                Text { text: "Colors"; font-size: 22px; font-weight: 700; color: #18253c; }
                Rectangle { horizontal-stretch: 1; }
                Rectangle { width: 80px; background: #d0d4dc;
                    Rectangle { background: root.current-color; }
                }
            }
            Text { text: "SATURATION / VALUE"; font-size: 11px; color: #526079; }
            sv := Rectangle {
                height: 180px;
                clip: true;
                Image { source: root.sv-image; width: parent.width; height: parent.height; image-fit: fill; }
                Rectangle {
                    x: parent.width * root.saturation / 100 - 6px;
                    y: parent.height * (1 - root.brightness / 100) - 6px;
                    width: 12px; height: 12px; border-radius: 6px;
                    border-width: 2px; border-color: white; background: transparent;
                    Rectangle { x: 2px; y: 2px; width: 8px; height: 8px; border-radius: 4px; border-width: 1px; border-color: #18253c; background: transparent; }
                }
                area := TouchArea {
                    pointer-event(event) => { if event.kind == PointerEventKind.down {
                        root.set-hsv(root.hue, self.mouse-x / self.width * 100, (1 - self.mouse-y / self.height) * 100);
                    } }
                    moved => { if self.pressed { root.set-hsv(root.hue, self.mouse-x / self.width * 100, (1 - self.mouse-y / self.height) * 100); } }
                }
                accessible-role: image;
                accessible-label: "Saturation and value. Numeric controls below provide keyboard access.";
            }
            Channel { label: "Hue"; maximum: 360; value: root.hue; edited(v) => { root.set-hsv(v, root.saturation, root.brightness); } }
            HorizontalLayout {
                spacing: 16px;
                VerticalLayout {
                    Text { text: "RGB · 0–255"; color: #526079; }
                    red-channel := Channel { label: "R"; value: root.red; edited(v) => { root.set-rgba(round(v), root.green, root.blue, root.alpha); } }
                    Channel { label: "G"; value: root.green; edited(v) => { root.set-rgba(root.red, round(v), root.blue, root.alpha); } }
                    Channel { label: "B"; value: root.blue; edited(v) => { root.set-rgba(root.red, root.green, round(v), root.alpha); } }
                }
                VerticalLayout {
                    Text { text: "HSV · ° / % / %"; color: #526079; }
                    Channel { label: "H"; maximum: 360; value: root.hue; edited(v) => { root.set-hsv(v, root.saturation, root.brightness); } }
                    Channel { label: "S"; maximum: 100; value: root.saturation; edited(v) => { root.set-hsv(root.hue, v, root.brightness); } }
                    Channel { label: "V"; maximum: 100; value: root.brightness; edited(v) => { root.set-hsv(root.hue, root.saturation, v); } }
                }
            }
            Channel { label: "Alpha"; value: root.alpha; edited(v) => { root.set-rgba(root.red, root.green, root.blue, round(v)); } }
            HorizontalLayout {
                Text { text: "Hex"; width: 32px; vertical-alignment: center; color: #273650; }
                hex-entry := LineEdit { text <=> root.hex-value; accessible-label: "Hex RGB or RGBA color"; accepted(text) => { root.set-hex(text); } }
                Button { text: "Apply hex"; clicked => { root.set-hex(hex-entry.text); } }
            }
            Text { text: root.error != "" ? root.error : root.has-active-shape ? "Editing the active shape · Color changes apply immediately." : "Next drawing color · Committed pixels will not change.";
                color: root.error == "" ? #526079 : #a12828; height: 18px; }
            HorizontalLayout {
                Button { text: "Blue"; clicked => { root.set-rgba(72, 136, 240, root.alpha); } }
                Button { text: "Coral"; clicked => { root.set-rgba(232, 104, 92, root.alpha); } }
                Button { text: "Mint"; clicked => { root.set-rgba(54, 167, 140, root.alpha); } }
            }
            HorizontalLayout {
                Button { text: "Eyedropper on canvas"; clicked => { root.pick(); } }
                Button { text: "Close"; clicked => { root.close-picker(); } }
            }
        }
    }
}

fn synchronize(picker: &ColorPickerWindow, value: Color, override_hsv: Option<Hsv>) {
    let mut hsv = override_hsv.unwrap_or_else(|| color::color_to_hsv(value));
    if override_hsv.is_none() {
        if hsv.saturation == 0.0 {
            hsv.hue = picker.get_hue().into();
        }
        if hsv.value == 0.0 {
            hsv.saturation = picker.get_saturation().into();
        }
    }
    if (f64::from(picker.get_hue()) - hsv.hue).abs() > 0.0001
        || picker.get_sv_image().size().width == 0
    {
        let base = color::color_from_hsv(hsv.hue, 100.0, 100.0, 255).expect("validated hue");
        let mut pixels = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(256, 180);
        for (index, pixel) in pixels.make_mut_slice().iter_mut().enumerate() {
            let s = (index % 256) as f64 / 255.0;
            let v = 1.0 - (index / 256) as f64 / 179.0;
            let channel = |c: u8| (((1.0 - s) * 255.0 + s * f64::from(c)) * v).round() as u8;
            *pixel = slint::Rgba8Pixel {
                r: channel(base.r),
                g: channel(base.g),
                b: channel(base.b),
                a: 255,
            };
        }
        picker.set_sv_image(slint::Image::from_rgba8(pixels));
    }
    picker.set_red(value.r.into());
    picker.set_green(value.g.into());
    picker.set_blue(value.b.into());
    picker.set_alpha(value.a.into());
    picker.set_hue(hsv.hue as f32);
    picker.set_saturation(hsv.saturation as f32);
    picker.set_brightness(hsv.value as f32);
    picker.set_hex_value(color::format_hex(value).into());
    picker.set_current_color(slint::Color::from_argb_u8(
        value.a, value.r, value.g, value.b,
    ));
}

pub(super) fn bind_picker(
    ui: &AppWindow,
    controller: SharedController,
) -> Result<Rc<ColorPickerWindow>, slint::PlatformError> {
    let picker = Rc::new(ColorPickerWindow::new()?);
    let colors = picker.as_weak();
    ui.window().on_close_requested(move || {
        if let Some(picker) = colors.upgrade() {
            let _ = picker.hide();
        }
        slint::CloseRequestResponse::HideWindow
    });
    synchronize(&picker, controller.borrow().primary_color(), None);
    picker.set_has_active_shape(controller.borrow().document().active_shape().is_some());
    let weak = picker.as_weak();
    ui.on_hide_colors(move || {
        if let Some(p) = weak.upgrade() {
            let _ = p.hide();
        }
    });
    let weak = picker.as_weak();
    ui.on_sync_color(move |r, g, b, a, active| {
        if let Some(picker) = weak.upgrade() {
            picker.set_has_active_shape(active);
            synchronize(
                &picker,
                Color::rgba(r as u8, g as u8, b as u8, a as u8),
                None,
            );
        }
    });
    let (colors, main, state) = (picker.clone(), ui.as_weak(), controller.clone());
    ui.on_show_colors(move || {
        if let Some(ui) = main.upgrade() {
            if ui.get_modal_visible() {
                return;
            }
            // Switching to our color window must preserve the editable preview,
            // even if native pointer release/capture has not settled yet.
            {
                let mut controller = state.borrow_mut();
                controller
                    .pointer_up()
                    .expect("end gesture before color editing");
                controller
                    .set_aspect_lock(false)
                    .expect("gesture has ended");
            }
            refresh(&ui, &state.borrow());
            colors.show().expect("show color picker");
        }
    });
    let (main, colors, state) = (ui.as_weak(), picker.as_weak(), controller.clone());
    picker.on_set_rgba(move |r, g, b, a| {
        let value = Color::rgba(
            r.clamp(0, 255) as u8,
            g.clamp(0, 255) as u8,
            b.clamp(0, 255) as u8,
            a.clamp(0, 255) as u8,
        );
        update(&main, &state, |c| c.set_primary_color(value));
        if let Some(picker) = colors.upgrade() {
            picker.set_error("".into());
        }
    });
    let (main, colors, state) = (ui.as_weak(), picker.as_weak(), controller.clone());
    picker.on_set_hsv(move |h, s, v| {
        if !h.is_finite() || !s.is_finite() || !v.is_finite() {
            return;
        }
        let hsv = Hsv {
            hue: f64::from(h.clamp(0.0, 360.0)),
            saturation: f64::from(s.clamp(0.0, 100.0)),
            value: f64::from(v.clamp(0.0, 100.0)),
        };
        let alpha = state.borrow().primary_color().a;
        if let Ok(value) = color::color_from_hsv(hsv.hue, hsv.saturation, hsv.value, alpha) {
            update(&main, &state, |c| c.set_primary_color(value));
            if let Some(picker) = colors.upgrade() {
                synchronize(&picker, value, Some(hsv));
                picker.set_error("".into());
            }
        }
    });
    let (main, colors, state) = (ui.as_weak(), picker.as_weak(), controller.clone());
    picker.on_set_hex(move |text| match color::parse_hex(&text) {
        Ok(value) => {
            update(&main, &state, |c| c.set_primary_color(value));
            if let Some(p) = colors.upgrade() {
                p.set_error("".into());
            }
        }
        Err(_) => {
            if let Some(p) = colors.upgrade() {
                p.set_error("Use #RGB, #RGBA, #RRGGBB or #RRGGBBAA.".into());
            }
        }
    });
    let (main, colors, state) = (ui.as_weak(), picker.as_weak(), controller);
    picker.on_pick(move || {
        if let Some(ui) = main.upgrade() {
            update(&main, &state, |c| c.set_tool(Tool::Eyedropper));
            if let Some(picker) = colors.upgrade() {
                let _ = picker.hide();
            }
            refresh(&ui, &state.borrow());
        }
    });
    let colors = picker.as_weak();
    picker.on_close_picker(move || {
        if let Some(p) = colors.upgrade() {
            let _ = p.hide();
        }
    });
    Ok(picker)
}
