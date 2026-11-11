//! Native desktop lifecycle for independent dock locations and history browsing.
use slint::{ComponentHandle, Model};
use std::time::Duration;

fn save(window: &slint::Window, variable: &str) {
    if let Some(path) = std::env::var_os(variable) {
        let image = window.take_snapshot().unwrap();
        image::save_buffer(
            path,
            image.as_bytes(),
            image.width(),
            image.height(),
            image::ColorType::Rgba8,
        )
        .unwrap();
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (ui, state, colors, history) = hoshi_canvas_spike::create_app_with_panels()?;
    let weak = ui.as_weak();
    slint::Timer::single_shot(Duration::from_millis(150), move || {
        let ui = weak.upgrade().unwrap();
        ui.window().take_snapshot().expect("settle native layout");
        let point = |x, y| {
            (
                ui.get_offset_x() + x * ui.get_zoom(),
                ui.get_offset_y() + y * ui.get_zoom(),
            )
        };
        for (x, y) in [(40.0, 40.0), (200.0, 180.0)] {
            let start = point(x, y);
            let end = point(x + 100.0, y + 60.0);
            ui.invoke_pointer_down(start.0, start.1);
            ui.invoke_pointer_move(end.0, end.1);
            ui.invoke_pointer_up(end.0, end.1);
            ui.invoke_action(2);
        }
        ui.invoke_show_colors();
        ui.invoke_show_history();
        ui.invoke_set_colors_placement(1);
        ui.invoke_set_history_placement(2);
        slint::Timer::single_shot(Duration::from_millis(200), move || {
            ui.window().take_snapshot().unwrap();
            assert!(ui.get_colors_host_left() < ui.get_canvas_left());
            assert!(ui.get_history_host_left() >= ui.get_canvas_left() + ui.get_canvas_width());
            assert!(ui.get_canvas_width() >= 239.0);
            assert_eq!(history.get_rows().row_count(), 3);
            save(ui.window(), "HOSHI_PANELS_SNAPSHOT");
            ui.invoke_history_jump(0);
            assert!(state.borrow().composite().iter().all(|p| p.a == 0));
            ui.invoke_history_redo();
            assert_eq!(state.borrow().history_position(), 1);
            ui.invoke_set_history_placement(0);
            slint::Timer::single_shot(Duration::from_millis(200), move || {
                assert!(history.window().is_visible());
                save(history.window(), "HOSHI_HISTORY_SNAPSHOT");
                ui.window()
                    .dispatch_event(slint::platform::WindowEvent::CloseRequested);
                while ui.get_file_prompt_visible() {
                    ui.invoke_file_prompt_response(1);
                }
                assert!(!colors.window().is_visible() && !history.window().is_visible());
                println!(
                    "PASS: native left/right dock hosts, history navigation, floating history and app-close lifecycle"
                );
            });
        });
    });
    ui.run()?;
    Ok(())
}
