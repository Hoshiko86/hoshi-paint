use hoshi_canvas_spike::create_app_with_colors;
use hoshi_controller::ShapeStyle;
use hoshi_controller::{Color, ColorTarget};
use slint::{ComponentHandle, LogicalSize};

#[test]
fn style_controls_and_both_color_hosts_edit_separate_channels_without_committing() {
    slint::platform::set_platform(Box::new(i_slint_backend_testing::TestingBackend::new(
        i_slint_backend_testing::TestingBackendOptions {
            mock_time: true,
            threading: false,
            renderer_name: Some("software".into()),
        },
    )))
    .unwrap();
    let (ui, state, picker) = create_app_with_colors().unwrap();
    ui.window().set_size(LogicalSize::new(1120., 760.));
    ui.show().unwrap();
    ui.invoke_pointer_down(40., 40.);
    ui.invoke_pointer_move(160., 140.);
    ui.invoke_pointer_up(160., 140.);
    ui.window().take_snapshot().unwrap();
    let style =
        i_slint_backend_testing::ElementHandle::find_by_accessible_label(&ui, "Shape style")
            .next()
            .unwrap();
    style.mock_single_click(slint::platform::PointerEventButton::Left);
    for key in [
        slint::platform::Key::DownArrow,
        slint::platform::Key::DownArrow,
        slint::platform::Key::Return,
    ] {
        ui.window()
            .dispatch_event(slint::platform::WindowEvent::KeyPressed { text: key.into() });
        ui.window()
            .dispatch_event(slint::platform::WindowEvent::KeyReleased { text: key.into() });
    }
    assert_eq!(ui.get_shape_mode(), 2);
    ui.invoke_edit_stroke_width(6);
    ui.invoke_select_color_target(0);
    picker.invoke_set_rgba(240, 60, 20, 128);
    ui.invoke_show_colors();
    let fill = Color::rgba(240, 60, 20, 128);
    picker.window().take_snapshot().unwrap();
    let target = i_slint_backend_testing::ElementHandle::find_by_accessible_label(
        picker.as_ref(),
        "Color editing target",
    )
    .next()
    .unwrap();
    target.mock_single_click(slint::platform::PointerEventButton::Left);
    for key in [
        slint::platform::Key::DownArrow,
        slint::platform::Key::Return,
    ] {
        picker
            .window()
            .dispatch_event(slint::platform::WindowEvent::KeyPressed { text: key.into() });
        picker
            .window()
            .dispatch_event(slint::platform::WindowEvent::KeyReleased { text: key.into() });
    }
    assert_eq!(state.borrow().color_target(), ColorTarget::Outline);
    let original_outline = state.borrow().outline_color();
    picker.invoke_set_rgba(10, 40, 220, 255);
    let outline = Color::rgba(10, 40, 220, 255);
    assert_eq!(
        state.borrow().document().active_shape().unwrap().style,
        ShapeStyle::Both {
            fill,
            outline,
            width: 6
        }
    );
    picker.invoke_restore_original();
    assert_eq!(state.borrow().outline_color(), original_outline);
    picker.invoke_set_rgba(10, 40, 220, 255);
    ui.invoke_set_colors_placement(1);
    ui.invoke_select_color_target(0);
    assert_eq!(ui.get_color_r(), 240);
    ui.invoke_edit_rgba(30, 180, 70, 200);
    let new_fill = Color::rgba(30, 180, 70, 200);
    assert_eq!(state.borrow().fill_color(), new_fill);
    assert_eq!(state.borrow().outline_color(), outline);
    ui.invoke_restore_original_color();
    assert_eq!(state.borrow().fill_color(), fill);
    // Returning to a channel keeps its original baseline, rather than recapturing edits.
    ui.invoke_select_color_target(1);
    ui.invoke_restore_original_color();
    assert_eq!(state.borrow().outline_color(), original_outline);
    ui.invoke_edit_rgba(10, 40, 220, 255);
    ui.invoke_select_shape_mode(1);
    let pixels = state.borrow().composite();
    assert_eq!(pixels[80 * 512 + 80].a, 0);
    assert_eq!(pixels[40 * 512 + 40], outline);
    assert_eq!(state.borrow().document().undo_len(), 0);
    ui.window().take_snapshot().unwrap();
    ui.invoke_action(2);
    assert_eq!(state.borrow().document().undo_len(), 1);
    ui.invoke_action(0);
    assert!(state.borrow().composite().iter().all(|p| p.a == 0));
    ui.invoke_action(1);
    assert_eq!(state.borrow().composite(), pixels);
    ui.hide().unwrap();
}
