//! Brush/eraser stroke lifecycle: an isolated preview that commits as one
//! history entry, mirroring the shape draw → commit contract.

use crate::geometry::Point;
use crate::stroke::{CoverageMask, StrokeBlend, StrokeStyle};
use crate::{Color, Document, Error};
use std::sync::Arc;

/// A stroke in progress. Its coverage lives outside layer pixels until commit,
/// so the preview never changes committed state.
pub(crate) struct ActiveStroke {
    layer: usize,
    style: StrokeStyle,
    last: Point,
    mask: CoverageMask,
}

impl ActiveStroke {
    /// The layer the stroke paints on, fixed when the stroke begins.
    pub(crate) fn layer(&self) -> usize {
        self.layer
    }

    /// The previewed color of `(x, y)` on the stroke's layer.
    pub(crate) fn apply(&self, x: u32, y: u32, destination: Color) -> Color {
        self.style
            .blend
            .apply(destination, self.mask.coverage(x, y))
    }
}

impl Document {
    /// Starts a stroke on the selected layer with a dab at `at`. An active
    /// shape is committed first; if the dab cannot be stamped (for example
    /// over the memory budget) the shape stays editable and nothing changes.
    pub fn begin_stroke(&mut self, style: StrokeStyle, at: Point) -> Result<(), Error> {
        if !(at.x.is_finite() && at.y.is_finite()) {
            return Err(Error::InvalidStroke);
        }
        if self.stroke.is_some() {
            return Err(Error::ActiveStroke);
        }
        let mut mask = CoverageMask::new(self.width(), self.height());
        mask.stamp_segment(at, at, style.tip)?;
        self.commit_shape()?;
        self.stroke = Some(ActiveStroke {
            layer: self.state.selected,
            style,
            last: at,
            mask,
        });
        Ok(())
    }

    /// Extends the active stroke with a segment from its last point to `to`.
    /// Points may lie outside the canvas; coverage is clipped.
    pub fn extend_stroke(&mut self, to: Point) -> Result<(), Error> {
        let stroke = self.stroke.as_mut().ok_or(Error::NoActiveStroke)?;
        stroke
            .mask
            .stamp_segment(stroke.last, to, stroke.style.tip)?;
        stroke.last = to;
        Ok(())
    }

    /// Writes the stroke into its layer as exactly one history entry labelled
    /// "Brush" or "Eraser". Returns whether pixels changed; a no-op adds no
    /// entry. On error the layer and history are unchanged and the stroke
    /// stays active so the caller can retry or cancel.
    pub fn commit_stroke(&mut self) -> Result<bool, Error> {
        let Some(stroke) = self.stroke.take() else {
            return Ok(false);
        };
        if stroke.mask.is_empty() {
            return Ok(false);
        }
        let label = match stroke.style.blend {
            StrokeBlend::Paint(_) => "Brush",
            StrokeBlend::Erase => "Eraser",
        };
        let Some(layer) = self.state.layers.get(stroke.layer) else {
            return Err(Error::InvalidLayer);
        };
        let mut pixels = (*layer.pixels).clone();
        // One rectangle per touched tile; collected so the stroke can be put
        // back on failure while iterating.
        let rects: Vec<_> = stroke.mask.covered_rects().collect();
        let mut changed = false;
        for (left, top, right, bottom) in rects {
            for y in top..bottom {
                for x in left..right {
                    let previous = pixels.get(x, y);
                    let next = stroke.apply(x, y, previous);
                    if next != previous {
                        if let Err(error) = pixels.set(x, y, next) {
                            self.stroke = Some(stroke);
                            return Err(error);
                        }
                        changed = true;
                    }
                }
            }
        }
        if !changed {
            return Ok(false);
        }
        let before = self.state.clone();
        self.state.layers[stroke.layer].pixels = Arc::new(pixels);
        if let Err(error) = self.checkpoint(before, label) {
            self.stroke = Some(stroke);
            return Err(error);
        }
        Ok(true)
    }

    /// Discards the active stroke without touching pixels or history.
    pub fn cancel_stroke(&mut self) {
        self.stroke = None;
    }

    /// Whether a stroke preview is active.
    pub fn has_active_stroke(&self) -> bool {
        self.stroke.is_some()
    }

    /// Edits other than the stroke itself wait until it is committed or
    /// cancelled, so a preview can never be lost or applied to stale state.
    pub(crate) fn reject_active_stroke(&self) -> Result<(), Error> {
        if self.stroke.is_some() {
            Err(Error::ActiveStroke)
        } else {
            Ok(())
        }
    }
}
