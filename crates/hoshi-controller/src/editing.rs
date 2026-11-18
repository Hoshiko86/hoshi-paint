//! Shared selection editing: hit testing, gestures, pivots and color roles.
//!
//! This module edits a generic frame. Shape rasterization lives in hoshi-core;
//! native pointer/key events are translated by the UI adapter.
use super::tools::{PointerInput, shapes::ShapeGesture};
use super::{
    ActiveDrag, Controller, ControllerError, EditCursor, HANDLE_RADIUS, HandleKind, HandlePoint,
    InputModifiers, PointerButton, Tool,
};
use hoshi_core::geometry::{Frame, Point, ResizeHandle};
use hoshi_core::{Shape, ShapeKind};
#[derive(Clone, Copy, Default)]
pub(super) struct EditState {
    pub pivot: Option<Point>,
    pub secondary_colors: bool,
    pub numeric_ratio: Option<(u32, u32)>,
}
#[derive(Clone, Copy)]
pub(super) enum Gesture {
    Sample,
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
            self.shape_gesture().copied(),
            Some(
                ShapeGesture::Resize { .. }
                    | ShapeGesture::Rotate { .. }
                    | ShapeGesture::CreateShape { .. }
            )
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
            .map(|shape| self.pivot_for(shape))
    }
    fn pivot_for(&self, shape: Shape) -> Point {
        self.edit_state
            .pivot
            .unwrap_or(Frame::from_shape(shape).center)
    }
    /// Returns resize handles in `ResizeHandle::ALL` order, then rotate and pivot.
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
                    kind: HandleKind::Resize(handle),
                }
            })
            .collect();
        let (_, axis) = frame.axes();
        let top = frame.handle(ResizeHandle::Top);
        let (top_x, top_y) = self.viewport.to_logical(top.x, top.y);
        points.push(HandlePoint {
            x: top_x - axis.x * 28.,
            y: top_y - axis.y * 28.,
            kind: HandleKind::Rotate,
        });
        let pivot = self.pivot_for(shape);
        let (x, y) = self.viewport.to_logical(pivot.x, pivot.y);
        points.push(HandlePoint {
            x,
            y,
            kind: HandleKind::Pivot,
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
        if !matches!(self.tool, Tool::Rectangle | Tool::MoveShape) {
            return EditCursor::Draw;
        }
        if matches!(
            self.shape_gesture().copied(),
            Some(ShapeGesture::Rotate { .. })
        ) {
            return EditCursor::Rotate;
        }
        if matches!(
            self.shape_gesture().copied(),
            Some(ShapeGesture::Pivot { .. })
        ) {
            return EditCursor::Pivot;
        }
        let Some(shape) = self.document.active_shape() else {
            return EditCursor::Draw;
        };
        if let Some(hit) = self.hit_handle(x, y) {
            return match hit.kind {
                HandleKind::Pivot => EditCursor::Pivot,
                HandleKind::Rotate => EditCursor::Rotate,
                HandleKind::Resize(handle) => {
                    let (sx, sy) = handle.signs();
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
        let pivot = self.pivot_for(shape);
        self.begin_shape(ShapeGesture::Rotate {
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
                    HandleKind::Rotate => self.begin_rotation(shape, point),
                    HandleKind::Pivot => self.begin_shape(ShapeGesture::Pivot {
                        original: self.pivot_for(shape),
                        anchor: point,
                    }),
                    HandleKind::Resize(handle) => {
                        self.edit_state.numeric_ratio = None;
                        self.begin_shape(ShapeGesture::Resize {
                            original: shape,
                            handle,
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
                self.begin_shape(ShapeGesture::Move {
                    original: shape,
                    anchor: point,
                    pivot: self.edit_state.pivot,
                });
                return Ok(());
            }
        }
        match self.tool {
            Tool::Brush | Tool::Eraser => self.begin_paint(PointerInput { x, y }, button)?,
            Tool::Rectangle => {
                self.commit()?;
                if point.x < 0.
                    || point.y < 0.
                    || point.x >= f64::from(self.document.width())
                    || point.y >= f64::from(self.document.height())
                {
                    self.clear_gesture();
                    return Ok(());
                }
                self.edit_state.secondary_colors = button == PointerButton::Secondary;
                self.begin_shape(ShapeGesture::CreateShape {
                    anchor: self.clip_point((point.x, point.y)),
                });
            }
            Tool::MoveShape => return Err(ControllerError::NoActiveShape),
            Tool::Pan => self.begin_pan(PointerInput { x, y }, false)?,
            Tool::Eyedropper => {
                self.pick = Some(crate::tools::color_picker::ColorPick::new(
                    &self.document,
                    self.settings.picker,
                    self.modifiers.control,
                    button,
                ));
                self.sample_color(p)?;
                self.begin_gesture(Gesture::Sample)
            }
        }
        Ok(())
    }
    pub fn pointer_move(&mut self, x: f64, y: f64) -> Result<(), ControllerError> {
        let p = self.document_point(x, y)?;
        self.last_pointer = Some((x, y));
        if let Some(mut drag) = self.drag.take() {
            let result = match &mut drag {
                ActiveDrag::Tool { state, .. } => {
                    state.pointer_move(&mut self.tool_cx(), PointerInput { x, y })
                }
                ActiveDrag::Gesture(Gesture::Sample) => self.sample_color(p),
            };
            self.drag = Some(drag);
            result?;
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
            let pivot = self.pivot_for(shape);
            let angle = f64::from(shape.rotation_degrees) + (dx + dy) as f64 * f64::from(step);
            let updated = Frame::from_shape(shape)
                .rotated_about(pivot, angle)
                .apply_to(shape);
            self.document.adjust_shape(updated)?;
            if let Some((x, y)) = self.last_pointer {
                let p = self.document_point(x, y)?;
                self.begin_shape(ShapeGesture::Rotate {
                    original: updated,
                    pivot,
                    anchor_angle: (p.1 - pivot.y).atan2(p.0 - pivot.x),
                });
            }
        } else {
            let amount = if control { 10 } else { 1 };
            self.nudge_shape(dx.saturating_mul(amount), dy.saturating_mul(amount))?;
            let Some(updated) = self.document.active_shape() else {
                return Ok(());
            };
            if let Some((x, y)) = self.last_pointer
                && let Some(ShapeGesture::Resize {
                    original, handle, ..
                }) = self.shape_gesture().copied()
            {
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
                self.begin_shape(ShapeGesture::Resize {
                    original: updated,
                    handle: ResizeHandle::ALL[index],
                    pointer_anchor: Point::new(p.0, p.1),
                });
            }
            if let Some((x, y)) = self.last_pointer
                && let Some(ShapeGesture::Move { .. }) = self.shape_gesture().copied()
            {
                let p = self.document_point(x, y)?;
                self.begin_shape(ShapeGesture::Move {
                    anchor: Point::new(p.0, p.1),
                    original: updated,
                    pivot: self.edit_state.pivot,
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handle_positions_have_each_kind_once_in_documented_order() {
        let mut controller = Controller::new(256, 256).unwrap();
        controller.pointer_down(40., 40.).unwrap();
        controller.pointer_move(140., 100.).unwrap();
        controller.pointer_up().unwrap();
        let expected = ResizeHandle::ALL
            .into_iter()
            .map(HandleKind::Resize)
            .chain([HandleKind::Rotate, HandleKind::Pivot])
            .collect::<Vec<_>>();
        assert_eq!(
            controller
                .handle_positions()
                .into_iter()
                .map(|p| p.kind)
                .collect::<Vec<_>>(),
            expected
        );
    }
}
