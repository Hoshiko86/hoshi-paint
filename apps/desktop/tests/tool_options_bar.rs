use hoshi_controller::{
    AfterClick, FillPattern, LineStyle, PickerSampling, SampleSize, ShapeMode, Tool,
};
use hoshi_core::{HistoryLabel, ShapeKind};
use hoshi_desktop::{
    AppWindow, PickerAfterClickChoice, PickerSamplingChoice, PickerSizeChoice, ShapeKindChoice,
    SharedController, Translations, create_app_with_colors, refresh_workspace,
};
use i_slint_backend_testing::{AccessibleRole, ElementHandle};
use slint::platform::{Key, PointerEventButton, WindowEvent};
use slint::{ComponentHandle, LogicalPosition, LogicalSize};
use std::{cell::Cell, rc::Rc};

fn app() -> (
    AppWindow,
    SharedController,
    Rc<hoshi_desktop::ColorPickerWindow>,
) {
    slint::platform::set_platform(Box::new(i_slint_backend_testing::TestingBackend::new(
        i_slint_backend_testing::TestingBackendOptions {
            mock_time: true,
            threading: false,
            renderer_name: Some("software".into()),
        },
    )))
    .unwrap();
    let (ui, state, colors) = create_app_with_colors().unwrap();
    ui.window().set_size(LogicalSize::new(1120., 850.));
    ui.show().unwrap();
    ui.window().take_snapshot().unwrap();
    (ui, state, colors)
}
fn field(ui: &AppWindow, label: &str) -> Option<ElementHandle> {
    ElementHandle::find_by_accessible_label(ui, label)
        .find(|element| element.accessible_value().is_some())
}
fn key(ui: &AppWindow, text: slint::SharedString) {
    ui.window()
        .dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
    ui.window()
        .dispatch_event(WindowEvent::KeyReleased { text });
}
fn choose(ui: &AppWindow, label: &str, steps: usize) {
    field(ui, label)
        .unwrap()
        .mock_single_click(PointerEventButton::Left);
    for _ in 0..steps {
        key(ui, Key::DownArrow.into());
    }
    key(ui, Key::Return.into());
}
fn point(ui: &AppWindow, x: f32, y: f32) -> (f32, f32) {
    (
        ui.get_offset_x() + x * ui.get_zoom(),
        ui.get_offset_y() + y * ui.get_zoom(),
    )
}

#[test]
fn sections_follow_the_tool_without_moving_the_canvas() {
    let (ui, _, _) = app();
    let top = ui.get_canvas_top();
    let labels = ui.global::<Translations>();
    for (tool, expected) in [
        ("paintbrush", [false, true, false]),
        ("eraser", [false, true, false]),
        ("shapes", [true, false, false]),
        ("color-picker", [false, false, true]),
    ] {
        ui.invoke_choose_tool(tool.into());
        ui.window().take_snapshot().unwrap();
        assert_eq!(
            [
                field(&ui, &labels.get_shape_kind()).is_some(),
                field(&ui, &labels.get_brush_width()).is_some(),
                field(&ui, &labels.get_picker_sample_size()).is_some(),
            ],
            expected,
            "{tool}"
        );
        assert_eq!(ui.get_canvas_top(), top, "{tool}");
    }
    ui.hide().unwrap();
}

#[test]
fn brush_and_eraser_share_validated_size_and_antialiasing() {
    let (ui, state, _) = app();
    ui.invoke_choose_tool("paintbrush".into());
    ui.window().take_snapshot().unwrap();
    field(&ui, &ui.global::<Translations>().get_brush_width())
        .unwrap()
        .mock_single_click(PointerEventButton::Left);
    ui.window().dispatch_event(WindowEvent::KeyPressed {
        text: Key::Control.into(),
    });
    key(&ui, "a".into());
    ui.window().dispatch_event(WindowEvent::KeyReleased {
        text: Key::Control.into(),
    });
    key(&ui, Key::Backspace.into());
    for c in "4.5".chars() {
        key(&ui, c.to_string().into());
    }
    key(&ui, Key::Return.into());
    assert_eq!(state.borrow().brush_width(), 4.5);
    for bad in [0., 2001.] {
        ui.invoke_edit_brush_width(bad);
        assert_eq!(state.borrow().brush_width(), 4.5);
    }
    let antialias = ElementHandle::find_by_accessible_label(
        &ui,
        &ui.global::<Translations>().get_antialiasing(),
    )
    .find(|e| e.accessible_checked().is_some())
    .unwrap();
    antialias.mock_single_click(PointerEventButton::Left);
    assert!(!state.borrow().antialiasing());
    ui.invoke_choose_tool("eraser".into());
    ui.window().take_snapshot().unwrap();
    assert_eq!(
        field(&ui, &ui.global::<Translations>().get_brush_width())
            .unwrap()
            .accessible_value()
            .as_deref(),
        Some("4.5")
    );
    assert_eq!(state.borrow().document().undo_len(), 0);
    ui.hide().unwrap();
}

#[test]
fn shape_options_draw_and_finish_an_ellipse_with_one_history_entry() {
    let (ui, state, _) = app();
    choose(&ui, &ui.global::<Translations>().get_shape_kind(), 1);
    assert_eq!(state.borrow().shape_kind(), ShapeKind::Ellipse);
    ui.invoke_select_shape_mode(2);
    ui.invoke_edit_stroke_width_pixels(6.5);
    ui.invoke_select_line_style(3);
    ui.invoke_select_fill_pattern(7);
    ui.invoke_toggle_shape_antialiasing(false);
    assert_eq!(state.borrow().shape_mode(), ShapeMode::Both);
    assert_eq!(state.borrow().stroke_width_pixels(), 6.5);
    assert_eq!(state.borrow().line_style(), LineStyle::DashDot);
    assert_eq!(state.borrow().fill_pattern(), FillPattern::Checkerboard);
    assert!(!state.borrow().antialiasing());
    let start = point(&ui, 40., 40.);
    let end = point(&ui, 140., 120.);
    ui.invoke_pointer_down(start.0, start.1);
    ui.invoke_pointer_move(end.0, end.1);
    ui.invoke_pointer_up(end.0, end.1);
    assert_eq!(
        state.borrow().document().active_shape().unwrap().kind,
        ShapeKind::Ellipse
    );
    assert_eq!(state.borrow().document().undo_len(), 0);
    assert!(ui.get_has_shape());
    let finish = ElementHandle::find_by_accessible_label(
        &ui,
        &ui.global::<Translations>().get_finish_shape(),
    )
    .find(|e| e.accessible_role() == Some(AccessibleRole::Button))
    .unwrap();
    finish.mock_single_click(PointerEventButton::Left);
    assert_eq!(state.borrow().document().undo_len(), 1);
    assert!(!ui.get_has_shape());
    assert_eq!(
        state
            .borrow()
            .document()
            .history_steps()
            .last()
            .unwrap()
            .label,
        HistoryLabel::Ellipse
    );
    ui.invoke_select_shape_kind(ShapeKindChoice::Rectangle);
    assert_eq!(state.borrow().shape_kind(), ShapeKind::Rectangle);
    ui.hide().unwrap();
}

#[test]
fn screen_is_a_session_choice_with_image_canvas_sampling_and_one_entry_point() {
    let (ui, state, _) = app();
    ui.invoke_choose_tool("color-picker".into());
    ui.window().take_snapshot().unwrap();
    choose(&ui, &ui.global::<Translations>().get_picker_sampling(), 2);
    assert_eq!(
        state.borrow().settings().picker.sampling,
        PickerSampling::Image
    );
    assert!(ui.get_picker_screen_sampling());
    refresh_workspace(&ui, &state);
    assert_eq!(ui.get_picker_sampling(), PickerSamplingChoice::Screen);
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    ui.on_pick_screen(move || observed.set(observed.get() + 1));
    ui.window().take_snapshot().unwrap();
    ElementHandle::find_by_accessible_label(
        &ui,
        &ui.global::<Translations>().get_pick_from_screen(),
    )
    .find(|e| e.accessible_role() == Some(AccessibleRole::Button))
    .unwrap()
    .mock_single_click(PointerEventButton::Left);
    assert_eq!(calls.get(), 1);
    for (choice, expected) in [
        (PickerSamplingChoice::Layer, PickerSampling::Layer),
        (PickerSamplingChoice::Image, PickerSampling::Image),
    ] {
        ui.invoke_select_picker_sampling(choice);
        assert_eq!(state.borrow().settings().picker.sampling, expected);
        assert_eq!(ui.get_picker_sampling(), choice);
        assert!(!ui.get_picker_screen_sampling());
    }
    assert_eq!(state.borrow().document().undo_len(), 0);
    ui.hide().unwrap();
}

#[test]
fn every_picker_size_and_after_click_choice_preserves_other_settings_and_history() {
    let (ui, state, _) = app();
    ui.invoke_choose_tool("color-picker".into());
    for (choice, size) in [
        (PickerSizeChoice::Square3, SampleSize::Square3),
        (PickerSizeChoice::Square5, SampleSize::Square5),
        (PickerSizeChoice::Square11, SampleSize::Square11),
        (PickerSizeChoice::Square31, SampleSize::Square31),
        (PickerSizeChoice::Square51, SampleSize::Square51),
        (PickerSizeChoice::Single, SampleSize::Single),
    ] {
        let mut expected = state.borrow().settings();
        expected.picker.size = size;
        ui.invoke_select_picker_size(choice);
        assert_eq!(state.borrow().settings(), expected);
        assert_eq!(ui.get_picker_size(), choice);
    }
    for (choice, after_click) in [
        (
            PickerAfterClickChoice::PreviousTool,
            AfterClick::PreviousTool,
        ),
        (PickerAfterClickChoice::DoNotSwitch, AfterClick::DoNotSwitch),
    ] {
        let mut expected = state.borrow().settings();
        expected.picker.after_click = after_click;
        ui.invoke_select_picker_after_click(choice);
        assert_eq!(state.borrow().settings(), expected);
        assert_eq!(ui.get_picker_after_click(), choice);
    }
    assert_eq!(state.borrow().document().undo_len(), 0);
    ui.hide().unwrap();
}

#[test]
fn previous_tool_restores_brush_after_a_real_canvas_pick() {
    let (ui, state, _) = app();
    ui.invoke_select_picker_after_click(PickerAfterClickChoice::PreviousTool);
    ui.invoke_choose_tool("paintbrush".into());
    ui.invoke_choose_tool("color-picker".into());
    let p = point(&ui, 30., 30.);
    let position = LogicalPosition::new(ui.get_canvas_left() + p.0, ui.get_canvas_top() + p.1);
    ui.window().dispatch_event(WindowEvent::PointerPressed {
        position,
        button: PointerEventButton::Left,
    });
    ui.window().dispatch_event(WindowEvent::PointerReleased {
        position,
        button: PointerEventButton::Left,
    });
    assert_eq!(state.borrow().tool(), Tool::Brush);
    assert_eq!(state.borrow().document().undo_len(), 0);
    ui.hide().unwrap();
}

#[test]
fn obsolete_controls_are_absent_and_the_colors_rail_button_still_opens_colors() {
    let (ui, _, colors) = app();
    let text = ui.global::<Translations>();
    assert_eq!(
        ElementHandle::find_by_accessible_label(&ui, &text.get_tool()).count(),
        0
    );
    ui.invoke_choose_tool("paintbrush".into());
    ui.window().take_snapshot().unwrap();
    let rails = ElementHandle::find_by_element_type_name(&ui, "DrawingTools").collect::<Vec<_>>();
    assert!(!rails.is_empty());
    for rail in rails {
        let label = text.get_brush_width();
        assert!(
            rail.query_descendants()
                .match_predicate(move |e| e.accessible_label() == Some(label.clone()))
                .find_first()
                .is_none()
        );
    }
    assert_eq!(
        ElementHandle::find_by_accessible_label(&ui, &text.get_screen_picker()).count(),
        0
    );
    ElementHandle::find_by_accessible_label(&ui, &text.get_colors())
        .next()
        .unwrap()
        .mock_single_click(PointerEventButton::Left);
    assert!(ui.get_colors_visible());
    assert_eq!(
        ElementHandle::find_by_accessible_label(colors.as_ref(), &text.get_screen_picker()).count(),
        0
    );
    ui.hide().unwrap();
}
