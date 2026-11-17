//! Projects workspace state into AppWindow properties without mutating documents.

use crate::{AppWindow, ControlKind, ControlPoint, workspace};
use hoshi_controller::{ColorSlot, HandleKind, ShapeMode};
use std::rc::Rc;

pub(crate) fn refresh(ui: &AppWindow, controller: &workspace::Workspace) {
    ui.set_has_document(controller.has_document());
    let document = controller.document();
    ui.set_image_width(document.width() as i32);
    ui.set_image_height(document.height() as i32);
    ui.set_can_flatten(
        document.layers().len() > 1
            || !document.layers()[0].visible()
            || document.layers()[0].opacity() != 255,
    );
    let viewport = controller.viewport();
    ui.set_zoom(viewport.zoom as f32);
    ui.set_offset_x(viewport.offset_x as f32);
    ui.set_offset_y(viewport.offset_y as f32);
    crate::render_canvas(ui, controller);
    let points = controller
        .handle_positions()
        .into_iter()
        .map(|p| ControlPoint {
            x: p.x as f32,
            y: p.y as f32,
            kind: match p.kind {
                HandleKind::Resize(_) => ControlKind::Resize,
                HandleKind::Rotate => ControlKind::Rotate,
                HandleKind::Pivot => ControlKind::Pivot,
            },
        })
        .collect::<Vec<_>>();
    ui.set_control_points(Rc::new(slint::VecModel::from(points)).into());
    ui.set_has_shape(controller.has_document() && document.active_shape().is_some());
    ui.set_can_undo(controller.has_document() && document.undo_len() > 0);
    ui.set_can_redo(controller.has_document() && document.redo_len() > 0);
    ui.set_selected_tool(crate::tool_key(controller.tool()).into());
    crate::history::refresh(ui, controller);
    ui.set_shape_mode(match controller.shape_mode() {
        ShapeMode::Fill => 0,
        ShapeMode::Outline => 1,
        ShapeMode::Both => 2,
    });
    crate::shape_rendering::refresh(ui, controller);
    crate::shape_bounds::refresh(ui, controller);
    ui.set_color_slot(if controller.color_slot() == ColorSlot::Primary {
        0
    } else {
        1
    });
    let primary = controller.primary_color();
    let secondary = controller.secondary_color();
    let primary_ui = slint::Color::from_argb_u8(primary.a, primary.r, primary.g, primary.b);
    let secondary_ui =
        slint::Color::from_argb_u8(secondary.a, secondary.r, secondary.g, secondary.b);
    if ui.get_workspace_primary() != primary_ui {
        ui.set_workspace_primary(primary_ui);
        ui.set_workspace_primary_description(
            crate::colors::describe_workspace_color(primary).into(),
        );
    }
    if ui.get_workspace_secondary() != secondary_ui {
        ui.set_workspace_secondary(secondary_ui);
        ui.set_workspace_secondary_description(
            crate::colors::describe_workspace_color(secondary).into(),
        );
    }
    let color = controller.selected_color();
    ui.set_selected_color(slint::Color::from_argb_u8(
        color.a, color.r, color.g, color.b,
    ));
    ui.set_selected_hex(hoshi_controller::color::format_hex(color).into());
    ui.invoke_sync_color(
        color.r.into(),
        color.g.into(),
        color.b.into(),
        color.a.into(),
        document.active_shape().is_some(),
    );
    ui.set_layer_name(document.layers()[document.selected_layer()].name().into());
    let text = controller.text();
    let shape = document
        .active_shape()
        .map(|s| {
            text.format(
                "status_shape",
                &[
                    ("width", &s.width.to_string()),
                    ("height", &s.height.to_string()),
                    ("x", &s.x.to_string()),
                    ("y", &s.y.to_string()),
                    ("angle", &s.rotation_degrees.rem_euclid(360).to_string()),
                ],
            )
        })
        .unwrap_or_default();
    let percent = viewport.zoom * 100.;
    let zoom_label = if percent < 1. {
        format!("{percent:.3}")
    } else {
        format!("{percent:.0}")
    };
    ui.set_status(
        text.format(
            "status_document",
            &[
                ("width", &document.width().to_string()),
                ("height", &document.height().to_string()),
                ("zoom", &zoom_label),
                ("undo", &document.undo_len().to_string()),
                ("shape", &shape),
            ],
        )
        .into(),
    );
    crate::documents::refresh(ui, controller);
    ui.invoke_sync_shape_properties();
}
