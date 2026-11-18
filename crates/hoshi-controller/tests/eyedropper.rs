//! Canvas Color Picker (`Tool::Eyedropper`) sampling contract; see
//! `docs/V0_0_0.md`, "Color Picker and Screen Picker".

use hoshi_controller::{
    AfterClick, Color, ColorSlot, Controller, InputModifiers, PickerSampling, PickerSettings,
    PointerButton, SampleSize, Tool,
};
use hoshi_core::{Document, PixelStore, Shape, ShapeKind, ShapeStyle};

const WHITE: Color = Color::rgba(255, 255, 255, 255);
const RED: Color = Color::rgba(255, 0, 0, 255);
const BLUE: Color = Color::rgba(0, 0, 255, 255);
const TRANSLUCENT: Color = Color::rgba(10, 200, 30, 90);
const PRIMARY: Color = Color::rgba(1, 2, 3, 255);
const SECONDARY: Color = Color::rgba(4, 5, 6, 255);
/// BLUE at layer opacity 128 source-over opaque WHITE, rounded to nearest.
const HALF_BLUE_ON_WHITE: Color = Color::rgba(127, 127, 255, 255);

fn fill(document: &mut Document, layer: usize, x: i32, width: u32, color: Color) {
    document.select_layer(layer).unwrap();
    let shape = Shape::new(
        ShapeKind::Rectangle,
        x,
        0,
        width,
        4,
        ShapeStyle::Fill(color),
    );
    document.begin_shape(shape).unwrap();
    assert!(document.commit_shape().unwrap());
}

/// 8x4 canvas, columns (bottom to top):
/// 0..2 white background under a hidden red layer;
/// 2..4 white background under a blue layer at 50% opacity;
/// 4..6 a translucent layer over transparency; 6..8 fully transparent.
fn layered_document() -> Document {
    let mut document = Document::new(8, 4).unwrap();
    fill(&mut document, 0, 0, 4, WHITE);
    let hidden = document.add_layer("Hidden").unwrap();
    fill(&mut document, hidden, 0, 2, RED);
    document.set_layer_properties(hidden, false, 255).unwrap();
    let half = document.add_layer("Half").unwrap();
    fill(&mut document, half, 2, 2, BLUE);
    document.set_layer_properties(half, true, 128).unwrap();
    let alpha = document.add_layer("Alpha").unwrap();
    fill(&mut document, alpha, 4, 2, TRANSLUCENT);
    document
}

fn picker(document: Document) -> Controller {
    configured_picker(document, PickerSettings::default())
}

fn configured_picker(document: Document, settings: PickerSettings) -> Controller {
    let mut c = Controller::new(8, 4).unwrap();
    configure(&mut c, settings);
    // Set slots before installing the document so an active preview keeps its style.
    c.set_primary_color(PRIMARY).unwrap();
    c.set_secondary_color(SECONDARY).unwrap();
    c.replace_document(document);
    c.set_tool(Tool::Eyedropper).unwrap();
    c
}

/// Click at a document coordinate, converted through the current viewport.
fn click(c: &mut Controller, x: f64, y: f64) {
    click_button(c, x, y, PointerButton::Primary);
}

fn click_button(c: &mut Controller, x: f64, y: f64, button: PointerButton) {
    let p = c.viewport().to_logical(x, y);
    c.pointer_down_with_button(p.0, p.1, button).unwrap();
    c.pointer_up().unwrap();
}

fn configure(c: &mut Controller, picker: PickerSettings) {
    let history = c.document().undo_len();
    c.set_settings(hoshi_controller::ToolSettings {
        picker,
        ..c.settings()
    })
    .unwrap();
    assert_eq!(c.document().undo_len(), history);
}

fn image(c: &mut Controller) {
    configure(
        c,
        PickerSettings {
            sampling: PickerSampling::Image,
            ..Default::default()
        },
    );
}

#[test]
fn samples_visible_composite_with_layer_opacity_and_alpha() {
    let mut c = picker(layered_document());
    image(&mut c);
    let history = c.document().undo_len();
    c.zoom_at(0., 0., 3.).unwrap();
    c.pan(11., -7.).unwrap();

    for (x, y, expected) in [
        (1.5, 1.5, WHITE),               // hidden red layer excluded
        (3.99, 0.2, HALF_BLUE_ON_WHITE), // floor(3.99) = 3: 50% layer blended
        (4.0, 3.99, TRANSLUCENT),        // straight alpha preserved
        (7.5, 2.5, Color::TRANSPARENT),  // empty pixel samples (0,0,0,0)
    ] {
        click(&mut c, x, y);
        assert_eq!(c.primary_color(), expected, "at ({x}, {y})");
        assert_eq!(
            c.primary_color(),
            c.document()
                .composite_pixel(x.floor() as u32, y.floor() as u32, true)
        );
        assert_eq!(c.secondary_color(), SECONDARY);
    }
    assert_eq!(Color::TRANSPARENT, Color::rgba(0, 0, 0, 0));
    assert_eq!(c.document().undo_len(), history);
}

#[test]
fn secondary_slot_receives_the_sample() {
    let mut c = picker(layered_document());
    image(&mut c);
    let history = c.document().undo_len();
    c.set_color_slot(ColorSlot::Secondary);
    click_button(&mut c, 2.5, 1., PointerButton::Secondary);
    assert_eq!(c.secondary_color(), HALF_BLUE_ON_WHITE);
    assert_eq!(c.primary_color(), PRIMARY);
    assert_eq!(c.color_slot(), ColorSlot::Secondary);
    assert_eq!(c.document().undo_len(), history);
}

#[test]
fn outside_canvas_is_ignored_for_clicks_and_drags() {
    let mut c = picker(layered_document());
    let history = c.document().undo_len();
    for (x, y) in [(-0.01, 1.), (8., 1.), (1., -0.5), (1., 4.), (100., 100.)] {
        click(&mut c, x, y);
        assert_eq!(
            (c.primary_color(), c.secondary_color()),
            (PRIMARY, SECONDARY)
        );
    }
    // A drag that leaves the canvas keeps the last in-canvas sample.
    let inside = c.viewport().to_logical(4.5, 1.);
    c.pointer_down(inside.0, inside.1).unwrap();
    assert_eq!(c.primary_color(), TRANSLUCENT);
    let outside = c.viewport().to_logical(9., 1.);
    c.pointer_move(outside.0, outside.1).unwrap();
    assert_eq!(c.primary_color(), TRANSLUCENT);
    c.pointer_up().unwrap();
    assert_eq!(c.secondary_color(), SECONDARY);
    assert_eq!(c.document().undo_len(), history);
}

#[test]
fn samples_uncommitted_shape_preview_without_history() {
    let mut document = layered_document();
    let history = document.undo_len();
    let preview = Color::rgba(200, 100, 50, 255);
    let shape = Shape::new(ShapeKind::Rectangle, 6, 0, 2, 4, ShapeStyle::Fill(preview));
    document.begin_shape(shape).unwrap();
    let mut c = configured_picker(
        document,
        PickerSettings {
            sampling: PickerSampling::Image,
            ..Default::default()
        },
    );
    c.set_color_slot(ColorSlot::Secondary);
    click_button(&mut c, 6.5, 2., PointerButton::Secondary);
    assert_eq!(c.secondary_color(), preview);
    assert!(c.document().active_shape().is_some());
    assert_eq!(c.document().undo_len(), history);
}

#[test]
fn default_sampling_reads_hidden_layer_pixels_without_visibility_or_opacity() {
    let mut document = layered_document();
    document.select_layer(1).unwrap();
    let mut c = picker(document);
    let history = c.document().undo_len();
    click(&mut c, 1., 1.);
    assert_eq!(c.primary_color(), RED);
    assert_eq!(c.document().undo_len(), history);
}

#[test]
fn mouse_button_writes_its_slot_without_changing_the_selected_slot() {
    for selected in [ColorSlot::Primary, ColorSlot::Secondary] {
        for button in [PointerButton::Primary, PointerButton::Secondary] {
            let mut c = picker(layered_document());
            c.set_color_slot(selected);
            let history = c.document().undo_len();
            click_button(&mut c, 4.5, 1., button);
            assert_eq!(c.color_slot(), selected);
            match button {
                PointerButton::Primary => {
                    assert_eq!(c.primary_color(), TRANSLUCENT);
                    assert_eq!(c.secondary_color(), SECONDARY);
                }
                PointerButton::Secondary => {
                    assert_eq!(c.secondary_color(), TRANSLUCENT);
                    assert_eq!(c.primary_color(), PRIMARY);
                }
            }
            assert_eq!(c.document().undo_len(), history);
        }
    }
}

#[test]
fn source_target_and_size_stay_fixed_when_modifiers_slot_and_settings_change() {
    for control in [false, true] {
        let mut document = layered_document();
        document.select_layer(1).unwrap();
        let mut c = picker(document);
        let history = c.document().undo_len();
        c.set_color_slot(ColorSlot::Secondary);
        c.set_input_modifiers(InputModifiers {
            control,
            ..Default::default()
        })
        .unwrap();
        let p = c.viewport().to_logical(1., 1.);
        c.pointer_down_with_button(p.0, p.1, PointerButton::Secondary)
            .unwrap();
        assert_eq!(c.secondary_color(), if control { WHITE } else { RED });
        c.set_color_slot(ColorSlot::Primary);
        c.set_input_modifiers(InputModifiers {
            control: !control,
            ..Default::default()
        })
        .unwrap();
        configure(
            &mut c,
            PickerSettings {
                sampling: PickerSampling::Image,
                size: SampleSize::Square51,
                ..Default::default()
            },
        );
        let p = c.viewport().to_logical(3., 1.);
        c.pointer_move(p.0, p.1).unwrap();
        assert_eq!(
            c.secondary_color(),
            if control {
                HALF_BLUE_ON_WHITE
            } else {
                Color::TRANSPARENT
            }
        );
        assert_eq!(c.primary_color(), PRIMARY);
        assert_eq!(c.color_slot(), ColorSlot::Primary);
        c.pointer_up().unwrap();
        assert_eq!(c.document().undo_len(), history);
    }
}

#[test]
fn layer_ignores_opacity_and_preview_while_control_reads_the_frozen_image() {
    let mut document = layered_document();
    document.select_layer(2).unwrap();
    let preview = Shape::new(ShapeKind::Rectangle, 2, 0, 2, 4, ShapeStyle::Fill(RED));
    document.begin_shape(preview).unwrap();
    let mut c = picker(document);
    let history = c.document().undo_len();
    click(&mut c, 3., 1.);
    assert_eq!(c.primary_color(), BLUE);
    assert!(c.document().active_shape().is_some());
    // Freeze the composite after recoloring; the next sample must use that exact image.
    let composite = c.document().composite_pixel(3, 1, true);
    c.set_input_modifiers(InputModifiers {
        control: true,
        ..Default::default()
    })
    .unwrap();
    click(&mut c, 3., 1.);
    assert_eq!(c.primary_color(), composite);
    assert_eq!(c.document().undo_len(), history);
}

fn pattern_picker() -> Controller {
    // Nine samples include translucent pixels and a zero-alpha pixel with nonzero RGB.
    let pixels = vec![
        Color::rgba(255, 0, 0, 255),
        Color::rgba(0, 255, 0, 128),
        Color::rgba(0, 0, 255, 64),
        Color::rgba(100, 50, 0, 200),
        Color::rgba(20, 40, 60, 100),
        Color::rgba(9, 8, 7, 0),
        Color::rgba(0, 100, 200, 50),
        Color::rgba(200, 100, 50, 250),
        Color::rgba(5, 10, 15, 103),
    ];
    picker(Document::from_pixels(3, 3, 96., PixelStore::from_vec(3, 3, pixels).unwrap()).unwrap())
}

#[test]
fn square_three_uses_premultiplied_rounding_and_clips_at_the_corner() {
    let mut c = pattern_picker();
    let history = c.document().undo_len();
    configure(
        &mut c,
        PickerSettings {
            size: SampleSize::Square3,
            ..Default::default()
        },
    );
    click(&mut c, 1.9, 1.1);
    // Sum alpha=1150, RGB sums=137540/77670/46365; rounded RGBA=(120,68,40,128).
    assert_eq!(c.primary_color(), Color::rgba(120, 68, 40, 128));
    click(&mut c, 0., 0.);
    // Four pixels: alpha=683, RGB sums=87025/46640/6000, so (127,68,9,171).
    assert_eq!(c.primary_color(), Color::rgba(127, 68, 9, 171));
    assert_eq!(c.document().undo_len(), history);
}

#[test]
fn largest_square_averages_the_entire_small_canvas() {
    let mut c = picker(layered_document());
    image(&mut c);
    configure(
        &mut c,
        PickerSettings {
            sampling: PickerSampling::Image,
            size: SampleSize::Square51,
            ..Default::default()
        },
    );
    let history = c.document().undo_len();
    click(&mut c, 0., 0.);
    // Eight pixels of each category: WHITE, HALF_BLUE_ON_WHITE, TRANSLUCENT, TRANSPARENT.
    assert_eq!(c.primary_color(), Color::rgba(164, 192, 221, 150));
    assert_eq!(c.document().undo_len(), history);
}

#[test]
fn every_size_preserves_a_uniform_sample_and_exposes_its_odd_side() {
    for (size, side) in SampleSize::ALL.into_iter().zip([1, 3, 5, 11, 31, 51]) {
        assert_eq!(size.side(), side);
        for sampling in [PickerSampling::Layer, PickerSampling::Image] {
            let document = Document::new_with_background(8, 4, 96., TRANSLUCENT).unwrap();
            let mut c = picker(document);
            let history = c.document().undo_len();
            configure(
                &mut c,
                PickerSettings {
                    sampling,
                    size,
                    ..Default::default()
                },
            );
            click(&mut c, 0., 0.);
            assert_eq!(c.primary_color(), TRANSLUCENT);
            assert_eq!(c.document().undo_len(), history);
        }
    }
}

#[test]
fn single_keeps_hidden_rgb_while_transparent_averages_are_canonical() {
    let hidden = Color::rgba(91, 82, 73, 0);
    for size in SampleSize::ALL {
        let document = Document::new_with_background(8, 4, 96., hidden).unwrap();
        let mut c = picker(document);
        let history = c.document().undo_len();
        configure(
            &mut c,
            PickerSettings {
                size,
                ..Default::default()
            },
        );
        click(&mut c, 1., 1.);
        assert_eq!(
            c.primary_color(),
            if size == SampleSize::Single {
                hidden
            } else {
                Color::TRANSPARENT
            }
        );
        assert_eq!(c.document().undo_len(), history);
    }
}

#[test]
fn previous_tool_returns_only_after_a_successful_released_pick_and_keeps_the_shape() {
    for previous in [
        Tool::Rectangle,
        Tool::Pan,
        Tool::Brush,
        Tool::Eraser,
        Tool::MoveShape,
    ] {
        let mut document = layered_document();
        document
            .begin_shape(Shape::new(
                ShapeKind::Rectangle,
                6,
                0,
                2,
                4,
                ShapeStyle::Fill(RED),
            ))
            .unwrap();
        let mut c = picker(document);
        c.set_tool(previous).unwrap();
        c.set_tool(Tool::Eyedropper).unwrap();
        // Re-selecting an already-selected tool must retain the recorded previous tool.
        c.set_tool(Tool::Eyedropper).unwrap();
        configure(
            &mut c,
            PickerSettings {
                after_click: AfterClick::PreviousTool,
                ..Default::default()
            },
        );
        let history = c.document().undo_len();
        let before = c.document().active_shape().unwrap();
        click(&mut c, 4., 1.);
        assert_eq!(c.tool(), previous);
        let after = c.document().active_shape().unwrap();
        assert_eq!(
            (after.x, after.y, after.width, after.height),
            (before.x, before.y, before.width, before.height)
        );
        assert_eq!(c.document().undo_len(), history);
    }
}

#[test]
fn outside_cancel_and_do_not_switch_leave_the_picker_selected() {
    for after_click in [AfterClick::DoNotSwitch, AfterClick::PreviousTool] {
        for cancel in [false, true] {
            let mut c = picker(layered_document());
            let history = c.document().undo_len();
            configure(
                &mut c,
                PickerSettings {
                    after_click,
                    ..Default::default()
                },
            );
            let p = c.viewport().to_logical(if cancel { 4. } else { -1. }, 1.);
            c.pointer_down(p.0, p.1).unwrap();
            if cancel {
                c.cancel_pointer().unwrap();
            } else {
                c.pointer_up().unwrap();
            }
            assert_eq!(c.tool(), Tool::Eyedropper);
            c.pointer_up().unwrap();
            assert_eq!(c.tool(), Tool::Eyedropper);
            assert_eq!(c.document().undo_len(), history);
        }
    }
    let mut c = picker(layered_document());
    let history = c.document().undo_len();
    click(&mut c, 4., 1.);
    assert_eq!(c.tool(), Tool::Eyedropper);
    assert_eq!(c.document().undo_len(), history);
}
