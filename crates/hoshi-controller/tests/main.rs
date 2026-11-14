//! Every integration test of this crate, built as one test binary.
//!
//! Each `tests/*.rs` file used to be its own executable, statically linked
//! against every dependency. One binary links once. Add a new test file here
//! as a module; `cargo test -p xtask` fails if a file is left out.

mod bounds;
mod canvas;
mod color;
mod eyedropper;
mod history;
mod input;
mod large_preview;
mod named_colors;
mod painting;
mod palette;
mod rendering;
mod shape_editing;
mod styles;
mod support;
