use hoshi_canvas_spike::create_app_with_shape_properties;
use hoshi_controller::{FillPattern, LineStyle};
use slint::platform::{Key, PointerEventButton as Button, WindowEvent};
use slint::{ComponentHandle, LogicalPosition, LogicalSize};

fn key(ui: &impl ComponentHandle, text: impl Into<slint::SharedString>, pressed: bool) {
    let text = text.into();
    ui.window().dispatch_event(if pressed {
        WindowEvent::KeyPressed { text }
    } else {
        WindowEvent::KeyReleased { text }
    });
}
fn click(ui: &impl ComponentHandle, x: f32, y: f32) {
    for pressed in [true, false] {
        ui.window().dispatch_event(if pressed {
            WindowEvent::PointerPressed {
                position: LogicalPosition::new(x, y),
                button: Button::Left,
            }
        } else {
            WindowEvent::PointerReleased {
                position: LogicalPosition::new(x, y),
                button: Button::Left,
            }
        });
    }
}
#[test]
fn rendering_controls_update_preview_defaults_and_preserve_text_focus() {
    slint::platform::set_platform(Box::new(i_slint_backend_testing::TestingBackend::new(
        i_slint_backend_testing::TestingBackendOptions {
            mock_time: true,
            threading: false,
            renderer_name: Some("software".into()),
        },
    )))
    .unwrap();
    let (ui, state, properties) = create_app_with_shape_properties().unwrap();
    ui.window().set_size(LogicalSize::new(1120., 850.));
    ui.show().unwrap();
    ui.window().take_snapshot().unwrap();
    ui.invoke_select_shape_mode(2);
    ui.invoke_pointer_down(40., 50.);
    ui.invoke_pointer_move(200., 150.);
    ui.invoke_pointer_up(200., 150.);
    let original = state.borrow().composite();
    ui.invoke_edit_stroke_width_pixels(0.01);
    assert_eq!(state.borrow().stroke_width_pixels(), 0.01);
    ui.invoke_edit_stroke_width_pixels(6.5);
    assert_eq!(state.borrow().stroke_width_pixels(), 6.5);
    assert_ne!(state.borrow().composite(), original);
    ui.invoke_select_line_style(3);
    assert_eq!(state.borrow().line_style(), LineStyle::DashDot);
    ui.invoke_select_fill_pattern(7);
    assert_eq!(state.borrow().fill_pattern(), FillPattern::Checkerboard);
    ui.invoke_toggle_shape_antialiasing(false);
    assert!(!state.borrow().antialiasing());
    assert_eq!(
        state.borrow().document().undo_len(),
        0,
        "preview adjustments must not commit"
    );
    assert!(state.borrow().document().active_shape().is_some());
    ui.invoke_show_shape_properties();
    properties.window().take_snapshot().unwrap();
    // An actual decimal edit must keep the trailing decimal point until typing finishes.
    click(
        properties.as_ref(),
        properties.get_outline_input_left() + 12.,
        properties.get_outline_input_top() + 12.,
    );
    key(properties.as_ref(), Key::Control, true);
    key(properties.as_ref(), "a", true);
    key(properties.as_ref(), "a", false);
    key(properties.as_ref(), Key::Control, false);
    for character in ["3", ".", "2", "5"] {
        key(properties.as_ref(), character, true);
        key(properties.as_ref(), character, false);
    }
    assert_eq!(state.borrow().stroke_width_pixels(), 3.25);
    key(properties.as_ref(), "]", true);
    key(properties.as_ref(), "]", false);
    assert_eq!(
        state.borrow().stroke_width_pixels(),
        3.25,
        "brackets in a text field must not run canvas shortcuts"
    );
    // Focus the canvas away from the control points.
    click(&ui, ui.get_canvas_left() + 100., ui.get_canvas_top() + 100.);
    key(&ui, "]", true);
    key(&ui, "]", false);
    assert_eq!(state.borrow().stroke_width_pixels(), 4.25);
    key(&ui, Key::Control, true);
    key(&ui, "]", true);
    key(&ui, "]", false);
    key(&ui, Key::Control, false);
    assert_eq!(state.borrow().stroke_width_pixels(), 9.25);
    ui.invoke_action(2);
    assert_eq!(state.borrow().document().undo_len(), 1);
    ui.invoke_pointer_down(230., 50.);
    ui.invoke_pointer_move(350., 150.);
    ui.invoke_pointer_up(350., 150.);
    assert_eq!(state.borrow().stroke_width_pixels(), 9.25);
    assert_eq!(state.borrow().line_style(), LineStyle::DashDot);
    assert_eq!(state.borrow().fill_pattern(), FillPattern::Checkerboard);
    assert!(!state.borrow().antialiasing());
    let next = state.borrow().document().active_shape().unwrap();
    assert_eq!(next.rendering.line_style, LineStyle::DashDot);
    assert_eq!(next.rendering.fill_pattern, FillPattern::Checkerboard);
    assert!(!next.rendering.antialias);
    assert!(matches!(
        next.style,
        hoshi_core::ShapeStyle::Both { width: 9.25, .. }
    ));

    ui.hide().unwrap();
}
