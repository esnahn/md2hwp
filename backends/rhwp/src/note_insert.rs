// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Euisoon Ahn
//! Replace an adapter-owned text marker with a native, non-text footnote slot.

use rhwp::model::control::Control;
use rhwp::model::footnote::Footnote;
use rhwp::model::paragraph::{CharShapeRef, Paragraph};

pub(crate) fn insert(
    paragraph: &mut Paragraph,
    marker: &str,
    note: Footnote,
) -> Result<(), String> {
    if marker.is_empty() || marker.chars().any(char::is_control) {
        return Err("Footnote marker must be nonempty plain text".into());
    }
    // Repeated heading slots can clone one private marker into a paragraph.
    // The caller supplies a fresh note for each occurrence in manuscript order.
    let byte_start = paragraph
        .text
        .find(marker)
        .ok_or("Footnote marker was not found")?;
    let start = paragraph.text[..byte_start].chars().count();
    let count = marker.chars().count();
    let end = start + count;
    let chars: Vec<char> = paragraph.text.chars().collect();
    if paragraph.char_offsets.len() != chars.len() {
        return Err("Footnote paragraph has incomplete native character offsets".into());
    }
    let native_start = paragraph.char_offsets[start];
    let slot_end = native_start
        .checked_add(8)
        .ok_or("Footnote slot offset overflow")?;
    let mut native_end = native_start;
    for (index, character) in chars.iter().enumerate().take(end).skip(start) {
        if paragraph.char_offsets[index] != native_end {
            return Err("Footnote marker crosses a native control".into());
        }
        native_end = native_end
            .checked_add(character.len_utf16() as u32)
            .ok_or("Footnote marker offset overflow")?;
    }
    let inside = |index| start < index && index < end;
    if paragraph.field_ranges.iter().any(|range| {
        (range.start_char_idx < end && range.end_char_idx > start)
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
        return Err("Footnote marker overlaps a native field or title marker".into());
    }
    let positions = paragraph.control_text_positions();
    if positions.iter().any(|&position| inside(position)) {
        return Err("Footnote marker crosses a native control".into());
    }
    // Every existing slot before the first marker character stays before the note.
    let control_index = positions.partition_point(|&position| position <= start);
    if positions.windows(2).any(|pair| pair[0] > pair[1]) {
        return Err("Footnote paragraph has unordered native controls".into());
    }
    let removed = native_end - native_start;
    let remap = |position: u32| -> Result<u32, String> {
        if position < native_start {
            Ok(position)
        } else if position < native_end {
            Ok(native_start)
        } else {
            position
                .checked_sub(removed)
                .and_then(|p| p.checked_add(8))
                .ok_or_else(|| "Footnote native position overflow".into())
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
    let reference_shape =
        shape_at(native_start).ok_or("Footnote marker has no character format")?;
    let following_shape = shape_at(native_end).unwrap_or(reference_shape);
    let new_count = paragraph
        .char_count
        .checked_sub(removed)
        .and_then(|c| c.checked_add(8))
        .ok_or("Footnote paragraph character count overflow")?;

    // Work on a detached clone: malformed offsets must never partially edit a caller.
    let mut edited = paragraph.clone();
    edited
        .text
        .replace_range(byte_start..byte_start + marker.len(), "");
    edited.char_offsets.drain(start..end);
    for offset in &mut edited.char_offsets[start..] {
        *offset = remap(*offset)?;
    }
    edited
        .char_shapes
        .retain(|shape| shape.start_pos < native_start || shape.start_pos >= native_end);
    for shape in &mut edited.char_shapes {
        shape.start_pos = remap(shape.start_pos)?;
    }
    edited.char_shapes.push(CharShapeRef {
        start_pos: native_start,
        char_shape_id: reference_shape,
    });
    edited.char_shapes.push(CharShapeRef {
        start_pos: slot_end,
        char_shape_id: following_shape,
    });
    edited.char_shapes.sort_by_key(|shape| shape.start_pos);
    // Inserted explicit boundaries own equal-position entries; preserve all other runs.
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
    let text_index = |index: usize| if index >= end { index - count } else { index };
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
            mark.char_idx - count
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
        return Err("Footnote paragraph has excess native control data".into());
    }
    edited.ctrl_data_records.resize(edited.controls.len(), None);
    edited
        .controls
        .insert(control_index, Control::Footnote(Box::new(note)));
    edited.ctrl_data_records.insert(control_index, None);
    edited.char_count = new_count;
    edited.control_mask |= 1 << 0x11;
    edited.has_para_text = true;
    crate::template::discard_stale_layout(&mut edited);
    *paragraph = edited;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rhwp::DocumentCore;

    fn text(value: &str) -> Paragraph {
        let mut offset = 0;
        let char_offsets = value
            .chars()
            .map(|character| {
                let current = offset;
                offset += if character == '\t' {
                    8
                } else {
                    character.len_utf16() as u32
                };
                current
            })
            .collect();
        Paragraph {
            text: value.into(),
            char_offsets,
            char_count: offset + 1,
            has_para_text: true,
            char_shapes: vec![CharShapeRef {
                start_pos: 0,
                char_shape_id: 0,
            }],
            ..Default::default()
        }
    }

    fn note(number: u16) -> Footnote {
        Footnote {
            number,
            instance_id: u32::from(number) + 100,
            paragraphs: vec![text("각주 본문")],
            after_decoration_letter: ')' as u16,
            ..Default::default()
        }
    }

    fn roundtrip(paragraph: Paragraph) -> Paragraph {
        let mut core = DocumentCore::new_empty();
        core.create_blank_document_native().unwrap();
        core.insert_text_native(0, 0, 0, "static root").unwrap();
        let mut document = core.document().clone();
        // Keep required section/column controls in a separate, unchanged root.
        // They must not be synthesized into the paragraph under test.
        document.sections[0].paragraphs.push(paragraph);
        document.sections[0].raw_stream = None;
        document.sections[0].raw_provenance = None;
        core.set_document(document);
        let output = core.export_hwp_with_adapter_snapshot_with_report().unwrap();
        assert!(output.content_loss().is_empty());
        let reopened = DocumentCore::from_bytes(output.bytes()).unwrap();
        assert_eq!(
            reopened.document().sections[0].paragraphs.len(),
            2,
            "reopened paragraphs: {:?}",
            reopened.document().sections[0].paragraphs
        );
        reopened.document().sections[0].paragraphs[1].clone()
    }

    fn check(
        paragraph: &Paragraph,
        expected_text: &str,
        offsets: &[u32],
        numbers: &[u16],
        positions: &[usize],
    ) {
        assert_eq!(paragraph.text, expected_text);
        assert_eq!(paragraph.char_offsets, offsets);
        assert_eq!(paragraph.control_text_positions(), positions);
        assert_eq!(
            paragraph
                .controls
                .iter()
                .map(|control| {
                    let Control::Footnote(note) = control else {
                        panic!("unexpected control")
                    };
                    assert_eq!(note.paragraphs[0].text, "각주 본문");
                    note.number
                })
                .collect::<Vec<_>>(),
            numbers
        );
    }

    #[test]
    fn unicode_middle_start_end_and_note_only_have_no_phantom_text() {
        for (input, expected, offsets, position) in [
            ("앞😀[N]뒤한", "앞😀뒤한", vec![0, 1, 11, 12], 2),
            ("[N]😀뒤", "😀뒤", vec![8, 10], 0),
            ("앞😀[N]", "앞😀", vec![0, 1], 2),
            ("[N]", "", vec![], 0),
        ] {
            let mut paragraph = text(input);
            insert(&mut paragraph, "[N]", note(7)).unwrap();
            check(&paragraph, expected, &offsets, &[7], &[position]);
            check(&roundtrip(paragraph), expected, &offsets, &[7], &[position]);
        }
    }

    #[test]
    fn preserves_tabs_and_controls_on_both_sides() {
        let mut paragraph = text("가\t[N]😀나\t끝");
        paragraph.char_offsets = vec![8, 9, 17, 18, 19, 28, 30, 31, 39];
        paragraph.char_count = 41;
        paragraph.controls = vec![
            Control::Footnote(Box::new(note(1))),
            Control::Footnote(Box::new(note(3))),
        ];
        paragraph.ctrl_data_records = vec![None, None];
        paragraph.tab_extended = vec![[0, 0, 0, 0, 0, 0, 9], [0, 0, 0, 0, 0, 0, 9]];
        insert(&mut paragraph, "[N]", note(2)).unwrap();
        // First note precedes 가, third follows the marker and precedes 😀.
        let offsets = [8, 9, 33, 35, 36, 44];
        check(&paragraph, "가\t😀나\t끝", &offsets, &[1, 2, 3], &[0, 2, 2]);
        let reopened = roundtrip(paragraph);
        check(&reopened, "가\t😀나\t끝", &offsets, &[1, 2, 3], &[0, 2, 2]);
        assert_eq!(reopened.tab_extended, vec![[0, 0, 0, 0, 0, 0, 9]; 2]);
    }

    #[test]
    fn first_marker_character_owns_note_and_following_rich_runs_survive() {
        let mut paragraph = text("A[N]BC");
        paragraph.char_shapes = vec![
            CharShapeRef {
                start_pos: 0,
                char_shape_id: 1,
            },
            CharShapeRef {
                start_pos: 1,
                char_shape_id: 2,
            },
            CharShapeRef {
                start_pos: 2,
                char_shape_id: 3,
            },
            CharShapeRef {
                start_pos: 4,
                char_shape_id: 4,
            },
            CharShapeRef {
                start_pos: 5,
                char_shape_id: 5,
            },
        ];
        insert(&mut paragraph, "[N]", note(1)).unwrap();
        let expected = vec![(0, 1), (1, 2), (9, 4), (10, 5)];
        let shapes = |p: &Paragraph| {
            p.char_shapes
                .iter()
                .map(|s| (s.start_pos, s.char_shape_id))
                .collect::<Vec<_>>()
        };
        assert_eq!(shapes(&paragraph), expected);
        assert_eq!(shapes(&roundtrip(paragraph)), expected);
    }

    #[test]
    fn adjacent_markers_keep_their_native_order() {
        let mut paragraph = text("前[N][M]後");
        insert(&mut paragraph, "[N]", note(1)).unwrap();
        insert(&mut paragraph, "[M]", note(2)).unwrap();
        check(&paragraph, "前後", &[0, 17], &[1, 2], &[1, 1]);
        check(&roundtrip(paragraph), "前後", &[0, 17], &[1, 2], &[1, 1]);
    }

    #[test]
    fn repeated_marker_occurrences_receive_independent_notes() {
        let mut paragraph = text("앞😀[N]중[N]뒤");
        insert(&mut paragraph, "[N]", note(11)).unwrap();
        assert_eq!(paragraph.text, "앞😀중[N]뒤");
        assert_eq!(paragraph.char_offsets, vec![0, 1, 11, 12, 13, 14, 15]);
        insert(&mut paragraph, "[N]", note(12)).unwrap();
        let verify = |p: &Paragraph| {
            check(p, "앞😀중뒤", &[0, 1, 11, 20], &[11, 12], &[2, 3]);
            let identities: Vec<_> = p
                .controls
                .iter()
                .map(|control| {
                    let Control::Footnote(note) = control else {
                        panic!()
                    };
                    note.instance_id
                })
                .collect();
            assert_eq!(identities, vec![111, 112]);
        };
        verify(&paragraph);
        verify(&roundtrip(paragraph));
    }

    #[test]
    fn marker_at_start_keeps_section_and_column_definitions_before_note() {
        let mut core = DocumentCore::new_empty();
        core.create_blank_document_native().unwrap();
        core.insert_text_native(0, 0, 0, "[N]뒤").unwrap();
        let paragraph = &mut core.document_mut().sections[0].paragraphs[0];
        insert(paragraph, "[N]", note(1)).unwrap();
        let verify = |p: &Paragraph| {
            assert_eq!(p.text, "뒤");
            assert_eq!(p.char_offsets, vec![24]);
            assert!(matches!(p.controls[0], Control::SectionDef(_)));
            assert!(matches!(p.controls[1], Control::ColumnDef(_)));
            assert!(matches!(p.controls[2], Control::Footnote(_)));
        };
        verify(paragraph);
        let output = core.export_hwp_with_adapter_snapshot_with_report().unwrap();
        assert!(output.content_loss().is_empty());
        let reopened = DocumentCore::from_bytes(output.bytes()).unwrap();
        verify(&reopened.document().sections[0].paragraphs[0]);
    }

    #[test]
    fn rejects_marker_crossing_a_control_without_mutation() {
        let mut paragraph = text("A[N]B");
        paragraph.char_offsets = vec![0, 1, 10, 11, 12];
        paragraph.char_count = 14;
        paragraph.controls = vec![Control::Footnote(Box::new(note(1)))];
        let before = serde_json::to_value(&paragraph).unwrap();
        assert!(insert(&mut paragraph, "[N]", note(2)).is_err());
        assert_eq!(serde_json::to_value(&paragraph).unwrap(), before);
    }

    #[test]
    fn rejects_marker_inside_field_without_mutation() {
        let mut paragraph = text("A[N]B");
        paragraph
            .field_ranges
            .push(rhwp::model::paragraph::FieldRange {
                start_char_idx: 0,
                end_char_idx: 5,
                ..Default::default()
            });
        let before = serde_json::to_value(&paragraph).unwrap();
        assert!(insert(&mut paragraph, "[N]", note(1)).is_err());
        assert_eq!(serde_json::to_value(&paragraph).unwrap(), before);
    }

    #[test]
    fn surrounding_fields_keep_their_native_identities_and_ranges() {
        let mut core = DocumentCore::new_empty();
        core.create_blank_document_native().unwrap();
        core.document_mut().sections[0]
            .paragraphs
            .push(text("A[N]B"));
        let mut document = core.document().clone();
        document.sections[0].raw_stream = None;
        document.sections[0].raw_provenance = None;
        core.set_document(document);
        core.insert_click_here_field_at(0, 1, 0, "guide", "memo", "left", true)
            .unwrap();
        core.set_field_value_by_name("left", "왼").unwrap();
        core.insert_click_here_field_at(0, 1, 6, "guide", "memo", "right", true)
            .unwrap();
        core.set_field_value_by_name("right", "오").unwrap();
        let paragraph = &mut core.document_mut().sections[0].paragraphs[1];
        assert_eq!(paragraph.text, "왼A[N]B오");
        let controls = serde_json::to_value(&paragraph.controls).unwrap();
        insert(paragraph, "[N]", note(9)).unwrap();
        let verify = |p: &Paragraph| {
            assert_eq!(p.text, "왼AB오");
            assert_eq!(
                p.field_ranges
                    .iter()
                    .map(|r| (r.start_char_idx, r.end_char_idx, r.control_idx))
                    .collect::<Vec<_>>(),
                vec![(0, 1, 0), (3, 4, 2)]
            );
            assert_eq!(
                serde_json::to_value(vec![p.controls[0].clone(), p.controls[2].clone()]).unwrap(),
                controls
            );
        };
        verify(paragraph);
        let output = core.export_hwp_with_adapter_snapshot_with_report().unwrap();
        assert!(output.content_loss().is_empty());
        let reopened = DocumentCore::from_bytes(output.bytes()).unwrap();
        verify(&reopened.document().sections[0].paragraphs[1]);
    }
}
