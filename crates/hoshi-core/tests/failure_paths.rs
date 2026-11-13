use hoshi_core::{
    Color, Document, DocumentSnapshot, Error, MAX_LAYERS, Shape, ShapeKind, ShapeStyle,
};

#[derive(Debug, PartialEq)]
struct DocumentState {
    snapshot: DocumentSnapshot,
    active_shape: Option<Shape>,
    history_position: usize,
    history_len: usize,
    undo_len: usize,
    redo_len: usize,
}

impl DocumentState {
    fn capture(document: &Document) -> Self {
        Self {
            snapshot: document.snapshot(),
            active_shape: document.active_shape(),
            history_position: document.history_position(),
            history_len: document.history_len(),
            undo_len: document.undo_len(),
            redo_len: document.redo_len(),
        }
    }
}

fn rectangle(width: u32, height: u32) -> Shape {
    Shape {
        kind: ShapeKind::Rectangle,
        x: 1,
        y: 1,
        width,
        height,
        rotation_degrees: 0,
        flip_horizontal: false,
        flip_vertical: false,
        rendering: Default::default(),
        style: ShapeStyle::Fill(Color::rgba(240, 40, 80, 255)),
    }
}

fn assert_unchanged<T: std::fmt::Debug + PartialEq>(
    document: &Document,
    before: DocumentState,
    result: Result<T, Error>,
    expected: Error,
) {
    assert_eq!(result, Err(expected));
    assert_eq!(DocumentState::capture(document), before);
}

#[test]
fn invalid_layer_operations_are_atomic() {
    let mut document = Document::new(8, 8).unwrap();
    let invalid = document.layers().len();

    let before = DocumentState::capture(&document);
    let result = document.select_layer(invalid);
    assert_unchanged(&document, before, result, Error::InvalidLayer);

    let before = DocumentState::capture(&document);
    let result = document.delete_layer(invalid);
    assert_unchanged(&document, before, result, Error::InvalidLayer);

    let before = DocumentState::capture(&document);
    let result = document.rename_layer(invalid, "unreachable");
    assert_unchanged(&document, before, result, Error::InvalidLayer);

    let before = DocumentState::capture(&document);
    let result = document.set_layer_properties(invalid, false, 0);
    assert_unchanged(&document, before, result, Error::InvalidLayer);

    let before = DocumentState::capture(&document);
    let result = document.move_layer(0, invalid);
    assert_unchanged(&document, before, result, Error::InvalidLayer);
}

#[test]
fn active_preview_rejects_layer_and_history_operations_atomically() {
    let mut document = Document::new(8, 8).unwrap();
    document.begin_shape(rectangle(4, 4)).unwrap();

    let before = DocumentState::capture(&document);
    let result = document.add_layer("blocked");
    assert_unchanged(&document, before, result, Error::ActiveShape);

    let before = DocumentState::capture(&document);
    let result = document.rename_layer(0, "blocked");
    assert_unchanged(&document, before, result, Error::ActiveShape);

    let before = DocumentState::capture(&document);
    let result = document.go_to_history_step(0);
    assert_unchanged(&document, before, result, Error::ActiveShape);

    let before = DocumentState::capture(&document);
    let result = document.redo();
    assert_unchanged(&document, before, result, Error::ActiveShape);
}

#[test]
fn shape_and_history_validation_fail_without_side_effects() {
    let mut document = Document::new(8, 8).unwrap();

    let before = DocumentState::capture(&document);
    let result = document.adjust_shape(rectangle(2, 2));
    assert_unchanged(&document, before, result, Error::NoActiveShape);

    let before = DocumentState::capture(&document);
    let result = document.begin_shape(rectangle(0, 2));
    assert_unchanged(&document, before, result, Error::InvalidShape);

    let before = DocumentState::capture(&document);
    let result = document.go_to_history_step(1);
    assert_unchanged(&document, before, result, Error::InvalidHistoryStep);

    let before = DocumentState::capture(&document);
    let result = document.delete_layer(0);
    assert_unchanged(&document, before, result, Error::LastLayer);
}

#[test]
fn layer_limit_failure_preserves_document_and_history() {
    let mut document = Document::new(1, 1).unwrap();
    for index in 1..MAX_LAYERS {
        document.add_layer(format!("Layer {index}")).unwrap();
    }
    let before = DocumentState::capture(&document);
    let result = document.add_layer("one too many");
    assert_unchanged(&document, before, result, Error::LayerLimit);
}

#[test]
fn history_archive_failure_rolls_back_the_edit() {
    let mut document = Document::new(2, 2).unwrap();
    let missing =
        std::env::temp_dir().join(format!("hoshi-missing-history-{}", std::process::id()));
    document.set_history_directory(&missing).unwrap();
    let before = DocumentState::capture(&document);
    let result = document.rename_layer(0, "must roll back");
    assert_unchanged(&document, before, result, Error::HistoryIo);
}

#[test]
fn history_directory_cannot_change_after_an_edit() {
    let mut document = Document::new(2, 2).unwrap();
    document.rename_layer(0, "Edited").unwrap();
    let before = DocumentState::capture(&document);
    let result = document.set_history_directory(std::env::temp_dir());
    assert_unchanged(&document, before, result, Error::InvalidHistoryStep);
}

#[test]
fn public_error_messages_and_history_metadata_are_observable() {
    assert!(Error::HistoryIo.to_string().contains("undo history"));
    assert!(Error::HistoryCorrupt.to_string().contains("undo state"));
    assert!(Error::MemoryLimit.to_string().contains("memory budget"));
    assert_eq!(Error::InvalidLayer.to_string(), "InvalidLayer");

    let mut document = Document::new(2, 2).unwrap();
    assert_eq!(document.history_structure_generation(), 0);
    assert!(!document.history_cleanup_pending());
    assert_eq!(document.history_storage_bytes(), 0);
    assert_eq!(
        document.composite_pixel(u32::MAX, u32::MAX, false),
        Color::TRANSPARENT
    );

    document.rename_layer(0, "First").unwrap();
    document.rename_layer(0, "Second").unwrap();
    document.undo().unwrap();
    let generation = document.history_structure_generation();
    document.rename_layer(0, "Branched").unwrap();
    assert_eq!(document.history_structure_generation(), generation + 1);
    assert!(!document.history_cleanup_pending());
    assert!(document.history_storage_bytes() > 0);
}

#[test]
fn beginning_a_new_shape_commits_the_existing_preview_once() {
    let mut document = Document::new(8, 8).unwrap();
    document.begin_shape(rectangle(2, 2)).unwrap();
    let replacement = Shape {
        x: 4,
        y: 4,
        ..rectangle(2, 2)
    };
    document.begin_shape(replacement).unwrap();
    assert_eq!(document.active_shape(), Some(replacement));
    assert_eq!(document.undo_len(), 1);
}

#[test]
fn transparency_fast_paths_cover_empty_and_opaque_active_documents() {
    let mut document = Document::new(8, 8).unwrap();
    assert!(document.has_transparency_fast());

    let mut opaque = rectangle(8, 8);
    opaque.x = 0;
    opaque.y = 0;
    opaque.style = ShapeStyle::Outline {
        color: Color::rgba(20, 40, 60, 255),
        width: 4.0,
    };
    document.begin_shape(opaque).unwrap();
    assert!(!document.has_transparency_fast());
}
