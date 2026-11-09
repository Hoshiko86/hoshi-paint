//! Separable resampling in premultiplied RGBA, with optional sRGB linearization.
use crate::{Color, Error, MemoryReservation, PixelStore, Resampling, ResizeOptions};
use std::collections::{HashMap, HashSet, VecDeque};

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
    _memory: MemoryReservation,
}
impl Axis {
    fn new(source: usize, target: usize, method: Resampling) -> Result<Self, Error> {
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
        // Tap storage grows with axis lengths, not with image area. Reserve an
        // upper bound before allocating so downsampling cannot surprise the budget.
        let tap_limit = if source == target {
            target
        } else if matches!(kernel, Kernel::Area) {
            source.checked_add(target).ok_or(Error::MemoryLimit)?
        } else {
            target
                .checked_mul((2.0 * support * scale).ceil() as usize + 2)
                .ok_or(Error::MemoryLimit)?
        };
        let bytes = (target as u64 + 1) * std::mem::size_of::<usize>() as u64
            + tap_limit as u64 * std::mem::size_of::<(usize, f64)>() as u64;
        let memory = MemoryReservation::new(bytes)?;
        let mut offsets = Vec::new();
        offsets
            .try_reserve_exact(target + 1)
            .map_err(|_| Error::MemoryLimit)?;
        let mut taps = Vec::new();
        taps.try_reserve_exact(tap_limit)
            .map_err(|_| Error::MemoryLimit)?;
        let mut axis = Self {
            offsets,
            taps,
            _memory: memory,
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
        Ok(axis)
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
const ROW_CACHE_BYTES: usize = 8 * 1024 * 1024;
const ROW_CACHE_ENTRIES: usize = 64;
struct Row {
    source_y: u32,
    start: u32,
    pixels: Vec<[f64; 4]>,
    _memory: MemoryReservation,
}
fn float_row(length: usize) -> Result<(Vec<[f64; 4]>, MemoryReservation), Error> {
    let memory = MemoryReservation::new(length as u64 * 32)?;
    let mut row = Vec::new();
    row.try_reserve_exact(length)
        .map_err(|_| Error::MemoryLimit)?;
    row.resize(length, [0.0; 4]);
    Ok((row, memory))
}
fn linear_color(c: Color, decoded: &[f64; 256]) -> [f64; 4] {
    let alpha = f64::from(c.a) / 255.;
    [
        decoded[c.r as usize] * alpha,
        decoded[c.g as usize] * alpha,
        decoded[c.b as usize] * alpha,
        alpha,
    ]
}
fn horizontal_row(
    input: &PixelStore,
    source_y: u32,
    range: (u32, u32),
    transposed: bool,
    weights: &Axis,
    decoded: &[f64; 256],
    source_rows: &HashMap<u32, Vec<(u32, u32)>>,
) -> Result<Row, Error> {
    let (start, end) = range;
    let (mut pixels, memory) = float_row((end - start) as usize)?;
    let base = linear_color(input.background(), decoded);
    let Some(spans) = source_rows.get(&(source_y / crate::TILE_SIZE)) else {
        pixels.fill(base);
        return Ok(Row {
            source_y,
            start,
            pixels,
            _memory: memory,
        });
    };
    for (offset, p) in pixels.iter_mut().enumerate() {
        let weights = weights.weights(start as usize + offset);
        if weights.len() > 1024 {
            // Wide filters are background plus the weighted deviation of
            // allocated tiles, rather than a scan through implicit pixels.
            *p = base;
            for &(left, right) in spans {
                let lo = weights.partition_point(|&(x, _)| x < left as usize);
                let hi = weights.partition_point(|&(x, _)| x < right as usize);
                for &(source_x, weight) in &weights[lo..hi] {
                    let c = if transposed {
                        input.get(source_y, source_x as u32)
                    } else {
                        input.get(source_x as u32, source_y)
                    };
                    let sample = linear_color(c, decoded);
                    for channel in 0..4 {
                        p[channel] += (sample[channel] - base[channel]) * weight;
                    }
                }
            }
        } else {
            for &(source_x, weight) in weights {
                let c = if transposed {
                    input.get(source_y, source_x as u32)
                } else {
                    input.get(source_x as u32, source_y)
                };
                let sample = linear_color(c, decoded);
                for channel in 0..4 {
                    p[channel] += sample[channel] * weight;
                }
            }
        }
    }
    Ok(Row {
        source_y,
        start,
        pixels,
        _memory: memory,
    })
}
fn filtered_color(p: [f64; 4], gamma: bool) -> Color {
    let alpha = p[3].clamp(0.0, 1.0);
    let a = (alpha * 255.0).round() as u8;
    if a == 0 {
        return Color::TRANSPARENT;
    }
    Color::rgba(
        // Unpremultiply before clamping coverage. Sharp filters may overshoot
        // alpha1; dividing by the clipped value would saturate otherwise-constant
        // RGB along translucent edges.
        encode(p[0] / p[3], gamma),
        encode(p[1] / p[3], gamma),
        encode(p[2] / p[3], gamma),
        a,
    )
}
fn source_padding(source: u32, target: u32, method: Resampling) -> f64 {
    if source == target || method == Resampling::Nearest {
        return 0.;
    }
    let ratio = f64::from(source) / f64::from(target);
    match method {
        Resampling::BilinearLowQuality => 1.,
        Resampling::Bilinear => ratio.max(1.),
        Resampling::Bicubic | Resampling::BicubicSmooth => 2. * ratio.max(1.),
        Resampling::Fant if ratio > 1. => 0.,
        Resampling::Fant => 1.,
        Resampling::AdaptiveSharp if ratio <= 1. => 2.,
        _ => 3. * ratio.max(1.),
    }
}
fn affected_axis(left: u32, right: u32, source: u32, target: u32, padding: f64) -> (u32, u32) {
    let scale = f64::from(target) / f64::from(source);
    let start = ((f64::from(left) - padding) * scale).floor() - 2.;
    let end = ((f64::from(right) + padding) * scale).ceil() + 2.;
    (
        start.clamp(0., f64::from(target)) as u32,
        end.clamp(0., f64::from(target)) as u32,
    )
}

/// Estimate the union of affected destination tiles without allocating those
/// pixels. Partial edge tiles use only their valid width/height, and overlapping
/// filter footprints count once. The estimate is deliberately
/// conservative when filtering later cancels pixels back to the background.
fn output_footprint(
    rectangles: &[(u32, u32, u32, u32)],
    target: (u32, u32),
) -> Result<(u64, u32), Error> {
    let mut events = Vec::new();
    events
        .try_reserve_exact(rectangles.len() * 2)
        .map_err(|_| Error::MemoryLimit)?;
    for (id, &(_, top, _, bottom)) in rectangles.iter().enumerate() {
        events.push((top / crate::TILE_SIZE, true, id));
        events.push((bottom.div_ceil(crate::TILE_SIZE), false, id));
    }
    events.sort_unstable();
    let mut active = HashSet::new();
    active
        .try_reserve(rectangles.len())
        .map_err(|_| Error::MemoryLimit)?;
    let mut spans = Vec::new();
    spans
        .try_reserve_exact(rectangles.len())
        .map_err(|_| Error::MemoryLimit)?;
    let (mut position, mut pixels, mut max_row) = (0, 0u64, 0u32);
    let span = |start: u32, end: u32, limit: u32| {
        (end * crate::TILE_SIZE).min(limit) - (start * crate::TILE_SIZE).min(limit)
    };
    while position < events.len() {
        let top = events[position].0;
        while position < events.len() && events[position].0 == top {
            let (_, add, id) = events[position];
            if add {
                active.insert(id);
            } else {
                active.remove(&id);
            }
            position += 1;
        }
        let bottom = events.get(position).map_or(top, |e| e.0);
        if top == bottom || active.is_empty() {
            continue;
        }
        spans.clear();
        spans.extend(active.iter().map(|&id| {
            (
                rectangles[id].0 / crate::TILE_SIZE,
                rectangles[id].2.div_ceil(crate::TILE_SIZE),
            )
        }));
        spans.sort_unstable();
        let mut start = spans[0].0;
        let mut end = spans[0].1;
        let mut row_pixels = 0u64;
        for &(left, right) in &spans[1..] {
            if left <= end {
                end = end.max(right);
            } else {
                let width = span(start, end, target.0);
                row_pixels += u64::from(width);
                max_row = max_row.max(width);
                (start, end) = (left, right);
            }
        }
        let width = span(start, end, target.0);
        row_pixels += u64::from(width);
        max_row = max_row.max(width);
        pixels = pixels
            .checked_add(row_pixels * u64::from(span(top, bottom, target.1)))
            .ok_or(Error::MemoryLimit)?;
    }
    let bytes = pixels
        .checked_mul(std::mem::size_of::<Color>() as u64)
        .ok_or(Error::MemoryLimit)?;
    Ok((bytes, max_row))
}

/// Scanline filtering keeps the floating-point working set independent of image
/// area. Only destination regions touched by allocated source tiles are visited.
/// Uniform layers, including maximum-size empty canvases, resize without buffers.
pub(crate) fn resize(
    input: &PixelStore,
    source: (u32, u32),
    target: (u32, u32),
    options: ResizeOptions,
) -> Result<PixelStore, Error> {
    if source == target {
        return Ok(input.clone());
    }
    // A tiny edited canvas can be logically uniform even if it retains one
    // allocated tile. Recognize that before a very large enlargement.
    if u64::from(source.0) * u64::from(source.1) <= u64::from(crate::TILE_SIZE).pow(2) {
        let color = input.get(0, 0);
        if input.iter().all(|&p| p == color) {
            let color = if options.method == Resampling::Nearest || color.a != 0 {
                color
            } else {
                Color::TRANSPARENT
            };
            return Ok(PixelStore::new(target.0, target.1, color));
        }
    }
    let background = if options.method == Resampling::Nearest || input.background().a != 0 {
        input.background()
    } else {
        Color::TRANSPARENT
    };
    let mut output = PixelStore::new(target.0, target.1, background);
    if input.is_uniform() {
        return Ok(output);
    }
    // Keep the reference implementation's pass ordering, including its rounding
    // behavior, but transpose access instead of building the other full frame.
    let transposed = options.method != Resampling::Nearest
        && u64::from(target.0) * u64::from(source.1) > u64::from(source.0) * u64::from(target.1);
    let ((sw, sh), (dw, dh)) = if transposed {
        ((source.1, source.0), (target.1, target.0))
    } else {
        (source, target)
    };
    let padding_x = source_padding(sw, dw, options.method);
    let padding_y = source_padding(sh, dh, options.method);
    let region_count = input.allocated_rects().count();
    let _regions_memory = MemoryReservation::new(region_count as u64 * 256)?;
    let mut source_rows: HashMap<u32, Vec<(u32, u32)>> = HashMap::new();
    source_rows
        .try_reserve(region_count)
        .map_err(|_| Error::MemoryLimit)?;
    let mut rectangles = Vec::new();
    rectangles
        .try_reserve_exact(region_count)
        .map_err(|_| Error::MemoryLimit)?;
    for (left, top, right, bottom) in input.allocated_rects() {
        // Tighten a partial tile to its changed pixels. Scaling a single pixel
        // by 512x must not visit the whole 128x128 source tile's 65536² footprint.
        let (mut x0, mut y0, mut x1, mut y1) = (right, bottom, left, top);
        for y in top..bottom {
            for x in left..right {
                let color = input.get(x, y);
                if color != input.background()
                    && (options.method == Resampling::Nearest
                        || color.a != 0
                        || input.background().a != 0)
                {
                    x0 = x0.min(x);
                    y0 = y0.min(y);
                    x1 = x1.max(x + 1);
                    y1 = y1.max(y + 1);
                }
            }
        }
        if x0 >= x1 || y0 >= y1 {
            continue;
        }
        let (left, top, right, bottom) = (x0, y0, x1, y1);
        let (left, top, right, bottom) = if transposed {
            (top, left, bottom, right)
        } else {
            (left, top, right, bottom)
        };
        let spans = source_rows.entry(top / crate::TILE_SIZE).or_default();
        spans.try_reserve(1).map_err(|_| Error::MemoryLimit)?;
        spans.push((left, right));
        let (x0, x1) = affected_axis(left, right, sw, dw, padding_x);
        let (y0, y1) = affected_axis(top, bottom, sh, dh, padding_y);
        if x0 < x1 && y0 < y1 {
            rectangles.push((x0, y0, x1, y1));
        }
    }
    if rectangles.is_empty() {
        return Ok(output);
    }
    let (output_bytes, max_row) = output_footprint(&rectangles, (dw, dh))?;
    // Fail an obviously over-budget enlargement before building filter tables or
    // entering its pixel loops. Real tiles still reserve their memory as written.
    drop(MemoryReservation::new(output_bytes)?);
    for spans in source_rows.values_mut() {
        spans.sort_unstable();
        let mut merged = 0;
        for i in 0..spans.len() {
            let (start, end) = spans[i];
            if merged > 0 && start <= spans[merged - 1].1 {
                spans[merged - 1].1 = spans[merged - 1].1.max(end)
            } else {
                spans[merged] = (start, end);
                merged += 1;
            }
        }
        spans.truncate(merged);
    }
    let mut events = Vec::new();
    events
        .try_reserve_exact(rectangles.len() * 2)
        .map_err(|_| Error::MemoryLimit)?;
    for (id, &(_, top, _, bottom)) in rectangles.iter().enumerate() {
        events.push((top, true, id));
        events.push((bottom, false, id));
    }
    events.sort_unstable();
    let (xweights, yweights) = if options.method == Resampling::Nearest {
        (None, None)
    } else {
        (
            Some(Axis::new(sw as usize, dw as usize, options.method)?),
            Some(Axis::new(sh as usize, dh as usize, options.method)?),
        )
    };
    let cache_metadata = (ROW_CACHE_ENTRIES * std::mem::size_of::<Row>()) as u64;
    let scratch_bytes = if options.method == Resampling::Nearest {
        cache_metadata
    } else {
        let row_bytes = u64::from(max_row.min(dw)) * 32;
        row_bytes
            + (ROW_CACHE_BYTES as u64).min(row_bytes * ROW_CACHE_ENTRIES as u64)
            + cache_metadata
    };
    // Axis tables are now live, so this includes their cost together with all
    // documents/history. No transient preflight reservation is double charged.
    drop(MemoryReservation::new(
        output_bytes
            .checked_add(scratch_bytes)
            .ok_or(Error::MemoryLimit)?,
    )?);
    let decoded: [f64; 256] = std::array::from_fn(|i| decode(i as u8, options.gamma_correction));
    let mut active = HashSet::new();
    active
        .try_reserve(rectangles.len())
        .map_err(|_| Error::MemoryLimit)?;
    let mut intervals = Vec::new();
    intervals
        .try_reserve_exact(rectangles.len())
        .map_err(|_| Error::MemoryLimit)?;
    let _cache_metadata =
        MemoryReservation::new((ROW_CACHE_ENTRIES * std::mem::size_of::<Row>()) as u64)?;
    let mut cache: VecDeque<Row> = VecDeque::new();
    cache
        .try_reserve_exact(ROW_CACHE_ENTRIES)
        .map_err(|_| Error::MemoryLimit)?;
    let mut cache_bytes = 0usize;
    let mut position = 0;
    while position < events.len() {
        let y = events[position].0;
        while position < events.len() && events[position].0 == y {
            let (_, add, id) = events[position];
            if add {
                active.insert(id);
            } else {
                active.remove(&id);
            }
            position += 1;
        }
        let next_y = events.get(position).map_or(dh, |e| e.0);
        if active.is_empty() {
            continue;
        }
        intervals.clear();
        intervals.extend(
            active
                .iter()
                .map(|&id| (rectangles[id].0, rectangles[id].2)),
        );
        intervals.sort_unstable();
        let mut merged = 0;
        for i in 0..intervals.len() {
            let (start, end) = intervals[i];
            if merged > 0 && start <= intervals[merged - 1].1 {
                intervals[merged - 1].1 = intervals[merged - 1].1.max(end);
            } else {
                intervals[merged] = (start, end);
                merged += 1;
            }
        }
        for dy in y..next_y {
            for &(start, end) in &intervals[..merged] {
                if options.method == Resampling::Nearest {
                    let sy = (((u64::from(dy) * 2 + 1) * u64::from(sh)) / (u64::from(dh) * 2))
                        .min(u64::from(sh - 1)) as u32;
                    for dx in start..end {
                        let sx = (((u64::from(dx) * 2 + 1) * u64::from(sw)) / (u64::from(dw) * 2))
                            .min(u64::from(sw - 1)) as u32;
                        output.set(dx, dy, input.get(sx, sy))?;
                    }
                    continue;
                }
                let (mut row, _row_memory) = float_row((end - start) as usize)?;
                let xweights = xweights.as_ref().unwrap();
                let vertical_weights = yweights.as_ref().unwrap().weights(dy as usize);
                let sparse_vertical = vertical_weights.len() > 1024;
                let base = linear_color(input.background(), &decoded);
                if sparse_vertical {
                    row.fill(base);
                }
                for &(sy, weight) in vertical_weights {
                    if sparse_vertical && !source_rows.contains_key(&(sy as u32 / crate::TILE_SIZE))
                    {
                        continue;
                    }
                    let cached = cache.iter().position(|r| {
                        r.source_y == sy as u32 && r.start == start && r.pixels.len() == row.len()
                    });
                    let index = if let Some(index) = cached {
                        index
                    } else {
                        let bytes = row.len() * 32;
                        while !cache.is_empty()
                            && (cache_bytes + bytes > ROW_CACHE_BYTES
                                || cache.len() >= ROW_CACHE_ENTRIES)
                        {
                            cache_bytes -= cache.pop_front().unwrap().pixels.len() * 32;
                        }
                        let horizontal = horizontal_row(
                            input,
                            sy as u32,
                            (start, end),
                            transposed,
                            xweights,
                            &decoded,
                            &source_rows,
                        )?;
                        cache_bytes += bytes;
                        cache.push_back(horizontal);
                        cache.len() - 1
                    };
                    for (p, sample) in row.iter_mut().zip(&cache[index].pixels) {
                        for channel in 0..4 {
                            let value = if sparse_vertical {
                                sample[channel] - base[channel]
                            } else {
                                sample[channel]
                            };
                            p[channel] += value * weight;
                        }
                    }
                }
                for (offset, p) in row.into_iter().enumerate() {
                    let dx = start + offset as u32;
                    let (x, y) = if transposed { (dy, dx) } else { (dx, dy) };
                    output.set(x, y, filtered_color(p, options.gamma_correction))?;
                }
            }
        }
    }
    Ok(output)
}

#[cfg(test)]
mod tests;
