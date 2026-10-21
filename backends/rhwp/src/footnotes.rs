// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Euisoon Ahn
//! Capture native note prototypes before disposable template definitions are removed.

use std::collections::BTreeSet;

use md2hwp_core::ir::{FootnoteBlock, Inline};
use rhwp::model::{
    control::{AutoNumber, AutoNumberType, Control},
    document::{DocInfo, Document},
    footnote::{Footnote, FootnoteNumbering, FootnoteShape, NumberFormat},
    paragraph::{ColumnBreakType, Paragraph},
};

const FIRST: &str = "{{md2hwp:footnote}}";
const NEXT: &str = "{{md2hwp:footnote.next}}";

pub(crate) struct Sample {
    first: Paragraph,
    next: Option<Paragraph>,
    native: Option<Footnote>,
    root: usize,
    next_root: Option<usize>,
}

impl Sample {
    pub(crate) fn read(roots: &[Paragraph], info: &DocInfo) -> Result<Self, String> {
        let boundary = |tag: &str| -> Result<usize, String> {
            let indices: Vec<_> = roots
                .iter()
                .enumerate()
                .filter(|(_, p)| p.text == tag)
                .map(|(i, _)| i)
                .collect();
            if indices.len() != 1 {
                return Err(format!("Expected exactly one {tag}"));
            }
            Ok(indices[0])
        };
        let begin = boundary("{{md2hwp:begin:template}}")?;
        let end = boundary("{{md2hwp:end:template}}")?;
        let mut found = Vec::new();
        let mut nexts = Vec::new();
        // Count every container too, so a misplaced duplicate cannot escape validation.
        fn count(p: &Paragraph, first: &mut usize, next: &mut usize) -> Result<(), String> {
            *first += p.text.matches(FIRST).count();
            *next += p.text.matches(NEXT).count();
            crate::model_tools::map_nested_containers(&mut p.clone(), &mut |ps| {
                for p in ps {
                    count(p, first, next)?;
                }
                Ok(())
            })
        }
        let (mut first_count, mut next_count) = (0, 0);
        for (i, p) in roots.iter().enumerate() {
            count(p, &mut first_count, &mut next_count)?;
            if p.text.contains(FIRST) {
                found.push((i, p.clone(), None));
            }
            if p.text.contains(NEXT) {
                nexts.push((Some(i), p.clone()));
            }
            for control in &p.controls {
                if let Control::Footnote(note) = control
                    && let Some(first) = note.paragraphs.first().filter(|p| p.text.contains(FIRST))
                {
                    plain_anchor(p)?;
                    if note.paragraphs.len() > 2 {
                        return Err("Native footnote sample allows at most two paragraphs".into());
                    }
                    found.push((i, first.clone(), Some((**note).clone())));
                    if let Some(next) = note.paragraphs.get(1) {
                        if !next.text.contains(NEXT) {
                            return Err(
                                "Second native sample paragraph requires footnote.next".into()
                            );
                        }
                        nexts.push((None, next.clone()));
                    }
                }
            }
        }
        if first_count != 1 || found.len() != 1 || next_count > 1 || nexts.len() != next_count {
            return Err("Expected one footnote sample and at most one footnote.next in template definitions; regenerate or explicitly adopt the declarations".into());
        }
        let (root, first, native) = found.pop().unwrap();
        if root <= begin || root >= end {
            return Err("Footnote sample must be inside template definitions".into());
        }
        validate_paragraph(&first, FIRST, info, native.is_some())?;
        if native.is_some() {
            if first.text.trim() != FIRST || first.controls.len() != 1 {
                return Err("Native footnote first sample requires only whitespace, tabs and one Footnote AUTONUM before its slot".into());
            }
            let Control::AutoNumber(number) = &first.controls[0] else {
                unreachable!()
            };
            let slot = first.text[..first.text.find(FIRST).unwrap()]
                .chars()
                .count();
            if number.number_type != AutoNumberType::Footnote
                || first.control_text_positions()[0] > slot
            {
                return Err(
                    "Native footnote sample requires a Footnote AUTONUM before the slot".into(),
                );
            }
        } else if first.text != FIRST || !first.controls.is_empty() {
            return Err("Plain root footnote sample must contain only its slot".into());
        }
        let next_root = nexts.first().and_then(|(i, _)| *i);
        if let Some(i) = next_root
            && (i <= begin || i >= end)
        {
            return Err("footnote.next must be inside template definitions".into());
        }
        let next = nexts.pop().map(|(_, p)| p);
        if let Some(next) = &next {
            validate_paragraph(next, NEXT, info, true)?;
        }
        Ok(Self {
            first,
            next,
            native,
            root,
            next_root,
        })
    }

    /// The ordinary declaration binder sees a plain body style; retained native
    /// sample controls are used only after manuscript composition is complete.
    pub(crate) fn lower_sample(&self, roots: &[Paragraph]) -> Vec<Paragraph> {
        roots
            .iter()
            .enumerate()
            .filter_map(|(i, p)| {
                if Some(i) == self.next_root {
                    None
                } else if i == self.root {
                    Some(clean(&self.first, FIRST))
                } else {
                    Some(p.clone())
                }
            })
            .collect()
    }

    pub(crate) fn render(
        &self,
        blocks: &[FootnoteBlock],
        info: &mut DocInfo,
        shape: &FootnoteShape,
    ) -> Result<Footnote, String> {
        if blocks.is_empty() {
            return Err("Footnote requires nonempty paragraph blocks".into());
        }
        let mut note = self.native.clone().unwrap_or_default();
        if self.native.is_none() {
            // CTRL_FOOTNOTE's flag is not FOOTNOTESHAPE.attr: its low byte is
            // the numbering format. Unknown upper source flag bytes belong to
            // native samples only and must never be synthesized from options.
            note.number_shape = FootnoteShape::number_format_attr_code(shape.number_format);
            note.before_decoration_letter = u16::try_from(u32::from(shape.prefix_char))
                .map_err(|_| "Footnote prefix must fit a native WCHAR")?;
            note.decoration_is_user_char = shape.number_format == NumberFormat::UserChar;
            let after = if note.decoration_is_user_char {
                shape.user_char
            } else {
                shape.suffix_char
            };
            note.after_decoration_letter = u16::try_from(u32::from(after))
                .map_err(|_| "Footnote suffix or user character must fit a native WCHAR")?;
        }
        note.paragraphs.clear();
        for (i, FootnoteBlock::Paragraph { inlines }) in blocks.iter().enumerate() {
            reject_nested(inlines)?;
            let (mut p, slot) = if i == 0 {
                (self.first.clone(), FIRST)
            } else if let Some(next) = &self.next {
                (next.clone(), NEXT)
            } else {
                (clean(&self.first, FIRST), FIRST)
            };
            let start = p.text[..p.text.find(slot).unwrap()].chars().count();
            let base = p
                .char_shape_id_at(start)
                .ok_or("Footnote slot has no character format")?;
            let mut spans = Vec::new();
            crate::template::flatten(inlines, false, false, &mut spans);
            crate::template::replace_rich(&mut p, slot, &spans, base, info)?;
            if p.text.is_empty() {
                p.set_single_char_shape(base);
            }
            if i == 0 && self.native.is_none() {
                // Native AUTONUM occupies eight HWP units and its reader-visible
                // placeholder is one space; the following space is sample spacing.
                p.insert_text_at(0, "  ");
                for offset in p.char_offsets.iter_mut().skip(1) {
                    *offset += 7;
                }
                for run in &mut p.char_shapes {
                    if run.start_pos > 0 {
                        run.start_pos += 7;
                    }
                }
                p.char_count += 7;
                p.control_mask |= 1 << 18;
                p.controls.push(Control::AutoNumber(AutoNumber {
                    number_type: AutoNumberType::Footnote,
                    format: FootnoteShape::number_format_attr_code(shape.number_format) as u8,
                    superscript: shape.number_code_superscript,
                    assigned_number: 1,
                    number: 1,
                    user_symbol: shape.user_char,
                    prefix_char: shape.prefix_char,
                    suffix_char: shape.suffix_char,
                }));
                p.ctrl_data_records.push(None);
            }
            note.paragraphs.push(p);
        }
        Ok(note)
    }
}

fn reject_nested(inlines: &[Inline]) -> Result<(), String> {
    for inline in inlines {
        match inline {
            Inline::Footnote { .. } => return Err("Nested footnotes are unsupported".into()),
            Inline::CrossReference { .. } => {
                return Err("rhwp cross references remain unsupported".into());
            }
            Inline::Strong { inlines }
            | Inline::Emph { inlines }
            | Inline::Link { inlines, .. } => reject_nested(inlines)?,
            _ => {}
        }
    }
    Ok(())
}

fn plain_anchor(p: &Paragraph) -> Result<(), String> {
    if p.controls.len() != 1
        || p.text.contains("{{md2hwp:")
        || p.column_type != ColumnBreakType::None
        || p.raw_break_type & 0xf != 0
        || !p.field_ranges.is_empty()
        || !p.orphan_field_ends.is_empty()
        || !p.range_tags.is_empty()
        || !p.title_marks.is_empty()
        || !p.markpen_marks.is_empty()
        || p.text.chars().any(char::is_control)
    {
        return Err("Native footnote sample requires one plain root anchor without other controls or breaks".into());
    }
    Ok(())
}

fn validate_paragraph(
    p: &Paragraph,
    slot: &str,
    info: &DocInfo,
    controls: bool,
) -> Result<(), String> {
    if p.text.matches(slot).count() != 1
        || p.text.replace(slot, "").contains("{{md2hwp:")
        || p.column_type != ColumnBreakType::None
        || p.raw_break_type & 0xf != 0
        || !p.field_ranges.is_empty()
        || !p.orphan_field_ends.is_empty()
        || !p.range_tags.is_empty()
        || !p.title_marks.is_empty()
        || !p.markpen_marks.is_empty()
        || p.text.chars().any(|c| c.is_control() && c != '\t')
        || p.controls
            .iter()
            .any(|c| !controls || !matches!(c, Control::AutoNumber(_)))
        || info.para_shapes.get(usize::from(p.para_shape_id)).is_none()
        || p.char_shapes.is_empty()
        || p.char_shapes
            .iter()
            .any(|s| info.char_shapes.get(s.char_shape_id as usize).is_none())
    {
        return Err("Footnote samples require one slot, valid formats and only permitted native tabs/AUTONUM affixes, without breaks or other controls".into());
    }
    crate::template::replace(&mut p.clone(), slot, "")?;
    Ok(())
}

fn clean(sample: &Paragraph, text: &str) -> Paragraph {
    let slot = if sample.text.contains(FIRST) {
        FIRST
    } else {
        NEXT
    };
    let offset = sample.text[..sample.text.find(slot).unwrap()]
        .chars()
        .count();
    let base = sample.char_shape_id_at(offset).unwrap();
    let mut p = sample.clone();
    p.text.clear();
    p.char_offsets.clear();
    p.char_count = 1;
    p.control_mask = 0;
    p.controls.clear();
    p.ctrl_data_records.clear();
    p.tab_extended.clear();
    p.set_single_char_shape(base);
    p.insert_text_at(0, text);
    crate::template::discard_stale_layout(&mut p);
    p
}

/// Assign actual document-order values to generated note headers and their
/// first-body AUTONUM only. Native OnPage layout will restart display numbers;
/// page membership cannot be inferred from invalidated paragraph caches here.
pub(crate) fn assign_numbers(
    document: &mut Document,
    generated: &BTreeSet<u32>,
) -> Result<(), String> {
    let mut next = u32::from(document.doc_properties.footnote_start_num.max(1));
    let mut found = BTreeSet::new();
    for section in &mut document.sections {
        // A retained SECDEF is encountered at its true stream position. The
        // modeled section definition is the fallback when no root owns one.
        if !section.paragraphs.iter().any(|p| {
            p.controls
                .iter()
                .any(|c| matches!(c, Control::SectionDef(_)))
        }) && section.section_def.footnote_shape.numbering == FootnoteNumbering::RestartSection
        {
            next = u32::from(section.section_def.footnote_shape.start_number.max(1));
        }
        number_paragraphs(
            &mut section.paragraphs,
            &mut next,
            generated,
            &mut found,
            false,
        )?;
    }
    if found != *generated {
        return Err("A generated footnote was lost before native numbering".into());
    }
    Ok(())
}

fn number_paragraphs(
    paragraphs: &mut [Paragraph],
    next: &mut u32,
    generated: &BTreeSet<u32>,
    found: &mut BTreeSet<u32>,
    continuation: bool,
) -> Result<(), String> {
    for paragraph in paragraphs {
        for control in &mut paragraph.controls {
            match control {
                Control::Header(_) | Control::Footer(_) => {}
                Control::SectionDef(section) => {
                    if section.footnote_shape.numbering == FootnoteNumbering::RestartSection {
                        *next = u32::from(section.footnote_shape.start_number.max(1));
                    }
                    // Master pages never participate in manuscript counters.
                }
                Control::NewNumber(number) if number.number_type == AutoNumberType::Footnote => {
                    *next = u32::from(number.number)
                }
                Control::AutoNumber(number)
                    if !continuation && number.number_type == AutoNumberType::Footnote =>
                {
                    *next = next.checked_add(1).ok_or("Footnote counter overflow")?;
                }
                Control::AutoNumber(_) | Control::NewNumber(_) => {}
                Control::Footnote(note) => {
                    let is_generated = generated.contains(&note.instance_id);
                    if is_generated && !found.insert(note.instance_id) {
                        return Err("Generated notes share a native numbering identity".into());
                    }
                    if is_generated {
                        let first = note
                            .paragraphs
                            .first()
                            .ok_or("Generated footnote has no first body paragraph")?;
                        let count = first.controls.iter().filter(|c| matches!(c, Control::AutoNumber(n) if n.number_type == AutoNumberType::Footnote)).count();
                        if count != 1 {
                            return Err(
                                "Generated footnote requires one first-body Footnote AUTONUM"
                                    .into(),
                            );
                        }
                    }
                    for (index, paragraph) in note.paragraphs.iter_mut().enumerate() {
                        if index == 0 && is_generated {
                            // Process controls in native order, including an
                            // explicit restart immediately before the counter.
                            for child in &mut paragraph.controls {
                                if let Control::AutoNumber(number) = child
                                    && number.number_type == AutoNumberType::Footnote
                                {
                                    let value = u16::try_from(*next).map_err(|_| "Generated footnote number exceeds the native UINT16 range")?;
                                    number.number = value;
                                    number.assigned_number = value;
                                    note.number = value;
                                }
                                number_control(child, next, generated, found, false)?;
                            }
                        } else {
                            number_paragraphs(
                                std::slice::from_mut(paragraph),
                                next,
                                generated,
                                found,
                                index > 0,
                            )?;
                        }
                    }
                }
                _ => number_control(control, next, generated, found, continuation)?,
            }
        }
    }
    Ok(())
}

fn number_control(
    control: &mut Control,
    next: &mut u32,
    generated: &BTreeSet<u32>,
    found: &mut BTreeSet<u32>,
    continuation: bool,
) -> Result<(), String> {
    // Reuse the shared visitor for table cells, captions, drawing text boxes and
    // grouped drawings while retaining each control's position in the stream.
    let mut holder = Paragraph {
        controls: vec![control.clone()],
        ..Default::default()
    };
    match control {
        Control::NewNumber(_)
        | Control::AutoNumber(_)
        | Control::Footnote(_)
        | Control::Header(_)
        | Control::Footer(_)
        | Control::SectionDef(_) => {
            number_paragraphs(
                std::slice::from_mut(&mut holder),
                next,
                generated,
                found,
                continuation,
            )?;
        }
        _ => crate::model_tools::map_nested_containers(&mut holder, &mut |paragraphs| {
            number_paragraphs(paragraphs, next, generated, found, continuation)
        })?,
    }
    *control = holder.controls.remove(0);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rhwp::model::{
        document::Section,
        style::{CharShape, Font, ParaShape},
    };

    fn paragraph(text: &str) -> Paragraph {
        let mut p = Paragraph::new_empty();
        p.set_single_char_shape(0);
        p.insert_text_at(0, text);
        p
    }
    fn fixture() -> (Vec<Paragraph>, DocInfo) {
        let info = DocInfo {
            char_shapes: vec![CharShape::default(), CharShape::default()],
            para_shapes: vec![ParaShape::default()],
            font_faces: vec![vec![Font::default()]; 7],
            ..Default::default()
        };
        (
            vec![
                paragraph("{{md2hwp:begin:template}}"),
                paragraph(FIRST),
                paragraph("{{md2hwp:end:template}}"),
            ],
            info,
        )
    }
    fn blocks() -> Vec<FootnoteBlock> {
        vec![
            FootnoteBlock::Paragraph {
                inlines: vec![Inline::Strong {
                    inlines: vec![Inline::Text {
                        value: "각주😀".into(),
                    }],
                }],
            },
            FootnoteBlock::Paragraph {
                inlines: vec![Inline::Link {
                    target: "https://example.org".into(),
                    title: None,
                    inlines: vec![Inline::Text {
                        value: "계속".into(),
                    }],
                }],
            },
        ]
    }
    #[test]
    fn plain_sample_uses_section_number_format_and_continuations_do_not_repeat_number() {
        let (roots, mut info) = fixture();
        let shape = FootnoteShape {
            prefix_char: '[',
            suffix_char: ']',
            number_code_superscript: true,
            ..Default::default()
        };
        let sample = Sample::read(&roots, &info).unwrap();
        let note = sample.render(&blocks(), &mut info, &shape).unwrap();
        let Control::AutoNumber(number) = &note.paragraphs[0].controls[0] else {
            panic!()
        };
        assert_eq!(number.prefix_char, '[');
        assert_eq!(number.suffix_char, ']');
        assert!(number.superscript);
        assert_eq!(note.paragraphs[0].text, "  각주😀");
        assert_eq!(note.paragraphs[0].char_offsets[1], 8);
        assert_eq!(note.paragraphs[1].text, "계속");
        assert!(note.paragraphs[1].controls.is_empty());
        assert_eq!(roots[1].text, FIRST);
    }

    #[test]
    fn plain_header_uses_native_format_code_and_exact_section_decorations() {
        use rhwp::parser::{record::Record, tags};
        let (roots, mut info) = fixture();
        let sample = Sample::read(&roots, &info).unwrap();
        for (format, code, suffix, user, expected_after) in [
            (NumberFormat::UpperRoman, 2_u32, ']', '\0', ']'),
            (NumberFormat::UserChar, 0x81, '\0', '※', '※'),
            (NumberFormat::Digit, 0, '\0', '\0', '\0'),
        ] {
            let shape = FootnoteShape {
                number_format: format,
                prefix_char: '[',
                suffix_char: suffix,
                user_char: user,
                number_code_superscript: true,
                numbering: FootnoteNumbering::RestartPage,
                ..Default::default()
            };
            let note = sample.render(&blocks(), &mut info, &shape).unwrap();
            assert_eq!(note.number_shape, code);
            assert_eq!(note.before_decoration_letter, '[' as u16);
            assert_eq!(note.after_decoration_letter, expected_after as u16);
            assert_eq!(
                note.decoration_is_user_char,
                format == NumberFormat::UserChar
            );
            let Control::AutoNumber(body_number) = &note.paragraphs[0].controls[0] else {
                panic!()
            };
            assert_eq!(u32::from(body_number.format), code);
            assert_eq!(body_number.user_symbol, user);
            let mut records = Vec::new();
            rhwp::serializer::body_text::serialize_paragraph_list(
                &[Paragraph {
                    controls: vec![Control::Footnote(Box::new(note))],
                    ..Default::default()
                }],
                0,
                &mut records,
            );
            let header = records
                .iter()
                .find(|record: &&Record| {
                    record.tag_id == tags::HWPTAG_CTRL_HEADER
                        && record.data.get(..4)
                            == Some(tags::CTRL_FOOTNOTE.to_le_bytes().as_slice())
                })
                .unwrap();
            assert_eq!(&header.data[8..10], &('[' as u16).to_le_bytes());
            assert_eq!(&header.data[10..12], &(expected_after as u16).to_le_bytes());
            assert_eq!(&header.data[12..16], &code.to_le_bytes());
        }
        let invalid = FootnoteShape {
            prefix_char: '😀',
            ..Default::default()
        };
        assert!(sample.render(&blocks(), &mut info, &invalid).is_err());
    }
    #[test]
    fn native_sample_preserves_tabs_number_format_and_continuation_affixes_after_reparse() {
        let (mut roots, mut info) = fixture();
        let plain = Sample::read(&roots, &info).unwrap();
        let mut note = plain
            .render(
                &[FootnoteBlock::Paragraph {
                    inlines: vec![Inline::Text {
                        value: FIRST.into(),
                    }],
                }],
                &mut info,
                &FootnoteShape::default(),
            )
            .unwrap();
        let first = &mut note.paragraphs[0];
        first.insert_text_at(2, "\t");
        first.tab_extended = vec![[123, 4, 5, 6, 7, 8, 9]];
        let slot_start = first.text[..first.text.find(FIRST).unwrap()]
            .chars()
            .count();
        first.apply_char_shape_range(slot_start, slot_start + 1, 1);
        let Control::AutoNumber(number) = &mut first.controls[0] else {
            panic!()
        };
        number.suffix_char = ']';
        number.superscript = true;
        let mut next = paragraph(&format!("접두\t{NEXT} 꼬리"));
        next.tab_extended = vec![[432, 1, 2, 3, 4, 5, 6]];
        note.list_header_property = 12;
        note.number_shape = 0x0031_0003;
        note.before_decoration_letter = '문' as u16;
        note.after_decoration_letter = '）' as u16;
        note.paragraphs.push(next);
        let mut anchor = paragraph("sample");
        for offset in &mut anchor.char_offsets {
            *offset += 8;
        }
        anchor.char_count += 8;
        anchor.controls.push(Control::Footnote(Box::new(note)));
        roots[1] = anchor;
        let original = serde_json::to_value(&roots).unwrap();
        let sample = Sample::read(&roots, &info).unwrap();
        assert_eq!(sample.lower_sample(&roots)[1].text, FIRST);
        let generated = sample
            .render(&blocks(), &mut info, &FootnoteShape::default())
            .unwrap();
        assert_eq!(generated.list_header_property, 12);
        assert_eq!(generated.number_shape, 0x0031_0003);
        assert_eq!(generated.before_decoration_letter, '문' as u16);
        assert_eq!(generated.after_decoration_letter, '）' as u16);
        assert_eq!(
            generated.paragraphs[0].tab_extended[0],
            [123, 4, 5, 6, 7, 8, 9]
        );
        assert_eq!(generated.paragraphs[1].text, "접두\t계속 꼬리");
        assert_eq!(
            generated.paragraphs[1].tab_extended[0],
            [432, 1, 2, 3, 4, 5, 6]
        );
        let base = generated.paragraphs[0].char_shape_id_at(3).unwrap();
        assert!(info.char_shapes[base as usize].bold);
        assert_eq!(serde_json::to_value(&roots).unwrap(), original);
        let mut output = paragraph("본문");
        for offset in &mut output.char_offsets {
            *offset += 8;
        }
        output.char_count += 8;
        output.controls.push(Control::Footnote(Box::new(generated)));
        let mut core = rhwp::DocumentCore::new_empty();
        let mut doc = core.document().clone();
        doc.doc_info = info;
        doc.sections = vec![Section {
            paragraphs: vec![output],
            ..Default::default()
        }];
        core.set_document(doc);
        let result = core.export_hwp_with_adapter_snapshot_with_report().unwrap();
        assert!(result.content_loss().is_empty());
        let reopened = rhwp::DocumentCore::from_bytes(result.bytes()).unwrap();
        let Control::Footnote(actual) = &reopened.document().sections[0].paragraphs[0].controls[0]
        else {
            panic!()
        };
        assert_eq!(actual.list_header_property, 12);
        assert_eq!(actual.number_shape, 0x0031_0003);
        assert_eq!(actual.before_decoration_letter, '문' as u16);
        assert_eq!(actual.after_decoration_letter, '）' as u16);
        assert_eq!(actual.paragraphs[1].text, "접두\t계속 꼬리");
        assert_eq!(
            actual.paragraphs[0].tab_extended[0],
            [123, 4, 5, 6, 7, 8, 9]
        );
        let Control::AutoNumber(number) = &actual.paragraphs[0].controls[0] else {
            panic!()
        };
        assert_eq!(number.suffix_char, ']');
        assert!(number.superscript);
    }
    #[test]
    fn rejects_duplicate_slots_controls_inside_slots_and_nested_notes() {
        let (mut roots, mut info) = fixture();
        roots.insert(2, paragraph(FIRST));
        assert!(Sample::read(&roots, &info).is_err());
        roots.remove(2);
        roots[1].char_offsets[5] += 8;
        assert!(Sample::read(&roots, &info).is_err());
        roots[1] = paragraph(FIRST);
        let sample = Sample::read(&roots, &info).unwrap();
        let nested = vec![FootnoteBlock::Paragraph {
            inlines: vec![Inline::Footnote { blocks: blocks() }],
        }];
        assert!(
            sample
                .render(&nested, &mut info, &FootnoteShape::default())
                .is_err()
        );
    }

    fn numbered_note(id: u32, value: u16, continuation: Option<u16>) -> Control {
        let number = |value| {
            Control::AutoNumber(AutoNumber {
                number_type: AutoNumberType::Footnote,
                number: value,
                assigned_number: value,
                suffix_char: ']',
                ..Default::default()
            })
        };
        let mut body = paragraph("note");
        body.controls.push(number(value));
        let mut paragraphs = vec![body];
        if let Some(value) = continuation {
            let mut next = paragraph("continuation");
            next.controls.push(number(value));
            paragraphs.push(next);
        }
        Control::Footnote(Box::new(Footnote {
            instance_id: id,
            number: value,
            paragraphs,
            ..Default::default()
        }))
    }

    #[test]
    fn generated_numbers_respect_starts_restarts_and_static_notes_but_skip_continuation_affixes() {
        use rhwp::model::{
            control::NewNumber,
            table::{Cell, Table},
        };
        for mode in [
            FootnoteNumbering::Continue,
            FootnoteNumbering::RestartSection,
            FootnoteNumbering::RestartPage,
        ] {
            let mut document = rhwp::DocumentCore::new_empty().document().clone();
            document.doc_properties.footnote_start_num = 7;
            let mut section = Section::default();
            section.section_def.footnote_shape.numbering = mode;
            section.section_def.footnote_shape.start_number = 20;
            let mut root = paragraph("body");
            root.controls = vec![
                numbered_note(100, 91, Some(92)),
                numbered_note(42, 1, Some(77)),
                Control::Table(Box::new(Table {
                    cells: vec![Cell {
                        paragraphs: vec![Paragraph {
                            controls: vec![
                                Control::NewNumber(NewNumber {
                                    number_type: AutoNumberType::Footnote,
                                    number: 12,
                                }),
                                numbered_note(43, 1, None),
                            ],
                            ..Default::default()
                        }],
                        ..Default::default()
                    }],
                    ..Default::default()
                })),
            ];
            section.paragraphs = vec![root];
            document.sections = vec![section];
            assign_numbers(&mut document, &BTreeSet::from([42, 43])).unwrap();
            let controls = &document.sections[0].paragraphs[0].controls;
            let Control::Footnote(static_note) = &controls[0] else {
                panic!()
            };
            assert_eq!(static_note.number, 91);
            let Control::Footnote(first) = &controls[1] else {
                panic!()
            };
            let expected = if mode == FootnoteNumbering::RestartSection {
                21
            } else {
                8
            };
            assert_eq!(first.number, expected);
            let Control::AutoNumber(number) = &first.paragraphs[0].controls[0] else {
                panic!()
            };
            assert_eq!(
                (number.number, number.assigned_number),
                (expected, expected)
            );
            assert_eq!(number.suffix_char, ']');
            let Control::AutoNumber(continuation) = &first.paragraphs[1].controls[0] else {
                panic!()
            };
            assert_eq!(
                (continuation.number, continuation.assigned_number),
                (77, 77)
            );
            let Control::Table(table) = &controls[2] else {
                panic!()
            };
            let Control::Footnote(restarted) = &table.cells[0].paragraphs[0].controls[1] else {
                panic!()
            };
            assert_eq!(restarted.number, 12);
        }
    }

    #[test]
    fn numbering_uses_native_section_definition_position_and_excludes_headers() {
        use rhwp::model::{document::SectionDef, header_footer::Header};
        let mut document = rhwp::DocumentCore::new_empty().document().clone();
        document.doc_properties.footnote_start_num = 7;
        let mut definition = SectionDef::default();
        definition.footnote_shape.numbering = FootnoteNumbering::RestartSection;
        definition.footnote_shape.start_number = 30;
        document.sections = vec![Section {
            paragraphs: vec![Paragraph {
                controls: vec![
                    numbered_note(42, 1, None),
                    Control::Header(Box::new(Header {
                        paragraphs: vec![Paragraph {
                            controls: vec![numbered_note(100, 91, None)],
                            ..Default::default()
                        }],
                        ..Default::default()
                    })),
                    Control::SectionDef(Box::new(definition)),
                    numbered_note(43, 1, None),
                ],
                ..Default::default()
            }],
            ..Default::default()
        }];
        assign_numbers(&mut document, &BTreeSet::from([42, 43])).unwrap();
        let controls = &document.sections[0].paragraphs[0].controls;
        let Control::Footnote(first) = &controls[0] else {
            panic!()
        };
        let Control::Footnote(second) = &controls[3] else {
            panic!()
        };
        assert_eq!((first.number, second.number), (7, 30));
        assert!(assign_numbers(&mut document, &BTreeSet::from([999])).is_err());
    }
}
