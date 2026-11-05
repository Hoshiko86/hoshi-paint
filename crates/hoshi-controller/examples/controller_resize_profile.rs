//! CPU-only resize profiling; not a measurement of window/input/display latency.
//! Run both `cargo run -p hoshi-controller --example controller_resize_profile --locked`
//! and the same command with `--release`. Optional argument: preview sample count.
use hoshi_controller::Controller;
use std::{hint::black_box, time::Instant};

fn controller(rotation: i32, locked: bool) -> (Controller, (f64, f64)) {
    let mut controller = Controller::new(512, 512).unwrap();
    controller.pointer_down(96.0, 128.0).unwrap();
    controller.pointer_move(320.0, 272.0).unwrap();
    controller.pointer_up().unwrap();
    controller.set_shape_rotation(rotation).unwrap();
    controller.set_aspect_lock(locked).unwrap();
    let handle = controller.handle_positions()[4];
    controller.pointer_down(handle.x, handle.y).unwrap();
    (controller, (handle.x, handle.y))
}

fn point(anchor: (f64, f64), index: usize) -> (f64, f64) {
    // Repeat a bounded drag path, keeping work comparable between runs.
    let step = (index % 64) as f64;
    (anchor.0 + step * 1.5, anchor.1 + step * 0.75)
}

fn measure(rotation: i32, locked: bool, preview: bool, samples: usize) {
    let (mut controller, anchor) = controller(rotation, locked);
    let mut timings = Vec::with_capacity(samples);
    for index in 0..samples + 8 {
        let (x, y) = point(anchor, index);
        let start = Instant::now();
        controller.pointer_move(black_box(x), black_box(y)).unwrap();
        if preview {
            black_box(controller.preview_rgba());
        } else {
            black_box(controller.document().active_shape());
        }
        let elapsed = start.elapsed().as_secs_f64() * 1_000_000.0;
        if index >= 8 {
            timings.push(elapsed);
        }
    }
    timings.sort_by(f64::total_cmp);
    println!(
        "rotation={rotation:2} locked={locked:5} path={:16} n={samples:4} p50_us={:10.3} p95_us={:10.3}",
        if preview { "move+preview" } else { "move-only" },
        timings[timings.len() / 2],
        timings[(timings.len() * 95 / 100).min(timings.len() - 1)],
    );
}

fn main() {
    let samples = std::env::args()
        .nth(1)
        .map(|value| value.parse::<usize>().unwrap())
        .unwrap_or(60);
    assert!(
        (10..=10_000).contains(&samples),
        "use 10..=10000 preview samples"
    );
    println!(
        "CPU resize benchmark; debug_assertions={}; 512x512 RGBA; no Slint, GPU, display or OS input",
        cfg!(debug_assertions)
    );
    for rotation in [0, 33] {
        for locked in [false, true] {
            measure(rotation, locked, false, 2000);
            measure(rotation, locked, true, samples);
        }
    }
}
