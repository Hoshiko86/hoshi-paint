//! Repeatable CPU/UI resize probe, not physical pointer-to-photon latency.
use hoshi_controller::Color;
use slint::{ComponentHandle, LogicalSize};
use std::{hint::black_box, time::Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    slint::platform::set_platform(Box::new(i_slint_backend_testing::TestingBackend::new(
        i_slint_backend_testing::TestingBackendOptions {
            mock_time: true,
            threading: false,
            renderer_name: Some("software".into()),
        },
    )))?;
    let (ui, state, picker) = hoshi_canvas_spike::create_app_with_colors()?;
    ui.window().set_size(LogicalSize::new(1120.0, 760.0));
    ui.show()?;
    ui.invoke_pointer_down(80.0, 80.0);
    ui.invoke_pointer_move(280.0, 180.0);
    ui.invoke_pointer_up(280.0, 180.0);
    picker.invoke_toggle_dock();
    ui.window().take_snapshot()?;
    for locked in [false, true] {
        ui.invoke_shift_modifier(locked);
        let handle = state.borrow().handle_positions()[4];
        ui.invoke_pointer_down(handle.x as f32, handle.y as f32);
        let mut update_times = Vec::new();
        let mut frame_times = Vec::new();
        for i in 0..32 {
            let start = Instant::now();
            ui.invoke_pointer_move(
                handle.x as f32 + 30.0 + (i % 8) as f32,
                handle.y as f32 + 20.0,
            );
            let update = start.elapsed();
            black_box(ui.window().take_snapshot()?);
            if i >= 4 {
                update_times.push(update.as_secs_f64() * 1000.0);
                frame_times.push(start.elapsed().as_secs_f64() * 1000.0);
            }
        }
        ui.invoke_pointer_up(handle.x as f32 + 37.0, handle.y as f32 + 20.0);
        update_times.sort_by(f64::total_cmp);
        frame_times.sort_by(f64::total_cmp);
        println!(
            "locked={locked} docked=true logical=1120x760 samples={} update_ms p50={:.3} p95={:.3} update_plus_software_snapshot_ms p50={:.3} p95={:.3}",
            update_times.len(),
            update_times[14],
            update_times[26],
            frame_times[14],
            frame_times[26]
        );
    }
    let color = Color::rgba(12, 34, 56, 255);
    picker.invoke_set_rgba(12, 34, 56, 255);
    assert!(state.borrow().composite().contains(&color));
    ui.hide()?;
    Ok(())
}
