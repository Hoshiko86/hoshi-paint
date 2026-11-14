use hoshi_core::{Color, Document, Error, HistoryLabel, Shape, ShapeKind, ShapeStyle};

fn rectangle(color: Color) -> Shape {
    Shape {
        kind: ShapeKind::Rectangle,
        x: 0,
        y: 0,
        width: 2,
        height: 2,
        rotation_degrees: 0,
        flip_horizontal: false,
        flip_vertical: false,
        rendering: hoshi_core::RenderOptions {
            antialias: false,
            ..Default::default()
        },
        style: ShapeStyle::Fill(color),
    }
}

#[test]
fn chronological_history_tracks_current_redo_and_navigation() {
    let mut doc = Document::new(4, 4).unwrap();
    doc.add_layer("Ink").unwrap();
    doc.rename_layer(1, "Sketch").unwrap();
    doc.set_layer_properties(1, false, 128).unwrap();
    let labels = || {
        doc.history_steps()
            .into_iter()
            .map(|step| step.label)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        labels(),
        [
            HistoryLabel::OldestAvailableState,
            HistoryLabel::AddLayer,
            HistoryLabel::RenameLayer,
            HistoryLabel::LayerProperties
        ]
    );
    assert_eq!(doc.history_position(), 3);
    assert!(doc.history_steps()[3].is_current);
    doc.undo().unwrap();
    let steps = doc.history_steps();
    assert!(steps[2].is_current && steps[3].is_redo);
    assert!(!steps[1].is_redo);
    assert!(doc.go_to_history_step(1).unwrap());
    assert_eq!(doc.layers()[1].name(), "Ink");
    assert!(doc.layers()[1].visible());
    assert_eq!(doc.redo_len(), 2);
    assert!(doc.go_to_history_step(3).unwrap());
    assert_eq!(doc.layers()[1].name(), "Sketch");
    assert!(!doc.layers()[1].visible());
    assert_eq!(doc.layers()[1].opacity(), 128);
    assert!(!doc.go_to_history_step(3).unwrap());
    assert_eq!(doc.go_to_history_step(4), Err(Error::InvalidHistoryStep));
    assert_eq!(doc.history_position(), 3);
}

#[test]
fn all_history_remains_available_and_branch_discards_only_redo() {
    let mut doc = Document::new(1, 1).unwrap();
    for index in 0..25 {
        doc.rename_layer(0, format!("Layer {index}")).unwrap();
    }
    assert_eq!(doc.history_steps().len(), 26);
    assert_eq!(doc.history_position(), 25);
    assert_eq!(
        doc.history_steps()[0].label,
        HistoryLabel::OldestAvailableState
    );
    doc.go_to_history_step(0).unwrap();
    assert_eq!(doc.layers()[0].name(), "Background");
    assert!(doc.history_steps()[0].is_current);
    assert!(doc.history_steps()[1..].iter().all(|step| step.is_redo));
    doc.go_to_history_step(25).unwrap();
    assert_eq!(doc.layers()[0].name(), "Layer 24");
    doc.go_to_history_step(10).unwrap();
    doc.rename_layer(0, "Branch").unwrap();
    assert_eq!(doc.redo_len(), 0);
    assert_eq!(doc.history_steps().len(), 12);
    assert_eq!(
        doc.history_steps().last().unwrap().label,
        HistoryLabel::RenameLayer
    );
    assert!(doc.history_steps().last().unwrap().is_current);
    assert_eq!(doc.go_to_history_step(12), Err(Error::InvalidHistoryStep));
}

#[test]
fn previews_noops_and_toolbar_undo_keep_existing_history_contract() {
    let mut doc = Document::new(4, 4).unwrap();
    doc.rename_layer(0, "Background").unwrap();
    doc.begin_shape(rectangle(Color::TRANSPARENT)).unwrap();
    assert!(!doc.commit_shape().unwrap());
    assert_eq!(doc.history_steps().len(), 1);
    let shape = rectangle(Color::rgba(255, 0, 0, 255));
    doc.begin_shape(shape).unwrap();
    assert_eq!(doc.go_to_history_step(0), Err(Error::ActiveShape));
    assert_eq!(doc.active_shape(), Some(shape));
    assert_eq!(doc.history_steps().len(), 1);
    assert!(
        doc.undo().unwrap(),
        "toolbar Undo still commits then undoes the preview"
    );
    assert_eq!((doc.undo_len(), doc.redo_len()), (0, 1));
    assert_eq!(doc.history_steps()[1].label, HistoryLabel::Rectangle);
    assert!(doc.history_steps()[1].is_redo);
    doc.go_to_history_step(1).unwrap();
    assert_eq!(doc.composite(false)[0], Color::rgba(255, 0, 0, 255));
}
