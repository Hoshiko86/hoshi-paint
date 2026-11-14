use slint::ComponentHandle;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let paths: Vec<_> = std::env::args_os().skip(1).collect();
    if paths.first().is_some_and(|p| p == "--help" || p == "-h") {
        println!(
            "Hoshi Paint\nUsage: hoshi-canvas-spike [IMAGE ...]\nOpens Hoshi projects and supported raster images in separate tabs."
        );
        return Ok(());
    }
    let (ui, state) = hoshi_canvas_spike::create_app()?;
    let initial = state.borrow().active_id();
    let mut opened = false;
    let mut errors = Vec::new();
    for path in paths {
        match state.borrow_mut().open_path(std::path::Path::new(&path)) {
            Ok(_) => opened = true,
            Err(error) => errors.push(format!(
                "{}: {}",
                path.to_string_lossy(),
                hoshi_canvas_spike::describe_error(&error)
            )),
        }
    }
    if opened && let Some(id) = initial {
        state.borrow_mut().close_discard(id)?;
    }
    hoshi_canvas_spike::refresh_workspace(&ui, &state);
    if !errors.is_empty() {
        ui.set_file_error(errors.join("\n").into())
    }
    ui.run()?;
    Ok(())
}
