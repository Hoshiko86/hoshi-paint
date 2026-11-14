//! Every native desktop smoke, built as one example binary.
//!
//! Each `*_smoke.rs` file used to be its own executable, statically linked
//! against Slint and every other dependency. Run one with
//! `cargo run -p hoshi-canvas-spike --example smoke -- <name>`; `cargo xtask test
//! smoke` does this for every entry in `xtask/src/smoke.rs`. A new smoke needs
//! a `mod` line and a match arm here; `cargo test -p xtask` checks both.

use std::process::ExitCode;

mod bounds_smoke;
mod color_library_smoke;
mod color_smoke;
mod cursor_smoke;
mod desktop_smoke;
mod disk_history_smoke;
mod documents_smoke;
mod image_settings_smoke;
mod large_canvas_smoke;
mod panels_smoke;
mod portal_smoke;
mod rectangle_editing_smoke;
mod rendering_smoke;
mod shape_properties_smoke;

/// What a smoke's `main` returns: nothing (it panics on failure) or a result.
trait Outcome {
    fn exit_code(self) -> ExitCode;
}

impl Outcome for () {
    fn exit_code(self) -> ExitCode {
        ExitCode::SUCCESS
    }
}

impl Outcome for Result<(), Box<dyn std::error::Error>> {
    fn exit_code(self) -> ExitCode {
        match self {
            Ok(()) => ExitCode::SUCCESS,
            // Same report as a `main` that returns `Err`.
            Err(error) => {
                eprintln!("Error: {error:?}");
                ExitCode::FAILURE
            }
        }
    }
}

fn run<T: Outcome>(smoke: fn() -> T) -> ExitCode {
    smoke().exit_code()
}

fn main() -> ExitCode {
    let name = std::env::args().nth(1).unwrap_or_default();
    match name.as_str() {
        "bounds_smoke" => run(bounds_smoke::main),
        "color_library_smoke" => run(color_library_smoke::main),
        "color_smoke" => run(color_smoke::main),
        "cursor_smoke" => run(cursor_smoke::main),
        "desktop_smoke" => run(desktop_smoke::main),
        "disk_history_smoke" => run(disk_history_smoke::main),
        "documents_smoke" => run(documents_smoke::main),
        "image_settings_smoke" => run(image_settings_smoke::main),
        "large_canvas_smoke" => run(large_canvas_smoke::main),
        "panels_smoke" => run(panels_smoke::main),
        "portal_smoke" => run(portal_smoke::main),
        "rectangle_editing_smoke" => run(rectangle_editing_smoke::main),
        "rendering_smoke" => run(rendering_smoke::main),
        "shape_properties_smoke" => run(shape_properties_smoke::main),
        _ => {
            eprintln!("usage: smoke <name>; names are listed by `cargo xtask list`");
            ExitCode::from(2)
        }
    }
}
