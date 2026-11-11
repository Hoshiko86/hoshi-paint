use hoshi_controller::{Color, Controller, Tool};

#[test]
fn large_canvas_previews_are_bounded_and_respect_pan_zoom_and_transparency() {
    let mut c = Controller::new(262_144, 262_144).unwrap();
    let preview = c.render_viewport(160, 100, 2.);
    assert_eq!((preview.width, preview.height), (320, 200));
    assert_eq!(preview.rgba.len(), 320 * 200 * 4);
    assert_eq!(&preview.rgba[..4], &[224, 224, 224, 255]);
    let thumb = c.thumbnail(40, 32);
    assert_eq!((thumb.width, thumb.height), (40, 32));
    assert_eq!(thumb.rgba.len(), 40 * 32 * 4);
    assert_eq!(
        &thumb.rgba[..4],
        &[0, 0, 0, 0],
        "aspect-preserving thumbnail margins"
    );
    let (pw, ph) = c.preview_size();
    assert_eq!((pw, ph), (1024, 1024));
    assert_eq!(c.preview_rgba().len(), 1024 * 1024 * 4);
    // A small preview shape on a huge canvas must be visible without a full composite.
    c.set_primary_color(Color::rgba(240, 30, 20, 255)).unwrap();
    c.pointer_down(10., 10.).unwrap();
    c.pointer_move(30., 30.).unwrap();
    c.pointer_up().unwrap();
    let image = c.render_viewport(64, 64, 1.);
    let index = (15 * 64 + 15) * 4;
    assert_eq!(&image.rgba[index..index + 4], &[240, 30, 20, 255]);
    c.pan(10., 20.).unwrap();
    let image = c.render_viewport(64, 64, 1.);
    assert_eq!(
        &image.rgba[..4],
        &[0, 0, 0, 0],
        "outside the image is transparent workspace"
    );
    let index = (35 * 64 + 25) * 4;
    assert_eq!(&image.rgba[index..index + 4], &[240, 30, 20, 255]);
    c.zoom_at(0., 0., 2.).unwrap();
    let viewport = c.viewport();
    let x = ((15. * viewport.zoom + viewport.offset_x) * 2.).floor() as usize;
    let y = ((15. * viewport.zoom + viewport.offset_y) * 2.).floor() as usize;
    let image = c.render_viewport(128, 128, 2.);
    let index = (y * image.width as usize + x) * 4;
    assert_eq!(&image.rgba[index..index + 4], &[240, 30, 20, 255]);
}

#[test]
fn viewport_upload_budget_is_independent_of_canvas_or_window_size() {
    let c = Controller::new(262_144, 2).unwrap();
    let preview = c.render_viewport(20_000, 10_000, 4.);
    assert!(preview.width <= 4096 && preview.height <= 4096);
    assert!(u64::from(preview.width) * u64::from(preview.height) <= 4_194_304);
    assert_eq!(
        preview.rgba.len(),
        preview.width as usize * preview.height as usize * 4
    );
}

#[test]
fn canvas_picker_freezes_only_a_sparse_snapshot_including_the_preview() {
    let mut c = Controller::new(262_144, 262_144).unwrap();
    c.set_primary_color(Color::rgba(200, 80, 20, 128)).unwrap();
    c.pointer_down(10., 10.).unwrap();
    c.pointer_move(30., 30.).unwrap();
    c.pointer_up().unwrap();
    c.set_tool(Tool::Eyedropper).unwrap();
    c.pointer_down(15., 15.).unwrap();
    let first = c.primary_color();
    c.pointer_move(16., 16.).unwrap();
    assert_eq!(
        c.primary_color(),
        first,
        "sampling must not feed recolored preview pixels back into the drag"
    );
    c.pointer_up().unwrap();
}

#[test]
fn fit_view_makes_the_maximum_canvas_accessible_and_keeps_zoom_anchored() {
    let mut c = Controller::new(262_144, 262_144).unwrap();
    c.fit_viewport(1000., 700.).unwrap();
    let view = c.viewport();
    assert!(view.zoom < 0.01);
    assert!(view.offset_x >= 0. && view.offset_y >= 0.);
    assert!(262_144. * view.zoom + view.offset_x <= 1000.);
    assert!(262_144. * view.zoom + view.offset_y <= 700.);
    let point = view.to_document(500., 350.);
    c.zoom_at(500., 350., 2.).unwrap();
    let next = c.viewport().to_document(500., 350.);
    assert!((next.0 - point.0).abs() < 1e-6 && (next.1 - point.1).abs() < 1e-6);
    c.fit_viewport(240., 200.).unwrap();
    assert!(262_144. * c.viewport().zoom + c.viewport().offset_y <= 200.);
    let before = c.viewport();
    assert!(c.fit_viewport(f64::NAN, 500.).is_err());
    assert_eq!(c.viewport(), before);
}

#[test]
fn center_view_keeps_one_to_one_zoom_and_places_the_document_in_the_workspace() {
    let mut c = Controller::new(320, 200).unwrap();
    c.center_viewport(1000., 700.).unwrap();
    let view = c.viewport();
    assert_eq!(view.zoom, 1.);
    assert_eq!(view.offset_x, 340.);
    assert_eq!(view.offset_y, 250.);

    let before = view;
    assert!(c.center_viewport(0., 700.).is_err());
    assert_eq!(c.viewport(), before);
}

#[test]
fn very_thin_images_remain_visible_in_thumbnails_and_fit_view() {
    for (width, height) in [(262_144, 1), (1, 262_144)] {
        let mut c = Controller::new(width, height).unwrap();
        let preview = c.thumbnail(40, 32);
        assert!(preview.rgba.as_chunks::<4>().0.iter().any(|p| p[3] == 255));
        let preview = c.preview_rgba();
        assert!(preview.as_chunks::<4>().0.iter().any(|p| p[3] == 255));
        c.fit_viewport(1000., 700.).unwrap();
        let preview = c.render_viewport(1000, 700, 1.);
        assert!(preview.rgba.as_chunks::<4>().0.iter().any(|p| p[3] == 255));
    }
}
