//! Compiles the Slint UI. `ui/main-window.slint` is the root; it imports
//! every other layout, so one compile covers the whole tree.
fn main() {
    // Without a build script the compile error is the generic "slint! failed";
    // with one, Slint reports the offending .slint file and line.
    if let Err(error) = slint_build::compile("ui/main-window.slint") {
        eprintln!("slint compile failed: {error}");
        std::process::exit(1);
    }
}
