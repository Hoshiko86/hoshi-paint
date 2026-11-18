//! Frozen Color Picker source, target slot and alpha-aware sample windows.
use crate::{ColorSlot, PointerButton};
use hoshi_core::{Color, Document, DocumentSnapshot, Shape};

/// Source used for a canvas color sample; Ctrl temporarily selects Image.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PickerSampling {
    /// Committed pixels of the layer selected at pointer-down.
    #[default]
    Layer,
    /// Visible composite and editable preview frozen at pointer-down.
    Image,
}

/// Odd square sample windows, clipped to the canvas.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SampleSize {
    /// Exactly the pixel under the pointer, including its straight RGBA.
    #[default]
    Single,
    /// A three-pixel square.
    Square3,
    /// A five-pixel square.
    Square5,
    /// An eleven-pixel square.
    Square11,
    /// A thirty-one-pixel square.
    Square31,
    /// A fifty-one-pixel square.
    Square51,
}
impl SampleSize {
    /// All supported sizes in increasing order.
    pub const ALL: [Self; 6] = [
        Self::Single,
        Self::Square3,
        Self::Square5,
        Self::Square11,
        Self::Square31,
        Self::Square51,
    ];
    /// Return the odd window width in document pixels.
    pub const fn side(self) -> u32 {
        match self {
            Self::Single => 1,
            Self::Square3 => 3,
            Self::Square5 => 5,
            Self::Square11 => 11,
            Self::Square31 => 31,
            Self::Square51 => 51,
        }
    }
}

/// Tool selection after releasing a drag that sampled at least one pixel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AfterClick {
    /// Keep the Color Picker selected.
    #[default]
    DoNotSwitch,
    /// Restore the last selected tool other than the Color Picker.
    PreviousTool,
}

/// Independent picker options; changing them does not add document history.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PickerSettings {
    /// Layer or Image source; frozen when a drag starts.
    pub sampling: PickerSampling,
    /// Square window size; frozen when a drag starts.
    pub size: SampleSize,
    /// Action after a successful sample is released.
    pub after_click: AfterClick,
}

/// Source frozen at pointer-down for the whole drag (#48 contract).
pub(crate) enum PickSource {
    Image {
        snapshot: DocumentSnapshot,
        preview: Option<Shape>,
    },
    Layer {
        index: usize,
    },
}
impl PickSource {
    fn pixel(&self, document: &Document, x: u32, y: u32) -> Option<Color> {
        match self {
            Self::Image { snapshot, preview } => Some(snapshot.composite_pixel(x, y, *preview)),
            Self::Layer { index } => Some(document.layers().get(*index)?.pixels().get(x, y)),
        }
    }
}

/// One pick drag with its fixed source, size and mouse-button color target.
pub(crate) struct ColorPick {
    source: PickSource,
    size: SampleSize,
    target: ColorSlot,
    sampled: bool,
}
impl ColorPick {
    /// Capture source and button target without changing document or slot selection.
    pub(crate) fn new(
        document: &Document,
        settings: PickerSettings,
        control: bool,
        button: PointerButton,
    ) -> Self {
        let source = if settings.sampling == PickerSampling::Image || control {
            PickSource::Image {
                snapshot: document.snapshot(),
                preview: document.active_shape(),
            }
        } else {
            PickSource::Layer {
                index: document.selected_layer(),
            }
        };
        Self {
            source,
            size: settings.size,
            target: match button {
                PointerButton::Primary => ColorSlot::Primary,
                PointerButton::Secondary => ColorSlot::Secondary,
            },
            sampled: false,
        }
    }

    /// None outside the canvas or when the frozen layer no longer exists.
    pub(crate) fn sample(&mut self, document: &Document, at: (f64, f64)) -> Option<Color> {
        if !(at.0 >= 0.
            && at.1 >= 0.
            && at.0 < f64::from(document.width())
            && at.1 < f64::from(document.height()))
        {
            return None;
        }
        let (x, y) = (at.0.floor() as u32, at.1.floor() as u32);
        let color = if self.size == SampleSize::Single {
            self.source.pixel(document, x, y)?
        } else {
            self.average(document, x, y)?
        };
        self.sampled = true;
        Some(color)
    }

    fn average(&self, document: &Document, x: u32, y: u32) -> Option<Color> {
        let radius = self.size.side() / 2;
        let mut channels = [0_u64; 3];
        let (mut alpha, mut count) = (0_u64, 0_u64);
        for y in y.saturating_sub(radius)..=(y + radius).min(document.height() - 1) {
            for x in x.saturating_sub(radius)..=(x + radius).min(document.width() - 1) {
                let pixel = self.source.pixel(document, x, y)?;
                let a = u64::from(pixel.a);
                for (sum, channel) in channels.iter_mut().zip([pixel.r, pixel.g, pixel.b]) {
                    *sum += u64::from(channel) * a;
                }
                alpha += a;
                count += 1;
            }
        }
        if alpha == 0 {
            return Some(Color::TRANSPARENT);
        }
        let [r, g, b] = channels.map(|sum| ((sum + alpha / 2) / alpha) as u8);
        Some(Color::rgba(r, g, b, ((alpha + count / 2) / count) as u8))
    }

    /// Fixed color slot selected by the drag's initiating mouse button.
    pub(crate) fn target(&self) -> ColorSlot {
        self.target
    }
    /// Whether this drag has sampled an in-canvas pixel.
    pub(crate) fn sampled(&self) -> bool {
        self.sampled
    }
}
