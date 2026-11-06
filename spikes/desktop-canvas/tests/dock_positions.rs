use hoshi_canvas_spike::create_app_with_colors;
use slint::{ComponentHandle, LogicalSize};

#[derive(Clone, Copy, Debug)]
struct Bounds {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

fn separated(a: Bounds, b: Bounds) -> bool {
    a.x + a.width <= b.x + 0.5
        || b.x + b.width <= a.x + 0.5
        || a.y + a.height <= b.y + 0.5
        || b.y + b.height <= a.y + 0.5
}

#[test]
fn colors_and_history_can_dock_on_every_edge_without_covering_canvas_or_each_other() {
    slint::platform::set_platform(Box::new(i_slint_backend_testing::TestingBackend::new(
        i_slint_backend_testing::TestingBackendOptions {
            mock_time: true,
            threading: false,
            renderer_name: Some("software".into()),
        },
    )))
    .unwrap();
    let (ui, state, picker) = create_app_with_colors().unwrap();
    ui.window().set_size(LogicalSize::new(1400.0, 1100.0));
    ui.show().unwrap();
    ui.invoke_pointer_down(40.0, 40.0);
    ui.invoke_pointer_move(140.0, 100.0);
    ui.invoke_pointer_up(140.0, 100.0);
    let shape = state.borrow().document().active_shape();
    ui.invoke_show_colors();
    ui.invoke_show_history();
    picker.window().take_snapshot().unwrap();
    let choice = i_slint_backend_testing::ElementHandle::find_by_accessible_label(
        picker.as_ref(),
        "Colors dock location",
    )
    .next()
    .expect("floating panel location control");
    choice.mock_single_click(slint::platform::PointerEventButton::Left);
    picker
        .window()
        .dispatch_event(slint::platform::WindowEvent::KeyPressed {
            text: slint::platform::Key::DownArrow.into(),
        });
    picker
        .window()
        .dispatch_event(slint::platform::WindowEvent::KeyReleased {
            text: slint::platform::Key::DownArrow.into(),
        });
    picker
        .window()
        .dispatch_event(slint::platform::WindowEvent::KeyPressed {
            text: slint::platform::Key::Return.into(),
        });
    picker
        .window()
        .dispatch_event(slint::platform::WindowEvent::KeyReleased {
            text: slint::platform::Key::Return.into(),
        });
    assert_eq!(
        ui.get_colors_placement(),
        1,
        "actual location selector input must dock the panel"
    );
    for placement in 1..=4 {
        ui.invoke_set_colors_placement(placement);
        ui.invoke_set_history_placement(placement);
        ui.window().take_snapshot().unwrap();
        assert_eq!(ui.get_colors_placement(), placement);
        assert_eq!(ui.get_history_placement(), placement);
        assert!(ui.get_colors_docked() && ui.get_colors_visible() && ui.get_history_visible());
        assert!(!picker.window().is_visible());
        let canvas = Bounds {
            x: ui.get_canvas_left(),
            y: ui.get_canvas_top(),
            width: ui.get_canvas_width(),
            height: ui.get_canvas_height(),
        };
        let colors = Bounds {
            x: ui.get_colors_host_left(),
            y: ui.get_colors_host_top(),
            width: ui.get_colors_host_width(),
            height: ui.get_colors_host_height(),
        };
        let history = Bounds {
            x: ui.get_history_host_left(),
            y: ui.get_history_host_top(),
            width: ui.get_history_host_width(),
            height: ui.get_history_host_height(),
        };
        assert!(
            canvas.width >= 239.0 && canvas.height >= 199.0,
            "canvas must remain usable: {canvas:?}"
        );
        assert!(
            separated(canvas, colors),
            "Colors overlaps canvas on edge {placement}: {canvas:?} {colors:?}"
        );
        assert!(
            separated(canvas, history),
            "History overlaps canvas on edge {placement}: {canvas:?} {history:?}"
        );
        assert!(
            separated(colors, history),
            "same-edge panels overlap on edge {placement}: {colors:?} {history:?}"
        );
        match placement {
            1 => assert!(
                colors.x + colors.width <= canvas.x && history.x + history.width <= canvas.x
            ),
            2 => {
                assert!(colors.x >= canvas.x + canvas.width && history.x >= canvas.x + canvas.width)
            }
            3 => assert!(
                colors.y + colors.height <= canvas.y && history.y + history.height <= canvas.y
            ),
            4 => assert!(
                colors.y >= canvas.y + canvas.height && history.y >= canvas.y + canvas.height
            ),
            _ => unreachable!(),
        }
        assert_eq!(state.borrow().document().active_shape(), shape);
        assert_eq!(state.borrow().document().undo_len(), 0);
    }
    // Real header shortcuts return to the selected edge rather than resetting
    // to the initial right-hand default.
    i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(50));
    let float =
        i_slint_backend_testing::ElementHandle::find_by_accessible_label(&ui, "Float Colors")
            .next()
            .expect("docked header Float control");
    float.mock_single_click(slint::platform::PointerEventButton::Left);
    assert_eq!(ui.get_colors_placement(), 0);
    assert!(picker.window().is_visible());
    picker.window().take_snapshot().unwrap();
    i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(50));
    let dock = i_slint_backend_testing::ElementHandle::find_by_accessible_label(
        picker.as_ref(),
        "Dock Colors",
    )
    .next()
    .expect("floating header Dock control");
    dock.mock_single_click(slint::platform::PointerEventButton::Left);
    assert_eq!(
        ui.get_colors_placement(),
        4,
        "Dock must return to the last selected edge"
    );
    // Opposite horizontal bands adapt to a normal-height window rather than
    // demanding an oversized desktop or reducing the canvas to a sliver.
    ui.invoke_set_colors_placement(3);
    ui.invoke_set_history_placement(4);
    ui.window().set_size(LogicalSize::new(1120.0, 760.0));
    ui.window().take_snapshot().unwrap();
    assert!(ui.get_canvas_height() >= 199.0);
    assert!(ui.get_colors_host_height() < 250.0 && ui.get_colors_host_height() > 56.0);
    assert!(ui.get_history_host_height() < 250.0 && ui.get_history_host_height() > 56.0);
    assert_eq!(state.borrow().document().active_shape(), shape);
    // Different edges are independent rather than a single shared dock toggle.
    ui.invoke_set_colors_placement(1);
    ui.invoke_set_history_placement(4);
    ui.window().take_snapshot().unwrap();
    assert_eq!(ui.get_colors_placement(), 1);
    assert_eq!(ui.get_history_placement(), 4);
    ui.invoke_set_colors_placement(0);
    assert!(!ui.get_colors_docked());
    assert!(picker.window().is_visible());
    assert_eq!(ui.get_history_placement(), 4);
    assert_eq!(state.borrow().document().active_shape(), shape);
    ui.invoke_close_colors();
    ui.invoke_close_history();
}
