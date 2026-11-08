use hoshi_canvas_spike::create_app;
use hoshi_controller::Color;
use slint::platform::{Key, PointerEventButton, WindowEvent};
use slint::{ComponentHandle, LogicalPosition, LogicalSize};

#[test]
fn image_dialogs_validate_cancel_and_apply_with_real_shortcuts_and_dynamic_canvas_bounds() {
    slint::platform::set_platform(Box::new(i_slint_backend_testing::TestingBackend::new(
        i_slint_backend_testing::TestingBackendOptions {
            mock_time: true,
            threading: false,
            renderer_name: Some("software".into()),
        },
    )))
    .unwrap();
    let (ui, state) = create_app().unwrap();
    ui.window().set_size(LogicalSize::new(1120., 760.));
    ui.show().unwrap();
    ui.invoke_show_image_settings(0);
    ui.invoke_select_image_aspect(false);
    ui.invoke_edit_image_setting(0, "96".into());
    ui.invoke_edit_image_setting(1, "64".into());
    ui.invoke_edit_image_setting(2, "144".into());
    ui.set_image_background(1);
    ui.invoke_apply_image_settings();
    assert_eq!(ui.get_image_dialog_kind(), -1);
    assert_eq!((ui.get_image_width(), ui.get_image_height()), (96, 64));
    assert_eq!(state.borrow().document().resolution(), 144.);
    assert_eq!(
        (
            ui.get_canvas_image().size().width,
            ui.get_canvas_image().size().height
        ),
        (96, 64)
    );
    ui.window().take_snapshot().unwrap();
    let point = |x, y| LogicalPosition::new(ui.get_canvas_left() + x, ui.get_canvas_top() + y);
    ui.window().dispatch_event(WindowEvent::PointerPressed {
        position: point(10., 10.),
        button: PointerEventButton::Left,
    });
    ui.window().dispatch_event(WindowEvent::PointerMoved {
        position: point(30., 20.),
    });
    ui.window().dispatch_event(WindowEvent::PointerReleased {
        position: point(30., 20.),
        button: PointerEventButton::Left,
    });
    let shape = state.borrow().document().active_shape();
    // The actual canvas shortcut must open Resize and move focus into the dialog.
    ui.window().dispatch_event(WindowEvent::KeyPressed {
        text: Key::Control.into(),
    });
    ui.window()
        .dispatch_event(WindowEvent::KeyPressed { text: "r".into() });
    ui.window()
        .dispatch_event(WindowEvent::KeyReleased { text: "r".into() });
    ui.window().dispatch_event(WindowEvent::KeyReleased {
        text: Key::Control.into(),
    });
    assert_eq!(ui.get_image_dialog_kind(), 1);
    ui.window().take_snapshot().unwrap();
    ui.window().dispatch_event(WindowEvent::KeyPressed {
        text: Key::Escape.into(),
    });
    ui.window().dispatch_event(WindowEvent::KeyReleased {
        text: Key::Escape.into(),
    });
    assert_eq!(ui.get_image_dialog_kind(), -1);
    assert_eq!(state.borrow().document().active_shape(), shape);
    assert_eq!(state.borrow().document().undo_len(), 0);
    ui.invoke_show_image_settings(1);
    ui.invoke_edit_image_setting(0, "99999999".into());
    ui.invoke_apply_image_settings();
    assert_eq!(ui.get_image_dialog_kind(), 1);
    assert!(!ui.get_image_error().is_empty());
    assert_eq!(state.borrow().document().active_shape(), shape);
    assert_eq!(state.borrow().document().undo_len(), 0);
    ui.invoke_edit_image_setting(0, "48".into());
    assert_eq!(ui.get_image_height_text(), "32");
    ui.invoke_edit_image_setting(2, "300".into());
    ui.invoke_apply_image_settings();
    assert_eq!(ui.get_image_dialog_kind(), -1);
    assert_eq!((ui.get_image_width(), ui.get_image_height()), (48, 32));
    assert_eq!(state.borrow().document().undo_len(), 2);
    assert_eq!(state.borrow().document().resolution(), 300.);
    ui.invoke_action(0);
    assert_eq!((ui.get_image_width(), ui.get_image_height()), (96, 64));
    assert_eq!(state.borrow().document().resolution(), 144.);
    ui.invoke_action(1);
    ui.invoke_image_command(2);
    assert_eq!((ui.get_image_width(), ui.get_image_height()), (32, 48));
    ui.window().dispatch_event(WindowEvent::KeyPressed {
        text: Key::Control.into(),
    });
    ui.window().dispatch_event(WindowEvent::KeyPressed {
        text: Key::Shift.into(),
    });
    ui.window()
        .dispatch_event(WindowEvent::KeyPressed { text: "R".into() });
    ui.window()
        .dispatch_event(WindowEvent::KeyReleased { text: "R".into() });
    ui.window().dispatch_event(WindowEvent::KeyReleased {
        text: Key::Shift.into(),
    });
    ui.window().dispatch_event(WindowEvent::KeyReleased {
        text: Key::Control.into(),
    });
    assert_eq!(ui.get_image_dialog_kind(), 2);

    ui.invoke_edit_image_setting(0, "64".into());
    ui.invoke_edit_image_setting(1, "64".into());
    ui.set_image_anchor(8);
    ui.invoke_apply_image_settings();
    assert_eq!((ui.get_image_width(), ui.get_image_height()), (64, 64));
    let snapshot = ui.window().take_snapshot().unwrap();
    let x = ui.get_canvas_left() as usize + 70;
    let y = ui.get_canvas_top() as usize + 30;
    let pixel = snapshot.as_slice()[y * snapshot.width() as usize + x];
    assert_eq!(
        (pixel.r, pixel.g, pixel.b),
        (220, 226, 235),
        "image display must use the new dimensions"
    );
    // Unit switching and percentage previews never mutate the image before Apply.
    ui.invoke_show_image_settings(1);
    ui.invoke_select_image_units(1);
    ui.invoke_edit_image_setting(0, "0.5".into());
    assert!(ui.get_image_summary().starts_with("150 × 150 pixels"));
    ui.invoke_close_image_settings();
    assert_eq!(state.borrow().document().width(), 64);
    ui.invoke_show_image_settings(1);
    ui.invoke_select_image_percentage(true);
    ui.invoke_edit_image_setting(3, "50".into());
    ui.invoke_apply_image_settings();
    assert_eq!((ui.get_image_width(), ui.get_image_height()), (32, 32));
    assert_eq!(state.borrow().primary_color(), Color::rgba(0, 0, 0, 255));
    // Physical-unit formatting must round-trip tiny sizes at high resolutions.
    ui.invoke_show_image_settings(0);
    ui.invoke_edit_image_setting(0, "1".into());
    ui.invoke_edit_image_setting(2, "100000".into());
    ui.invoke_select_image_units(1);
    assert_eq!(ui.get_image_width_text(), "0.00001");
    ui.invoke_apply_image_settings();
    assert_eq!((ui.get_image_width(), ui.get_image_height()), (1, 1));
    assert_eq!(state.borrow().document().resolution(), 100000.);
    assert_eq!(state.borrow().document().undo_len(), 0);
    ui.hide().unwrap();
}
