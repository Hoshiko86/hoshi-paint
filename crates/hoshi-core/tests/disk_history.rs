use hoshi_core::{Anchor, Color, Document, Error, Shape, ShapeKind, ShapeStyle, memory_usage};

fn rectangle(color: Color) -> Shape {
    Shape {
        kind: ShapeKind::Rectangle,
        x: 1,
        y: 1,
        width: 1,
        height: 1,
        rotation_degrees: 0,
        style: ShapeStyle::Fill(color),
    }
}

#[derive(Debug, PartialEq)]
struct Expected {
    width: u32,
    height: u32,
    resolution: f64,
    selected: usize,
    layers: Vec<(String, bool, u8, Vec<Color>)>,
}
fn capture(doc: &Document) -> Expected {
    Expected {
        width: doc.width(),
        height: doc.height(),
        resolution: doc.resolution(),
        selected: doc.selected_layer(),
        layers: doc
            .layers()
            .iter()
            .map(|layer| {
                (
                    layer.name().into(),
                    layer.visible(),
                    layer.opacity(),
                    layer.pixels().to_vec(),
                )
            })
            .collect(),
    }
}
fn remember(doc: &Document, states: &mut Vec<Expected>) {
    let next = capture(doc);
    if states.last() != Some(&next) {
        states.push(next);
    }
    assert_eq!(doc.history_position(), states.len() - 1);
}

// One test function deliberately runs these checks sequentially: resident pixel
// counters are process-wide and must not race another history allocation test.
#[test]
fn disk_history_retains_deep_timelines_without_pinning_old_pixel_tiles() {
    let scratch = tempfile::tempdir().unwrap();
    {
        let mut seed = 0x1234_5678u32;
        let pixels = (0..512 * 512)
            .map(|_| {
                seed ^= seed << 13;
                seed ^= seed >> 17;
                seed ^= seed << 5;
                Color::rgba(seed as u8, (seed >> 8) as u8, (seed >> 16) as u8, 255)
            })
            .collect();
        let mut doc = Document::from_rgba(512, 512, 96., pixels).unwrap();
        doc.set_history_directory(scratch.path()).unwrap();
        let resident = memory_usage();
        for index in 0..1024 {
            doc.rename_layer(0, format!("Layer {index}")).unwrap();
        }
        assert_eq!(doc.undo_len(), 1024);
        assert_eq!(doc.history_len(), 1025);
        assert!(doc.history_step(1025).is_none());
        assert!(doc.history_storage_bytes() > 0);
        assert!(
            doc.history_storage_bytes() < 3 * 1024 * 1024,
            "metadata edits must reuse the same noisy image tiles on disk"
        );
        let generation = doc.history_generation();
        let disk_before_noop = doc.history_storage_bytes();
        doc.rename_layer(0, "Layer 1023").unwrap();
        assert_eq!(doc.history_generation(), generation);
        assert_eq!(doc.history_storage_bytes(), disk_before_noop);
        let initial = doc.composite_pixel(1, 1, false);
        let green = initial.g.wrapping_add(1);
        for index in 0..128 {
            let color = Color::rgba(index, green, 80, 255);
            doc.begin_shape(rectangle(color)).unwrap();
            doc.commit_shape().unwrap();
        }
        assert_eq!(doc.undo_len(), 1152);
        assert!(
            memory_usage() <= resident + 256 * 1024,
            "archived history must not strongly retain all 128 changed pixel tiles"
        );
        let archived_bytes = doc.history_storage_bytes();
        assert!(doc.go_to_history_step(1024).unwrap());
        assert_eq!(doc.composite_pixel(1, 1, false), initial);
        assert_eq!(doc.redo_len(), 128);
        assert_eq!(
            doc.history_storage_bytes(),
            archived_bytes,
            "undo/redo navigation should read committed archives without writing new states"
        );
        for index in 0..128 {
            assert!(doc.redo().unwrap());
            assert_eq!(
                doc.composite_pixel(1, 1, false),
                Color::rgba(index, green, 80, 255)
            );
        }
        assert!(!doc.redo().unwrap());
        assert!(doc.go_to_history_step(0).unwrap());
        assert_eq!(doc.layers()[0].name(), "Background");
        assert_eq!(doc.composite_pixel(1, 1, false), initial);
        doc.go_to_history_step(1056).unwrap();
        let before_branch = doc.history_storage_bytes();
        doc.rename_layer(0, "Branch").unwrap();
        assert_eq!(doc.redo_len(), 0);
        assert_eq!(doc.undo_len(), 1057);
        assert_eq!(doc.history_len(), 1058);
        assert!(
            doc.history_storage_bytes() < before_branch,
            "discarded redo tile blobs should be reclaimed after a successful branch"
        );
        assert_eq!(doc.go_to_history_step(1058), Err(Error::InvalidHistoryStep));
        doc.go_to_history_step(0).unwrap();
        assert_eq!(doc.layers()[0].name(), "Background");
        doc.go_to_history_step(1057).unwrap();
        assert_eq!(doc.layers()[0].name(), "Branch");
    }
    assert_eq!(
        std::fs::read_dir(scratch.path()).unwrap().count(),
        0,
        "closing the document must remove its temporary history files"
    );

    // Compare ordinary values, not retained snapshots, so these expectations do
    // not keep all source tile Arcs alive and accidentally bypass disk restoration.
    {
        let mut doc = Document::new(2, 3).unwrap();
        doc.set_history_directory(scratch.path()).unwrap();
        let mut states = vec![capture(&doc)];
        for index in 0..40 {
            let layer = doc.add_layer(format!("Ink {index}")).unwrap();
            remember(&doc, &mut states);
            doc.rename_layer(layer, format!("Draft {index}")).unwrap();
            remember(&doc, &mut states);
            doc.set_layer_properties(layer, index % 2 == 0, 100 + index as u8)
                .unwrap();
            remember(&doc, &mut states);
            doc.begin_shape(Shape {
                x: 0,
                y: 0,
                ..rectangle(Color::rgba(index as u8, 80, 200, 255))
            })
            .unwrap();
            doc.commit_shape().unwrap();
            remember(&doc, &mut states);
            doc.set_resolution(100. + f64::from(index)).unwrap();
            remember(&doc, &mut states);
            doc.resize_canvas(
                2 + index % 2,
                3 + index % 2,
                Anchor { column: 1, row: 1 },
                Color::TRANSPARENT,
            )
            .unwrap();
            remember(&doc, &mut states);
            doc.delete_layer(layer).unwrap();
            remember(&doc, &mut states);
        }
        assert!(states.len() > 250);
        for index in (0..states.len() - 1).rev() {
            assert!(doc.undo().unwrap());
            assert_eq!(capture(&doc), states[index], "undo state {index}");
        }
        assert!(!doc.undo().unwrap());
        for (index, expected) in states.iter().enumerate().skip(1) {
            assert!(doc.redo().unwrap());
            assert_eq!(&capture(&doc), expected, "redo state {index}");
        }
        assert!(!doc.redo().unwrap());
    }
    assert_eq!(std::fs::read_dir(scratch.path()).unwrap().count(), 0);

    // Invalid scratch storage fails at the edit boundary, preserving the image,
    // cursor, generation and editable shape. A valid directory allows retry.
    {
        let blocked = scratch.path().join("not-a-directory");
        std::fs::write(&blocked, b"blocker").unwrap();
        let mut doc = Document::new(4, 4).unwrap();
        doc.set_history_directory(&blocked).unwrap();
        let before = capture(&doc);
        assert_eq!(doc.rename_layer(0, "Cannot persist"), Err(Error::HistoryIo));
        assert_eq!(capture(&doc), before);
        assert_eq!(
            (
                doc.history_len(),
                doc.history_position(),
                doc.history_generation()
            ),
            (1, 0, 0)
        );
        let preview = rectangle(Color::rgba(255, 0, 0, 255));
        doc.begin_shape(preview).unwrap();
        assert_eq!(doc.commit_shape(), Err(Error::HistoryIo));
        assert_eq!(doc.active_shape(), Some(preview));
        assert_eq!(capture(&doc), before);
        let valid = scratch.path().join("valid");
        std::fs::create_dir(&valid).unwrap();
        doc.set_history_directory(&valid).unwrap();
        doc.commit_shape().unwrap();
        assert_eq!(doc.undo_len(), 1);
        doc.undo().unwrap();
        assert_eq!(capture(&doc), before);
        doc.redo().unwrap();
        assert_eq!(doc.composite_pixel(1, 1, false).r, 255);
    }
    assert_eq!(
        std::fs::read_dir(scratch.path().join("valid"))
            .unwrap()
            .count(),
        0
    );
}
