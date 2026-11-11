//! Real winit/software backend launch; run under Xvfb in headless Linux CI.
use slint::platform::{Key, PointerEventButton, WindowEvent};
use slint::{ComponentHandle, LogicalPosition};
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (ui, controller) = hoshi_canvas_spike::create_app()?;
    let weak = ui.as_weak();
    slint::Timer::single_shot(Duration::from_millis(150), move || {
        let ui = weak.upgrade().unwrap();
        // Native font/window layout may still be settling on a fresh runner.
        // Paint before generating input, and use the current viewport origin
        // rather than retaining coordinates across view-model/layout changes.
        ui.window().take_snapshot().expect("settle native layout");
        let point =
            |dx, dy| LogicalPosition::new(ui.get_canvas_left() + dx, ui.get_canvas_top() + dy);
        let document_offset = |x, y| {
            (
                ui.get_offset_x() + x * ui.get_zoom(),
                ui.get_offset_y() + y * ui.get_zoom(),
            )
        };
        let start = document_offset(96.0, 120.0);
        let end = document_offset(320.0, 280.0);
        for (x, y) in [start, end] {
            assert!(
                x >= 0.0 && x < ui.get_canvas_width() && y >= 0.0 && y < ui.get_canvas_height(),
                "document point ({x}, {y}) must be inside live canvas {} × {}",
                ui.get_canvas_width(),
                ui.get_canvas_height()
            );
        }
        ui.window().dispatch_event(WindowEvent::PointerPressed {
            position: point(start.0, start.1),
            button: PointerEventButton::Left,
        });
        ui.window().dispatch_event(WindowEvent::PointerMoved {
            position: point(end.0, end.1),
        });
        ui.window().dispatch_event(WindowEvent::PointerReleased {
            position: point(end.0, end.1),
            button: PointerEventButton::Left,
        });
        assert!(
            ui.get_has_shape(),
            "desktop pointer events must reach the controller"
        );
        ui.window().take_snapshot().expect("paint editable handles");
        let corner = controller.borrow().handle_positions()[4];
        ui.window().dispatch_event(WindowEvent::PointerPressed {
            position: point(corner.x as f32, corner.y as f32),
            button: PointerEventButton::Left,
        });
        ui.window().dispatch_event(WindowEvent::PointerMoved {
            position: point(corner.x as f32 + 32.0, corner.y as f32 + 16.0),
        });
        ui.window().dispatch_event(WindowEvent::PointerReleased {
            position: point(corner.x as f32 + 32.0, corner.y as f32 + 16.0),
            button: PointerEventButton::Left,
        });
        let shape = controller.borrow().document().active_shape().unwrap();
        assert_eq!((shape.width, shape.height), (256, 176));
        ui.window().take_snapshot().expect("paint resized handles");
        let handles = controller.borrow().handle_positions();
        let knob = handles[8];
        let center = handles[9];
        let radius = center.y - knob.y;
        let destination = (
            center.x + radius / 2.0_f64.sqrt(),
            center.y - radius / 2.0_f64.sqrt(),
        );
        ui.window().dispatch_event(WindowEvent::PointerPressed {
            position: point(knob.x as f32, knob.y as f32),
            button: PointerEventButton::Left,
        });
        ui.window().dispatch_event(WindowEvent::PointerMoved {
            position: point(destination.0 as f32, destination.1 as f32),
        });
        ui.window().dispatch_event(WindowEvent::PointerReleased {
            position: point(destination.0 as f32, destination.1 as f32),
            button: PointerEventButton::Left,
        });
        assert_eq!(
            controller
                .borrow()
                .document()
                .active_shape()
                .unwrap()
                .rotation_degrees,
            45
        );
        // Let standard-widget enable/focus animations settle for the visual artifact.
        slint::Timer::single_shot(Duration::from_millis(200), move || {
            if let Some(path) = std::env::var_os("HOSHI_SPIKE_EDITABLE_SNAPSHOT") {
                let snapshot = ui.window().take_snapshot().unwrap();
                image::save_buffer(
                    path,
                    snapshot.as_bytes(),
                    snapshot.width(),
                    snapshot.height(),
                    image::ColorType::Rgba8,
                )
                .unwrap();
            }
            ui.window().dispatch_event(WindowEvent::KeyPressed {
                text: Key::Return.into(),
            });
            ui.window().dispatch_event(WindowEvent::KeyReleased {
                text: Key::Return.into(),
            });
            assert!(!ui.get_has_shape());
            assert_eq!(controller.borrow().document().undo_len(), 1);
            let snapshot = ui
                .window()
                .take_snapshot()
                .expect("desktop software snapshot");
            let x = ui.get_canvas_left() + ui.get_offset_x() + 224.0 * ui.get_zoom();
            let y = ui.get_canvas_top() + ui.get_offset_y() + 208.0 * ui.get_zoom();
            let sample = snapshot.as_slice()[(y as usize * snapshot.width() as usize) + x as usize];
            assert_eq!((sample.r, sample.g, sample.b), (0, 0, 0));
            if let Some(path) = std::env::var_os("HOSHI_SPIKE_SNAPSHOT") {
                image::save_buffer(
                    path,
                    snapshot.as_bytes(),
                    snapshot.width(),
                    snapshot.height(),
                    image::ColorType::Rgba8,
                )
                .unwrap();
            }
            println!(
                "PASS: desktop window rendered, corner resize and rotation committed, displayed pixels match core color."
            );
            slint::quit_event_loop().unwrap();
        });
    });
    ui.run()?;
    Ok(())
}
