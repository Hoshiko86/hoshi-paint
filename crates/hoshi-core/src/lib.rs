//! UI-independent raster document operations. No filesystem or windowing required.

use std::collections::VecDeque;
use std::sync::Arc;

pub const MAX_DIMENSION: u32 = 262_144;
mod memory;
mod pixels;
pub use memory::{MemoryReservation, configure_memory_budget, memory_budget, memory_usage};
pub use pixels::{PixelStore, TILE_SIZE};
const HISTORY_LIMIT: usize = 16;
/// Maximum number of layers in a document, bounding dense pixel-buffer allocation.
pub const MAX_LAYERS: usize = 32;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const TRANSPARENT: Self = Self::rgba(0, 0, 0, 0);

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Straight-alpha source-over in encoded RGBA8 space, rounded to nearest.
    fn over(self, destination: Self, opacity: u8) -> Self {
        // Keep effective source alpha at full precision until the final channels
        // are rounded. Quantizing it to u8 first loses low-opacity colors.
        let alpha = u64::from(self.a) * u64::from(opacity);
        if alpha == 0 {
            // Preserve canonical transparent output even when the destination
            // carries RGB under zero alpha (including rounded low opacity).
            return if destination.a == 0 {
                Self::TRANSPARENT
            } else {
                destination
            };
        }
        if alpha == 65_025 {
            return self;
        }
        let back = u64::from(destination.a) * (65_025 - alpha);
        let denominator = alpha * 255 + back;
        if denominator == 0 {
            return Self::TRANSPARENT;
        }
        let channel = |source: u8, target: u8| {
            ((u64::from(source) * alpha * 255 + u64::from(target) * back + denominator / 2)
                / denominator) as u8
        };
        Self::rgba(
            channel(self.r, destination.r),
            channel(self.g, destination.g),
            channel(self.b, destination.b),
            ((denominator + 32_512) / 65_025) as u8,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layer {
    name: String,
    visible: bool,
    opacity: u8,
    pixels: Arc<PixelStore>,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShapeKind {
    Rectangle,
    Ellipse,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShapeStyle {
    Fill(Color),
    Outline {
        color: Color,
        width: u32,
    },
    Both {
        fill: Color,
        outline: Color,
        width: u32,
    },
}

/// Integer local bounds; outline is drawn inside the bounds. Rotation is about
/// the bounds' center, clockwise in screen coordinates, at whole-degree steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shape {
    pub kind: ShapeKind,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub rotation_degrees: i32,
    pub style: ShapeStyle,
}

impl Shape {
    fn validate(self) -> Result<(), Error> {
        if self.width == 0
            || self.height == 0
            || self.width > MAX_DIMENSION
            || self.height > MAX_DIMENSION
        {
            return Err(Error::InvalidShape);
        }
        match self.style {
            ShapeStyle::Outline { width: 0, .. } | ShapeStyle::Both { width: 0, .. } => {
                Err(Error::InvalidShape)
            }
            _ => Ok(()),
        }
    }

    fn contains(self, x: f64, y: f64, inset: f64) -> bool {
        let width = f64::from(self.width) - 2.0 * inset;
        let height = f64::from(self.height) - 2.0 * inset;
        if width <= 0.0 || height <= 0.0 {
            return false;
        }
        let left = f64::from(self.x) + inset;
        let top = f64::from(self.y) + inset;
        if x < left || y < top || x >= left + width || y >= top + height {
            return false;
        }
        match self.kind {
            ShapeKind::Rectangle => true,
            ShapeKind::Ellipse => {
                let dx = (x - left - width / 2.0) / (width / 2.0);
                let dy = (y - top - height / 2.0) / (height / 2.0);
                dx * dx + dy * dy <= 1.0
            }
        }
    }
}

/// Cache the transform once per raster operation rather than taking a sine and
/// cosine for every pixel. Exact quarter turns avoid floating-point edge drift.
#[derive(Clone, Copy)]
struct RasterShape {
    shape: Shape,
    center_x: f64,
    center_y: f64,
    sin: f64,
    cos: f64,
}

impl RasterShape {
    fn new(shape: Shape) -> Self {
        let angle = shape.rotation_degrees.rem_euclid(360);
        let (sin, cos) = match angle {
            0 => (0.0, 1.0),
            90 => (1.0, 0.0),
            180 => (0.0, -1.0),
            270 => (-1.0, 0.0),
            _ => f64::from(angle).to_radians().sin_cos(),
        };
        Self {
            shape,
            center_x: f64::from(shape.x) + f64::from(shape.width) / 2.0,
            center_y: f64::from(shape.y) + f64::from(shape.height) / 2.0,
            sin,
            cos,
        }
    }

    fn color_at(self, x: u32, y: u32) -> Option<Color> {
        let dx = f64::from(x) + 0.5 - self.center_x;
        let dy = f64::from(y) + 0.5 - self.center_y;
        let x = self.center_x + self.cos * dx + self.sin * dy;
        let y = self.center_y - self.sin * dx + self.cos * dy;
        if !self.shape.contains(x, y, 0.0) {
            return None;
        }
        match self.shape.style {
            ShapeStyle::Fill(color) => Some(color),
            ShapeStyle::Outline { color, width } => {
                (!self.shape.contains(x, y, f64::from(width))).then_some(color)
            }
            ShapeStyle::Both {
                fill,
                outline,
                width,
            } => Some(if self.shape.contains(x, y, f64::from(width)) {
                fill
            } else {
                outline
            }),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidDimensions,
    InvalidLayer,
    LastLayer,
    InvalidShape,
    ActiveShape,
    NoActiveShape,
    LayerLimit,
    InvalidHistoryStep,
    MemoryLimit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct State {
    width: u32,
    height: u32,
    resolution: u32,
    layers: Vec<Layer>,
    selected: usize,
}

/// A retained committed state in chronological undo/redo order. The first row
/// is the oldest state still available, not necessarily the original document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HistoryStep {
    pub label: &'static str,
    pub is_current: bool,
    pub is_redo: bool,
}

struct HistoryEntry {
    state: State,
    label: &'static str,
}

pub struct Document {
    state: State,
    active: Option<Shape>,
    undo: VecDeque<HistoryEntry>,
    redo: Vec<HistoryEntry>,
}

impl Document {
    pub fn new(width: u32, height: u32) -> Result<Self, Error> {
        Self::validate_image_size(width, height)?;
        let pixels = PixelStore::new(width, height, Color::TRANSPARENT);
        Ok(Self {
            state: State {
                width,
                height,
                resolution: 96_000,
                layers: vec![Layer {
                    name: "Background".into(),
                    visible: true,
                    opacity: 255,
                    pixels: Arc::new(pixels),
                }],
                selected: 0,
            },
            active: None,
            undo: VecDeque::new(),
            redo: Vec::new(),
        })
    }

    pub fn width(&self) -> u32 {
        self.state.width
    }
    pub fn height(&self) -> u32 {
        self.state.height
    }
    /// Layers are ordered bottom to top.
    pub fn layers(&self) -> &[Layer] {
        &self.state.layers
    }
    pub fn selected_layer(&self) -> usize {
        self.state.selected
    }
    pub fn active_shape(&self) -> Option<Shape> {
        self.active
    }
    pub fn undo_len(&self) -> usize {
        self.undo.len()
    }
    pub fn redo_len(&self) -> usize {
        self.redo.len()
    }

    /// The index of the current committed state in `history_steps()`.
    pub fn history_position(&self) -> usize {
        self.undo.len()
    }

    /// Includes retained undo states, the current committed state and redo states.
    /// An active shape is a preview and has no history row until it is committed.
    pub fn history_steps(&self) -> Vec<HistoryStep> {
        std::iter::once("Oldest available state")
            .chain(self.undo.iter().map(|entry| entry.label))
            .chain(self.redo.iter().rev().map(|entry| entry.label))
            .enumerate()
            .map(|(index, label)| HistoryStep {
                label,
                is_current: index == self.undo.len(),
                is_redo: index > self.undo.len(),
            })
            .collect()
    }

    /// Navigate retained committed states. Callers must explicitly commit or
    /// cancel a preview first, avoiding implicit commits evicting a clicked row.
    pub fn go_to_history_step(&mut self, index: usize) -> Result<bool, Error> {
        if index > self.undo.len() + self.redo.len() {
            return Err(Error::InvalidHistoryStep);
        }
        if self.active.is_some() {
            return Err(Error::ActiveShape);
        }
        let changed = index != self.undo.len();
        while self.undo.len() > index {
            self.undo()?;
        }
        while self.undo.len() < index {
            self.redo()?;
        }
        Ok(changed)
    }

    fn checkpoint(&mut self, before: State, label: &'static str) {
        if before == self.state {
            return;
        }
        if self.undo.len() == HISTORY_LIMIT {
            self.undo.pop_front();
        }
        self.undo.push_back(HistoryEntry {
            state: before,
            label,
        });
        self.redo.clear();
    }

    fn editable(&self, layer: usize) -> Result<(), Error> {
        if layer >= self.state.layers.len() {
            return Err(Error::InvalidLayer);
        }
        if self.active.is_some() {
            return Err(Error::ActiveShape);
        }
        Ok(())
    }

    /// Selecting a different layer first commits the active shape.
    pub fn select_layer(&mut self, layer: usize) -> Result<(), Error> {
        if layer >= self.state.layers.len() {
            return Err(Error::InvalidLayer);
        }
        if layer != self.state.selected {
            self.commit_shape()?;
            self.state.selected = layer;
        }
        Ok(())
    }

    pub fn add_layer(&mut self, name: impl Into<String>) -> Result<usize, Error> {
        self.editable(self.state.selected)?;
        if self.state.layers.len() >= MAX_LAYERS {
            return Err(Error::LayerLimit);
        }
        let before = self.state.clone();
        let index = self.state.layers.len();
        self.state.layers.push(Layer {
            name: name.into(),
            visible: true,
            opacity: 255,
            pixels: Arc::new(PixelStore::new(
                self.width(),
                self.height(),
                Color::TRANSPARENT,
            )),
        });
        self.state.selected = index;
        self.checkpoint(before, "Add layer");
        Ok(index)
    }

    pub fn delete_layer(&mut self, layer: usize) -> Result<(), Error> {
        self.editable(layer)?;
        if self.state.layers.len() == 1 {
            return Err(Error::LastLayer);
        }
        let before = self.state.clone();
        self.state.layers.remove(layer);
        if self.state.selected > layer {
            self.state.selected -= 1;
        }
        self.state.selected = self.state.selected.min(self.state.layers.len() - 1);
        self.checkpoint(before, "Delete layer");
        Ok(())
    }

    pub fn rename_layer(&mut self, layer: usize, name: impl Into<String>) -> Result<(), Error> {
        self.editable(layer)?;
        let before = self.state.clone();
        self.state.layers[layer].name = name.into();
        self.checkpoint(before, "Rename layer");
        Ok(())
    }

    pub fn set_layer_properties(
        &mut self,
        layer: usize,
        visible: bool,
        opacity: u8,
    ) -> Result<(), Error> {
        self.editable(layer)?;
        let before = self.state.clone();
        self.state.layers[layer].visible = visible;
        self.state.layers[layer].opacity = opacity;
        self.checkpoint(before, "Layer properties");
        Ok(())
    }

    pub fn move_layer(&mut self, from: usize, to: usize) -> Result<(), Error> {
        self.editable(from)?;
        self.editable(to)?;
        if from == to {
            return Ok(());
        }
        let before = self.state.clone();
        let layer = self.state.layers.remove(from);
        self.state.layers.insert(to, layer);
        self.state.selected = if self.state.selected == from {
            to
        } else if from < self.state.selected && self.state.selected <= to {
            self.state.selected - 1
        } else if to <= self.state.selected && self.state.selected < from {
            self.state.selected + 1
        } else {
            self.state.selected
        };
        self.checkpoint(before, "Reorder layer");
        Ok(())
    }

    pub fn begin_shape(&mut self, shape: Shape) -> Result<(), Error> {
        shape.validate()?;
        if self.active.is_some() {
            self.commit_shape()?;
        }
        self.active = Some(shape);
        Ok(())
    }

    pub fn adjust_shape(&mut self, shape: Shape) -> Result<(), Error> {
        shape.validate()?;
        if self.active.is_none() {
            return Err(Error::NoActiveShape);
        }
        self.active = Some(shape);
        Ok(())
    }

    pub fn cancel_shape(&mut self) {
        self.active = None;
    }

    /// Returns whether layer pixels changed. A no-op never adds history.
    pub fn commit_shape(&mut self) -> Result<bool, Error> {
        let Some(shape) = self.active else {
            return Ok(false);
        };
        let before = self.state.clone();
        let label = match shape.kind {
            ShapeKind::Rectangle => "Rectangle",
            ShapeKind::Ellipse => "Ellipse",
        };
        let raster = RasterShape::new(shape);
        let source = &self.state.layers[self.state.selected].pixels;
        let background = source.background();
        let solid = match shape.style {
            ShapeStyle::Fill(c) => Some(c),
            ShapeStyle::Both { fill, outline, .. } if fill == outline => Some(fill),
            ShapeStyle::Outline { color, width }
                if u64::from(width) * 2 >= u64::from(shape.width.min(shape.height)) =>
            {
                Some(color)
            }
            _ => None,
        };
        let corners = [
            (0, 0),
            (self.width() - 1, 0),
            (0, self.height() - 1),
            (self.width() - 1, self.height() - 1),
        ];
        if let Some(color) = solid
            && (color.a == 255 || source.is_uniform())
            && corners
                .into_iter()
                .all(|(x, y)| raster.color_at(x, y) == Some(color))
        {
            let result = color.over(background, 255);
            let pixels = PixelStore::new(self.width(), self.height(), result);
            let changed = **source != pixels;
            if changed {
                self.state.layers[self.state.selected].pixels = Arc::new(pixels);
            }
            self.active = None;
            self.checkpoint(before, label);
            return Ok(changed);
        }
        let unchanged_background = match shape.style {
            ShapeStyle::Fill(c) | ShapeStyle::Outline { color: c, .. } => {
                c.over(background, 255) == background
            }
            ShapeStyle::Both { fill, outline, .. } => {
                fill.over(background, 255) == background
                    && outline.over(background, 255) == background
            }
        };
        let mut pixels = (*self.state.layers[self.state.selected].pixels).clone();
        let angle = f64::from(shape.rotation_degrees).to_radians();
        let halfw = f64::from(shape.width) / 2.;
        let halfh = f64::from(shape.height) / 2.;
        let rx = halfw * angle.cos().abs() + halfh * angle.sin().abs();
        let ry = halfw * angle.sin().abs() + halfh * angle.cos().abs();
        let cx = f64::from(shape.x) + halfw;
        let cy = f64::from(shape.y) + halfh;
        let left = (cx - rx - 1.).max(0.).min(f64::from(self.width())) as u32;
        let right = (cx + rx + 1.).ceil().max(0.).min(f64::from(self.width())) as u32;
        let top = (cy - ry - 1.).max(0.).min(f64::from(self.height())) as u32;
        let bottom = (cy + ry + 1.).ceil().max(0.).min(f64::from(self.height())) as u32;
        let mut paint_rects = vec![(left, top, right, bottom)];
        if shape.kind == ShapeKind::Rectangle
            && shape.rotation_degrees.rem_euclid(90) == 0
            && let ShapeStyle::Outline { width, .. } = shape.style
            && f64::from(width) < halfw.min(halfh)
        {
            // Quarter-turn rectangle outlines are four strips. Visiting their
            // bounding-box interior would make a thin large outline quadratic.
            let iw = halfw - f64::from(width);
            let ih = halfh - f64::from(width);
            let irx = iw * angle.cos().abs() + ih * angle.sin().abs();
            let iry = iw * angle.sin().abs() + ih * angle.cos().abs();
            let il = (cx - irx).ceil().max(f64::from(left)).min(f64::from(right)) as u32;
            let ir = (cx + irx).floor().max(f64::from(il)).min(f64::from(right)) as u32;
            let it = (cy - iry).ceil().max(f64::from(top)).min(f64::from(bottom)) as u32;
            let ib = (cy + iry).floor().max(f64::from(it)).min(f64::from(bottom)) as u32;
            paint_rects = vec![
                (left, top, right, it),
                (left, ib, right, bottom),
                (left, it, il, ib),
                (ir, it, right, ib),
            ];
        }
        let rects: Vec<_> = if unchanged_background {
            pixels
                .allocated_rects()
                .flat_map(|(l, t, r, b)| {
                    paint_rects
                        .iter()
                        .map(move |&(pl, pt, pr, pb)| (l.max(pl), t.max(pt), r.min(pr), b.min(pb)))
                })
                .collect()
        } else {
            let mut bytes = 0u64;
            for &(l, t, r, b) in &paint_rects {
                if l < r && t < b {
                    let l = l / TILE_SIZE * TILE_SIZE;
                    let t = t / TILE_SIZE * TILE_SIZE;
                    let r = (r.div_ceil(TILE_SIZE) * TILE_SIZE).min(self.width());
                    let b = (b.div_ceil(TILE_SIZE) * TILE_SIZE).min(self.height());
                    bytes = bytes
                        .checked_add(u64::from(r - l) * u64::from(b - t) * 4)
                        .ok_or(Error::MemoryLimit)?;
                }
            }
            {
                let _check = MemoryReservation::new(bytes)?;
            }
            paint_rects
        };
        let mut changed = false;
        for (l, t, r, b) in rects {
            for y in t..b {
                for x in l..r {
                    if let Some(color) = raster.color_at(x, y) {
                        let previous = pixels.get(x, y);
                        let result = color.over(previous, 255);
                        if result != previous {
                            pixels.set(x, y, result)?;
                            changed = true;
                        }
                    }
                }
            }
        }
        if changed {
            self.state.layers[self.state.selected].pixels = Arc::new(pixels);
        }
        self.active = None;
        self.checkpoint(before, label);
        Ok(changed)
    }
    pub fn sampler(&self, include_preview: bool) -> CompositeSampler<'_> {
        CompositeSampler::new(
            &self.state,
            if include_preview { self.active } else { None },
        )
    }
    pub fn composite_pixel(&self, x: u32, y: u32, include_preview: bool) -> Color {
        self.sampler(include_preview).pixel(x, y)
    }
    pub fn try_composite(&self, include_preview: bool) -> Result<Vec<Color>, Error> {
        let size = self.width() as usize * self.height() as usize;
        let _memory = MemoryReservation::new(size as u64 * 4)?;
        let mut output = Vec::new();
        output
            .try_reserve_exact(size)
            .map_err(|_| Error::MemoryLimit)?;
        let sampler = self.sampler(include_preview);
        for y in 0..self.height() {
            for x in 0..self.width() {
                output.push(sampler.pixel(x, y));
            }
        }
        Ok(output)
    }
    /// Small-buffer reference helper. Production large-image paths stream/sample.
    pub fn composite(&self, include_preview: bool) -> Vec<Color> {
        self.try_composite(include_preview)
            .expect("dense composite exceeds memory budget; use sampler or try_composite")
    }
    pub fn has_transparency_fast(&self) -> bool {
        let sampler = self.sampler(true);
        let corners = [
            (0, 0),
            (self.width() - 1, 0),
            (0, self.height() - 1),
            (self.width() - 1, self.height() - 1),
        ];
        if corners
            .into_iter()
            .any(|(x, y)| sampler.pixel(x, y).a < 255)
        {
            return true;
        }
        if let Some(shape) = self.active {
            let opaque = match shape.style {
                ShapeStyle::Fill(color) => color.a == 255,
                ShapeStyle::Outline { color, width } => {
                    color.a == 255
                        && u64::from(width) * 2 >= u64::from(shape.width.min(shape.height))
                }
                ShapeStyle::Both { fill, outline, .. } => fill.a == 255 && outline.a == 255,
            };
            let layer = &self.state.layers[self.state.selected];
            // A filled rectangle/ellipse is convex. Opaque global corners imply
            // coverage of the entire canvas, including rotated shapes.
            if opaque && layer.visible && layer.opacity == 255 {
                let raster = RasterShape::new(shape);
                if corners
                    .into_iter()
                    .all(|(x, y)| raster.color_at(x, y).is_some_and(|c| c.a == 255))
                {
                    return false;
                }
            }
        }
        let mut background = Color::TRANSPARENT;
        let mut tiles = std::collections::HashSet::new();
        for layer in &self.state.layers {
            if layer.visible {
                background = layer.pixels.background().over(background, layer.opacity);
                tiles.extend(layer.pixels.tile_positions());
            }
        }
        if background.a == 255 {
            for (tx, ty) in tiles {
                for y in ty * TILE_SIZE..((ty + 1) * TILE_SIZE).min(self.height()) {
                    for x in tx * TILE_SIZE..((tx + 1) * TILE_SIZE).min(self.width()) {
                        if sampler.pixel(x, y).a < 255 {
                            return true;
                        }
                    }
                }
            }
            return false;
        }
        // Stop at the first transparent sample; empty huge canvases take O(1).
        for ty in 0..self.height().div_ceil(TILE_SIZE) {
            for tx in 0..self.width().div_ceil(TILE_SIZE) {
                if !tiles.contains(&(tx, ty)) && self.active.is_none() {
                    return true;
                }
                for y in ty * TILE_SIZE..((ty + 1) * TILE_SIZE).min(self.height()) {
                    for x in tx * TILE_SIZE..((tx + 1) * TILE_SIZE).min(self.width()) {
                        if sampler.pixel(x, y).a < 255 {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
    pub fn flattened_pixels(&self, matte: Option<Color>) -> Result<PixelStore, Error> {
        let mut background = Color::TRANSPARENT;
        let mut tiles = std::collections::HashSet::new();
        for layer in &self.state.layers {
            if layer.visible {
                background = layer.pixels.background().over(background, layer.opacity);
                tiles.extend(layer.pixels.tile_positions());
            }
        }
        if let Some(matte) = matte {
            background = background.over(matte, 255)
        }
        let mut pixels = PixelStore::new(self.width(), self.height(), background);
        let sampler = self.sampler(false);
        for (tx, ty) in tiles {
            for y in ty * TILE_SIZE..((ty + 1) * TILE_SIZE).min(self.height()) {
                for x in tx * TILE_SIZE..((tx + 1) * TILE_SIZE).min(self.width()) {
                    let mut color = sampler.pixel(x, y);
                    if let Some(matte) = matte {
                        color = color.over(matte, 255)
                    }
                    if color != background {
                        pixels.set(x, y, color)?;
                    }
                }
            }
        }
        Ok(pixels)
    }
    pub fn undo(&mut self) -> Result<bool, Error> {
        self.commit_shape()?;
        let Some(previous) = self.undo.pop_back() else {
            return Ok(false);
        };
        self.redo.push(HistoryEntry {
            state: std::mem::replace(&mut self.state, previous.state),
            label: previous.label,
        });
        Ok(true)
    }

    pub fn redo(&mut self) -> Result<bool, Error> {
        if self.active.is_some() {
            return Err(Error::ActiveShape);
        }
        let Some(next) = self.redo.pop() else {
            return Ok(false);
        };
        self.undo.push_back(HistoryEntry {
            state: std::mem::replace(&mut self.state, next.state),
            label: next.label,
        });
        Ok(true)
    }
}

#[cfg(test)]
mod color_tests;

mod canvas;
pub use canvas::{Anchor, ImageTransform, Resampling, ResizeOptions};
mod resampling;

mod project;
pub use project::{DocumentSnapshot, ProjectError};

/// Prepared shape geometry and sparse layer sampling, independent of image area.
pub struct CompositeSampler<'a> {
    state: &'a State,
    active: Option<RasterShape>,
}
impl<'a> CompositeSampler<'a> {
    fn new(state: &'a State, preview: Option<Shape>) -> Self {
        Self {
            state,
            active: preview.map(RasterShape::new),
        }
    }
    pub fn pixel(&self, x: u32, y: u32) -> Color {
        if x >= self.state.width || y >= self.state.height {
            return Color::TRANSPARENT;
        }
        let mut output = Color::TRANSPARENT;
        for (index, layer) in self.state.layers.iter().enumerate() {
            if !layer.visible {
                continue;
            }
            let mut color = layer.pixels.get(x, y);
            if index == self.state.selected
                && let Some(preview) = self.active.and_then(|s| s.color_at(x, y))
            {
                color = preview.over(color, 255)
            }
            output = color.over(output, layer.opacity);
        }
        output
    }
}
