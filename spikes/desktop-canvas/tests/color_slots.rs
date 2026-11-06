use hoshi_canvas_spike::create_app_with_colors;
use hoshi_controller::{Color, ColorSlot};
use slint::platform::{PointerEventButton, WindowEvent};
use slint::{ComponentHandle, LogicalPosition, LogicalSize};

#[test]
fn workspace_shortcuts_preserve_slot_selection_and_do_not_intercept_text_input() {
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
    ui.window().take_snapshot().unwrap();
    let click = |position| {
        ui.window().dispatch_event(WindowEvent::PointerPressed {
            position,
            button: PointerEventButton::Left,
        });
        ui.window().dispatch_event(WindowEvent::PointerReleased {
            position,
            button: PointerEventButton::Left,
        });
    };
    let key = |text: &str| {
        ui.window()
            .dispatch_event(WindowEvent::KeyPressed { text: text.into() });
        ui.window()
            .dispatch_event(WindowEvent::KeyReleased { text: text.into() });
    };
    picker.invoke_set_rgba(220, 30, 40, 128);
    ui.invoke_select_color_slot(1);
    picker.invoke_set_rgba(20, 70, 240, 200);
    let primary = state.borrow().primary_color();
    let secondary = state.borrow().secondary_color();
    click(LogicalPosition::new(
        ui.get_canvas_left() + 300.,
        ui.get_canvas_top() + 300.,
    ));
    key("x");
    assert_eq!(state.borrow().primary_color(), secondary);
    assert_eq!(state.borrow().secondary_color(), primary);
    assert_eq!(state.borrow().color_slot(), ColorSlot::Secondary);
    key("d");
    assert_eq!(state.borrow().primary_color(), Color::rgba(0, 0, 0, 255));
    assert_eq!(
        state.borrow().secondary_color(),
        Color::rgba(255, 255, 255, 255)
    );
    click(LogicalPosition::new(
        ui.get_layer_input_left() + 50.,
        ui.get_layer_input_top() + 12.,
    ));
    let text = ui.get_layer_edit_text();
    key("x");
    key("d");
    assert_ne!(ui.get_layer_edit_text(), text);
    assert_eq!(state.borrow().primary_color(), Color::rgba(0, 0, 0, 255));
    assert_eq!(
        state.borrow().secondary_color(),
        Color::rgba(255, 255, 255, 255)
    );
    ui.hide().unwrap();
}
