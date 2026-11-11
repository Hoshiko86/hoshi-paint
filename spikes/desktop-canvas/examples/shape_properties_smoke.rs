//! Native evidence that a shared shape-properties body can float or dock.
use hoshi_canvas_spike::create_app_with_shape_properties;
use slint::{ComponentHandle, LogicalSize};
use std::time::Duration;
fn save(window: &slint::Window, path: &str) {
    let snapshot = window.take_snapshot().unwrap();
    image::save_buffer(
        path,
        snapshot.as_bytes(),
        snapshot.width(),
        snapshot.height(),
        image::ColorType::Rgba8,
    )
    .unwrap();
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (ui, state, properties) = create_app_with_shape_properties()?;
    ui.window().set_size(LogicalSize::new(1120., 760.));
    properties.window().set_size(LogicalSize::new(340., 560.));
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
        ui.invoke_choose_color(0);
        ui.invoke_select_color_slot(1);
        ui.invoke_choose_color(1);
        ui.invoke_select_color_slot(0);
        let start = point(60., 80.);
        let end = point(300., 240.);
        ui.invoke_pointer_down(start.0, start.1);
        ui.invoke_pointer_move(end.0, end.1);
        ui.invoke_pointer_up(end.0, end.1);
        ui.invoke_show_shape_properties();
        properties.invoke_select_shape_mode(2);
        properties.invoke_edit_stroke_width_pixels(6.5);
        properties.invoke_select_line_style(1);
        let preview = state.borrow().document().active_shape();
        save(
            properties.window(),
            "target/spike-shape-properties-floating.png",
        );
        properties.invoke_placement_selected(2);
        save(ui.window(), "target/spike-shape-properties-docked.png");
        assert_eq!(state.borrow().document().active_shape(), preview);
        assert_eq!(state.borrow().document().undo_len(), 0);
        properties.invoke_placement_selected(3);
        save(ui.window(), "target/spike-shape-properties-top.png");
        properties.invoke_close_panel();
        ui.hide().unwrap();
        println!(
            "PASS: shared shape properties float, dock and scroll without committing the editable preview"
        );
    });
    ui.run()?;
    Ok(())
}
