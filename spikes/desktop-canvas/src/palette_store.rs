//! Palette library persistence, separate from image documents and undo history.
//!
//! Named palettes are Paint.NET-compatible text files. The session file keeps
//! unsaved edits and a saved baseline, so editing a loaded palette never silently
//! overwrites its original file. Every state change is prepared before replacing
//! the in-memory library.
use hoshi_controller::{
    Color,
    palette::{Palette, PaletteError},
};
use serde_json::{Value, json};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

const STATE_FILE: &str = "palette-state.json";
const MAX_TEXT_BYTES: u64 = 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SavedPalette {
    pub name: String,
    pub path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct PaletteLibrary {
    root: Option<PathBuf>,
    current: Palette,
    baseline: Palette,
    title: String,
    selected_path: Option<PathBuf>,
    default_selected: bool,
    registered_paths: Vec<PathBuf>,
    entries: Vec<SavedPalette>,
}

impl PaletteLibrary {
    pub fn open(config_root: Option<PathBuf>) -> Result<Self, String> {
        let mut library = Self {
            root: config_root.map(|root| absolute(&root)).transpose()?,
            current: Palette::default(),
            baseline: Palette::default(),
            title: "Default".into(),
            selected_path: None,
            default_selected: true,
            registered_paths: Vec::new(),
            entries: Vec::new(),
        };
        if let Some(root) = &library.root {
            let path = root.join(STATE_FILE);
            match fs::metadata(&path) {
                Ok(_) => library.restore(&read_text(&path)?)?,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
                Err(error) => return Err(format!("Cannot read palette settings: {error}")),
            }
        }
        library.refresh()?;
        Ok(library)
    }

    pub fn current(&self) -> &Palette {
        &self.current
    }
    pub fn title(&self) -> &str {
        &self.title
    }
    pub fn dirty(&self) -> bool {
        self.current != self.baseline
    }
    /// Custom palettes only; UI index zero is the built-in Default palette.
    pub fn entries(&self) -> &[SavedPalette] {
        &self.entries
    }
    pub fn selected_index(&self) -> i32 {
        if let Some(path) = &self.selected_path {
            self.entries
                .iter()
                .position(|entry| &entry.path == path)
                .and_then(|index| i32::try_from(index + 1).ok())
                .unwrap_or(-1)
        } else if self.default_selected {
            0
        } else {
            -1
        }
    }

    pub fn replace(&mut self, index: usize, color: Color) -> Result<(), String> {
        let mut candidate = self.clone();
        candidate
            .current
            .replace(index, color)
            .map_err(|_| "Choose a valid palette slot.".to_string())?;
        self.accept(candidate)
    }
    pub fn reset(&mut self) -> Result<(), String> {
        let mut candidate = self.clone();
        candidate.current = Palette::default();
        candidate.baseline = candidate.current.clone();
        candidate.title = "Default".into();
        candidate.selected_path = None;
        candidate.default_selected = true;
        self.accept(candidate)
    }
    pub fn select(&mut self, index: usize) -> Result<(), String> {
        if index == 0 {
            return self.reset();
        }
        let entry = self
            .entries
            .get(index - 1)
            .ok_or("Choose a valid saved palette.")?;
        let palette = load_palette(&entry.path)?;
        let mut candidate = self.clone();
        candidate.current = palette.clone();
        candidate.baseline = palette;
        candidate.title = entry.name.clone();
        candidate.selected_path = Some(entry.path.clone());
        candidate.default_selected = false;
        self.accept(candidate)
    }
    /// Load into the working palette; subsequent edits never write this source.
    pub fn import(&mut self, path: &Path) -> Result<(), String> {
        let palette = load_palette(path)?;
        let mut candidate = self.clone();
        candidate.current = palette;
        // An imported palette is unsaved work until explicitly saved into the
        // library. Keeping the old baseline also preserves meaningful dirty state.
        candidate.title = palette_name(path);
        candidate.selected_path = None;
        candidate.default_selected = false;
        self.accept(candidate)
    }
    pub fn export(&self, path: &Path) -> Result<(), String> {
        self.check_destination(path)?;
        atomic_write(path, self.current.to_paint_net().as_bytes())
    }
    pub fn save_as(&mut self, path: &Path) -> Result<(), String> {
        let path = absolute(path)?;
        self.check_destination(&path)?;
        let mut candidate = self.clone();
        if !candidate.registered_paths.contains(&path) {
            candidate.registered_paths.push(path.clone());
        }
        candidate.selected_path = Some(path.clone());
        candidate.default_selected = false;
        candidate.title = palette_name(&path);
        candidate.baseline = candidate.current.clone();
        candidate.refresh()?;
        // Include the new path even though its file has not been installed yet.
        if !candidate.entries.iter().any(|entry| entry.path == path) {
            candidate.entries.push(SavedPalette {
                name: palette_name(&path),
                path: path.clone(),
            });
            sort_entries(&mut candidate.entries);
        }
        // Stage both files before replacing either destination. If installing
        // session state fails, restore the named palette's prior contents.
        let state = candidate.stage_state()?;
        let previous = match fs::metadata(&path) {
            Ok(_) => Some(read_bytes(&path)?),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.to_string()),
        };
        let palette = stage_file(&path, candidate.current.to_paint_net().as_bytes())?;
        install(palette, &path)?;
        if let Some((temporary, state_path)) = state
            && let Err(error) = install(temporary, &state_path)
        {
            let rollback = if let Some(bytes) = previous {
                atomic_write(&path, &bytes)
            } else {
                fs::remove_file(&path).map_err(|e| e.to_string())
            };
            return match rollback {
                Ok(()) => Err(error),
                Err(rollback) => Err(format!(
                    "{error}. Restoring the palette file also failed: {rollback}"
                )),
            };
        }
        *self = candidate;
        Ok(())
    }
    /// Rescan the managed folder and explicitly registered external Save As files.
    /// Malformed palette contents are reported only when selected, not discarded.
    pub fn refresh(&mut self) -> Result<(), String> {
        let mut paths = self.registered_paths.clone();
        if let Some(root) = &self.root {
            match fs::read_dir(root.join("palettes")) {
                Ok(directory) => {
                    for entry in directory {
                        let path = entry.map_err(|e| e.to_string())?.path();
                        if path.is_file()
                            && path
                                .extension()
                                .and_then(|e| e.to_str())
                                .is_some_and(|e| e.eq_ignore_ascii_case("txt"))
                        {
                            paths.push(absolute(&path)?);
                        }
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
                Err(error) => return Err(format!("Cannot list palettes: {error}")),
            }
        }
        paths.sort();
        paths.dedup();
        let mut entries: Vec<_> = paths
            .into_iter()
            .filter(|path| path.is_file())
            .map(|path| SavedPalette {
                name: palette_name(&path),
                path,
            })
            .collect();
        sort_entries(&mut entries);
        self.entries = entries;
        Ok(())
    }
    pub fn folder(&self) -> Result<PathBuf, String> {
        let path = self
            .root
            .as_ref()
            .ok_or("Palette storage is not configured.")?
            .join("palettes");
        fs::create_dir_all(&path).map_err(|e| format!("Cannot create the palettes folder: {e}"))?;
        Ok(path)
    }

    fn accept(&mut self, candidate: Self) -> Result<(), String> {
        if let Some((temporary, path)) = candidate.stage_state()? {
            install(temporary, &path)?;
        }
        *self = candidate;
        Ok(())
    }
    fn stage_state(&self) -> Result<Option<(tempfile::NamedTempFile, PathBuf)>, String> {
        let Some(root) = &self.root else {
            return Ok(None);
        };
        fs::create_dir_all(root)
            .map_err(|e| format!("Cannot create palette settings folder: {e}"))?;
        // Path serialization can fail for non-Unicode filenames. Keep that
        // fallible step outside json!, whose convenience conversion can panic.
        let selected_path = serde_json::to_value(&self.selected_path)
            .map_err(|_| "Palette filenames must use Unicode text.")?;
        let registered_paths = serde_json::to_value(&self.registered_paths)
            .map_err(|_| "Palette filenames must use Unicode text.")?;
        let state = json!({
            "version": 1,
            "current": self.current.to_paint_net(),
            "baseline": self.baseline.to_paint_net(),
            "title": self.title,
            "selected_path": selected_path,
            "default_selected": self.default_selected,
            "registered_paths": registered_paths,
        });
        let bytes = serde_json::to_vec_pretty(&state).map_err(|e| e.to_string())?;
        let path = root.join(STATE_FILE);
        Ok(Some((stage_file(&path, &bytes)?, path)))
    }
    fn restore(&mut self, text: &str) -> Result<(), String> {
        let state: Value =
            serde_json::from_str(text).map_err(|e| format!("Invalid palette settings: {e}"))?;
        if state.get("version").and_then(Value::as_u64) != Some(1) {
            return Err("Unsupported palette settings version.".into());
        }
        let palette = |name| -> Result<Palette, String> {
            let text = state
                .get(name)
                .and_then(Value::as_str)
                .ok_or("Incomplete palette settings.")?;
            parse_palette(text)
        };
        let current = palette("current")?;
        let baseline = palette("baseline")?;
        let title = state
            .get("title")
            .and_then(Value::as_str)
            .ok_or("Incomplete palette settings.")?
            .to_string();
        let selected_path: Option<PathBuf> = serde_json::from_value(
            state
                .get("selected_path")
                .cloned()
                .ok_or("Incomplete palette settings.")?,
        )
        .map_err(|e| e.to_string())?;
        let registered_paths: Vec<PathBuf> = serde_json::from_value(
            state
                .get("registered_paths")
                .cloned()
                .ok_or("Incomplete palette settings.")?,
        )
        .map_err(|e| e.to_string())?;
        self.default_selected = state
            .get("default_selected")
            .and_then(Value::as_bool)
            .ok_or("Incomplete palette settings.")?;
        self.current = current;
        self.baseline = baseline;
        self.title = title;
        self.selected_path = selected_path;
        self.registered_paths = registered_paths;
        Ok(())
    }
    fn check_destination(&self, path: &Path) -> Result<(), String> {
        if let Some(root) = &self.root
            && identity(path)? == identity(&root.join(STATE_FILE))?
        {
            return Err("Choose a palette text file, not the application's settings file.".into());
        }
        Ok(())
    }
}

fn palette_name(path: &Path) -> String {
    path.file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}
fn sort_entries(entries: &mut [SavedPalette]) {
    entries.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then(a.path.cmp(&b.path))
    });
    // Distinguish same-named files while retaining stable paths as identities.
    for index in 0..entries.len() {
        if entries
            .iter()
            .filter(|entry| {
                palette_name(&entry.path).eq_ignore_ascii_case(&palette_name(&entries[index].path))
            })
            .count()
            > 1
        {
            entries[index].name = format!(
                "{} ({})",
                palette_name(&entries[index].path),
                entries[index].path.display()
            );
        }
    }
}
fn absolute(path: &Path) -> Result<PathBuf, String> {
    let full = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(path)
    };
    if let Some(parent) = full.parent()
        && let Ok(parent) = fs::canonicalize(parent)
        && let Some(name) = full.file_name()
    {
        return Ok(parent.join(name));
    }
    let mut normalized = PathBuf::new();
    for component in full.components() {
        match component {
            std::path::Component::CurDir => (),
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            component => normalized.push(component.as_os_str()),
        }
    }
    Ok(normalized)
}
fn identity(path: &Path) -> Result<PathBuf, String> {
    fs::canonicalize(path)
        .or_else(|_| absolute(path).map_err(std::io::Error::other))
        .map_err(|e| e.to_string())
}
fn parse_palette(text: &str) -> Result<Palette, String> {
    Palette::parse_paint_net(text).map_err(|error| match error {
        PaletteError::InvalidColor { line } => {
            format!("Palette line {line} must contain eight ARGB hexadecimal digits.")
        }
        PaletteError::InvalidIndex { .. } => "Invalid palette slot.".into(),
    })
}
fn load_palette(path: &Path) -> Result<Palette, String> {
    parse_palette(&read_text(path)?)
}
fn read_bytes(path: &Path) -> Result<Vec<u8>, String> {
    let file = fs::File::open(path).map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
    let mut bytes = Vec::new();
    file.take(MAX_TEXT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_TEXT_BYTES {
        return Err("Palette text exceeds the supported 1 MiB size.".into());
    }
    Ok(bytes)
}
fn read_text(path: &Path) -> Result<String, String> {
    String::from_utf8(read_bytes(path)?).map_err(|_| "Palette text must use UTF-8 encoding.".into())
}
fn stage_file(path: &Path, bytes: &[u8]) -> Result<tempfile::NamedTempFile, String> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let permissions = match fs::metadata(path) {
        Ok(metadata) => {
            if metadata.is_dir() {
                return Err("Choose a file, not a folder.".into());
            }
            if metadata.permissions().readonly() {
                return Err("The palette destination is read-only.".into());
            }
            Some(metadata.permissions())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.to_string()),
    };
    let mut temporary = tempfile::NamedTempFile::new_in(parent)
        .map_err(|e| format!("Cannot create palette file: {e}"))?;
    temporary.write_all(bytes).map_err(|e| e.to_string())?;
    if let Some(permissions) = permissions {
        temporary
            .as_file()
            .set_permissions(permissions)
            .map_err(|e| e.to_string())?;
    }
    temporary.as_file().sync_all().map_err(|e| e.to_string())?;
    Ok(temporary)
}
fn install(temporary: tempfile::NamedTempFile, path: &Path) -> Result<(), String> {
    temporary
        .persist(path)
        .map_err(|e| format!("Cannot save {}: {}", path.display(), e.error))?;
    Ok(())
}
fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    install(stage_file(path, bytes)?, path)
}
