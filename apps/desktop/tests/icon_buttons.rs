use hoshi_desktop::{AppPanels, AppWindow, DesktopTheme, Palette, PanelPlacement, Translations};
use i_slint_backend_testing::{AccessibleRole, ElementHandle};
use slint::platform::{Key, PointerEventButton, WindowEvent};
use slint::{ComponentHandle, LogicalSize, language::ColorScheme};
use std::{cell::Cell, rc::Rc};

fn backend() {
    slint::platform::set_platform(Box::new(i_slint_backend_testing::TestingBackend::new(
        i_slint_backend_testing::TestingBackendOptions {
            mock_time: true,
            threading: false,
            renderer_name: Some("software".into()),
        },
    )))
    .unwrap();
}
fn app() -> AppPanels {
    let app = hoshi_desktop::create_app_with_all_panels().unwrap();
    app.ui.window().set_size(LogicalSize::new(1120., 850.));
    app.ui.show().unwrap();
    app.ui.window().take_snapshot().unwrap();
    app
}
fn button(ui: &impl ComponentHandle, label: &str) -> ElementHandle {
    ElementHandle::find_by_accessible_label(ui, label)
        .find(|e| e.accessible_role() == Some(AccessibleRole::Button))
        .unwrap()
}
fn image(button: &ElementHandle) -> ElementHandle {
    button
        .query_descendants()
        .match_type_name("Image")
        .find_first()
        .unwrap()
}
fn assert_centered(button: ElementHandle) {
    let icon = image(&button);
    let (b, bs) = (button.absolute_position(), button.size());
    let (i, is) = (icon.absolute_position(), icon.size());
    assert!((b.x + bs.width / 2. - i.x - is.width / 2.).abs() <= 0.5);
    assert!((b.y + bs.height / 2. - i.y - is.height / 2.).abs() <= 0.5);
}

#[test]
fn history_header_command_bar_and_rail_icons_are_centered() {
    backend();
    let app = app();
    app.ui
        .invoke_set_history_placement(PanelPlacement::Floating);
    app.ui.invoke_show_history();
    app.history.window().take_snapshot().unwrap();
    let labels = app.history.global::<Translations>();
    assert_centered(button(
        app.history.as_ref(),
        &format!("{} {}", labels.get_dock_panel(), labels.get_history()),
    ));
    assert_centered(button(app.history.as_ref(), &labels.get_close_panel()));
    let labels = app.ui.global::<Translations>();
    assert_centered(button(&app.ui, &labels.get_undo()));
    assert_centered(button(&app.ui, &labels.get_brush()));
    app.history.hide().unwrap();
    app.ui.hide().unwrap();
}

fn key(ui: &AppWindow, key: Key, pressed: bool) {
    ui.window().dispatch_event(if pressed {
        WindowEvent::KeyPressed { text: key.into() }
    } else {
        WindowEvent::KeyReleased { text: key.into() }
    });
}
#[test]
fn pointer_focus_and_keyboard_activate_once_and_disabled_tools_do_not_activate() {
    backend();
    let app = app();
    let count = Rc::new(Cell::new(0));
    let observed = count.clone();
    app.ui
        .on_show_colors(move || observed.set(observed.get() + 1));
    let labels = app.ui.global::<Translations>();
    button(&app.ui, &labels.get_colors()).mock_single_click(PointerEventButton::Left);
    assert_eq!(count.replace(0), 1);
    key(&app.ui, Key::Space, true);
    key(&app.ui, Key::Space, true);
    assert_eq!(count.get(), 0);
    key(&app.ui, Key::Space, false);
    assert_eq!(count.get(), 1);
    key(&app.ui, Key::Return, true);
    key(&app.ui, Key::Return, false);
    assert_eq!(count.get(), 2);
    let chosen = Rc::new(Cell::new(0));
    let observed = chosen.clone();
    app.ui
        .on_choose_tool(move |_| observed.set(observed.get() + 1));
    app.ui.set_has_document(false);
    app.ui.window().take_snapshot().unwrap();
    let brush = button(&app.ui, &labels.get_brush());
    assert_eq!(brush.accessible_enabled(), Some(false));
    assert!((brush.computed_opacity() - 0.4).abs() < 0.001);
    brush.mock_single_click(PointerEventButton::Left);
    assert_eq!(chosen.get(), 0);
    app.ui.hide().unwrap();
}

#[test]
fn only_the_selected_rail_tool_reports_checked() {
    backend();
    let app = app();
    let labels = app.ui.global::<Translations>();
    let tools = [
        ("shapes", labels.get_shapes()),
        ("paintbrush", labels.get_brush()),
        ("eraser", labels.get_eraser()),
        ("color-picker", labels.get_eyedropper()),
    ];
    for selected in ["shapes", "paintbrush", "eraser", "color-picker"] {
        app.ui.invoke_choose_tool(selected.into());
        app.ui.window().take_snapshot().unwrap();
        for (tool, label) in &tools {
            assert_eq!(
                button(&app.ui, label).accessible_checked(),
                Some(*tool == selected)
            );
        }
    }
    app.ui.hide().unwrap();
}

fn light_text() -> slint::Color {
    let json: serde_json::Value =
        serde_json::from_str(include_str!("../themes/light.json")).unwrap();
    let hex = json["tokens"]["text"].as_str().unwrap();
    slint::Color::from_rgb_u8(
        u8::from_str_radix(&hex[1..3], 16).unwrap(),
        u8::from_str_radix(&hex[3..5], 16).unwrap(),
        u8::from_str_radix(&hex[5..7], 16).unwrap(),
    )
}
#[test]
fn mocked_light_system_starts_all_roots_in_light_and_recolors_the_icon_pixels() {
    backend();
    // Slint 1.18.1 exposes no testing-backend scheme setter. Change only the
    // test context before the factory reads its new AppWindow's initial Palette.
    let probe = AppWindow::new().unwrap();
    slint::private_unstable_api::re_exports::WindowInner::from_pub(probe.window())
        .context()
        .set_color_scheme(ColorScheme::Light);
    drop(probe);
    let app = app();
    let expected = light_text();
    for global in [
        app.ui.global::<DesktopTheme>(),
        app.history.global::<DesktopTheme>(),
        app.colors.global::<DesktopTheme>(),
        app.shapes.global::<DesktopTheme>(),
    ] {
        assert_eq!(global.get_text(), expected);
    }
    for palette in [
        app.ui.global::<Palette>(),
        app.history.global::<Palette>(),
        app.colors.global::<Palette>(),
        app.shapes.global::<Palette>(),
    ] {
        assert_eq!(palette.get_color_scheme(), ColorScheme::Light);
    }
    let labels = app.ui.global::<Translations>();
    let icon = image(&button(&app.ui, &labels.get_colors()));
    assert_icon_uses_text_color(&app.ui, icon, expected);
    app.ui.hide().unwrap();
}

fn assert_icon_uses_text_color(ui: &AppWindow, icon: ElementHandle, text: slint::Color) {
    let snapshot = ui.window().take_snapshot().unwrap();
    let (p, size) = (icon.absolute_position(), icon.size());
    let scale = ui.window().scale_factor();
    let (x0, y0) = ((p.x * scale) as usize, (p.y * scale) as usize);
    let width = snapshot.width() as usize;
    let background = snapshot.as_slice()[y0 * width + x0];
    let bg = [background.r, background.g, background.b].map(f32::from);
    let ink = [text.red(), text.green(), text.blue()].map(f32::from);
    let mut matching_strokes = 0;
    for y in y0..((p.y + size.height) * scale) as usize {
        for x in x0..((p.x + size.width) * scale) as usize {
            let pixel = snapshot.as_slice()[y * width + x];
            let rgb = [pixel.r, pixel.g, pixel.b].map(f32::from);
            // Alpha-aware: a tinted stroke must be a blend of the sampled
            // background and this palette's text, not the SVG's pale source.
            let alpha = (rgb[0] - bg[0]) / (ink[0] - bg[0]);
            if (0.15..=1.01).contains(&alpha)
                && (0..3).all(|i| (rgb[i] - (bg[i] + alpha * (ink[i] - bg[i]))).abs() < 3.)
            {
                matching_strokes += 1;
            }
        }
    }
    assert!(
        matching_strokes > 0,
        "icon output must use the Light text color"
    );
}
