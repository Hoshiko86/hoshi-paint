//! Shared selection editing: hit testing, gestures, pivots and color roles.
//!
//! This module edits a generic frame. Shape rasterization lives in hoshi-core;
//! native pointer/key events are translated by the UI adapter.
use super::{
    Controller, ControllerError, EditCursor, HANDLE_RADIUS, HandlePoint, InputModifiers,
    PIVOT_HANDLE, PointerButton, ROTATION_HANDLE, Tool,
};
use hoshi_core::geometry::{Frame, Point, ResizeConstraints, ResizeHandle};
use hoshi_core::{Shape, ShapeKind};
#[derive(Clone, Copy, Default)]
pub(super) struct EditState {
    pub pivot: Option<Point>,
    pub secondary_colors: bool,
}
#[derive(Clone, Copy)]
pub(super) enum Gesture {
    Sample,
    CreateShape {
        anchor: (f64, f64),
    },
    Move {
        anchor: Point,
        original: Shape,
        pivot: Option<Point>,
    },
    Pan {
        last: (f64, f64),
    },
    TemporaryPan {
        last: (f64, f64),
    },
    Resize {
        original: Shape,
        handle: ResizeHandle,
        pointer_anchor: Point,
    },
    Rotate {
        original: Shape,
        pivot: Point,
        anchor_angle: f64,
    },
    Pivot {
        original: Point,
        anchor: Point,
    },
}
impl Controller {
    pub fn aspect_lock(&self) -> bool {
        self.modifiers.shift
    }
    pub fn set_aspect_lock(&mut self, shift: bool) -> Result<(), ControllerError> {
        self.set_input_modifiers(InputModifiers {
            shift,
            ..self.modifiers
        })
    }
    pub fn set_input_modifiers(
        &mut self,
        modifiers: InputModifiers,
    ) -> Result<(), ControllerError> {
        if modifiers == self.modifiers {
            return Ok(());
        }
        self.modifiers = modifiers;
        if matches!(
            self.gesture,
            Some(Gesture::Resize { .. } | Gesture::Rotate { .. } | Gesture::CreateShape { .. })
        ) && let Some((x, y)) = self.last_pointer
        {
            self.pointer_move(x, y)?;
        }
        Ok(())
    }
    pub fn set_shape_kind(&mut self, kind: ShapeKind) -> Result<(), ControllerError> {
        if let Some(mut shape) = self.document.active_shape() {
            shape.kind = kind;
            self.document.adjust_shape(shape)?;
        }
        self.shape_kind = kind;
        Ok(())
    }
    pub fn rotation_pivot(&self) -> Option<Point> {
        self.document
            .active_shape()
            .map(|s| self.edit_state.pivot.unwrap_or(Frame::from_shape(s).center))
    }
    pub fn handle_positions(&self) -> Vec<HandlePoint> {
        let Some(shape) = self.document.active_shape() else {
            return Vec::new();
        };
        let frame = Frame::from_shape(shape);
        let mut points: Vec<_> = ResizeHandle::ALL
            .into_iter()
            .map(|handle| {
                let p = frame.handle(handle);
                let (x, y) = self.viewport.to_logical(p.x, p.y);
                HandlePoint {
                    x,
                    y,
                    kind: handle.index() as i32,
                }
            })
            .collect();
        let (_, axis) = frame.axes();
        let top = points[1];
        points.push(HandlePoint {
            x: top.x - axis.x * 28.,
            y: top.y - axis.y * 28.,
            kind: ROTATION_HANDLE,
        });
        let pivot = self.rotation_pivot().expect("active shape");
        let (x, y) = self.viewport.to_logical(pivot.x, pivot.y);
        points.push(HandlePoint {
            x,
            y,
            kind: PIVOT_HANDLE,
        });
        points
    }
    fn hit_handle(&self, x: f64, y: f64) -> Option<HandlePoint> {
        self.handle_positions()
            .into_iter()
            .filter(|h| (h.x - x).hypot(h.y - y) <= HANDLE_RADIUS)
            .min_by(|a, b| {
                (a.x - x)
                    .hypot(a.y - y)
                    .total_cmp(&(b.x - x).hypot(b.y - y))
            })
    }
    fn near_rotation_edge(&self, frame: Frame, point: Point) -> bool {
        let padded = Frame {
            width: frame.width + 24. / self.viewport.zoom,
            height: frame.height + 24. / self.viewport.zoom,
            ..frame
        };
        !frame.contains(point) && padded.contains(point)
    }
    pub fn hover_cursor(&self, x: f64, y: f64) -> EditCursor {
        if matches!(self.gesture, Some(Gesture::Rotate { .. })) {
            return EditCursor::Rotate;
        }
        if matches!(self.gesture, Some(Gesture::Pivot { .. })) {
            return EditCursor::Pivot;
        }
        let Some(shape) = self.document.active_shape() else {
            return EditCursor::Draw;
        };
        if let Some(hit) = self.hit_handle(x, y) {
            return match hit.kind {
                PIVOT_HANDLE => EditCursor::Pivot,
                ROTATION_HANDLE => EditCursor::Rotate,
                index => {
                    let (sx, sy) = ResizeHandle::ALL[index as usize].signs();
                    let angle = (f64::from(shape.rotation_degrees) + sy.atan2(sx).to_degrees())
                        .rem_euclid(180.);
                    match (angle / 45.).round() as i32 % 4 {
                        0 => EditCursor::ResizeHorizontal,
                        1 => EditCursor::ResizeDiagonalBackward,
                        2 => EditCursor::ResizeVertical,
                        _ => EditCursor::ResizeDiagonalForward,
                    }
                }
            };
        }
        let Ok(p) = self.document_point(x, y) else {
            return EditCursor::Draw;
        };
        let p = Point::new(p.0, p.1);
        let frame = Frame::from_shape(shape);
        if frame.contains(p) {
            EditCursor::Move
        } else if self.near_rotation_edge(frame, p) {
            EditCursor::Rotate
        } else {
            EditCursor::Draw
        }
    }
    fn begin_rotation(&mut self, shape: Shape, point: Point) {
        let pivot = self
            .rotation_pivot()
            .unwrap_or(Frame::from_shape(shape).center);
        self.gesture = Some(Gesture::Rotate {
            original: shape,
            pivot,
            anchor_angle: (point.y - pivot.y).atan2(point.x - pivot.x),
        });
    }
    pub fn pointer_down(&mut self, x: f64, y: f64) -> Result<(), ControllerError> {
        self.pointer_down_with_button(x, y, PointerButton::Primary)
    }
    pub fn pointer_down_with_button(
        &mut self,
        x: f64,
        y: f64,
        button: PointerButton,
    ) -> Result<(), ControllerError> {
        let p = self.document_point(x, y)?;
        let point = Point::new(p.0, p.1);
        self.last_pointer = Some((x, y));
        if matches!(self.tool, Tool::Rectangle | Tool::MoveShape)
            && let Some(shape) = self.document.active_shape()
        {
            if let Some(hit) = self.hit_handle(x, y) {
                match hit.kind {
                    ROTATION_HANDLE => self.begin_rotation(shape, point),
                    PIVOT_HANDLE => {
                        self.gesture = Some(Gesture::Pivot {
                            original: self.rotation_pivot().expect("active shape"),
                            anchor: point,
                        })
                    }
                    index => {
                        self.gesture = Some(Gesture::Resize {
                            original: shape,
                            handle: ResizeHandle::ALL[index as usize],
                            pointer_anchor: point,
                        })
                    }
                }
                return Ok(());
            }
            let frame = Frame::from_shape(shape);
            if button == PointerButton::Secondary || self.near_rotation_edge(frame, point) {
                self.begin_rotation(shape, point);
                return Ok(());
            }
            if frame.contains(point) || self.tool == Tool::MoveShape {
                self.gesture = Some(Gesture::Move {
                    original: shape,
                    anchor: point,
                    pivot: self.edit_state.pivot,
                });
                return Ok(());
            }
        }
        match self.tool {
            Tool::Rectangle => {
                self.commit()?;
                if point.x < 0.
                    || point.y < 0.
                    || point.x >= f64::from(self.document.width())
                    || point.y >= f64::from(self.document.height())
                {
                    self.gesture = None;
                    return Ok(());
                }
                self.edit_state.secondary_colors = button == PointerButton::Secondary;
                self.gesture = Some(Gesture::CreateShape {
                    anchor: self.clip_point((point.x, point.y)),
                });
            }
            Tool::MoveShape => return Err(ControllerError::NoActiveShape),
            Tool::Pan => self.gesture = Some(Gesture::Pan { last: (x, y) }),
            Tool::Eyedropper => {
                self.sample_snapshot =
                    Some((self.document.snapshot(), self.document.active_shape()));
                self.sample_color(p)?;
                self.gesture = Some(Gesture::Sample)
            }
        }
        Ok(())
    }
    fn shape_from_drag(&self, anchor: (f64, f64), point: (f64, f64)) -> Shape {
        if self.modifiers.alt {
            // Alt fixes the creation center. Bounds may extend beyond the canvas;
            // the rasterizer clips them, just as it does when moving a shape.
            let mut half_width = (point.0 - anchor.0).abs();
            let mut half_height = (point.1 - anchor.1).abs();
            if self.modifiers.shift {
                half_width = half_width.max(half_height);
                half_height = half_width;
            }
            let width = (2. * half_width)
                .round()
                .clamp(1., f64::from(hoshi_core::MAX_DIMENSION));
            let height = (2. * half_height)
                .round()
                .clamp(1., f64::from(hoshi_core::MAX_DIMENSION));
            return self.create_shape(
                (anchor.0 - width / 2.).round() as i32,
                (anchor.1 - height / 2.).round() as i32,
                width as u32,
                height as u32,
            );
        }
        // Bounds stay inside this small document. The gesture handler only calls
        // this for a drag with nonzero extent; a click alone creates no shape.
        let left = anchor
            .0
            .min(point.0)
            .floor()
            .min(f64::from(self.document.width() - 1));
        let top = anchor
            .1
            .min(point.1)
            .floor()
            .min(f64::from(self.document.height() - 1));
        let right = anchor.0.max(point.0).ceil();
        let bottom = anchor.1.max(point.1).ceil();
        let (left, top, width, height) = if self.modifiers.shift {
            // Snap the fixed corner outward once, then use equal integer extents.
            // Limit both axes together so clipping cannot turn a square into a rectangle.
            let ax = if point.0 < anchor.0 {
                anchor.0.ceil()
            } else {
                anchor.0.floor()
            };
            let ay = if point.1 < anchor.1 {
                anchor.1.ceil()
            } else {
                anchor.1.floor()
            };
            let available_x = if point.0 < anchor.0 {
                ax
            } else {
                f64::from(self.document.width()) - ax
            };
            let available_y = if point.1 < anchor.1 {
                ay
            } else {
                f64::from(self.document.height()) - ay
            };
            let size = (right - left)
                .max(bottom - top)
                .min(available_x)
                .min(available_y)
                .max(1.0);
            (
                if point.0 < anchor.0 { ax - size } else { ax },
                if point.1 < anchor.1 { ay - size } else { ay },
                size,
                size,
            )
        } else {
            (left, top, right - left, bottom - top)
        };
        self.create_shape(
            left as i32,
            top as i32,
            (width as u32).max(1),
            (height as u32).max(1),
        )
    }

    pub fn pointer_move(&mut self, x: f64, y: f64) -> Result<(), ControllerError> {
        let p = self.document_point(x, y)?;
        let point = Point::new(p.0, p.1);
        self.last_pointer = Some((x, y));
        match self.gesture {
            Some(Gesture::Sample) => self.sample_color(p)?,
            Some(Gesture::CreateShape { anchor }) => {
                let p = self.clip_point(p);
                if (p.0 == anchor.0 && p.1 == anchor.1)
                    || (!self.modifiers.shift && (p.0 == anchor.0 || p.1 == anchor.1))
                {
                    self.document.cancel_shape();
                } else {
                    let shape = self.shape_from_drag(anchor, p);
                    if self.document.active_shape().is_some() {
                        self.document.adjust_shape(shape)?
                    } else {
                        self.document.begin_shape(shape)?
                    }
                }
            }
            Some(Gesture::Move {
                anchor,
                mut original,
                pivot,
            }) => {
                let dx = (point.x - anchor.x).round() as i32;
                let dy = (point.y - anchor.y).round() as i32;
                original.x = original.x.saturating_add(dx);
                original.y = original.y.saturating_add(dy);
                self.document.adjust_shape(original)?;
                self.edit_state.pivot = pivot.map(|p| p.translated(f64::from(dx), f64::from(dy)));
            }
            Some(Gesture::Resize {
                original,
                handle,
                pointer_anchor,
            }) => {
                let frame = Frame::from_shape(original).resized(
                    handle,
                    pointer_anchor,
                    point,
                    ResizeConstraints {
                        preserve_aspect: self.modifiers.shift,
                        from_center: self.modifiers.alt,
                    },
                );
                self.document.adjust_shape(frame.apply_to(original))?;
            }
            Some(Gesture::Rotate {
                original,
                pivot,
                anchor_angle,
            }) => {
                if point.distance(pivot) > f64::EPSILON {
                    let delta =
                        ((point.y - pivot.y).atan2(point.x - pivot.x) - anchor_angle).to_degrees();
                    let mut angle = f64::from(original.rotation_degrees) + delta;
                    if self.modifiers.shift {
                        angle = (angle / 15.).round() * 15.;
                    }
                    self.document.adjust_shape(
                        Frame::from_shape(original)
                            .rotated_about(pivot, angle)
                            .apply_to(original),
                    )?;
                }
            }
            Some(Gesture::Pivot { original, anchor }) => {
                self.edit_state.pivot =
                    Some(original.translated(point.x - anchor.x, point.y - anchor.y))
            }
            Some(Gesture::Pan { last } | Gesture::TemporaryPan { last }) => {
                let temporary = matches!(self.gesture, Some(Gesture::TemporaryPan { .. }));
                self.pan(x - last.0, y - last.1)?;
                self.gesture = Some(if temporary {
                    Gesture::TemporaryPan { last: (x, y) }
                } else {
                    Gesture::Pan { last: (x, y) }
                });
            }
            None => (),
        }
        Ok(())
    }
    pub fn keyboard_transform(
        &mut self,
        dx: i32,
        dy: i32,
        secondary_held: bool,
        control: bool,
    ) -> Result<(), ControllerError> {
        let Some(shape) = self.document.active_shape() else {
            return Ok(());
        };
        if secondary_held {
            let step = if self.modifiers.shift { 15 } else { 1 };
            let pivot = self.rotation_pivot().expect("active shape");
            let angle = f64::from(shape.rotation_degrees) + (dx + dy) as f64 * f64::from(step);
            let updated = Frame::from_shape(shape)
                .rotated_about(pivot, angle)
                .apply_to(shape);
            self.document.adjust_shape(updated)?;
            if let Some((x, y)) = self.last_pointer {
                let p = self.document_point(x, y)?;
                self.gesture = Some(Gesture::Rotate {
                    original: updated,
                    pivot,
                    anchor_angle: (p.1 - pivot.y).atan2(p.0 - pivot.x),
                });
            }
        } else {
            let amount = if control { 10 } else { 1 };
            self.nudge_shape(dx.saturating_mul(amount), dy.saturating_mul(amount))?;
            if let Some((x, y)) = self.last_pointer
                && let Some(Gesture::Resize {
                    original, handle, ..
                }) = self.gesture
            {
                let updated = self.document.active_shape().expect("active shape");
                // The dragged physical corner changes sides after crossing its
                // opposite. Rebase keyboard movement without losing that corner.
                let mut index = handle.index();
                if original.flip_horizontal != updated.flip_horizontal {
                    index = [2, 1, 0, 7, 6, 5, 4, 3][index];
                }
                if original.flip_vertical != updated.flip_vertical {
                    index = [6, 5, 4, 3, 2, 1, 0, 7][index];
                }
                let p = self.document_point(x, y)?;
                self.gesture = Some(Gesture::Resize {
                    original: updated,
                    handle: ResizeHandle::ALL[index],
                    pointer_anchor: Point::new(p.0, p.1),
                });
            }
            if let Some((x, y)) = self.last_pointer
                && let Some(Gesture::Move { .. }) = self.gesture
            {
                let p = self.document_point(x, y)?;
                self.gesture = Some(Gesture::Move {
                    anchor: Point::new(p.0, p.1),
                    original: self.document.active_shape().expect("active shape"),
                    pivot: self.edit_state.pivot,
                });
            }
        }
        Ok(())
    }
}
