//! Native settings dialogs, dimension changes and whole-image history.
use slint::ComponentHandle;
use std::{path::PathBuf, time::Duration};
fn save(ui: &hoshi_canvas_spike::AppWindow, name: &str) {
    let snapshot = ui.window().take_snapshot().unwrap();
    if let Some(directory) = std::env::var_os("HOSHI_IMAGE_SNAPSHOT_DIR") {
        let path = PathBuf::from(directory).join(name);
        image::save_buffer(
            path,
            snapshot.as_bytes(),
            snapshot.width(),
            snapshot.height(),
            image::ColorType::Rgba8,
        )
        .unwrap();
    }
}
pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (ui, state) = hoshi_canvas_spike::create_app()?;
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
        ui.invoke_show_image_settings(0);
        ui.invoke_select_image_aspect(false);
        ui.invoke_edit_image_setting(0, "320".into());
        ui.invoke_edit_image_setting(1, "200".into());
        save(&ui, "spike-image-new.png");
        ui.invoke_apply_image_settings();
        assert_eq!((ui.get_image_width(), ui.get_image_height()), (320, 200));
        let start = point(20., 20.);
        let end = point(100., 80.);
        ui.invoke_pointer_down(start.0, start.1);
        ui.invoke_pointer_move(end.0, end.1);
        ui.invoke_pointer_up(end.0, end.1);
        ui.invoke_show_image_settings(1);
        ui.invoke_edit_image_setting(0, "480".into());
        ui.invoke_edit_image_setting(2, "144".into());
        save(&ui, "spike-image-resize.png");
        ui.invoke_apply_image_settings();
        assert_eq!((ui.get_image_width(), ui.get_image_height()), (480, 300));
        assert_eq!(state.borrow().document().resolution(), 144.);
        ui.invoke_show_image_settings(2);
        ui.invoke_edit_image_setting(0, "640".into());
        ui.invoke_edit_image_setting(1, "400".into());
        ui.set_image_anchor(4);
        ui.set_image_background(1);
        save(&ui, "spike-canvas-size.png");
        ui.invoke_apply_image_settings();
        assert_eq!((ui.get_image_width(), ui.get_image_height()), (640, 400));
        ui.invoke_image_command(2);
        assert_eq!((ui.get_image_width(), ui.get_image_height()), (400, 640));
        ui.invoke_action(0);
        assert_eq!((ui.get_image_width(), ui.get_image_height()), (640, 400));
        ui.invoke_action(0);
        assert_eq!((ui.get_image_width(), ui.get_image_height()), (480, 300));
        ui.invoke_action(1);
        assert_eq!((ui.get_image_width(), ui.get_image_height()), (640, 400));
        save(&ui, "spike-image-edited.png");
        println!(
            "PASS: native New, Resize and Canvas Size dialogs, all-layer rotation and dimension history"
        );
        slint::quit_event_loop().unwrap();
    });
    ui.run()?;
    Ok(())
}
