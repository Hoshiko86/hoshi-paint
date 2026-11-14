//! Native numeric shape bounds and locked dimensions at the default window size.
use hoshi_canvas_spike::create_app;
use slint::{ComponentHandle, LogicalSize};
use std::time::Duration;
/// Snapshot directory: `HOSHI_SMOKE_SNAPSHOT_DIR` (set by `cargo xtask test
/// smoke`), else `target/`. Created if missing so a fresh checkout works.
fn snapshot_path(name: &str) -> std::path::PathBuf {
    let dir = std::path::PathBuf::from(
        std::env::var_os("HOSHI_SMOKE_SNAPSHOT_DIR").unwrap_or_else(|| "target".into()),
    );
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}
pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (ui, state) = create_app()?;
    ui.window().set_size(LogicalSize::new(1120., 760.));
    let weak = ui.as_weak();
    slint::Timer::single_shot(Duration::from_millis(200), move || {
        let ui = weak.upgrade().unwrap();
        ui.window().take_snapshot().expect("settle native layout");
        let point = |x, y| {
            (
                ui.get_offset_x() + x * ui.get_zoom(),
                ui.get_offset_y() + y * ui.get_zoom(),
            )
        };
        ui.invoke_set_shape_properties_placement(2);
        let start = point(30., 40.);
        let end = point(110., 80.);
        ui.invoke_pointer_down(start.0, start.1);
        ui.invoke_pointer_move(end.0, end.1);
        ui.invoke_pointer_up(end.0, end.1);
        ui.invoke_edit_shape_bound(0, "36".into(), false);
        ui.invoke_edit_shape_bound(1, "42".into(), false);
        ui.set_shape_bounds_locked(true);
        ui.invoke_edit_shape_bound(2, "240".into(), true);
        state.borrow_mut().set_shape_rotation(15).unwrap();
        hoshi_canvas_spike::refresh_workspace(&ui, &state);
        let shape = state.borrow().document().active_shape().unwrap();
        assert_eq!(
            (shape.x, shape.y, shape.width, shape.height),
            (36, 42, 240, 120)
        );
        assert_eq!(state.borrow().document().undo_len(), 0);
        let image = ui.window().take_snapshot().unwrap();
        image::save_buffer(
            snapshot_path("spike-shape-bounds.png"),
            image.as_bytes(),
            image.width(),
            image.height(),
            image::ColorType::Rgba8,
        )
        .unwrap();
        ui.invoke_action(2);
        assert_eq!(state.borrow().document().undo_len(), 1);
        ui.invoke_action(0);
        assert_eq!(state.borrow().document().undo_len(), 0);
        ui.hide().unwrap();
        println!(
            "PASS: native numeric position/dimensions, locked proportions, rotated preview, commit and undo"
        );
    });
    ui.run()?;
    Ok(())
}
