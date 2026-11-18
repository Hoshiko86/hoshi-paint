use crate::support::{layered, noise};
use hoshi_core::{Color, Document, MAX_DIMENSION};
use hoshi_desktop::{
    files::{self, FileError, FileFormat},
    workspace::Workspace,
};
use std::borrow::Cow;

fn encode(
    width: u32,
    height: u32,
    color: png::ColorType,
    depth: png::BitDepth,
    data: &[u8],
) -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, width, height);
        encoder.set_color(color);
        encoder.set_depth(depth);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(data).unwrap();
        writer.finish().unwrap();
    }
    bytes
}

fn chunks(bytes: &[u8]) -> Vec<(usize, u32, [u8; 4])> {
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    let mut result = Vec::new();
    let mut offset = 8;
    while offset < bytes.len() {
        let len = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap());
        let kind = bytes[offset + 4..offset + 8].try_into().unwrap();
        assert!(offset + 12 + len as usize <= bytes.len());
        result.push((offset, len, kind));
        offset += 12 + len as usize;
    }
    result
}

fn assert_pixels(
    document: &Document,
    expected: &[Color],
    width: u32,
    height: u32,
    resolution: f64,
) {
    assert_eq!(document.layers().len(), 1);
    assert_eq!((document.width(), document.height()), (width, height));
    assert_eq!(document.resolution(), resolution);
    for y in 0..height {
        for x in 0..width {
            assert_eq!(
                document.layers()[0].pixels().get(x, y),
                expected[(y * width + x) as usize],
                "x={x}, y={y}"
            );
        }
    }
}

fn load_bytes(bytes: &[u8]) -> Result<Document, FileError> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("input.png");
    std::fs::write(&path, bytes).unwrap();
    files::load(&path, "Background")
}

#[test]
fn layered_png_export_matches_the_composite_pixel_for_pixel() {
    let document = layered();
    let expected = document.composite(false);
    assert!(expected.iter().any(|c| c.a > 0 && c.a < 255));
    for y in 5..12 {
        for x in 4..12 {
            assert_ne!(expected[y * 64 + x], Color::rgba(255, 0, 0, 255));
        }
    }
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("direct.png");
    files::save_atomic(&out, &document, FileFormat::Png, 95).unwrap();
    assert_pixels(
        &files::load(&out, "Background").unwrap(),
        &expected,
        64,
        48,
        144.125,
    );
    let project = dir.path().join("layered.hoshi");
    files::save_atomic(&project, &layered(), FileFormat::Project, 95).unwrap();
    let mut workspace = Workspace::default();
    workspace.open_path(&project).unwrap();
    let undo_before = workspace.document().undo_len();
    let out = dir.path().join("workspace.png");
    workspace.save_to(&out, 95, true).unwrap();
    assert_pixels(
        &files::load(&out, "Background").unwrap(),
        &expected,
        64,
        48,
        144.125,
    );
    assert_eq!(workspace.document().undo_len(), undo_before + 1);
    workspace.undo().unwrap();
    assert_eq!(workspace.document().layers().len(), 3);
}

#[test]
fn png_beyond_max_dimension_is_rejected() {
    for (width, height) in [(MAX_DIMENSION + 1, 1), (1, MAX_DIMENSION + 1)] {
        let bytes = encode(
            width,
            height,
            png::ColorType::Rgba,
            png::BitDepth::Eight,
            &vec![0; width as usize * height as usize * 4],
        );
        assert!(matches!(
            load_bytes(&bytes),
            Err(FileError::InvalidDimensions)
        ));
    }
}

#[test]
fn huge_declared_png_with_a_tiny_body_fails_without_allocating() {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, MAX_DIMENSION, MAX_DIMENSION);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer
            .write_chunk(png::chunk::IDAT, &[0x78, 0x9c, 0x03, 0, 0, 0, 0, 1])
            .unwrap();
    }
    assert!(bytes.len() < 1024);
    assert!(matches!(
        load_bytes(&bytes),
        Err(FileError::PngDecode(_) | FileError::TruncatedPngRows)
    ));
}

fn truncated_idat(bytes: &[u8]) -> &[u8] {
    let (offset, len, _) = chunks(bytes)
        .into_iter()
        .find(|(_, _, kind)| kind == b"IDAT")
        .unwrap();
    &bytes[..offset + 8 + len as usize / 2]
}

#[test]
fn truncated_png_is_rejected() {
    let bytes = encode(
        64,
        64,
        png::ColorType::Rgba,
        png::BitDepth::Eight,
        &noise(64, 64),
    );
    let all_chunks = chunks(&bytes);
    assert_eq!(all_chunks.last().unwrap().2, *b"IEND");
    for (label, truncated) in [
        ("after IHDR", &bytes[..33]),
        ("half IDAT", truncated_idat(&bytes)),
        ("missing IEND policy", &bytes[..bytes.len() - 12]),
    ] {
        let result = load_bytes(truncated);
        assert!(
            matches!(
                result,
                Err(FileError::PngDecode(_) | FileError::TruncatedPngRows)
            ),
            "{label}: {:?}",
            result.as_ref().map(|_| "loaded successfully")
        );
    }
}

#[test]
fn png_with_a_bad_crc_is_rejected() {
    let bytes = encode(
        64,
        64,
        png::ColorType::Rgba,
        png::BitDepth::Eight,
        &noise(64, 64),
    );
    let idat: Vec<_> = chunks(&bytes)
        .into_iter()
        .filter(|(_, _, kind)| kind == b"IDAT")
        .collect();
    assert_eq!(idat.len(), 1);
    let (offset, len, _) = idat[0];
    // The last Adler-32 byte is ignored by png by default; only the chunk CRC rejects it.
    let idat_last_byte = offset + 8 + len as usize - 1;
    for index in [29, idat_last_byte] {
        let mut corrupted = bytes.clone();
        corrupted[index] ^= 1;
        assert!(matches!(
            load_bytes(&corrupted),
            Err(FileError::PngDecode(_))
        ));
        if index == idat_last_byte {
            let mut decoder = png::Decoder::new(std::io::Cursor::new(&corrupted));
            decoder.ignore_checksums(true);
            let mut reader = decoder.read_info().unwrap();
            let mut output = vec![0; reader.output_buffer_size().unwrap()];
            let info = reader.next_frame(&mut output).unwrap();
            assert_eq!(&output[..info.buffer_size()], noise(64, 64));
        }
    }
}

#[test]
fn sixteen_bit_pngs_are_rejected_in_every_color_type() {
    for color in [
        png::ColorType::Grayscale,
        png::ColorType::GrayscaleAlpha,
        png::ColorType::Rgb,
        png::ColorType::Rgba,
    ] {
        let bytes = encode(
            2,
            2,
            color,
            png::BitDepth::Sixteen,
            &vec![0; 4 * color.samples() * 2],
        );
        assert!(matches!(
            load_bytes(&bytes),
            Err(FileError::Unsupported16Bit)
        ));
    }
}

#[test]
fn icc_tagged_png_is_rejected_and_an_empty_profile_is_untagged() {
    let data = noise(2, 2);
    for profile in [vec![42; 132], Vec::new()] {
        let empty = profile.is_empty();
        let mut info = png::Info::with_size(2, 2);
        info.color_type = png::ColorType::Rgba;
        info.bit_depth = png::BitDepth::Eight;
        info.icc_profile = Some(Cow::Owned(profile));
        let mut bytes = Vec::new();
        {
            let mut writer = png::Encoder::with_info(&mut bytes, info)
                .unwrap()
                .write_header()
                .unwrap();
            writer.write_image_data(&data).unwrap();
            writer.finish().unwrap();
        }
        let result = load_bytes(&bytes);
        if empty {
            let expected: Vec<_> = data
                .as_chunks::<4>()
                .0
                .iter()
                .map(|p| Color::rgba(p[0], p[1], p[2], p[3]))
                .collect();
            assert_pixels(&result.unwrap(), &expected, 2, 2, 96.);
        } else {
            assert!(matches!(result, Err(FileError::UnsupportedPngIccProfile)));
        }
    }
}

#[test]
fn hostile_open_keeps_the_workspace() {
    let bytes = encode(
        64,
        64,
        png::ColorType::Rgba,
        png::BitDepth::Eight,
        &noise(64, 64),
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("truncated.png");
    std::fs::write(&path, truncated_idat(&bytes)).unwrap();
    let mut workspace = Workspace::default();
    let tabs = workspace.tabs().len();
    let active = workspace.active_id();
    assert!(workspace.open_path(&path).is_err());
    assert_eq!(workspace.tabs().len(), tabs);
    assert_eq!(workspace.active_id(), active);
}
