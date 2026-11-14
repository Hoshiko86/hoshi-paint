//! Every integration test of this crate, built as one test binary.
//!
//! Each `tests/*.rs` file used to be its own executable, statically linked
//! against every dependency. One binary links once. Add a new test file here
//! as a module; `cargo test -p xtask` fails if a file is left out.

mod aspect_ratio;
mod color_comparison;
mod color_library;
mod color_slots;
mod colors;
mod disk_history;
mod dock_positions;
mod docking;
mod documents;
mod drawing_tools;
mod handles;
mod history_panel;
mod image_settings;
mod interaction;
mod large_canvas;
mod large_codecs;
mod navigation;
mod palette_store;
mod panel_lifecycle;
mod rectangle_editing;
mod resampling;
mod shape_bounds;
mod shape_properties;
mod shape_rendering;
mod shape_styles;
mod tooltips;
