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
    FieldEnd(usize),
    OrphanEnd(usize),
    TitleMark(usize),
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
        for part in parts(paragraph)? {
            atoms.push(Atom {
                paragraph: index,
                part,
            });
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
    // A field may span the paragraph boundary that is about to disappear.
    // Require both original native boundaries to make the same selection;
    // retaining an unmatched half cannot survive strict native reopen checks.
    let mut open_fields = std::collections::HashMap::<(u32, u32), Vec<usize>>::new();
    for (index, atom) in atoms.iter().enumerate() {
        let key = match atom.part {
            Part::Control(control) => {
                if let Control::Field(field) = &paragraphs[atom.paragraph].controls[control] {
                    open_fields
                        .entry((field.field_id, field.ctrl_id))
                        .or_default()
                        .push(index);
                }
                continue;
            }
            Part::FieldEnd(range) => {
                let paragraph = &paragraphs[atom.paragraph];
                let Control::Field(field) =
                    &paragraph.controls[paragraph.field_ranges[range].control_idx]
                else {
                    // Legacy Hyperlink controls have no document-wide field ID.
                    // Their local pair selection is checked by select below.
                    continue;
                };
                (field.field_id, field.ctrl_id)
            }
            Part::OrphanEnd(marker) => {
                let marker = &paragraphs[atom.paragraph].orphan_field_ends[marker];
                (marker.begin_id_ref, marker.begin_ctrl_id)
            }
            _ => continue,
        };
        if let Some(begin) = open_fields.get_mut(&key).and_then(Vec::pop)
            && keep[begin] != keep[index]
        {
            return Err(
                "Conditional ranges cannot remove only one boundary of a native field".into(),
            );
        }
    }
    let mut boundaries = vec![true; paragraphs.len()];
    for (index, atom) in atoms.iter().enumerate() {
        if matches!(atom.part, Part::Boundary) {
            boundaries[atom.paragraph] = keep[index];
        }
    }
    for (paragraph_index, paragraph) in paragraphs.iter_mut().enumerate() {
        let selected: Vec<_> = atoms
            .iter()
            .enumerate()
            .filter(|(_, atom)| {
                atom.paragraph == paragraph_index && !matches!(atom.part, Part::Boundary)
            })
            .map(|(index, _)| keep[index])
            .collect();
        select(paragraph, &selected)?;
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
                if matches!(
                    original.column_type,
                    ColumnBreakType::Section | ColumnBreakType::MultiColumn
                ) || original.raw_break_type & 3 != 0
                {
                    return Err(
                        "Conditional paragraph joins do not support section/column definitions"
                            .into(),
                    );
                }
            }
            let mut tabs = paragraph.tab_extended.clone();
            for next in &paragraphs[cursor + 1..=end] {
                join(&mut paragraph, next)?;
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
            || !paragraph.title_marks.is_empty()
            || !paragraph.orphan_field_ends.is_empty()
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

// Upstream merge_from omits orphan field ends, trailing hidden slots and
// recomputes char_count without FIELD_END/TITLE_MARK units. Join on the full
// native axis instead, retaining every saved control and formatting identity.
fn join(left: &mut Paragraph, right: &Paragraph) -> Result<(), String> {
    let text_shift = left.text.chars().count();
    let native_shift = left
        .char_count
        .checked_sub(1)
        .ok_or("Conditional join has no paragraph terminator")?;
    let total_native = native_shift
        .checked_add(right.char_count)
        .ok_or("Conditional join character count overflow")?;
    text_shift
        .checked_add(right.text.chars().count())
        .ok_or("Conditional join text index overflow")?;
    let control_shift = left.controls.len();
    control_shift
        .checked_add(right.controls.len())
        .ok_or("Conditional join control index overflow")?;
    // Prove every addition below stays within the independently checked native
    // and text axes, including source side channels that do not contain text.
    for paragraph in [&*left, right] {
        let text_len = paragraph.text.chars().count();
        if paragraph.char_count == 0
            || paragraph
                .char_offsets
                .iter()
                .any(|&p| p >= paragraph.char_count)
            || paragraph
                .char_shapes
                .iter()
                .any(|s| s.start_pos > paragraph.char_count)
            || paragraph
                .range_tags
                .iter()
                .any(|r| r.start > r.end || r.end > paragraph.char_count)
            || paragraph.markpen_marks.iter().any(|m| {
                m.char_idx > text_len || m.utf16_pos.is_some_and(|p| p > paragraph.char_count)
            })
            || paragraph.title_marks.iter().any(|m| m.char_idx > text_len)
            || paragraph
                .orphan_field_ends
                .iter()
                .any(|m| m.char_idx > text_len)
            || paragraph.field_ranges.iter().any(|r| {
                r.start_char_idx > r.end_char_idx
                    || r.end_char_idx > text_len
                    || r.control_idx
                        .checked_add(r.inner_slot_count)
                        .is_none_or(|i| i >= paragraph.controls.len())
            })
        {
            return Err("Conditional join contains an invalid native position".into());
        }
    }
    left.text.push_str(&right.text);
    left.char_offsets
        .extend(right.char_offsets.iter().map(|p| p + native_shift));
    left.char_shapes
        .extend(right.char_shapes.iter().cloned().map(|mut shape| {
            shape.start_pos += native_shift;
            shape
        }));
    // At an empty join boundary the ending paragraph's run owns the next text.
    let mut shapes = Vec::<rhwp::model::paragraph::CharShapeRef>::new();
    for shape in left.char_shapes.drain(..) {
        if shapes
            .last()
            .is_some_and(|s| s.start_pos == shape.start_pos)
        {
            shapes.pop();
        }
        shapes.push(shape);
    }
    left.char_shapes = shapes;
    left.range_tags
        .extend(right.range_tags.iter().cloned().map(|mut range| {
            range.start += native_shift;
            range.end += native_shift;
            range
        }));
    left.markpen_marks
        .extend(right.markpen_marks.iter().cloned().map(|mut mark| {
            mark.char_idx += text_shift;
            if let Some(pos) = &mut mark.utf16_pos {
                *pos += native_shift;
            }
            mark
        }));
    left.title_marks
        .extend(right.title_marks.iter().cloned().map(|mut mark| {
            mark.char_idx += text_shift;
            mark
        }));
    left.orphan_field_ends
        .extend(right.orphan_field_ends.iter().cloned().map(|mut mark| {
            mark.char_idx += text_shift;
            mark
        }));
    left.field_ranges
        .extend(right.field_ranges.iter().cloned().map(|mut range| {
            range.start_char_idx += text_shift;
            range.end_char_idx += text_shift;
            range.control_idx += control_shift;
            range
        }));
    left.align_ctrl_data_records();
    left.ctrl_data_records.extend(
        (0..right.controls.len()).map(|i| right.ctrl_data_records.get(i).cloned().flatten()),
    );
    left.controls.extend(right.controls.iter().cloned());
    left.control_mask |= right.control_mask;
    left.char_count = total_native;
    left.has_para_text |= right.has_para_text;
    // A cross-paragraph field becomes a local pair when its boundary disappears.
    // Use its native identity, never the display label or a fabricated field ID.
    let positions = left.control_text_positions();
    let mut unmatched = Vec::new();
    for end in std::mem::take(&mut left.orphan_field_ends) {
        let matched = left.controls.iter().enumerate().find(|(i, control)| {
            matches!(control, Control::Field(field) if field.field_id == end.begin_id_ref && field.ctrl_id == end.begin_ctrl_id)
                && positions[*i] <= end.char_idx && !left.field_ranges.iter().any(|range| range.control_idx == *i)
        });
        if let Some((control_idx, _)) = matched {
            let inner_slot_count = positions
                .iter()
                .enumerate()
                .skip(control_idx + 1)
                .take_while(|(_, position)| **position < end.char_idx)
                .count();
            left.field_ranges.push(rhwp::model::paragraph::FieldRange {
                control_idx,
                start_char_idx: positions[control_idx],
                end_char_idx: end.char_idx,
                end_field_id: end.field_id,
                inner_slot_count,
            });
        } else {
            unmatched.push(end);
        }
    }
    left.orphan_field_ends = unmatched;
    Ok(())
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

// Keep hidden eight-unit slots in the same stream as visible controls. In
// particular, a tag cannot jump across a field end or a title marker.
fn parts(paragraph: &Paragraph) -> Result<Vec<Part>, String> {
    let chars: Vec<_> = paragraph.text.chars().collect();
    if paragraph.char_offsets.len() != chars.len() {
        return Err("Conditional ranges require complete native character offsets".into());
    }
    let mut positions = paragraph.control_text_positions();
    if chars.is_empty() {
        positions.fill(0);
    }
    if positions.iter().any(|&p| p > chars.len())
        || paragraph.field_ranges.iter().any(|r| {
            r.start_char_idx > r.end_char_idx
                || r.end_char_idx > chars.len()
                || r.control_idx >= paragraph.controls.len()
                || r.control_idx
                    .checked_add(r.inner_slot_count)
                    .is_none_or(|i| i >= paragraph.controls.len())
        })
        || paragraph
            .orphan_field_ends
            .iter()
            .any(|m| m.char_idx > chars.len())
        || paragraph
            .title_marks
            .iter()
            .any(|m| m.char_idx > chars.len())
    {
        return Err("Conditional ranges require complete native control/field positions".into());
    }
    let mut result = Vec::new();
    let mut ends = vec![false; paragraph.field_ranges.len()];
    for position in 0..=chars.len() {
        for (i, _) in paragraph
            .orphan_field_ends
            .iter()
            .enumerate()
            .filter(|(_, m)| m.char_idx == position)
        {
            result.push(Part::OrphanEnd(i));
        }
        for (i, _) in paragraph
            .title_marks
            .iter()
            .enumerate()
            .filter(|(_, m)| m.char_idx == position)
        {
            result.push(Part::TitleMark(i));
        }
        for (i, range) in paragraph.field_ranges.iter().enumerate() {
            if range.end_char_idx == position
                && range.start_char_idx < position
                && positions[range.control_idx + range.inner_slot_count] < position
            {
                result.push(Part::FieldEnd(i));
                ends[i] = true;
            }
        }
        for (control, _) in positions
            .iter()
            .enumerate()
            .filter(|(_, p)| **p == position)
        {
            result.push(Part::Control(control));
            for (i, range) in paragraph.field_ranges.iter().enumerate() {
                if !ends[i]
                    && range.end_char_idx == position
                    && range.control_idx + range.inner_slot_count == control
                {
                    result.push(Part::FieldEnd(i));
                    ends[i] = true;
                }
            }
        }
        if let Some(&c) = chars.get(position) {
            result.push(Part::Character(position, c));
        }
    }
    if ends.iter().any(|&emitted| !emitted) {
        return Err("Conditional ranges found an invalid native field boundary".into());
    }
    Ok(result)
}

fn select(paragraph: &mut Paragraph, keep: &[bool]) -> Result<(), String> {
    if keep.iter().all(|&keep| keep) {
        return Ok(());
    }
    let stream = parts(paragraph)?;
    if paragraph.char_offsets.len() != paragraph.text.chars().count()
        || paragraph.ctrl_data_records.len() > paragraph.controls.len()
        || stream.len() != keep.len()
    {
        return Err("Conditional edits require complete native text/control offsets".into());
    }
    let mut text_keep = vec![false; paragraph.char_offsets.len()];
    let mut control_keep = vec![false; paragraph.controls.len()];
    let mut field_keep = vec![false; paragraph.field_ranges.len()];
    let mut orphan_keep = vec![false; paragraph.orphan_field_ends.len()];
    let mut title_keep = vec![false; paragraph.title_marks.len()];
    let mut removed = Vec::new();
    let mut native = 0u32;
    let mut previous_control = None;
    for (part, &retain) in stream.iter().zip(keep) {
        let units = match *part {
            Part::Character(i, c) => {
                // The HWP5 reader leaves one visible space for a native number
                // control. That character shares the control's eight units.
                let number_placeholder = c == ' '
                    && previous_control.is_some_and(|control| {
                        matches!(
                            paragraph.controls[control],
                            Control::AutoNumber(_) | Control::NewNumber(_)
                        ) && paragraph.char_offsets[i].checked_add(8) == Some(native)
                            && control_keep[control] == retain
                    });
                if !number_placeholder && paragraph.char_offsets[i] != native {
                    return Err("Conditional edits require exact native UTF-16 slots".into());
                }
                text_keep[i] = retain;
                if number_placeholder { 0 } else { width(c) }
            }
            Part::Control(i) => {
                control_keep[i] = retain;
                if paragraph.controls[i].occupies_ctrl_char_slot() {
                    8
                } else {
                    0
                }
            }
            Part::FieldEnd(i) => {
                field_keep[i] = retain;
                8
            }
            Part::OrphanEnd(i) => {
                orphan_keep[i] = retain;
                8
            }
            Part::TitleMark(i) => {
                title_keep[i] = retain;
                8
            }
            Part::Boundary => unreachable!(),
        };
        if !retain {
            removed.push((native, units));
        }
        native = native
            .checked_add(units)
            .ok_or("Conditional native position overflow")?;
        previous_control = if let Part::Control(i) = *part {
            Some(i)
        } else {
            None
        };
    }
    if native.checked_add(1) != Some(paragraph.char_count) {
        return Err("Conditional edits require an exact native character count".into());
    }
    let shift = |position: u32| -> u32 {
        position
            - removed
                .iter()
                .map(|&(start, units)| position.saturating_sub(start).min(units))
                .sum::<u32>()
    };
    let mut text_indices = vec![0; text_keep.len() + 1];
    for (i, &retain) in text_keep.iter().enumerate() {
        text_indices[i + 1] = text_indices[i] + usize::from(retain);
    }
    let mut control_indices = vec![0; control_keep.len() + 1];
    for (i, &retain) in control_keep.iter().enumerate() {
        control_indices[i + 1] = control_indices[i] + usize::from(retain);
    }
    // Removing just one native field boundary cannot leave a valid native field.
    // The enclosing template contract requires matched fields after save/reopen.
    for (i, range) in paragraph.field_ranges.iter().enumerate() {
        if field_keep[i] != control_keep[range.control_idx] {
            return Err(
                "Conditional ranges cannot remove only one boundary of a native field".into(),
            );
        }
    }
    let mut tab = 0;
    let old_tabs = std::mem::take(&mut paragraph.tab_extended);
    paragraph.text = paragraph
        .text
        .chars()
        .enumerate()
        .filter_map(|(i, c)| {
            if c == '\t' {
                if text_keep[i]
                    && let Some(extension) = old_tabs.get(tab)
                {
                    paragraph.tab_extended.push(*extension);
                }
                tab += 1;
            }
            text_keep[i].then_some(c)
        })
        .collect();
    paragraph.char_offsets = paragraph
        .char_offsets
        .iter()
        .zip(&text_keep)
        .filter(|(_, retain)| **retain)
        .map(|(&offset, _)| shift(offset))
        .collect();
    let mut shapes = Vec::<rhwp::model::paragraph::CharShapeRef>::new();
    for mut shape in paragraph.char_shapes.drain(..) {
        shape.start_pos = shift(shape.start_pos);
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
        mark.char_idx = *text_indices
            .get(mark.char_idx)
            .ok_or("Conditional edits found an invalid mark position")?;
        if let Some(pos) = &mut mark.utf16_pos {
            *pos = shift(*pos);
        }
    }
    let mut index = 0;
    paragraph.field_ranges.retain_mut(|range| {
        let retain = field_keep[index];
        index += 1;
        if retain {
            let last = range.control_idx + range.inner_slot_count + 1;
            range.inner_slot_count = control_indices[last] - control_indices[range.control_idx + 1];
            range.control_idx = control_indices[range.control_idx];
            range.start_char_idx = text_indices[range.start_char_idx];
            range.end_char_idx = text_indices[range.end_char_idx];
        }
        retain
    });
    let mut index = 0;
    paragraph.orphan_field_ends.retain_mut(|mark| {
        let retain = orphan_keep[index];
        index += 1;
        mark.char_idx = text_indices[mark.char_idx];
        retain
    });
    let mut index = 0;
    paragraph.title_marks.retain_mut(|mark| {
        let retain = title_keep[index];
        index += 1;
        mark.char_idx = text_indices[mark.char_idx];
        retain
    });
    paragraph.align_ctrl_data_records();
    let mut index = 0;
    paragraph.controls.retain(|_| {
        let retain = control_keep[index];
        index += 1;
        retain
    });
    let mut index = 0;
    paragraph.ctrl_data_records.retain(|_| {
        let retain = control_keep[index];
        index += 1;
        retain
    });
    paragraph.char_count = shift(paragraph.char_count);
    // Match the pinned rhwp native stream categories. The upstream mask helper
    // is private; stale bits must not survive removed controls or tabs.
    let literal_mask = [(24, '\u{00ad}'), (30, '\u{00a0}'), (31, '\u{2007}')]
        .into_iter()
        .filter(|(_, c)| paragraph.text.contains(*c))
        .fold(0, |mask, (bit, _)| {
            mask | (paragraph.control_mask & (1 << bit))
        });
    paragraph.control_mask = paragraph
        .controls
        .iter()
        .filter(|control| control.occupies_ctrl_char_slot())
        .fold(literal_mask, |mask, control| {
            mask | (1
                << match control {
                    Control::SectionDef(_) | Control::ColumnDef(_) => 2,
                    Control::Field(_) => 3,
                    Control::Table(_)
                    | Control::Shape(_)
                    | Control::Picture(_)
                    | Control::Hyperlink(_)
                    | Control::Equation(_)
                    | Control::Form(_)
                    | Control::Unknown(_) => 11,

                    Control::Header(_) | Control::Footer(_) => 16,
                    Control::Footnote(_) | Control::Endnote(_) => 17,
                    Control::AutoNumber(_) => 18,
                    Control::NewNumber(_)
                    | Control::PageNumberPos(_)
                    | Control::PageHide(_)
                    | Control::PageNumCtrl(_) => 21,
                    Control::Bookmark(_) | Control::IndexMark(_) => 22,
                    Control::CharOverlap(_) | Control::Ruby(_) | Control::HiddenComment(_) => 23,
                })
        });
    if !paragraph.field_ranges.is_empty() || !paragraph.orphan_field_ends.is_empty() {
        paragraph.control_mask |= 1 << 4;
    }
    if !paragraph.title_marks.is_empty() {
        paragraph.control_mask |= 1 << 8;
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

    fn field() -> Control {
        use rhwp::model::control::{Field, FieldType};
        Control::Field(Field {
            field_type: FieldType::ClickHere,
            field_id: 42,
            ctrl_id: u32::from_le_bytes(*b"klc%"),
            instance_id: Some(87),
            ..Default::default()
        })
    }

    #[test]
    fn deleting_inside_native_field_preserves_hidden_end_and_utf16_axis() {
        use rhwp::model::paragraph::FieldRange;
        let mut paragraph = p(
            &format!("A{}remove{}한😀", tag("begin:once"), tag("end:once")),
            3,
        );
        let length = paragraph.text.chars().count();
        paragraph.controls.push(field());
        paragraph.ctrl_data_records.push(Some(vec![11, 22]));
        paragraph.field_ranges.push(FieldRange {
            start_char_idx: 0,
            end_char_idx: length,
            control_idx: 0,
            end_field_id: 91,
            inner_slot_count: 0,
        });
        for offset in &mut paragraph.char_offsets {
            *offset += 8;
        }
        paragraph.char_count += 16;
        let output = apply(vec![paragraph], false).unwrap();
        let paragraph = &output[0];
        assert_eq!(paragraph.text, "A한😀");
        assert_eq!(paragraph.char_offsets, [8, 9, 10]);
        assert_eq!(paragraph.char_count, 21);
        assert_eq!(paragraph.field_ranges.len(), 1);
        assert_eq!(paragraph.field_ranges[0].end_char_idx, 3);
        assert_eq!(paragraph.field_ranges[0].end_field_id, 91);
        assert_eq!(paragraph.ctrl_data_records, [Some(vec![11, 22])]);
        let Control::Field(field) = &paragraph.controls[0] else {
            panic!("field lost")
        };
        assert_eq!((field.field_id, field.instance_id), (42, Some(87)));
    }

    #[test]
    fn deletes_complete_field_and_reindexes_surviving_field() {
        use rhwp::model::paragraph::FieldRange;
        let begin = tag("begin:once");
        let end = tag("end:once");
        let mut paragraph = p(&format!("{begin}x{end}😀"), 0);
        let start = begin.chars().count();
        let after = start + 1;
        let final_index = paragraph.text.chars().count() - 1;
        paragraph.controls = vec![field(), field()];
        paragraph.field_ranges = vec![
            FieldRange {
                start_char_idx: start,
                end_char_idx: after,
                control_idx: 0,
                ..Default::default()
            },
            FieldRange {
                start_char_idx: final_index,
                end_char_idx: final_index + 1,
                control_idx: 1,
                end_field_id: 99,
                ..Default::default()
            },
        ];
        for (i, offset) in paragraph.char_offsets.iter_mut().enumerate() {
            *offset += u32::from(i >= start) * 8
                + u32::from(i >= after) * 8
                + u32::from(i >= final_index) * 8;
        }
        paragraph.char_count += 32;
        let output = apply(vec![paragraph], false).unwrap();
        let paragraph = &output[0];
        assert_eq!(paragraph.text, "😀");
        assert_eq!(paragraph.controls.len(), 1);
        assert_eq!(paragraph.char_offsets, [8]);
        assert_eq!(paragraph.char_count, 19);
        assert_eq!(paragraph.field_ranges[0].control_idx, 0);
        assert_eq!(paragraph.field_ranges[0].start_char_idx, 0);
        assert_eq!(paragraph.field_ranges[0].end_char_idx, 1);
        assert_eq!(paragraph.field_ranges[0].end_field_id, 99);
    }

    #[test]
    fn joins_cross_paragraph_field_using_native_identity_and_trailing_slots() {
        use rhwp::model::paragraph::{OrphanFieldEnd, TitleMark};
        let mut left = p(&format!("A{}drop", tag("begin:once")), 1);
        left.controls.push(field());
        left.ctrl_data_records.push(Some(vec![71]));
        for offset in &mut left.char_offsets {
            *offset += 8;
        }
        left.char_count += 8;
        left.column_type = ColumnBreakType::Page;
        let mut right = p(&format!("drop{}B\t😀", tag("end:once")), 4);
        let length = right.text.chars().count();
        right.orphan_field_ends.push(OrphanFieldEnd {
            char_idx: length,
            begin_id_ref: 42,
            field_id: 99,
            begin_ctrl_id: u32::from_le_bytes(*b"klc%"),
        });
        right.title_marks.push(TitleMark {
            char_idx: length,
            ignore: true,
        });
        right.tab_extended = vec![[7, 6, 5, 4, 3, 2, 1]];
        right.char_count += 16;
        right.column_type = ColumnBreakType::Column;
        let output = apply(vec![left, right], false).unwrap();
        assert_eq!(output.len(), 1);
        let paragraph = &output[0];
        assert_eq!(paragraph.text, "AB\t😀");
        assert_eq!(paragraph.char_offsets, [8, 9, 10, 18]);
        assert_eq!(paragraph.char_count, 37);
        assert_eq!(paragraph.para_shape_id, 4);
        assert_eq!(paragraph.column_type, ColumnBreakType::Page);
        assert_eq!(
            paragraph.title_marks,
            [TitleMark {
                char_idx: 4,
                ignore: true
            }]
        );
        assert!(paragraph.orphan_field_ends.is_empty());
        assert_eq!(paragraph.field_ranges.len(), 1);
        assert_eq!(paragraph.field_ranges[0].start_char_idx, 0);
        assert_eq!(paragraph.field_ranges[0].end_char_idx, 4);
        assert_eq!(paragraph.field_ranges[0].end_field_id, 99);
        assert_eq!(paragraph.tab_extended, [[7, 6, 5, 4, 3, 2, 1]]);
        assert_eq!(paragraph.ctrl_data_records, [Some(vec![71])]);
    }

    #[test]
    fn retained_control_mask_matches_native_hwp5_categories() {
        let mut paragraph = p(&format!("{}x{}B", tag("begin:once"), tag("end:once")), 0);
        paragraph.controls = vec![
            Control::Ruby(Default::default()),
            Control::HiddenComment(Default::default()),
            Control::NewNumber(Default::default()),
        ];
        for offset in &mut paragraph.char_offsets {
            *offset += 24;
        }
        paragraph.char_count += 24;
        let output = apply(vec![paragraph], false).unwrap();
        assert_eq!(output[0].text, "B");
        assert_eq!(output[0].char_offsets, [24]);
        assert_eq!(output[0].control_mask, (1 << 23) | (1 << 21));
        assert_eq!(output[0].char_count, 26);
    }

    #[test]
    fn joins_reject_overflow_and_out_of_axis_native_positions_before_editing() {
        let mut left = p("A", 0);
        left.char_count = u32::MAX;
        assert!(
            join(&mut left, &p("B", 0))
                .unwrap_err()
                .contains("overflow")
        );
        assert_eq!(left.text, "A");
        let mut left = p("A", 0);
        let mut right = p("B", 0);
        right.char_shapes[0].start_pos = u32::MAX;
        assert!(
            join(&mut left, &right)
                .unwrap_err()
                .contains("invalid native position")
        );
        assert_eq!(left.text, "A");
    }

    #[test]
    fn rejects_partial_cross_paragraph_field_deletion() {
        use rhwp::model::paragraph::OrphanFieldEnd;
        let mut left = p(&format!("A{}", tag("begin:once")), 0);
        left.controls.push(field());
        for offset in &mut left.char_offsets {
            *offset += 8;
        }
        left.char_count += 8;
        let mut right = p(&format!("B{}C", tag("end:once")), 0);
        right.orphan_field_ends.push(OrphanFieldEnd {
            char_idx: 1,
            begin_id_ref: 42,
            field_id: 91,
            begin_ctrl_id: u32::from_le_bytes(*b"klc%"),
        });
        for offset in &mut right.char_offsets[1..] {
            *offset += 8;
        }
        right.char_count += 8;
        let error = apply(vec![left, right], false).unwrap_err();
        assert!(error.contains("only one boundary"), "{error}");
    }

    #[test]
    fn retained_native_number_placeholder_shares_its_eight_unit_slot() {
        use rhwp::model::control::AutoNumber;
        let mut paragraph = p(
            &format!(" {}remove{}😀", tag("begin:once"), tag("end:once")),
            0,
        );
        paragraph
            .controls
            .push(Control::AutoNumber(AutoNumber::default()));
        // The reader's visible number space consumes no additional native unit.
        for offset in &mut paragraph.char_offsets[1..] {
            *offset += 7;
        }
        paragraph.char_count += 7;
        let output = apply(vec![paragraph], false).unwrap();
        assert_eq!(output[0].text, " 😀");
        assert_eq!(output[0].char_offsets, [0, 8]);
        assert_eq!(output[0].char_count, 11);
        assert_eq!(output[0].controls.len(), 1);
    }

    #[test]
    fn hidden_markers_interrupt_tokens_and_are_selected_with_their_native_slots() {
        use rhwp::model::paragraph::TitleMark;
        for first in [true, false] {
            let begin = tag("begin:once");
            let end = tag("end:once");
            let mut paragraph = p(&format!("{begin}한{end}😀"), 0);
            let position = begin.chars().count();
            paragraph.title_marks.push(TitleMark {
                char_idx: position,
                ignore: false,
            });
            for offset in &mut paragraph.char_offsets[position..] {
                *offset += 8;
            }
            paragraph.char_count += 8;
            let output = apply(vec![paragraph], first).unwrap();
            let paragraph = &output[0];
            assert_eq!(paragraph.text, if first { "한😀" } else { "😀" });
            assert_eq!(paragraph.title_marks.len(), usize::from(first));
            assert_eq!(
                paragraph.char_offsets,
                if first { vec![8, 9] } else { vec![0] }
            );
            assert_eq!(paragraph.char_count, if first { 12 } else { 3 });
        }
        let mut interrupted = p(&format!("{}x{}", tag("begin:once"), tag("end:once")), 0);
        interrupted.title_marks.push(TitleMark {
            char_idx: 3,
            ignore: false,
        });
        for offset in &mut interrupted.char_offsets[3..] {
            *offset += 8;
        }
        interrupted.char_count += 8;
        assert!(apply(vec![interrupted], true).is_err());
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
