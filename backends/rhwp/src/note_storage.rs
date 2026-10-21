// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Euisoon Ahn
//! Separate stored native numbers from the pinned reader's derived counters.

use std::collections::{BTreeMap, BTreeSet};

use rhwp::model::{
    control::{AutoNumber, AutoNumberType, Control},
    document::Document,
    paragraph::Paragraph,
    shape::{Caption, ShapeObject},
};
use rhwp::parser::{record::Record, tags};

type NotePayload = (Vec<u8>, Vec<Vec<u8>>);

fn word(data: &[u8], offset: usize) -> Result<u32, String> {
    let bytes = data
        .get(offset..offset + 4)
        .ok_or("Truncated native number record")?;
    Ok(u32::from_le_bytes(
        bytes
            .try_into()
            .map_err(|_| "Invalid native number record")?,
    ))
}

fn ctrl_id(record: &Record) -> Result<Option<u32>, String> {
    if record.tag_id == tags::HWPTAG_CTRL_HEADER {
        Ok(Some(word(&record.data, 0)?))
    } else {
        Ok(None)
    }
}

fn stored_number(record: &Record) -> Result<u16, String> {
    if record.data.len() != 16 {
        return Err("Unsupported native AUTONUM payload length".into());
    }
    Ok(u16::from_le_bytes([record.data[8], record.data[9]]))
}

fn visit_all(
    paragraphs: &mut [Paragraph],
    callback: &mut impl FnMut(&mut AutoNumber) -> Result<(), String>,
) -> Result<(), String> {
    for paragraph in paragraphs {
        for control in &mut paragraph.controls {
            if let Control::AutoNumber(number) = control {
                callback(number)?;
            }
        }
        crate::model_tools::map_nested_containers(paragraph, &mut |children| {
            visit_all(children, callback)
        })?;
    }
    Ok(())
}

/// Restore serialized values before cloning samples. Parsing derives `number`
/// and `assigned_number`, so neither field is evidence of a source stored value.
/// A sentinel links each modeled control to the public serializer's actual native
/// traversal. We require the complete native control-kind sequence and every
/// nonnumeric AUTONUM byte to match the retained source records.
pub(crate) fn restore_source_autonums(document: &mut Document) -> Result<(), String> {
    let mut restored = document.clone();
    for section in &mut restored.sections {
        let raw = section
            .raw_stream
            .as_ref()
            .ok_or("Source section has no retained native records")?;
        let source = Record::read_all(raw).map_err(|error| error.to_string())?;
        let mut labeled = section.paragraphs.clone();
        let mut next = 0_u32;
        visit_all(&mut labeled, &mut |number| {
            next += 1;
            let label =
                u16::try_from(next).map_err(|_| "Too many AUTONUM controls to restore safely")?;
            number.number = label;
            number.assigned_number = label;
            Ok(())
        })?;
        if next == 0 {
            continue;
        }
        let mut encoded = Vec::new();
        rhwp::serializer::body_text::serialize_paragraph_list(&labeled, 0, &mut encoded);
        let controls = |records: Vec<Record>| -> Result<Vec<Record>, String> {
            let mut result = Vec::new();
            for record in records {
                if ctrl_id(&record)?.is_some() {
                    result.push(record);
                }
            }
            Ok(result)
        };
        let source = controls(source)?;
        let encoded = controls(encoded)?;
        if source.len() != encoded.len() {
            return Err(
                "Source native control ordering cannot be matched for AUTONUM restoration".into(),
            );
        }
        let mut values = BTreeMap::new();
        for (original, labeled) in source.iter().zip(&encoded) {
            if ctrl_id(original)? != ctrl_id(labeled)? || original.level != labeled.level {
                return Err(
                    "Source native control order/levels changed before AUTONUM restoration".into(),
                );
            }
            if ctrl_id(original)? != Some(tags::CTRL_AUTO_NUMBER) {
                continue;
            }
            let value = stored_number(original)?;
            let label = stored_number(labeled)?;
            if original.data[..8] != labeled.data[..8] || original.data[10..] != labeled.data[10..]
            {
                return Err("Source AUTONUM format does not match its modeled control".into());
            }
            if values.insert(label, value).is_some() {
                return Err("Ambiguous source AUTONUM restoration label".into());
            }
        }
        if values.len() != next as usize {
            return Err("A modeled AUTONUM has no unique native source record".into());
        }
        let mut label = 0_u16;
        visit_all(&mut section.paragraphs, &mut |number| {
            label = label
                .checked_add(1)
                .ok_or("AUTONUM restoration label overflow")?;
            let value = *values
                .get(&label)
                .ok_or("Missing source AUTONUM restoration label")?;
            number.number = value;
            // parse_auto_number initializes this derived cache to zero. The
            // reader's later assignment pass intentionally skips TotalPage and
            // several containers; never invent nonzero caches in those cases.
            // Zero also preserves the serializer's number==0 fallback exactly.
            number.assigned_number = 0;
            Ok(())
        })?;
    }
    *document = restored;
    Ok(())
}

fn note_payload(records: &[Record], index: usize) -> Result<NotePayload, String> {
    let header = &records[index];
    if header.data.len() != 20 {
        return Err("Unsupported generated native FOOTNOTE header length".into());
    }
    let mut numbers = Vec::new();
    for record in records[index + 1..]
        .iter()
        .take_while(|record| record.level > header.level)
    {
        if ctrl_id(record)? == Some(tags::CTRL_AUTO_NUMBER) {
            stored_number(record)?;
            numbers.push(record.data.clone());
        }
    }
    Ok((header.data.clone(), numbers))
}

/// Verify bytes that the pinned reader overwrites before deriving expected read
/// caches. All generated note headers and all their AUTONUMs (including optional
/// continuation affixes) must match the independently composed native model.
pub(crate) fn verify_stored(
    expected: &Document,
    reopened: &Document,
    generated: &BTreeSet<u32>,
) -> Result<(), String> {
    let mut expected_notes = BTreeMap::new();
    fn collect(
        paragraphs: &mut Vec<Paragraph>,
        generated: &BTreeSet<u32>,
        notes: &mut BTreeMap<u32, NotePayload>,
    ) -> Result<(), String> {
        for paragraph in paragraphs {
            for control in &paragraph.controls {
                if let Control::Footnote(note) = control
                    && generated.contains(&note.instance_id)
                {
                    let mut records = Vec::new();
                    rhwp::serializer::control::serialize_control(control, 0, None, &mut records);
                    let payload = note_payload(&records, 0)?;
                    if notes.insert(note.instance_id, payload).is_some() {
                        return Err("Duplicate generated native footnote identity".into());
                    }
                }
            }
            crate::model_tools::map_nested_containers(paragraph, &mut |children| {
                collect(children, generated, notes)
            })?;
        }
        Ok(())
    }
    for section in &expected.sections {
        collect(
            &mut section.paragraphs.clone(),
            generated,
            &mut expected_notes,
        )?;
    }
    if expected_notes.keys().copied().collect::<BTreeSet<_>>() != *generated {
        return Err("A generated native footnote is missing from the expected model".into());
    }
    let mut found = BTreeSet::new();
    if expected.sections.len() != reopened.sections.len() {
        return Err("Native section count changed before stored-number verification".into());
    }
    for (expected_section, section) in expected.sections.iter().zip(&reopened.sections) {
        let raw = section
            .raw_stream
            .as_ref()
            .ok_or("Reopened section has no retained native records")?;
        let records = Record::read_all(raw).map_err(|error| error.to_string())?;
        let mut composed = Vec::new();
        rhwp::serializer::body_text::serialize_paragraph_list(
            &expected_section.paragraphs,
            0,
            &mut composed,
        );
        let numbers = |records: &[Record]| -> Result<Vec<(u16, Vec<u8>)>, String> {
            let mut result = Vec::new();
            for record in records {
                if ctrl_id(record)? == Some(tags::CTRL_AUTO_NUMBER) {
                    stored_number(record)?;
                    result.push((record.level, record.data.clone()));
                }
            }
            Ok(result)
        };
        if numbers(&composed)? != numbers(&records)? {
            return Err("Serialized native AUTONUM values/formats differ from the independently composed document".into());
        }
        for (index, record) in records.iter().enumerate() {
            if ctrl_id(record)? != Some(tags::CTRL_FOOTNOTE) {
                continue;
            }
            let id = word(&record.data, 16)?;
            if !generated.contains(&id) {
                continue;
            }
            if !found.insert(id) {
                return Err("Duplicate serialized generated footnote identity".into());
            }
            if expected_notes.get(&id) != Some(&note_payload(&records, index)?) {
                return Err(format!(
                    "Generated footnote {id} changed stored native header/AUTONUM data"
                ));
            }
        }
    }
    if found != *generated {
        return Err("A generated footnote is missing from the serialized native records".into());
    }
    Ok(())
}

fn counter_index(kind: AutoNumberType) -> Option<usize> {
    match kind {
        AutoNumberType::Page => Some(0),
        AutoNumberType::Footnote => Some(1),
        AutoNumberType::Endnote => Some(2),
        AutoNumberType::Picture => Some(3),
        AutoNumberType::Table => Some(4),
        AutoNumberType::Equation => Some(5),
        AutoNumberType::TotalPage => None,
    }
}

fn reader_caption(caption: &mut Option<Caption>, counters: &mut [u16; 6]) -> Result<(), String> {
    if let Some(caption) = caption {
        reader_paragraphs(&mut caption.paragraphs, counters)?;
    }
    Ok(())
}

fn reader_drawing(
    drawing: &mut rhwp::model::shape::DrawingObjAttr,
    counters: &mut [u16; 6],
) -> Result<(), String> {
    reader_caption(&mut drawing.caption, counters)?;
    if let Some(text) = &mut drawing.text_box {
        reader_paragraphs(&mut text.paragraphs, counters)?;
    }
    Ok(())
}

fn reader_paragraphs(paragraphs: &mut [Paragraph], counters: &mut [u16; 6]) -> Result<(), String> {
    for paragraph in paragraphs {
        for control in &mut paragraph.controls {
            match control {
                Control::AutoNumber(number) => {
                    if let Some(index) = counter_index(number.number_type) {
                        counters[index] = counters[index]
                            .checked_add(1)
                            .ok_or("Pinned reader AUTONUM counter overflow")?;
                        number.number = counters[index];
                        number.assigned_number = counters[index];
                    }
                }
                Control::NewNumber(number) => {
                    if let Some(index) = counter_index(number.number_type) {
                        counters[index] = number.number.saturating_sub(1);
                    }
                }
                Control::Table(table) => {
                    for cell in &mut table.cells {
                        reader_paragraphs(&mut cell.paragraphs, counters)?;
                    }
                    reader_caption(&mut table.caption, counters)?;
                }
                Control::Picture(picture) => reader_caption(&mut picture.caption, counters)?,
                Control::Shape(shape) => match shape.as_mut() {
                    ShapeObject::Line(shape) => reader_drawing(&mut shape.drawing, counters)?,
                    ShapeObject::Rectangle(shape) => reader_drawing(&mut shape.drawing, counters)?,
                    ShapeObject::Ellipse(shape) => reader_drawing(&mut shape.drawing, counters)?,
                    ShapeObject::Arc(shape) => reader_drawing(&mut shape.drawing, counters)?,
                    ShapeObject::Polygon(shape) => reader_drawing(&mut shape.drawing, counters)?,
                    ShapeObject::Curve(shape) => reader_drawing(&mut shape.drawing, counters)?,
                    ShapeObject::Group(shape) => reader_caption(&mut shape.caption, counters)?,
                    ShapeObject::Picture(shape) => reader_caption(&mut shape.caption, counters)?,
                    ShapeObject::Chart(shape) => {
                        if shape.caption.is_some() {
                            reader_caption(&mut shape.caption, counters)?;
                        } else {
                            reader_caption(&mut shape.drawing.caption, counters)?;
                        }
                        if let Some(text) = &mut shape.drawing.text_box {
                            reader_paragraphs(&mut text.paragraphs, counters)?;
                        }
                    }
                    ShapeObject::Ole(shape) => {
                        if shape.caption.is_some() {
                            reader_caption(&mut shape.caption, counters)?;
                        } else {
                            reader_caption(&mut shape.drawing.caption, counters)?;
                        }
                        if let Some(text) = &mut shape.drawing.text_box {
                            reader_paragraphs(&mut text.paragraphs, counters)?;
                        }
                    }
                },
                Control::Header(header) => reader_paragraphs(&mut header.paragraphs, counters)?,
                Control::Footer(footer) => reader_paragraphs(&mut footer.paragraphs, counters)?,
                Control::Footnote(note) => reader_paragraphs(&mut note.paragraphs, counters)?,
                Control::Endnote(note) => reader_paragraphs(&mut note.paragraphs, counters)?,
                // This deliberately matches the pinned reader: master pages,
                // hidden comments, field memos and grouped drawing children are
                // not visited by its assign_auto_numbers pass.
                _ => {}
            }
        }
    }
    Ok(())
}

/// Derive only reader cache fields from the composed model, never reopened data.
/// Call on a verification clone AFTER verify_stored; serialize the original.
/// OnPage and OnSection are native layout settings, but the pinned reader's
/// cache pass does not consult them. This mirrors that limitation precisely.
pub(crate) fn expected_reader_numbers(document: &mut Document) -> Result<(), String> {
    let properties = &document.doc_properties;
    let mut counters = [
        properties.page_start_num.saturating_sub(1),
        properties.footnote_start_num.saturating_sub(1),
        properties.endnote_start_num.saturating_sub(1),
        properties.picture_start_num.saturating_sub(1),
        properties.table_start_num.saturating_sub(1),
        properties.equation_start_num.saturating_sub(1),
    ];
    for section in &mut document.sections {
        let definition = &section.section_def;
        for (index, start) in [
            (0, definition.page_num),
            (3, definition.picture_num),
            (4, definition.table_num),
            (5, definition.equation_num),
        ] {
            if start > 0 {
                counters[index] = start.saturating_sub(1);
            }
        }
        reader_paragraphs(&mut section.paragraphs, &mut counters)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rhwp::model::footnote::Footnote;

    fn sample() -> Document {
        let mut core = rhwp::DocumentCore::new_empty();
        core.create_blank_document_native().unwrap();
        let mut document = core.document().clone();
        document.doc_properties.footnote_start_num = 1;
        let paragraph = |value| Paragraph {
            controls: vec![Control::AutoNumber(AutoNumber {
                number_type: AutoNumberType::Footnote,
                number: value,
                assigned_number: value,
                suffix_char: ')',
                ..Default::default()
            })],
            ..Default::default()
        };
        document.sections[0].paragraphs = vec![Paragraph {
            controls: vec![Control::Footnote(Box::new(Footnote {
                instance_id: 42,
                number: 7,
                paragraphs: vec![paragraph(7), paragraph(91)],
                ..Default::default()
            }))],
            ..Default::default()
        }];
        capture_raw(&mut document);
        document
    }

    fn capture_raw(document: &mut Document) {
        for section in &mut document.sections {
            let mut records = Vec::new();
            rhwp::serializer::body_text::serialize_paragraph_list(
                &section.paragraphs,
                0,
                &mut records,
            );
            section.raw_stream = Some(rhwp::serializer::record_writer::write_records(&records));
        }
    }

    fn note_values(document: &Document) -> Vec<(u16, u16)> {
        let mut document = document.clone();
        let mut result = Vec::new();
        for section in &mut document.sections {
            visit_all(&mut section.paragraphs, &mut |number| {
                result.push((number.number, number.assigned_number));
                Ok(())
            })
            .unwrap();
        }
        result
    }

    #[test]
    fn source_restoration_recovers_continuation_values_overwritten_by_reader() {
        let mut document = sample();
        expected_reader_numbers(&mut document).unwrap();
        assert_eq!(note_values(&document), vec![(1, 1), (2, 2)]);
        restore_source_autonums(&mut document).unwrap();
        assert_eq!(note_values(&document), vec![(7, 0), (91, 0)]);
    }

    #[test]
    fn stored_verification_detects_corruption_hidden_by_reader_counters() {
        let expected = sample();
        let mut reopened = expected.clone();
        expected_reader_numbers(&mut reopened).unwrap();
        let generated = BTreeSet::from([42]);
        verify_stored(&expected, &reopened, &generated).unwrap();
        let raw = reopened.sections[0].raw_stream.as_ref().unwrap();
        let mut records = Record::read_all(raw).unwrap();
        let continuation = records
            .iter_mut()
            .filter(|record| ctrl_id(record).unwrap() == Some(tags::CTRL_AUTO_NUMBER))
            .nth(1)
            .unwrap();
        continuation.data[8..10].copy_from_slice(&92_u16.to_le_bytes());
        reopened.sections[0].raw_stream =
            Some(rhwp::serializer::record_writer::write_records(&records));
        assert!(
            verify_stored(&expected, &reopened, &generated)
                .unwrap_err()
                .contains("AUTONUM")
        );
        assert_eq!(note_values(&reopened), vec![(1, 1), (2, 2)]);
    }

    #[test]
    fn source_restoration_fails_atomically_on_format_mismatch() {
        let mut document = sample();
        expected_reader_numbers(&mut document).unwrap();
        visit_all(&mut document.sections[0].paragraphs, &mut |number| {
            number.suffix_char = ']';
            Ok(())
        })
        .unwrap();
        assert!(
            restore_source_autonums(&mut document)
                .unwrap_err()
                .contains("format")
        );
        assert_eq!(note_values(&document), vec![(1, 1), (2, 2)]);
    }

    #[test]
    fn restoration_preserves_reader_zero_caches_for_total_page_and_hidden_notes() {
        use rhwp::model::control::HiddenComment;
        let mut document = sample();
        let number = |kind, value| {
            Control::AutoNumber(AutoNumber {
                number_type: kind,
                number: value,
                assigned_number: 0,
                ..Default::default()
            })
        };
        document.sections[0].paragraphs[0].controls.extend([
            number(AutoNumberType::TotalPage, 44),
            Control::HiddenComment(Box::new(HiddenComment {
                paragraphs: vec![Paragraph {
                    controls: vec![number(AutoNumberType::Footnote, 88)],
                    ..Default::default()
                }],
            })),
        ]);
        capture_raw(&mut document);
        expected_reader_numbers(&mut document).unwrap();
        let read_values = note_values(&document);
        assert_eq!(read_values, vec![(44, 0), (1, 1), (2, 2), (88, 0)]);
        restore_source_autonums(&mut document).unwrap();
        assert_eq!(
            note_values(&document),
            vec![(44, 0), (7, 0), (91, 0), (88, 0)]
        );
        expected_reader_numbers(&mut document).unwrap();
        assert_eq!(note_values(&document), read_values);
    }

    #[test]
    fn stored_numbers_remain_verified_without_generated_notes() {
        let expected = sample();
        let mut reopened = expected.clone();
        expected_reader_numbers(&mut reopened).unwrap();
        verify_stored(&expected, &reopened, &BTreeSet::new()).unwrap();
        let raw = reopened.sections[0].raw_stream.as_ref().unwrap();
        let mut records = Record::read_all(raw).unwrap();
        let number = records
            .iter_mut()
            .find(|record| ctrl_id(record).unwrap() == Some(tags::CTRL_AUTO_NUMBER))
            .unwrap();
        number.data[8..10].copy_from_slice(&99_u16.to_le_bytes());
        reopened.sections[0].raw_stream =
            Some(rhwp::serializer::record_writer::write_records(&records));
        assert!(verify_stored(&expected, &reopened, &BTreeSet::new()).is_err());
    }
}
