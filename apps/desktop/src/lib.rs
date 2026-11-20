//! Slint desktop adapter around the UI-independent controller (ADR 0006).

use hoshi_controller::{Controller, ControllerError, ShapeMode, Tool, ToolId};
use slint::ComponentHandle;
use std::{cell::RefCell, rc::Rc};
mod color_library;
mod colors;
pub mod documents;
mod error_text;
pub mod files;
mod history;
mod image_settings;
mod input;
mod localization;
pub mod palette_store;
mod panel_hosts;
mod panel_layout;
mod resources;
/// Native screen sampling (public so every platform build sees its full error type).
pub mod screen_pick;
mod shape_bounds;
mod shape_properties;
mod shape_rendering;
mod theme;
mod tool_options;
mod view_model;
pub(crate) use view_model::refresh;
pub mod workspace;

slint::include_modules!();

pub type SharedController = Rc<RefCell<workspace::Workspace>>;

/// The user-facing message for a workspace error in the system locale.
pub fn describe_error(error: &workspace::WorkspaceError) -> String {
    let text = localization::Localizer::new(&localization::system_selection());
    error_text::workspace(&text, error)
}

/// The user-facing message for a workspace error in the first supported
/// locale of `preferred` (BCP 47 tags such as `"es-ES"`), falling back to en-US.
pub fn describe_error_in(error: &workspace::WorkspaceError, preferred: &[&str]) -> String {
    let text =
        localization::Localizer::new(&localization::select_preferred(preferred.iter().copied()));
    error_text::workspace(&text, error)
}

/// Command-line help in the system locale.
pub fn usage_text() -> String {
    localization::Localizer::new(&localization::system_selection())
        .text("cli_usage")
        .to_owned()
}

pub fn refresh_workspace(ui: &AppWindow, state: &SharedController) {
    refresh(ui, &state.borrow());
}

fn render_canvas(ui: &AppWindow, controller: &Controller) {
    if !ui.get_has_document() {
        ui.set_canvas_image(slint::Image::default());
        return;
    }
    let document = controller.document();
    let large = u64::from(document.width()) * u64::from(document.height()) > 1_048_576;
    ui.set_viewport_preview(large);
    let preview = if large {
        controller.render_viewport(
            ui.get_canvas_width().ceil().max(1.) as u32,
            ui.get_canvas_height().ceil().max(1.) as u32,
            f64::from(ui.window().scale_factor()),
        )
    } else {
        controller.thumbnail(document.width(), document.height())
    };
    let mut pixels =
        slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(preview.width, preview.height);
    pixels.make_mut_bytes().copy_from_slice(&preview.rgba);
    ui.set_canvas_image(slint::Image::from_rgba8(pixels));
}
fn bind_rendering(ui: &AppWindow, state: SharedController) {
    let weak = ui.as_weak();
    let center_state = state.clone();
    ui.on_initialize_canvas_view(move || {
        let weak = weak.clone();
        let center_state = center_state.clone();
        slint::Timer::single_shot(std::time::Duration::ZERO, move || {
            if let Some(ui) = weak.upgrade()
                && ui.window().is_visible()
                && let Ok(mut state) = center_state.try_borrow_mut()
                && state.has_document()
            {
                let _ = state.center_viewport(
                    f64::from(ui.get_canvas_width()),
                    f64::from(ui.get_canvas_height()),
                );
                refresh(&ui, &state);
            }
        });
    });
    let weak = ui.as_weak();
    let fit_state = state.clone();
    ui.on_fit_canvas(move || {
        if let Some(ui) = weak.upgrade()
            && !ui.get_modal_visible()
            && let Ok(mut state) = fit_state.try_borrow_mut()
            && state.has_document()
        {
            let result = state.fit_viewport(
                f64::from(ui.get_canvas_width()),
                f64::from(ui.get_canvas_height()),
            );
            refresh(&ui, &state);
            if let Err(error) = result {
                ui.set_status(error_text::controller(state.text(), &error).into());
            }
        }
    });
    let weak = ui.as_weak();
    ui.on_render_viewport(move || {
        if let Some(ui) = weak.upgrade()
            && let Ok(state) = state.try_borrow()
        {
            render_canvas(&ui, &state);
        }
    });
}

fn tool_for(id: ToolId) -> Option<Tool> {
    match id {
        ToolId::Shapes => Some(Tool::Rectangle),
        ToolId::Pan => Some(Tool::Pan),
        ToolId::ColorPicker => Some(Tool::Eyedropper),
        ToolId::Paintbrush => Some(Tool::Brush),
        ToolId::Eraser => Some(Tool::Eraser),
        _ => None,
    }
}

fn tool_key(tool: Tool) -> &'static str {
    match tool {
        Tool::Rectangle | Tool::MoveShape => ToolId::Shapes,
        Tool::Pan => ToolId::Pan,
        Tool::Eyedropper => ToolId::ColorPicker,
        Tool::Brush => ToolId::Paintbrush,
        Tool::Eraser => ToolId::Eraser,
    }
    .key()
}

fn update(
    ui: &slint::Weak<AppWindow>,
    controller: &SharedController,
    action: impl FnOnce(&mut Controller) -> Result<(), ControllerError>,
) {
    if let Some(ui) = ui.upgrade() {
        if ui.get_modal_visible() {
            return;
        }
        let (result, message) = {
            let mut controller = controller.borrow_mut();
            let result = action(&mut controller);
            refresh(&ui, &controller);
            // Errors carry no user copy; the locale catalogs supply it (#86).
            let message = result
                .as_ref()
                .err()
                .map(|error| error_text::controller(controller.text(), error));
            (result, message)
        };
        if let (Err(error), Some(message)) = (result, message) {
            if matches!(
                error,
                ControllerError::Core(
                    hoshi_core::Error::HistoryIo | hoshi_core::Error::HistoryCorrupt
                )
            ) {
                panel_hosts::close_adjustment_panels(&ui);
                ui.set_file_error(message.as_str().into());
            }
            ui.set_status(message.into());
        }
    }
}

pub fn create_app() -> Result<(AppWindow, SharedController), slint::PlatformError> {
    let (ui, controller, _) = create_app_with_colors()?;
    Ok((ui, controller))
}

pub fn create_app_with_colors()
-> Result<(AppWindow, SharedController, Rc<ColorPickerWindow>), slint::PlatformError> {
    let (ui, controller, colors, _) = create_app_with_panels()?;
    Ok((ui, controller, colors))
}

/// Handles to the main window and its reusable adjustment-panel hosts.
pub struct AppPanels {
    pub ui: AppWindow,
    pub controller: SharedController,
    pub colors: Rc<ColorPickerWindow>,
    pub history: Rc<HistoryWindow>,
    pub shapes: Rc<ShapePropertiesWindow>,
}
pub fn create_app_with_dialogs(
    dialogs: Rc<dyn documents::FileDialogs>,
) -> Result<(AppWindow, SharedController), slint::PlatformError> {
    let app = create_panels(Some(dialogs), None)?;
    Ok((app.ui, app.controller))
}
pub fn create_app_with_shape_properties()
-> Result<(AppWindow, SharedController, Rc<ShapePropertiesWindow>), slint::PlatformError> {
    let app = create_panels(None, default_palette_root())?;
    Ok((app.ui, app.controller, app.shapes))
}
pub fn create_app_with_all_panels() -> Result<AppPanels, slint::PlatformError> {
    create_panels(None, default_palette_root())
}
pub fn create_app_with_panels() -> Result<
    (
        AppWindow,
        SharedController,
        Rc<ColorPickerWindow>,
        Rc<HistoryWindow>,
    ),
    slint::PlatformError,
> {
    let app = create_panels(None, default_palette_root())?;
    Ok((app.ui, app.controller, app.colors, app.history))
}
pub fn create_app_with_palette_root(
    root: Option<std::path::PathBuf>,
) -> Result<AppPanels, slint::PlatformError> {
    create_panels(None, root)
}
fn default_palette_root() -> Option<std::path::PathBuf> {
    documents::recent_store_path().and_then(|path| path.parent().map(std::path::Path::to_path_buf))
}
fn create_panels(
    dialogs: Option<Rc<dyn documents::FileDialogs>>,
    palette_root: Option<std::path::PathBuf>,
) -> Result<AppPanels, slint::PlatformError> {
    resources::initialize();
    let ui = AppWindow::new()?;
    // Keep fixed-color icons legible until they follow the theme in #244.
    let scheme = theme::Scheme::Dark;
    let locale = localization::system_selection();
    let text = localization::Localizer::new(&locale);
    localization::apply(&ui, &text);
    history::install(&ui, &text);
    let recent_store = if dialogs.is_some() {
        None
    } else {
        documents::recent_store_path()
    };
    let controller = Rc::new(RefCell::new(workspace::Workspace::with_recent_store(
        recent_store,
    )));
    controller.borrow_mut().set_text(text.clone());
    image_settings::bind(&ui, controller.clone(), &text);
    bind_rendering(&ui, controller.clone());
    panel_hosts::bind(&ui);
    refresh(&ui, &controller.borrow());
    input::bind(&ui, controller.clone());
    shape_rendering::bind(&ui, controller.clone());
    tool_options::bind(&ui, controller.clone());
    shape_bounds::bind(&ui, controller.clone());
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_choose_tool(move |tool| {
        if let Some(tool) = ToolId::from_key(tool.as_str()).and_then(tool_for) {
            update(&weak, &state, |c| c.set_tool(tool));
        }
    });
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_edit_brush_width(move |width| {
        // Preserve decimal input through Slint's f32 transport, as in Shape Properties.
        let width = width.to_string().parse::<f64>().unwrap_or(f64::NAN);
        update(&weak, &state, |c| c.set_brush_width(width));
    });
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_choose_color(move |color| {
        update(&weak, &state, |c| {
            c.set_selected_color(match color {
                1 => hoshi_controller::Color::rgba(232, 104, 92, 255),
                2 => hoshi_controller::Color::rgba(54, 167, 140, 255),
                _ => hoshi_controller::Color::rgba(72, 136, 240, 255),
            })
        })
    });
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_rename(move |name| update(&weak, &state, |c| c.rename_layer(&name)));
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_select_shape_mode(move |mode| {
        update(&weak, &state, |c| {
            c.set_shape_mode(match mode {
                1 => ShapeMode::Outline,
                2 => ShapeMode::Both,
                _ => ShapeMode::Fill,
            })
        });
    });

    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_zoom_at(move |x, y, delta| {
        update(&weak, &state, |c| {
            c.zoom_at(x.into(), y.into(), 1.2_f64.powf(delta.into()))
        })
    });
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_action(move |action| {
        let canvas_size = weak.upgrade().map(|ui| {
            (
                f64::from(ui.get_canvas_width()),
                f64::from(ui.get_canvas_height()),
            )
        });
        update(&weak, &state, |c| match action {
            0 => c.undo().map(|_| ()),
            1 => c.redo().map(|_| ()),
            2 => c.commit().map(|_| ()),
            3 => {
                c.cancel();
                Ok(())
            }
            4 => {
                c.reset_view();
                canvas_size.map_or(Ok(()), |(width, height)| c.center_viewport(width, height))
            }
            5 => c.nudge_shape(-1, 0),
            6 => c.nudge_shape(1, 0),
            7 => c.nudge_shape(0, -1),
            8 => c.nudge_shape(0, 1),
            9 | 10 => {
                let s = c
                    .document()
                    .active_shape()
                    .ok_or(ControllerError::NoActiveShape)?;
                let size = |n: u32| {
                    if action == 9 {
                        n.saturating_sub(8).max(1)
                    } else {
                        n.saturating_add(8)
                    }
                };
                c.set_shape_bounds(s.x, s.y, size(s.width), size(s.height))
            }
            _ => Ok(()),
        })
    });
    let picker = colors::bind_picker(&ui, controller.clone(), palette_root, &text)?;
    localization::apply_colors(&picker, &text);
    let history = history::bind_window(&ui, controller.clone(), &text)?;
    let shapes = shape_properties::create(&ui, controller.clone())?;
    localization::apply_shapes(&shapes, &text);
    if let Ok(theme) = theme::builtin(scheme) {
        theme::apply(&theme, &ui, &history, &picker, &shapes);
    }
    documents::bind(&ui, controller.clone(), dialogs, text);
    let main = ui.as_weak();
    picker.on_file_action(move |action| {
        if let Some(ui) = main.upgrade() {
            ui.invoke_file_action(action);
        }
    });
    let main = ui.as_weak();
    picker.on_cycle_document(move |backward| {
        if let Some(ui) = main.upgrade() {
            ui.invoke_cycle_document(backward);
        }
    });
    let main = ui.as_weak();
    history.on_file_action(move |action| {
        if let Some(ui) = main.upgrade() {
            ui.invoke_file_action(action);
        }
    });
    let main = ui.as_weak();
    history.on_cycle_document(move |backward| {
        if let Some(ui) = main.upgrade() {
            ui.invoke_cycle_document(backward);
        }
    });

    Ok(AppPanels {
        ui,
        controller,
        colors: picker,
        history,
        shapes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_rail_tool_id_parses_and_maps_to_an_implemented_tool() {
        let rail = include_str!("../ui/drawing-tools.slint");
        let mut tools = Vec::new();
        for line in rail.lines() {
            if let Some(entry) = line.trim().strip_prefix("{ id: \"") {
                let key = entry.split('"').next().unwrap();
                let id = ToolId::from_key(key).expect("rail tool has a stable tool id");
                assert!(tool_for(id).is_some());
                tools.push(id);
            }
        }
        assert_eq!(
            tools,
            [ToolId::Paintbrush, ToolId::Eraser, ToolId::ColorPicker]
        );
        let main = include_str!("../ui/main-window.slint");
        assert!(main.contains("root.choose-tool(\"shapes\")"));
        assert_eq!(tool_for(ToolId::Shapes), Some(Tool::Rectangle));
    }

    #[test]
    fn supported_tools_round_trip_and_move_shape_uses_the_shapes_key() {
        for tool in [
            Tool::Rectangle,
            Tool::Pan,
            Tool::Eyedropper,
            Tool::Brush,
            Tool::Eraser,
        ] {
            assert_eq!(
                ToolId::from_key(tool_key(tool)).and_then(tool_for),
                Some(tool)
            );
        }
        assert_eq!(tool_key(Tool::MoveShape), ToolId::Shapes.key());
        for id in ToolId::ALL {
            if !matches!(
                id,
                ToolId::Shapes
                    | ToolId::Pan
                    | ToolId::ColorPicker
                    | ToolId::Paintbrush
                    | ToolId::Eraser
            ) {
                assert_eq!(tool_for(id), None);
            }
        }
    }

    #[test]
    fn unknown_and_unmapped_tool_keys_preserve_the_current_tool_and_drag() {
        slint::platform::set_platform(Box::new(i_slint_backend_testing::TestingBackend::new(
            i_slint_backend_testing::TestingBackendOptions {
                mock_time: true,
                threading: false,
                renderer_name: Some("software".into()),
            },
        )))
        .unwrap();
        let app = create_app_with_palette_root(None).unwrap();
        app.ui.invoke_choose_tool(ToolId::Paintbrush.key().into());
        assert_eq!(app.controller.borrow().tool(), Tool::Brush);
        app.ui.invoke_pointer_down(40., 50.);
        assert!(app.controller.borrow().is_dragging());
        for key in ["unknown-tool", "", ToolId::Zoom.key(), "screen-picker"] {
            app.ui.invoke_choose_tool(key.into());
            assert_eq!(app.controller.borrow().tool(), Tool::Brush);
            assert_eq!(app.ui.get_selected_tool(), ToolId::Paintbrush.key());
            assert!(app.controller.borrow().is_dragging());
        }
        app.ui.invoke_pointer_up(40., 50.);
        app.controller
            .borrow_mut()
            .set_tool(Tool::MoveShape)
            .unwrap();
        refresh_workspace(&app.ui, &app.controller);
        assert_eq!(app.ui.get_selected_tool(), ToolId::Shapes.key());
    }
}
