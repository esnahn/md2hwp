// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Euisoon Ahn
//! Conditional ranges operate on native paragraph containers, never flattened text.

use rhwp::model::{
    control::Control,
    paragraph::{ColumnBreakType, Paragraph},
};

const PREFIX: &str = "{{md2hwp:";
const TOKENS: [&str; 4] = [
    "{{md2hwp:begin:once}}",
    "{{md2hwp:end:once}}",
    "{{md2hwp:begin:except.once}}",
    "{{md2hwp:end:except.once}}",
];

#[derive(Clone, Copy)]
enum Part {
    Character(usize, char),
    Control(usize),
    Boundary,
}
#[derive(Clone, Copy)]
struct Atom {
    paragraph: usize,
    part: Part,
}

pub(crate) fn apply(mut paragraphs: Vec<Paragraph>, first: bool) -> Result<Vec<Paragraph>, String> {
    let mut atoms = Vec::new();
    for (index, paragraph) in paragraphs.iter().enumerate() {
        if index > 0 {
            atoms.push(Atom {
                paragraph: index,
                part: Part::Boundary,
            });
        }
        let chars: Vec<_> = paragraph.text.chars().collect();
        let mut positions = paragraph.control_text_positions();
        // rhwp's text-free fallback counts inline objects as cursor positions;
        // they still all sit before the paragraph terminator in this text axis.
        if chars.is_empty() {
            positions.fill(0);
        }
        if positions.iter().any(|&position| position > chars.len()) {
            return Err(
                "Conditional ranges require native control positions within their paragraph".into(),
            );
        }
        for position in 0..=chars.len() {
            for (control, _) in positions
                .iter()
                .enumerate()
                .filter(|(_, p)| **p == position)
            {
                atoms.push(Atom {
                    paragraph: index,
                    part: Part::Control(control),
                });
            }
            if let Some(&character) = chars.get(position) {
                atoms.push(Atom {
                    paragraph: index,
                    part: Part::Character(position, character),
                });
            }
        }
    }
    let matches = |start: usize, token: &str| {
        token.chars().enumerate().all(|(offset, expected)| {
            matches!(
            atoms.get(start + offset).map(|a| a.part), Some(Part::Character(_, c)) if c == expected)
        })
    };
    let mut keep = vec![true; atoms.len()];
    let mut marker_paragraphs = vec![false; paragraphs.len()];
    let mut inside: Option<bool> = None;
    let mut began_at = 0;
    let mut index = 0;
    while index < atoms.len() {
        if let Some(token_index) = TOKENS.iter().position(|token| matches(index, token)) {
            let begin = token_index % 2 == 0;
            let once = token_index < 2;
            if (begin && inside.is_some()) || (!begin && inside != Some(once)) {
                return Err("once/except.once ranges must pair within one container without nesting or mismatched boundaries".into());
            }
            if begin {
                began_at = atoms[index].paragraph;
                inside = Some(once);
            } else {
                if began_at != atoms[index].paragraph
                    && paragraphs[began_at].text == TOKENS[token_index - 1]
                    && paragraphs[began_at].controls.is_empty()
                {
                    marker_paragraphs[began_at] = true;
                }
                inside = None;
            }
            let length = TOKENS[token_index].chars().count();
            keep[index..index + length].fill(false);
            index += length;
            continue;
        }
        if let Some(once) = inside {
            let nested_declaration = match atoms[index].part {
                Part::Control(control) => {
                    contains_declaration(&paragraphs[atoms[index].paragraph].controls[control])?
                }
                _ => false,
            };
            if matches(index, PREFIX) || nested_declaration {
                return Err("once/except.once ranges cannot contain template declarations, title slots or nested ranges".into());
            }
            if first != once {
                keep[index] = false;
            }
        }
        index += 1;
    }
    if inside.is_some() {
        return Err("Conditional ranges must end within the same paragraph container".into());
    }
    let mut boundaries = vec![true; paragraphs.len()];
    for (index, atom) in atoms.iter().enumerate() {
        if matches!(atom.part, Part::Boundary) {
            boundaries[atom.paragraph] = keep[index];
        }
    }
    for (paragraph_index, paragraph) in paragraphs.iter_mut().enumerate() {
        let mut text_keep = vec![true; paragraph.text.chars().count()];
        let mut control_keep = vec![true; paragraph.controls.len()];
        for (index, atom) in atoms
            .iter()
            .enumerate()
            .filter(|(_, a)| a.paragraph == paragraph_index)
        {
            match atom.part {
                Part::Character(position, _) => text_keep[position] = keep[index],
                Part::Control(position) => control_keep[position] = keep[index],
                Part::Boundary => {}
            }
        }
        select(paragraph, &text_keep, &control_keep)?;
    }
    let mut result = Vec::new();
    let mut cursor = 0;
    while cursor < paragraphs.len() {
        let mut end = cursor;
        while end + 1 < paragraphs.len() && !boundaries[end + 1] {
            end += 1;
        }
        let mut paragraph = paragraphs[cursor].clone();
        if end > cursor {
            for original in &paragraphs[cursor..=end] {
                if !original.orphan_field_ends.is_empty()
                    || !original.title_marks.is_empty()
                    || !original.field_ranges.is_empty()
                    || matches!(
                        original.column_type,
                        ColumnBreakType::Section | ColumnBreakType::MultiColumn
                    )
                    || original.raw_break_type & 3 != 0
                {
                    return Err("Conditional paragraph joins do not support cross-paragraph fields, hidden slots or section/column definitions".into());
                }
            }
            let mut tabs = paragraph.tab_extended.clone();
            for next in &paragraphs[cursor + 1..=end] {
                paragraph.merge_from(next);
                tabs.extend_from_slice(&next.tab_extended);
            }
            let mut meta = paragraphs[end].capture_meta();
            meta.column_type = paragraphs[cursor].column_type;
            meta.raw_break_type = paragraphs[cursor].raw_break_type;
            meta.tab_extended = tabs;
            paragraph.apply_meta(meta);
            paragraph.page_break_synthesized = paragraphs[cursor].page_break_synthesized;
            crate::template::discard_stale_layout(&mut paragraph);
        }
        if end != cursor
            || !marker_paragraphs[cursor]
            || !paragraph.text.is_empty()
            || !paragraph.controls.is_empty()
            || paragraph.column_type != ColumnBreakType::None
            || paragraph.raw_break_type != 0
        {
            crate::model_tools::map_nested_containers(&mut paragraph, &mut |children| {
                *children = apply(std::mem::take(children), first)?;
                Ok(())
            })?;
            result.push(paragraph);
        }
        cursor = end + 1;
    }
    Ok(result)
}

fn contains_declaration(control: &Control) -> Result<bool, String> {
    fn visit(value: &serde_json::Value) -> bool {
        match value {
            serde_json::Value::Object(object) => object.iter().any(|(key, value)| {
                (key == "text" && value.as_str().is_some_and(|s| s.contains(PREFIX)))
                    || visit(value)
            }),
            serde_json::Value::Array(values) => values.iter().any(visit),
            _ => false,
        }
    }
    Ok(visit(
        &serde_json::to_value(control).map_err(|error| error.to_string())?,
    ))
}

fn width(character: char) -> u32 {
    if character == '\t' {
        8
    } else {
        character.len_utf16() as u32
    }
}

fn select(
    paragraph: &mut Paragraph,
    text_keep: &[bool],
    control_keep: &[bool],
) -> Result<(), String> {
    if text_keep.iter().all(|&keep| keep) && control_keep.iter().all(|&keep| keep) {
        return Ok(());
    }
    if paragraph.char_offsets.len() != text_keep.len()
        || paragraph.ctrl_data_records.len() > paragraph.controls.len()
    {
        return Err("Conditional edits require complete native text/control offsets".into());
    }
    if !paragraph.orphan_field_ends.is_empty() || !paragraph.title_marks.is_empty() {
        return Err("Conditional edits do not support hidden field-end/title slots".into());
    }
    if control_keep.iter().any(|&keep| !keep) && !paragraph.field_ranges.is_empty() {
        return Err("Conditional control removal does not support native field ranges".into());
    }
    let mut tab = 0;
    let original_tabs = paragraph.tab_extended.clone();
    paragraph.tab_extended = paragraph
        .text
        .chars()
        .zip(text_keep)
        .filter_map(|(character, &keep)| {
            if character != '\t' {
                return None;
            }
            let extension = original_tabs.get(tab).copied();
            tab += 1;
            if keep { extension } else { None }
        })
        .collect();
    // The upstream deletion API preserves rich runs and moves native UTF-16
    // positions. Delete backwards so every offset still addresses source text.
    let mut cursor = text_keep.len();
    while cursor > 0 {
        cursor -= 1;
        if text_keep[cursor] {
            continue;
        }
        let end = cursor + 1;
        while cursor > 0 && !text_keep[cursor - 1] {
            cursor -= 1;
        }
        paragraph.delete_text_at(cursor, end - cursor);
    }
    if control_keep.iter().any(|&keep| !keep) {
        let chars: Vec<_> = paragraph.text.chars().collect();
        let mut positions = paragraph.control_text_positions();
        if chars.is_empty() {
            positions.fill(0);
        }
        let mut removed = Vec::new();
        for position in 0..=chars.len() {
            let controls: Vec<_> = positions
                .iter()
                .enumerate()
                .filter(|(_, p)| **p == position)
                .map(|(i, _)| i)
                .collect();
            let end = paragraph
                .char_offsets
                .get(position)
                .copied()
                .unwrap_or_else(|| {
                    paragraph
                        .char_offsets
                        .last()
                        .zip(chars.last())
                        .map_or(0, |(&offset, &c)| offset + width(c))
                        + controls.len() as u32 * 8
                });
            let start = end
                .checked_sub(controls.len() as u32 * 8)
                .ok_or("Conditional control removal found unsupported native offsets")?;
            let previous_end = position.checked_sub(1).map_or(0, |previous| {
                paragraph.char_offsets[previous] + width(chars[previous])
            });
            if start < previous_end {
                return Err(
                    "Conditional control removal requires native eight-unit control slots".into(),
                );
            }
            for (ordinal, &control) in controls.iter().enumerate() {
                if !control_keep[control] {
                    removed.push(start + ordinal as u32 * 8);
                }
            }
        }
        let shift = |position: u32| -> u32 {
            position
                - removed
                    .iter()
                    .map(|&start| position.saturating_sub(start).min(8))
                    .sum::<u32>()
        };
        for offset in &mut paragraph.char_offsets {
            *offset = shift(*offset);
        }
        for shape in &mut paragraph.char_shapes {
            shape.start_pos = shift(shape.start_pos);
        }
        // Keep the rightmost run at a collapsed boundary, as delete_text_at does.
        let mut shapes = Vec::<rhwp::model::paragraph::CharShapeRef>::new();
        for shape in paragraph.char_shapes.drain(..) {
            if shapes
                .last()
                .is_some_and(|s| s.start_pos == shape.start_pos)
            {
                shapes.pop();
            }
            shapes.push(shape);
        }
        paragraph.char_shapes = shapes;
        for range in &mut paragraph.range_tags {
            range.start = shift(range.start);
            range.end = shift(range.end);
        }
        for mark in &mut paragraph.markpen_marks {
            if let Some(position) = &mut mark.utf16_pos {
                *position = shift(*position);
            }
        }
        paragraph.char_count = paragraph
            .char_count
            .checked_sub(removed.len() as u32 * 8)
            .ok_or("Conditional control removal found an invalid native character count")?;
        paragraph.align_ctrl_data_records();
        let mut index = 0;
        paragraph.controls.retain(|_| {
            let keep = control_keep[index];
            index += 1;
            keep
        });
        let mut index = 0;
        paragraph.ctrl_data_records.retain(|_| {
            let keep = control_keep[index];
            index += 1;
            keep
        });
    }
    // Match the pinned rhwp native stream categories. The upstream mask helper
    // is private; stale bits must not survive removed controls or tabs.
    paragraph.control_mask = paragraph.controls.iter().fold(0, |mask, control| {
        mask | (1
            << match control {
                Control::SectionDef(_) | Control::ColumnDef(_) => 2,
                Control::Field(_) => 3,
                Control::Table(_)
                | Control::Shape(_)
                | Control::Picture(_)
                | Control::Hyperlink(_)
                | Control::Ruby(_)
                | Control::Equation(_)
                | Control::Form(_)
                | Control::Unknown(_) => 11,
                Control::HiddenComment(_) => 15,
                Control::Header(_) | Control::Footer(_) => 16,
                Control::Footnote(_) | Control::Endnote(_) => 17,
                Control::AutoNumber(_) | Control::NewNumber(_) => 18,
                Control::PageNumberPos(_) | Control::PageHide(_) | Control::PageNumCtrl(_) => 21,
                Control::Bookmark(_) | Control::IndexMark(_) => 22,
                Control::CharOverlap(_) => 23,
            })
    });
    if !paragraph.field_ranges.is_empty() {
        paragraph.control_mask |= 1 << 4;
    }
    if paragraph.text.contains('\t') {
        paragraph.control_mask |= 1 << 9;
    }
    if paragraph.text.contains('\n') {
        paragraph.control_mask |= 1 << 10;
    }
    crate::template::discard_stale_layout(paragraph);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rhwp::model::paragraph::CharShapeRef;

    fn p(text: &str, shape: u16) -> Paragraph {
        let mut paragraph = Paragraph {
            para_shape_id: shape,
            char_count: 1,
            ..Default::default()
        };
        paragraph.char_shapes.push(CharShapeRef {
            start_pos: 0,
            char_shape_id: 3,
        });
        paragraph.insert_text_at(0, text);
        paragraph
    }
    fn tag(name: &str) -> String {
        format!("{{{{md2hwp:{name}}}}}")
    }

    #[test]
    fn inline_and_cross_paragraph_selection_preserves_format_and_breaks() {
        for kind in ["once", "except.once"] {
            for first in [true, false] {
                let include = first == (kind == "once");
                let begin = tag(&format!("begin:{kind}"));
                let end = tag(&format!("end:{kind}"));
                let mut inline = p(&format!("A{begin}한😀{end}C"), 2);
                inline.column_type = ColumnBreakType::Page;
                let output = apply(vec![inline], first).unwrap();
                assert_eq!(output[0].text, if include { "A한😀C" } else { "AC" });
                assert_eq!(output[0].column_type, ColumnBreakType::Page);
                let left = p(&format!("before{begin}inside"), 1);
                let mut right = p(&format!("inside{end}after"), 2);
                right.column_type = ColumnBreakType::Page;
                let output = apply(vec![left, right], first).unwrap();
                assert_eq!(output.len(), if include { 2 } else { 1 });
                assert_eq!(output.last().unwrap().para_shape_id, 2);
                assert_eq!(
                    output.last().unwrap().column_type,
                    if include {
                        ColumnBreakType::Page
                    } else {
                        ColumnBreakType::None
                    }
                );
                assert_eq!(
                    output.iter().map(|p| p.text.as_str()).collect::<String>(),
                    if include {
                        "beforeinsideinsideafter"
                    } else {
                        "beforeafter"
                    }
                );
            }
        }
    }

    #[test]
    fn marker_only_paragraphs_and_multiple_joins() {
        let begin = tag("begin:except.once");
        let end = tag("end:except.once");
        for first in [true, false] {
            let output = apply(vec![p(&begin, 1), p(&format!("{end}cover"), 2)], first).unwrap();
            assert_eq!(output.len(), 1);
            assert_eq!(output[0].text, "cover");
        }
        let output = apply(
            vec![
                p(&format!("A{begin}"), 1),
                p(&format!("{end}B{begin}"), 2),
                p(&format!("{end}C"), 3),
            ],
            true,
        )
        .unwrap();
        assert_eq!(output.len(), 1);
        assert_eq!(output[0].text, "ABC");
        assert_eq!(output[0].para_shape_id, 3);
    }

    #[test]
    fn rejects_nested_mismatched_unclosed_and_enclosed_declarations() {
        for text in [
            format!("{}{}", tag("begin:once"), tag("end:except.once")),
            format!(
                "{}{}{}{}",
                tag("begin:once"),
                tag("begin:once"),
                tag("end:once"),
                tag("end:once")
            ),
            tag("begin:once"),
            format!(
                "{}{}{}",
                tag("begin:once"),
                tag("slot:heading1"),
                tag("end:once")
            ),
        ] {
            assert!(apply(vec![p(&text, 0)], true).is_err());
        }
    }

    #[test]
    fn selects_native_controls_and_preserves_the_following_rich_run() {
        use rhwp::model::control::NewNumber;
        for first in [true, false] {
            let mut paragraph = p(&format!("A{}", tag("begin:once")), 0);
            let control = Paragraph {
                char_count: 9,
                controls: vec![Control::NewNumber(NewNumber {
                    number: 7,
                    ..Default::default()
                })],
                ctrl_data_records: vec![Some(vec![7, 8])],
                ..Default::default()
            };
            paragraph.merge_from(&control);
            let mut suffix = p(&format!("B{}C", tag("end:once")), 0);
            suffix.char_shapes[0].char_shape_id = 9;
            paragraph.merge_from(&suffix);
            let output = apply(vec![paragraph], first).unwrap();
            let paragraph = &output[0];
            assert_eq!(paragraph.text, if first { "ABC" } else { "AC" });
            assert_eq!(paragraph.controls.len(), usize::from(first));
            assert_eq!(
                paragraph.char_shape_id_at(paragraph.text.chars().count() - 1),
                Some(9)
            );
            assert_eq!(
                paragraph.char_offsets,
                if first { vec![0, 9, 10] } else { vec![0, 1] }
            );
            if first {
                assert_eq!(paragraph.ctrl_data_records, vec![Some(vec![7, 8])]);
            }
        }
    }

    #[test]
    fn recurses_within_each_cell_and_rejects_cross_cell_ranges() {
        use rhwp::model::table::{Cell, Table};
        let mut table = Table::default();
        table.cells.push(Cell {
            paragraphs: vec![
                p(&tag("begin:except.once"), 0),
                p(&format!("{}nested", tag("end:except.once")), 0),
            ],
            ..Default::default()
        });
        let mut anchor = p("", 0);
        anchor.controls.push(Control::Table(Box::new(table)));
        let output = apply(vec![anchor.clone()], true).unwrap();
        let Control::Table(table) = &output[0].controls[0] else {
            panic!("table lost")
        };
        assert_eq!(table.cells[0].paragraphs.len(), 1);
        assert_eq!(table.cells[0].paragraphs[0].text, "nested");
        let Control::Table(table) = &mut anchor.controls[0] else {
            unreachable!()
        };
        let end = table.cells[0].paragraphs.pop().unwrap();
        table.cells.push(Cell {
            paragraphs: vec![end],
            ..Default::default()
        });
        assert!(apply(vec![anchor], true).is_err());
    }
}
