//! Raster layer properties and shared pixel storage.

use std::sync::Arc;

use crate::PixelStore;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layer {
    pub(crate) name: String,
    pub(crate) visible: bool,
    pub(crate) opacity: u8,
    pub(crate) pixels: Arc<PixelStore>,
}

impl Layer {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn visible(&self) -> bool {
        self.visible
    }
    pub fn opacity(&self) -> u8 {
        self.opacity
    }
    pub fn pixels(&self) -> &PixelStore {
        &self.pixels
    }
}
