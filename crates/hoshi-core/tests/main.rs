//! Every integration test of this crate, built as one test binary.
//!
//! Each `tests/*.rs` file used to be its own executable, statically linked
//! against every dependency. One binary links once. Add a new test file here
//! as a module; `cargo test -p xtask` fails if a file is left out.

mod canvas;
mod disk_history;
mod editing;
mod failure_paths;
mod geometry;
mod history;
mod rendering;
mod strokes;
