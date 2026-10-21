// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Euisoon Ahn
//! Template-owned cell formats measured by the same GDI policy as Hancom.

use md2hwp_core::ir::Inline;
use md2hwp_font_measurement::{FontSpec, Measurement};
use rhwp::model::{
    document::DocInfo,
    style::CharShape,
    table::{Cell, Table},
};
use std::collections::HashMap;
use unicode_segmentation::UnicodeSegmentation;

const LOGICAL_SCALE: f64 = 4.0;

// Keep the manuscript inputs and independently owned native sample references
// explicit at the rendering boundary.
#[allow(clippy::too_many_arguments)]
pub(crate) fn calculate(
    header: &[Vec<Inline>],
    rows: &[Vec<Vec<Inline>>],
    header_sample: &Cell,
    body_sample: &Cell,
    prototype: &Table,
    info: &DocInfo,
    width_hwp_units: i32,
) -> Result<Vec<i32>, String> {
    if header.is_empty() || width_hwp_units <= 0 || rows.iter().any(|r| r.len() != header.len()) {
        return Err(
            "Table width and column count must be positive; rows must be rectangular".into(),
        );
    }
    let header_format = CellFormat::read(header_sample, prototype, info)?;
    let body_format = CellFormat::read(body_sample, prototype, info)?;
    let mut measurement = TextMeasurement {
        native: Measurement::new()?,
        info,
        widths: HashMap::new(),
    };
    let mut minimum = vec![1.0f64; header.len()];
    let mut preferred = vec![1.0f64; header.len()];
    for col in 0..header.len() {
        for (inlines, format) in std::iter::once((&header[col], &header_format))
            .chain(rows.iter().map(|row| (&row[col], &body_format)))
        {
            let (min, pref) = measurement.measure(inlines, format)?;
            minimum[col] = minimum[col].max(min * 1.01 + format.margins);
            preferred[col] = preferred[col]
                .max(pref * 1.01 + format.margins)
                .max(minimum[col]);
        }
    }
    distribute(&minimum, &preferred, width_hwp_units)
}

struct CellFormat<'a> {
    shape: &'a CharShape,
    keep_latin: bool,
    keep_other: bool,
    margins: f64,
}

impl<'a> CellFormat<'a> {
    fn read(cell: &Cell, table: &Table, info: &'a DocInfo) -> Result<Self, String> {
        let [paragraph] = cell.paragraphs.as_slice() else {
            return Err("Table sample cell must contain one paragraph".into());
        };
        let char_id = paragraph
            .char_shape_id_at(0)
            .ok_or("Table sample slot has no character format")?;
        let shape = info
            .char_shapes
            .get(char_id as usize)
            .ok_or("Invalid table sample character format")?;
        let para = info
            .para_shapes
            .get(paragraph.para_shape_id as usize)
            .ok_or("Invalid table sample paragraph format")?;
        let margin = if cell.apply_inner_margin {
            cell.padding
        } else {
            table.padding
        };
        // Native paragraph margins are URC (1/14400 inch), cells are HWPUNIT.
        let margins = f64::from(margin.left.max(0))
            + f64::from(margin.right.max(0))
            + (f64::from(para.margin_left.max(0))
                + f64::from(para.margin_right.max(0))
                + f64::from(para.indent).abs())
                / 2.0;
        let keep_latin = match para.break_latin_word.as_deref() {
            Some(value) => value == "KEEP_WORD",
            None => matches!((para.attr1 >> 5) & 3, 0 | 3),
        };
        Ok(Self {
            shape,
            keep_latin,
            // Native HWP5 bit 7 means non-Latin word breaking is allowed,
            // matching HWPML BreakNonLatinWord=true. Keep words only when
            // it is clear. Do not reuse rhwp's inverted HWPX lexical mapping.
            keep_other: para.attr1 & (1 << 7) == 0,
            margins,
        })
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct FontKey {
    spec: FontSpec,
    ratio: u8,
    spacing: i8,
    use_font_space: bool,
}

struct TextMeasurement<'a> {
    native: Measurement,
    info: &'a DocInfo,
    widths: HashMap<(FontKey, String), f64>,
}

#[derive(Default)]
struct Bounds {
    minimum: f64,
    preferred: f64,
    line: f64,
    word: f64,
    previous_gap: f64,
    word_gap: f64,
}

impl Bounds {
    fn end_line(&mut self) {
        self.minimum = self.minimum.max(self.word);
        self.preferred = self.preferred.max(self.line);
        self.line = 0.0;
        self.word = 0.0;
        self.previous_gap = 0.0;
        self.word_gap = 0.0;
    }
}

impl TextMeasurement<'_> {
    fn measure(
        &mut self,
        inlines: &[Inline],
        format: &CellFormat<'_>,
    ) -> Result<(f64, f64), String> {
        let mut bounds = Bounds::default();
        self.walk(inlines, format, false, false, &mut bounds)?;
        bounds.end_line();
        Ok((bounds.minimum, bounds.preferred))
    }

    fn walk(
        &mut self,
        inlines: &[Inline],
        format: &CellFormat<'_>,
        strong: bool,
        emphasis: bool,
        bounds: &mut Bounds,
    ) -> Result<(), String> {
        for inline in inlines {
            match inline {
                Inline::Text { value } => self.text(value, format, strong, emphasis, bounds)?,
                Inline::Space => self.text(" ", format, strong, emphasis, bounds)?,
                Inline::LineBreak => bounds.end_line(),
                Inline::Strong { inlines } => self.walk(inlines, format, true, emphasis, bounds)?,
                Inline::Emph { inlines } => self.walk(inlines, format, strong, true, bounds)?,
                Inline::Link { inlines, .. } => {
                    self.walk(inlines, format, strong, emphasis, bounds)?
                }
                Inline::Footnote { .. } | Inline::CrossReference { .. } => {
                    return Err(
                        "rhwp table automatic widths do not support footnotes or cross references"
                            .into(),
                    );
                }
            }
        }
        Ok(())
    }

    fn text(
        &mut self,
        text: &str,
        format: &CellFormat<'_>,
        strong: bool,
        emphasis: bool,
        bounds: &mut Bounds,
    ) -> Result<(), String> {
        for element in text.graphemes(true) {
            let lang = language(element);
            let key = self.font(format.shape, lang, strong, emphasis)?;
            let width = self.width(&key, element)?;
            bounds.line += bounds.previous_gap + width;
            bounds.previous_gap = width * f64::from(key.spacing) / 100.0;
            if element.chars().all(char::is_whitespace) {
                bounds.minimum = bounds.minimum.max(bounds.word);
                bounds.word = 0.0;
                bounds.word_gap = 0.0;
            } else if if lang == 1 {
                format.keep_latin
            } else {
                format.keep_other
            } {
                bounds.word += bounds.word_gap + width;
                bounds.word_gap = bounds.previous_gap;
            } else {
                bounds.minimum = bounds.minimum.max(bounds.word).max(width);
                bounds.word = 0.0;
                bounds.word_gap = 0.0;
            }
        }
        Ok(())
    }

    fn font(
        &self,
        shape: &CharShape,
        lang: usize,
        strong: bool,
        emphasis: bool,
    ) -> Result<FontKey, String> {
        let font = self
            .info
            .font_faces
            .get(lang)
            .and_then(|faces| faces.get(shape.font_ids[lang] as usize))
            .ok_or("Table character format refers to an absent language font")?;
        if font.alt_type != 1 {
            return Err("Table automatic widths require a Windows TrueType/OpenType font; Hancom HFT fonts cannot be measured. Replace the table sample font.".into());
        }
        let relative = shape.relative_sizes[lang];
        let ratio = shape.ratios[lang];
        if shape.base_size <= 0
            || relative == 0
            || ratio == 0
            || font.name.is_empty()
            || font.name.encode_utf16().count() > 31
        {
            return Err("Table font size, horizontal scale or face name is invalid".into());
        }
        // C# Math.Round uses ties-to-even, then scales by exactly four.
        let height = (f64::from(shape.base_size) * f64::from(relative) / 100.0 * LOGICAL_SCALE)
            .round_ties_even();
        if height < 1.0 || height > f64::from(i32::MAX) {
            return Err("Table measurement font height is out of range".into());
        }
        Ok(FontKey {
            spec: FontSpec {
                face: font.name.clone(),
                height: height as i32,
                bold: shape.bold || strong,
                italic: shape.italic || emphasis,
            },
            ratio,
            spacing: shape.spacings[lang],
            use_font_space: shape.use_font_space,
        })
    }

    fn width(&mut self, key: &FontKey, text: &str) -> Result<f64, String> {
        let cache = (key.clone(), text.to_owned());
        if let Some(width) = self.widths.get(&cache) {
            return Ok(*width);
        }
        // Measure even fixed-width spaces, so unavailable fonts still fail.
        let extent = self
            .native
            .extent(&key.spec, if text == "\t" { "    " } else { text })?;
        let advance = if text == " " && !key.use_font_space {
            f64::from(key.spec.height) / LOGICAL_SCALE / 2.0
        } else {
            f64::from(extent) / LOGICAL_SCALE
        };
        let width = (advance * f64::from(key.ratio) / 100.0).max(0.0);
        self.widths.insert(cache, width);
        Ok(width)
    }
}

fn language(text: &str) -> usize {
    match text.chars().next().map(u32::from).unwrap_or(0) {
        0x1100..=0x11ff | 0x3130..=0x318f | 0xa960..=0xa97f | 0xac00..=0xd7ff => 0,
        0x3040..=0x30ff | 0x31f0..=0x31ff | 0xff66..=0xff9f => 3,
        0x3400..=0x9fff | 0xf900..=0xfaff | 0x20000..=0x3134f => 2,
        0..=0x024f => 1,
        0x2000..=0x2bff => 5,
        _ => 4,
    }
}

fn distribute(minimum: &[f64], preferred: &[f64], total: i32) -> Result<Vec<i32>, String> {
    if minimum.is_empty()
        || minimum.len() != preferred.len()
        || total <= 0
        || minimum
            .iter()
            .chain(preferred)
            .any(|v| !v.is_finite() || *v < 0.0)
    {
        return Err("Invalid measured table widths".into());
    }
    let floors: Vec<_> = minimum.iter().map(|v| v.max(1.0).ceil()).collect();
    if floors.iter().sum::<f64>() > f64::from(total) {
        return Err("The declared table width is smaller than the cells' minimum text widths and margins. Increase table.width-mm, reduce the cell margins/font size, or allow word wrapping in the prototype.".into());
    }
    let remainder = f64::from(total) - floors.iter().sum::<f64>();
    let mut weights: Vec<_> = preferred
        .iter()
        .zip(&floors)
        .map(|(v, f)| (v - f).max(0.0))
        .collect();
    if weights.iter().sum::<f64>() <= remainder {
        weights = preferred
            .iter()
            .zip(&floors)
            .map(|(v, f)| v.max(*f))
            .collect();
    }
    let mut sum = weights.iter().sum::<f64>();
    if sum <= 0.0 {
        weights.fill(1.0);
        sum = weights.len() as f64;
    }
    let shares: Vec<_> = weights.iter().map(|w| remainder * w / sum).collect();
    let mut result: Vec<i32> = floors
        .iter()
        .zip(&shares)
        .map(|(f, s)| (f + s.floor()) as i32)
        .collect();
    let missing = total - result.iter().sum::<i32>();
    let mut order: Vec<_> = (0..result.len()).collect();
    order.sort_by(|a, b| {
        shares[*b]
            .fract()
            .total_cmp(&shares[*a].fract())
            .then(a.cmp(b))
    });
    for index in order.into_iter().take(missing as usize) {
        result[index] += 1;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn constrained_distribution_preserves_minimums_and_total() {
        assert_eq!(
            distribute(&[10.1, 20.1, 30.1], &[20.0, 80.0, 40.0], 100).unwrap(),
            [15, 50, 35]
        );
        assert!(distribute(&[50.1, 50.0], &[60.0, 60.0], 100).is_err());
    }
    #[test]
    fn spare_width_uses_preferred_proportions_and_stable_ties() {
        assert_eq!(
            distribute(&[1.0, 1.0], &[10.0, 30.0], 102).unwrap(),
            [26, 76]
        );
        assert_eq!(
            distribute(&[0.0, 0.0, 0.0], &[0.0, 0.0, 0.0], 5).unwrap(),
            [2, 2, 1]
        );
    }
    #[test]
    fn paragraph_margins_convert_from_urc_once() {
        use rhwp::model::{paragraph::Paragraph, style::ParaShape};
        let mut p = Paragraph::new_empty();
        p.char_shapes.push(rhwp::model::paragraph::CharShapeRef {
            start_pos: 0,
            char_shape_id: 0,
        });
        p.text = "x".into();
        let info = DocInfo {
            char_shapes: vec![CharShape::default()],
            para_shapes: vec![ParaShape {
                margin_left: 600,
                margin_right: 200,
                indent: -400,
                ..Default::default()
            }],
            ..Default::default()
        };
        let cell = Cell {
            paragraphs: vec![p],
            ..Default::default()
        };
        let table = Table {
            padding: rhwp::model::Padding {
                left: 100,
                right: 200,
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(
            CellFormat::read(&cell, &table, &info).unwrap().margins,
            900.0
        );
    }

    #[test]
    fn native_non_latin_break_permission_matches_hwpml_policy() {
        use rhwp::model::{
            paragraph::{CharShapeRef, Paragraph},
            style::ParaShape,
        };
        let mut paragraph = Paragraph::new_empty();
        paragraph.text = "표".into();
        paragraph.char_shapes.push(CharShapeRef {
            start_pos: 0,
            char_shape_id: 0,
        });
        let cell = Cell {
            paragraphs: vec![paragraph],
            ..Default::default()
        };
        let table = Table::default();
        for (attr1, expected_keep) in [(0, true), (1 << 7, false)] {
            let info = DocInfo {
                char_shapes: vec![CharShape::default()],
                para_shapes: vec![ParaShape {
                    attr1,
                    ..Default::default()
                }],
                ..Default::default()
            };
            // Header samples export BreakNonLatinWord=false (keep whole
            // Korean words); body samples export true (allow character breaks).
            assert_eq!(
                CellFormat::read(&cell, &table, &info).unwrap().keep_other,
                expected_keep
            );
        }
    }
}
