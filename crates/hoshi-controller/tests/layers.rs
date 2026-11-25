use hoshi_controller::{Controller, ControllerError, HistoryLabel, Tool};
use hoshi_core::{Error, MAX_LAYERS};

fn shape(c: &mut Controller) {
    c.pointer_down(2.0, 3.0).unwrap();
    c.pointer_move(10.0, 12.0).unwrap();
    c.pointer_up().unwrap();
    assert!(c.document().active_shape().is_some());
}

#[test]
fn adding_commits_shape_on_old_layer_and_undo_restores_empty_document() {
    let mut c = Controller::new(16, 16).unwrap();
    shape(&mut c);
    let preview = c.composite();
    assert_eq!(c.add_layer("Ink").unwrap(), 1);
    assert_eq!(c.document().selected_layer(), 1);
    assert_eq!(c.document().layers()[0].pixels(), preview);
    assert!(c.document().layers()[1].pixels().iter().all(|p| p.a == 0));
    assert_eq!(c.document().layers()[1].name(), "Ink");
    assert_eq!(
        c.history_steps()
            .iter()
            .skip(1)
            .map(|s| s.label)
            .collect::<Vec<_>>(),
        [HistoryLabel::Rectangle, HistoryLabel::AddLayer]
    );
    c.undo().unwrap();
    assert_eq!(c.document().layers().len(), 1);
    assert_eq!(c.composite(), preview);
    c.undo().unwrap();
    assert!(c.composite().iter().all(|p| p.a == 0));
    c.redo().unwrap();
    c.redo().unwrap();
    assert_eq!(c.document().selected_layer(), 1);
    assert_eq!(c.composite(), preview);
}

#[test]
fn selecting_finishes_mid_drag_brush_stroke_on_old_layer() {
    let mut c = Controller::new(16, 16).unwrap();
    c.add_layer("Ink").unwrap();
    c.set_tool(Tool::Brush).unwrap();
    c.pointer_down(3.0, 4.0).unwrap();
    c.pointer_move(10.0, 11.0).unwrap();
    assert!(c.is_dragging());
    c.select_layer(0).unwrap();
    assert!(!c.is_dragging());
    assert_eq!(c.document().selected_layer(), 0);
    assert!(c.document().layers()[1].pixels().iter().any(|p| p.a != 0));
    assert!(c.document().layers()[0].pixels().iter().all(|p| p.a == 0));
    assert_eq!(c.document().undo_len(), 2);
    c.undo().unwrap();
    assert!(c.composite().iter().all(|p| p.a == 0));
}

#[test]
fn invalid_commands_preserve_live_shape_and_history() {
    let mut c = Controller::new(16, 16).unwrap();
    shape(&mut c);
    let before = c.document().active_shape();
    let errors = [
        (c.delete_layer(), ControllerError::Core(Error::LastLayer)),
        (
            c.select_layer(99),
            ControllerError::Core(Error::InvalidLayer),
        ),
        (
            c.set_layer_visible(99, false),
            ControllerError::Core(Error::InvalidLayer),
        ),
        (
            c.set_layer_properties(99, false, 128),
            ControllerError::Core(Error::InvalidLayer),
        ),
        (
            c.add_layer("  ").map(|_| ()),
            ControllerError::InvalidLayerName,
        ),
        (
            c.add_layer(&"絵".repeat(129)).map(|_| ()),
            ControllerError::InvalidLayerName,
        ),
    ];
    for (actual, expected) in errors {
        assert_eq!(actual, Err(expected));
    }
    assert_eq!(c.document().active_shape(), before);
    assert_eq!(c.document().undo_len(), 0);
    c.cancel();
    for _ in 1..MAX_LAYERS {
        c.add_layer("Ink").unwrap();
    }
    shape(&mut c);
    let before = c.document().active_shape();
    let undo = c.document().undo_len();
    assert_eq!(
        c.add_layer("Ink"),
        Err(ControllerError::Core(Error::LayerLimit))
    );
    assert_eq!(c.document().active_shape(), before);
    assert_eq!(c.document().undo_len(), undo);
}

#[test]
fn no_ops_preserve_live_shape_and_history() {
    let mut c = Controller::new(16, 16).unwrap();
    shape(&mut c);
    let before = c.document().active_shape();
    c.move_layer_up().unwrap();
    c.move_layer_down().unwrap();
    c.select_layer(0).unwrap();
    c.set_layer_visible(0, true).unwrap();
    c.set_layer_properties(0, true, 255).unwrap();
    assert_eq!(c.document().active_shape(), before);
    assert_eq!(c.document().undo_len(), 0);
}

#[test]
fn reordering_visibility_properties_and_deletion_each_undo_and_redo_once() {
    let mut c = Controller::new(16, 16).unwrap();
    c.add_layer("Ink").unwrap();
    c.move_layer_down().unwrap();
    assert_eq!(c.document().selected_layer(), 0);
    assert_eq!(c.document().layers()[0].name(), "Ink");
    assert_eq!(c.document().undo_len(), 2);
    c.undo().unwrap();
    assert_eq!(c.document().selected_layer(), 1);
    c.redo().unwrap();
    assert_eq!(c.document().selected_layer(), 0);
    c.move_layer_up().unwrap();
    assert_eq!(c.document().selected_layer(), 1);
    c.undo().unwrap();
    assert_eq!(c.document().selected_layer(), 0);
    c.redo().unwrap();
    assert_eq!(c.document().selected_layer(), 1);
    c.set_layer_visible(0, false).unwrap();
    assert_eq!(c.document().selected_layer(), 1);
    assert!(!c.document().layers()[0].visible());
    assert_eq!(c.document().undo_len(), 4);
    c.undo().unwrap();
    assert!(c.document().layers()[0].visible());
    c.redo().unwrap();
    assert!(!c.document().layers()[0].visible());
    c.set_layer_properties(1, false, 128).unwrap();
    assert_eq!(c.document().undo_len(), 5);
    assert_eq!(
        c.history_steps().last().unwrap().label,
        HistoryLabel::LayerProperties
    );
    c.undo().unwrap();
    assert!(c.document().layers()[1].visible());
    assert_eq!(c.document().layers()[1].opacity(), 255);
    c.redo().unwrap();
    assert!(!c.document().layers()[1].visible());
    assert_eq!(c.document().layers()[1].opacity(), 128);
    c.delete_layer().unwrap();
    assert_eq!(c.document().layers().len(), 1);
    assert_eq!(c.document().undo_len(), 6);
    c.undo().unwrap();
    assert_eq!(c.document().layers()[1].name(), "Ink");
    c.redo().unwrap();
    assert_eq!(c.document().layers().len(), 1);
}

#[test]
fn changing_layer_commands_commit_live_shape_before_their_own_history_entry() {
    type Command = fn(&mut Controller) -> Result<(), ControllerError>;
    for (command, label, selected) in [
        (
            Controller::delete_layer as Command,
            HistoryLabel::DeleteLayer,
            1,
        ),
        (Controller::move_layer_down, HistoryLabel::ReorderLayer, 1),
        (Controller::move_layer_up, HistoryLabel::ReorderLayer, 0),
        (
            |c| c.set_layer_visible(0, false),
            HistoryLabel::LayerProperties,
            1,
        ),
        (
            |c| c.set_layer_properties(0, false, 128),
            HistoryLabel::LayerProperties,
            1,
        ),
    ] {
        let mut c = Controller::new(16, 16).unwrap();
        c.add_layer("Ink").unwrap();
        c.select_layer(selected).unwrap();
        shape(&mut c);
        command(&mut c).unwrap();
        assert!(c.document().active_shape().is_none());
        assert_eq!(c.document().undo_len(), 3);
        assert_eq!(c.history_steps()[2].label, HistoryLabel::Rectangle);
        assert_eq!(c.history_steps()[3].label, label);
    }
}
