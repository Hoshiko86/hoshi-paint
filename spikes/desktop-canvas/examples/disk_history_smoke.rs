//! Real desktop long-history browsing with disk-backed states and shared hosts.
use slint::{ComponentHandle, LogicalSize, Model};
use std::{path::PathBuf, time::Duration};

fn snapshot(ui: &hoshi_canvas_spike::AppWindow) {
    let directory =
        std::env::var_os("HOSHI_HISTORY_SNAPSHOT_DIR").unwrap_or_else(|| "target".into());
    let pixels = ui.window().take_snapshot().unwrap();
    image::save_buffer(
        PathBuf::from(directory).join("spike-disk-history.png"),
        pixels.as_bytes(),
        pixels.width(),
        pixels.height(),
        image::ColorType::Rgba8,
    )
    .unwrap();
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (ui, state, _, history) = hoshi_canvas_spike::create_app_with_panels()?;
    ui.window().set_size(LogicalSize::new(1260., 900.));
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
        let start = point(40., 40.);
        let end = point(240., 190.);
        ui.invoke_pointer_down(start.0, start.1);
        ui.invoke_pointer_move(end.0, end.1);
        ui.invoke_pointer_up(end.0, end.1);
        ui.invoke_action(2);
        for index in 0..512 {
            state
                .borrow_mut()
                .rename_layer(&format!("Drawing {index}"))
                .unwrap();
        }
        hoshi_canvas_spike::refresh_workspace(&ui, &state);
        ui.invoke_show_history();
        ui.invoke_set_history_placement(2);
        let rows = ui.get_history_rows();
        assert_eq!(rows.row_count(), 514);
        assert!(state.borrow().document().history_storage_bytes() > 0);
        ui.invoke_history_jump(0);
        assert_eq!(state.borrow().document().layers()[0].name(), "Background");
        assert_eq!(
            state.borrow().document().composite_pixel(60, 60, false).a,
            0
        );
        ui.invoke_history_jump(513);
        assert_eq!(state.borrow().document().layers()[0].name(), "Drawing 511");
        assert_eq!(
            state.borrow().document().composite_pixel(60, 60, false).a,
            255
        );
        assert_eq!(ui.get_history_rows(), rows);
        assert_eq!(history.get_rows(), rows);
        assert!(history.get_status_text().contains("on disk"));
        slint::Timer::single_shot(Duration::from_millis(150), move || {
            let ui = weak.upgrade().unwrap();
            snapshot(&ui);
            ui.invoke_set_history_placement(0);
            assert!(history.window().is_visible());
            assert_eq!(history.get_rows().row_count(), 514);
            ui.invoke_file_action(6);
            while ui.get_file_prompt_visible() {
                ui.invoke_file_prompt_response(1);
            }
            assert!(!ui.window().is_visible());
            assert!(!history.window().is_visible());
            println!(
                "PASS: 513 disk-backed undo steps, direct oldest/latest jumps, shared docked/floating model and session close"
            );
        });
    });
    ui.run()?;
    Ok(())
}
