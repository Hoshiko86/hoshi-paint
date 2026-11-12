//! Whole-document edits. Geometry and resolution participate in history snapshots.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageTransform {
    FlipHorizontal,
    FlipVertical,
    RotateClockwise,
    RotateCounterClockwise,
    Rotate180,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resampling {
    Nearest,
    Bilinear,
    Bicubic,
    BicubicSmooth,
    BilinearLowQuality,
    AdaptiveSharp,
    Lanczos,
    Fant,
}
/// GUI defaults match Paint.NET. A bare method retains the legacy encoded-RGB API.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResizeOptions {
    pub method: Resampling,
    pub gamma_correction: bool,
}
impl Default for ResizeOptions {
    fn default() -> Self {
        Self {
            method: Resampling::Bicubic,
            gamma_correction: true,
        }
    }
}
impl From<Resampling> for ResizeOptions {
    fn from(method: Resampling) -> Self {
        Self {
            method,
            gamma_correction: false,
        }
    }
}
/// The fixed point when extending or cropping a canvas (row-major 3 by 3 grid).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Anchor {
    pub column: u8,
    pub row: u8,
}

impl Document {
    pub fn resolution(&self) -> f64 {
        f64::from(self.state.resolution) / 1000.0
    }
    pub fn validate_image_size(width: u32, height: u32) -> Result<(), Error> {
        if width == 0
            || height == 0
            || width > MAX_DIMENSION
            || height > MAX_DIMENSION
            || (u64::from(width) * u64::from(height)) > usize::MAX as u64 / 4
        {
            Err(Error::InvalidDimensions)
        } else {
            Ok(())
        }
    }
    pub fn validate_resolution(resolution: f64) -> Result<(), Error> {
        if !resolution.is_finite() || !(1.0..=100_000.0).contains(&resolution) {
            Err(Error::InvalidDimensions)
        } else {
            Ok(())
        }
    }
    fn image_editable(&self) -> Result<(), Error> {
        self.reject_active_stroke()?;
        if self.active.is_some() {
            Err(Error::ActiveShape)
        } else {
            Ok(())
        }
    }
    pub fn set_resolution(&mut self, resolution: f64) -> Result<(), Error> {
        Self::validate_resolution(resolution)?;
        self.image_editable()?;
        let before = self.state.clone();
        self.state.resolution = (resolution * 1000.0).round() as u32;
        self.checkpoint(before, "Resolution")?;
        Ok(())
    }
    /// New document initialization, without adding an undo entry.
    pub fn new_with_background(
        width: u32,
        height: u32,
        resolution: f64,
        color: Color,
    ) -> Result<Self, Error> {
        Self::validate_resolution(resolution)?;
        let mut document = Self::new(width, height)?;
        document.state.resolution = (resolution * 1000.0).round() as u32;
        document.state.layers[0].pixels = Arc::new(PixelStore::new(width, height, color));
        Ok(document)
    }
    /// Canvas size changes bounds only; no resampling. New areas can be transparent
    /// or filled on the bottom layer, leaving upper layers transparent.
    pub fn resize_canvas(
        &mut self,
        width: u32,
        height: u32,
        anchor: Anchor,
        fill: Color,
    ) -> Result<(), Error> {
        Self::validate_image_size(width, height)?;
        if anchor.column > 2 || anchor.row > 2 {
            return Err(Error::InvalidDimensions);
        }
        self.image_editable()?;
        if (width, height) == (self.width(), self.height()) {
            return Ok(());
        }
        let before = self.state.clone();
        let dx = (i64::from(width) - i64::from(self.width())) * i64::from(anchor.column) / 2;
        let dy = (i64::from(height) - i64::from(self.height())) * i64::from(anchor.row) / 2;
        let mut layers = before.layers.clone();
        for (index, layer) in layers.iter_mut().enumerate() {
            let fill = if index == 0 { fill } else { Color::TRANSPARENT };
            let covers = dx <= 0
                && dy <= 0
                && dx + i64::from(before.width) >= i64::from(width)
                && dy + i64::from(before.height) >= i64::from(height);
            let background = if covers {
                layer.pixels.background()
            } else {
                fill
            };
            let mut pixels = PixelStore::new(width, height, background);
            let rects: Vec<_> = if background == layer.pixels.background() {
                layer.pixels.allocated_rects().collect()
            } else {
                // Different padding around a uniform region needs materialized tiles.
                // Reject a buffer that cannot fit before spending time filling it.
                let area =
                    u64::from(before.width.min(width)) * u64::from(before.height.min(height));
                let _check = MemoryReservation::new(area.saturating_mul(4))?;
                vec![(0, 0, before.width, before.height)]
            };
            for (left, top, right, bottom) in rects {
                for y in top..bottom {
                    for x in left..right {
                        let nx = i64::from(x) + dx;
                        let ny = i64::from(y) + dy;
                        if nx >= 0 && ny >= 0 && nx < i64::from(width) && ny < i64::from(height) {
                            let color = layer.pixels.get(x, y);
                            if color != background {
                                pixels.set(nx as u32, ny as u32, color)?;
                            }
                        }
                    }
                }
            }
            layer.pixels = Arc::new(pixels);
        }
        self.state.layers = layers;
        self.state.width = width;
        self.state.height = height;
        self.checkpoint(before, "Canvas Size")?;
        Ok(())
    }
    /// Resize each layer in premultiplied alpha to avoid dark fringes at transparency.
    pub fn resize_image(
        &mut self,
        width: u32,
        height: u32,
        resolution: f64,
        options: impl Into<ResizeOptions>,
    ) -> Result<(), Error> {
        Self::validate_image_size(width, height)?;
        Self::validate_resolution(resolution)?;
        self.image_editable()?;
        let before = self.state.clone();
        if (width, height) != (self.width(), self.height()) {
            let options = options.into();
            let mut layers = before.layers.clone();
            for layer in &mut layers {
                layer.pixels = Arc::new(crate::resampling::resize(
                    &layer.pixels,
                    (before.width, before.height),
                    (width, height),
                    options,
                )?);
            }
            self.state.layers = layers;
        }
        self.state.width = width;
        self.state.height = height;
        self.state.resolution = (resolution * 1000.0).round() as u32;
        self.checkpoint(before, "Resize")?;
        Ok(())
    }
    pub fn transform_image(&mut self, transform: ImageTransform) -> Result<(), Error> {
        self.image_editable()?;
        let before = self.state.clone();
        let (width, height) = match transform {
            ImageTransform::RotateClockwise | ImageTransform::RotateCounterClockwise => {
                (before.height, before.width)
            }
            _ => (before.width, before.height),
        };
        let mut layers = before.layers.clone();
        for layer in &mut layers {
            let mut pixels = PixelStore::new(width, height, layer.pixels.background());
            for (left, top, right, bottom) in layer.pixels.allocated_rects() {
                for y in top..bottom {
                    for x in left..right {
                        let color = layer.pixels.get(x, y);
                        if color == pixels.background() {
                            continue;
                        }
                        let (nx, ny) = match transform {
                            ImageTransform::FlipHorizontal => (before.width - 1 - x, y),
                            ImageTransform::FlipVertical => (x, before.height - 1 - y),
                            ImageTransform::RotateClockwise => (before.height - 1 - y, x),
                            ImageTransform::RotateCounterClockwise => (y, before.width - 1 - x),
                            ImageTransform::Rotate180 => {
                                (before.width - 1 - x, before.height - 1 - y)
                            }
                        };
                        pixels.set(nx, ny, color)?;
                    }
                }
            }
            layer.pixels = Arc::new(pixels);
        }
        self.state.layers = layers;
        self.state.width = width;
        self.state.height = height;
        self.checkpoint(
            before,
            match transform {
                ImageTransform::FlipHorizontal => "Flip Horizontal",
                ImageTransform::FlipVertical => "Flip Vertical",
                ImageTransform::RotateClockwise => "Rotate 90° Clockwise",
                ImageTransform::RotateCounterClockwise => "Rotate 90° Counter-Clockwise",
                ImageTransform::Rotate180 => "Rotate 180°",
            },
        )?;
        Ok(())
    }
    pub fn flatten(&mut self) -> Result<(), Error> {
        self.image_editable()?;
        if self.state.layers.len() == 1
            && self.state.layers[0].visible
            && self.state.layers[0].opacity == 255
        {
            return Ok(());
        }
        let before = self.state.clone();
        self.state.layers = vec![Layer {
            name: "Background".into(),
            visible: true,
            opacity: 255,
            pixels: Arc::new(self.flattened_pixels(None)?),
        }];
        self.state.selected = 0;
        self.checkpoint(before, "Flatten")?;
        Ok(())
    }
}
