use hoshi_controller::{Color, ColorSlot, Controller, ShapeMode};
use hoshi_core::ShapeStyle;

#[test]
fn style_edits_preserve_geometry_and_commit_as_one_undoable_shape() {
    let mut c = Controller::new(64, 64).unwrap();
    let fill = Color::rgba(255, 40, 20, 128);
    let outline = Color::rgba(10, 30, 240, 255);
    c.set_primary_color(fill).unwrap();
    c.set_secondary_color(fill).unwrap();
    c.set_primary_color(outline).unwrap();
    c.pointer_down(8., 8.).unwrap();
    c.pointer_move(48., 40.).unwrap();
    c.pointer_up().unwrap();
    let geometry = c.document().active_shape().unwrap();
    assert_eq!(geometry.style, ShapeStyle::Fill(outline));
    c.set_shape_mode(ShapeMode::Both).unwrap();
    c.set_stroke_width(4).unwrap();
    let pixels = c.composite();
    assert_eq!(pixels[8 * 64 + 8], outline);
    assert_eq!(pixels[20 * 64 + 20], fill);
    c.set_shape_mode(ShapeMode::Outline).unwrap();
    assert_eq!(c.composite()[20 * 64 + 20], Color::rgba(0, 0, 0, 0));
    c.set_shape_mode(ShapeMode::Both).unwrap();
    c.set_shape_rotation(25).unwrap();
    let rotated = c.document().active_shape().unwrap();
    c.set_color_slot(ColorSlot::Secondary);
    c.set_selected_color(Color::rgba(20, 210, 80, 100)).unwrap();
    let changed = c.document().active_shape().unwrap();
    assert_eq!(
        (
            changed.x,
            changed.y,
            changed.width,
            changed.height,
            changed.rotation_degrees
        ),
        (
            rotated.x,
            rotated.y,
            rotated.width,
            rotated.height,
            rotated.rotation_degrees
        )
    );
    assert_eq!(c.primary_color(), outline);
    assert_eq!(c.document().undo_len(), 0);
    let expected = c.composite();
    c.commit().unwrap();
    assert_eq!(c.document().undo_len(), 1);
    c.undo().unwrap();
    assert!(c.composite().iter().all(|p| p.a == 0));
    c.redo().unwrap();
    assert_eq!(c.composite(), expected);
}

#[test]
fn settings_survive_cancel_and_apply_to_the_next_shape_without_changing_old_pixels() {
    let mut c = Controller::new(64, 64).unwrap();
    c.set_shape_mode(ShapeMode::Both).unwrap();
    c.set_stroke_width(5).unwrap();
    c.pointer_down(8., 8.).unwrap();
    c.pointer_move(48., 40.).unwrap();
    c.pointer_up().unwrap();
    c.commit().unwrap();
    let committed = c.composite();
    c.set_color_slot(ColorSlot::Secondary);
    let outline = Color::rgba(255, 255, 0, 255);
    c.set_primary_color(outline).unwrap();
    c.set_stroke_width(65536).unwrap();
    assert_eq!(c.composite(), committed);
    assert_eq!(c.document().undo_len(), 1);
    c.pointer_down(10., 10.).unwrap();
    c.pointer_move(30., 30.).unwrap();
    c.pointer_up().unwrap();
    assert_eq!(
        c.document().active_shape().unwrap().style,
        ShapeStyle::Both {
            fill: c.secondary_color(),
            outline,
            width: 65536,
        }
    );
    assert_eq!(c.composite()[20 * 64 + 20], outline);
    let before = c.document().active_shape();
    assert!(c.set_stroke_width(0).is_err());
    assert!(c.set_stroke_width(65537).is_err());
    assert_eq!(c.stroke_width(), 65536);
    assert_eq!(c.document().active_shape(), before);
    c.cancel_pointer();
    // Escape's explicit shape cancellation leaves drawing settings intact.
    c.cancel();
    assert_eq!(c.composite(), committed);
    assert_eq!(c.stroke_width(), 65536);
}

#[test]
fn workspace_slots_are_independent_of_style_and_swapping_is_one_live_preview_edit() {
    let mut c = Controller::new(64, 64).unwrap();
    assert_eq!(c.primary_color(), Color::rgba(0, 0, 0, 255));
    assert_eq!(c.secondary_color(), Color::rgba(255, 255, 255, 255));
    let primary = Color::rgba(220, 20, 40, 128);
    let secondary = Color::rgba(20, 80, 240, 210);
    c.set_primary_color(primary).unwrap();
    c.set_color_slot(ColorSlot::Secondary);
    c.set_selected_color(secondary).unwrap();
    c.pointer_down(8., 8.).unwrap();
    c.pointer_move(48., 40.).unwrap();
    c.pointer_up().unwrap();
    assert_eq!(
        c.document().active_shape().unwrap().style,
        ShapeStyle::Fill(primary)
    );
    c.set_shape_mode(ShapeMode::Outline).unwrap();
    assert_eq!(c.color_slot(), ColorSlot::Secondary);
    assert_eq!(
        c.document().active_shape().unwrap().style,
        ShapeStyle::Outline {
            color: primary,
            width: 2
        }
    );
    c.set_shape_mode(ShapeMode::Both).unwrap();
    let before = c.document().active_shape().unwrap();
    c.swap_colors().unwrap();
    assert_eq!(c.primary_color(), secondary);
    assert_eq!(c.secondary_color(), primary);
    let after = c.document().active_shape().unwrap();
    assert_eq!(
        (after.x, after.y, after.width, after.height),
        (before.x, before.y, before.width, before.height)
    );
    assert_eq!(
        after.style,
        ShapeStyle::Both {
            fill: primary,
            outline: secondary,
            width: 2
        }
    );
    assert_eq!(c.document().undo_len(), 0);
    c.commit().unwrap();
    let committed = c.composite();
    c.reset_colors().unwrap();
    assert_eq!(c.color_slot(), ColorSlot::Secondary);
    assert_eq!(c.primary_color(), Color::rgba(0, 0, 0, 255));
    assert_eq!(c.secondary_color(), Color::rgba(255, 255, 255, 255));
    assert_eq!(c.composite(), committed);
    c.undo().unwrap();
    c.redo().unwrap();
    assert_eq!(c.composite(), committed);
    assert_eq!(c.selected_color(), Color::rgba(255, 255, 255, 255));
}
