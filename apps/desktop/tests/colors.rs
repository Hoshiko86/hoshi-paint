use hoshi_controller::Color;
use hoshi_desktop::create_app_with_colors;
use slint::{ComponentHandle, LogicalSize};

#[test]
fn picker_controls_sync_live_color_alpha_and_repeat_edits() {
    slint::platform::set_platform(Box::new(i_slint_backend_testing::TestingBackend::new(
        i_slint_backend_testing::TestingBackendOptions {
            mock_time: true,
            threading: false,
            renderer_name: Some("software".into()),
        },
    )))
    .unwrap();
    let (ui, state, picker) = create_app_with_colors().unwrap();
    ui.window().set_size(LogicalSize::new(1120.0, 760.0));
    ui.show().unwrap();
    // A committed source swatch, separate from the shape being edited.
    picker.invoke_set_hex("#3C87E0".into());
    ui.invoke_pointer_down(10.0, 10.0);
    ui.invoke_pointer_move(20.0, 20.0);
    ui.invoke_pointer_up(20.0, 20.0);
    ui.invoke_action(2);
    ui.window().take_snapshot().unwrap();
    ui.window()
        .dispatch_event(slint::platform::WindowEvent::WindowActiveChanged(true));
    let canvas_point =
        |x, y| slint::LogicalPosition::new(ui.get_canvas_left() + x, ui.get_canvas_top() + y);
    ui.window()
        .dispatch_event(slint::platform::WindowEvent::PointerPressed {
            position: canvas_point(40.0, 40.0),
            button: slint::platform::PointerEventButton::Left,
        });
    ui.window()
        .dispatch_event(slint::platform::WindowEvent::PointerMoved {
            position: canvas_point(100.0, 80.0),
        });
    ui.invoke_show_colors();
    assert!(
        !state.borrow().is_dragging(),
        "opening Colors must settle the gesture without discarding its preview"
    );
    assert!(state.borrow().document().active_shape().is_some());
    picker.window().set_size(LogicalSize::new(640.0, 680.0));
    picker.invoke_set_rgba(12, 34, 56, 128);
    assert_eq!(state.borrow().primary_color(), Color::rgba(12, 34, 56, 128));
    assert_eq!(picker.get_hex_value(), "#0C223880");
    ui.window()
        .dispatch_event(slint::platform::WindowEvent::WindowActiveChanged(false));
    picker
        .window()
        .dispatch_event(slint::platform::WindowEvent::WindowActiveChanged(true));
    let before_click = state.borrow().primary_color();
    picker.window().take_snapshot().unwrap();
    let position = slint::LogicalPosition::new(
        picker.get_sv_left() + picker.get_sv_width() * 0.8,
        picker.get_sv_top() + picker.get_sv_height() * 0.2,
    );
    picker
        .window()
        .dispatch_event(slint::platform::WindowEvent::PointerPressed {
            position,
            button: slint::platform::PointerEventButton::Left,
        });
    picker
        .window()
        .dispatch_event(slint::platform::WindowEvent::PointerReleased {
            position,
            button: slint::platform::PointerEventButton::Left,
        });
    assert_ne!(
        state.borrow().primary_color(),
        before_click,
        "actual SV pointer input must select a color"
    );
    assert_eq!(
        state.borrow().composite()[50 * 512 + 50],
        state.borrow().primary_color(),
        "picker input must update the released active shape while the main window is inactive"
    );
    assert!(picker.get_has_active_shape());
    let selected = state.borrow().primary_color();
    let canvas = ui.window().take_snapshot().unwrap();
    let pixel = canvas.as_slice()[(ui.get_canvas_top() as usize + 50) * canvas.width() as usize
        + ui.get_canvas_left() as usize
        + 50];
    let display = |channel: u8| {
        ((u32::from(channel) * u32::from(selected.a) + 224 * (255 - u32::from(selected.a)) + 127)
            / 255) as u8
    };
    assert_eq!(
        (pixel.r, pixel.g, pixel.b),
        (
            display(selected.r),
            display(selected.g),
            display(selected.b)
        ),
        "inactive main window must render the newly selected color"
    );
    let before_slider = state.borrow().primary_color();
    // Scroll the shared Colors body before reaching a control below the palette.
    picker
        .window()
        .dispatch_event(slint::platform::WindowEvent::PointerScrolled {
            position: slint::LogicalPosition::new(610., 200.),
            delta_x: 0.,
            delta_y: 440. - picker.get_red_slider_top(),
        });
    i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(1000));
    picker.window().take_snapshot().unwrap();
    let position = slint::LogicalPosition::new(
        picker.get_red_slider_left() + picker.get_red_slider_width() * 0.9,
        picker.get_red_slider_top() + picker.get_red_slider_height() * 0.5,
    );
    picker
        .window()
        .dispatch_event(slint::platform::WindowEvent::PointerPressed {
            position,
            button: slint::platform::PointerEventButton::Left,
        });
    picker
        .window()
        .dispatch_event(slint::platform::WindowEvent::PointerReleased {
            position,
            button: slint::platform::PointerEventButton::Left,
        });
    assert_ne!(
        state.borrow().primary_color().r,
        before_slider.r,
        "actual RGB slider input must change the color"
    );
    assert_eq!(
        state.borrow().composite()[50 * 512 + 50],
        state.borrow().primary_color()
    );
    picker.invoke_set_hsv(120.0, 100.0, 100.0);
    assert_eq!(state.borrow().primary_color(), Color::rgba(0, 255, 0, 128));
    assert_eq!(
        (picker.get_red(), picker.get_green(), picker.get_blue()),
        (0, 255, 0)
    );
    assert_eq!(
        state.borrow().composite()[50 * 512 + 50],
        Color::rgba(0, 255, 0, 128)
    );
    assert_eq!(state.borrow().document().undo_len(), 1);
    picker.invoke_set_hsv(240.0, 80.0, 0.0);
    assert_eq!(picker.get_hue(), 240.0);
    assert_eq!(picker.get_saturation(), 80.0);
    picker.invoke_set_hsv(picker.get_hue(), picker.get_saturation(), 100.0);
    assert_eq!(
        state.borrow().primary_color(),
        Color::rgba(51, 51, 255, 128)
    );
    picker.invoke_set_hex("#F008".into());
    assert_eq!(state.borrow().primary_color(), Color::rgba(255, 0, 0, 136));
    let before = state.borrow().primary_color();
    picker.invoke_set_hex("invalid".into());
    assert_eq!(state.borrow().primary_color(), before);
    assert!(!picker.get_error().is_empty());
    picker.invoke_set_hex("#3C87E0".into());
    assert!(picker.get_has_active_shape());
    picker.invoke_set_rgba(12, 34, 56, 255);
    assert_eq!(
        state.borrow().composite()[50 * 512 + 50],
        Color::rgba(12, 34, 56, 255)
    );
    picker.invoke_set_hex("#3C87E0".into());
    assert!(state.borrow().document().active_shape().is_some());
    assert_eq!(state.borrow().document().undo_len(), 1);
    ui.invoke_action(2);
    assert!(!picker.get_has_active_shape());
    assert_eq!(state.borrow().document().undo_len(), 2);
    picker.invoke_set_rgba(0, 0, 0, 255);
    assert_eq!(
        state.borrow().composite()[50 * 512 + 50],
        Color::rgba(60, 135, 224, 255)
    );
    ui.invoke_show_colors();
    let snapshot = picker.window().take_snapshot().unwrap();
    assert_eq!((snapshot.width(), snapshot.height()), (640, 680));
    if let Some(path) = std::env::var_os("HOSHI_COLOR_SNAPSHOT") {
        image::save_buffer(
            path,
            snapshot.as_bytes(),
            snapshot.width(),
            snapshot.height(),
            image::ColorType::Rgba8,
        )
        .unwrap();
    }
    ui.window()
        .dispatch_event(slint::platform::WindowEvent::CloseRequested);
    while ui.get_file_prompt_visible() {
        ui.invoke_file_prompt_response(1);
    }
    assert!(!ui.window().is_visible());
    assert!(
        !picker.window().is_visible(),
        "closing the app must not leave an orphaned color window"
    );
}

fn gradient_app() -> (
    hoshi_desktop::AppWindow,
    hoshi_desktop::SharedController,
    std::rc::Rc<hoshi_desktop::ColorPickerWindow>,
) {
    slint::platform::set_platform(Box::new(i_slint_backend_testing::TestingBackend::new(
        i_slint_backend_testing::TestingBackendOptions {
            mock_time: true,
            threading: false,
            renderer_name: Some("software".into()),
        },
    )))
    .unwrap();
    let (ui, state, picker) = create_app_with_colors().unwrap();
    ui.show().unwrap();
    ui.invoke_show_colors();
    picker.window().set_size(LogicalSize::new(640., 1200.));
    picker.window().take_snapshot().unwrap();
    (ui, state, picker)
}
fn channel_control(
    picker: &hoshi_desktop::ColorPickerWindow,
    label: &str,
    role: i_slint_backend_testing::AccessibleRole,
) -> i_slint_backend_testing::ElementHandle {
    i_slint_backend_testing::ElementHandle::find_by_accessible_label(picker, label)
        .find(|e| e.accessible_role() == Some(role))
        .unwrap()
}
fn gradient_key(picker: &hoshi_desktop::ColorPickerWindow, key: slint::platform::Key) {
    let text: slint::SharedString = key.into();
    picker
        .window()
        .dispatch_event(slint::platform::WindowEvent::KeyPressed { text: text.clone() });
    picker
        .window()
        .dispatch_event(slint::platform::WindowEvent::KeyReleased { text });
}
fn pointer(picker: &hoshi_desktop::ColorPickerWindow, x: f32, y: f32, pressed: bool) {
    let position = slint::LogicalPosition::new(x, y);
    let button = slint::platform::PointerEventButton::Left;
    picker.window().dispatch_event(if pressed {
        slint::platform::WindowEvent::PointerPressed { position, button }
    } else {
        slint::platform::WindowEvent::PointerReleased { position, button }
    });
}

#[test]
fn gradient_red_drag_and_midpoint_preserve_other_rgba_channels() {
    let (ui, state, picker) = gradient_app();
    picker.invoke_set_rgba(12, 34, 56, 128);
    picker.window().take_snapshot().unwrap();
    let (x, y, width) = (
        picker.get_red_slider_left(),
        picker.get_red_slider_top() + picker.get_red_slider_height() / 2.,
        picker.get_red_slider_width(),
    );
    pointer(&picker, x + 0.01, y, true);
    assert_eq!(state.borrow().primary_color(), Color::rgba(0, 34, 56, 128));
    picker
        .window()
        .dispatch_event(slint::platform::WindowEvent::PointerMoved {
            position: slint::LogicalPosition::new(x + width - 0.01, y),
        });
    pointer(&picker, x + width - 0.01, y, false);
    assert_eq!(
        state.borrow().primary_color(),
        Color::rgba(255, 34, 56, 128)
    );
    pointer(&picker, x + width / 2., y, true);
    pointer(&picker, x + width / 2., y, false);
    let color = state.borrow().primary_color();
    assert!((i16::from(color.r) - 128).abs() <= 1);
    assert_eq!((color.g, color.b, color.a), (34, 56, 128));
    assert_eq!(state.borrow().document().undo_len(), 0);
    picker.hide().unwrap();
    ui.hide().unwrap();
}

#[test]
fn gradient_keyboard_and_accessibility_follow_the_red_channel_and_clamp() {
    use i_slint_backend_testing::AccessibleRole;
    use slint::platform::{Key, PointerEventButton};
    let (ui, state, picker) = gradient_app();
    picker.invoke_set_rgba(12, 34, 56, 128);
    let slider = channel_control(&picker, "R", AccessibleRole::Slider);
    assert_eq!(
        slider.accessible_value().unwrap().parse::<f32>().unwrap(),
        12.
    );
    assert_eq!(slider.accessible_value_minimum(), Some(0.));
    assert_eq!(slider.accessible_value_maximum(), Some(255.));
    assert_eq!(slider.accessible_value_step(), Some(1.));
    slider.mock_single_click(PointerEventButton::Left);
    let before = picker.get_red();
    gradient_key(&picker, Key::RightArrow);
    assert_eq!(picker.get_red(), before + 1);
    gradient_key(&picker, Key::PageUp);
    assert_eq!(picker.get_red(), before + 11);
    for (key, expected) in [
        (Key::Home, 0),
        (Key::LeftArrow, 0),
        (Key::End, 255),
        (Key::RightArrow, 255),
        (Key::PageDown, 245),
    ] {
        gradient_key(&picker, key);
        assert_eq!(picker.get_red(), expected);
        assert_eq!(
            slider.accessible_value().unwrap().parse::<f32>().unwrap(),
            expected as f32
        );
    }
    slider.invoke_accessible_decrement_action();
    assert_eq!(picker.get_red(), 244);
    slider.invoke_accessible_increment_action();
    assert_eq!(picker.get_red(), 245);
    assert_eq!(
        state.borrow().primary_color(),
        Color::rgba(245, 34, 56, 128)
    );
    picker.hide().unwrap();
    ui.hide().unwrap();
}

#[test]
fn gradient_hsv_and_alpha_edit_only_their_channel_and_keep_spinboxes_in_sync() {
    use hoshi_desktop::Translations;
    use i_slint_backend_testing::AccessibleRole;
    use slint::platform::{Key, PointerEventButton};
    let (ui, state, picker) = gradient_app();
    let alpha = picker.global::<Translations>().get_alpha();
    for label in ["H", "S", "V", alpha.as_str()] {
        picker.invoke_set_rgba(12, 34, 56, 90);
        picker.invoke_set_hsv(120., 40., 60.);
        let before = (
            picker.get_hue(),
            picker.get_saturation(),
            picker.get_brightness(),
        );
        let rgb = state.borrow().primary_color();
        let slider = channel_control(&picker, label, AccessibleRole::Slider);
        slider.mock_single_click(PointerEventButton::Left);
        gradient_key(&picker, Key::RightArrow);
        let value = slider.accessible_value().unwrap().parse::<f32>().unwrap();
        let spin = channel_control(&picker, label, AccessibleRole::Spinbox);
        assert_eq!(
            spin.accessible_value().unwrap().parse::<f32>().unwrap(),
            value.round()
        );
        match label {
            "H" => {
                assert_eq!(
                    (picker.get_saturation(), picker.get_brightness()),
                    (before.1, before.2)
                );
                assert_eq!(picker.get_hue(), value);
            }
            "S" => {
                assert_eq!(
                    (picker.get_hue(), picker.get_brightness()),
                    (before.0, before.2)
                );
                assert_eq!(picker.get_saturation(), value);
            }
            "V" => {
                assert_eq!(
                    (picker.get_hue(), picker.get_saturation()),
                    (before.0, before.1)
                );
                assert_eq!(picker.get_brightness(), value);
            }
            _ => {
                let color = state.borrow().primary_color();
                assert_eq!((color.r, color.g, color.b), (rgb.r, rgb.g, rgb.b));
                assert_eq!(f32::from(color.a), value);
            }
        }
        if label != alpha.as_str() {
            assert_eq!(state.borrow().primary_color().a, 90);
        }
    }
    assert_eq!(state.borrow().document().undo_len(), 0);
    picker.hide().unwrap();
    ui.hide().unwrap();
}

#[test]
fn gradient_track_reacts_to_other_channels_without_changing_its_own_value() {
    let (ui, _, picker) = gradient_app();
    picker.invoke_set_rgba(12, 34, 56, 128);
    let sample = || {
        let snapshot = picker.window().take_snapshot().unwrap();
        let x = (picker.get_red_slider_left() + picker.get_red_slider_width() * 0.25) as usize;
        let y = (picker.get_red_slider_top() + picker.get_red_slider_height() * 0.5) as usize;
        snapshot.as_slice()[y * snapshot.width() as usize + x]
    };
    let first = sample();
    assert_eq!((first.g, first.b), (34, 56));
    picker.invoke_set_rgba(12, 200, 56, 128);
    let second = sample();
    assert_eq!((second.g, second.b), (200, 56));
    assert_eq!(picker.get_red(), 12);
    picker.hide().unwrap();
    ui.hide().unwrap();
}
