use slint::ComponentHandle;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let paths: Vec<_> = std::env::args_os().skip(1).collect();
    if paths.first().is_some_and(|p| p == "--help" || p == "-h") {
        println!("{}", hoshi_desktop::usage_text());
        return Ok(());
    }
    let (ui, state) = hoshi_desktop::create_app()?;
    let initial = state.borrow().active_id();
    let mut opened = false;
    let mut errors = Vec::new();
    for path in paths {
        match state.borrow_mut().open_path(std::path::Path::new(&path)) {
            Ok(_) => opened = true,
            Err(error) => errors.push(format!(
                "{}: {}",
                path.to_string_lossy(),
                hoshi_desktop::describe_error(&error)
            )),
        }
    }
    if opened && let Some(id) = initial {
        state.borrow_mut().close_discard(id)?;
    }
    hoshi_desktop::refresh_workspace(&ui, &state);
    if !errors.is_empty() {
        ui.set_file_error(errors.join("\n").into())
    }
    ui.run()?;
    Ok(())
}
