//! Bounded preview sampling. This module does not mutate document or gesture state.
use super::{Color, Controller};
impl Controller {
    pub fn composite(&self) -> Vec<Color> {
        self.document.composite(true)
    }

    /// Bounded whole-image preview. Use `preview_size` to interpret its dimensions.
    /// Actual canvas display uses `render_viewport` for large images.
    pub fn preview_rgba(&self) -> Vec<u8> {
        let (width, height) = self.preview_size();
        self.thumbnail(width, height).rgba
    }

    pub fn preview_size(&self) -> (u32, u32) {
        let width = self.document.width();
        let height = self.document.height();
        if u64::from(width) * u64::from(height) <= 1_048_576 {
            return (width, height);
        }
        let scale = (1024. / f64::from(width)).min(1024. / f64::from(height));
        (
            (f64::from(width) * scale).round().max(1.) as u32,
            (f64::from(height) * scale).round().max(1.) as u32,
        )
    }

    /// Checkerboard-composited, aspect-preserving thumbnail with transparent margins.
    /// Cost is proportional to the requested preview, never the document dimensions.
    pub fn thumbnail(&self, width: u32, height: u32) -> PreviewImage {
        let (width, height) = bounded_preview_size(width, height, 1.);
        let mut rgba = vec![0; width as usize * height as usize * 4];
        let fit = (f64::from(width) / f64::from(self.document.width()))
            .min(f64::from(height) / f64::from(self.document.height()));
        let fitted_width = (f64::from(self.document.width()) * fit).max(1.);
        let fitted_height = (f64::from(self.document.height()) * fit).max(1.);
        let left = (f64::from(width) - fitted_width) / 2.;
        let top = (f64::from(height) - fitted_height) / 2.;
        let sampler = self.document.sampler(true);
        let xs = preview_axis(
            width,
            f64::from(width),
            left,
            self.document.width(),
            fitted_width / f64::from(self.document.width()),
        );
        let ys = preview_axis(
            height,
            f64::from(height),
            top,
            self.document.height(),
            fitted_height / f64::from(self.document.height()),
        );
        sample_preview(&mut rgba, width, &xs, &ys, |x, y| sampler.pixel(x, y));
        PreviewImage {
            width,
            height,
            rgba,
        }
    }

    /// Rasterize only what is visible on screen, with a bounded HiDPI texture.
    /// Screen coordinates remain logical; sampling resolution follows the scale factor.
    pub fn render_viewport(
        &self,
        logical_width: u32,
        logical_height: u32,
        scale_factor: f64,
    ) -> PreviewImage {
        let (width, height) = bounded_preview_size(logical_width, logical_height, scale_factor);
        let mut rgba = vec![0; width as usize * height as usize * 4];
        let sampler = self.document.sampler(true);
        let xs = preview_axis(
            width,
            f64::from(logical_width),
            self.viewport.offset_x,
            self.document.width(),
            self.viewport.zoom,
        );
        let ys = preview_axis(
            height,
            f64::from(logical_height),
            self.viewport.offset_y,
            self.document.height(),
            self.viewport.zoom,
        );
        sample_preview(&mut rgba, width, &xs, &ys, |x, y| sampler.pixel(x, y));
        PreviewImage {
            width,
            height,
            rgba,
        }
    }
}

/// A small CPU upload texture, independent of canvas storage size.
#[derive(Debug)]
pub struct PreviewImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}
fn bounded_preview_size(width: u32, height: u32, scale_factor: f64) -> (u32, u32) {
    let width = width.max(1);
    let height = height.max(1);
    let scale_factor = if scale_factor.is_finite() {
        scale_factor.clamp(0.25, 4.)
    } else {
        1.
    };
    let scale = scale_factor
        .min(4096. / f64::from(width))
        .min(4096. / f64::from(height))
        .min((4_194_304. / (f64::from(width) * f64::from(height))).sqrt());
    (
        (f64::from(width) * scale).floor().max(1.) as u32,
        (f64::from(height) * scale).floor().max(1.) as u32,
    )
}
/// A screen pixel that intersects a very thin image still samples that image.
/// This keeps one-pixel-wide panoramas visible when fitting a huge canvas.
fn preview_axis(
    pixels: u32,
    logical_extent: f64,
    offset: f64,
    image_extent: u32,
    zoom: f64,
) -> Vec<Option<u32>> {
    let end = offset + f64::from(image_extent) * zoom;
    (0..pixels)
        .map(|p| {
            let left = f64::from(p) * logical_extent / f64::from(pixels);
            let right = f64::from(p + 1) * logical_extent / f64::from(pixels);
            if right > offset && left < end {
                Some(
                    (((left + right) * 0.5 - offset) / zoom)
                        .floor()
                        .clamp(0., f64::from(image_extent - 1)) as u32,
                )
            } else {
                None
            }
        })
        .collect()
}
fn sample_preview(
    rgba: &mut [u8],
    width: u32,
    xs: &[Option<u32>],
    ys: &[Option<u32>],
    mut pixel: impl FnMut(u32, u32) -> Color,
) {
    for (y, sy) in ys.iter().enumerate() {
        let Some(sy) = *sy else { continue };
        for (x, sx) in xs.iter().enumerate() {
            if let Some(sx) = *sx {
                let index = (y * width as usize + x) * 4;
                rgba[index..index + 4].copy_from_slice(&preview_pixel(pixel(sx, sy), sx, sy));
            }
        }
    }
}
fn preview_pixel(pixel: Color, x: u32, y: u32) -> [u8; 4] {
    if pixel.a == 255 {
        return [pixel.r, pixel.g, pixel.b, 255];
    }
    let background = if (x / 8 + y / 8).is_multiple_of(2) {
        224
    } else {
        192
    };
    let channel = |value: u8| {
        ((u32::from(value) * u32::from(pixel.a) + background * (255 - u32::from(pixel.a)) + 127)
            / 255) as u8
    };
    [channel(pixel.r), channel(pixel.g), channel(pixel.b), 255]
}
