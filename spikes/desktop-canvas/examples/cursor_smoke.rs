#[cfg(target_os = "linux")]
pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    use slint::ComponentHandle;
    use std::{process::Command, time::Duration};
    const SCRIPT: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../scripts/check_x11_cursor.py"
    );
    /// The cursor changes when the event loop handles the warped pointer, and
    /// a check blocks that loop. So retry from fresh timers instead of
    /// failing on the first look (CI once saw the 16x16 default cursor).
    fn check_cursor(expected: [String; 4], attempt: u32) {
        const ATTEMPTS: u32 = 15;
        let output = Command::new("python3")
            .arg(SCRIPT)
            .arg("check")
            .args(&expected)
            .output()
            .unwrap();
        if output.status.success() {
            print!("{}", String::from_utf8_lossy(&output.stdout));
            println!(
                "PASS: the native desktop cursor is the outlined Lucide Color Picker, with the correct hotspot."
            );
            slint::quit_event_loop().unwrap();
            return;
        }
        assert!(
            attempt < ATTEMPTS,
            "native cursor check failed {ATTEMPTS} times, 200 ms apart:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        slint::Timer::single_shot(Duration::from_millis(200), move || {
            check_cursor(expected, attempt + 1)
        });
    }
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
        let size = ((32.0 * scale) as u32).to_string();
        let hot_x = (3.0 * scale).round().to_string();
        let hot_y = (29.0 * scale).round().to_string();
        let expected = [size.clone(), size, hot_x, hot_y];
        slint::Timer::single_shot(Duration::from_millis(300), move || {
            check_cursor(expected, 1)
        });
    });
    ui.run()?;
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub fn main() {
    println!("Native cursor inspection uses X11 and runs in Linux CI.");
}
