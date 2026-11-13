//! Independent tab state, shared drawing settings, and save points.
use crate::files::{self, FileError, FileFormat};
use hoshi_controller::{
    Color, ColorSlot, Controller, ControllerError, FillPattern, LineStyle, ShapeMode,
};
use hoshi_core::{Document, DocumentSnapshot};
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};
/// Workspace (tab and save-flow) failures, keeping the kind and source.
/// `Display` is developer text; user copy comes from `crate::error_text`.
#[derive(Debug, thiserror::Error)]
pub enum WorkspaceError {
    /// A controller edit failed while switching, adding or closing tabs.
    #[error("controller: {0}")]
    Controller(#[from] ControllerError),
    /// Opening, resolving or writing a file failed.
    #[error("file: {0}")]
    File(#[from] FileError),
    /// The tab id is not open any more.
    #[error("document not open")]
    MissingDocument,
    /// Adding a tab would exceed `MAX_DOCUMENTS`.
    #[error("document limit reached")]
    DocumentLimit,
    /// Opening a file would exceed `MAX_DOCUMENTS`.
    #[error("document limit reached on open")]
    OpenLimit,
    /// Save was requested with no tab open.
    #[error("no document to save")]
    NoDocument,
    /// The save target is already open in a different tab.
    #[error("path open in another tab")]
    PathOpenInAnotherTab,
    /// The format needs flattening and the user has not confirmed it.
    #[error("flatten not confirmed")]
    FlattenRequired,
    /// JPEG cannot store a density above 65535 pixels per inch.
    #[error("jpeg density too high")]
    JpegDensityTooHigh,
    /// JPEG cannot store more than 65535 pixels per side.
    #[error("jpeg dimensions too large")]
    JpegDimensionsTooLarge,
}
#[derive(Clone, Copy)]
struct Drawing {
    primary: Color,
    secondary: Color,
    slot: ColorSlot,
    mode: ShapeMode,
    width: f64,
    antialiasing: bool,
    line_style: LineStyle,
    fill_pattern: FillPattern,
}
impl Drawing {
    fn apply(self, target: &mut Controller) -> Result<(), ControllerError> {
        target.set_primary_color(self.primary)?;
        target.set_secondary_color(self.secondary)?;
        target.set_color_slot(self.slot);
        target.set_shape_mode(self.mode)?;
        target.set_stroke_width_pixels(self.width)?;
        target.set_antialiasing(self.antialiasing)?;
        target.set_line_style(self.line_style)?;
        target.set_fill_pattern(self.fill_pattern)?;
        Ok(())
    }
}
pub const MAX_DOCUMENTS: usize = 32;
pub struct DocumentTab {
    pub id: u32,
    pub controller: Controller,
    pub path: Option<PathBuf>,
    pub name: String,
    pub jpeg_quality: u8,
    saved: DocumentSnapshot,
    cache: RefCell<(DocumentSnapshot, bool)>,
    generation: Cell<u32>,
}
impl DocumentTab {
    fn new(id: u32, controller: Controller, path: Option<PathBuf>, name: String) -> Self {
        let saved = controller.document().snapshot();
        Self {
            id,
            controller,
            path,
            name,
            jpeg_quality: 95,
            cache: RefCell::new((saved.clone(), false)),
            saved,
            generation: Cell::new(0),
        }
    }
    pub fn dirty(&self) -> bool {
        let document = self.controller.document();
        let mut cache = self.cache.borrow_mut();
        if !cache.0.same_storage_as(document) {
            let snapshot = document.snapshot();
            cache.1 = snapshot != self.saved;
            cache.0 = snapshot;
            self.generation.set(self.generation.get().wrapping_add(1));
        }
        cache.1 || document.active_shape().is_some()
    }
    pub fn generation(&self) -> u32 {
        self.dirty();
        self.generation.get()
    }
    fn mark_saved(&mut self) {
        self.saved = self.controller.document().snapshot();
        *self.cache.borrow_mut() = (self.saved.clone(), false);
        self.generation.set(self.generation.get().wrapping_add(1));
    }
}
pub struct Workspace {
    tabs: Vec<DocumentTab>,
    active: usize,
    next_id: u32,
    untitled: u32,
    empty: Controller,
    recent: VecDeque<PathBuf>,
    recent_store: Option<PathBuf>,
}
impl Default for Workspace {
    fn default() -> Self {
        let c = Controller::default();
        Self {
            tabs: vec![DocumentTab::new(1, c, None, "Untitled".into())],
            active: 0,
            next_id: 2,
            untitled: 1,
            empty: Controller::default(),
            recent: VecDeque::new(),
            recent_store: None,
        }
    }
}
impl Deref for Workspace {
    type Target = Controller;
    fn deref(&self) -> &Controller {
        self.tabs
            .get(self.active)
            .map(|t| &t.controller)
            .unwrap_or(&self.empty)
    }
}
impl DerefMut for Workspace {
    fn deref_mut(&mut self) -> &mut Controller {
        self.tabs
            .get_mut(self.active)
            .map(|t| &mut t.controller)
            .unwrap_or(&mut self.empty)
    }
}
impl Workspace {
    pub fn with_recent_store(path: Option<PathBuf>) -> Self {
        use std::io::Read;
        let mut workspace = Self {
            recent_store: path,
            ..Self::default()
        };
        if let Some(path) = &workspace.recent_store
            && let Ok(file) = std::fs::File::open(path)
        {
            let mut bytes = Vec::new();
            if file.take(256 * 1024 + 1).read_to_end(&mut bytes).is_ok()
                && bytes.len() <= 256 * 1024
                && let Ok(paths) = serde_json::from_slice::<Vec<String>>(&bytes)
            {
                for path in paths
                    .into_iter()
                    .take(10)
                    .filter(|p| p.len() <= 32768)
                    .map(PathBuf::from)
                    .filter(|p| p.is_absolute())
                {
                    if !workspace.recent.contains(&path) {
                        workspace.recent.push_back(path)
                    }
                }
            }
        }
        workspace
    }
    fn persist_recent(&self) {
        use std::io::Write;
        let Some(path) = &self.recent_store else {
            return;
        };
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            let parent = path.parent().ok_or("Recent file path has no parent")?;
            std::fs::create_dir_all(parent)?;
            let paths: Vec<_> = self.recent.iter().filter_map(|p| p.to_str()).collect();
            let mut file = tempfile::NamedTempFile::new_in(parent)?;
            serde_json::to_writer(&mut file, &paths)?;
            file.flush()?;
            file.as_file().sync_all()?;
            file.persist(path)?;
            Ok(())
        })();
        if result.is_err() {
            eprintln!("Recent files could not be persisted; the image save is unaffected.")
        }
    }
    pub fn tabs(&self) -> &[DocumentTab] {
        &self.tabs
    }
    pub fn active_tab(&self) -> Option<&DocumentTab> {
        self.tabs.get(self.active)
    }
    pub fn active_id(&self) -> Option<u32> {
        self.active_tab().map(|t| t.id)
    }
    pub fn has_document(&self) -> bool {
        !self.tabs.is_empty()
    }
    pub fn recent(&self) -> &VecDeque<PathBuf> {
        &self.recent
    }
    pub fn clear_recent(&mut self) {
        self.recent.clear();
        self.persist_recent()
    }
    pub fn any_dirty(&self) -> bool {
        self.tabs.iter().any(DocumentTab::dirty)
    }
    fn drawing(&self) -> Drawing {
        Drawing {
            primary: self.primary_color(),
            secondary: self.secondary_color(),
            slot: self.color_slot(),
            mode: self.shape_mode(),
            width: self.stroke_width_pixels(),
            antialiasing: self.antialiasing(),
            line_style: self.line_style(),
            fill_pattern: self.fill_pattern(),
        }
    }
    fn settings_to(&self, target: &mut Controller) {
        self.drawing()
            .apply(target)
            .expect("valid drawing settings");
    }
    fn finish(&mut self) -> Result<(), ControllerError> {
        if self.has_document() {
            self.pointer_up()?;
            self.commit()?;
        }
        Ok(())
    }
    pub fn activate(&mut self, id: u32) -> Result<(), WorkspaceError> {
        let index = self
            .tabs
            .iter()
            .position(|t| t.id == id)
            .ok_or(WorkspaceError::MissingDocument)?;
        if index == self.active {
            return Ok(());
        }
        self.finish()?;
        let settings = self.drawing();
        settings.apply(&mut self.tabs[index].controller)?;
        self.active = index;
        Ok(())
    }
    pub fn cycle(&mut self, backward: bool) -> Result<(), WorkspaceError> {
        if self.tabs.is_empty() {
            return Ok(());
        }
        let index = if backward {
            (self.active + self.tabs.len() - 1) % self.tabs.len()
        } else {
            (self.active + 1) % self.tabs.len()
        };
        self.activate(self.tabs[index].id)
    }
    pub fn new_image(
        &mut self,
        width: u32,
        height: u32,
        resolution: f64,
        background: Color,
    ) -> Result<(), ControllerError> {
        if self.tabs.len() >= MAX_DOCUMENTS || self.next_id >= i32::MAX as u32 {
            return Err(ControllerError::InvalidInput);
        }
        let document = Document::new_with_background(width, height, resolution, background)
            .map_err(ControllerError::from)?;
        self.add(document, None)
            .map_err(|_| ControllerError::InvalidInput)?;
        Ok(())
    }
    fn add(&mut self, document: Document, path: Option<PathBuf>) -> Result<u32, WorkspaceError> {
        if self.tabs.len() >= MAX_DOCUMENTS || self.next_id >= i32::MAX as u32 {
            return Err(WorkspaceError::DocumentLimit);
        }
        self.finish()?;
        let mut c = Controller::default();
        self.settings_to(&mut c);
        c.replace_document(document);
        let name = if let Some(path) = &path {
            path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
        } else {
            self.untitled += 1;
            format!("Untitled {}", self.untitled)
        };
        let id = self.next_id;
        self.next_id += 1;
        self.tabs.push(DocumentTab::new(id, c, path, name));
        self.active = self.tabs.len() - 1;
        Ok(id)
    }
    pub fn open_path(&mut self, path: &Path) -> Result<u32, WorkspaceError> {
        let path = files::absolute_path(path)?;
        if let Some(id) = self
            .tabs
            .iter()
            .find(|t| t.path.as_ref() == Some(&path))
            .map(|t| t.id)
        {
            self.activate(id)?;
            return Ok(id);
        }
        if self.tabs.len() >= MAX_DOCUMENTS {
            return Err(WorkspaceError::OpenLimit);
        }
        let document = files::load(&path)?;
        let id = self.add(document, Some(path.clone()))?;
        self.remember(path);
        Ok(id)
    }
    fn remember(&mut self, path: PathBuf) {
        self.recent.retain(|p| p != &path);
        self.recent.push_front(path);
        self.recent.truncate(10);
        self.persist_recent();
    }
    pub fn requires_flatten(&self, format: FileFormat) -> bool {
        if format == FileFormat::Project {
            return false;
        }
        let d = self.document();
        d.layers().len() > 1
            || !d.layers()[0].visible()
            || d.layers()[0].opacity() != 255
            || (format == FileFormat::Jpeg && d.has_transparency())
    }
    pub fn save_to(
        &mut self,
        path: &Path,
        quality: u8,
        allow_flatten: bool,
    ) -> Result<(), WorkspaceError> {
        if !self.has_document() {
            return Err(WorkspaceError::NoDocument);
        }
        let path = files::absolute_path(path)?;
        let format = FileFormat::for_path(&path)?;
        if self
            .tabs
            .iter()
            .enumerate()
            .any(|(i, t)| i != self.active && t.path.as_ref() == Some(&path))
        {
            return Err(WorkspaceError::PathOpenInAnotherTab);
        }
        if self.requires_flatten(format) && !allow_flatten {
            return Err(WorkspaceError::FlattenRequired);
        }
        if format == FileFormat::Jpeg && self.document().resolution().round() > 65535. {
            return Err(WorkspaceError::JpegDensityTooHigh);
        }
        if format == FileFormat::Jpeg
            && (self.document().width() > 65535 || self.document().height() > 65535)
        {
            return Err(WorkspaceError::JpegDimensionsTooLarge);
        }
        self.finish()?;
        if self.requires_flatten(format) {
            self.document_for_save_flatten(format)?;
        }
        files::save_atomic(&path, self.document(), format, quality)?;
        let tab = &mut self.tabs[self.active];
        tab.path = Some(path.clone());
        tab.name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        tab.jpeg_quality = quality.clamp(1, 100);
        tab.mark_saved();
        self.remember(path);
        Ok(())
    }
    fn document_for_save_flatten(&mut self, format: FileFormat) -> Result<(), ControllerError> {
        self.flatten_for_export(if format == FileFormat::Jpeg {
            Some(Color::rgba(255, 255, 255, 255))
        } else {
            None
        })
    }
    pub fn close_discard(&mut self, id: u32) -> Result<(), WorkspaceError> {
        let index = self
            .tabs
            .iter()
            .position(|t| t.id == id)
            .ok_or(WorkspaceError::MissingDocument)?;
        if index == self.active {
            let settings = self.drawing();
            settings.apply(&mut self.empty)?;
            self.tabs.remove(index);
            self.active = index.min(self.tabs.len().saturating_sub(1));
            if let Some(tab) = self.tabs.get_mut(self.active) {
                settings.apply(&mut tab.controller)?;
            }
        } else {
            self.tabs.remove(index);
            if index < self.active {
                self.active -= 1
            }
        }
        Ok(())
    }
}
