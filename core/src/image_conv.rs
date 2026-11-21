//! Image conversions, backed by the pure-Rust `image` crate.

use std::io::Cursor;

use image::codecs::jpeg::JpegEncoder;
use image::{DynamicImage, ImageFormat};

use crate::engine::ConvertError;
use crate::model;

pub fn is_input(ext: &str) -> bool {
    let e = model::canonical(ext);
    model::IMAGE_INPUT.contains(&e)
}

pub fn is_output(ext: &str) -> bool {
    let e = model::canonical(ext);
    model::IMAGE_OUTPUT.contains(&e)
}

pub fn convert(
    data: &[u8],
    from: &str,
    to: &str,
    quality: u8,
) -> Result<Vec<u8>, ConvertError> {
    let in_fmt = ImageFormat::from_extension(from)
        .ok_or_else(|| ConvertError::InvalidData("image".into(), format!("no decoder for .{from}")))?;
    let out_fmt = ImageFormat::from_extension(to)
        .ok_or_else(|| ConvertError::InvalidData("image".into(), format!("no encoder for .{to}")))?;

    let image: DynamicImage = image::load_from_memory_with_format(data, in_fmt)?;

    let mut cursor = Cursor::new(Vec::new());
    if out_fmt == ImageFormat::Jpeg {
        let encoder = JpegEncoder::new_with_quality(&mut cursor, quality.clamp(1, 100));
        image.write_with_encoder(encoder)?;
    } else {
        image.write_to(&mut cursor, out_fmt)?;
    }

    Ok(cursor.into_inner())
}
