//! Real desktop input and shared editing controls, without an OS-specific UI.
use hoshi_canvas_spike::create_app;
use slint::platform::{Key, PointerEventButton as Button, WindowEvent};
use slint::{ComponentHandle, LogicalPosition, LogicalSize};
use std::{path::PathBuf, time::Duration};
pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (ui, state) = create_app()?;
    ui.window().set_size(LogicalSize::new(1120., 850.));
    let weak = ui.as_weak();
    slint::Timer::single_shot(Duration::from_millis(200), move || {
        let ui = weak.upgrade().unwrap();
        ui.window().take_snapshot().expect("settle native layout");
        let point = |x: f64, y: f64| {
            LogicalPosition::new(
                ui.get_canvas_left() + x as f32,
                ui.get_canvas_top() + y as f32,
            )
        };
        let document_point = |x: f64, y: f64| {
            (
                f64::from(ui.get_offset_x()) + x * f64::from(ui.get_zoom()),
                f64::from(ui.get_offset_y()) + y * f64::from(ui.get_zoom()),
            )
        };
        let drag = |from: (f64, f64), to: (f64, f64), button| {
            ui.window().dispatch_event(WindowEvent::PointerPressed {
                position: point(from.0, from.1),
                button,
            });
            ui.window().dispatch_event(WindowEvent::PointerMoved {
                position: point(to.0, to.1),
            });
            ui.window().dispatch_event(WindowEvent::PointerReleased {
                position: point(to.0, to.1),
                button,
            });
        };
        drag(
            document_point(100., 100.),
            document_point(300., 220.),
            Button::Left,
        );
        let original = state.borrow().document().active_shape().unwrap();
        let pivot = state.borrow().handle_positions()[9];
        drag((pivot.x, pivot.y), document_point(380., 280.), Button::Left);
        assert_eq!(state.borrow().document().active_shape(), Some(original));
        let center = (
            f64::from(original.x) + f64::from(original.width) / 2.,
            f64::from(original.y) + f64::from(original.height) / 2.,
        );
        let radians = 17_f64.to_radians();
        let offset = (center.0 - 380., center.1 - 280.);
        let target = (
            380. + offset.0 * radians.cos() - offset.1 * radians.sin(),
            280. + offset.0 * radians.sin() + offset.1 * radians.cos(),
        );
        ui.window().dispatch_event(WindowEvent::KeyPressed {
            text: Key::Shift.into(),
        });
        drag(
            document_point(center.0, center.1),
            document_point(target.0, target.1),
            Button::Right,
        );
        ui.window().dispatch_event(WindowEvent::KeyReleased {
            text: Key::Shift.into(),
        });
        assert_eq!(
            state
                .borrow()
                .document()
                .active_shape()
                .unwrap()
                .rotation_degrees,
            15
        );
        let snapshot = ui.window().take_snapshot().unwrap();
        let path = PathBuf::from(
            std::env::var_os("HOSHI_EDITING_SNAPSHOT")
                .unwrap_or_else(|| "target/spike-rectangle-editing.png".into()),
        );
        image::save_buffer(
            path,
            snapshot.as_bytes(),
            snapshot.width(),
            snapshot.height(),
            image::ColorType::Rgba8,
        )
        .unwrap();
        ui.window().dispatch_event(WindowEvent::KeyPressed {
            text: Key::Control.into(),
        });
        ui.window().dispatch_event(WindowEvent::KeyPressed {
            text: Key::RightArrow.into(),
        });
        ui.window().dispatch_event(WindowEvent::KeyReleased {
            text: Key::RightArrow.into(),
        });
        ui.window().dispatch_event(WindowEvent::KeyReleased {
            text: Key::Control.into(),
        });
        ui.invoke_action(2);
        assert!(state.borrow().document().active_shape().is_none());
        ui.invoke_action(0);
        assert_eq!(state.borrow().document().undo_len(), 0);
        ui.hide().unwrap();
        println!(
            "PASS: native shared shape controls, custom pivot, snapped right-button rotation, keyboard movement, commit and undo"
        );
    });
    ui.run()?;
    Ok(())
}
