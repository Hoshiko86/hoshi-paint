//! Real winit/software backend launch; run under Xvfb in headless Linux CI.
use slint::platform::{Key, PointerEventButton, WindowEvent};
use slint::{ComponentHandle, LogicalPosition};
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (ui, controller) = hoshi_canvas_spike::create_app()?;
    let weak = ui.as_weak();
    slint::Timer::single_shot(Duration::from_millis(150), move || {
        let ui = weak.upgrade().unwrap();
        let x = ui.get_canvas_left();
        let y = ui.get_canvas_top();
        let point = |dx, dy| LogicalPosition::new(x + dx, y + dy);
        ui.window().dispatch_event(WindowEvent::PointerPressed {
            position: point(64.0, 64.0),
            button: PointerEventButton::Left,
        });
        ui.window().dispatch_event(WindowEvent::PointerMoved {
            position: point(320.0, 240.0),
        });
        ui.window().dispatch_event(WindowEvent::PointerReleased {
            position: point(320.0, 240.0),
            button: PointerEventButton::Left,
        });
        assert!(
            ui.get_has_shape(),
            "desktop pointer events must reach the controller"
        );
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
        let sample = snapshot.as_slice()
            [((y as usize + 100) * snapshot.width() as usize) + x as usize + 100];
        assert_eq!((sample.r, sample.g, sample.b), (72, 136, 240));
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
            "PASS: desktop window rendered, pointer drag committed, displayed pixels match core color."
        );
        slint::quit_event_loop().unwrap();
    });
    ui.run()?;
    Ok(())
}
