use crate::{Controller, ControllerError, MAX_LAYER_NAME_CHARS};
use hoshi_core::{Error, MAX_LAYERS};

fn validate_layer_name(name: &str) -> Result<(), ControllerError> {
    if name.trim().is_empty()
        || name.chars().take(MAX_LAYER_NAME_CHARS + 1).count() > MAX_LAYER_NAME_CHARS
    {
        return Err(ControllerError::InvalidLayerName);
    }
    Ok(())
}

impl Controller {
    /// Selects a layer (0 = bottom), committing the live edit; the current layer is a no-op.
    pub fn select_layer(&mut self, layer: usize) -> Result<(), ControllerError> {
        if layer >= self.document.layers().len() {
            return Err(Error::InvalidLayer.into());
        }
        if layer == self.document.selected_layer() {
            return Ok(());
        }
        self.commit()?;
        self.document.select_layer(layer)?;
        Ok(())
    }

    /// Adds and selects a transparent layer; invalid names and the layer limit fail before commit.
    pub fn add_layer(&mut self, name: &str) -> Result<usize, ControllerError> {
        validate_layer_name(name)?;
        if self.document.layers().len() >= MAX_LAYERS {
            return Err(Error::LayerLimit.into());
        }
        self.commit()?;
        // TODO(#7): insert above the active layer once core supports insertion at an index.
        Ok(self.document.add_layer(name)?)
    }

    /// Deletes the selected layer after committing; the last layer cannot be deleted.
    pub fn delete_layer(&mut self) -> Result<(), ControllerError> {
        if self.document.layers().len() == 1 {
            return Err(Error::LastLayer.into());
        }
        self.commit()?;
        self.document.delete_layer(self.document.selected_layer())?;
        Ok(())
    }

    /// Moves the selected layer one step up, committing first; the top layer is a no-op.
    pub fn move_layer_up(&mut self) -> Result<(), ControllerError> {
        let layer = self.document.selected_layer();
        if layer + 1 == self.document.layers().len() {
            return Ok(());
        }
        self.commit()?;
        self.document.move_layer(layer, layer + 1)?;
        Ok(())
    }

    /// Moves the selected layer one step down, committing first; the bottom layer is a no-op.
    pub fn move_layer_down(&mut self) -> Result<(), ControllerError> {
        let layer = self.document.selected_layer();
        if layer == 0 {
            return Ok(());
        }
        self.commit()?;
        self.document.move_layer(layer, layer - 1)?;
        Ok(())
    }

    /// Sets visibility without selecting the layer; invalid indices fail before commit, unchanged is a no-op.
    pub fn set_layer_visible(
        &mut self,
        layer: usize,
        visible: bool,
    ) -> Result<(), ControllerError> {
        let opacity = self
            .document
            .layers()
            .get(layer)
            .ok_or(Error::InvalidLayer)?
            .opacity();
        self.set_layer_properties(layer, visible, opacity)
    }

    /// Sets visibility and opacity in one history entry after commit; unchanged values are a no-op.
    pub fn set_layer_properties(
        &mut self,
        layer: usize,
        visible: bool,
        opacity: u8,
    ) -> Result<(), ControllerError> {
        let current = self
            .document
            .layers()
            .get(layer)
            .ok_or(Error::InvalidLayer)?;
        if current.visible() == visible && current.opacity() == opacity {
            return Ok(());
        }
        self.commit()?;
        self.document
            .set_layer_properties(layer, visible, opacity)?;
        Ok(())
    }

    pub fn rename_layer(&mut self, name: &str) -> Result<(), ControllerError> {
        // Validate before committing an active shape or allocating a new name.
        validate_layer_name(name)?;
        self.commit()?;
        self.document
            .rename_layer(self.document.selected_layer(), name)?;
        Ok(())
    }
}
