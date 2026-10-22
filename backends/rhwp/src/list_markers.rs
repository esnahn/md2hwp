// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Euisoon Ahn
//! Resolve native marker text before measuring generated list continuations.

use std::collections::{BTreeMap, BTreeSet};

use rhwp::{
    model::{
        control::Control,
        document::Document,
        paragraph::{NumberingRestart, Paragraph},
        shape::{Caption, DrawingObjAttr, ShapeObject},
        style::{HeadType, Numbering},
    },
    renderer::{NumberFormat, format_number},
};

#[derive(Default)]
struct Counters {
    active: Option<u16>,
    values: [u32; 7],
    history: BTreeMap<u16, [u32; 7]>,
}

impl Counters {
    fn advance(
        &mut self,
        id: u16,
        level: usize,
        restart: Option<NumberingRestart>,
    ) -> Result<[u32; 7], String> {
        if self.active != Some(id) {
            if let Some(previous) = self.active {
                self.history.insert(previous, self.values);
            }
            if let Some(previous) = self.history.get(&id) {
                self.values = *previous;
            } else {
                // A fresh native definition inherits the active ancestor counts,
                // not the ancestor's displayed numbers or manuscript list starts.
                self.values[level..].fill(0);
            }
            self.active = Some(id);
        }
        if let Some(NumberingRestart::NewStart(start)) = restart {
            self.values[level] = start.saturating_sub(1);
        }
        self.values[level] = self.values[level]
            .checked_add(1)
            .ok_or("Native list counter overflow")?;
        self.values[level + 1..].fill(0);
        Ok(self.values)
    }
}

/// Shape IDs identify uniquely appended generated first-marker paragraphs.
pub(crate) fn resolve(
    document: &Document,
    wanted: &BTreeSet<u16>,
) -> Result<BTreeMap<u16, String>, String> {
    let mut resolver = Resolver {
        document,
        wanted,
        result: BTreeMap::new(),
        counters: Counters::default(),
    };
    for section in &document.sections {
        let outline = section.section_def.outline_numbering_id;
        resolver.paragraphs(&section.paragraphs, outline, true, 0)?;
        for master in &section.section_def.master_pages {
            resolver.paragraphs(&master.paragraphs, outline, false, 0)?;
        }
    }
    if resolver.result.len() != wanted.len() {
        return Err(
            "Cannot locate every generated list marker for continuation measurement".into(),
        );
    }
    Ok(resolver.result)
}

struct Resolver<'a> {
    document: &'a Document,
    wanted: &'a BTreeSet<u16>,
    result: BTreeMap<u16, String>,
    counters: Counters,
}

impl Resolver<'_> {
    fn paragraphs(
        &mut self,
        paragraphs: &[Paragraph],
        outline: u16,
        body: bool,
        depth: usize,
    ) -> Result<(), String> {
        if depth > 128 {
            return Err("Native list marker container nesting exceeds the supported limit".into());
        }
        for paragraph in paragraphs {
            let wanted = self.wanted.contains(&paragraph.para_shape_id);
            if wanted && !body {
                return Err("List continuation measurement in captions, headers, footers, master pages, comments or notes is unsupported".into());
            }
            if body {
                self.marker(paragraph, outline, wanted)?;
            }
            for control in &paragraph.controls {
                match control {
                    Control::Table(table) => {
                        for cell in &table.cells {
                            self.paragraphs(&cell.paragraphs, outline, body, depth + 1)?;
                        }
                        self.caption(&table.caption, outline, depth + 1)?;
                    }
                    Control::Shape(shape) => self.shape(shape, outline, body, depth + 1)?,
                    Control::Picture(picture) => {
                        self.caption(&picture.caption, outline, depth + 1)?
                    }
                    Control::SectionDef(section) => {
                        for master in &section.master_pages {
                            self.paragraphs(&master.paragraphs, outline, false, depth + 1)?;
                        }
                    }
                    Control::Header(header) => {
                        self.paragraphs(&header.paragraphs, outline, false, depth + 1)?
                    }
                    Control::Footer(footer) => {
                        self.paragraphs(&footer.paragraphs, outline, false, depth + 1)?
                    }
                    Control::Footnote(note) => {
                        self.paragraphs(&note.paragraphs, outline, false, depth + 1)?
                    }
                    Control::Endnote(note) => {
                        self.paragraphs(&note.paragraphs, outline, false, depth + 1)?
                    }
                    Control::HiddenComment(comment) => {
                        self.paragraphs(&comment.paragraphs, outline, false, depth + 1)?
                    }
                    Control::Field(field) => {
                        self.paragraphs(&field.memo_paragraphs, outline, false, depth + 1)?
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    fn caption(
        &mut self,
        caption: &Option<Caption>,
        outline: u16,
        depth: usize,
    ) -> Result<(), String> {
        if let Some(caption) = caption {
            self.paragraphs(&caption.paragraphs, outline, false, depth)?;
        }
        Ok(())
    }

    fn drawing(
        &mut self,
        drawing: &DrawingObjAttr,
        outline: u16,
        body: bool,
        depth: usize,
    ) -> Result<(), String> {
        if let Some(text) = &drawing.text_box {
            self.paragraphs(&text.paragraphs, outline, body, depth)?;
        }
        self.caption(&drawing.caption, outline, depth)
    }

    fn shape(
        &mut self,
        shape: &ShapeObject,
        outline: u16,
        body: bool,
        depth: usize,
    ) -> Result<(), String> {
        if depth > 128 {
            return Err("Native list marker drawing nesting exceeds the supported limit".into());
        }
        match shape {
            ShapeObject::Group(group) => {
                for child in &group.children {
                    self.shape(child, outline, body, depth + 1)?;
                }
                self.caption(&group.caption, outline, depth)?;
            }
            ShapeObject::Picture(picture) => self.caption(&picture.caption, outline, depth)?,
            ShapeObject::Chart(chart) => {
                self.caption(&chart.caption, outline, depth)?;
                self.drawing(&chart.drawing, outline, body, depth)?;
            }
            ShapeObject::Ole(ole) => {
                self.caption(&ole.caption, outline, depth)?;
                self.drawing(&ole.drawing, outline, body, depth)?;
                if let Some(fallback) = &ole.chart_switch_fallback {
                    // Fallback drawings are an alternative representation, not
                    // another position in the native numbering flow.
                    self.caption(&fallback.caption, outline, depth)?;
                    self.drawing(&fallback.drawing, outline, false, depth)?;
                }
            }
            _ => {
                if let Some(drawing) = shape.drawing() {
                    self.drawing(drawing, outline, body, depth)?;
                }
            }
        }
        Ok(())
    }

    fn marker(&mut self, paragraph: &Paragraph, outline: u16, wanted: bool) -> Result<(), String> {
        let shape = self
            .document
            .doc_info
            .para_shapes
            .get(usize::from(paragraph.para_shape_id))
            .ok_or("Native list paragraph references an absent paragraph format")?;
        let display = match shape.head_type {
            HeadType::Bullet if wanted => {
                let bullet = shape
                    .numbering_id
                    .checked_sub(1)
                    .and_then(|id| self.document.doc_info.bullets.get(usize::from(id)))
                    .ok_or("Native list marker references an absent bullet definition")?;
                if bullet.image_bullet != 0 {
                    return Err("Image bullet continuation measurement is unsupported; use the Hancom backend".into());
                }
                let ch = bullet.bullet_char;
                if matches!(u32::from(ch), 0xe000..=0xf8ff | 0xf0000..=0xffffd | 0x100000..=0x10fffd)
                {
                    return Err("Private-use bullet glyph measurement is unsupported; use an ordinary Unicode bullet with an installed TrueType/OpenType font or the Hancom backend".into());
                }
                ch.to_string()
            }
            HeadType::Number | HeadType::Outline => {
                let id = if shape.numbering_id == 0 && shape.head_type == HeadType::Outline {
                    outline
                } else {
                    shape.numbering_id
                };
                let definition = id
                    .checked_sub(1)
                    .and_then(|id| self.document.doc_info.numberings.get(usize::from(id)));
                if definition.is_none() && shape.head_type == HeadType::Number {
                    if wanted {
                        return Err(
                            "Native list marker references an absent numbering definition".into(),
                        );
                    }
                    return Ok(());
                }
                let level = usize::from(shape.para_level);
                if level > 6 {
                    return Err("Native list marker level exceeds seven levels".into());
                }
                let counters = self
                    .counters
                    .advance(id, level, paragraph.numbering_restart)?;
                if !wanted {
                    return Ok(());
                }
                let definition = definition
                    .ok_or("Generated list marker requires a native numbering definition")?;
                expand(definition, &counters, level)?
            }
            _ if wanted => {
                return Err(
                    "Generated continuation marker paragraph has no native list marker".into(),
                );
            }
            _ => return Ok(()),
        };
        if display.trim().is_empty() || display.chars().any(char::is_control) {
            return Err(
                "Native list marker is empty or contains unsupported control characters".into(),
            );
        }
        if self
            .result
            .insert(paragraph.para_shape_id, display)
            .is_some()
        {
            return Err("Generated continuation marker paragraph identity is not unique".into());
        }
        Ok(())
    }
}

fn value(definition: &Numbering, counters: &[u32; 7], level: usize) -> Result<u16, String> {
    let start = definition.level_start_numbers[level];
    let value = if counters[level] == 0 {
        start
    } else {
        start
            .saturating_sub(1)
            .checked_add(counters[level])
            .ok_or("Native displayed list number overflow")?
    };
    u16::try_from(value).map_err(|_| "Native displayed list number exceeds 65535".into())
}

fn number(definition: &Numbering, counters: &[u32; 7], level: usize) -> Result<String, String> {
    let value = value(definition, counters, level)?;
    // Paragraph numbering codes are not NumberFormat::from_hwp_format codes.
    let code = definition.heads[level].number_format;
    if matches!(code, 1) && !(1..=20).contains(&value)
        || matches!(code, 2 | 3) && !(1..=3999).contains(&value)
        || matches!(code, 8 | 10) && !(1..=14).contains(&value)
        || matches!(code, 4 | 5) && value == 0
    {
        return Err(format!(
            "Native list number {value} is outside the supported range for format {code}; use the Hancom backend"
        ));
    }
    let format = match code {
        0 => NumberFormat::Digit,
        1 => NumberFormat::CircledDigit,
        2 => NumberFormat::RomanUpper,
        3 => NumberFormat::RomanLower,
        4 => NumberFormat::LatinUpper,
        5 => NumberFormat::LatinLower,
        8 => NumberFormat::HangulGaNaDa,
        10 => NumberFormat::HangulJamo,
        12 => NumberFormat::HangulNumber,
        13 => NumberFormat::HanjaNumber,
        code => {
            return Err(format!(
                "Native list number format {code} is unsupported for continuation measurement; use the Hancom backend"
            ));
        }
    };
    Ok(format_number(value, format))
}

fn expand(definition: &Numbering, counters: &[u32; 7], level: usize) -> Result<String, String> {
    let mut result = String::new();
    let mut chars = definition.level_formats[level].chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '^' {
            match chars.peek().copied() {
                Some(digit @ '1'..='7') => {
                    chars.next();
                    result.push_str(&number(
                        definition,
                        counters,
                        (digit as u8 - b'1') as usize,
                    )?);
                    continue;
                }
                Some(code @ ('n' | 'N')) => {
                    chars.next();
                    for index in 0..=level {
                        if index > 0 {
                            result.push('.');
                        }
                        // Hancom expands the aggregate path in decimal even
                        // when explicit ^1..^7 slots use Roman/alphabetic forms.
                        result.push_str(&value(definition, counters, index)?.to_string());
                    }
                    if code == 'N' {
                        result.push('.');
                    }
                    continue;
                }
                Some('0' | '8' | '9') => {
                    return Err(
                        "Native list format references an unsupported numbering level".into(),
                    );
                }
                _ => {}
            }
        }
        result.push(ch);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_actual_generated_paragraph_order_and_rejects_missing_targets() {
        use rhwp::model::{document::Section, style::ParaShape};
        let mut definition = Numbering {
            level_start_numbers: [1; 7],
            ..Default::default()
        };
        definition.level_formats[0] = "(^1)".into();
        let mut document = Document::default();
        document.doc_info.numberings.push(definition);
        document.doc_info.para_shapes = (0..11)
            .map(|_| ParaShape {
                head_type: HeadType::Number,
                numbering_id: 1,
                ..Default::default()
            })
            .collect();
        document.sections.push(Section {
            paragraphs: (0..11)
                .map(|id| {
                    let mut paragraph = Paragraph::new_empty();
                    paragraph.para_shape_id = id;
                    paragraph
                })
                .collect(),
            ..Default::default()
        });
        let result = resolve(&document, &BTreeSet::from([8, 9, 10])).unwrap();
        assert_eq!(result[&8], "(9)");
        assert_eq!(result[&9], "(10)");
        assert_eq!(result[&10], "(11)");
        assert!(resolve(&document, &BTreeSet::from([12])).is_err());
    }

    #[test]
    fn counters_inherit_restore_and_restart_independent_definitions() {
        let mut state = Counters::default();
        assert_eq!(state.advance(1, 0, None).unwrap(), [1, 0, 0, 0, 0, 0, 0]);
        assert_eq!(state.advance(2, 1, None).unwrap(), [1, 1, 0, 0, 0, 0, 0]);
        assert_eq!(state.advance(2, 1, None).unwrap(), [1, 2, 0, 0, 0, 0, 0]);
        assert_eq!(state.advance(3, 0, None).unwrap(), [1, 0, 0, 0, 0, 0, 0]);
        assert_eq!(
            state
                .advance(2, 1, Some(NumberingRestart::ContinuePrevious))
                .unwrap(),
            [1, 3, 0, 0, 0, 0, 0]
        );
        assert_eq!(
            state
                .advance(2, 1, Some(NumberingRestart::NewStart(9)))
                .unwrap(),
            [1, 9, 0, 0, 0, 0, 0]
        );
        let mut definition = Numbering {
            level_start_numbers: [7, 99, 1, 1, 1, 1, 1],
            ..Default::default()
        };
        definition.level_formats[1] = "^1.^2".into();
        assert_eq!(
            expand(&definition, &[1, 2, 0, 0, 0, 0, 0], 1).unwrap(),
            "7.100"
        );
    }

    #[test]
    fn template_formats_preserve_affixes_and_use_each_referenced_level_format() {
        let mut definition = Numbering {
            level_start_numbers: [1; 7],
            ..Default::default()
        };
        definition.heads[0].number_format = 2;
        definition.heads[1].number_format = 8;
        definition.level_formats[1] = "제 ^1장 (^2) ^n / ^N".into();
        assert_eq!(
            expand(&definition, &[4, 2, 0, 0, 0, 0, 0], 1).unwrap(),
            "제 IV장 (나) 4.2 / 4.2."
        );
        definition.heads[1].number_format = 10;
        assert_eq!(
            number(&definition, &[4, 2, 0, 0, 0, 0, 0], 1).unwrap(),
            "ㄴ"
        );
        definition.heads[1].number_format = 7;
        assert!(expand(&definition, &[4, 2, 0, 0, 0, 0, 0], 1).is_err());
    }

    #[test]
    fn measured_formats_never_silently_fall_back_to_decimal() {
        let mut definition = Numbering {
            level_start_numbers: [1; 7],
            ..Default::default()
        };
        for (code, count) in [(1, 21), (2, 4000), (3, 4000), (8, 15), (10, 15)] {
            definition.heads[0].number_format = code;
            assert!(number(&definition, &[count, 0, 0, 0, 0, 0, 0], 0).is_err());
        }
        definition.heads[0].number_format = 0;
        assert_eq!(
            number(&definition, &[100, 0, 0, 0, 0, 0, 0], 0).unwrap(),
            "100"
        );
        definition.heads[0].number_format = 4;
        assert_eq!(
            number(&definition, &[27, 0, 0, 0, 0, 0, 0], 0).unwrap(),
            "AA"
        );
    }
}
