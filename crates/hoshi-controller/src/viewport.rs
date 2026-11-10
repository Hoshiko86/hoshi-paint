//! Shared camera coordinates and canvas navigation, independent of native DPI APIs.
use super::{Controller, ControllerError, MAX_VIEW_OFFSET};
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    pub zoom: f64,
    pub offset_x: f64,
    pub offset_y: f64,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            offset_x: 0.0,
            offset_y: 0.0,
        }
    }
}

impl Viewport {
    pub fn to_document(self, x: f64, y: f64) -> (f64, f64) {
        (
            (x - self.offset_x) / self.zoom,
            (y - self.offset_y) / self.zoom,
        )
    }

    pub fn to_logical(self, x: f64, y: f64) -> (f64, f64) {
        (x * self.zoom + self.offset_x, y * self.zoom + self.offset_y)
    }

    /// For adapters receiving physical pixels. Slint pointer positions are
    /// already logical, so Slint adapters must not apply this conversion again.
    pub fn physical_to_document(self, x: f64, y: f64, scale_factor: f64) -> Option<(f64, f64)> {
        if !x.is_finite() || !y.is_finite() || !scale_factor.is_finite() || scale_factor <= 0.0 {
            return None;
        }
        let point = self.to_document(x / scale_factor, y / scale_factor);
        (point.0.is_finite() && point.1.is_finite()).then_some(point)
    }
}

impl Controller {
    /// Center the document at the current zoom within a logical viewport.
    pub fn center_viewport(&mut self, width: f64, height: f64) -> Result<(), ControllerError> {
        if !width.is_finite() || !height.is_finite() || width <= 0. || height <= 0. {
            return Err(ControllerError::InvalidInput);
        }
        let x = (width - f64::from(self.document.width()) * self.viewport.zoom) / 2.;
        let y = (height - f64::from(self.document.height()) * self.viewport.zoom) / 2.;
        if x.abs() > MAX_VIEW_OFFSET || y.abs() > MAX_VIEW_OFFSET {
            return Err(ControllerError::InvalidInput);
        }
        self.viewport.offset_x = x;
        self.viewport.offset_y = y;
        Ok(())
    }

    pub fn pan(&mut self, dx: f64, dy: f64) -> Result<(), ControllerError> {
        let x = self.viewport.offset_x + dx;
        let y = self.viewport.offset_y + dy;
        if !x.is_finite()
            || !y.is_finite()
            || x.abs() > MAX_VIEW_OFFSET
            || y.abs() > MAX_VIEW_OFFSET
        {
            return Err(ControllerError::InvalidInput);
        }
        self.viewport.offset_x = x;
        self.viewport.offset_y = y;
        Ok(())
    }

    /// Keeps the document coordinate beneath a logical pointer fixed.
    pub fn zoom_at(&mut self, x: f64, y: f64, factor: f64) -> Result<(), ControllerError> {
        if !factor.is_finite() || factor <= 0.0 {
            return Err(ControllerError::InvalidInput);
        }
        let anchor = self.document_point(x, y)?;
        let zoom = (self.viewport.zoom * factor).clamp(1.0 / 4096.0, 32.0);
        let next = Viewport {
            zoom,
            offset_x: x - anchor.0 * zoom,
            offset_y: y - anchor.1 * zoom,
        };
        if !next.offset_x.is_finite()
            || !next.offset_y.is_finite()
            || next.offset_x.abs() > MAX_VIEW_OFFSET
            || next.offset_y.abs() > MAX_VIEW_OFFSET
        {
            return Err(ControllerError::InvalidInput);
        }
        self.viewport = next;
        Ok(())
    }

    /// Fit the whole image within a logical viewport, retaining a small workspace margin.
    pub fn fit_viewport(&mut self, width: f64, height: f64) -> Result<(), ControllerError> {
        if !width.is_finite() || !height.is_finite() || width <= 0. || height <= 0. {
            return Err(ControllerError::InvalidInput);
        }
        let margin = 16.0_f64.min(width / 4.).min(height / 4.);
        let zoom = ((width - margin * 2.) / f64::from(self.document.width()))
            .min((height - margin * 2.) / f64::from(self.document.height()))
            .clamp(1.0 / 4096.0, 32.);
        let x = (width - f64::from(self.document.width()) * zoom) / 2.;
        let y = (height - f64::from(self.document.height()) * zoom) / 2.;
        if x.abs() > MAX_VIEW_OFFSET || y.abs() > MAX_VIEW_OFFSET {
            return Err(ControllerError::InvalidInput);
        }
        self.viewport = Viewport {
            zoom,
            offset_x: x,
            offset_y: y,
        };
        Ok(())
    }

    pub fn reset_view(&mut self) {
        self.viewport = Viewport::default();
    }
}
