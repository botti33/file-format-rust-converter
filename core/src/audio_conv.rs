//! Audio decoding. Everything symphonia can read is written as 16-bit PCM
//! WAV, which is the one audio format with a universally interoperable,
//! dependency-free encoder in Rust.

use std::io::Cursor;

use hound::{SampleFormat, WavSpec, WavWriter};
use symphonia::core::codecs::audio::{AudioDecoderOptions, CODEC_ID_NULL_AUDIO};
use symphonia::core::errors::Error;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::TrackType;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

use crate::model;

pub fn is_input(ext: &str) -> bool {
    let e = model::canonical(ext);
    model::AUDIO_INPUT.contains(&e)
}

pub fn is_output(ext: &str) -> bool {
    model::canonical(ext) == "wav"
}

pub fn to_wav(data: &[u8], from: &str) -> Result<Vec<u8>, String> {
    let mut hint = Hint::new();
    hint.with_extension(from);

    let source = Cursor::new(data.to_vec());
    let stream = MediaSourceStream::new(Box::new(source), Default::default());
    let probed = symphonia::default::get_probe()
        .probe(&hint, stream, Default::default(), MetadataOptions::default())
        .map_err(|e| format!("cannot probe audio: {e}"))?;

    let mut format = probed;
    let track = format
        .default_track(TrackType::Audio)
        .ok_or("no audio track found")?;
    let params = track
        .codec_params
        .as_ref()
        .and_then(|p| p.audio())
        .ok_or("track has no audio parameters")?;
    if params.codec == CODEC_ID_NULL_AUDIO {
        return Err("unsupported audio codec".to_string());
    }

    let track_id = track.id;
    let sample_rate = params.sample_rate.unwrap_or(44_100);
    let channels = params
        .channels
        .as_ref()
        .map(|c| c.count() as u16)
        .unwrap_or(2)
        .max(1);

    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(params, &AudioDecoderOptions::default())
        .map_err(|e| format!("cannot create decoder: {e}"))?;

    let spec = WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    let mut wav_bytes = Cursor::new(Vec::new());
    {
        let mut writer = WavWriter::new(&mut wav_bytes, spec)
            .map_err(|e| format!("cannot create WAV: {e}"))?;

        loop {
            let packet = match format.next_packet() {
                Ok(Some(packet)) => packet,
                Ok(None) => break,
                Err(Error::ResetRequired) => continue,
                Err(Error::DecodeError(_)) => continue,
                Err(e) => return Err(format!("packet read failed: {e}")),
            };
            if packet.track_id != track_id {
                continue;
            }

            let decoded = match decoder.decode(&packet) {
                Ok(decoded) => decoded,
                Err(Error::DecodeError(_)) => continue,
                Err(Error::ResetRequired) => continue,
                Err(e) => return Err(format!("decode failed: {e}")),
            };

            if decoded.spec().channels().count() == 0 {
                continue;
            }
            let mut samples: Vec<i16> = Vec::with_capacity(decoded.samples_interleaved());
            decoded.copy_to_vec_interleaved(&mut samples);
            for sample in samples {
                writer
                    .write_sample(sample)
                    .map_err(|e| format!("cannot write WAV sample: {e}"))?;
            }
        }
        writer
            .finalize()
            .map_err(|e| format!("cannot finalize WAV: {e}"))?;
    }

    Ok(wav_bytes.into_inner())
}
