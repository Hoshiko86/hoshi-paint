//! Actual desktop color-window lifecycle; input is synthetic, not physical OS input.
use hoshi_controller::Color;
use slint::ComponentHandle;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (ui, state, picker) = hoshi_canvas_spike::create_app_with_colors()?;
    let main = ui.as_weak();
    slint::Timer::single_shot(Duration::from_millis(150), move || {
        let ui = main.upgrade().unwrap();
        ui.invoke_pointer_down(64.0, 64.0);
        ui.invoke_pointer_move(256.0, 224.0);
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
        let pixel = canvas.as_slice()[(ui.get_canvas_top() as usize + 128)
            * canvas.width() as usize
            + ui.get_canvas_left() as usize
            + 128];
        assert_eq!(
            (pixel.r, pixel.g, pixel.b),
            (selected.r, selected.g, selected.b)
        );
        picker.invoke_set_hsv(120.0, 100.0, 100.0);
        assert_eq!(state.borrow().primary_color(), Color::rgba(0, 255, 0, 255));
        picker.invoke_set_hex("#3C87E080".into());
        picker.invoke_pick();
        assert!(!picker.get_has_active_shape());
        assert_eq!(state.borrow().document().undo_len(), 1);
        picker.invoke_set_rgba(0, 0, 0, 255);
        ui.invoke_pointer_down(80.0, 80.0);
        ui.invoke_pointer_up(80.0, 80.0);
        assert_eq!(
            state.borrow().primary_color(),
            Color::rgba(60, 135, 224, 128)
        );
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
            assert!(!picker.window().is_visible());
            println!(
                "PASS: desktop color picker rendered, HSV/hex edits and alpha-preserving eyedropper sampling succeeded."
            );
        });
    });
    ui.run()?;
    Ok(())
}
