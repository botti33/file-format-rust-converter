//! Format registry and conversion capability tables.
//!
//! Everything in this module is static data: it describes what the engine can
//! do without touching any file. The GUI and CLI both read from here so the
//! capability list can never drift away from the actual converter.

use std::path::Path;

/// Broad family a format belongs to. Used to group formats in the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Image,
    Audio,
    Data,
    Text,
    Document,
}

impl Kind {
    pub const fn label(self) -> &'static str {
        match self {
            Kind::Image => "Images",
            Kind::Audio => "Audio",
            Kind::Data => "Structured data",
            Kind::Text => "Text / markup",
            Kind::Document => "Documents",
        }
    }
}

/// A file extension the application knows about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Format {
    pub ext: &'static str,
    pub label: &'static str,
    pub kind: Kind,
    /// Can this extension be read by the engine?
    pub input: bool,
    /// Can the engine write this extension?
    pub output: bool,
}

/// Image extensions the `image` crate can decode.
pub const IMAGE_INPUT: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "bmp", "tif", "tiff", "ico", "pnm", "pbm", "pgm",
    "ppm", "tga", "qoi", "hdr", "exr", "dds", "farbfeld",
];

/// Image extensions the `image` crate can reliably encode.
pub const IMAGE_OUTPUT: &[&str] = &[
    "png", "jpg", "gif", "webp", "bmp", "tiff", "ico", "pnm", "ppm", "tga", "qoi", "hdr", "exr",
];

/// Formats that round-trip through `serde_json::Value`.
pub const DATA_FORMATS: &[&str] = &["json", "yaml", "toml", "csv", "xml", "ron", "json5", "ini"];

/// Formats handled by the text / markup converter.
pub const TEXT_FORMATS: &[&str] = &["txt", "md", "html"];

/// Formats the audio decoder can read (all are written to PCM WAV).
pub const AUDIO_INPUT: &[&str] = &[
    "mp3", "wav", "flac", "ogg", "aac", "m4a", "opus", "aiff", "mp2", "mp1",
];

/// Document formats.
pub const DOC_INPUT: &[&str] = &["pdf"];
pub const DOC_OUTPUT: &[&str] = &["pdf"];

pub const FORMATS: &[Format] = &[
    // Images
    Format { ext: "png", label: "PNG", kind: Kind::Image, input: true, output: true },
    Format { ext: "jpg", label: "JPEG", kind: Kind::Image, input: true, output: true },
    Format { ext: "jpeg", label: "JPEG", kind: Kind::Image, input: true, output: true },
    Format { ext: "gif", label: "GIF", kind: Kind::Image, input: true, output: true },
    Format { ext: "webp", label: "WebP", kind: Kind::Image, input: true, output: true },
    Format { ext: "bmp", label: "BMP", kind: Kind::Image, input: true, output: true },
    Format { ext: "tif", label: "TIFF", kind: Kind::Image, input: true, output: true },
    Format { ext: "tiff", label: "TIFF", kind: Kind::Image, input: true, output: true },
    Format { ext: "ico", label: "ICON", kind: Kind::Image, input: true, output: true },
    Format { ext: "pnm", label: "PNM", kind: Kind::Image, input: true, output: true },
    Format { ext: "ppm", label: "Portable Pixmap", kind: Kind::Image, input: true, output: true },
    Format { ext: "pbm", label: "Portable Bitmap", kind: Kind::Image, input: true, output: false },
    Format { ext: "pgm", label: "Portable Graymap", kind: Kind::Image, input: true, output: false },
    Format { ext: "tga", label: "TGA", kind: Kind::Image, input: true, output: true },
    Format { ext: "qoi", label: "Quite OK Image", kind: Kind::Image, input: true, output: true },
    Format { ext: "hdr", label: "Radiance HDR", kind: Kind::Image, input: true, output: true },
    Format { ext: "exr", label: "OpenEXR", kind: Kind::Image, input: true, output: true },
    Format { ext: "dds", label: "DirectDraw Surface", kind: Kind::Image, input: true, output: false },
    Format { ext: "farbfeld", label: "farbfeld", kind: Kind::Image, input: true, output: false },
    // Structured data
    Format { ext: "json", label: "JSON", kind: Kind::Data, input: true, output: true },
    Format { ext: "yaml", label: "YAML", kind: Kind::Data, input: true, output: true },
    Format { ext: "yml", label: "YAML", kind: Kind::Data, input: true, output: true },
    Format { ext: "toml", label: "TOML", kind: Kind::Data, input: true, output: true },
    Format { ext: "csv", label: "CSV", kind: Kind::Data, input: true, output: true },
    Format { ext: "xml", label: "XML", kind: Kind::Data, input: true, output: true },
    Format { ext: "ron", label: "RON", kind: Kind::Data, input: true, output: true },
    Format { ext: "json5", label: "JSON5", kind: Kind::Data, input: true, output: true },
    Format { ext: "ini", label: "INI", kind: Kind::Data, input: true, output: true },
    // Text / markup
    Format { ext: "txt", label: "Plain text", kind: Kind::Text, input: true, output: true },
    Format { ext: "md", label: "Markdown", kind: Kind::Text, input: true, output: true },
    Format { ext: "html", label: "HTML", kind: Kind::Text, input: true, output: true },
    Format { ext: "htm", label: "HTML", kind: Kind::Text, input: true, output: true },
    // Audio
    Format { ext: "mp3", label: "MP3", kind: Kind::Audio, input: true, output: false },
    Format { ext: "mp2", label: "MP2", kind: Kind::Audio, input: true, output: false },
    Format { ext: "mp1", label: "MP1", kind: Kind::Audio, input: true, output: false },
    Format { ext: "wav", label: "WAV (PCM)", kind: Kind::Audio, input: true, output: true },
    Format { ext: "flac", label: "FLAC", kind: Kind::Audio, input: true, output: false },
    Format { ext: "ogg", label: "Ogg", kind: Kind::Audio, input: true, output: false },
    Format { ext: "opus", label: "Opus", kind: Kind::Audio, input: true, output: false },
    Format { ext: "aac", label: "AAC", kind: Kind::Audio, input: true, output: false },
    Format { ext: "m4a", label: "M4A", kind: Kind::Audio, input: true, output: false },
    Format { ext: "aiff", label: "AIFF", kind: Kind::Audio, input: true, output: false },
    Format { ext: "aif", label: "AIFF", kind: Kind::Audio, input: true, output: false },
    // Documents
    Format { ext: "pdf", label: "PDF", kind: Kind::Document, input: true, output: true },
];

/// Normalise extension aliases (`jpeg` -> `jpg`, `yml` -> `yaml`, ...).
/// Returns a slice borrowed from either the static table or the input.
pub fn canonical(ext: &str) -> &str {
    let e = ext.trim().trim_start_matches('.');
    if e.eq_ignore_ascii_case("jpeg") {
        return "jpg";
    }
    if e.eq_ignore_ascii_case("tif") {
        return "tiff";
    }
    if e.eq_ignore_ascii_case("yml") {
        return "yaml";
    }
    if e.eq_ignore_ascii_case("htm") {
        return "html";
    }
    if e.eq_ignore_ascii_case("aif") {
        return "aiff";
    }
    FORMATS
        .iter()
        .find(|f| f.ext.eq_ignore_ascii_case(e))
        .map(|f| f.ext)
        .unwrap_or(e)
}

/// Look up a format by extension (aliases accepted).
pub fn format(ext: &str) -> Option<&'static Format> {
    let e = canonical(ext);
    FORMATS.iter().find(|f| f.ext == e)
}

/// Extension of a path, lower-cased and canonicalised.
pub fn extension_of(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| canonical(e).to_string())
}

/// A single supported from -> to conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Conversion {
    pub from: &'static str,
    pub to: &'static str,
}

/// Every conversion the engine advertises, generated from the tables above.
pub fn all_conversions() -> Vec<Conversion> {
    let mut out = Vec::new();

    for f in IMAGE_INPUT {
        for t in IMAGE_OUTPUT {
            let from = canonical(f);
            let to = canonical(t);
            if from != to {
                out.push(Conversion { from, to });
            }
        }
    }

    for f in DATA_FORMATS {
        for t in DATA_FORMATS {
            if f != t {
                out.push(Conversion { from: *f, to: *t });
            }
        }
    }

    // Text / markup conversions that are actually meaningful offline.
    for (f, t) in [
        ("md", "html"),
        ("md", "txt"),
        ("html", "txt"),
        ("html", "md"),
        ("txt", "html"),
        ("txt", "md"),
    ] {
        out.push(Conversion { from: f, to: t });
    }

    // Documents.
    out.push(Conversion { from: "pdf", to: "txt" });
    out.push(Conversion { from: "txt", to: "pdf" });
    out.push(Conversion { from: "md", to: "pdf" });
    out.push(Conversion { from: "html", to: "pdf" });

    // Images can be embedded into a PDF (via JPEG).
    for f in IMAGE_INPUT {
        out.push(Conversion { from: canonical(f), to: "pdf" });
    }

    // Audio is decoded to lossless PCM WAV.
    for f in AUDIO_INPUT {
        if *f != "wav" {
            out.push(Conversion { from: *f, to: "wav" });
        }
    }

    out.sort_by_key(|c| (c.from, c.to));
    out.dedup();
    out
}

/// Output extensions compatible with an input extension.
pub fn targets_for(from: &str) -> Vec<&'static str> {
    let f = canonical(from);
    let mut out: Vec<&'static str> = all_conversions()
        .into_iter()
        .filter(|c| c.from == f)
        .map(|c| c.to)
        .collect();
    out.sort_unstable();
    out.dedup();
    out
}

/// Is this conversion advertised as supported?
pub fn can_convert(from: &str, to: &str) -> bool {
    let f = canonical(from);
    let t = canonical(to);
    all_conversions().iter().any(|c| c.from == f && c.to == t)
}

/// Check whether a file extension looks like a known readable format.
pub fn is_readable(ext: &str) -> bool {
    format(ext).map(|f| f.input).unwrap_or(false)
}
