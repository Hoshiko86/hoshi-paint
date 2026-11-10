use hoshi_canvas_spike::create_app_with_all_panels;
use hoshi_controller::ShapeStyle;
use hoshi_controller::{Color, ColorSlot};
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
    let app = create_app_with_all_panels().unwrap();
    let (ui, state, picker, properties) = (app.ui, app.controller, app.colors, app.shapes);
    ui.window().set_size(LogicalSize::new(1120., 760.));
    ui.show().unwrap();
    ui.invoke_pointer_down(40., 40.);
    ui.invoke_pointer_move(160., 140.);
    ui.invoke_pointer_up(160., 140.);
    ui.window().take_snapshot().unwrap();
    ui.invoke_show_shape_properties();
    properties.window().take_snapshot().unwrap();
    let style = i_slint_backend_testing::ElementHandle::find_by_accessible_label(
        properties.as_ref(),
        "Shape style",
    )
    .next()
    .unwrap();
    style.mock_single_click(slint::platform::PointerEventButton::Left);
    for key in [
        slint::platform::Key::DownArrow,
        slint::platform::Key::DownArrow,
        slint::platform::Key::Return,
    ] {
        properties
            .window()
            .dispatch_event(slint::platform::WindowEvent::KeyPressed { text: key.into() });
        properties
            .window()
            .dispatch_event(slint::platform::WindowEvent::KeyReleased { text: key.into() });
    }
    assert_eq!(ui.get_shape_mode(), 2);
    ui.invoke_edit_stroke_width(6);
    ui.invoke_select_color_slot(0);
    picker.invoke_set_rgba(240, 60, 20, 128);
    ui.invoke_show_colors();
    let fill = Color::rgba(240, 60, 20, 128);
    picker.window().take_snapshot().unwrap();
    let secondary_button = i_slint_backend_testing::ElementHandle::find_by_accessible_label(
        picker.as_ref(),
        "Secondary color",
    )
    .next()
    .unwrap();
    secondary_button.mock_single_click(slint::platform::PointerEventButton::Left);
    assert_eq!(state.borrow().color_slot(), ColorSlot::Secondary);
    let original_outline = state.borrow().secondary_color();
    picker.invoke_set_rgba(10, 40, 220, 255);
    let outline = Color::rgba(10, 40, 220, 255);
    assert_eq!(
        state.borrow().document().active_shape().unwrap().style,
        ShapeStyle::Both {
            fill: outline,
            outline: fill,
            width: 6.0
        }
    );
    picker.invoke_restore_original();
    assert_eq!(state.borrow().secondary_color(), original_outline);
    picker.invoke_set_rgba(10, 40, 220, 255);
    ui.invoke_set_colors_placement(1);
    ui.invoke_select_color_slot(0);
    assert_eq!(ui.get_color_r(), 240);
    ui.invoke_edit_rgba(30, 180, 70, 200);
    let new_fill = Color::rgba(30, 180, 70, 200);
    assert_eq!(state.borrow().primary_color(), new_fill);
    assert_eq!(state.borrow().secondary_color(), outline);
    ui.invoke_restore_original_color();
    assert_eq!(state.borrow().primary_color(), fill);
    // Returning to a channel keeps its original baseline, rather than recapturing edits.
    ui.invoke_select_color_slot(1);
    ui.invoke_restore_original_color();
    assert_eq!(state.borrow().secondary_color(), original_outline);
    ui.invoke_edit_rgba(10, 40, 220, 255);
    let before_swap = state.borrow().document().active_shape().unwrap();
    picker.invoke_swap_colors();
    assert_eq!(state.borrow().primary_color(), outline);
    assert_eq!(state.borrow().secondary_color(), fill);
    assert_eq!(
        ui.get_workspace_primary(),
        slint::Color::from_argb_u8(outline.a, outline.r, outline.g, outline.b)
    );
    ui.invoke_swap_colors();
    assert_eq!(
        state.borrow().document().active_shape().unwrap(),
        before_swap
    );
    picker.invoke_reset_colors();
    assert_eq!(state.borrow().primary_color(), Color::rgba(0, 0, 0, 255));
    assert_eq!(
        state.borrow().secondary_color(),
        Color::rgba(255, 255, 255, 255)
    );
    ui.invoke_restore_original_color(); // Secondary restores its baseline, not a swapped value.
    assert_eq!(state.borrow().secondary_color(), original_outline);
    ui.invoke_edit_rgba(10, 40, 220, 255);
    ui.invoke_select_color_slot(0);
    ui.invoke_restore_original_color();
    assert_eq!(state.borrow().primary_color(), fill);
    ui.invoke_select_color_slot(1);
    ui.invoke_select_shape_mode(1);
    let pixels = state.borrow().composite();
    assert_eq!(pixels[80 * 512 + 80].a, 0);
    assert_eq!(pixels[40 * 512 + 40], fill);
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
