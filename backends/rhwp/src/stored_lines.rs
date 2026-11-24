// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Euisoon Ahn
//! Keep template paragraphs that the file stores without line records free of
//! the rows rhwp computes while opening it.
//!
//! Hancom can save a template without any line records, for example an HWPX it
//! never laid out (Hancom backend `init-template`). rhwp converts such an HWPX
//! to HWP without line records and marks it as converted from a Hancom HWPX;
//! `DocumentCore` then opens that HWP like an HWPX and computes one row per
//! paragraph, tagged with bit 31. The HWP serializer leaves computed rows out,
//! so an untouched static paragraph keeps a row in the composed model that its
//! saved copy does not have. The template file is the reference: a paragraph
//! it stores without rows stays without rows, as in the saved output.

use crate::model_tools::map_nested_containers;
use rhwp::model::document::Document;
use rhwp::model::paragraph::{LineSeg, Paragraph};

const MISMATCH: &str =
    "Template paragraphs could not be matched with their stored records to restore line records";

/// Remove the rows computed on open from paragraphs that `stored`, the same
/// file read with `rhwp::parser::parse_hwp`, records without rows. Rows the
/// file stores are kept unchanged.
pub(crate) fn restore(document: &mut Document, mut stored: Document) -> Result<(), String> {
    let mut computed = false;
    for section in &mut document.sections {
        computed |= has_computed_rows(&mut section.paragraphs)?;
    }
    if !computed {
        return Ok(());
    }
    if document.sections.len() != stored.sections.len() {
        return Err(MISMATCH.into());
    }
    for (section, source) in document.sections.iter_mut().zip(&mut stored.sections) {
        restore_list(&mut section.paragraphs, &mut source.paragraphs)?;
    }
    Ok(())
}

fn is_computed(row: &LineSeg) -> bool {
    row.tag & LineSeg::TAG_IMPLEMENTATION_PROPERTY != 0
}

fn has_computed_rows(paragraphs: &mut [Paragraph]) -> Result<bool, String> {
    let mut found = false;
    for paragraph in paragraphs {
        found |= paragraph.line_segs.iter().any(is_computed);
        map_nested_containers(paragraph, &mut |children| {
            found |= has_computed_rows(children)?;
            Ok(())
        })?;
    }
    Ok(found)
}

fn restore_list(paragraphs: &mut [Paragraph], stored: &mut [Paragraph]) -> Result<(), String> {
    if paragraphs.len() != stored.len() {
        return Err(MISMATCH.into());
    }
    for (paragraph, source) in paragraphs.iter_mut().zip(stored.iter_mut()) {
        // Opening may normalize field text, so match by native control kinds.
        if paragraph.controls.len() != source.controls.len()
            || paragraph
                .controls
                .iter()
                .zip(&source.controls)
                .any(|(a, b)| std::mem::discriminant(a) != std::mem::discriminant(b))
        {
            return Err(MISMATCH.into());
        }
        if source.line_segs.is_empty() && !paragraph.line_segs.is_empty() {
            if !paragraph.line_segs.iter().all(is_computed) {
                return Err(
                    "A template paragraph stored without line records has rows rhwp did not compute"
                        .into(),
                );
            }
            clear_rows(paragraph);
        }
        let mut children = Vec::new();
        map_nested_containers(source, &mut |list| {
            children.push(std::mem::take(list));
            Ok(())
        })?;
        let mut children = children.into_iter();
        map_nested_containers(paragraph, &mut |list| {
            let mut source = children.next().ok_or_else(|| MISMATCH.to_string())?;
            restore_list(list, &mut source)
        })?;
        if children.next().is_some() {
            return Err(MISMATCH.into());
        }
    }
    Ok(())
}

/// The row-owned caches upstream `Paragraph::replace_line_segs` resets. Text,
/// its axis and the edit state are unchanged: the paragraph is not edited.
fn clear_rows(paragraph: &mut Paragraph) {
    paragraph.line_segs.clear();
    paragraph.hwp3_saved_space_corrections.clear();
    paragraph.layout_object_projection = false;
    paragraph.layout_float_band = false;
    paragraph.layout_space_metrics.clear();
    paragraph.layout_only_fill_lines = 0;
    paragraph.source_line_seg_vertical_pos = None;
    paragraph.layout_rebased_body_axis = false;
}

#[cfg(test)]
mod tests {
    use super::*;
    use rhwp::model::control::Control;
    use rhwp::model::document::Section;
    use rhwp::model::table::{Cell, Table};

    fn row(tag: u32) -> LineSeg {
        LineSeg {
            line_height: 1000,
            text_height: 1000,
            baseline_distance: 850,
            line_spacing: 600,
            segment_width: 42520,
            tag,
            ..Default::default()
        }
    }

    const COMPUTED: u32 = 0x8006_0000;
    const STORED: u32 = 0x0006_0000;

    fn paragraph(rows: Vec<LineSeg>) -> Paragraph {
        Paragraph {
            line_segs: rows,
            ..Default::default()
        }
    }

    fn with_cell(outer: Vec<LineSeg>, inner: Vec<LineSeg>) -> Paragraph {
        let mut paragraph = paragraph(outer);
        paragraph.controls.push(Control::Table(Box::new(Table {
            cells: vec![Cell {
                paragraphs: vec![self::paragraph(inner)],
                ..Default::default()
            }],
            ..Default::default()
        })));
        paragraph
    }

    fn document(paragraphs: Vec<Paragraph>) -> Document {
        Document {
            sections: vec![Section {
                paragraphs,
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    fn cell_rows(document: &Document) -> &[LineSeg] {
        match &document.sections[0].paragraphs[0].controls[0] {
            Control::Table(table) => &table.cells[0].paragraphs[0].line_segs,
            _ => unreachable!(),
        }
    }

    #[test]
    fn removes_computed_rows_only_where_the_file_stores_none() {
        let mut opened = document(vec![
            with_cell(vec![row(COMPUTED)], vec![row(COMPUTED)]),
            paragraph(vec![row(STORED)]),
            paragraph(vec![row(COMPUTED)]),
        ]);
        opened.sections[0].paragraphs[2].layout_only_fill_lines = 1;
        opened.sections[0].paragraphs[2].source_line_seg_vertical_pos = Some(vec![0]);
        let stored = document(vec![
            with_cell(vec![], vec![]),
            paragraph(vec![row(STORED)]),
            paragraph(vec![row(COMPUTED)]),
        ]);
        restore(&mut opened, stored).unwrap();
        let paragraphs = &opened.sections[0].paragraphs;
        assert!(paragraphs[0].line_segs.is_empty());
        assert!(cell_rows(&opened).is_empty());
        assert!(!paragraphs[0].stored_text_partition_is_dirty());
        // Stored rows stay exactly as read, including stored computed rows.
        assert_eq!(paragraphs[1].line_segs, vec![row(STORED)]);
        assert_eq!(paragraphs[2].line_segs, vec![row(COMPUTED)]);
        assert_eq!(paragraphs[2].layout_only_fill_lines, 1);
        assert_eq!(paragraphs[2].source_line_seg_vertical_pos, Some(vec![0]));
    }

    #[test]
    fn leaves_documents_without_computed_rows_untouched() {
        let mut opened = document(vec![paragraph(vec![row(STORED)]), paragraph(vec![])]);
        // Mismatched structure is not inspected when nothing was computed.
        restore(&mut opened, document(vec![])).unwrap();
        assert_eq!(
            opened.sections[0].paragraphs[0].line_segs,
            vec![row(STORED)]
        );
    }

    #[test]
    fn rejects_unexpected_rows_and_unmatched_structure() {
        let rejects = |opened: Vec<Paragraph>, stored: Vec<Paragraph>| {
            restore(&mut document(opened), document(stored)).unwrap_err()
        };
        let error = rejects(
            vec![paragraph(vec![row(COMPUTED), row(STORED)])],
            vec![paragraph(vec![])],
        );
        assert!(error.contains("did not compute"), "{error}");
        for (opened, stored) in [
            (vec![paragraph(vec![row(COMPUTED)])], vec![]),
            (
                vec![with_cell(vec![row(COMPUTED)], vec![])],
                vec![paragraph(vec![])],
            ),
            (
                vec![with_cell(vec![], vec![row(COMPUTED)])],
                vec![with_cell(vec![], vec![]), paragraph(vec![])],
            ),
        ] {
            assert!(rejects(opened, stored).contains("could not be matched"));
        }
    }
}
