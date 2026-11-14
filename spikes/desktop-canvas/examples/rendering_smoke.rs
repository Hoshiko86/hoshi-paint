//! Native visual check of the shared shape-rendering controls and live preview.
use hoshi_canvas_spike::{create_app, refresh_workspace};
use slint::{ComponentHandle, LogicalSize};
use std::{path::PathBuf, time::Duration};

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
        let draw = |left, top, right, bottom| {
            let start = point(left, top);
            let end = point(right, bottom);
            ui.invoke_pointer_down(start.0, start.1);
            ui.invoke_pointer_move(end.0, end.1);
            ui.invoke_pointer_up(end.0, end.1);
        };
        // Solid coral makes the gaps in the fractional blue outline obvious.
        ui.invoke_select_shape_mode(2);
        ui.invoke_choose_color(0);
        ui.invoke_select_color_slot(1);
        ui.invoke_choose_color(1);
        ui.invoke_select_color_slot(0);
        ui.invoke_edit_stroke_width_pixels(6.5);
        ui.invoke_select_line_style(1);
        draw(35., 45., 215., 145.);
        ui.invoke_action(2);
        // A rotated solid edge makes partial pixel coverage visible separately
        // from the pattern sampler and outline dash sampler.
        ui.invoke_select_shape_mode(0);
        ui.invoke_choose_color(2);
        ui.invoke_select_line_style(0);
        draw(280., 55., 430., 155.);
        state.borrow_mut().set_shape_rotation(21).unwrap();
        refresh_workspace(&ui, &state);
        ui.invoke_action(2);
        // Keep the third shape editable so the controls and live defaults can
        // be reviewed together with the already committed rendering examples.
        ui.invoke_select_shape_mode(2);
        ui.invoke_choose_color(0);
        ui.invoke_select_fill_pattern(6);
        draw(115., 240., 395., 390.);
        assert_eq!(state.borrow().stroke_width_pixels(), 6.5);
        assert!(state.borrow().antialiasing());
        assert_eq!(state.borrow().document().undo_len(), 2);
        let snapshot = ui.window().take_snapshot().unwrap();
        assert!(
            ui.get_layer_input_top() + 32. < snapshot.height() as f32,
            "the complete sidebar must fit at the default 760px window height"
        );
        let path = PathBuf::from(
            std::env::var_os("HOSHI_RENDERING_SNAPSHOT")
                .unwrap_or_else(|| "target/spike-shape-rendering.png".into()),
        );
        image::save_buffer(
            path,
            snapshot.as_bytes(),
            snapshot.width(),
            snapshot.height(),
            image::ColorType::Rgba8,
        )
        .unwrap();
        ui.invoke_action(2);
        assert_eq!(state.borrow().document().undo_len(), 3);
        ui.invoke_action(0);
        assert_eq!(state.borrow().document().undo_len(), 2);
        ui.hide().unwrap();
        println!(
            "PASS: native decimal outlines, line styles, fill patterns, antialiasing controls and commit/undo"
        );
    });
    ui.run()?;
    Ok(())
}
