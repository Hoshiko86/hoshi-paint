//! Actual desktop color-window lifecycle; input is synthetic, not physical OS input.
use hoshi_controller::Color;
use slint::ComponentHandle;
use std::time::Duration;

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (ui, state, picker) = hoshi_canvas_spike::create_app_with_colors()?;
    let main = ui.as_weak();
    slint::Timer::single_shot(Duration::from_millis(150), move || {
        let ui = main.upgrade().unwrap();
        ui.window().take_snapshot().expect("settle native layout");
        let document_point = |x, y| {
            (
                ui.get_offset_x() + x * ui.get_zoom(),
                ui.get_offset_y() + y * ui.get_zoom(),
            )
        };
        picker.invoke_set_hex("#3C87E080".into());
        let start = document_point(10.0, 10.0);
        let end = document_point(30.0, 30.0);
        ui.invoke_pointer_down(start.0, start.1);
        ui.invoke_pointer_move(end.0, end.1);
        ui.invoke_pointer_up(end.0, end.1);
        ui.invoke_action(2);
        picker.invoke_set_rgba(72, 136, 240, 255);
        let start = document_point(64.0, 64.0);
        let end = document_point(256.0, 224.0);
        ui.invoke_pointer_down(start.0, start.1);
        ui.invoke_pointer_move(end.0, end.1);
        ui.invoke_show_colors();
        assert!(picker.get_has_active_shape());
        picker.window().take_snapshot().unwrap();
        let position = slint::LogicalPosition::new(
            picker.get_sv_left() + picker.get_sv_width() * 0.8,
            picker.get_sv_top() + picker.get_sv_height() * 0.2,
        );
        picker
            .window()
            .dispatch_event(slint::platform::WindowEvent::PointerPressed {
                position,
                button: slint::platform::PointerEventButton::Left,
            });
        picker
            .window()
            .dispatch_event(slint::platform::WindowEvent::PointerReleased {
                position,
                button: slint::platform::PointerEventButton::Left,
            });
        let selected = state.borrow().primary_color();
        assert_eq!(state.borrow().composite()[128 * 512 + 128], selected);
        let canvas = ui.window().take_snapshot().unwrap();
        let document_pixel = document_point(128.0, 128.0);
        let pixel_x = (ui.get_canvas_left() + document_pixel.0).round() as usize;
        let pixel_y = (ui.get_canvas_top() + document_pixel.1).round() as usize;
        let pixel = canvas.as_slice()[pixel_y * canvas.width() as usize + pixel_x];
        assert_eq!(
            (pixel.r, pixel.g, pixel.b),
            (selected.r, selected.g, selected.b)
        );
        picker.invoke_set_hsv(120.0, 100.0, 100.0);
        assert_eq!(state.borrow().primary_color(), Color::rgba(0, 255, 0, 255));
        picker.invoke_set_hex("#3C87E080".into());
        assert!(picker.get_has_active_shape());
        picker.invoke_set_rgba(0, 0, 0, 255);
        picker.invoke_set_hex("#3C87E080".into());
        assert_eq!(
            state.borrow().composite()[128 * 512 + 128],
            Color::rgba(60, 135, 224, 128)
        );
        picker.invoke_toggle_dock();
        assert!(ui.get_colors_docked());
        assert!(!picker.window().is_visible());
        ui.invoke_edit_rgba(80, 140, 200, 255);
        assert_eq!(
            state.borrow().composite()[128 * 512 + 128],
            Color::rgba(80, 140, 200, 255)
        );
        ui.invoke_select_color_slot(1);
        ui.invoke_edit_rgba(80, 140, 200, 255);
        ui.invoke_select_shape_mode(2);
        ui.invoke_edit_stroke_width_pixels(8.0);
        ui.invoke_select_color_slot(0);
        ui.invoke_edit_rgba(210, 60, 70, 255);
        let shape = state.borrow().document().active_shape().unwrap();
        let edge = shape.y as usize * 512 + shape.x as usize;
        assert_eq!(
            state.borrow().composite()[edge],
            Color::rgba(210, 60, 70, 255)
        );
        assert_eq!(
            state.borrow().composite()[128 * 512 + 128],
            Color::rgba(80, 140, 200, 255)
        );
        if let Some(path) = std::env::var_os("HOSHI_DOCK_SNAPSHOT") {
            let snapshot = ui.window().take_snapshot().unwrap();
            image::save_buffer(
                path,
                snapshot.as_bytes(),
                snapshot.width(),
                snapshot.height(),
                image::ColorType::Rgba8,
            )
            .unwrap();
        }
        ui.invoke_toggle_color_dock();
        assert!(!ui.get_colors_docked());
        assert!(picker.window().is_visible());
        ui.invoke_show_colors();
        slint::Timer::single_shot(Duration::from_millis(200), move || {
            let snapshot = picker.window().take_snapshot().unwrap();
            assert!(snapshot.width() >= 640 && snapshot.height() >= 660);
            if let Some(path) = std::env::var_os("HOSHI_COLOR_SNAPSHOT") {
                image::save_buffer(
                    path,
                    snapshot.as_bytes(),
                    snapshot.width(),
                    snapshot.height(),
                    image::ColorType::Rgba8,
                )
                .unwrap();
            }
            ui.window()
                .dispatch_event(slint::platform::WindowEvent::CloseRequested);
            while ui.get_file_prompt_visible() {
                ui.invoke_file_prompt_response(1);
            }
            assert!(!picker.window().is_visible());
            println!(
                "PASS: desktop color picker rendered, HSV/hex edits and floating/docked fill and outline editing succeeded."
            );
        });
    });
    ui.run()?;
    Ok(())
}
