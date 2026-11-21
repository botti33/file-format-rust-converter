use std::io::Cursor;

use anyconvert_core::{convert_bytes, ConvertOptions};
use hound::{SampleFormat, WavSpec, WavWriter};
use image::{ImageFormat, RgbaImage};

fn options() -> ConvertOptions {
    ConvertOptions::default()
}

fn sample_png() -> Vec<u8> {
    let mut image = RgbaImage::new(4, 3);
    for (x, y, pixel) in image.enumerate_pixels_mut() {
        *pixel = image::Rgba([(x * 60) as u8, (y * 80) as u8, 120, 255]);
    }
    let mut bytes = Cursor::new(Vec::new());
    image
        .write_to(&mut bytes, ImageFormat::Png)
        .expect("write sample png");
    bytes.into_inner()
}

#[test]
fn images_convert_to_many_formats() {
    let png = sample_png();
    for target in ["jpg", "webp", "bmp", "gif", "tif", "qoi", "tga", "ico"] {
        let out = convert_bytes(&png, "png", target, &options())
            .unwrap_or_else(|e| panic!("png -> {target}: {e}"));
        let format = ImageFormat::from_extension(target).unwrap();
        let decoded = image::load_from_memory_with_format(&out, format)
            .unwrap_or_else(|e| panic!("decode {target}: {e}"));
        assert_eq!(decoded.width(), 4, "{target} width");
        assert_eq!(decoded.height(), 3, "{target} height");
    }
}

#[test]
fn structured_data_round_trips() {
    let json = br#"{"name":"sample","count":3,"enabled":true,"nested":{"a":"b"}}"#;

    for intermediate in ["yaml", "toml", "json5", "ron", "xml", "json"] {
        let encoded = convert_bytes(json, "json", intermediate, &options())
            .unwrap_or_else(|e| panic!("json -> {intermediate}: {e}"));
        let back = convert_bytes(&encoded, intermediate, "json", &options())
            .unwrap_or_else(|e| panic!("{intermediate} -> json: {e}"));
        let value: serde_json::Value = serde_json::from_slice(&back).unwrap();
        assert_eq!(value["name"], "sample", "via {intermediate}");
        assert_eq!(value["count"], 3, "via {intermediate}");
        assert_eq!(value["enabled"], true, "via {intermediate}");
    }
}

#[test]
fn csv_and_ini_convert() {
    let json = br#"[{"name":"a","qty":2},{"name":"b","qty":7}]"#;
    let csv = convert_bytes(json, "json", "csv", &options()).unwrap();
    assert!(String::from_utf8_lossy(&csv).contains("name,qty"));
    let back = convert_bytes(&csv, "csv", "json", &options()).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&back).unwrap();
    assert_eq!(value[1]["qty"], 7);

    let ini_json = br#"{"server":{"host":"localhost","port":8080}}"#;
    let ini = convert_bytes(ini_json, "json", "ini", &options()).unwrap();
    let text = String::from_utf8_lossy(&ini);
    assert!(text.contains("[server]"), "{text}");
    let back = convert_bytes(&ini, "ini", "json", &options()).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&back).unwrap();
    assert_eq!(value["server"]["port"], 8080);
}

#[test]
fn markdown_html_and_text() {
    let md = b"# Heading\n\nSome **bold** text and [a link](https://example.com).\n";
    let html = convert_bytes(md, "md", "html", &options()).unwrap();
    let html = String::from_utf8_lossy(&html);
    assert!(html.contains("<h1>"), "{html}");
    assert!(html.contains("<strong>bold</strong>"), "{html}");

    let plain = convert_bytes(md, "md", "txt", &options()).unwrap();
    let plain = String::from_utf8_lossy(&plain);
    assert!(plain.contains("Heading"), "{plain}");
    assert!(plain.contains("bold"), "{plain}");

    let back = convert_bytes(html.as_bytes(), "html", "md", &options()).unwrap();
    assert!(String::from_utf8_lossy(&back).contains("Heading"));

    let from_txt = convert_bytes(b"plain text", "txt", "html", &options()).unwrap();
    assert!(String::from_utf8_lossy(&from_txt).contains("<pre>plain text</pre>"));
}

#[test]
fn pdf_text_and_image_embedding() {
    let pdf = convert_bytes(b"Hello from AnyConvert\n", "txt", "pdf", &options()).unwrap();
    assert!(pdf.starts_with(b"%PDF-"));
    let text = convert_bytes(&pdf, "pdf", "txt", &options()).unwrap();
    assert!(String::from_utf8_lossy(&text).contains("Hello from AnyConvert"));

    let image_pdf = convert_bytes(&sample_png(), "png", "pdf", &options()).unwrap();
    assert!(image_pdf.starts_with(b"%PDF-"));
    assert!(image_pdf.windows(10).any(|w| w == b"/DCTDecode"));
}

#[test]
fn audio_decodes_to_pcm_wav() {
    let spec = WavSpec {
        channels: 2,
        sample_rate: 44_100,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = WavWriter::new(&mut cursor, spec).unwrap();
        for i in 0..2_000 {
            writer.write_sample((i % 256) as i16).unwrap();
            writer.write_sample(-((i % 128) as i16)).unwrap();
        }
        writer.finalize().unwrap();
    }
    let wav = cursor.into_inner();

    let decoded = anyconvert_core::audio_conv::to_wav(&wav, "wav").expect("decode wav");
    assert!(decoded.starts_with(b"RIFF"), "missing RIFF header");
    assert!(decoded.windows(4).any(|w| w == b"WAVE"));
    assert!(decoded.len() > wav.len() / 2, "unexpectedly small output");
}

#[test]
fn capability_tables_are_consistent() {
    for conversion in anyconvert_core::all_conversions() {
        assert!(
            anyconvert_core::targets_for(conversion.from).contains(&conversion.to),
            "{:?} missing from targets_for",
            conversion
        );
        assert!(
            anyconvert_core::can_convert(conversion.from, conversion.to),
            "{:?} missing from can_convert",
            conversion
        );
    }
}
