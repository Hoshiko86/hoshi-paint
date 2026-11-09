//! Native numeric shape bounds and locked dimensions at the default window size.
use hoshi_canvas_spike::create_app;
use slint::{ComponentHandle, LogicalSize};
use std::time::Duration;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (ui, state) = create_app()?;
    ui.window().set_size(LogicalSize::new(1120., 760.));
    let weak = ui.as_weak();
    slint::Timer::single_shot(Duration::from_millis(200), move || {
        let ui = weak.upgrade().unwrap();
        ui.invoke_pointer_down(30., 40.);
        ui.invoke_pointer_move(110., 80.);
        ui.invoke_pointer_up(110., 80.);
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
            "target/spike-shape-bounds.png",
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
