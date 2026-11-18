//! Raster layer properties and shared pixel storage.

use std::sync::Arc;

use crate::PixelStore;

/// A raster layer in [`crate::Document::layers`] (bottom first), edited through document operations with one history entry per edit.
/// [`crate::Document::add_layer`] adds a transparent, visible, document-sized layer at opacity 255.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layer {
    pub(crate) name: String,
    pub(crate) visible: bool,
    pub(crate) opacity: u8,
    pub(crate) pixels: Arc<PixelStore>,
}

impl Layer {
    /// Returns the stored, unvalidated name; the controller validates names.
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Returns whether the layer participates in compositing; hidden layers are skipped.
    pub fn visible(&self) -> bool {
        self.visible
    }
    /// Returns opacity (0 transparent, 255 opaque), multiplied by each pixel's alpha during compositing.
    pub fn opacity(&self) -> u8 {
        self.opacity
    }
    /// Borrows document-sized sparse, copy-on-write pixels shared with history; reading never copies.
    pub fn pixels(&self) -> &PixelStore {
        &self.pixels
    }
}
