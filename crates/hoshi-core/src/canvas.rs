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
    Lanczos,
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
        if width == 0 || height == 0 || u64::from(width) * u64::from(height) > MAX_PIXELS {
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
        self.checkpoint(before, "Resolution");
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
        document.state.layers[0].pixels = Arc::new(vec![color; (width * height) as usize]);
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
        for (index, layer) in self.state.layers.iter_mut().enumerate() {
            let mut pixels =
                vec![if index == 0 { fill } else { Color::TRANSPARENT }; (width * height) as usize];
            for y in 0..before.height {
                for x in 0..before.width {
                    let nx = i64::from(x) + dx;
                    let ny = i64::from(y) + dy;
                    if nx >= 0 && ny >= 0 && nx < i64::from(width) && ny < i64::from(height) {
                        pixels[ny as usize * width as usize + nx as usize] =
                            layer.pixels[(y * before.width + x) as usize];
                    }
                }
            }
            layer.pixels = Arc::new(pixels);
        }
        self.state.width = width;
        self.state.height = height;
        self.checkpoint(before, "Canvas Size");
        Ok(())
    }
    /// Resize each layer in premultiplied alpha to avoid dark fringes at transparency.
    pub fn resize_image(
        &mut self,
        width: u32,
        height: u32,
        resolution: f64,
        method: Resampling,
    ) -> Result<(), Error> {
        Self::validate_image_size(width, height)?;
        Self::validate_resolution(resolution)?;
        self.image_editable()?;
        let before = self.state.clone();
        if (width, height) != (self.width(), self.height()) {
            let filter = match method {
                Resampling::Nearest => image::imageops::FilterType::Nearest,
                Resampling::Bilinear => image::imageops::FilterType::Triangle,
                Resampling::Bicubic => image::imageops::FilterType::CatmullRom,
                Resampling::Lanczos => image::imageops::FilterType::Lanczos3,
            };
            for layer in &mut self.state.layers {
                let mut input = image::Rgba32FImage::new(before.width, before.height);
                for (pixel, color) in input.pixels_mut().zip(layer.pixels.iter()) {
                    let alpha = f32::from(color.a) / 255.0;
                    *pixel = image::Rgba([
                        f32::from(color.r) / 255.0 * alpha,
                        f32::from(color.g) / 255.0 * alpha,
                        f32::from(color.b) / 255.0 * alpha,
                        alpha,
                    ]);
                }
                let resized = image::imageops::resize(&input, width, height, filter);
                let pixels = resized
                    .pixels()
                    .map(|p| {
                        let alpha = p[3].clamp(0.0, 1.0);
                        if alpha <= 0.0 {
                            return Color::TRANSPARENT;
                        }
                        let channel =
                            |value: f32| ((value / alpha).clamp(0.0, 1.0) * 255.0).round() as u8;
                        Color::rgba(
                            channel(p[0]),
                            channel(p[1]),
                            channel(p[2]),
                            (alpha * 255.0).round() as u8,
                        )
                    })
                    .collect();
                layer.pixels = Arc::new(pixels);
            }
        }
        self.state.width = width;
        self.state.height = height;
        self.state.resolution = (resolution * 1000.0).round() as u32;
        self.checkpoint(before, "Resize");
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
        for layer in &mut self.state.layers {
            let mut pixels = vec![Color::TRANSPARENT; (width * height) as usize];
            for y in 0..before.height {
                for x in 0..before.width {
                    let (nx, ny) = match transform {
                        ImageTransform::FlipHorizontal => (before.width - 1 - x, y),
                        ImageTransform::FlipVertical => (x, before.height - 1 - y),
                        ImageTransform::RotateClockwise => (before.height - 1 - y, x),
                        ImageTransform::RotateCounterClockwise => (y, before.width - 1 - x),
                        ImageTransform::Rotate180 => (before.width - 1 - x, before.height - 1 - y),
                    };
                    pixels[(ny * width + nx) as usize] =
                        layer.pixels[(y * before.width + x) as usize];
                }
            }
            layer.pixels = Arc::new(pixels);
        }
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
        );
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
            pixels: Arc::new(self.composite(false)),
        }];
        self.state.selected = 0;
        self.checkpoint(before, "Flatten");
        Ok(())
    }
}
