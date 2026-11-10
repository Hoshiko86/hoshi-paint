//! Connect the shared color library to both Colors hosts. Palette persistence is
//! independent of document history; choosing a color uses the existing editor commands.
use crate::{
    AppWindow, ColorPickerWindow, NamedColorSuggestion, PaletteSwatch, SharedController,
    palette_store::PaletteLibrary, update,
};
use hoshi_controller::{ColorSlot, named_colors};
use slint::{ComponentHandle, ModelRc, VecModel};
use std::{
    cell::{Cell, RefCell},
    path::PathBuf,
    rc::Rc,
};

#[derive(Clone, Copy)]
enum PaletteFileAction {
    Import,
    SaveAs,
    Export,
}

struct Library {
    main: slint::Weak<AppWindow>,
    picker: slint::Weak<ColorPickerWindow>,
    controller: SharedController,
    palettes: RefCell<PaletteLibrary>,
    inserting: Cell<bool>,
    expanded: Cell<bool>,
    startup_error: Option<String>,
}
impl Library {
    fn blocked(&self) -> bool {
        self.main.upgrade().is_none_or(|ui| ui.get_modal_visible())
    }

    fn refresh(&self, error: Option<String>) {
        let (Some(ui), Some(picker)) = (self.main.upgrade(), self.picker.upgrade()) else {
            return;
        };
        let error = error.or_else(|| self.startup_error.clone());
        let library = self.palettes.borrow();
        let swatches: ModelRc<PaletteSwatch> = Rc::new(VecModel::from(
            library
                .current()
                .colors()
                .iter()
                .enumerate()
                .map(|(index, &color)| PaletteSwatch {
                    red: color.r.into(),
                    green: color.g.into(),
                    blue: color.b.into(),
                    alpha: color.a.into(),
                    index: index as i32,
                    label: hoshi_controller::color::format_hex(color).into(),
                    description: crate::colors::describe_swatch(
                        color,
                        named_colors::name_for(color),
                    )
                    .into(),
                })
                .collect::<Vec<_>>(),
        ))
        .into();
        let names: ModelRc<slint::SharedString> = Rc::new(VecModel::from(
            std::iter::once("Default".into())
                .chain(library.entries().iter().map(|p| p.name.clone().into()))
                .collect::<Vec<_>>(),
        ))
        .into();
        ui.set_color_swatches(swatches.clone());
        picker.set_swatches(swatches);
        ui.set_palette_names(names.clone());
        picker.set_palette_names(names);
        ui.set_palette_selected(library.selected_index());
        picker.set_palette_selected(library.selected_index());
        ui.set_palette_title(library.title().into());
        picker.set_palette_title(library.title().into());
        ui.set_palette_dirty(library.dirty());
        picker.set_palette_dirty(library.dirty());
        ui.set_palette_inserting(self.inserting.get());
        picker.set_palette_inserting(self.inserting.get());
        ui.set_palette_expanded(self.expanded.get());
        picker.set_palette_expanded(self.expanded.get());
        ui.set_palette_error(error.clone().unwrap_or_default().into());
        picker.set_palette_error(error.unwrap_or_default().into());
    }
    fn search(&self, query: slint::SharedString) {
        let suggestions: ModelRc<NamedColorSuggestion> = Rc::new(VecModel::from(
            named_colors::suggestions(&query)
                .into_iter()
                .map(|entry| NamedColorSuggestion {
                    name: entry.name.into(),
                    preview: slint::Color::from_argb_u8(
                        entry.color.a,
                        entry.color.r,
                        entry.color.g,
                        entry.color.b,
                    ),
                    description: crate::colors::describe_swatch(entry.color, Some(entry.name))
                        .into(),
                })
                .collect::<Vec<_>>(),
        ))
        .into();
        if let Some(ui) = self.main.upgrade() {
            ui.set_color_name_query(query.clone());
            ui.set_color_name_suggestions(suggestions.clone());
        }
        if let Some(picker) = self.picker.upgrade() {
            picker.set_color_name_query(query);
            picker.set_color_name_suggestions(suggestions);
        }
    }
    fn path_action(&self, path: PathBuf, action: PaletteFileAction) {
        if self.blocked() {
            return;
        }
        let result = match action {
            PaletteFileAction::Import => self.palettes.borrow_mut().import(&path),
            PaletteFileAction::SaveAs => self.palettes.borrow_mut().save_as(&path),
            PaletteFileAction::Export => self.palettes.borrow().export(&path),
        };
        if result.is_ok() {
            self.inserting.set(false);
        }
        self.refresh(result.err());
    }
    fn dialog(&self, action: PaletteFileAction) {
        let Some(ui) = self.main.upgrade() else {
            return;
        };
        if ui.get_modal_visible() {
            return;
        }
        let picker = self.picker.upgrade();
        let parent = if ui.get_colors_docked() {
            ui.window()
        } else if let Some(p) = &picker {
            p.window()
        } else {
            ui.window()
        };
        let mut dialog = rfd::FileDialog::new()
            .set_parent(&parent.window_handle())
            .add_filter("Paint.NET palette", &["txt"]);
        if let Ok(folder) = self.palettes.borrow().folder() {
            dialog = dialog.set_directory(folder);
        }
        ui.set_file_busy(true);
        let path = if matches!(action, PaletteFileAction::Import) {
            dialog.set_title("Import palette").pick_file()
        } else {
            dialog.set_file_name("Palette.txt").save_file()
        };
        ui.set_file_busy(false);
        if let Some(path) = path {
            self.path_action(path, action);
        }
    }
}

pub(super) fn bind(
    ui: &AppWindow,
    picker: &ColorPickerWindow,
    controller: SharedController,
    root: Option<PathBuf>,
) {
    let (palettes, error) = match PaletteLibrary::open(root) {
        Ok(library) => (library, None),
        Err(error) => (
            PaletteLibrary::open(None).expect("in-memory palette"),
            Some(format!("{error}. Using a temporary palette.")),
        ),
    };
    let library = Rc::new(Library {
        main: ui.as_weak(),
        picker: picker.as_weak(),
        controller,
        palettes: RefCell::new(palettes),
        inserting: Cell::new(false),
        expanded: Cell::new(false),
        startup_error: error.clone(),
    });
    let l = library.clone();
    ui.on_refresh_palette_library(move || {
        let result = l.palettes.borrow_mut().refresh();
        l.refresh(result.err());
    });
    let l = library.clone();
    ui.on_search_color_name(move |q| l.search(q));
    let l = library.clone();
    ui.on_choose_named_color(move |name| {
        if let Some(color) = named_colors::lookup(&name) {
            if let Some(p) = l.picker.upgrade() {
                p.invoke_set_rgba(
                    color.r.into(),
                    color.g.into(),
                    color.b.into(),
                    color.a.into(),
                );
            }
            l.search("".into());
            l.refresh(None);
        } else {
            l.refresh(Some("Choose a recognized color name.".into()));
        }
    });
    let l = library.clone();
    ui.on_select_palette_color(move |index, inactive| {
        if l.blocked() {
            return;
        }
        let Ok(index) = usize::try_from(index) else {
            return;
        };
        if l.inserting.get() {
            let color = l.controller.borrow().selected_color();
            let result = l.palettes.borrow_mut().replace(index, color);
            if result.is_ok() {
                l.inserting.set(false);
            }
            l.refresh(result.err());
        } else {
            let color = l.palettes.borrow().current().colors().get(index).copied();
            if let Some(color) = color {
                if inactive {
                    update(&l.main, &l.controller, |c| match c.color_slot() {
                        ColorSlot::Primary => c.set_secondary_color(color),
                        ColorSlot::Secondary => c.set_primary_color(color),
                    });
                } else if let Some(p) = l.picker.upgrade() {
                    p.invoke_set_rgba(
                        color.r.into(),
                        color.g.into(),
                        color.b.into(),
                        color.a.into(),
                    );
                }
            }
        }
    });
    let l = library.clone();
    ui.on_toggle_palette_insert(move || {
        l.inserting.set(!l.inserting.get());
        l.refresh(None);
    });
    let l = library.clone();
    ui.on_set_palette_expanded(move |value| {
        l.expanded.set(value);
        l.refresh(None);
    });
    let l = library.clone();
    ui.on_select_palette(move |index| {
        if l.blocked() {
            return;
        }
        let result = usize::try_from(index)
            .map_err(|_| "Choose a saved palette.".into())
            .and_then(|i| l.palettes.borrow_mut().select(i));
        l.inserting.set(false);
        l.refresh(result.err());
    });
    let l = library.clone();
    ui.on_reset_palette(move || {
        if l.blocked() {
            return;
        }
        let result = l.palettes.borrow_mut().reset();
        l.inserting.set(false);
        l.refresh(result.err());
    });
    let l = library.clone();
    ui.on_import_palette_path(move |p| {
        l.path_action(PathBuf::from(p.as_str()), PaletteFileAction::Import)
    });
    let l = library.clone();
    ui.on_save_palette_path(move |p| {
        l.path_action(PathBuf::from(p.as_str()), PaletteFileAction::SaveAs)
    });
    let l = library.clone();
    ui.on_export_palette_path(move |p| {
        l.path_action(PathBuf::from(p.as_str()), PaletteFileAction::Export)
    });
    let l = library.clone();
    ui.on_import_palette(move || l.dialog(PaletteFileAction::Import));
    let l = library.clone();
    ui.on_save_palette_as(move || l.dialog(PaletteFileAction::SaveAs));
    let l = library.clone();
    ui.on_export_palette(move || l.dialog(PaletteFileAction::Export));
    let l = library.clone();
    ui.on_open_palettes_folder(move || {
        let result = l.palettes.borrow().folder().and_then(|folder| {
            let command = if cfg!(target_os = "windows") {
                "explorer.exe"
            } else if cfg!(target_os = "macos") {
                "open"
            } else {
                "xdg-open"
            };
            std::process::Command::new(command)
                .arg(folder)
                .spawn()
                .map(|_| ())
                .map_err(|e| format!("Cannot open palettes folder: {e}"))
        });
        l.refresh(result.err());
    });
    macro_rules! forward { ($on:ident, $invoke:ident $(, $arg:ident)*) => { let weak = ui.as_weak(); picker.$on(move |$($arg),*| { if let Some(ui) = weak.upgrade() { ui.$invoke($($arg),*); } }); }; }
    forward!(on_search_color_name, invoke_search_color_name, q);
    forward!(on_choose_named_color, invoke_choose_named_color, name);
    forward!(
        on_select_palette_color,
        invoke_select_palette_color,
        index,
        inactive
    );
    forward!(on_toggle_palette_insert, invoke_toggle_palette_insert);
    forward!(on_set_palette_expanded, invoke_set_palette_expanded, value);
    forward!(on_select_palette, invoke_select_palette, index);
    forward!(on_reset_palette, invoke_reset_palette);
    forward!(on_import_palette, invoke_import_palette);
    forward!(on_save_palette_as, invoke_save_palette_as);
    forward!(on_export_palette, invoke_export_palette);
    forward!(on_open_palettes_folder, invoke_open_palettes_folder);
    library.search("".into());
    library.refresh(error);
}
