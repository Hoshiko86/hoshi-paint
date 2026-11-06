use slint::ComponentHandle;

fn main() -> Result<(), slint::PlatformError> {
    let (ui, _controller) = hoshi_canvas_spike::create_app()?;
    ui.run()
}
