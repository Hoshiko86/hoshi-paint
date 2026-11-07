//! Desktop codecs and atomic replacement; the document core stays filesystem-free.
use hoshi_core::{Color, Document, MAX_PROJECT_BYTES};
use image::ImageDecoder;
use std::{
    fs::{self, File},
    io::{Cursor, Read, Write},
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileFormat {
    Project,
    Png,
    Jpeg,
}
impl FileFormat {
    pub fn for_path(path: &Path) -> Result<Self, String> {
        match path.extension().and_then(|s|s.to_str()).unwrap_or("").to_ascii_lowercase().as_str(){
            "hoshi"=>Ok(Self::Project),"png"=>Ok(Self::Png),"jpg"|"jpeg"|"jpe"|"jfif"=>Ok(Self::Jpeg),
            "pdn"=>Err("Paint.NET .pdn compatibility is not implemented. Use a .hoshi project or a raster image.".into()),
            _=>Err("Choose a .hoshi, .png, or .jpg filename for saving.".into()),
        }
    }
}
pub fn absolute_path(path: &Path) -> Result<PathBuf, String> {
    if path.exists() {
        return path.canonicalize().map_err(|e| e.to_string());
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(path)
    };
    let parent = absolute
        .parent()
        .ok_or("Choose a file in an existing folder.")?
        .canonicalize()
        .map_err(|e| e.to_string())?;
    Ok(parent.join(absolute.file_name().ok_or("Choose a filename.")?))
}
pub fn load(path: &Path) -> Result<Document, String> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    if file.metadata().map_err(|e| e.to_string())?.len() > MAX_PROJECT_BYTES as u64 {
        return Err("File exceeds the prototype's bounded input size.".into());
    }
    let mut bytes = Vec::new();
    file.take(MAX_PROJECT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > MAX_PROJECT_BYTES {
        return Err("File is too large.".into());
    }
    if bytes.starts_with(b"HOSHIPNT")
        || path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("hoshi"))
    {
        return Document::decode_project(&bytes).map_err(|e| format!("Invalid Hoshi project: {e}"));
    }
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("pdn"))
    {
        return Err("Paint.NET .pdn files are not supported yet. Export PNG or another supported raster format first.".into());
    }
    if image::guess_format(&bytes).ok() == Some(image::ImageFormat::Png) {
        return decode_png(&bytes);
    }
    let resolution = jpeg_density(&bytes).unwrap_or(96.);
    let format = image::guess_format(&bytes)
        .or_else(|_| image::ImageFormat::from_path(path))
        .map_err(|e| e.to_string())?;
    let mut reader = image::ImageReader::with_format(Cursor::new(&bytes), format);
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(64 * 1024 * 1024);
    limits.max_image_width = Some(1_048_576);
    limits.max_image_height = Some(1_048_576);
    reader.limits(limits);
    let mut decoder = reader.into_decoder().map_err(|e| e.to_string())?;
    let (w, h) = decoder.dimensions();
    Document::validate_image_size(w, h)
        .map_err(|_| "Image exceeds the 1,048,576-pixel limit.".to_string())?;
    if !matches!(
        decoder.color_type(),
        image::ColorType::L8
            | image::ColorType::La8
            | image::ColorType::Rgb8
            | image::ColorType::Rgba8
    ) {
        return Err("This prototype supports 8-bit image channels; higher-depth images require conversion before opening.".into());
    }
    if decoder
        .icc_profile()
        .map_err(|e| e.to_string())?
        .is_some_and(|p| !p.is_empty())
    {
        return Err("Embedded ICC profiles are not supported yet. Convert to untagged sRGB before opening; the original file has not been changed.".into());
    }
    let orientation = decoder.orientation().map_err(|e| e.to_string())?;
    let mut picture = image::DynamicImage::from_decoder(decoder).map_err(|e| e.to_string())?;
    picture.apply_orientation(orientation);
    let rgba = picture.into_rgba8();
    Document::from_rgba(
        rgba.width(),
        rgba.height(),
        resolution,
        rgba.pixels()
            .map(|c| Color::rgba(c[0], c[1], c[2], c[3]))
            .collect(),
    )
    .map_err(|e| format!("Invalid image: {e:?}"))
}
fn decode_png(bytes: &[u8]) -> Result<Document, String> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_limits(png::Limits {
        bytes: 64 * 1024 * 1024,
    });
    decoder.set_transformations(png::Transformations::EXPAND);
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let info = reader.info();
    if info.icc_profile.as_ref().is_some_and(|p| !p.is_empty()) {
        return Err("Embedded ICC profiles are not supported yet. Convert to untagged sRGB first; the original file is unchanged.".into());
    }
    Document::validate_image_size(info.width, info.height)
        .map_err(|_| "Image exceeds the 1,048,576-pixel limit.".to_string())?;
    if info.bit_depth == png::BitDepth::Sixteen {
        return Err("16-bit PNG channels are not supported yet; convert to 8-bit first.".into());
    }
    let mut resolution = info
        .pixel_dims
        .filter(|p| p.unit == png::Unit::Meter && p.xppu > 0)
        .map(|p| f64::from(p.xppu) * 0.0254)
        .unwrap_or(96.);
    if let Some(exact) = info
        .uncompressed_latin1_text
        .iter()
        .find(|t| t.keyword == "HoshiPaint.Resolution")
        .and_then(|t| t.text.parse::<f64>().ok())
        .filter(|r| Document::validate_resolution(*r).is_ok())
    {
        resolution = exact
    }
    if Document::validate_resolution(resolution).is_err() {
        resolution = 96.
    }
    let size = reader
        .output_buffer_size()
        .ok_or("PNG output is too large.")?;
    if size > 4 * 1_048_576 {
        return Err("PNG output is too large.".into());
    }
    let mut output = vec![0; size];
    let frame = reader.next_frame(&mut output).map_err(|e| e.to_string())?;
    let pixels = match frame.color_type {
        png::ColorType::Rgba => output[..frame.buffer_size()]
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| Color::rgba(p[0], p[1], p[2], p[3]))
            .collect(),
        png::ColorType::Rgb => output[..frame.buffer_size()]
            .as_chunks::<3>()
            .0
            .iter()
            .map(|p| Color::rgba(p[0], p[1], p[2], 255))
            .collect(),
        png::ColorType::GrayscaleAlpha => output[..frame.buffer_size()]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|p| Color::rgba(p[0], p[0], p[0], p[1]))
            .collect(),
        png::ColorType::Grayscale => output[..frame.buffer_size()]
            .iter()
            .map(|v| Color::rgba(*v, *v, *v, 255))
            .collect(),
        _ => return Err("Unsupported PNG color type.".into()),
    };
    Document::from_rgba(frame.width, frame.height, resolution, pixels)
        .map_err(|e| format!("Invalid PNG: {e:?}"))
}
fn jpeg_density(bytes: &[u8]) -> Option<f64> {
    if !bytes.starts_with(&[255, 216]) {
        return None;
    }
    let mut i = 2;
    while i + 4 <= bytes.len() {
        if bytes[i] != 255 {
            return None;
        }
        let marker = bytes[i + 1];
        i += 2;
        if matches!(marker, 0xda | 0xd9) {
            break;
        }
        let length = u16::from_be_bytes([*bytes.get(i)?, *bytes.get(i + 1)?]) as usize;
        if length < 2 {
            return None;
        }
        let data = bytes.get(i + 2..i + length)?;
        if marker == 0xe0 && data.starts_with(b"JFIF\0") && data.len() >= 12 {
            let value = f64::from(u16::from_be_bytes([data[8], data[9]]));
            let value = match data[7] {
                1 => value,
                2 => value * 2.54,
                _ => return None,
            };
            return Document::validate_resolution(value).ok().map(|_| value);
        }
        i += length;
    }
    None
}
pub fn save_atomic(
    path: &Path,
    document: &Document,
    format: FileFormat,
    jpeg_quality: u8,
) -> Result<(), String> {
    if document.active_shape().is_some() {
        return Err("Commit the active shape before saving.".into());
    }
    if format == FileFormat::Jpeg && (document.resolution().round() > 65535.) {
        return Err("JPEG density cannot exceed 65535 pixels/inch; use PNG or .hoshi.".into());
    }
    let parent = path.parent().ok_or("Choose an existing folder.")?;
    let permissions = match fs::metadata(path) {
        Ok(meta) => {
            if meta.permissions().readonly() {
                return Err(
                    "The destination file is read-only. Use Save As to choose another file.".into(),
                );
            }
            Some(meta.permissions())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e.to_string()),
    };
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    match format {
        FileFormat::Project => temporary
            .write_all(&document.encode_project().map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?,
        FileFormat::Png => {
            let pixels = if document.layers().len() == 1
                && document.layers()[0].visible()
                && document.layers()[0].opacity() == 255
            {
                document.layers()[0].pixels().to_vec()
            } else {
                document.composite(false)
            };
            let raw: Vec<_> = pixels.iter().flat_map(|p| [p.r, p.g, p.b, p.a]).collect();
            let mut encoder =
                png::Encoder::new(temporary.as_file_mut(), document.width(), document.height());
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let density = (document.resolution() / 0.0254).round() as u32;
            encoder.set_pixel_dims(Some(png::PixelDimensions {
                xppu: density,
                yppu: density,
                unit: png::Unit::Meter,
            }));
            encoder
                .add_text_chunk(
                    "HoshiPaint.Resolution".into(),
                    document.resolution().to_string(),
                )
                .map_err(|e| e.to_string())?;
            let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
            writer.write_image_data(&raw).map_err(|e| e.to_string())?;
            writer.finish().map_err(|e| e.to_string())?;
        }
        FileFormat::Jpeg => {
            let pixels = document.composite(false);
            if pixels.iter().any(|c| c.a < 255) {
                return Err("JPEG transparency must be flattened after confirmation.".into());
            }
            let raw: Vec<_> = pixels.iter().flat_map(|c| [c.r, c.g, c.b]).collect();
            let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
                temporary.as_file_mut(),
                jpeg_quality.clamp(1, 100),
            );
            encoder.set_pixel_density(image::codecs::jpeg::PixelDensity::dpi(
                document.resolution().round() as u16,
            ));
            encoder
                .encode(
                    &raw,
                    document.width(),
                    document.height(),
                    image::ExtendedColorType::Rgb8,
                )
                .map_err(|e| e.to_string())?;
        }
    }
    temporary.flush().map_err(|e| e.to_string())?;
    if let Some(p) = permissions {
        temporary
            .as_file()
            .set_permissions(p)
            .map_err(|e| e.to_string())?
    }
    temporary.as_file().sync_all().map_err(|e| e.to_string())?;
    temporary.persist(path).map_err(|e| e.error.to_string())?;
    Ok(())
}
