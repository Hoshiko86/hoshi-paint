//! Streaming desktop codecs and atomic replacement; no encoded-file byte ceiling.
use hoshi_core::{
    Color, CompositeSampler, Document, MAX_DIMENSION, MemoryReservation, PixelStore, memory_budget,
    memory_usage,
};
use image::ImageDecoder;
use std::{
    fs::{self, File},
    io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};
const MIB: u64 = 1024 * 1024;
/// File-operation failures. `Display` is developer text for logs; user copy
/// comes from the locale catalogs (`crate::error_text`).
#[derive(Debug, thiserror::Error)]
pub enum FileError {
    /// The operating system rejected a read, write, rename or metadata call.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    /// The `image` crate failed to detect, decode or encode a raster file.
    #[error("image codec: {0}")]
    Image(#[from] image::ImageError),
    /// The PNG decoder rejected the stream.
    #[error("png decode: {0}")]
    PngDecode(#[from] png::DecodingError),
    /// The PNG encoder failed while writing.
    #[error("png encode: {0}")]
    PngEncode(#[from] png::EncodingError),
    /// A buffer reservation would exceed the shared image memory budget.
    #[error("memory budget exceeded")]
    MemoryLimit,
    /// Decoded width or height is outside `1..=MAX_DIMENSION`.
    #[error("image dimensions out of range")]
    InvalidDimensions,
    /// Reading a `.hoshi` project failed.
    #[error("invalid project: {0}")]
    InvalidProject(#[source] hoshi_core::ProjectError),
    /// Writing a `.hoshi` project failed.
    #[error("project write: {0}")]
    Project(#[from] hoshi_core::ProjectError),
    /// Decoded raster pixels were rejected by the core document.
    #[error("invalid image: {0:?}")]
    InvalidImage(#[source] hoshi_core::Error),
    /// Decoded PNG pixels were rejected by the core document.
    #[error("invalid png: {0:?}")]
    InvalidPng(#[source] hoshi_core::Error),
    /// The save target has a `.pdn` extension.
    #[error("pdn save unsupported")]
    PdnUnsupported,
    /// The save target's extension is not .hoshi, .png or .jpg.
    #[error("unsupported save extension")]
    UnsupportedFormat,
    /// The path to open or save has no existing parent folder.
    #[error("missing parent folder")]
    MissingParent,
    /// The path to save has no file name.
    #[error("missing file name")]
    MissingFilename,
    /// The path to open is a directory or another non-regular file.
    #[error("not a regular file")]
    NotARegularFile,
    /// The file to open has a `.pdn` extension.
    #[error("pdn open unsupported")]
    PdnImportUnsupported,
    /// The raster decoder reports more than 8 bits per channel.
    #[error("unsupported channel depth")]
    UnsupportedChannelDepth,
    /// A raster file (not PNG) embeds an ICC profile.
    #[error("embedded icc profile")]
    UnsupportedIccProfile,
    /// A PNG embeds an ICC profile.
    #[error("embedded png icc profile")]
    UnsupportedPngIccProfile,
    /// A PNG uses 16-bit channels.
    #[error("16-bit png")]
    Unsupported16Bit,
    /// The expanded PNG output is not 8-bit.
    #[error("unsupported png bit depth")]
    UnsupportedPngDepth,
    /// The PNG ended before its last row.
    #[error("png rows truncated")]
    TruncatedPngRows,
    /// A PNG row's length does not match the header width.
    #[error("png row length mismatch")]
    InvalidPngRowDimensions,
    /// The PNG produced no pixel data.
    #[error("png has no pixels")]
    MissingPngPixels,
    /// The PNG color type is not RGB(A) or grayscale(+alpha).
    #[error("unsupported png color type")]
    UnsupportedPngColorType,
    /// Save was requested while a shape is still editable.
    #[error("active shape")]
    ActiveShape,
    /// JPEG cannot store the document's resolution.
    #[error("jpeg density too high")]
    JpegDensityTooHigh,
    /// JPEG cannot store the document's width or height.
    #[error("jpeg dimensions too large")]
    JpegDimensionsTooLarge,
    /// JPEG save was attempted before flattening transparency.
    #[error("jpeg transparency not flattened")]
    JpegTransparency,
    /// The save path has no parent folder.
    #[error("missing save folder")]
    MissingSaveParent,
    /// The existing destination file is read-only.
    #[error("read-only destination")]
    ReadOnlyDestination,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileFormat {
    Project,
    Png,
    Jpeg,
}
impl FileFormat {
    pub fn for_path(path: &Path) -> Result<Self, FileError> {
        match path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str()
        {
            "hoshi" => Ok(Self::Project),
            "png" => Ok(Self::Png),
            "jpg" | "jpeg" | "jpe" | "jfif" => Ok(Self::Jpeg),
            "pdn" => Err(FileError::PdnUnsupported),
            _ => Err(FileError::UnsupportedFormat),
        }
    }
}
pub fn absolute_path(path: &Path) -> Result<PathBuf, FileError> {
    if path.exists() {
        return Ok(path.canonicalize()?);
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let parent = absolute
        .parent()
        .ok_or(FileError::MissingParent)?
        .canonicalize()?;
    Ok(parent.join(absolute.file_name().ok_or(FileError::MissingFilename)?))
}
fn reserve(bytes: u64) -> Result<MemoryReservation, FileError> {
    MemoryReservation::new(bytes).map_err(|_| FileError::MemoryLimit)
}
fn dimensions(width: u32, height: u32) -> Result<(), FileError> {
    Document::validate_image_size(width, height).map_err(|_| FileError::InvalidDimensions)
}
// Snapshot the regular file's length so codecs that internally read_to_end
// cannot outgrow their reservation if another process appends while importing.
struct InputFile {
    file: File,
    length: u64,
    position: u64,
}
impl Read for InputFile {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        let count = self
            .length
            .saturating_sub(self.position)
            .min(output.len() as u64) as usize;
        let read = self.file.read(&mut output[..count])?;
        self.position += read as u64;
        Ok(read)
    }
}
impl Seek for InputFile {
    fn seek(&mut self, from: SeekFrom) -> std::io::Result<u64> {
        let target = match from {
            SeekFrom::Start(n) => i128::from(n),
            SeekFrom::Current(n) => i128::from(self.position) + i128::from(n),
            SeekFrom::End(n) => i128::from(self.length) + i128::from(n),
        };
        let target = u64::try_from(target).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid image file offset",
            )
        })?;
        self.position = self.file.seek(SeekFrom::Start(target))?;
        Ok(self.position)
    }
}
pub fn load(path: &Path) -> Result<Document, FileError> {
    let file = File::open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(FileError::NotARegularFile);
    }
    let mut file = InputFile {
        file,
        length: metadata.len(),
        position: 0,
    };
    let mut prefix = [0; 65536];
    let count = file.read(&mut prefix)?;
    file.seek(SeekFrom::Start(0))?;
    let prefix = &prefix[..count];
    if prefix.starts_with(b"HOSHIPNT")
        || path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("hoshi"))
    {
        return Document::read_project(BufReader::new(file)).map_err(FileError::InvalidProject);
    }
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("pdn"))
    {
        return Err(FileError::PdnImportUnsupported);
    }
    let format = image::guess_format(prefix).or_else(|_| image::ImageFormat::from_path(path))?;
    if format == image::ImageFormat::Png {
        return decode_png(BufReader::new(file));
    }
    let resolution = jpeg_density(prefix).unwrap_or(96.);
    // image's JPEG decoder and ICO's embedded-image path retain encoded input.
    // Charge that format-specific working set rather than imposing a disk-size cap.
    let encoded_bytes = if matches!(format, image::ImageFormat::Jpeg | image::ImageFormat::Ico) {
        file.length.checked_mul(3).ok_or(FileError::MemoryLimit)?
    } else {
        0
    };
    let _encoded = reserve(encoded_bytes)?;
    let header_bytes = (memory_budget().saturating_sub(memory_usage()) / 4).min(32 * MIB);
    let _header = reserve(header_bytes)?;
    let mut reader = image::ImageReader::with_format(BufReader::new(file), format);
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(header_bytes);
    limits.max_image_width = Some(MAX_DIMENSION);
    limits.max_image_height = Some(MAX_DIMENSION);
    reader.limits(limits);
    let mut decoder = reader.into_decoder()?;
    let (width, height) = decoder.dimensions();
    dimensions(width, height)?;
    if !matches!(
        decoder.color_type(),
        image::ColorType::L8
            | image::ColorType::La8
            | image::ColorType::Rgb8
            | image::ColorType::Rgba8
    ) {
        return Err(FileError::UnsupportedChannelDepth);
    }
    let scratch_bytes = u64::from(width) * u64::from(height) * 8;
    let _scratch = reserve(scratch_bytes)?;
    let mut frame_limits = image::Limits::default();
    frame_limits.max_alloc = Some(scratch_bytes + header_bytes);
    frame_limits.max_image_width = Some(MAX_DIMENSION);
    frame_limits.max_image_height = Some(MAX_DIMENSION);
    decoder.set_limits(frame_limits)?;
    if decoder.icc_profile()?.is_some_and(|p| !p.is_empty()) {
        return Err(FileError::UnsupportedIccProfile);
    }
    let orientation = decoder.orientation()?;
    let mut picture = image::DynamicImage::from_decoder(decoder)?;
    // The consumed decoder has released its encoded bytes and metadata now.
    drop(_encoded);
    drop(_header);
    picture.apply_orientation(orientation);
    let rgba = picture.into_rgba8();
    let (width, height) = rgba.dimensions();
    let mut background = Color::TRANSPARENT;
    let mut balance = 0;
    for p in rgba.pixels() {
        background_vote(
            &mut background,
            &mut balance,
            Color::rgba(p[0], p[1], p[2], p[3]),
        );
    }
    let mut pixels = PixelStore::new(width, height, background);
    for (x, y, p) in rgba.enumerate_pixels() {
        pixels
            .set(x, y, Color::rgba(p[0], p[1], p[2], p[3]))
            .map_err(|_| FileError::MemoryLimit)?;
    }
    Document::from_pixels(width, height, resolution, pixels).map_err(FileError::InvalidImage)
}
fn decode_png(input: BufReader<InputFile>) -> Result<Document, FileError> {
    // png's metadata and decoder workspace are bounded independently of the
    // encoded file's length. Pixel rows are moved directly into sparse storage.
    let decoder_bytes = (memory_budget().saturating_sub(memory_usage()) / 2).min(32 * MIB);
    let _decoder_memory = reserve(decoder_bytes)?;
    let mut decoder = png::Decoder::new(input);
    decoder.set_limits(png::Limits {
        bytes: usize::try_from(decoder_bytes).map_err(|_| FileError::MemoryLimit)?,
    });
    decoder.set_transformations(png::Transformations::EXPAND);
    let mut reader = decoder.read_info()?;
    let info = reader.info();
    if info.icc_profile.as_ref().is_some_and(|p| !p.is_empty()) {
        return Err(FileError::UnsupportedPngIccProfile);
    }
    let (width, height) = info
        .frame_control
        .map_or((info.width, info.height), |f| (f.width, f.height));
    dimensions(width, height)?;
    if info.bit_depth == png::BitDepth::Sixteen {
        return Err(FileError::Unsupported16Bit);
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
        resolution = exact;
    }
    if Document::validate_resolution(resolution).is_err() {
        resolution = 96.;
    }
    let interlaced = info.interlaced;
    let (color_type, depth) = reader.output_color_type();
    if depth != png::BitDepth::Eight {
        return Err(FileError::UnsupportedPngDepth);
    }
    let channels = color_type.samples();
    let _rows = reserve(u64::from(width) * 4 * 8 + MIB)?;
    let passes: &[(u32, u32, u32, u32)] = if interlaced {
        &[
            (0, 0, 8, 8),
            (4, 0, 8, 8),
            (0, 4, 4, 8),
            (2, 0, 4, 4),
            (0, 2, 2, 4),
            (1, 0, 2, 2),
            (0, 1, 1, 2),
        ]
    } else {
        &[(0, 0, 1, 1)]
    };
    let mut pixels = None;
    let mut pending: Vec<(u32, u32, u32, Vec<u8>)> = Vec::with_capacity(4);
    let mut background = Color::TRANSPARENT;
    let mut balance = 0;
    for &(x0, y0, dx, dy) in passes {
        if x0 >= width || y0 >= height {
            continue;
        }
        for y in (y0..height).step_by(dy as usize) {
            let row = reader.next_row()?.ok_or(FileError::TruncatedPngRows)?;
            let expected = (width - x0).div_ceil(dx) as usize * channels;
            if row.data().len() != expected {
                return Err(FileError::InvalidPngRowDimensions);
            }
            if let Some(pixels) = pixels.as_mut() {
                store_png_row(pixels, row.data(), x0, y, dx, color_type)?;
            } else {
                for p in row.data().chunks_exact(channels) {
                    background_vote(&mut background, &mut balance, png_color(color_type, p)?);
                }
                let mut buffered = Vec::new();
                buffered
                    .try_reserve_exact(row.data().len())
                    .map_err(|_| FileError::MemoryLimit)?;
                buffered.extend_from_slice(row.data());
                pending.push((x0, y, dx, buffered));
                if pending.len() == 4 {
                    let mut initialized = PixelStore::new(width, height, background);
                    for (x0, y, dx, row) in pending.drain(..) {
                        store_png_row(&mut initialized, &row, x0, y, dx, color_type)?;
                    }
                    pixels = Some(initialized);
                }
            }
        }
    }
    if pixels.is_none() && !pending.is_empty() {
        let mut initialized = PixelStore::new(width, height, background);
        for (x0, y, dx, row) in pending.drain(..) {
            store_png_row(&mut initialized, &row, x0, y, dx, color_type)?;
        }
        pixels = Some(initialized);
    }
    reader.finish()?;
    Document::from_pixels(
        width,
        height,
        resolution,
        pixels.ok_or(FileError::MissingPngPixels)?,
    )
    .map_err(FileError::InvalidPng)
}
fn background_vote(candidate: &mut Color, balance: &mut usize, color: Color) {
    if *balance == 0 {
        *candidate = color;
        *balance = 1;
    } else if *candidate == color {
        *balance += 1;
    } else {
        *balance -= 1;
        if *balance == 0 {
            *candidate = color;
        }
    }
}
fn png_color(kind: png::ColorType, p: &[u8]) -> Result<Color, FileError> {
    Ok(match kind {
        png::ColorType::Rgba => Color::rgba(p[0], p[1], p[2], p[3]),
        png::ColorType::Rgb => Color::rgba(p[0], p[1], p[2], 255),
        png::ColorType::GrayscaleAlpha => Color::rgba(p[0], p[0], p[0], p[1]),
        png::ColorType::Grayscale => Color::rgba(p[0], p[0], p[0], 255),
        _ => return Err(FileError::UnsupportedPngColorType),
    })
}
fn store_png_row(
    pixels: &mut PixelStore,
    row: &[u8],
    x0: u32,
    y: u32,
    dx: u32,
    kind: png::ColorType,
) -> Result<(), FileError> {
    for (index, p) in row.chunks_exact(kind.samples()).enumerate() {
        pixels
            .set(x0 + index as u32 * dx, y, png_color(kind, p)?)
            .map_err(|_| FileError::MemoryLimit)?;
    }
    Ok(())
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
struct JpegView<'a> {
    sampler: CompositeSampler<'a>,
    width: u32,
    height: u32,
}
impl image::GenericImageView for JpegView<'_> {
    type Pixel = image::Rgb<u8>;
    fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
    fn get_pixel(&self, x: u32, y: u32) -> Self::Pixel {
        let c = self.sampler.pixel(x, y);
        image::Rgb([c.r, c.g, c.b])
    }
}
pub fn save_atomic(
    path: &Path,
    document: &Document,
    format: FileFormat,
    jpeg_quality: u8,
) -> Result<(), FileError> {
    if document.active_shape().is_some() {
        return Err(FileError::ActiveShape);
    }
    if format == FileFormat::Jpeg {
        if document.resolution().round() > 65535. {
            return Err(FileError::JpegDensityTooHigh);
        }
        if document.width() > 65535 || document.height() > 65535 {
            return Err(FileError::JpegDimensionsTooLarge);
        }
        if document.has_transparency() {
            return Err(FileError::JpegTransparency);
        }
    }
    let parent = path.parent().ok_or(FileError::MissingSaveParent)?;
    let permissions = match fs::metadata(path) {
        Ok(meta) => {
            if meta.permissions().readonly() {
                return Err(FileError::ReadOnlyDestination);
            }
            Some(meta.permissions())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e.into()),
    };
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    {
        let mut output = BufWriter::new(temporary.as_file_mut());
        match format {
            FileFormat::Project => document.write_project(&mut output)?,
            FileFormat::Png => {
                let _scratch = reserve(u64::from(document.width()) * 4 * 8 + MIB)?;
                let mut encoder =
                    png::Encoder::new(&mut output, document.width(), document.height());
                encoder.set_color(png::ColorType::Rgba);
                encoder.set_depth(png::BitDepth::Eight);
                let density = (document.resolution() / 0.0254).round() as u32;
                encoder.set_pixel_dims(Some(png::PixelDimensions {
                    xppu: density,
                    yppu: density,
                    unit: png::Unit::Meter,
                }));
                encoder.add_text_chunk(
                    "HoshiPaint.Resolution".into(),
                    document.resolution().to_string(),
                )?;
                let mut writer = encoder.write_header()?;
                let mut row = Vec::new();
                row.try_reserve_exact(document.width() as usize * 4)
                    .map_err(|_| FileError::MemoryLimit)?;
                row.resize(document.width() as usize * 4, 0);
                let direct = document.layers().len() == 1
                    && document.layers()[0].visible()
                    && document.layers()[0].opacity() == 255;
                let sampler = document.sampler(false);
                {
                    let mut stream = writer.stream_writer()?;
                    for y in 0..document.height() {
                        for x in 0..document.width() {
                            let c = if direct {
                                document.layers()[0].pixels().get(x, y)
                            } else {
                                sampler.pixel(x, y)
                            };
                            let i = x as usize * 4;
                            row[i..i + 4].copy_from_slice(&[c.r, c.g, c.b, c.a]);
                        }
                        stream.write_all(&row)?;
                    }
                    stream.finish()?;
                }
                writer.finish()?;
            }
            FileFormat::Jpeg => {
                let _scratch = reserve(4 * MIB)?;
                let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
                    &mut output,
                    jpeg_quality.clamp(1, 100),
                );
                encoder.set_pixel_density(image::codecs::jpeg::PixelDensity::dpi(
                    document.resolution().round() as u16,
                ));
                encoder.encode_image(&JpegView {
                    sampler: document.sampler(false),
                    width: document.width(),
                    height: document.height(),
                })?;
            }
        }
        output.flush()?;
    }
    temporary.flush()?;
    if let Some(p) = permissions {
        temporary.as_file().set_permissions(p)?;
    }
    temporary.as_file().sync_all()?;
    temporary.persist(path).map_err(|e| e.error)?;
    Ok(())
}
