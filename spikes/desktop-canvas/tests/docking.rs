use hoshi_canvas_spike::create_app_with_colors;
use hoshi_controller::{Color, Tool};
use slint::{ComponentHandle, LogicalSize};

fn click(window: &slint::Window, x: f32, y: f32) {
    let position = slint::LogicalPosition::new(x, y);
    window.dispatch_event(slint::platform::WindowEvent::PointerPressed {
        position,
        button: slint::platform::PointerEventButton::Left,
    });
    window.dispatch_event(slint::platform::WindowEvent::PointerReleased {
        position,
        button: slint::platform::PointerEventButton::Left,
    });
}

#[test]
fn colors_float_dock_reopen_and_pointer_edits_share_one_editable_shape() {
    slint::platform::set_platform(Box::new(i_slint_backend_testing::TestingBackend::new(
        i_slint_backend_testing::TestingBackendOptions {
            mock_time: true,
            threading: false,
            renderer_name: Some("software".into()),
        },
    )))
    .unwrap();
    let (ui, state, picker) = create_app_with_colors().unwrap();
    ui.window().set_size(LogicalSize::new(1120.0, 920.0));
    ui.show().unwrap();
    ui.invoke_pointer_down(40.0, 40.0);
    ui.invoke_pointer_move(140.0, 100.0);
    ui.invoke_pointer_up(140.0, 100.0);
    ui.invoke_show_colors();
    picker.invoke_set_rgba(12, 34, 56, 255);
    let geometry = state.borrow().document().active_shape().unwrap();
    picker.invoke_toggle_dock();
    assert!(ui.get_colors_docked() && ui.get_colors_visible());
    assert!(!picker.window().is_visible());
    ui.window().take_snapshot().unwrap();
    assert!(
        ui.get_docked_red_width() > 170.0,
        "narrow dock must have a readable full-width slider"
    );
    click(
        ui.window(),
        ui.get_docked_red_left() + ui.get_docked_red_width() * 0.9,
        ui.get_docked_red_top() + ui.get_docked_red_height() * 0.5,
    );
    let selected = state.borrow().primary_color();
    assert_ne!(selected.r, 12, "docked slider must route actual input");
    assert_eq!(state.borrow().composite()[60 * 512 + 60], selected);
    assert_eq!(picker.get_red(), i32::from(selected.r));
    // Narrow panels stack channels; scroll their body to the palette instead
    // of assuming every control fits in the visible height.
    let viewport = state.borrow().viewport();
    ui.window()
        .dispatch_event(slint::platform::WindowEvent::PointerScrolled {
            position: slint::LogicalPosition::new(
                ui.get_colors_host_left() + ui.get_colors_host_width() - 10.0,
                ui.get_colors_host_top() + 100.0,
            ),
            delta_x: 0.0,
            delta_y: -500.0,
        });
    i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(1000));
    assert_eq!(
        state.borrow().viewport(),
        viewport,
        "panel scrolling must not pan the canvas"
    );
    let snapshot = ui.window().take_snapshot().unwrap();
    assert!(ui.get_docked_swatches_top() + 40.0 < snapshot.height() as f32);
    let x = ui.get_docked_swatches_left() as u32 + 20;
    let y = ui.get_docked_swatches_top() as u32 + 20;
    let pixel = snapshot.as_slice()[(y * snapshot.width() + x) as usize];
    assert_eq!(
        (pixel.r, pixel.g, pixel.b),
        (72, 136, 240),
        "swatch must show its actual RGB value"
    );
    click(ui.window(), x as f32, y as f32);
    assert_eq!(
        state.borrow().primary_color(),
        Color::rgba(72, 136, 240, 255)
    );
    ui.invoke_toggle_color_dock();
    assert!(!ui.get_colors_docked());
    assert!(picker.window().is_visible());
    picker.invoke_set_hex("#123456".into());
    assert_eq!(
        state.borrow().composite()[60 * 512 + 60],
        Color::rgba(18, 52, 86, 255)
    );
    // Native capture loss during the former sampling mode must not sever the
    // active shape's connection to either host's color controls.
    state.borrow_mut().set_tool(Tool::Eyedropper).unwrap();
    ui.invoke_pointer_down(60.0, 60.0);
    ui.invoke_pointer_cancel();
    picker.invoke_set_rgba(90, 120, 150, 255);
    assert_eq!(
        state.borrow().composite()[60 * 512 + 60],
        Color::rgba(90, 120, 150, 255)
    );
    state.borrow_mut().set_tool(Tool::Rectangle).unwrap();
    ui.invoke_toggle_color_dock();
    ui.invoke_close_colors();
    assert!(!ui.get_colors_visible());
    ui.invoke_show_colors();
    assert!(ui.get_colors_docked() && ui.get_colors_visible());
    assert!(!picker.window().is_visible());
    let shape = state.borrow().document().active_shape().unwrap();
    assert_eq!(
        (shape.x, shape.y, shape.width, shape.height),
        (geometry.x, geometry.y, geometry.width, geometry.height)
    );
    assert_eq!(state.borrow().document().undo_len(), 0);
    ui.window().set_size(LogicalSize::new(1120.0, 760.0));
    ui.window().take_snapshot().unwrap();
    ui.invoke_action(2);
    assert_eq!(state.borrow().document().undo_len(), 1);
    ui.invoke_action(0);
    assert_eq!(state.borrow().composite()[60 * 512 + 60].a, 0);
}
