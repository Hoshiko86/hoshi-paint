//! Real desktop rendering of a maximum-dimension sparse canvas.
use hoshi_canvas_spike::{create_app, refresh_workspace};
use hoshi_controller::Color;
use slint::{ComponentHandle, LogicalSize};
use std::{path::PathBuf, time::Duration};
fn screenshot(ui: &hoshi_canvas_spike::AppWindow, name: &str) {
    if let Some(directory) = std::env::var_os("HOSHI_LARGE_SNAPSHOT_DIR") {
        let snapshot = ui.window().take_snapshot().unwrap();
        image::save_buffer(
            PathBuf::from(directory).join(name),
            snapshot.as_bytes(),
            snapshot.width(),
            snapshot.height(),
            image::ColorType::Rgba8,
        )
        .unwrap();
    }
}
pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (ui, state) = create_app()?;
    ui.window().set_size(LogicalSize::new(1120., 850.));
    let weak = ui.as_weak();
    slint::Timer::single_shot(Duration::from_millis(150), move || {
        let ui = weak.upgrade().unwrap();
        state
            .borrow_mut()
            .new_image(262_144, 262_144, 96., Color::rgba(255, 255, 255, 255))
            .unwrap();
        refresh_workspace(&ui, &state);
        ui.invoke_fit_canvas();
        assert!(ui.get_viewport_preview());
        assert!(state.borrow().viewport().zoom < 0.01);
        assert!(
            hoshi_core::memory_usage() < 8 * 1024 * 1024,
            "blank canvas must not allocate a dense pixel plane"
        );
        screenshot(&ui, "spike-large-canvas-fit.png");
        state.borrow_mut().reset_view();
        refresh_workspace(&ui, &state);
        ui.invoke_pointer_down(40., 40.);
        ui.invoke_pointer_move(120., 100.);
        ui.invoke_pointer_up(120., 100.);
        ui.invoke_action(2);
        assert_eq!(
            state.borrow().document().layers()[0].pixels().get(50, 50),
            Color::rgba(0, 0, 0, 255)
        );
        assert_eq!(state.borrow().document().undo_len(), 1);
        ui.invoke_action(0);
        assert_eq!(
            state.borrow().document().layers()[0].pixels().get(50, 50),
            Color::rgba(255, 255, 255, 255)
        );
        ui.invoke_action(1);
        assert!(
            hoshi_core::memory_usage() < 8 * 1024 * 1024,
            "small edit and history must retain only touched tiles"
        );
        screenshot(&ui, "spike-large-canvas-edit.png");
        ui.window().set_size(LogicalSize::new(1280., 900.));
        let next = ui.as_weak();
        slint::Timer::single_shot(Duration::from_millis(150), move || {
            let ui = next.upgrade().unwrap();
            let size = ui.get_canvas_image().size();
            assert!(u64::from(size.width) * u64::from(size.height) <= 4_194_304);
            ui.invoke_fit_canvas();
            screenshot(&ui, "spike-large-canvas-resized.png");
            println!(
                "PASS: native maximum canvas, Fit, editing, undo/redo, viewport resize and bounded texture"
            );
            slint::quit_event_loop().unwrap();
        });
    });
    ui.run()?;
    Ok(())
}
