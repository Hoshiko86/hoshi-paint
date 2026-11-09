//! Numeric geometry edits shared by shape tools and platform adapters.
use super::{Controller, ControllerError};
use hoshi_core::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoundsField {
    X,
    Y,
    Width,
    Height,
}
impl Controller {
    /// Capture the current proportion once, before integer rounding can change it.
    pub fn set_numeric_aspect_lock(&mut self, enabled: bool) {
        self.edit_state.numeric_ratio = if enabled {
            self.document.active_shape().map(|s| (s.width, s.height))
        } else {
            None
        };
    }

    /// X/Y describe the unrotated local bounds in canvas pixels. A dimension
    /// edit keeps that origin fixed; rotation, mirrors and paints are preserved.
    pub fn edit_shape_bound(
        &mut self,
        field: BoundsField,
        value: i64,
        keep_aspect: bool,
    ) -> Result<(), ControllerError> {
        let shape = self
            .document
            .active_shape()
            .ok_or(ControllerError::NoActiveShape)?;
        let (mut x, mut y, mut width, mut height) = (shape.x, shape.y, shape.width, shape.height);
        let (ratio_width, ratio_height) = self.edit_state.numeric_ratio.unwrap_or((width, height));
        let invalid = || ControllerError::Core(Error::InvalidShape);
        match field {
            BoundsField::X => x = i32::try_from(value).map_err(|_| invalid())?,
            BoundsField::Y => y = i32::try_from(value).map_err(|_| invalid())?,
            BoundsField::Width => {
                width = u32::try_from(value).map_err(|_| invalid())?;
                if keep_aspect {
                    height = u32::try_from(
                        (u64::from(width) * u64::from(ratio_height) + u64::from(ratio_width) / 2)
                            / u64::from(ratio_width),
                    )
                    .map_err(|_| invalid())?
                    .max(1);
                }
            }
            BoundsField::Height => {
                height = u32::try_from(value).map_err(|_| invalid())?;
                if keep_aspect {
                    width = u32::try_from(
                        (u64::from(height) * u64::from(ratio_width) + u64::from(ratio_height) / 2)
                            / u64::from(ratio_height),
                    )
                    .map_err(|_| invalid())?
                    .max(1);
                }
            }
        }
        self.set_shape_bounds(x, y, width, height)?;
        self.edit_state.numeric_ratio = keep_aspect.then_some((ratio_width, ratio_height));
        Ok(())
    }
}
