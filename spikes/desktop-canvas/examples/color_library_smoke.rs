//! Native snapshots of the same name search and palette in floating/docked hosts.
use hoshi_canvas_spike::create_app_with_palette_root;
use slint::{ComponentHandle, LogicalPosition, LogicalSize};
use std::time::Duration;

fn save(window: &slint::Window, path: &str) {
    let pixels = window.take_snapshot().unwrap();
    image::save_buffer(
        path,
        pixels.as_bytes(),
        pixels.width(),
        pixels.height(),
        image::ColorType::Rgba8,
    )
    .unwrap();
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_app_with_palette_root(None)?;
    app.ui.window().set_size(LogicalSize::new(1120., 760.));
    app.colors.window().set_size(LogicalSize::new(640., 950.));
    let weak = app.ui.as_weak();
    slint::Timer::single_shot(Duration::from_millis(200), move || {
        let ui = weak.upgrade().unwrap();
        ui.invoke_pointer_down(50., 60.);
        ui.invoke_pointer_move(270., 230.);
        ui.invoke_pointer_up(270., 230.);
        ui.invoke_show_colors();
        app.colors.invoke_choose_named_color("RebeccaPurple".into());
        save(
            app.colors.window(),
            "target/spike-color-library-floating.png",
        );
        let preview = app.controller.borrow().document().active_shape();
        let position = LogicalPosition::new(
            app.colors.get_name_input_left() + 12.,
            app.colors.get_name_input_top() + 14.,
        );
        use slint::platform::{PointerEventButton, WindowEvent};
        app.colors
            .window()
            .dispatch_event(WindowEvent::PointerPressed {
                position,
                button: PointerEventButton::Left,
            });
        app.colors
            .window()
            .dispatch_event(WindowEvent::PointerReleased {
                position,
                button: PointerEventButton::Left,
            });
        app.colors.window().dispatch_event(WindowEvent::KeyPressed {
            text: "blue".into(),
        });
        app.colors
            .window()
            .dispatch_event(WindowEvent::KeyReleased {
                text: "blue".into(),
            });
        save(
            app.colors.window(),
            "target/spike-color-library-suggestions.png",
        );
        app.colors.invoke_placement_selected(2);
        save(ui.window(), "target/spike-color-library-docked.png");
        assert_eq!(app.controller.borrow().document().active_shape(), preview);
        assert_eq!(app.controller.borrow().document().undo_len(), 0);
        ui.hide().unwrap();
        println!(
            "PASS: floating and docked palettes share state; name search preserves editable shape"
        );
    });
    app.ui.run()?;
    Ok(())
}
