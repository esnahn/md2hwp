// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Euisoon Ahn
//! Converts an SVG figure into a format Hancom embeds: EMF when the drawing needs
//! only opaque solid fills, otherwise a PNG rendered by resvg.

#![forbid(unsafe_code)]

mod emf;

use resvg::{tiny_skia, usvg};
use std::sync::{
    Arc, OnceLock,
    atomic::{AtomicBool, Ordering},
};

/// Largest accepted SVG file and largest accepted width or height in CSS pixels.
pub const MAX_BYTES: u64 = 64 * 1024 * 1024;
const MAX_SIZE: f32 = 20_000.0;
/// Raster fallback: at least this many pixels wide, at most this many per side.
const RASTER_WIDTH: f32 = 2400.0;
const RASTER_LIMIT: f32 = 8000.0;
const PNG_MARK: &[u8] = b"Software\0md2hwp-svg";
/// JPEG comment segment: big-endian length, then the text.
const JPEG_MARK: &[u8] = b"\0\x0cmd2hwp-svg";
const JPEG_QUALITY: u8 = 92;

/// How an SVG figure is stored. `Auto` keeps vectors when the drawing allows it,
/// otherwise stores a PNG, and prefers a PNG that is smaller than a large EMF.
/// `Emf` stores EMF or fails.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Target {
    #[default]
    Auto,
    Emf,
    Png,
    Jpg,
}

/// File extensions a conversion can produce.
pub const EXTENSIONS: [&str; 3] = ["emf", "png", "jpg"];
/// `Target::Auto` compares an EMF larger than this with the PNG of the drawing.
pub const LARGE_EMF_BYTES: usize = 5 * 1024 * 1024;

impl Target {
    pub const NAMES: [&str; 4] = ["auto", "emf", "png", "jpg"];

    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "auto" => Some(Self::Auto),
            "emf" => Some(Self::Emf),
            "png" => Some(Self::Png),
            "jpg" => Some(Self::Jpg),
            _ => None,
        }
    }
}

pub struct Converted {
    pub bytes: Vec<u8>,
    /// One of [`EXTENSIONS`].
    pub extension: &'static str,
    /// Why an `Auto` conversion was stored as PNG, as a clause.
    pub fallback: Option<String>,
}

pub fn convert(svg: &[u8], target: Target) -> Result<Converted, String> {
    convert_with_limit(svg, target, LARGE_EMF_BYTES)
}

fn convert_with_limit(svg: &[u8], target: Target, large_emf: usize) -> Result<Converted, String> {
    let tree = parse(svg)?;
    let converted = |bytes, extension, fallback| Converted {
        bytes,
        extension,
        fallback,
    };
    Ok(match target {
        Target::Auto | Target::Emf => match emf::encode(&tree) {
            Ok(bytes) if target == Target::Auto && bytes.len() > large_emf => {
                let raster = png(&rasterize(&tree)?)?;
                if raster.len() < bytes.len() {
                    let reason = format!(
                        "its EMF ({}) is larger than the PNG ({})",
                        mebibytes(bytes.len()),
                        mebibytes(raster.len())
                    );
                    converted(raster, "png", Some(reason))
                } else {
                    converted(bytes, "emf", None)
                }
            }
            Ok(bytes) => converted(bytes, "emf", None),
            Err(feature) if target == Target::Emf => {
                return Err(format!(
                    "this drawing cannot be stored as EMF because it uses {feature}"
                ));
            }
            Err(feature) => {
                let reason = format!("it uses {feature}");
                converted(png(&rasterize(&tree)?)?, "png", Some(reason))
            }
        },
        Target::Png => converted(png(&rasterize(&tree)?)?, "png", None),
        Target::Jpg => converted(jpeg(&rasterize(&tree)?)?, "jpg", None),
    })
}

fn mebibytes(bytes: usize) -> String {
    format!("{:.1} MiB", bytes as f64 / (1024.0 * 1024.0))
}

/// Whether `bytes` is a file this crate wrote, so that it may be replaced.
pub fn is_generated(bytes: &[u8]) -> bool {
    emf::is_generated(bytes)
        || (bytes.starts_with(b"\x89PNG\r\n\x1a\n")
            && bytes.get(37..41) == Some(b"tEXt".as_slice())
            && bytes.get(41..41 + PNG_MARK.len()) == Some(PNG_MARK))
        || jpeg_comment_offset(bytes)
            .and_then(|at| bytes.get(at..at + 2 + JPEG_MARK.len()))
            .is_some_and(|comment| comment[..2] == [0xff, 0xfe] && comment[2..] == *JPEG_MARK)
}

/// Where the generated-file comment sits: after SOI and the JFIF APP0 segment.
fn jpeg_comment_offset(bytes: &[u8]) -> Option<usize> {
    if bytes.get(..2)? != [0xff, 0xd8] {
        return None;
    }
    if bytes.get(2..4)? != [0xff, 0xe0] {
        return Some(2);
    }
    Some(4 + usize::from(u16::from_be_bytes([*bytes.get(4)?, *bytes.get(5)?])))
}
fn fonts() -> Arc<usvg::fontdb::Database> {
    static FONTS: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();
    FONTS
        .get_or_init(|| {
            let mut database = usvg::fontdb::Database::new();
            database.load_system_fonts();
            Arc::new(database)
        })
        .clone()
}

fn parse(svg: &[u8]) -> Result<usvg::Tree, String> {
    if svg.len() as u64 > MAX_BYTES {
        return Err("SVG exceeds 64 MiB".into());
    }
    let external = Arc::new(AtomicBool::new(false));
    let seen = external.clone();
    let options = usvg::Options {
        resources_dir: None,
        fontdb: fonts(),
        image_href_resolver: usvg::ImageHrefResolver {
            resolve_data: usvg::ImageHrefResolver::default_data_resolver(),
            // Never read other files on behalf of an SVG.
            resolve_string: Box::new(move |_, _| {
                seen.store(true, Ordering::Relaxed);
                None
            }),
        },
        ..usvg::Options::default()
    };
    let tree = usvg::Tree::from_data(svg, &options).map_err(|e| format!("invalid SVG: {e}"))?;
    if external.load(Ordering::Relaxed) {
        return Err("SVG refers to an external image; embed it as a data URL".into());
    }
    let size = tree.size();
    if size.width() > MAX_SIZE || size.height() > MAX_SIZE {
        return Err("SVG is larger than 20000 pixels".into());
    }
    check_text(tree.root())?;
    Ok(tree)
}

/// usvg drops text it cannot shape; refuse instead of losing it silently.
fn check_text(group: &usvg::Group) -> Result<(), String> {
    for node in group.children() {
        match node {
            usvg::Node::Group(group) => check_text(group)?,
            usvg::Node::Text(text) => {
                let written = text
                    .chunks()
                    .iter()
                    .any(|chunk| !chunk.text().trim().is_empty());
                if written && !text.flattened().has_children() {
                    return Err(
                        "SVG text could not be converted to outlines; check its fonts".into(),
                    );
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn rasterize(tree: &usvg::Tree) -> Result<tiny_skia::Pixmap, String> {
    let size = tree.size();
    let scale = (RASTER_WIDTH / size.width())
        .max(1.0)
        .min(RASTER_LIMIT / size.width())
        .min(RASTER_LIMIT / size.height());
    let pixels = |length: f32| ((length * scale).ceil() as u32).max(1);
    let mut pixmap = tiny_skia::Pixmap::new(pixels(size.width()), pixels(size.height()))
        .ok_or("SVG raster size is invalid")?;
    resvg::render(
        tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    Ok(pixmap)
}

fn png(pixmap: &tiny_skia::Pixmap) -> Result<Vec<u8>, String> {
    let png = pixmap
        .encode_png()
        .map_err(|e| format!("PNG encoding failed: {e}"))?;
    // Mark the file as generated with a tEXt chunk right after IHDR.
    let mut chunk = b"tEXt".to_vec();
    chunk.extend_from_slice(PNG_MARK);
    let mut marked = png[..33].to_vec();
    marked.extend_from_slice(&(chunk.len() as u32 - 4).to_be_bytes());
    marked.extend_from_slice(&chunk);
    marked.extend_from_slice(&crc32(&chunk).to_be_bytes());
    marked.extend_from_slice(&png[33..]);
    Ok(marked)
}

/// JPEG has no alpha: the drawing is placed on white.
fn jpeg(pixmap: &tiny_skia::Pixmap) -> Result<Vec<u8>, String> {
    let rgb: Vec<u8> = pixmap
        .pixels()
        .iter()
        .flat_map(|pixel| {
            // Premultiplied colour over white.
            let rest = 255 - pixel.alpha();
            [
                pixel.red() + rest,
                pixel.green() + rest,
                pixel.blue() + rest,
            ]
        })
        .collect();
    let mut encoded = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut encoded, JPEG_QUALITY)
        .encode(
            &rgb,
            pixmap.width(),
            pixmap.height(),
            image::ExtendedColorType::Rgb8,
        )
        .map_err(|e| format!("JPEG encoding failed: {e}"))?;
    let at = jpeg_comment_offset(&encoded).ok_or("JPEG encoder wrote no SOI")?;
    let mut marked = encoded[..at].to_vec();
    marked.extend_from_slice(&[0xff, 0xfe]);
    marked.extend_from_slice(JPEG_MARK);
    marked.extend_from_slice(&encoded[at..]);
    Ok(marked)
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320 & 0u32.wrapping_sub(crc & 1));
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    fn svg(body: &str) -> Vec<u8> {
        format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100" viewBox="0 0 200 100">{body}</svg>"#
        )
        .into_bytes()
    }

    const SHAPES: &str = r##"<defs><clipPath id="c"><rect x="10" y="10" width="100" height="60"/></clipPath></defs>
        <rect width="200" height="100" fill="#ffeecc"/>
        <g clip-path="url(#c)" transform="rotate(5)"><circle cx="60" cy="40" r="50" fill="#336699" fill-rule="evenodd"/></g>
        <path d="M20 80 Q100 20 180 80" fill="none" stroke="#c00" stroke-width="3" stroke-dasharray="6 3" stroke-linecap="round" stroke-opacity="0.5"/>
        <rect width="9" height="9" fill="#000" fill-opacity="0"/>"##;

    type PlusRecord = (u16, u16, Vec<u8>);

    fn records(emf: &[u8]) -> (Vec<u32>, Vec<PlusRecord>) {
        let word = |at: usize| u32::from_le_bytes(emf[at..at + 4].try_into().unwrap());
        let half = |at: usize| u16::from_le_bytes(emf[at..at + 2].try_into().unwrap());
        assert_eq!((word(0), word(40), word(44)), (1, 0x464d_4520, 0x0001_0000));
        assert_eq!(word(48) as usize, emf.len());
        let (mut gdi, mut plus) = (Vec::new(), Vec::new());
        let (mut cursor, mut count) = (0, 0);
        while cursor < emf.len() {
            let size = word(cursor + 4) as usize;
            assert!(size >= 8 && size.is_multiple_of(4) && size <= emf.len() - cursor);
            count += 1;
            if word(cursor) == 70 {
                assert_eq!(&emf[cursor + 12..cursor + 16], b"EMF+");
                assert_eq!(word(cursor + 8) as usize, size - 12);
                let mut at = cursor + 16;
                while at < cursor + size {
                    let (length, data) = (word(at + 4) as usize, word(at + 8) as usize);
                    assert!(length.is_multiple_of(4) && data <= length - 12);
                    plus.push((
                        half(at),
                        half(at + 2),
                        emf[at + 12..at + 12 + data].to_vec(),
                    ));
                    at += length;
                }
                assert_eq!(at, cursor + size);
            } else {
                gdi.push(word(cursor));
            }
            cursor += size;
        }
        assert_eq!(count, word(52) as usize);
        assert_eq!(gdi.last(), Some(&14));
        (gdi, plus)
    }

    #[test]
    fn shapes_strokes_alpha_and_simple_clips_become_dual_emf() {
        let converted = convert(&svg(SHAPES), Target::Emf).unwrap();
        assert_eq!((converted.extension, &converted.fallback), ("emf", &None));
        let bytes = &converted.bytes;
        assert!(is_generated(bytes));
        let word = |at: usize| i32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
        // Bounds in device pixels, frame in 0.01 mm at 96 dpi.
        assert_eq!((word(16) - word(8), word(20) - word(12)), (199, 99));
        assert_eq!((word(32) - word(24), word(36) - word(28)), (5292, 2646));
        let (gdi, plus) = records(bytes);
        for kind in [59, 60, 62, 67, 33, 34, 5, 39] {
            assert!(gdi.contains(&kind), "missing GDI record {kind}");
        }
        // Three fills in each half; the fully transparent one is in neither.
        assert_eq!(gdi.iter().filter(|kind| **kind == 62).count(), 3);
        let kinds: Vec<u16> = plus.iter().map(|record| record.0).collect();
        assert_eq!(
            kinds,
            [
                0x4001, 0x401e, 0x4030, 0x4032, 0x4008, 0x4014, 0x4025, 0x4008, 0x4033, 0x4008,
                0x4014, 0x4026, 0x4008, 0x4014, 0x4002
            ]
        );
        assert_eq!(plus[0].1, 1, "dual header");
        // Colours are ARGB; only the EMF+ half keeps the stroke's alpha.
        let fills: Vec<&[u8]> = plus
            .iter()
            .filter(|record| record.0 == 0x4014)
            .map(|record| record.2.as_slice())
            .collect();
        assert_eq!(fills[0], [0xcc, 0xee, 0xff, 0xff]);
        assert_eq!(fills[2], [0x00, 0x00, 0xcc, 0x80]);
        // Path flags carry the fill rule: even-odd circle, winding stroke outline.
        let paths: Vec<u32> = plus
            .iter()
            .filter(|record| record.0 == 0x4008)
            .map(|record| u32::from_le_bytes(record.2[8..12].try_into().unwrap()))
            .collect();
        assert_eq!(paths, [0x2000, 0x2000, 0, 0x2000]);
        // The first path is the 200x100 rectangle: four float points, closed.
        let rectangle = &plus[4].2;
        assert_eq!(u32::from_le_bytes(rectangle[4..8].try_into().unwrap()), 4);
        assert_eq!(rectangle.len(), 12 + 4 * 8 + 4);
        assert_eq!(rectangle[44..48], [0, 1, 1, 0x81]);
    }

    #[test]
    fn features_without_an_emf_form_fall_back_to_marked_png() {
        for (body, reason) in [
            (
                r##"<g opacity="0.5"><rect width="50" height="50" fill="#000"/><rect width="5" height="5" fill="#f00"/></g>"##,
                "opacity",
            ),
            (
                r##"<defs><linearGradient id="g"><stop offset="0" stop-color="#fff"/><stop offset="1" stop-color="#000"/></linearGradient></defs><rect width="50" height="50" fill="url(#g)"/>"##,
                "gradient",
            ),
        ] {
            let error = convert(&svg(body), Target::Emf).err().unwrap();
            assert!(
                error.contains("cannot be stored as EMF") && error.contains(reason),
                "{error}"
            );
            let converted = convert(&svg(body), Target::Auto).unwrap();
            assert_eq!(converted.extension, "png");
            let why = converted.fallback.unwrap();
            assert!(why.contains(reason), "{why}");
        }
    }

    #[test]
    fn opacity_on_one_shape_fades_its_colours() {
        // matplotlib writes artist alpha as `opacity` on the element itself.
        let converted = convert(
            &svg(r##"<path d="M10 10h60v60h-60z" style="fill:#ff0000;stroke:#0000ff;stroke-width:8;opacity:0.5"/>"##),
            Target::Emf,
        )
        .unwrap();
        assert_eq!((converted.extension, &converted.fallback), ("emf", &None));
        let (gdi, plus) = records(&converted.bytes);
        // The fill is drawn outside the stroke outline (exclude clip), then the stroke.
        let clip = plus.iter().find(|record| record.0 == 0x4033).unwrap();
        assert_eq!(clip.1, 0x0400);
        assert!(gdi.windows(2).any(|pair| pair == [60, 67]));
        let fills: Vec<&[u8]> = plus
            .iter()
            .filter(|record| record.0 == 0x4014)
            .map(|record| record.2.as_slice())
            .collect();
        assert_eq!(fills, [[0x00, 0x00, 0xff, 0x80], [0xff, 0x00, 0x00, 0x80]]);
    }

    #[test]
    fn opacity_over_shapes_of_one_colour_fills_their_union_once() {
        let converted = convert(
            &svg(r##"<g opacity="0.25"><rect width="50" height="50" fill="#00f"/><g transform="translate(30 0)"><rect width="50" height="50" fill="#00f"/></g></g>"##),
            Target::Emf,
        )
        .unwrap();
        assert_eq!((converted.extension, &converted.fallback), ("emf", &None));
        let (_, plus) = records(&converted.bytes);
        let fills: Vec<&[u8]> = plus
            .iter()
            .filter(|record| record.0 == 0x4014)
            .map(|record| record.2.as_slice())
            .collect();
        assert_eq!(fills, [[0xff, 0x00, 0x00, 0x40]]);
        let path = &plus.iter().find(|record| record.0 == 0x4008).unwrap().2;
        // One winding path holding both rectangles.
        assert_eq!(u32::from_le_bytes(path[4..8].try_into().unwrap()), 8);
        assert_eq!(u32::from_le_bytes(path[8..12].try_into().unwrap()), 0x2000);
    }

    #[test]
    fn pattern_fills_repeat_their_content_under_a_clip() {
        // A 20x20 tile over a 50x30 rectangle touches 3x2 tiles; the dot stays in
        // its tile, the bar leaves it and gets a clip per tile.
        let hatch = |content: &str| {
            let body = format!(
                r##"<defs><pattern id="h" patternUnits="userSpaceOnUse" width="20" height="20">{content}</pattern></defs><rect x="5" y="5" width="50" height="30" fill="url(#h)"/>"##
            );
            let converted = convert(&svg(&body), Target::Emf).unwrap();
            assert_eq!((converted.extension, &converted.fallback), ("emf", &None));
            let (_, plus) = records(&converted.bytes);
            let count = |kind: u16| plus.iter().filter(|record| record.0 == kind).count();
            (count(0x4014), count(0x4033))
        };
        assert_eq!(
            hatch(r##"<circle cx="10" cy="10" r="3" fill="#00f"/>"##),
            (6, 1)
        );
        assert_eq!(
            hatch(r##"<path d="M-5 10H25" fill="none" stroke="#f00" stroke-width="2"/>"##),
            (6, 7)
        );
        let converted = convert(
            &svg(r##"<defs><pattern id="h" patternUnits="userSpaceOnUse" width="0.01" height="0.01"><rect width="1" height="1"/></pattern></defs><rect width="200" height="100" fill="url(#h)"/>"##),
            Target::Auto,
        )
        .unwrap();
        assert!(converted.fallback.unwrap().contains("too many times"));
    }

    #[test]
    fn auto_prefers_a_smaller_png_only_for_large_emf() {
        let many: String = (0..600)
            .map(|i| {
                format!(
                    r##"<circle cx="{}" cy="{}" r="2" fill="#000"/>"##,
                    10 + i % 7,
                    10 + i % 5
                )
            })
            .collect();
        let emf = convert(&svg(&many), Target::Emf).unwrap();
        assert_eq!((emf.extension, &emf.fallback), ("emf", &None));
        // Over the limit and larger than the PNG: the PNG is stored.
        let auto = convert_with_limit(&svg(&many), Target::Auto, 1000).unwrap();
        assert_eq!(auto.extension, "png");
        assert!(auto.bytes.len() < emf.bytes.len());
        assert!(auto.fallback.unwrap().contains("is larger than the PNG"));
        // Under the limit the EMF stays, whatever the PNG would weigh.
        let kept = convert_with_limit(&svg(&many), Target::Auto, usize::MAX).unwrap();
        assert_eq!((kept.extension, kept.bytes.len()), ("emf", emf.bytes.len()));
        // Over the limit but smaller than its PNG: the EMF stays.
        let small = svg(r##"<rect width="200" height="100" fill="#abc"/>"##);
        let kept = convert_with_limit(&small, Target::Auto, 0).unwrap();
        assert_eq!((kept.extension, &kept.fallback), ("emf", &None));
        assert_eq!(convert(&small, Target::Auto).unwrap().extension, "emf");
        assert_eq!(Target::parse("auto"), Some(Target::default()));
    }

    /// An `<image>` element holding a 3x2 PNG whose first pixel has `alpha`.
    fn picture(alpha: u8) -> String {
        let mut pixmap = tiny_skia::Pixmap::new(3, 2).unwrap();
        pixmap.fill(tiny_skia::Color::from_rgba8(200, 100, 50, 255));
        pixmap.pixels_mut()[0] = tiny_skia::PremultipliedColorU8::from_rgba(0, 0, 0, alpha)
            .unwrap_or(tiny_skia::PremultipliedColorU8::TRANSPARENT);
        let png = pixmap.encode_png().unwrap();
        const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut text = String::new();
        for chunk in png.chunks(3) {
            let word = chunk
                .iter()
                .fold(0u32, |word, byte| word << 8 | u32::from(*byte))
                << (8 * (3 - chunk.len()));
            for index in 0..4 {
                text.push(if index <= chunk.len() {
                    TABLE[(word >> (18 - 6 * index)) as usize & 63] as char
                } else {
                    '='
                });
            }
        }
        format!(
            r#"<g transform="rotate(10)"><image x="20" y="10" width="90" height="60" href="data:image/png;base64,{text}"/></g>"#
        )
    }

    #[test]
    fn embedded_images_keep_their_file_and_get_a_gdi_bitmap() {
        for (alpha, blits) in [
            (255, vec![0x00cc_0020]),
            (0, vec![0x0088_00c6, 0x00ee_0086]),
        ] {
            let converted = convert(&svg(&picture(alpha)), Target::Emf).unwrap();
            let (_, plus) = records(&converted.bytes);
            let object = plus.iter().find(|record| record.1 == 0x0501).unwrap();
            assert_eq!(object.0, 0x4008);
            assert_eq!(object.2[4..8], [1, 0, 0, 0], "bitmap image");
            assert_eq!(object.2[24..32], [1, 0, 0, 0, 0x89, b'P', b'N', b'G']);
            let draw = plus.iter().find(|record| record.0 == 0x401b).unwrap();
            assert_eq!((draw.1, draw.2.len()), (1, 52));
            let float = |at: usize| f32::from_le_bytes(draw.2[at..at + 4].try_into().unwrap());
            assert_eq!(
                (float(8), float(16), float(20)),
                (-0.5, 3.0, 2.0),
                "source rectangle"
            );
            // First destination point: (20, 10) rotated by ten degrees.
            assert!((float(28) - 17.96).abs() < 0.01 && (float(32) - 13.32).abs() < 0.01);
            // GDI: world transform, then one blit, or mask and paint.
            let bytes = &converted.bytes;
            let word = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
            let (mut cursor, mut seen) = (0, Vec::new());
            while cursor < bytes.len() {
                if word(cursor) == 81 {
                    assert_eq!(
                        (word(cursor + 40), word(cursor + 44)),
                        (3, 2),
                        "source size"
                    );
                    assert_eq!(word(cursor + 48), 80, "offBmiSrc");
                    let bits = word(cursor + 56) as usize + word(cursor + 60) as usize;
                    assert_eq!(word(cursor + 4) as usize, bits.next_multiple_of(4));
                    seen.push(word(cursor + 68));
                }
                cursor += word(cursor + 4) as usize;
            }
            assert_eq!(seen, blits);
        }
        let error = convert(
            &svg(
                r#"<image width="9" height="9" href="data:image/gif;base64,R0lGODlhAQABAAAAACw="/>"#,
            ),
            Target::Emf,
        );
        assert!(error.is_err());
    }

    #[test]
    fn raster_targets_are_marked_and_well_formed() {
        let png = convert(&svg(SHAPES), Target::Png).unwrap();
        assert_eq!((png.extension, &png.fallback), ("png", &None));
        let bytes = &png.bytes;
        assert!(is_generated(bytes));
        assert_eq!(&bytes[12..16], b"IHDR");
        let size = |at: usize| u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap());
        assert_eq!((size(16), size(20)), (2400, 1200));
        // The inserted chunk keeps the stream well formed.
        let length = size(33) as usize;
        assert_eq!(size(41 + length), crc32(&bytes[37..41 + length]));
        assert_eq!(&bytes[49 + length..53 + length], b"IDAT");

        let jpg = convert(&svg(SHAPES), Target::Jpg).unwrap();
        assert_eq!(jpg.extension, "jpg");
        assert!(is_generated(&jpg.bytes));
        let decoded = image::load_from_memory_with_format(&jpg.bytes, image::ImageFormat::Jpeg)
            .unwrap()
            .into_rgb8();
        assert_eq!(decoded.dimensions(), (2400, 1200));
        // Outside the clipped circle the background colour shows, not black.
        let corner = decoded.get_pixel(2390, 10).0;
        assert!(
            corner[0] > 245 && corner[1] > 228 && corner[2] > 194,
            "{corner:?}"
        );
    }

    #[test]
    fn invalid_external_and_foreign_files_are_rejected() {
        assert!(convert(b"<svg", Target::Emf).is_err());
        let error = convert(
            &svg(r#"<image href="other.png" width="10" height="10"/>"#),
            Target::Png,
        )
        .err()
        .unwrap();
        assert!(error.contains("external image"), "{error}");
        for foreign in [
            b"\x89PNG\r\n\x1a\n".to_vec(),
            vec![0; 200],
            b"\xff\xd8\xff\xe0\x00\x10JFIF".to_vec(),
        ] {
            assert!(!is_generated(&foreign));
        }
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
        assert_eq!(Target::parse("jpg"), Some(Target::Jpg));
        assert_eq!(Target::parse("jpeg"), None);
    }
}
