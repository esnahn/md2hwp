//! Character formats for manuscript bold, italic, strikeout, superscript and
//! subscript, derived from a template character format.
//!
//! Strikeout and superscript/subscript use the encoding Hancom 2020
//! (11.0.0.9166) stores for its own 취소선/위 첨자/아래 첨자 commands:
//! strikeout sets attr bits 18-20 to 1 and strike shape bits 26-29 to 0
//! (solid), writes underline bits 2-3 as 2 only when there is no underline,
//! and stores strike color 0. Superscript is bit 15 and subscript bit 16.
//! A template format that already has a strikeout keeps its own type, shape
//! and color.

use rhwp::model::document::DocInfo;
use rhwp::model::raw_provenance::{record_digest, record_raw_permitted};

/// Hancom draws superscript/subscript glyphs at 64% of the character size
/// (measured from Hancom 2020 page images at 10pt and 20pt).
pub(crate) const SCRIPT_SCALE: f64 = 0.64;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Script {
    #[default]
    None,
    Superscript,
    Subscript,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Marks {
    pub bold: bool,
    pub italic: bool,
    pub strikeout: bool,
    pub script: Script,
}

impl Marks {
    pub fn with_bold(self) -> Self {
        Self { bold: true, ..self }
    }
    pub fn with_italic(self) -> Self {
        Self {
            italic: true,
            ..self
        }
    }
    pub fn with_strikeout(self) -> Self {
        Self {
            strikeout: true,
            ..self
        }
    }
    pub fn with_script(self, script: Script) -> Self {
        Self { script, ..self }
    }
}

const ATTR: std::ops::Range<usize> = 46..50;
const STRIKE_COLOR: std::ops::Range<usize> = 70..74;

/// Returns the character format id for `base` with `marks` applied, appending
/// a new format when one is needed.
pub(crate) fn derive(info: &mut DocInfo, base: u32, marks: Marks) -> Result<u32, String> {
    if marks == Marks::default() {
        return Ok(base);
    }
    let mut shape = info
        .char_shapes
        .get(base as usize)
        .ok_or("Template references an absent character format")?
        .clone();
    if !marks.strikeout && marks.script == Script::None {
        // Bold/italic-only formats keep their established encoding.
        shape.bold |= marks.bold;
        shape.italic |= marks.italic;
        shape.attr = (shape.attr & !3) | u32::from(shape.italic) | (u32::from(shape.bold) << 1);
        shape.raw_data = None;
        let id = u32::try_from(info.char_shapes.len()).map_err(|_| "Too many character formats")?;
        info.char_shapes.push(shape);
        info.raw_stream_dirty = true;
        return Ok(id);
    }
    let seals = info
        .raw_provenance
        .as_ref()
        .map(|p| p.record_seals.char_shapes.as_slice());
    let mut raw = shape
        .raw_data
        .take()
        .filter(|_| record_raw_permitted(seals, base as usize, &shape))
        .unwrap_or_else(|| rhwp::serializer::doc_info::serialize_char_shape(&shape));
    if raw.len() < STRIKE_COLOR.end {
        return Err("Truncated native character format".into());
    }
    let mut attr = u32::from_le_bytes(raw[ATTR].try_into().expect("four bytes"));
    if marks.bold {
        shape.bold = true;
        attr |= 1 << 1;
    }
    if marks.italic {
        shape.italic = true;
        attr |= 1;
    }
    if marks.strikeout && !shape.strikethrough {
        shape.strikethrough = true;
        shape.strike_shape = 0;
        shape.strike_color = 0;
        if (attr >> 18) & 0x07 == 0 {
            attr |= 1 << 18;
        }
        attr &= !(0x0F << 26);
        if (attr >> 2) & 0x03 == 0 {
            attr |= 2 << 2;
        }
        raw[STRIKE_COLOR].copy_from_slice(&0u32.to_le_bytes());
    }
    match marks.script {
        Script::None => {}
        Script::Superscript => {
            shape.superscript = true;
            shape.subscript = false;
            attr = (attr | 1 << 15) & !(1 << 16);
        }
        Script::Subscript => {
            shape.subscript = true;
            shape.superscript = false;
            attr = (attr | 1 << 16) & !(1 << 15);
        }
    }
    shape.attr = attr;
    raw[ATTR].copy_from_slice(&attr.to_le_bytes());
    shape.raw_data = Some(raw);
    let id = u32::try_from(info.char_shapes.len()).map_err(|_| "Too many character formats")?;
    if let Some(provenance) = &mut info.raw_provenance {
        let seals = &mut provenance.record_seals.char_shapes;
        seals.resize(info.char_shapes.len(), [0; 32]);
        seals.push(record_digest(&shape));
    }
    info.char_shapes.push(shape);
    info.raw_stream_dirty = true;
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Character formats read back from the Hancom 2020 document whose
    /// 취소선/위 첨자/아래 첨자 commands were applied to plain and underlined text.
    fn reparse(info: &DocInfo, id: u32) -> rhwp::model::style::CharShape {
        let cs = &info.char_shapes[id as usize];
        let raw = cs.raw_data.clone().unwrap();
        let record = rhwp::serializer::record_writer::write_record(
            rhwp::parser::tags::HWPTAG_CHAR_SHAPE,
            1,
            &raw,
        );
        let mut reparsed = rhwp::parser::doc_info::parse_doc_info(&record)
            .map(|(info, _)| info.char_shapes)
            .unwrap_or_default();
        reparsed.pop().unwrap()
    }

    /// The tracked Hancom-authored template: native raw records and seals.
    fn plain_info() -> DocInfo {
        let core =
            rhwp::DocumentCore::from_bytes(include_bytes!("../../../templates/template.hwp"))
                .unwrap();
        let mut info = core.document().doc_info.clone();
        // Index 0 is a plain body format without underline or strikeout.
        assert!(!info.char_shapes[0].strikethrough);
        assert_eq!(
            info.char_shapes[0].underline_type,
            rhwp::model::style::UnderlineType::None
        );
        info.char_shapes.truncate(1);
        if let Some(p) = &mut info.raw_provenance {
            p.record_seals.char_shapes.truncate(1);
        }
        info
    }

    #[test]
    fn matches_hancom_encoding() {
        let mut info = plain_info();
        let base = 0;
        let before = info.char_shapes.len();
        let strike = derive(
            &mut info,
            base,
            Marks {
                strikeout: true,
                ..Marks::default()
            },
        )
        .unwrap();
        let attr = |info: &DocInfo, id: u32| {
            u32::from_le_bytes(
                info.char_shapes[id as usize].raw_data.as_ref().unwrap()[ATTR]
                    .try_into()
                    .unwrap(),
            )
        };
        let base_attr = attr(&info, base) & !(0x07 << 18 | 0x0F << 26 | 0x03 << 2);
        assert_eq!(attr(&info, strike) & !base_attr, 0x0004_0008 & !base_attr);
        let sup = derive(
            &mut info,
            base,
            Marks {
                bold: true,
                strikeout: true,
                script: Script::Superscript,
                ..Marks::default()
            },
        )
        .unwrap();
        assert_eq!(attr(&info, sup) & 0x0001_800a, 0x0000_800a);
        assert!(info.char_shapes[sup as usize].superscript);
        assert!(info.char_shapes[sup as usize].strikethrough);
        let sub = derive(
            &mut info,
            sup,
            Marks::default().with_script(Script::Subscript),
        )
        .unwrap();
        assert!(info.char_shapes[sub as usize].subscript);
        assert!(!info.char_shapes[sub as usize].superscript);
        assert_eq!(info.char_shapes.len(), before + 3);
        let reparsed = reparse(&info, sub);
        assert!(reparsed.subscript && reparsed.strikethrough && reparsed.bold);
        assert_eq!(reparsed.strike_shape, 0);
    }

    #[test]
    fn keeps_existing_underline_and_strikeout_settings() {
        let mut info = plain_info();
        let mut underlined = info.char_shapes[0].clone();
        underlined.underline_type = rhwp::model::style::UnderlineType::Bottom;
        underlined.attr = (underlined.attr & !(0x03 << 2)) | 1 << 2;
        underlined.raw_data = None;
        info.char_shapes.push(underlined);
        let base = (info.char_shapes.len() - 1) as u32;
        let id = derive(&mut info, base, Marks::default().with_strikeout()).unwrap();
        let reparsed = reparse(&info, id);
        assert!(reparsed.strikethrough);
        assert_eq!(
            reparsed.underline_type,
            rhwp::model::style::UnderlineType::Bottom
        );

        let mut red = info.char_shapes[id as usize].clone();
        red.strike_shape = 3;
        red.strike_color = 0x0000ff;
        red.attr = (red.attr & !(0x0F << 26)) | 3 << 26;
        red.raw_data = None;
        info.char_shapes.push(red);
        let base = (info.char_shapes.len() - 1) as u32;
        let id = derive(
            &mut info,
            base,
            Marks::default().with_strikeout().with_bold(),
        )
        .unwrap();
        let reparsed = reparse(&info, id);
        assert_eq!(
            (reparsed.strike_shape, reparsed.strike_color),
            (3, 0x0000ff)
        );
    }
}
