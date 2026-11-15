//! Translate reusable dock geometry into view properties. No document mutation.
use crate::{
    AppWindow,
    panel_layout::{DockLayout, PanelId, PanelSpec, Rect},
};
use slint::ComponentHandle;
use std::{cell::Cell, rc::Rc};

fn arrange(ui: &AppWindow) {
    let panels = [
        PanelSpec {
            id: PanelId::Colors,
            visible: ui.get_colors_visible() && ui.get_colors_docked(),
            placement: ui.get_colors_placement(),
            width: 420.,
        },
        PanelSpec {
            id: PanelId::History,
            visible: ui.get_history_visible(),
            placement: ui.get_history_placement(),
            width: 300.,
        },
        PanelSpec {
            id: PanelId::ShapeProperties,
            visible: ui.get_shape_properties_visible(),
            placement: ui.get_shape_properties_placement(),
            width: 340.,
        },
    ];
    let layout = DockLayout::calculate(
        f64::from(ui.get_workspace_width()),
        f64::from(ui.get_workspace_height()),
        78.,
        &panels,
    );
    ui.set_layout_min_width(layout.min_window_width as f32);
    let Rect {
        x,
        y,
        width,
        height,
    } = layout.canvas;
    ui.set_canvas_layout_x(x as f32);
    ui.set_canvas_layout_y(y as f32);
    ui.set_canvas_layout_width(width as f32);
    ui.set_canvas_layout_height(height as f32);
    for id in [PanelId::Colors, PanelId::History, PanelId::ShapeProperties] {
        let Rect {
            x,
            y,
            width,
            height,
        } = layout.panel(id).unwrap_or_default();
        match id {
            PanelId::Colors => {
                ui.set_colors_layout_x(x as f32);
                ui.set_colors_layout_y(y as f32);
                ui.set_colors_layout_width(width as f32);
                ui.set_colors_layout_height(height as f32);
            }
            PanelId::History => {
                ui.set_history_layout_x(x as f32);
                ui.set_history_layout_y(y as f32);
                ui.set_history_layout_width(width as f32);
                ui.set_history_layout_height(height as f32);
            }
            PanelId::ShapeProperties => {
                ui.set_shapes_layout_x(x as f32);
                ui.set_shapes_layout_y(y as f32);
                ui.set_shapes_layout_width(width as f32);
                ui.set_shapes_layout_height(height as f32);
            }
        }
    }
}
pub(super) fn bind(ui: &AppWindow) {
    let weak = ui.as_weak();
    let arranging = Rc::new(Cell::new(false));
    ui.on_layout_panels(move || {
        if arranging.replace(true) {
            return;
        }
        if let Some(ui) = weak.upgrade() {
            arrange(&ui);
        }
        arranging.set(false);
    });
    ui.invoke_layout_panels();
}

/// Finish pending property entries before document/modal transitions, then hide
/// every auxiliary host so closing the main window leaves no orphan window.
pub(super) fn close_adjustment_panels(ui: &AppWindow) {
    ui.invoke_close_shape_properties();
    ui.invoke_hide_colors();
    ui.invoke_close_history();
}
