//! Shared helpers for desktop integration tests.

use hoshi_core::{Color, Document, PixelStore, Shape, ShapeKind, ShapeStyle};
use std::process::Command;

/// Set in the child process to the name of the test it must run.
const ISOLATED_TEST: &str = "HOSHI_ISOLATED_TEST";

/// Runs `body` in a fresh process, for tests that exhaust or inspect the
/// process-wide memory budget (`hoshi_core::memory_budget`). `cargo test` runs
/// tests as threads of one process, so they would otherwise interfere.
///
/// Call it as the whole test body, with `test_name` equal to the test's full
/// libtest name: its module (the file under `tests/`) and function, such as
/// `save_failures::png_save_failing_after_the_temp_file_keeps_destination_tab_and_history`. The
/// parent re-runs this test binary with `--exact test_name` and asserts the child ran exactly one test, so a renamed test
/// or a typo fails instead of passing without running `body`.
pub fn run_isolated(test_name: &str, body: impl FnOnce()) {
    if std::env::var(ISOLATED_TEST).as_deref() == Ok(test_name) {
        body();
        return;
    }
    let exe = std::env::current_exe().expect("test binary path");
    let output = Command::new(exe)
        .args(["--exact", test_name, "--test-threads=1"])
        .env(ISOLATED_TEST, test_name)
        .output()
        .expect("spawn isolated test");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "isolated test `{test_name}` failed:\n{stdout}\n{stderr}"
    );
    assert!(
        stdout.lines().any(|line| line.trim() == "running 1 test"),
        "isolated run of `{test_name}` ran no test; is the name wrong?\n{stdout}"
    );
}

/// Deterministic RGBA pixels, including transparent pixels with nonzero RGB.
pub fn noise(width: u32, height: u32) -> Vec<u8> {
    (0..height)
        .flat_map(|y| {
            (0..width).flat_map(move |x| {
                [
                    (x * 17 + y * 11) as u8,
                    (x * 7 + y * 23) as u8,
                    ((x >> 4) ^ (y >> 3)) as u8,
                    if (x + y).is_multiple_of(7) { 0 } else { 173 },
                ]
            })
        })
        .collect()
}

/// A 64×48 document at 144.125 DPI with background, Ellipse (opacity 128),
/// and Hidden rectangle (hidden red rectangle at (4, 5), sized 8×7) layers.
pub fn layered() -> Document {
    let mut pixels = PixelStore::new(64, 48, Color::TRANSPARENT);
    for (i, rgba) in noise(64, 48).as_chunks::<4>().0.iter().enumerate() {
        pixels
            .set(
                i as u32 % 64,
                i as u32 / 64,
                Color::rgba(rgba[0], rgba[1], rgba[2], rgba[3]),
            )
            .unwrap();
    }
    let mut document = Document::from_pixels(64, 48, 144.125, pixels).unwrap();
    document.add_layer("Ellipse").unwrap();
    let mut shape = Shape {
        kind: ShapeKind::Ellipse,
        x: 12,
        y: 9,
        width: 31,
        height: 23,
        rotation_degrees: 27,
        flip_horizontal: false,
        flip_vertical: false,
        rendering: hoshi_core::RenderOptions {
            antialias: true,
            ..Default::default()
        },
        style: ShapeStyle::Fill(Color::rgba(40, 170, 220, 200)),
    };
    document.begin_shape(shape).unwrap();
    document.commit_shape().unwrap();
    document.set_layer_properties(1, true, 128).unwrap();
    document.add_layer("Hidden rectangle").unwrap();
    shape.kind = ShapeKind::Rectangle;
    shape.x = 4;
    shape.y = 5;
    shape.width = 8;
    shape.height = 7;
    shape.rotation_degrees = 0;
    shape.rendering.antialias = false;
    shape.style = ShapeStyle::Fill(Color::rgba(255, 0, 0, 255));
    document.begin_shape(shape).unwrap();
    document.commit_shape().unwrap();
    document.set_layer_properties(2, false, 255).unwrap();
    document
}
