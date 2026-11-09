use hoshi_canvas_spike::{ShapePropertiesWindow, create_app_with_shape_properties};
use slint::platform::{Key, PointerEventButton, WindowEvent};
use slint::{ComponentHandle, LogicalPosition, LogicalSize};

fn type_bound(window: &ShapePropertiesWindow, label: &str, value: &str) {
    type_pending_bound(window, label, value);
    window.window().dispatch_event(WindowEvent::KeyPressed {
        text: Key::Return.into(),
    });
    window.window().dispatch_event(WindowEvent::KeyReleased {
        text: Key::Return.into(),
    });
}
fn type_pending_bound<C: ComponentHandle>(window: &C, label: &str, value: &str) {
    i_slint_backend_testing::ElementHandle::find_by_accessible_label(window, label)
        .next()
        .unwrap()
        .mock_single_click(PointerEventButton::Left);
    let key = |text: slint::SharedString| {
        window
            .window()
            .dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
        window
            .window()
            .dispatch_event(WindowEvent::KeyReleased { text });
    };
    window.window().dispatch_event(WindowEvent::KeyPressed {
        text: Key::Control.into(),
    });
    key("a".into());
    window.window().dispatch_event(WindowEvent::KeyReleased {
        text: Key::Control.into(),
    });
    for character in value.chars() {
        key(character.to_string().into());
    }
}

#[test]
fn floating_and_docked_shape_properties_share_one_edit_session() {
    slint::platform::set_platform(Box::new(i_slint_backend_testing::TestingBackend::new(
        i_slint_backend_testing::TestingBackendOptions {
            mock_time: true,
            threading: false,
            renderer_name: Some("software".into()),
        },
    )))
    .unwrap();
    let (ui, state, properties) = create_app_with_shape_properties().unwrap();
    ui.window().set_size(LogicalSize::new(1120., 760.));
    properties.window().set_size(LogicalSize::new(340., 640.));
    ui.show().unwrap();
    ui.window().take_snapshot().unwrap();
    ui.invoke_pointer_down(40., 50.);
    ui.invoke_pointer_move(200., 130.);
    ui.invoke_pointer_up(200., 130.);
    let original = state.borrow().document().active_shape();
    assert!(!ui.get_shape_properties_visible());
    ui.invoke_show_shape_properties();
    properties.window().take_snapshot().unwrap();
    ui.window()
        .dispatch_event(WindowEvent::WindowActiveChanged(false));
    properties
        .window()
        .dispatch_event(WindowEvent::WindowActiveChanged(true));
    assert_eq!(state.borrow().document().active_shape(), original);
    properties.invoke_select_shape_mode(2);
    properties.invoke_edit_stroke_width_pixels(4.5);
    assert_eq!(ui.get_stroke_width_pixels(), 4.5);
    type_bound(&properties, "Shape X position", "-12");
    assert_eq!(state.borrow().document().active_shape().unwrap().x, -12);
    assert_eq!(
        state.borrow().document().undo_len(),
        0,
        "field Enter must not commit the preview"
    );
    i_slint_backend_testing::ElementHandle::find_by_accessible_label(
        &*properties,
        "Lock shape aspect ratio",
    )
    .next()
    .unwrap()
    .mock_single_click(PointerEventButton::Left);
    assert!(ui.get_shape_bounds_locked());
    type_bound(&properties, "Shape width", "240");
    assert_eq!(
        state.borrow().document().active_shape().unwrap().height,
        120
    );
    // Moving or hiding the host applies the field still being typed, without
    // requiring Enter and without letting an old hidden field revert it later.
    type_pending_bound(&*properties, "Shape width", "300");
    properties.invoke_placement_selected(2);
    assert_eq!(state.borrow().document().active_shape().unwrap().width, 300);
    ui.window().take_snapshot().unwrap();
    type_pending_bound(&ui, "Shape Y position", "88");
    properties.invoke_placement_selected(0);
    assert_eq!(state.borrow().document().active_shape().unwrap().y, 88);
    type_pending_bound(&*properties, "Shape X position", "-24");
    properties.invoke_close_panel();
    assert_eq!(state.borrow().document().active_shape().unwrap().x, -24);
    ui.invoke_show_shape_properties();
    type_pending_bound(&*properties, "Shape width", "320");
    ui.invoke_finish_shape_properties_inputs();
    assert_eq!(state.borrow().document().active_shape().unwrap().width, 320);
    assert!(
        ui.get_shape_properties_visible(),
        "saving can flush input without hiding the panel"
    );
    let preview = state.borrow().document().active_shape();
    for placement in [1, 2, 3, 4, 0] {
        properties.invoke_placement_selected(placement);
        assert_eq!(ui.get_shape_properties_placement(), placement);
        assert_eq!(properties.get_placement(), placement);
        assert_eq!(state.borrow().document().active_shape(), preview);
        assert_eq!(state.borrow().document().undo_len(), 0);
        ui.window().take_snapshot().unwrap();
    }
    // Every existing adjustment window can share each dock edge with Shapes.
    ui.window().set_size(LogicalSize::new(1600., 960.));
    ui.invoke_show_colors();
    ui.invoke_show_history();
    for placement in [1, 2, 3, 4] {
        ui.invoke_set_colors_placement(placement);
        ui.invoke_set_history_placement(placement);
        properties.invoke_placement_selected(placement);
        ui.window().take_snapshot().unwrap();
        let panels = [
            (
                ui.get_colors_layout_x(),
                ui.get_colors_layout_y(),
                ui.get_colors_layout_width(),
                ui.get_colors_layout_height(),
            ),
            (
                ui.get_history_layout_x(),
                ui.get_history_layout_y(),
                ui.get_history_layout_width(),
                ui.get_history_layout_height(),
            ),
            (
                ui.get_shapes_layout_x(),
                ui.get_shapes_layout_y(),
                ui.get_shapes_layout_width(),
                ui.get_shapes_layout_height(),
            ),
        ];
        for (index, &(x, y, width, height)) in panels.iter().enumerate() {
            assert!(width > 0. && height > 0.);
            for &(other_x, other_y, other_width, other_height) in &panels[index + 1..] {
                assert!(
                    x + width <= other_x
                        || other_x + other_width <= x
                        || y + height <= other_y
                        || other_y + other_height <= y,
                    "panels must not overlap at dock placement {placement}"
                );
            }
        }
        if placement == 3 {
            let panel_x =
                ui.get_canvas_left() - ui.get_canvas_layout_x() + ui.get_shapes_layout_x();
            let panel_y = ui.get_canvas_top() - ui.get_canvas_layout_y() + ui.get_shapes_layout_y();
            ui.window().dispatch_event(WindowEvent::PointerScrolled {
                position: LogicalPosition::new(panel_x + 12., panel_y + 100.),
                delta_x: 0.,
                delta_y: -1000.,
            });
            ui.window().take_snapshot().unwrap();
            let pattern_field = i_slint_backend_testing::ElementHandle::find_by_accessible_label(
                &ui,
                "Shape fill pattern",
            )
            .next()
            .unwrap();
            assert!(pattern_field.absolute_position().y >= panel_y + 40.);
            assert!(
                pattern_field.absolute_position().y + pattern_field.size().height
                    <= panel_y + ui.get_shapes_layout_height(),
                "scrolling a short top dock must reveal the rendering controls"
            );
        }
        assert_eq!(state.borrow().document().active_shape(), preview);
        assert_eq!(state.borrow().document().undo_len(), 0);
    }
    ui.invoke_set_colors_placement(0);
    ui.invoke_set_history_placement(0);
    properties.invoke_placement_selected(0);
    ui.invoke_close_colors();
    ui.invoke_close_history();
    properties.invoke_close_panel();
    assert!(!ui.get_shape_properties_visible());
    assert_eq!(state.borrow().document().active_shape(), preview);
    ui.invoke_show_shape_properties();
    assert_eq!(properties.get_shape_width_text(), "320");
    assert!(properties.get_shape_bounds_locked());
    properties.invoke_toggle_shape_antialiasing(false);
    assert!(!ui.get_shape_antialiasing());
    ui.invoke_close_shape_properties();
    ui.hide().unwrap();
}
