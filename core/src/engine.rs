//! The public conversion API used by both the GUI and the CLI.

use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::{audio_conv, doc_conv, image_conv, model, text_conv};

#[derive(Debug, Error)]
pub enum ConvertError {
    #[error("unsupported conversion: {0} -> {1}")]
    Unsupported(String, String),
    #[error("file error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid {0} data: {1}")]
    InvalidData(String, String),
    #[error("image error: {0}")]
    Image(#[from] image::ImageError),
    #[error("audio error: {0}")]
    Audio(String),
    #[error("{0}")]
    Other(String),
}

/// Options shared by every conversion backend.
#[derive(Debug, Clone)]
pub struct ConvertOptions {
    /// JPEG/WebP quality, 1-100. Ignored by lossless formats.
    pub quality: u8,
    /// Override source detection (useful when a file has no extension).
    pub from_ext: Option<String>,
}

impl Default for ConvertOptions {
    fn default() -> Self {
        Self { quality: 85, from_ext: None }
    }
}

/// Result of one successful conversion.
#[derive(Debug, Clone)]
pub struct ConvertOutcome {
    pub output: PathBuf,
    pub bytes: u64,
    /// Optional human-readable note, e.g. about a lossy step.
    pub note: Option<String>,
}

/// Convert raw bytes from one format to another, entirely in memory.
pub fn convert_bytes(
    input: &[u8],
    from: &str,
    to: &str,
    opts: &ConvertOptions,
) -> Result<Vec<u8>, ConvertError> {
    let from = model::canonical(from);
    let to = model::canonical(to);

    if from == to {
        return Ok(input.to_vec());
    }

    if image_conv::is_input(from) && image_conv::is_output(to) {
        return image_conv::convert(input, from, to, opts.quality);
    }

    if text_conv::is_structured(from) && text_conv::is_structured(to) {
        return text_conv::convert_structured(input, from, to);
    }

    if text_conv::is_text(from) && text_conv::is_text(to) {
        return text_conv::convert_text(input, from, to);
    }

    if to == "pdf" && (text_conv::is_text(from) || text_conv::is_structured(from)) {
        let text = if text_conv::is_structured(from) {
            let json = text_conv::convert_structured(input, from, "json")?;
            String::from_utf8_lossy(&json).into_owned()
        } else {
            String::from_utf8_lossy(input).into_owned()
        };
        return doc_conv::text_to_pdf(&text);
    }

    if from == "pdf" && to == "txt" {
        return doc_conv::pdf_to_text(input);
    }

    if to == "pdf" && image_conv::is_input(from) {
        let jpeg = image_conv::convert(input, from, "jpg", opts.quality)?;
        return doc_conv::jpeg_to_pdf(&jpeg);
    }

    if audio_conv::is_input(from) && audio_conv::is_output(to) {
        return audio_conv::to_wav(input, from).map_err(ConvertError::Audio);
    }

    Err(ConvertError::Unsupported(from.to_string(), to.to_string()))
}

/// Convert one file on disk to `to` format at `output`.
pub fn convert_file(
    input: &Path,
    output: &Path,
    to: &str,
    opts: &ConvertOptions,
) -> Result<ConvertOutcome, ConvertError> {
    let from = match &opts.from_ext {
        Some(e) => model::canonical(e).to_string(),
        None => model::extension_of(input)
            .ok_or_else(|| ConvertError::Other(format!("cannot determine format of {}", input.display())))?,
    };

    let bytes = std::fs::read(input)?;
    let converted = convert_bytes(&bytes, &from, to, opts)?;

    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(output, &converted)?;

    Ok(ConvertOutcome {
        output: output.to_path_buf(),
        bytes: converted.len() as u64,
        note: note_for(&from, to),
    })
}

/// Batch conversion. Each item is converted independently so one bad file
/// never aborts the rest.
pub fn convert_many<F>(
    files: &[PathBuf],
    to: &str,
    out_dir: Option<&Path>,
    opts: &ConvertOptions,
    mut progress: F,
) -> Vec<(PathBuf, Result<ConvertOutcome, ConvertError>)>
where
    F: FnMut(usize, usize, &Path),
{
    let total = files.len();
    files
        .iter()
        .enumerate()
        .map(|(i, input)| {
            progress(i + 1, total, input);
            let out_name = input.with_extension(to);
            let output = match out_dir {
                Some(dir) => dir.join(out_name.file_name().unwrap_or_default()),
                None => out_name,
            };
            let result = convert_file(input, &output, to, opts);
            (input.clone(), result)
        })
        .collect()
}

fn note_for(from: &str, to: &str) -> Option<String> {
    if (from == "gif" || from == "webp") && image_conv::is_output(to) {
        Some("Animated frames are flattened to the first frame.".to_string())
    } else if to == "wav" && from != "wav" {
        Some("Audio decoded to 16-bit PCM WAV.".to_string())
    } else if to == "pdf" && text_conv::is_text(from) {
        Some("PDF contains selectable text (basic fonts only).".to_string())
    } else {
        None
    }
}
