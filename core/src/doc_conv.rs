//! PDF handoff: extract text from PDFs and create simple, valid PDFs from
//! text or images. No external tools or network access are used.

use image::GenericImageView;

use crate::engine::ConvertError;

pub fn pdf_to_text(data: &[u8]) -> Result<Vec<u8>, ConvertError> {
    let text = pdf_extract::extract_text_from_mem(data)
        .map_err(|e| ConvertError::Other(format!("PDF text extraction failed: {e}")))?;
    Ok(text.into_bytes())
}

fn pdf_escape(s: &str) -> String {
    let mut out = String::new();
    for ch in s.chars() {
        match ch {
            '(' | ')' | '\\' => {
                out.push('\\');
                out.push(ch);
            }
            '\n' => out.push_str(" ) Tj T* ("),
            '\r' => {}
            c if c.is_control() => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

/// Build a minimal one-page PDF containing selectable text.
pub fn text_to_pdf(text: &str) -> Result<Vec<u8>, ConvertError> {
    let clean = text.replace('\r', "");
    let mut content = String::from("BT /F1 10 Tf 12 TL 48 744 Td\n");
    for (i, line) in clean.split('\n').enumerate() {
        if i > 0 {
            content.push_str("T*\n");
        }
        content.push('(');
        content.push_str(&pdf_escape(line));
        content.push_str(") Tj\n");
        if i > 58 {
            break;
        }
    }
    content.push_str("ET\n");

    let mut objects: Vec<Vec<u8>> = Vec::new();
    objects.push(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_vec());
    objects.push(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n".to_vec());
    objects.push(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
          /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>\nendobj\n"
            .to_vec(),
    );
    objects.push(
        format!(
            "4 0 obj\n<< /Length {} >>\nstream\n{}endstream\nendobj\n",
            content.len(),
            content
        )
        .into_bytes(),
    );
    objects.push(
        b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica \
          /Encoding /WinAnsiEncoding >>\nendobj\n"
            .to_vec(),
    );

    Ok(assemble_pdf(objects))
}

/// Embed an already-encoded JPEG as a full-page PDF image.
pub fn jpeg_to_pdf(jpeg: &[u8]) -> Result<Vec<u8>, ConvertError> {
    let image = image::load_from_memory(jpeg)?;
    let (w, h) = image.dimensions();
    let scale = (540.0 / w as f32).min((720.0 / h as f32).min(1.0));
    let dw = w as f32 * scale;
    let dh = h as f32 * scale;
    let x = (612.0 - dw) / 2.0;
    let y = (792.0 - dh) / 2.0;
    let content = format!("q\n{dw:.2} 0 0 {dh:.2} {x:.2} {y:.2} cm\n/Im0 Do\nQ\n");

    let mut objects: Vec<Vec<u8>> = Vec::new();
    objects.push(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_vec());
    objects.push(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n".to_vec());
    objects.push(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
          /Resources << /XObject << /Im0 5 0 R >> >> /Contents 4 0 R >>\nendobj\n"
            .to_vec(),
    );
    objects.push(
        format!(
            "4 0 obj\n<< /Length {} >>\nstream\n{}endstream\nendobj\n",
            content.len(),
            content
        )
        .into_bytes(),
    );

    let mut image_obj = format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Image /Width {w} /Height {h} \
         /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length {} >>\nstream\n",
        jpeg.len()
    )
    .into_bytes();
    image_obj.extend_from_slice(jpeg);
    image_obj.extend_from_slice(b"\nendstream\nendobj\n");
    objects.push(image_obj);

    Ok(assemble_pdf(objects))
}

fn assemble_pdf(objects: Vec<Vec<u8>>) -> Vec<u8> {
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets: Vec<usize> = vec![0];
    for object in objects {
        offsets.push(pdf.len());
        pdf.extend_from_slice(&object);
    }

    let xref = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n", offsets.len()).as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            offsets.len()
        )
        .as_bytes(),
    );
    pdf
}
