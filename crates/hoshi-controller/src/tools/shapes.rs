//! Shape preview gestures with narrow document, modifier and pivot capabilities.
use super::{ControllerError, PointerInput, ToolBehavior, ToolCx};
use hoshi_core::{
    Shape,
    geometry::{Frame, Point, ResizeConstraints, ResizeHandle},
};

/// One shape drag, retaining its original geometry for deterministic rebasing.
#[derive(Clone, Copy)]
pub(crate) enum ShapeGesture {
    CreateShape {
        anchor: (f64, f64),
    },
    Move {
        anchor: Point,
        original: Shape,
        pivot: Option<Point>,
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

/// Tool-local shape drag; release leaves the document preview editable.
pub(crate) struct ShapeTool {
    /// Original frame and anchor for the active drag.
    pub(crate) gesture: ShapeGesture,
}

impl ShapeTool {
    fn shape_at(template: Shape, x: i32, y: i32, width: u32, height: u32) -> Shape {
        Shape {
            x,
            y,
            width,
            height,
            ..template
        }
    }
    fn shape_from_drag(cx: &ToolCx<'_>, anchor: (f64, f64), point: (f64, f64)) -> Shape {
        if cx.modifiers.alt {
            // Alt fixes the creation center. Bounds may extend beyond the canvas;
            // the rasterizer clips them, just as it does when moving a shape.
            let mut half_width = (point.0 - anchor.0).abs();
            let mut half_height = (point.1 - anchor.1).abs();
            if cx.modifiers.shift {
                half_width = half_width.max(half_height);
                half_height = half_width;
            }
            let width = (2. * half_width)
                .round()
                .clamp(1., f64::from(hoshi_core::MAX_DIMENSION));
            let height = (2. * half_height)
                .round()
                .clamp(1., f64::from(hoshi_core::MAX_DIMENSION));
            return Self::shape_at(
                cx.template,
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
            .min(f64::from(cx.document.width() - 1));
        let top = anchor
            .1
            .min(point.1)
            .floor()
            .min(f64::from(cx.document.height() - 1));
        let right = anchor.0.max(point.0).ceil();
        let bottom = anchor.1.max(point.1).ceil();
        let (left, top, width, height) = if cx.modifiers.shift {
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
                f64::from(cx.document.width()) - ax
            };
            let available_y = if point.1 < anchor.1 {
                ay
            } else {
                f64::from(cx.document.height()) - ay
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
        Self::shape_at(
            cx.template,
            left as i32,
            top as i32,
            (width as u32).max(1),
            (height as u32).max(1),
        )
    }
}
impl ToolBehavior for ShapeTool {
    fn pointer_down(
        &mut self,
        _cx: &mut ToolCx<'_>,
        _p: PointerInput,
    ) -> Result<(), ControllerError> {
        Ok(())
    }
    fn pointer_move(
        &mut self,
        cx: &mut ToolCx<'_>,
        input: PointerInput,
    ) -> Result<(), ControllerError> {
        if !input.x.is_finite() || !input.y.is_finite() {
            return Err(ControllerError::InvalidInput);
        }
        let p = cx.viewport.to_document(input.x, input.y);
        if !p.0.is_finite() || !p.1.is_finite() {
            return Err(ControllerError::InvalidInput);
        }
        let point = Point::new(p.0, p.1);
        match self.gesture {
            ShapeGesture::CreateShape { anchor } => {
                let p = (
                    p.0.clamp(0., f64::from(cx.document.width())),
                    p.1.clamp(0., f64::from(cx.document.height())),
                );
                if (p.0 == anchor.0 && p.1 == anchor.1)
                    || (!cx.modifiers.shift && (p.0 == anchor.0 || p.1 == anchor.1))
                {
                    cx.document.cancel_shape();
                } else {
                    let shape = Self::shape_from_drag(cx, anchor, p);
                    if cx.document.active_shape().is_some() {
                        cx.document.adjust_shape(shape)?
                    } else {
                        cx.document.begin_shape(shape)?
                    }
                }
            }
            ShapeGesture::Move {
                anchor,
                mut original,
                pivot,
            } => {
                let dx = (point.x - anchor.x).round() as i32;
                let dy = (point.y - anchor.y).round() as i32;
                original.x = original.x.saturating_add(dx);
                original.y = original.y.saturating_add(dy);
                cx.document.adjust_shape(original)?;
                *cx.pivot = pivot.map(|p| p.translated(f64::from(dx), f64::from(dy)));
            }
            ShapeGesture::Resize {
                original,
                handle,
                pointer_anchor,
            } => {
                let frame = Frame::from_shape(original).resized(
                    handle,
                    pointer_anchor,
                    point,
                    ResizeConstraints {
                        preserve_aspect: cx.modifiers.shift,
                        from_center: cx.modifiers.alt,
                    },
                );
                cx.document.adjust_shape(frame.apply_to(original))?;
            }
            ShapeGesture::Rotate {
                original,
                pivot,
                anchor_angle,
            } => {
                if point.distance(pivot) > f64::EPSILON {
                    let delta =
                        ((point.y - pivot.y).atan2(point.x - pivot.x) - anchor_angle).to_degrees();
                    let mut angle = f64::from(original.rotation_degrees) + delta;
                    if cx.modifiers.shift {
                        angle = (angle / 15.).round() * 15.;
                    }
                    cx.document.adjust_shape(
                        Frame::from_shape(original)
                            .rotated_about(pivot, angle)
                            .apply_to(original),
                    )?;
                }
            }
            ShapeGesture::Pivot { original, anchor } => {
                *cx.pivot = Some(original.translated(point.x - anchor.x, point.y - anchor.y))
            }
        }
        Ok(())
    }
    fn pointer_up(&mut self, _cx: &mut ToolCx<'_>) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Controller, InputModifiers, Viewport};
    use hoshi_core::Document;

    fn template() -> Shape {
        Controller::new(32, 32).unwrap().create_shape(0, 0, 1, 1)
    }

    #[test]
    fn create_drag_respects_shift_and_alt_and_leaves_an_editable_preview() {
        for (shift, alt, expected) in [
            (false, false, (10, 10, 8, 4)),
            (true, false, (10, 10, 8, 8)),
            (false, true, (2, 6, 16, 8)),
            (true, true, (2, 2, 16, 16)),
        ] {
            let mut document = Document::new(32, 32).unwrap();
            let mut viewport = Viewport::default();
            let mut pivot = None;
            let mut cx = ToolCx {
                document: &mut document,
                viewport: &mut viewport,
                pivot: &mut pivot,
                template: template(),
                modifiers: InputModifiers {
                    shift,
                    alt,
                    ..InputModifiers::default()
                },
            };
            let mut tool = ShapeTool {
                gesture: ShapeGesture::CreateShape { anchor: (10., 10.) },
            };
            tool.pointer_down(&mut cx, PointerInput { x: 10., y: 10. })
                .unwrap();
            tool.pointer_move(&mut cx, PointerInput { x: 18., y: 14. })
                .unwrap();
            tool.pointer_up(&mut cx);
            let shape = cx.document.active_shape().unwrap();
            assert_eq!((shape.x, shape.y, shape.width, shape.height), expected);
            assert_eq!(cx.document.undo_len(), 0);
            assert_eq!(
                cx.document.composite(false),
                Document::new(32, 32).unwrap().composite(false)
            );
        }
    }

    #[test]
    fn move_translates_the_pivot_and_pivot_drag_writes_through() {
        let mut document = Document::new(32, 32).unwrap();
        let original = Shape {
            x: 4,
            y: 4,
            width: 10,
            height: 10,
            ..template()
        };
        document.begin_shape(original).unwrap();
        let mut viewport = Viewport::default();
        let mut pivot = Some(Point::new(7., 8.));
        let mut cx = ToolCx {
            document: &mut document,
            viewport: &mut viewport,
            pivot: &mut pivot,
            template: template(),
            modifiers: InputModifiers::default(),
        };
        let mut tool = ShapeTool {
            gesture: ShapeGesture::Move {
                anchor: Point::new(2., 2.),
                original,
                pivot: *cx.pivot,
            },
        };
        tool.pointer_move(&mut cx, PointerInput { x: 5., y: 6. })
            .unwrap();
        assert_eq!(*cx.pivot, Some(Point::new(10., 12.)));
        assert_eq!(cx.document.active_shape().unwrap().x, 7);
        tool.gesture = ShapeGesture::Pivot {
            original: Point::new(10., 12.),
            anchor: Point::new(5., 6.),
        };
        tool.pointer_move(&mut cx, PointerInput { x: 7., y: 3. })
            .unwrap();
        assert_eq!(*cx.pivot, Some(Point::new(12., 9.)));
    }

    #[test]
    fn rotation_snaps_to_fifteen_degrees_with_shift() {
        let mut document = Document::new(32, 32).unwrap();
        let original = Shape {
            x: 4,
            y: 4,
            width: 10,
            height: 10,
            ..template()
        };
        document.begin_shape(original).unwrap();
        let mut viewport = Viewport::default();
        let mut pivot = None;
        let mut cx = ToolCx {
            document: &mut document,
            viewport: &mut viewport,
            pivot: &mut pivot,
            template: template(),
            modifiers: InputModifiers {
                shift: true,
                ..InputModifiers::default()
            },
        };
        let mut tool = ShapeTool {
            gesture: ShapeGesture::Rotate {
                original,
                pivot: Point::new(9., 9.),
                anchor_angle: 0.,
            },
        };
        tool.pointer_move(&mut cx, PointerInput { x: 19., y: 12. })
            .unwrap();
        assert_eq!(cx.document.active_shape().unwrap().rotation_degrees, 15);
    }

    #[test]
    fn nonfinite_pointer_preserves_preview_and_pivot() {
        let mut document = Document::new(32, 32).unwrap();
        document.begin_shape(template()).unwrap();
        let before = document.composite(true);
        let mut viewport = Viewport::default();
        let mut pivot = Some(Point::new(3., 4.));
        let mut cx = ToolCx {
            document: &mut document,
            viewport: &mut viewport,
            pivot: &mut pivot,
            template: template(),
            modifiers: InputModifiers::default(),
        };
        let mut tool = ShapeTool {
            gesture: ShapeGesture::CreateShape { anchor: (1., 1.) },
        };
        for x in [f64::NAN, f64::INFINITY] {
            assert_eq!(
                tool.pointer_move(&mut cx, PointerInput { x, y: 5. }),
                Err(ControllerError::InvalidInput)
            );
            assert_eq!(cx.document.composite(true), before);
            assert_eq!(*cx.pivot, Some(Point::new(3., 4.)));
        }
    }

    #[test]
    fn cancel_pointer_discards_a_shape_create_preview() {
        let mut controller = Controller::new(32, 32).unwrap();
        let before = controller.document().composite(false);
        controller.pointer_down(2., 2.).unwrap();
        controller.pointer_move(20., 20.).unwrap();
        assert!(controller.document().active_shape().is_some());
        controller.cancel_pointer().unwrap();
        assert!(!controller.is_dragging());
        assert!(controller.document().active_shape().is_none());
        assert_eq!(controller.document().composite(true), before);
        assert_eq!(controller.document().undo_len(), 0);
    }
}
