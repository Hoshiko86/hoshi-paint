use hoshi_canvas_spike::{Translations, create_app_with_panels, refresh_workspace};
use hoshi_controller::{Color, Resampling};
use i_slint_backend_testing::ElementHandle;
use slint::{ComponentHandle, LogicalSize, Model};

#[test]
fn long_history_model_stays_shared_and_navigates_disk_states_across_tabs() {
    slint::platform::set_platform(Box::new(i_slint_backend_testing::TestingBackend::new(
        i_slint_backend_testing::TestingBackendOptions {
            mock_time: true,
            threading: false,
            renderer_name: Some("software".into()),
        },
    )))
    .unwrap();
    let (ui, state, _, history) = create_app_with_panels().unwrap();
    ui.window().set_size(LogicalSize::new(1120., 850.));
    ui.show().unwrap();
    let first = state.borrow().active_id().unwrap();
    for index in 0..1024 {
        state
            .borrow_mut()
            .rename_layer(&format!("Layer {index}"))
            .unwrap();
    }
    refresh_workspace(&ui, &state);
    ui.invoke_show_history();
    ui.invoke_set_history_placement(2);
    let model = ui.get_history_rows();
    assert_eq!(model.row_count(), 1025);
    // Slint may request visible rows while an editing callback owns the workspace.
    let editing = state.borrow_mut();
    assert!(model.row_data(1024).is_some());
    drop(editing);
    assert_eq!(
        history.get_rows(),
        model,
        "floating and docked hosts share one model"
    );
    assert!(model.row_data(1024).unwrap().current);
    assert!(state.borrow().document().history_storage_bytes() > 0);
    assert!(history.get_status_text().contains("on disk"));
    assert!(
        !model
            .as_any()
            .is::<slint::VecModel<hoshi_canvas_spike::HistoryRow>>()
    );
    ui.window().take_snapshot().unwrap();
    let current = ElementHandle::find_by_accessible_label(&ui, "Rename layer · current state")
        .next()
        .expect("opening a long history must reveal its current row");
    assert!(current.absolute_position().y >= 0.);
    assert!(
        current.absolute_position().y + current.size().height <= ui.window().size().height as f32
    );
    state.borrow_mut().pan(10., 20.).unwrap();
    refresh_workspace(&ui, &state);
    assert_eq!(
        model,
        ui.get_history_rows(),
        "view changes must not recreate the list"
    );
    ui.invoke_history_jump(0);
    assert_eq!(state.borrow().document().layers()[0].name(), "Background");
    assert_eq!(model, ui.get_history_rows());
    assert!(model.row_data(1024).unwrap().redo);
    ui.invoke_history_jump(1024);
    assert_eq!(state.borrow().document().layers()[0].name(), "Layer 1023");
    ui.invoke_history_jump(123);
    assert_eq!(state.borrow().document().layers()[0].name(), "Layer 122");
    state
        .borrow_mut()
        .resize_image(512, 512, 144., Resampling::Nearest)
        .unwrap();
    refresh_workspace(&ui, &state);
    assert_eq!(state.borrow().document().redo_len(), 0);
    assert_eq!(model.row_count(), 125);
    assert_eq!(model.row_data(124).unwrap().label, "Resize");
    assert!(model.row_data(124).unwrap().current);
    state
        .borrow_mut()
        .new_image(8, 8, 96., Color::TRANSPARENT)
        .unwrap();
    refresh_workspace(&ui, &state);
    assert_eq!(model.row_count(), 1);
    state.borrow_mut().rename_layer("Second tab").unwrap();
    refresh_workspace(&ui, &state);
    assert_eq!(model.row_count(), 2);
    ui.invoke_activate_document(first as i32);
    assert_eq!(model, ui.get_history_rows());
    assert_eq!(model.row_count(), 125);
    assert_eq!(model.row_data(124).unwrap().label, "Resize");
    assert_eq!(state.borrow().document().resolution(), 144.);
    // A real scratch-file creation failure must surface through the main-window
    // error path without reborrowing an editing callback's mutable workspace.
    let directory = tempfile::tempdir().unwrap();
    let mut failed = hoshi_core::Document::new(8, 8).unwrap();
    failed
        .set_history_directory(directory.path().join("missing"))
        .unwrap();
    state.borrow_mut().replace_document(failed);
    refresh_workspace(&ui, &state);
    let before = state.borrow().document().snapshot();
    ui.invoke_pointer_down(1., 1.);
    ui.invoke_pointer_move(6., 6.);
    ui.invoke_pointer_up(6., 6.);
    let preview = state.borrow().document().active_shape();
    assert!(preview.is_some());
    ui.invoke_action(2);
    // The message is catalog copy, not the core error's developer text.
    let expected = ui.global::<Translations>().get_error_history_io();
    assert!(!expected.is_empty());
    assert_eq!(ui.get_file_error(), expected);
    assert_eq!(ui.get_status(), expected);
    assert_ne!(
        ui.get_file_error().as_str(),
        hoshi_core::Error::HistoryIo.to_string()
    );
    assert!(ui.get_modal_visible());
    assert_eq!(state.borrow().document().snapshot(), before);
    assert_eq!(state.borrow().document().active_shape(), preview);
    assert_eq!(state.borrow().document().undo_len(), 0);
    ui.invoke_dismiss_file_error();
    assert!(ui.get_file_error().is_empty());
    ui.hide().unwrap();
}
