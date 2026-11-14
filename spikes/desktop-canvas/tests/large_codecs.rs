use hoshi_canvas_spike::files::{self, FileFormat};
use hoshi_core::{Color, Document, MAX_DIMENSION, PixelStore};
#[test]
fn full_width_png_streams_rows_and_round_trips_alpha_and_density() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("wide.png");
    let mut pixels = PixelStore::new(MAX_DIMENSION, 9, Color::rgba(250, 240, 230, 255));
    pixels
        .set(MAX_DIMENSION - 1, 8, Color::rgba(12, 34, 56, 0))
        .unwrap();
    let d = Document::from_pixels(MAX_DIMENSION, 9, 300.125, pixels).unwrap();
    files::save_atomic(&path, &d, FileFormat::Png, 95).unwrap();
    let loaded = files::load(&path).unwrap();
    assert_eq!(loaded.snapshot(), d.snapshot());
    assert_eq!(loaded.resolution(), 300.125);
}
#[test]
fn giant_native_project_is_compact_and_jpeg_rejects_its_format_dimension_limit_safely() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("large.hoshi");
    let d = Document::new_with_background(
        MAX_DIMENSION,
        MAX_DIMENSION,
        96.,
        Color::rgba(255, 255, 255, 255),
    )
    .unwrap();
    files::save_atomic(&path, &d, FileFormat::Project, 95).unwrap();
    assert!(std::fs::metadata(&path).unwrap().len() < 128);
    assert_eq!(files::load(&path).unwrap().snapshot(), d.snapshot());
    let jpg = dir.path().join("existing.jpg");
    std::fs::write(&jpg, b"original").unwrap();
    assert!(matches!(
        files::save_atomic(&jpg, &d, FileFormat::Jpeg, 95),
        Err(files::FileError::JpegDimensionsTooLarge)
    ));
    assert_eq!(std::fs::read(&jpg).unwrap(), b"original");
}
#[test]
fn adam7_png_rows_match_the_standard_full_frame_decoder() {
    // Generated using: convert -size 7x5 gradient:red-blue -depth 8 -interlace PNG PNG32:adam7-rgba8.png
    let fixture = include_bytes!("fixtures/adam7-rgba8.png");
    let mut reference = png::Decoder::new(std::io::Cursor::new(fixture))
        .read_info()
        .unwrap();
    assert!(reference.info().interlaced);
    let mut raw = vec![0; reference.output_buffer_size().unwrap()];
    let frame = reference.next_frame(&mut raw).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("interlaced.png");
    std::fs::write(&path, fixture).unwrap();
    let loaded = files::load(&path).unwrap();
    for y in 0..frame.height {
        for x in 0..frame.width {
            let i = (y as usize * frame.width as usize + x as usize) * 4;
            assert_eq!(
                loaded.layers()[0].pixels().get(x, y),
                Color::rgba(raw[i], raw[i + 1], raw[i + 2], raw[i + 3])
            );
        }
    }
}

#[test]
fn encoded_files_above_the_old_128_mib_cap_use_a_working_memory_budget() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("large-encoded.jpg");
    let d = Document::new_with_background(8, 8, 96., Color::rgba(80, 120, 160, 255)).unwrap();
    files::save_atomic(&path, &d, FileFormat::Jpeg, 95).unwrap();
    // JPEG permits trailing bytes after EOI. A sparse tail keeps the fixture tiny
    // on disk while exercising the decoder's real >128MiB retained input buffer.
    std::fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .unwrap()
        .set_len(129 * 1024 * 1024)
        .unwrap();
    let loaded = files::load(&path).unwrap();
    assert_eq!((loaded.width(), loaded.height()), (8, 8));
    let c = loaded.layers()[0].pixels().get(0, 0);
    assert!(c.r.abs_diff(80) < 5 && c.g.abs_diff(120) < 5 && c.b.abs_diff(160) < 5);
}

#[test]
fn png_background_sampling_keeps_a_corner_dot_and_first_row_stripe_sparse() {
    let dir = tempfile::tempdir().unwrap();
    for stripe in [false, true] {
        let mut pixels = PixelStore::new(1024, 1024, Color::rgba(255, 255, 255, 255));
        for x in 0..if stripe { 1024 } else { 1 } {
            pixels.set(x, 0, Color::rgba(0, 0, 0, 255)).unwrap();
        }
        let document = Document::from_pixels(1024, 1024, 96., pixels).unwrap();
        let path = dir.path().join(format!("sparse-{stripe}.png"));
        files::save_atomic(&path, &document, FileFormat::Png, 95).unwrap();
        let loaded = files::load(&path).unwrap();
        assert_eq!(loaded.snapshot(), document.snapshot());
        assert_eq!(
            loaded.layers()[0].pixels().background(),
            Color::rgba(255, 255, 255, 255)
        );
        assert!(loaded.layers()[0].pixels().allocated_bytes() <= 8 * 128 * 128 * 4);
    }
}

#[test]
fn ordinary_4k_nonuniform_rgba_png_round_trips_every_pixel() {
    let (width, height) = (3840, 2160);
    let mut pixels = PixelStore::new(width, height, Color::TRANSPARENT);
    for y in 0..height {
        for x in 0..width {
            pixels
                .set(
                    x,
                    y,
                    Color::rgba(
                        (x * 17 + y * 11) as u8,
                        (x * 7 + y * 23) as u8,
                        ((x >> 4) ^ (y >> 3)) as u8,
                        if (x + y).is_multiple_of(97) { 0 } else { 255 },
                    ),
                )
                .unwrap();
        }
    }
    let document = Document::from_pixels(width, height, 180.125, pixels).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("4k-rgba.png");
    files::save_atomic(&path, &document, FileFormat::Png, 95).unwrap();
    let loaded = files::load(&path).unwrap();
    assert_eq!(loaded.snapshot(), document.snapshot());
    assert_eq!(loaded.resolution(), 180.125);
}
