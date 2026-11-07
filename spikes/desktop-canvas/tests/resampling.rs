use hoshi_canvas_spike::create_app;
use hoshi_controller::{Color, Controller, Resampling, ResizeOptions};
use i_slint_backend_testing::ElementHandle;
use slint::platform::{Key, PointerEventButton, WindowEvent};
use slint::{ComponentHandle, LogicalSize};

#[test]
fn every_resize_option_reaches_the_engine_and_gamma_checkbox_changes_real_pixels() {
    slint::platform::set_platform(Box::new(i_slint_backend_testing::TestingBackend::new(
        i_slint_backend_testing::TestingBackendOptions {
            mock_time: true,
            threading: false,
            renderer_name: Some("software".into()),
        },
    )))
    .unwrap();
    let (ui, state) = create_app().unwrap();
    ui.window().set_size(LogicalSize::new(1120., 800.));
    ui.show().unwrap();
    for method in 0..8 {
        for gamma in [true, false] {
            ui.invoke_show_image_settings(0);
            ui.invoke_select_image_aspect(false);
            ui.invoke_edit_image_setting(0, "2".into());
            ui.invoke_edit_image_setting(1, "1".into());
            ui.invoke_apply_image_settings();
            ui.invoke_pointer_down(0., 0.);
            ui.invoke_pointer_move(1., 1.);
            ui.invoke_pointer_up(1., 1.);
            ui.invoke_action(2);
            assert_eq!(
                state.borrow().composite(),
                vec![Color::rgba(0, 0, 0, 255), Color::rgba(255, 255, 255, 255)]
            );
            ui.invoke_show_image_settings(1);
            assert_eq!(ui.get_image_resampling(), 0);
            assert!(ui.get_image_gamma_correction());
            ui.invoke_select_image_aspect(false);
            ui.invoke_edit_image_setting(0, "1".into());
            ui.window().take_snapshot().unwrap();
            if method == 6 && !gamma {
                // Exercise the actual dropdown and checkbox rather than only property setters.
                let choice = ElementHandle::find_by_accessible_label(&ui, "Resize resampling")
                    .next()
                    .unwrap();
                choice.mock_single_click(PointerEventButton::Left);
                for _ in 0..6 {
                    ui.window().dispatch_event(WindowEvent::KeyPressed {
                        text: Key::DownArrow.into(),
                    });
                    ui.window().dispatch_event(WindowEvent::KeyReleased {
                        text: Key::DownArrow.into(),
                    });
                }
                ui.window().dispatch_event(WindowEvent::KeyPressed {
                    text: Key::Return.into(),
                });
                ui.window().dispatch_event(WindowEvent::KeyReleased {
                    text: Key::Return.into(),
                });
                assert_eq!(ui.get_image_resampling(), 6);
                let checkbox = ElementHandle::find_by_accessible_label(&ui, "Use gamma correction")
                    .next()
                    .unwrap();
                checkbox.mock_single_click(PointerEventButton::Left);
                assert!(!ui.get_image_gamma_correction());
            } else {
                ui.set_image_resampling(method);
                ui.set_image_gamma_correction(gamma);
            }
            ui.invoke_apply_image_settings();
            assert_eq!(ui.get_image_dialog_kind(), -1);
            let value = if method == 7 {
                255
            } else if gamma {
                188
            } else {
                128
            };
            assert_eq!(
                state.borrow().composite(),
                vec![Color::rgba(value, value, value, 255)],
                "method={method}, gamma={gamma}"
            );
            assert_eq!(state.borrow().document().undo_len(), 2);
            ui.invoke_action(0);
            assert_eq!(state.borrow().document().width(), 2);
            ui.invoke_action(1);
            assert_eq!(state.borrow().document().width(), 1);
        }
    }
    // An asymmetric fixture distinguishes the adapter's kernel mappings, not
    // just gamma behavior on the symmetric black/white average above.
    for (index, method) in [
        Resampling::Bicubic,
        Resampling::BicubicSmooth,
        Resampling::Bilinear,
        Resampling::BilinearLowQuality,
        Resampling::AdaptiveSharp,
        Resampling::Lanczos,
        Resampling::Fant,
        Resampling::Nearest,
    ]
    .into_iter()
    .enumerate()
    {
        let mut reference = Controller::new(8, 1).unwrap();
        reference
            .new_image(8, 1, 96., Color::rgba(255, 255, 255, 255))
            .unwrap();
        ui.invoke_show_image_settings(0);
        ui.invoke_select_image_aspect(false);
        ui.invoke_edit_image_setting(0, "8".into());
        ui.invoke_edit_image_setting(1, "1".into());
        ui.invoke_apply_image_settings();
        for x in [1., 4.] {
            ui.invoke_pointer_down(x, 0.);
            ui.invoke_pointer_move(x + 1., 1.);
            ui.invoke_pointer_up(x + 1., 1.);
            ui.invoke_action(2);
            reference.pointer_down(x.into(), 0.).unwrap();
            reference.pointer_move((x + 1.).into(), 1.).unwrap();
            reference.pointer_up().unwrap();
            reference.commit().unwrap();
        }
        reference
            .resize_image(
                3,
                1,
                96.,
                ResizeOptions {
                    method,
                    gamma_correction: true,
                },
            )
            .unwrap();
        ui.invoke_show_image_settings(1);
        ui.invoke_select_image_aspect(false);
        ui.invoke_edit_image_setting(0, "3".into());
        ui.set_image_resampling(index as i32);
        ui.invoke_apply_image_settings();
        assert_eq!(
            state.borrow().composite(),
            reference.composite(),
            "mapping for {method:?}"
        );
    }
    ui.invoke_show_image_settings(1);
    ui.set_image_resampling(6);
    ui.set_image_gamma_correction(false);
    ui.window().take_snapshot().unwrap();
    let reset = ElementHandle::find_by_accessible_label(&ui, "Reset resampling to Bicubic")
        .next()
        .unwrap();
    reset.mock_single_click(PointerEventButton::Left);
    assert_eq!(ui.get_image_resampling(), 0);
    assert!(!ui.get_image_gamma_correction());
    ui.invoke_close_image_settings();
    ui.hide().unwrap();
}
