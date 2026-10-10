//! Inject Apple Live Photo pairing metadata into JPEG stills.
//!
//! Photos pairs a still + short MOV when both share the same Content Identifier.
//! The MOV side is written via FFmpeg `content.identifier` tags; the still needs
//! MakerNote tag 0x0011 (ContentIdentifier) plus a standard ImageUniqueID.

use std::path::Path;

use crate::error_handler::{QuickGifError, QuickGifResult};

const SOI: [u8; 2] = [0xff, 0xd8];
const MARKER_PREFIX: u8 = 0xff;
const APP1: u8 = 0xe1;
const SOS: u8 = 0xda;

/// Inject Live Photo Content Identifier into an existing JPEG file (in place).
pub fn inject_content_identifier(path: &Path, content_id: &str) -> QuickGifResult<()> {
    if content_id.is_empty() || content_id.len() > 128 {
        return Err(QuickGifError::Io(
            "Live Photo Content Identifier 无效".to_string(),
        ));
    }

    let original = std::fs::read(path)?;
    let rewritten = inject_content_identifier_bytes(&original, content_id)?;
    std::fs::write(path, rewritten)?;
    Ok(())
}

pub fn inject_content_identifier_bytes(jpeg: &[u8], content_id: &str) -> QuickGifResult<Vec<u8>> {
    if jpeg.len() < 4 || jpeg[0..2] != SOI {
        return Err(QuickGifError::Io("不是有效的 JPEG 文件".to_string()));
    }

    let app1 = build_exif_app1(content_id)?;
    let mut output = Vec::with_capacity(jpeg.len() + app1.len() + 16);
    output.extend_from_slice(&SOI);

    let mut index = 2usize;
    let mut inserted = false;

    while index + 1 < jpeg.len() {
        if jpeg[index] != MARKER_PREFIX {
            // Entropy-coded segment without marker alignment — append rest.
            if !inserted {
                output.extend_from_slice(&app1);
            }
            output.extend_from_slice(&jpeg[index..]);
            return Ok(output);
        }

        let marker = jpeg[index + 1];
        if marker == 0xd9 {
            // EOI before SOS (unusual) — insert then finish.
            if !inserted {
                output.extend_from_slice(&app1);
            }
            output.extend_from_slice(&jpeg[index..]);
            return Ok(output);
        }

        // Standalone markers without length.
        if marker == 0x01 || (0xd0..=0xd7).contains(&marker) {
            output.extend_from_slice(&jpeg[index..index + 2]);
            index += 2;
            continue;
        }

        if index + 4 > jpeg.len() {
            break;
        }

        let segment_len = u16::from_be_bytes([jpeg[index + 2], jpeg[index + 3]]) as usize;
        if segment_len < 2 || index + 2 + segment_len > jpeg.len() {
            return Err(QuickGifError::Io("JPEG 段长度无效".to_string()));
        }

        let segment_end = index + 2 + segment_len;

        if marker == SOS {
            if !inserted {
                output.extend_from_slice(&app1);
            }
            output.extend_from_slice(&jpeg[index..]);
            return Ok(output);
        }

        // Drop existing EXIF APP1 so we own ContentIdentifier; keep XMP / other APP1.
        if marker == APP1 && is_exif_app1(&jpeg[index..segment_end]) {
            index = segment_end;
            continue;
        }

        // Insert our EXIF as early as possible (after SOI / before other APPn).
        if !inserted && marker >= 0xe0 {
            output.extend_from_slice(&app1);
            inserted = true;
        }

        output.extend_from_slice(&jpeg[index..segment_end]);
        index = segment_end;
    }

    if !inserted {
        output.extend_from_slice(&app1);
    }
    if index < jpeg.len() {
        output.extend_from_slice(&jpeg[index..]);
    }
    Ok(output)
}

fn is_exif_app1(segment: &[u8]) -> bool {
    // marker(2) + len(2) + "Exif\0\0"
    segment.len() >= 10 && &segment[4..10] == b"Exif\0\0"
}

/// Build a complete APP1 segment including marker and length bytes.
fn build_exif_app1(content_id: &str) -> QuickGifResult<Vec<u8>> {
    let tiff = build_tiff_with_apple_content_id(content_id)?;
    let mut payload = Vec::with_capacity(6 + tiff.len());
    payload.extend_from_slice(b"Exif\0\0");
    payload.extend_from_slice(&tiff);

    let segment_len = payload.len() + 2; // length field includes itself
    if segment_len > u16::MAX as usize {
        return Err(QuickGifError::Io("EXIF 段过大".to_string()));
    }

    let mut app1 = Vec::with_capacity(4 + payload.len());
    app1.extend_from_slice(&[MARKER_PREFIX, APP1]);
    app1.extend_from_slice(&(segment_len as u16).to_be_bytes());
    app1.extend_from_slice(&payload);
    Ok(app1)
}

/// Little-endian TIFF with IFD0 + ExifIFD + Apple MakerNote ContentIdentifier.
fn build_tiff_with_apple_content_id(content_id: &str) -> QuickGifResult<Vec<u8>> {
    let content_bytes = {
        let mut bytes = content_id.as_bytes().to_vec();
        if !bytes.ends_with(&[0]) {
            bytes.push(0);
        }
        bytes
    };
    let make = b"Apple\0";
    let software = b"QuickGif\0";
    let maker_note = build_apple_makernote_ifd(&content_bytes);

    // Layout (all offsets relative to TIFF header start):
    // 0:  II 2A 00 | IFD0 offset=8
    // 8:  IFD0 (3 entries) -> Make, Software, ExifIFD
    // after IFD0 entries + next: value region for Make/Software if needed
    // ExifIFD: ImageUniqueID + MakerNote

    let mut tiff = Vec::new();
    // TIFF header
    tiff.extend_from_slice(b"II"); // little-endian
    tiff.extend_from_slice(&42u16.to_le_bytes());
    tiff.extend_from_slice(&8u32.to_le_bytes()); // IFD0 at 8

    // We'll assemble IFDs with a two-pass style: reserve then patch is messy;
    // build sections then concatenate with known sizes.

    // IFD0: 3 entries (Make 0x010F, Software 0x0131, ExifOffset 0x8769)
    // Entry = 12 bytes; count=2; next=4; values follow
    let ifd0_count: u16 = 3;
    let ifd0_header_size = 2 + (ifd0_count as usize) * 12 + 4;
    let ifd0_start = 8usize;
    let ifd0_values_start = ifd0_start + ifd0_header_size;

    // Make and Software are >4 bytes so they live in value area.
    let make_offset = ifd0_values_start as u32;
    let software_offset = (ifd0_values_start + make.len()) as u32;
    let after_ifd0_values = ifd0_values_start + make.len() + software.len();

    // Exif IFD follows IFD0 values
    let exif_ifd_start = after_ifd0_values;
    let exif_count: u16 = 2; // ImageUniqueID + MakerNote
    let exif_header_size = 2 + (exif_count as usize) * 12 + 4;
    let exif_values_start = exif_ifd_start + exif_header_size;

    let unique_id_offset = exif_values_start as u32;
    let maker_note_offset = (exif_values_start + content_bytes.len()) as u32;

    // Write IFD0
    tiff.extend_from_slice(&ifd0_count.to_le_bytes());
    write_ifd_entry(&mut tiff, 0x010f, 2, make.len() as u32, make_offset); // Make ASCII
    write_ifd_entry(&mut tiff, 0x0131, 2, software.len() as u32, software_offset); // Software
    write_ifd_entry(&mut tiff, 0x8769, 4, 1, exif_ifd_start as u32); // ExifOffset LONG
    tiff.extend_from_slice(&0u32.to_le_bytes()); // next IFD
    tiff.extend_from_slice(make);
    tiff.extend_from_slice(software);

    debug_assert_eq!(tiff.len(), exif_ifd_start);

    // Write Exif IFD
    tiff.extend_from_slice(&exif_count.to_le_bytes());
    // ImageUniqueID 0xA420 ASCII
    write_ifd_entry(
        &mut tiff,
        0xa420,
        2,
        content_bytes.len() as u32,
        unique_id_offset,
    );
    // MakerNote 0x927C UNDEFINED
    write_ifd_entry(
        &mut tiff,
        0x927c,
        7,
        maker_note.len() as u32,
        maker_note_offset,
    );
    tiff.extend_from_slice(&0u32.to_le_bytes());
    tiff.extend_from_slice(&content_bytes);
    tiff.extend_from_slice(&maker_note);

    Ok(tiff)
}

fn write_ifd_entry(buf: &mut Vec<u8>, tag: u16, type_id: u16, count: u32, value_or_offset: u32) {
    buf.extend_from_slice(&tag.to_le_bytes());
    buf.extend_from_slice(&type_id.to_le_bytes());
    buf.extend_from_slice(&count.to_le_bytes());
    buf.extend_from_slice(&value_or_offset.to_le_bytes());
}

/// Apple MakerNote as a miniature little-endian TIFF IFD with tag 0x0011.
fn build_apple_makernote_ifd(content_bytes: &[u8]) -> Vec<u8> {
    // Header used by many iOS stills; followed by offset to IFD (from start of MakerNote).
    // Layout: "Apple iOS\0" (10) + version u16 BE (2) + IFD offset u32 BE (4) = 16 bytes.
    let mut note = Vec::new();
    note.extend_from_slice(b"Apple iOS\0");
    note.extend_from_slice(&1u16.to_be_bytes()); // version
    let ifd_offset_from_start = 16u32;
    note.extend_from_slice(&ifd_offset_from_start.to_be_bytes());

    // IFD uses little-endian (common for third-party + accepted by Photos pairing).
    let ifd_start = note.len();
    debug_assert_eq!(ifd_start, 16);

    let entry_count: u16 = 1;
    let header_size = 2 + 12 + 4;
    let value_offset_in_note = (ifd_start + header_size) as u32;

    note.extend_from_slice(&entry_count.to_le_bytes());
    // ContentIdentifier 0x0011 ASCII
    write_ifd_entry(
        &mut note,
        0x0011,
        2,
        content_bytes.len() as u32,
        value_offset_in_note,
    );
    note.extend_from_slice(&0u32.to_le_bytes());
    note.extend_from_slice(content_bytes);
    note
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_jpeg() -> Vec<u8> {
        // SOI + APP0(JFIF) + SOS + a few bytes + EOI — enough for injector.
        let mut jpeg = vec![0xff, 0xd8];
        // APP0
        jpeg.extend_from_slice(&[0xff, 0xe0, 0x00, 0x10]);
        jpeg.extend_from_slice(b"JFIF\0");
        jpeg.extend_from_slice(&[0x01, 0x01, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00]);
        // SOS
        jpeg.extend_from_slice(&[0xff, 0xda, 0x00, 0x08, 0x01, 0x01, 0x00, 0x00, 0x3f, 0x00]);
        jpeg.extend_from_slice(&[0x00, 0xff, 0xd9]);
        jpeg
    }

    #[test]
    fn injects_content_identifier_into_jpeg() {
        let id = "A1B2C3D4-E5F6-7890-ABCD-EF1234567890";
        let out = inject_content_identifier_bytes(&minimal_jpeg(), id).unwrap();
        assert_eq!(&out[0..2], &SOI);
        assert!(out.windows(id.len()).any(|window| window == id.as_bytes()));
        assert!(out.windows(6).any(|window| window == b"Exif\0\0"));
        assert!(out.windows(9).any(|window| window == b"Apple iOS"));
    }

    #[test]
    fn replaces_existing_exif_app1() {
        let id = "11111111-2222-3333-4444-555555555555";
        let mut jpeg = vec![0xff, 0xd8];
        // Fake EXIF APP1
        let fake = b"Exif\0\0old-data-here!!!!";
        let len = (fake.len() + 2) as u16;
        jpeg.extend_from_slice(&[0xff, 0xe1]);
        jpeg.extend_from_slice(&len.to_be_bytes());
        jpeg.extend_from_slice(fake);
        jpeg.extend_from_slice(&[0xff, 0xda, 0x00, 0x08, 0x01, 0x01, 0x00, 0x00, 0x3f, 0x00]);
        jpeg.extend_from_slice(&[0x00, 0xff, 0xd9]);

        let out = inject_content_identifier_bytes(&jpeg, id).unwrap();
        let exif_count = out.windows(6).filter(|window| *window == b"Exif\0\0").count();
        assert_eq!(exif_count, 1);
        assert!(out.windows(id.len()).any(|window| window == id.as_bytes()));
        assert!(!out.windows(8).any(|window| window == b"old-data"));
    }
}
