use hoshi_core::Color;
use hoshi_desktop::{
    AppWindow, SharedController, create_app_with_dialogs, documents::FileDialogs, refresh_workspace,
};
use slint::{ComponentHandle, LogicalSize, platform::WindowEvent};
use std::{
    cell::RefCell,
    collections::VecDeque,
    path::{Path, PathBuf},
    rc::Rc,
};

#[derive(Default)]
struct Dialogs {
    open: RefCell<VecDeque<Option<Vec<PathBuf>>>>,
    save: RefCell<VecDeque<Option<PathBuf>>>,
}
impl FileDialogs for Dialogs {
    fn open(&self, _: &slint::Window) -> Option<Vec<PathBuf>> {
        self.open.borrow_mut().pop_front().flatten()
    }
    fn save(&self, _: &slint::Window, _: &Path) -> Option<PathBuf> {
        self.save.borrow_mut().pop_front().flatten()
    }
}
fn shortcut(ui: &hoshi_desktop::AppWindow, key: &str, shift: bool, alt: bool) {
    use slint::platform::{Key, WindowEvent};
    for modifier in [
        Some(Key::Control),
        shift.then_some(Key::Shift),
        alt.then_some(Key::Alt),
    ]
    .into_iter()
    .flatten()
    {
        ui.window().dispatch_event(WindowEvent::KeyPressed {
            text: modifier.into(),
        });
    }
    ui.window()
        .dispatch_event(WindowEvent::KeyPressed { text: key.into() });
    ui.window()
        .dispatch_event(WindowEvent::KeyReleased { text: key.into() });
    for modifier in [
        alt.then_some(Key::Alt),
        shift.then_some(Key::Shift),
        Some(Key::Control),
    ]
    .into_iter()
    .flatten()
    {
        ui.window().dispatch_event(WindowEvent::KeyReleased {
            text: modifier.into(),
        });
    }
}

fn setup() -> (AppWindow, SharedController, Rc<Dialogs>) {
    slint::platform::set_platform(Box::new(i_slint_backend_testing::TestingBackend::new(
        i_slint_backend_testing::TestingBackendOptions {
            mock_time: true,
            threading: false,
            renderer_name: Some("software".into()),
        },
    )))
    .unwrap();
    let dialogs = Rc::new(Dialogs::default());
    let (ui, state) = create_app_with_dialogs(dialogs.clone()).unwrap();
    ui.window().set_size(LogicalSize::new(1120., 850.));
    ui.show().unwrap();
    (ui, state, dialogs)
}

fn rename(ui: &AppWindow, state: &SharedController, name: &str) {
    state.borrow_mut().rename_layer(name).unwrap();
    refresh_workspace(ui, state);
}

fn new_dirty_tab(ui: &AppWindow, state: &SharedController, name: &str) -> (u32, String) {
    state
        .borrow_mut()
        .new_image(8, 8, 96., Color::TRANSPARENT)
        .unwrap();
    refresh_workspace(ui, state);
    rename(ui, state, name);
    let state = state.borrow();
    let tab = state.active_tab().unwrap();
    (tab.id, state.tab_name(&tab.name))
}

#[test]
fn failed_save_as_keeps_undo_and_redo() {
    let (ui, state, dialogs) = setup();
    rename(&ui, &state, "One");
    rename(&ui, &state, "Two");
    state.borrow_mut().undo().unwrap();
    refresh_workspace(&ui, &state);
    assert_eq!(state.borrow().document().undo_len(), 1);
    assert_eq!(state.borrow().document().redo_len(), 1);
    assert!(state.borrow().active_tab().unwrap().dirty());
    assert_eq!(state.borrow().document().layers()[0].name(), "One");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("missing/sub.hoshi");
    dialogs.save.borrow_mut().push_back(Some(path.clone()));
    shortcut(&ui, "S", true, false);
    assert!(!ui.get_file_error().is_empty());
    assert!(!path.exists());
    assert!(state.borrow().active_tab().unwrap().dirty());
    assert_eq!(state.borrow().document().undo_len(), 1);
    assert_eq!(state.borrow().document().redo_len(), 1);
    assert_eq!(state.borrow().document().layers()[0].name(), "One");
    ui.invoke_dismiss_file_error();
    state.borrow_mut().redo().unwrap();
    refresh_workspace(&ui, &state);
    assert_eq!(state.borrow().document().layers()[0].name(), "Two");
}

#[test]
fn ctrl_shift_w_closes_clean_tabs_and_prompts_for_each_dirty_one() {
    let (ui, state, dialogs) = setup();
    let a = state.borrow().active_id().unwrap();
    let (b, b_name) = new_dirty_tab(&ui, &state, "B");
    let (c, c_name) = new_dirty_tab(&ui, &state, "C");
    shortcut(&ui, "W", true, false);
    assert!(!state.borrow().tabs().iter().any(|tab| tab.id == a));
    assert!(ui.get_file_prompt_visible());
    assert_eq!(ui.get_file_prompt_name(), b_name);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("b.hoshi");
    dialogs.save.borrow_mut().push_back(Some(path.clone()));
    ui.invoke_file_prompt_response(0);
    assert!(path.exists());
    assert!(!state.borrow().tabs().iter().any(|tab| tab.id == b));
    assert!(ui.get_file_prompt_visible());
    assert_eq!(ui.get_file_prompt_name(), c_name);
    ui.invoke_file_prompt_response(2);
    assert!(!ui.get_file_prompt_visible());
    assert_eq!(state.borrow().tabs().len(), 1);
    assert_eq!(state.borrow().active_id(), Some(c));
    assert!(state.borrow().active_tab().unwrap().dirty());
    assert!(ui.window().is_visible());
    shortcut(&ui, "W", true, false);
    assert!(ui.get_file_prompt_visible());
    ui.invoke_file_prompt_response(1);
    assert!(!state.borrow().has_document());
    assert!(!ui.get_has_document());
    assert!(ui.window().is_visible());
}

#[test]
fn window_close_request_with_a_dirty_tab_prompts_and_cancel_keeps_the_window() {
    let (ui, state, _) = setup();
    rename(&ui, &state, "Dirty");
    let id = state.borrow().active_id();
    let undo_len = state.borrow().document().undo_len();
    ui.window().dispatch_event(WindowEvent::CloseRequested);
    assert!(ui.get_file_prompt_visible());
    assert!(ui.window().is_visible());
    assert_eq!(state.borrow().active_id(), id);
    let prompt_name = ui.get_file_prompt_name();
    let tab_count = state.borrow().tabs().len();
    ui.window().dispatch_event(WindowEvent::CloseRequested);
    assert!(ui.get_file_prompt_visible());
    assert_eq!(ui.get_file_prompt_name(), prompt_name);
    assert_eq!(state.borrow().tabs().len(), tab_count);
    ui.invoke_file_prompt_response(2);
    assert!(!ui.get_file_prompt_visible());
    assert!(ui.window().is_visible());
    assert_eq!(state.borrow().active_id(), id);
    assert!(state.borrow().active_tab().unwrap().dirty());
    assert_eq!(state.borrow().document().undo_len(), undo_len);
}

#[test]
fn window_close_request_discard_closes_every_tab_and_hides_the_window() {
    let (ui, state, _) = setup();
    rename(&ui, &state, "First");
    let first_name = {
        let state = state.borrow();
        state.tab_name(&state.active_tab().unwrap().name)
    };
    let (_, second_name) = new_dirty_tab(&ui, &state, "Second");
    ui.window().dispatch_event(WindowEvent::CloseRequested);
    assert!(ui.get_file_prompt_visible());
    assert_eq!(ui.get_file_prompt_name(), first_name);
    ui.invoke_file_prompt_response(1);
    assert!(ui.get_file_prompt_visible());
    assert_eq!(ui.get_file_prompt_name(), second_name);
    ui.invoke_file_prompt_response(1);
    assert!(!state.borrow().has_document());
    assert!(!ui.get_has_document());
    assert!(!ui.window().is_visible());
}

#[test]
fn window_close_request_with_only_clean_tabs_hides_without_a_prompt() {
    let (ui, state, _) = setup();
    assert!(!state.borrow().any_dirty());
    ui.window().dispatch_event(WindowEvent::CloseRequested);
    assert!(!ui.get_file_prompt_visible());
    assert!(!state.borrow().has_document());
    assert!(!ui.window().is_visible());
}
