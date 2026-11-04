#[cfg(target_os = "linux")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use slint::ComponentHandle;
    use std::{process::Command, time::Duration};
    const SCRIPT: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../scripts/check_x11_cursor.py"
    );
    let (ui, _, picker) = hoshi_canvas_spike::create_app_with_colors()?;
    let weak = ui.as_weak();
    slint::Timer::single_shot(Duration::from_millis(150), move || {
        let ui = weak.upgrade().unwrap();
        // Match the real picker: synchronize the current tool before entering
        // the shared pending-selection state and moving the native pointer.
        ui.invoke_choose_tool(0);
        picker.set_screen_picking(true);
        ui.set_screen_picking(true);
        let scale = ui.window().scale_factor();
        if let Ok(requested) = std::env::var("SLINT_SCALE_FACTOR") {
            assert_eq!(
                scale,
                requested.parse::<f32>().unwrap(),
                "native check must use the requested display scale"
            );
        }
        let position = ui.window().position();
        let x = position.x + ((ui.get_canvas_left() + 64.0) * scale) as i32;
        let y = position.y + ((ui.get_canvas_top() + 64.0) * scale) as i32;
        assert!(
            Command::new("python3")
                .args([SCRIPT, "warp", &x.to_string(), &y.to_string()])
                .status()
                .unwrap()
                .success()
        );
        slint::Timer::single_shot(Duration::from_millis(300), move || {
            let size = ((32.0 * scale) as u32).to_string();
            let hot_x = (3.0 * scale).round().to_string();
            let hot_y = (29.0 * scale).round().to_string();
            assert!(
                Command::new("python3")
                    .args([SCRIPT, "check", &size, &size, &hot_x, &hot_y])
                    .status()
                    .unwrap()
                    .success()
            );
            println!(
                "PASS: the native desktop cursor is the outlined Font Awesome eyedropper, with the correct hotspot."
            );
            slint::quit_event_loop().unwrap();
        });
    });
    ui.run()?;
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn main() {
    println!("Native cursor inspection uses X11 and runs in Linux CI.");
}
