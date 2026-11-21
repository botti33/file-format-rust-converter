//! AnyConvert conversion engine.
//!
//! All conversions happen locally: there is no network access and no external
//! executable required. The public surface is intentionally small so both the
//! CLI and GUI can share exactly the same behaviour.

pub mod audio_conv;
pub mod doc_conv;
pub mod engine;
pub mod image_conv;
pub mod model;
pub mod text_conv;

pub use engine::{convert_bytes, convert_file, convert_many, ConvertError, ConvertOptions, ConvertOutcome};
pub use model::{all_conversions, can_convert, extension_of, format, targets_for, Conversion, Format, Kind, FORMATS};
