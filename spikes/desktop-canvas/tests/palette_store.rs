use hoshi_canvas_spike::palette_store::PaletteLibrary;
use hoshi_controller::{Color, palette::Palette};
use std::fs;

const COLOR: Color = Color::rgba(12, 34, 56, 78);
#[test]
fn unsaved_work_and_named_baselines_survive_restarts_without_overwriting_saved_files() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("settings");
    let external = directory.path().join("Favorites.txt");
    let mut library = PaletteLibrary::open(Some(root.clone())).unwrap();
    library.replace(0, COLOR).unwrap();
    assert!(library.dirty());
    library.save_as(&external).unwrap();
    assert!(!library.dirty());
    assert_eq!(library.selected_index(), 1);
    let saved = fs::read(&external).unwrap();
    library.replace(0, Color::rgba(90, 80, 70, 60)).unwrap();
    assert_eq!(
        fs::read(&external).unwrap(),
        saved,
        "editing a saved palette must not overwrite it"
    );
    let mut restored = PaletteLibrary::open(Some(root.clone())).unwrap();
    assert_eq!(restored.current(), library.current());
    assert!(restored.dirty());
    assert_eq!(restored.selected_index(), 1);
    restored.select(1).unwrap();
    assert_eq!(restored.current().colors()[0], COLOR);
    assert!(!restored.dirty());
    restored.reset().unwrap();
    assert_eq!(restored.selected_index(), 0);
    assert_eq!(restored.current(), &Palette::default());
    assert!(external.is_file(), "reset must not delete custom palettes");
    assert_eq!(
        PaletteLibrary::open(Some(root)).unwrap().current(),
        &Palette::default()
    );
}
#[test]
fn import_and_export_preserve_argb_and_do_not_silently_save_working_changes() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("Default.txt");
    let output = directory.path().join("Copy.txt");
    fs::write(&source, "; Imported\n4E0C2238\n").unwrap();
    let mut library = PaletteLibrary::open(None).unwrap();
    library.import(&source).unwrap();
    assert_eq!(library.current().colors()[0], COLOR);
    assert_eq!(
        library.selected_index(),
        -1,
        "an imported file named Default is not the built-in palette"
    );
    let before = (
        library.current().clone(),
        library.dirty(),
        library.title().to_string(),
        library.selected_index(),
    );
    library.export(&output).unwrap();
    let exported = Palette::parse_paint_net(&fs::read_to_string(&output).unwrap()).unwrap();
    assert_eq!(&exported, library.current());
    assert_eq!(
        (
            library.current().clone(),
            library.dirty(),
            library.title().to_string(),
            library.selected_index()
        ),
        before
    );
    library.replace(0, Color::rgba(1, 2, 3, 4)).unwrap();
    assert_eq!(
        fs::read_to_string(source).unwrap(),
        "; Imported\n4E0C2238\n"
    );
}
#[test]
fn refresh_discovers_files_and_invalid_selection_preserves_active_palette_and_session() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("config");
    let mut library = PaletteLibrary::open(Some(root.clone())).unwrap();
    let folder = library.folder().unwrap();
    assert!(folder.is_dir());
    fs::write(folder.join("zebra.TXT"), "FF112233\n").unwrap();
    fs::write(folder.join("Amber.txt"), "bad\n").unwrap();
    library.refresh().unwrap();
    assert_eq!(library.entries().len(), 2);
    assert_eq!(library.entries()[0].name, "Amber");
    library.replace(0, COLOR).unwrap();
    let state = fs::read(root.join("palette-state.json")).unwrap();
    let before = library.current().clone();
    assert!(library.select(1).is_err());
    assert_eq!(library.current(), &before);
    assert_eq!(fs::read(root.join("palette-state.json")).unwrap(), state);
    library.select(2).unwrap();
    assert_eq!(library.current().colors()[0], Color::rgba(17, 34, 51, 255));
}
#[test]
fn failed_loads_and_saves_preserve_state_and_existing_destinations() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("config");
    let mut library = PaletteLibrary::open(Some(root.clone())).unwrap();
    library.replace(0, COLOR).unwrap();
    let before = library.current().clone();
    assert!(
        library
            .import(&directory.path().join("missing.txt"))
            .is_err()
    );
    assert_eq!(library.current(), &before);
    assert!(
        library
            .save_as(&directory.path().join("missing/Palette.txt"))
            .is_err()
    );
    assert_eq!(library.current(), &before);
    let destination = directory.path().join("Existing.txt");
    fs::write(&destination, "FFABCDEF\n").unwrap();
    let path = root.join("palette-state.json");
    let permissions = fs::metadata(&path).unwrap().permissions();
    let mut readonly = permissions.clone();
    readonly.set_readonly(true);
    fs::set_permissions(&path, readonly).unwrap();
    assert!(library.replace(0, Color::rgba(255, 0, 0, 255)).is_err());
    assert_eq!(library.current(), &before);
    assert!(library.save_as(&destination).is_err());
    assert_eq!(fs::read_to_string(&destination).unwrap(), "FFABCDEF\n");
    assert_eq!(library.current(), &before);
    assert!(library.dirty());
    fs::set_permissions(&path, permissions).unwrap();
    assert!(
        library.export(&root.join("./palette-state.json")).is_err(),
        "exports must not overwrite internal settings"
    );
}
#[test]
fn corrupt_session_is_reported_without_replacing_the_file() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("palette-state.json");
    fs::write(&path, "not JSON").unwrap();
    assert!(PaletteLibrary::open(Some(directory.path().to_path_buf())).is_err());
    assert_eq!(fs::read_to_string(path).unwrap(), "not JSON");
}

#[test]
fn oversized_import_preserves_work_and_duplicate_names_remain_distinguishable() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("settings");
    let mut library = PaletteLibrary::open(Some(root)).unwrap();
    let before = library.current().clone();
    let large = temporary.path().join("large.txt");
    fs::write(&large, vec![b' '; 1024 * 1024 + 1]).unwrap();
    assert!(library.import(&large).is_err());
    assert_eq!(library.current(), &before);

    let first = temporary.path().join("first");
    let second = temporary.path().join("second");
    fs::create_dir(&first).unwrap();
    fs::create_dir(&second).unwrap();
    library.save_as(&first.join("Favorites.txt")).unwrap();
    library.save_as(&second.join("favorites.txt")).unwrap();
    assert_eq!(library.entries().len(), 2);
    assert_ne!(library.entries()[0].name, library.entries()[1].name);
    let order = library.entries().to_vec();
    library.refresh().unwrap();
    assert_eq!(library.entries(), order);
}
