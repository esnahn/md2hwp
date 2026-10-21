// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Euisoon Ahn
//! Materialize a private reference marker as a native HWP5 CrossRef and cache.

use crate::model_tools::IdentityAllocator;
use rhwp::model::control::{Control, Field, FieldType};
use rhwp::model::paragraph::{CharShapeRef, FieldRange, Paragraph};

/// Expand literal affixes without the editor's non-hyperlink boundary affinity.
/// A field ending exactly before this marker must retain its original end.
pub(crate) fn expand(
    paragraph: &mut Paragraph,
    marker: &str,
    expanded_text: &str,
) -> Result<(), String> {
    if marker.is_empty()
        || marker.chars().any(char::is_control)
        || expanded_text.is_empty()
        || expanded_text.chars().any(char::is_control)
    {
        return Err("Native reference expansion requires nonempty plain text".into());
    }
    let byte_start = paragraph
        .text
        .find(marker)
        .ok_or("Native reference expansion marker is missing")?;
    let start = paragraph.text[..byte_start].chars().count();
    let count = marker.chars().count();
    let end = start + count;
    let chars: Vec<char> = paragraph.text.chars().collect();
    if paragraph.char_offsets.len() != chars.len() {
        return Err("Native reference expansion has incomplete character offsets".into());
    }
    let native_start = paragraph.char_offsets[start];
    let mut native_end = native_start;
    for (index, character) in chars.iter().enumerate().take(end).skip(start) {
        if paragraph.char_offsets[index] != native_end {
            return Err("Native reference expansion crosses a native control".into());
        }
        native_end = native_end
            .checked_add(character.len_utf16() as u32)
            .ok_or("Reference expansion offset overflow")?;
    }
    let inside = |index| start < index && index < end;
    if paragraph.field_ranges.iter().any(|range| {
        range.start_char_idx < end && range.end_char_idx > start
            || inside(range.start_char_idx)
            || inside(range.end_char_idx)
    }) || paragraph
        .orphan_field_ends
        .iter()
        .any(|mark| inside(mark.char_idx))
        || paragraph
            .title_marks
            .iter()
            .any(|mark| inside(mark.char_idx))
        || paragraph
            .control_text_positions()
            .iter()
            .any(|&index| inside(index))
    {
        return Err("Native reference expansion overlaps a native field or control".into());
    }
    let removed = native_end - native_start;
    let width = u32::try_from(expanded_text.encode_utf16().count())
        .map_err(|_| "Reference expansion too long")?;
    let new_end = native_start
        .checked_add(width)
        .ok_or("Reference expansion end overflow")?;
    let remap = |position: u32| -> Result<u32, String> {
        if position < native_start {
            Ok(position)
        } else if position < native_end {
            Ok(native_start)
        } else {
            position
                .checked_sub(removed)
                .and_then(|p| p.checked_add(width))
                .ok_or_else(|| "Reference expansion overflow".into())
        }
    };
    let shape_at = |position| {
        paragraph
            .char_shapes
            .iter()
            .rev()
            .find(|shape| shape.start_pos <= position)
            .map(|shape| shape.char_shape_id)
    };
    let source = shape_at(native_start).ok_or("Reference expansion has no character format")?;
    let following = shape_at(native_end).unwrap_or(source);
    let inserted_count = expanded_text.chars().count();
    let text_index = |index: usize| {
        if index >= end {
            index - count + inserted_count
        } else {
            index
        }
    };
    let mut edited = paragraph.clone();
    edited
        .text
        .replace_range(byte_start..byte_start + marker.len(), expanded_text);
    let mut offset = native_start;
    let mut offsets = Vec::with_capacity(inserted_count);
    for character in expanded_text.chars() {
        offsets.push(offset);
        offset = offset
            .checked_add(character.len_utf16() as u32)
            .ok_or("Reference expansion cache overflow")?;
    }
    for offset in &mut edited.char_offsets[end..] {
        *offset = remap(*offset)?;
    }
    edited.char_offsets.splice(start..end, offsets);
    edited
        .char_shapes
        .retain(|shape| shape.start_pos < native_start || shape.start_pos >= native_end);
    for shape in &mut edited.char_shapes {
        shape.start_pos = remap(shape.start_pos)?;
    }
    edited.char_shapes.extend([
        CharShapeRef {
            start_pos: native_start,
            char_shape_id: source,
        },
        CharShapeRef {
            start_pos: new_end,
            char_shape_id: following,
        },
    ]);
    edited.char_shapes.sort_by_key(|shape| shape.start_pos);
    let mut shapes: Vec<CharShapeRef> = Vec::new();
    for shape in edited.char_shapes.drain(..) {
        if let Some(previous) = shapes.last_mut()
            && previous.start_pos == shape.start_pos
        {
            *previous = shape;
        } else {
            shapes.push(shape);
        }
    }
    edited.char_shapes = shapes;
    for range in &mut edited.field_ranges {
        range.start_char_idx = text_index(range.start_char_idx);
        range.end_char_idx = text_index(range.end_char_idx);
    }
    for mark in &mut edited.orphan_field_ends {
        mark.char_idx = text_index(mark.char_idx);
    }
    for mark in &mut edited.title_marks {
        mark.char_idx = text_index(mark.char_idx);
    }
    for mark in &mut edited.markpen_marks {
        mark.char_idx = if mark.char_idx >= end {
            text_index(mark.char_idx)
        } else {
            mark.char_idx.min(start)
        };
        if let Some(position) = &mut mark.utf16_pos {
            *position = remap(*position)?;
        }
    }
    for range in &mut edited.range_tags {
        range.start = remap(range.start)?;
        range.end = remap(range.end)?;
    }
    edited.char_count = edited
        .char_count
        .checked_sub(removed)
        .and_then(|n| n.checked_add(width))
        .ok_or("Reference expansion paragraph overflow")?;
    edited.has_para_text = true;
    crate::template::discard_stale_layout(&mut edited);
    *paragraph = edited;
    Ok(())
}

pub(crate) fn insert(
    paragraph: &mut Paragraph,
    marker: &str,
    cached_text: &str,
    target_instance: u32,
    kind: &str,
    identity: &mut IdentityAllocator,
) -> Result<u32, String> {
    let category = match kind {
        "table_number" => 0,
        "figure_number" => 1,
        _ => return Err("Unsupported native reference kind".into()),
    };
    if marker.is_empty()
        || marker.chars().any(char::is_control)
        || cached_text.is_empty()
        || cached_text.chars().any(char::is_control)
        || target_instance == 0
    {
        return Err(
            "Native reference requires plain marker/cache and a positive target identity".into(),
        );
    }
    let byte_start = paragraph
        .text
        .find(marker)
        .ok_or("Native reference marker was not found")?;
    let start = paragraph.text[..byte_start].chars().count();
    let count = marker.chars().count();
    let end = start + count;
    let chars: Vec<char> = paragraph.text.chars().collect();
    if paragraph.char_offsets.len() != chars.len() {
        return Err("Native reference paragraph has incomplete character offsets".into());
    }
    let native_start = paragraph.char_offsets[start];
    let mut native_end = native_start;
    for (index, character) in chars.iter().enumerate().take(end).skip(start) {
        if paragraph.char_offsets[index] != native_end {
            return Err("Native reference marker crosses a native control".into());
        }
        native_end = native_end
            .checked_add(character.len_utf16() as u32)
            .ok_or("Native reference marker offset overflow")?;
    }
    let inside = |index| start < index && index < end;
    if paragraph.field_ranges.iter().any(|range| {
        range.start_char_idx < end && range.end_char_idx > start
            || inside(range.start_char_idx)
            || inside(range.end_char_idx)
    }) || paragraph
        .orphan_field_ends
        .iter()
        .any(|mark| inside(mark.char_idx))
        || paragraph
            .title_marks
            .iter()
            .any(|mark| inside(mark.char_idx))
    {
        return Err("Native reference marker overlaps a native field or title marker".into());
    }
    let positions = paragraph.control_text_positions();
    if positions.windows(2).any(|pair| pair[0] > pair[1])
        || positions.iter().any(|&position| inside(position))
    {
        return Err("Native reference marker crosses unordered or enclosed controls".into());
    }
    let control_index = positions.partition_point(|&position| position <= start);
    let cache_width = u32::try_from(cached_text.encode_utf16().count())
        .map_err(|_| "Native reference cache is too long")?;
    let width = cache_width
        .checked_add(16)
        .ok_or("Native reference width overflow")?;
    let removed = native_end - native_start;
    let remap = |position: u32| -> Result<u32, String> {
        if position < native_start {
            Ok(position)
        } else if position < native_end {
            Ok(native_start)
        } else {
            position
                .checked_sub(removed)
                .and_then(|p| p.checked_add(width))
                .ok_or_else(|| "Native reference position overflow".into())
        }
    };
    let shape_at = |position| {
        paragraph
            .char_shapes
            .iter()
            .rev()
            .find(|shape| shape.start_pos <= position)
            .map(|shape| shape.char_shape_id)
    };
    let source_shape =
        shape_at(native_start).ok_or("Native reference marker has no character format")?;
    let following_shape = shape_at(native_end).unwrap_or(source_shape);
    let new_count = paragraph
        .char_count
        .checked_sub(removed)
        .and_then(|n| n.checked_add(width))
        .ok_or("Native reference paragraph length overflow")?;
    let cache_end = native_start
        .checked_add(width)
        .ok_or("Native reference end overflow")?;
    let cache_count = cached_text.chars().count();
    let text_index = |index: usize| {
        if index >= end {
            index - count + cache_count
        } else {
            index
        }
    };
    let mut edited = paragraph.clone();
    edited
        .text
        .replace_range(byte_start..byte_start + marker.len(), cached_text);
    let mut offsets = Vec::with_capacity(cache_count);
    let mut offset = native_start
        .checked_add(8)
        .ok_or("Native reference start overflow")?;
    for character in cached_text.chars() {
        offsets.push(offset);
        offset = offset
            .checked_add(character.len_utf16() as u32)
            .ok_or("Native reference cache overflow")?;
    }
    for offset in &mut edited.char_offsets[end..] {
        *offset = remap(*offset)?;
    }
    edited.char_offsets.splice(start..end, offsets);
    edited
        .char_shapes
        .retain(|shape| shape.start_pos < native_start || shape.start_pos >= native_end);
    for shape in &mut edited.char_shapes {
        shape.start_pos = remap(shape.start_pos)?;
    }
    edited.char_shapes.extend([
        CharShapeRef {
            start_pos: native_start,
            char_shape_id: source_shape,
        },
        CharShapeRef {
            start_pos: cache_end,
            char_shape_id: following_shape,
        },
    ]);
    edited.char_shapes.sort_by_key(|shape| shape.start_pos);
    let mut shapes: Vec<CharShapeRef> = Vec::new();
    for shape in edited.char_shapes.drain(..) {
        if let Some(previous) = shapes.last_mut()
            && previous.start_pos == shape.start_pos
        {
            *previous = shape;
        } else {
            shapes.push(shape);
        }
    }
    edited.char_shapes = shapes;
    for range in &mut edited.field_ranges {
        range.start_char_idx = text_index(range.start_char_idx);
        range.end_char_idx = text_index(range.end_char_idx);
        if range.control_idx >= control_index {
            range.control_idx += 1;
        }
    }
    for mark in &mut edited.orphan_field_ends {
        mark.char_idx = text_index(mark.char_idx);
    }
    for mark in &mut edited.title_marks {
        mark.char_idx = text_index(mark.char_idx);
    }
    for mark in &mut edited.markpen_marks {
        mark.char_idx = if mark.char_idx >= end {
            text_index(mark.char_idx)
        } else {
            mark.char_idx.min(start)
        };
        if let Some(position) = &mut mark.utf16_pos {
            *position = remap(*position)?;
        }
    }
    for range in &mut edited.range_tags {
        range.start = remap(range.start)?;
        range.end = remap(range.end)?;
    }
    if edited.ctrl_data_records.len() > edited.controls.len() {
        return Err("Native reference paragraph has excess control data".into());
    }
    // Allocate only after every fallible paragraph transformation succeeds.
    let field_id = identity.id()?;
    edited.ctrl_data_records.resize(edited.controls.len(), None);
    edited.controls.insert(
        control_index,
        Control::Field(Field {
            field_type: FieldType::CrossRef,
            ctrl_id: rhwp::parser::tags::FIELD_CROSSREF,
            command: format!("?#{target_instance};{category};1;0;0"),
            field_id,
            // Native Hancom-generated %xrf CTRL_HEADERs use bit 1 even though
            // HWPML exports Editable=false, Dirty=false and Property=0.
            properties: 2,
            ..Default::default()
        }),
    );
    edited.ctrl_data_records.insert(control_index, None);
    edited.field_ranges.push(FieldRange {
        start_char_idx: start,
        end_char_idx: start + cache_count,
        control_idx: control_index,
        ..Default::default()
    });
    edited
        .field_ranges
        .sort_by_key(|range| (range.start_char_idx, range.control_idx));
    edited.char_count = new_count;
    edited.control_mask |= (1 << 3) | (1 << 4);
    edited.has_para_text = true;
    crate::template::discard_stale_layout(&mut edited);
    *paragraph = edited;
    Ok(field_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paragraph(text: &str) -> Paragraph {
        let mut p = Paragraph::new_empty();
        p.set_single_char_shape(0);
        p.insert_text_at(0, text);
        p
    }
    fn ids() -> IdentityAllocator {
        crate::model_tools::identity_seed(rhwp::DocumentCore::new_empty().document()).unwrap()
    }

    #[test]
    fn inserts_native_field_cache_with_unicode_and_source_format() {
        let mut p = paragraph("앞😀MARK뒤");
        p.set_single_char_shape(3);
        let id = insert(&mut p, "MARK", "12", 501, "figure_number", &mut ids()).unwrap();
        assert_eq!(p.text, "앞😀12뒤");
        assert_eq!(p.char_offsets, [0, 1, 11, 12, 21]);
        let Control::Field(field) = &p.controls[0] else {
            panic!()
        };
        assert_eq!(field.field_id, id);
        assert_eq!(field.command, "?#501;1;1;0;0");
        assert_eq!(field.ctrl_id, rhwp::parser::tags::FIELD_CROSSREF);
        assert_eq!(field.properties, 2);
        assert_eq!(
            (
                p.field_ranges[0].start_char_idx,
                p.field_ranges[0].end_char_idx
            ),
            (2, 4)
        );
        assert_eq!(p.char_shapes[0].char_shape_id, 3);
    }

    #[test]
    fn adjacent_references_have_distinct_identity_and_nonoverlapping_native_ranges() {
        let mut p = paragraph("ONETWO");
        let mut allocator = ids();
        let first = insert(&mut p, "ONE", "1", 51, "table_number", &mut allocator).unwrap();
        let second = insert(&mut p, "TWO", "2", 52, "table_number", &mut allocator).unwrap();
        assert_ne!(first, second);
        assert_eq!(p.text, "12");
        assert_eq!(p.char_offsets, [8, 25]);
        assert_eq!(p.char_count, 35);
        assert_eq!(p.controls.len(), 2);
        assert_eq!(p.field_ranges[1].control_idx, 1);
    }

    #[test]
    fn affix_expansion_does_not_extend_an_adjacent_native_field() {
        let mut p = paragraph("ONETWO");
        let mut allocator = ids();
        insert(&mut p, "ONE", "1", 51, "table_number", &mut allocator).unwrap();
        let before = serde_json::to_value(&p.controls[0]).unwrap();
        expand(&mut p, "TWO", "표 TWO 뒤😀").unwrap();
        assert_eq!(p.field_ranges[0].end_char_idx, 1);
        insert(&mut p, "TWO", "2", 52, "table_number", &mut allocator).unwrap();
        assert_eq!(p.text, "1표 2 뒤😀");
        assert_eq!(p.field_ranges[0].end_char_idx, 1);
        assert_eq!(
            (
                p.field_ranges[1].start_char_idx,
                p.field_ranges[1].end_char_idx
            ),
            (3, 4)
        );
        assert_eq!(p.char_offsets, [8, 17, 18, 27, 36, 37, 38]);
        assert_eq!(serde_json::to_value(&p.controls[0]).unwrap(), before);
    }

    #[test]
    fn rejects_control_gap_without_mutating_paragraph() {
        let mut p = paragraph("MARK");
        p.char_offsets[2] += 8;
        let before = serde_json::to_value(&p).unwrap();
        assert!(insert(&mut p, "MARK", "1", 51, "figure_number", &mut ids()).is_err());
        assert_eq!(serde_json::to_value(&p).unwrap(), before);
    }

    #[test]
    fn preserves_native_tabs_and_restores_following_character_format() {
        let mut p = paragraph("\tMARK\t뒤");
        p.char_offsets = vec![0, 8, 9, 10, 11, 12, 20];
        p.char_count = 22;
        p.tab_extended = vec![[0, 0, 0, 0, 0, 0, 9]; 2];
        p.char_shapes.push(CharShapeRef {
            start_pos: 12,
            char_shape_id: 7,
        });
        insert(&mut p, "MARK", "12", 501, "figure_number", &mut ids()).unwrap();
        assert_eq!(p.text, "\t12\t뒤");
        assert_eq!(p.char_offsets, [0, 16, 17, 26, 34]);
        assert_eq!(p.tab_extended, vec![[0, 0, 0, 0, 0, 0, 9]; 2]);
        assert!(
            p.char_shapes
                .iter()
                .any(|shape| shape.start_pos == 26 && shape.char_shape_id == 7)
        );
    }

    #[test]
    fn serializes_and_reopens_actual_crossref_begin_end_cache_and_neighbor_note() {
        use rhwp::model::footnote::Footnote;
        let mut p = paragraph("앞MARK뒤NOTE끝");
        crate::note_insert::insert(
            &mut p,
            "NOTE",
            Footnote {
                instance_id: 100,
                paragraphs: vec![paragraph("각주")],
                ..Default::default()
            },
        )
        .unwrap();
        let mut core = rhwp::DocumentCore::new_empty();
        core.create_blank_document_native().unwrap();
        core.insert_text_native(0, 0, 0, "static").unwrap();
        let mut document = core.document().clone();
        let mut allocator = crate::model_tools::identity_seed(&document).unwrap();
        let id = insert(&mut p, "MARK", "12", 501, "table_number", &mut allocator).unwrap();
        let expected_offsets = p.char_offsets.clone();
        document.sections[0].paragraphs.push(p);
        document.sections[0].raw_stream = None;
        document.sections[0].raw_provenance = None;
        core.set_document(document);
        let output = core.export_hwp_with_adapter_snapshot_with_report().unwrap();
        assert!(output.content_loss().is_empty());
        let reopened = rhwp::DocumentCore::from_bytes(output.bytes()).unwrap();
        let p = &reopened.document().sections[0].paragraphs[1];
        assert_eq!(p.text, "앞12뒤끝");
        assert_eq!(p.char_offsets, expected_offsets);
        assert_eq!(p.controls.len(), 2);
        let Control::Field(field) = &p.controls[0] else {
            panic!()
        };
        assert_eq!(field.field_type, FieldType::CrossRef);
        assert_eq!(field.field_id, id);
        assert_eq!(field.properties, 2);
        assert_eq!(field.command, "?#501;0;1;0;0");
        assert!(matches!(p.controls[1], Control::Footnote(_)));
        assert_eq!(p.field_ranges[0].start_char_idx, 1);
        assert_eq!(p.field_ranges[0].end_char_idx, 3);
    }
}
