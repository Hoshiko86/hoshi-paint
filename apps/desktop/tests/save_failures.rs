use hoshi_core::{
    Color, Document, MemoryReservation, PixelStore, ProjectError, RenderOptions, Shape, ShapeKind,
    ShapeStyle, memory_budget, memory_usage,
};
use hoshi_desktop::{
    files::{self, FileError, FileFormat},
    workspace::{Workspace, WorkspaceError},
};
use std::{ffi::OsString, fs, path::Path, process::Command};

/// Copied from crates/hoshi-controller/tests/support/mod.rs: isolate the global memory budget.
fn run_isolated(test_name: &str, body: impl FnOnce()) {
    const ISOLATED_TEST: &str = "HOSHI_ISOLATED_TEST";
    if std::env::var(ISOLATED_TEST).as_deref() == Ok(test_name) {
        body();
        return;
    }
    let exe = std::env::current_exe().expect("test binary path");
    let output = Command::new(exe)
        .args(["--exact", test_name, "--test-threads=1"])
        .env(ISOLATED_TEST, test_name)
        .output()
        .expect("spawn isolated test");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "isolated test `{test_name}` failed:\n{stdout}\n{stderr}"
    );
    assert!(
        stdout.lines().any(|line| line.trim() == "running 1 test"),
        "isolated run of `{test_name}` ran no test; is the name wrong?\n{stdout}"
    );
}

fn entries(dir: &Path) -> Vec<OsString> {
    let mut names: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    names.sort();
    names
}

fn layered() -> Document {
    let pixels = (0..64 * 48)
        .map(|i| {
            Color::rgba(
                (i % 251) as u8,
                (i % 197) as u8,
                83,
                if i % 7 == 0 { 0 } else { 255 },
            )
        })
        .collect();
    let mut document =
        Document::from_pixels(64, 48, 96., PixelStore::from_vec(64, 48, pixels).unwrap()).unwrap();
    document.add_layer("Ellipse").unwrap();
    let mut shape = Shape {
        kind: ShapeKind::Ellipse,
        x: 7,
        y: 5,
        width: 39,
        height: 27,
        rotation_degrees: 23,
        flip_horizontal: false,
        flip_vertical: false,
        rendering: RenderOptions {
            antialias: true,
            ..Default::default()
        },
        style: ShapeStyle::Fill(Color::rgba(31, 173, 219, 200)),
    };
    document.begin_shape(shape).unwrap();
    document.commit_shape().unwrap();
    document.set_layer_properties(1, true, 128).unwrap();
    document.add_layer("Hidden red").unwrap();
    shape.kind = ShapeKind::Rectangle;
    shape.style = ShapeStyle::Fill(Color::rgba(255, 0, 0, 255));
    document.begin_shape(shape).unwrap();
    document.commit_shape().unwrap();
    document.set_layer_properties(2, false, 255).unwrap();
    document
}

#[test]
fn png_save_failing_after_the_temp_file_keeps_destination_tab_and_history() {
    run_isolated(
        "save_failures::png_save_failing_after_the_temp_file_keeps_destination_tab_and_history",
        || {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("existing.png");
            fs::write(&path, b"original").unwrap();
            let mut workspace = Workspace::default();
            workspace.rename_layer("One").unwrap();
            workspace.rename_layer("Two").unwrap();
            workspace.undo().unwrap();
            let reservation = MemoryReservation::new(memory_budget() - memory_usage()).unwrap();
            assert!(matches!(
                workspace.save_to(&path, 95, false),
                Err(WorkspaceError::File(FileError::MemoryLimit))
            ));
            assert_eq!(fs::read(&path).unwrap(), b"original");
            assert_eq!(entries(dir.path()), [OsString::from("existing.png")]);
            assert!(workspace.active_tab().unwrap().dirty());
            assert_eq!(workspace.document().undo_len(), 1);
            assert_eq!(workspace.document().redo_len(), 1);
            assert_eq!(workspace.document().layers()[0].name(), "One");
            drop(reservation);
            workspace.save_to(&path, 95, false).unwrap();
            assert!(!workspace.active_tab().unwrap().dirty());
            assert_eq!(
                files::load(&path, "Background").unwrap().layers()[0].pixels(),
                workspace.document().layers()[0].pixels()
            );
        },
    );
}

#[test]
fn flatten_save_failing_after_the_temp_file_keeps_the_undoable_flatten() {
    run_isolated(
        "save_failures::flatten_save_failing_after_the_temp_file_keeps_the_undoable_flatten",
        || {
            let dir = tempfile::tempdir().unwrap();
            let project = dir.path().join("layers.hoshi");
            files::save_atomic(&project, &layered(), FileFormat::Project, 95).unwrap();
            let mut workspace = Workspace::default();
            workspace.open_path(&project).unwrap();
            let composite = workspace.document().composite(false);
            let undo_len = workspace.document().undo_len();
            assert_eq!(workspace.document().layers().len(), 3);
            let path = dir.path().join("existing.png");
            fs::write(&path, b"original").unwrap();
            let reservation =
                MemoryReservation::new(memory_budget() - memory_usage() - 512 * 1024).unwrap();
            let result = workspace.save_to(&path, 95, true);
            assert!(
                matches!(result, Err(WorkspaceError::File(FileError::MemoryLimit))),
                "expected post-temp PNG memory failure, got {result:?}"
            );
            assert_eq!(fs::read(&path).unwrap(), b"original");
            assert_eq!(
                entries(dir.path()),
                [
                    OsString::from("existing.png"),
                    OsString::from("layers.hoshi")
                ]
            );
            assert_eq!(workspace.document().layers().len(), 1);
            assert_eq!(workspace.document().composite(false), composite);
            assert_eq!(workspace.document().undo_len(), undo_len + 1);
            assert!(workspace.active_tab().unwrap().dirty());
            drop(reservation);
            workspace.undo().unwrap();
            let layers = workspace.document().layers();
            assert_eq!(layers.len(), 3);
            assert_eq!(
                layers
                    .iter()
                    .map(|layer| (layer.visible(), layer.opacity()))
                    .collect::<Vec<_>>(),
                [(true, 255), (true, 128), (false, 255)]
            );
        },
    );
}

#[test]
fn read_only_destination_is_reported_and_left_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let document = Document::new_with_background(8, 8, 96., Color::rgba(32, 61, 97, 255)).unwrap();
    let mut expected = Vec::new();
    for (extension, format) in [
        ("hoshi", FileFormat::Project),
        ("png", FileFormat::Png),
        ("jpg", FileFormat::Jpeg),
    ] {
        let path = dir.path().join(format!("existing.{extension}"));
        fs::write(&path, b"original").unwrap();
        let original = fs::metadata(&path).unwrap().permissions();
        let mut readonly = original.clone();
        readonly.set_readonly(true);
        fs::set_permissions(&path, readonly).unwrap();
        let result = files::save_atomic(&path, &document, format, 95);
        fs::set_permissions(&path, original).unwrap();
        assert!(matches!(result, Err(FileError::ReadOnlyDestination)));
        assert_eq!(fs::read(&path).unwrap(), b"original");
        expected.push(path.file_name().unwrap().to_os_string());
        expected.sort();
        assert_eq!(entries(dir.path()), expected);
    }
    let path = dir.path().join("existing.png");
    let mut workspace = Workspace::default();
    workspace.rename_layer("One").unwrap();
    workspace.rename_layer("Two").unwrap();
    workspace.undo().unwrap();
    let history = (
        workspace.document().undo_len(),
        workspace.document().redo_len(),
    );
    let original = fs::metadata(&path).unwrap().permissions();
    let mut readonly = original.clone();
    readonly.set_readonly(true);
    fs::set_permissions(&path, readonly).unwrap();
    let result = workspace.save_to(&path, 95, false);
    fs::set_permissions(&path, original).unwrap();
    assert!(matches!(
        result,
        Err(WorkspaceError::File(FileError::ReadOnlyDestination))
    ));
    assert!(workspace.active_tab().unwrap().dirty());
    assert_eq!(
        (
            workspace.document().undo_len(),
            workspace.document().redo_len()
        ),
        history
    );
    assert_eq!(fs::read(&path).unwrap(), b"original");
    assert_eq!(entries(dir.path()), expected);
}

// Windows ignores the read-only attribute on folders, so this case is Unix-only.
#[cfg(unix)]
#[test]
fn read_only_folder_fails_before_replacing_the_file() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("readonly");
    fs::create_dir(&folder).unwrap();
    let path = folder.join("a.png");
    fs::write(&path, b"original").unwrap();
    let document = Document::new(8, 8).unwrap();
    fs::set_permissions(&folder, fs::Permissions::from_mode(0o555)).unwrap();
    let probe = folder.join("probe");
    if fs::File::create(&probe).is_ok() {
        fs::remove_file(probe).unwrap();
        fs::set_permissions(&folder, fs::Permissions::from_mode(0o755)).unwrap();
        eprintln!(
            "skipped read-only folder assertion: probe succeeded (root or permission bypass)"
        );
        return;
    }
    let result = files::save_atomic(&path, &document, FileFormat::Png, 95);
    fs::set_permissions(&folder, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(
        matches!(result, Err(FileError::Io(error)) if error.kind() == std::io::ErrorKind::PermissionDenied)
    );
    assert_eq!(fs::read(&path).unwrap(), b"original");
    assert_eq!(entries(&folder), [OsString::from("a.png")]);
}

#[test]
fn project_read_over_the_memory_budget_fails_cleanly() {
    run_isolated(
        "save_failures::project_read_over_the_memory_budget_fails_cleanly",
        || {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("image.hoshi");
            let pixels = (0..512 * 512)
                .map(|i| Color::rgba((i % 251) as u8, ((i / 512) % 253) as u8, 83, 255))
                .collect();
            let document = Document::from_pixels(
                512,
                512,
                96.,
                PixelStore::from_vec(512, 512, pixels).unwrap(),
            )
            .unwrap();
            files::save_atomic(&path, &document, FileFormat::Project, 95).unwrap();
            let usage = memory_usage();
            let reservation = MemoryReservation::new(memory_budget() - usage).unwrap();
            assert!(matches!(
                files::load(&path, "Background"),
                Err(FileError::InvalidProject(ProjectError::MemoryLimit))
            ));
            drop(reservation);
            assert_eq!(memory_usage(), usage);
            assert_eq!(
                files::load(&path, "Background").unwrap().snapshot(),
                document.snapshot()
            );
        },
    );
}
