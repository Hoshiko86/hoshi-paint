//! UI-independent raster document operations. No filesystem or windowing required.

use std::collections::VecDeque;
use std::sync::Arc;

const MAX_PIXELS: u64 = 1_048_576;
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
    pixels: Arc<Vec<Color>>,
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
    pub fn pixels(&self) -> &[Color] {
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

/// Integer bounds; outline is drawn inside the bounds. Rotation comes later.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shape {
    pub kind: ShapeKind,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub style: ShapeStyle,
}

impl Shape {
    fn validate(self) -> Result<(), Error> {
        if self.width == 0 || self.height == 0 || self.width > 65_536 || self.height > 65_536 {
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

    fn color_at(self, x: u32, y: u32) -> Option<Color> {
        let x = f64::from(x) + 0.5;
        let y = f64::from(y) + 0.5;
        if !self.contains(x, y, 0.0) {
            return None;
        }
        match self.style {
            ShapeStyle::Fill(color) => Some(color),
            ShapeStyle::Outline { color, width } => {
                (!self.contains(x, y, f64::from(width))).then_some(color)
            }
            ShapeStyle::Both {
                fill,
                outline,
                width,
            } => Some(if self.contains(x, y, f64::from(width)) {
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct State {
    layers: Vec<Layer>,
    selected: usize,
}

pub struct Document {
    width: u32,
    height: u32,
    state: State,
    active: Option<Shape>,
    undo: VecDeque<State>,
    redo: Vec<State>,
}

impl Document {
    pub fn new(width: u32, height: u32) -> Result<Self, Error> {
        if width == 0 || height == 0 || u64::from(width) * u64::from(height) > MAX_PIXELS {
            return Err(Error::InvalidDimensions);
        }
        let pixels = vec![Color::TRANSPARENT; (width * height) as usize];
        Ok(Self {
            width,
            height,
            state: State {
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
        self.width
    }
    pub fn height(&self) -> u32 {
        self.height
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

    fn checkpoint(&mut self, before: State) {
        if before == self.state {
            return;
        }
        if self.undo.len() == HISTORY_LIMIT {
            self.undo.pop_front();
        }
        self.undo.push_back(before);
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
            pixels: Arc::new(vec![
                Color::TRANSPARENT;
                (self.width * self.height) as usize
            ]),
        });
        self.state.selected = index;
        self.checkpoint(before);
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
        self.checkpoint(before);
        Ok(())
    }

    pub fn rename_layer(&mut self, layer: usize, name: impl Into<String>) -> Result<(), Error> {
        self.editable(layer)?;
        let before = self.state.clone();
        self.state.layers[layer].name = name.into();
        self.checkpoint(before);
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
        self.checkpoint(before);
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
        self.checkpoint(before);
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
        let Some(shape) = self.active.take() else {
            return Ok(false);
        };
        let before = self.state.clone();
        let layer = &mut self.state.layers[self.state.selected];
        let mut changed = false;
        for y in 0..self.height {
            for x in 0..self.width {
                if let Some(color) = shape.color_at(x, y) {
                    let index = (y * self.width + x) as usize;
                    let result = color.over(layer.pixels[index], 255);
                    if result != layer.pixels[index] {
                        Arc::make_mut(&mut layer.pixels)[index] = result;
                        changed = true;
                    }
                }
            }
        }
        self.checkpoint(before);
        Ok(changed)
    }

    /// Preview blends the active shape into its layer before layer opacity.
    pub fn composite(&self, include_preview: bool) -> Vec<Color> {
        let mut output = vec![Color::TRANSPARENT; (self.width * self.height) as usize];
        for (layer_index, layer) in self.state.layers.iter().enumerate() {
            if !layer.visible {
                continue;
            }
            for y in 0..self.height {
                for x in 0..self.width {
                    let index = (y * self.width + x) as usize;
                    let mut color = layer.pixels[index];
                    if include_preview && layer_index == self.state.selected {
                        if let Some(preview) = self.active.and_then(|shape| shape.color_at(x, y)) {
                            color = preview.over(color, 255);
                        }
                    }
                    output[index] = color.over(output[index], layer.opacity);
                }
            }
        }
        output
    }

    pub fn undo(&mut self) -> Result<bool, Error> {
        self.commit_shape()?;
        let Some(previous) = self.undo.pop_back() else {
            return Ok(false);
        };
        self.redo.push(std::mem::replace(&mut self.state, previous));
        Ok(true)
    }

    pub fn redo(&mut self) -> Result<bool, Error> {
        if self.active.is_some() {
            return Err(Error::ActiveShape);
        }
        let Some(next) = self.redo.pop() else {
            return Ok(false);
        };
        self.undo
            .push_back(std::mem::replace(&mut self.state, next));
        Ok(true)
    }
}
