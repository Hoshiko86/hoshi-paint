//! Separable resampling in premultiplied RGBA, with optional sRGB linearization.
use crate::{Color, Resampling, ResizeOptions};

#[derive(Clone, Copy)]
enum Kernel {
    Triangle,
    Catrom,
    Smooth,
    Lanczos,
    Area,
}
struct Axis {
    offsets: Vec<usize>,
    taps: Vec<(usize, f64)>,
}
impl Axis {
    fn new(source: usize, target: usize, method: Resampling) -> Self {
        let ratio = source as f64 / target as f64;
        let kernel = match method {
            Resampling::Bicubic => Kernel::Catrom,
            Resampling::BicubicSmooth => Kernel::Smooth,
            Resampling::Bilinear | Resampling::BilinearLowQuality => Kernel::Triangle,
            Resampling::Lanczos => Kernel::Lanczos,
            Resampling::Fant if ratio > 1.0 => Kernel::Area,
            Resampling::Fant => Kernel::Triangle,
            // Paint.NET does not publish its exact adaptive selection rule.
            // Our policy selects a sharp antialiased filter independently per axis.
            Resampling::AdaptiveSharp if ratio > 1.0 => Kernel::Lanczos,
            Resampling::AdaptiveSharp => Kernel::Catrom,
            Resampling::Nearest => unreachable!("nearest bypasses color conversion"),
        };
        let scale = if method == Resampling::BilinearLowQuality {
            1.0
        } else {
            ratio.max(1.0)
        };
        let support = match kernel {
            Kernel::Triangle => 1.0,
            Kernel::Catrom | Kernel::Smooth => 2.0,
            Kernel::Lanczos => 3.0,
            Kernel::Area => 0.0,
        };
        let mut axis = Self {
            offsets: Vec::with_capacity(target + 1),
            taps: Vec::new(),
        };
        for position in 0..target {
            axis.offsets.push(axis.taps.len());
            if source == target {
                axis.taps.push((position, 1.0));
                continue;
            }
            if matches!(kernel, Kernel::Area) {
                let left = position as f64 * ratio;
                let right = (position + 1) as f64 * ratio;
                for index in (left.floor() as usize)..(right.ceil() as usize).min(source) {
                    let overlap = right.min((index + 1) as f64) - left.max(index as f64);
                    if overlap > 0.0 {
                        axis.taps.push((index, overlap / ratio));
                    }
                }
            } else {
                let center = (position as f64 + 0.5) * ratio - 0.5;
                let start = ((center - support * scale).ceil() as i64).max(0) as usize;
                let end =
                    ((center + support * scale).floor() as i64).min(source as i64 - 1) as usize;
                for index in start..=end {
                    let distance = (index as f64 - center).abs() / scale;
                    let weight = match kernel {
                        Kernel::Triangle => (1.0 - distance).max(0.0),
                        Kernel::Catrom => cubic(distance, 0.0, 0.5),
                        Kernel::Smooth => cubic(distance, 1.0, 0.0),
                        Kernel::Lanczos if distance < 3.0 => sinc(distance) * sinc(distance / 3.0),
                        _ => 0.0,
                    };
                    if weight != 0.0 {
                        axis.taps.push((index, weight));
                    }
                }
            }
            let start = axis.offsets[position];
            let sum: f64 = axis.taps[start..].iter().map(|(_, weight)| *weight).sum();
            debug_assert!(sum.is_finite() && sum.abs() > 1e-12);
            for (_, weight) in &mut axis.taps[start..] {
                *weight /= sum;
            }
        }
        axis.offsets.push(axis.taps.len());
        axis
    }
    fn weights(&self, position: usize) -> &[(usize, f64)] {
        &self.taps[self.offsets[position]..self.offsets[position + 1]]
    }
}
fn cubic(x: f64, b: f64, c: f64) -> f64 {
    if x < 1.0 {
        ((12.0 - 9.0 * b - 6.0 * c) * x.powi(3) + (-18.0 + 12.0 * b + 6.0 * c) * x * x + 6.0
            - 2.0 * b)
            / 6.0
    } else if x < 2.0 {
        ((-b - 6.0 * c) * x.powi(3)
            + (6.0 * b + 30.0 * c) * x * x
            + (-12.0 * b - 48.0 * c) * x
            + 8.0 * b
            + 24.0 * c)
            / 6.0
    } else {
        0.0
    }
}
fn sinc(x: f64) -> f64 {
    if x.abs() < 1e-12 {
        1.0
    } else {
        let x = std::f64::consts::PI * x;
        x.sin() / x
    }
}
fn decode(value: u8, gamma: bool) -> f64 {
    let x = f64::from(value) / 255.0;
    if !gamma {
        x
    } else if x <= 0.04045 {
        x / 12.92
    } else {
        ((x + 0.055) / 1.055).powf(2.4)
    }
}
fn encode(value: f64, gamma: bool) -> u8 {
    let x = value.clamp(0.0, 1.0);
    let x = if !gamma {
        x
    } else if x <= 0.0031308 {
        x * 12.92
    } else {
        1.055 * x.powf(1.0 / 2.4) - 0.055
    };
    (x * 255.0).round() as u8
}
fn horizontal(
    input: &[[f64; 4]],
    width: usize,
    height: usize,
    target: usize,
    weights: &Axis,
) -> Vec<[f64; 4]> {
    let mut output = vec![[0.0; 4]; target * height];
    for y in 0..height {
        for x in 0..target {
            for &(sx, weight) in weights.weights(x) {
                for channel in 0..4 {
                    output[y * target + x][channel] += input[y * width + sx][channel] * weight;
                }
            }
        }
    }
    output
}
fn vertical(input: &[[f64; 4]], width: usize, target: usize, weights: &Axis) -> Vec<[f64; 4]> {
    let mut output = vec![[0.0; 4]; width * target];
    for y in 0..target {
        for &(sy, weight) in weights.weights(y) {
            for x in 0..width {
                for channel in 0..4 {
                    output[y * width + x][channel] += input[sy * width + x][channel] * weight;
                }
            }
        }
    }
    output
}
pub(crate) fn resize(
    input: &[Color],
    source: (u32, u32),
    target: (u32, u32),
    options: ResizeOptions,
) -> Vec<Color> {
    let (sw, sh, dw, dh) = (
        source.0 as usize,
        source.1 as usize,
        target.0 as usize,
        target.1 as usize,
    );
    if source == target {
        return input.to_vec();
    }
    if options.method == Resampling::Nearest {
        return (0..dw * dh)
            .map(|i| {
                // Pixel centers; copy stored RGBA exactly, including transparent RGB.
                let x = (((i % dw) as u64 * 2 + 1) * sw as u64 / (dw as u64 * 2)) as usize;
                let y = (((i / dw) as u64 * 2 + 1) * sh as u64 / (dh as u64 * 2)) as usize;
                input[y.min(sh - 1) * sw + x.min(sw - 1)]
            })
            .collect();
    }
    let xweights = Axis::new(sw, dw, options.method);
    let yweights = Axis::new(sh, dh, options.method);
    let decoded: [f64; 256] = std::array::from_fn(|i| decode(i as u8, options.gamma_correction));
    let linear: Vec<_> = input
        .iter()
        .map(|c| {
            let a = f64::from(c.a) / 255.0;
            [
                decoded[c.r as usize] * a,
                decoded[c.g as usize] * a,
                decoded[c.b as usize] * a,
                a,
            ]
        })
        .collect();
    // Pick the smaller intermediate: both endpoints fit the document limit, but
    // a fixed pass order could allocate a huge buffer for opposing axis ratios.
    let filtered = if dw as u64 * sh as u64 <= sw as u64 * dh as u64 {
        vertical(
            &horizontal(&linear, sw, sh, dw, &xweights),
            dw,
            dh,
            &yweights,
        )
    } else {
        horizontal(&vertical(&linear, sw, dh, &yweights), sw, dh, dw, &xweights)
    };
    filtered
        .into_iter()
        .map(|p| {
            let alpha = p[3].clamp(0.0, 1.0);
            let a = (alpha * 255.0).round() as u8;
            if a == 0 {
                return Color::TRANSPARENT;
            }
            Color::rgba(
                encode(p[0] / alpha, options.gamma_correction),
                encode(p[1] / alpha, options.gamma_correction),
                encode(p[2] / alpha, options.gamma_correction),
                a,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests;
