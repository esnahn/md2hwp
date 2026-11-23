// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Euisoon Ahn
//! Lay out only the part of the verification copy that starts at a forced page
//! break, so a table page-break correction does not read the content before the
//! break again. rhwp derives caption, note and page numbers from document order;
//! each window is seeded with the values at its start and its numbers are checked
//! against the whole reading of the copy. Callers fall back to whole readings
//! whenever a window cannot be built or a check fails.

use rhwp::model::{
    control::{AutoNumberType, Control, NewNumber},
    document::{Document, SectionDef},
    paragraph::{NumberingMarker, Paragraph},
    shape::{ObjectNumberingType, ShapeObject},
};

/// Counter slots in the pinned reader's order: page, footnote, endnote, picture,
/// table, equation (rhwp `parser/auto_numbers.rs`).
type Counters = [u16; 6];

/// Which note body a paragraph belongs to; the reader counts note numbers per body.
#[derive(Clone, Copy)]
enum Body {
    Main,
    Footnote,
    Endnote,
}

/// Mirror of rhwp `assign_auto_numbers_in_controls`: advance the counters the
/// way the reader does when it parses `paragraphs`.
fn count(paragraphs: &[Paragraph], counters: &mut Counters, body: Body) {
    for paragraph in paragraphs {
        for control in &paragraph.controls {
            count_control(control, counters, body);
        }
    }
}

fn count_control(control: &Control, counters: &mut Counters, body: Body) {
    let category = match control {
        Control::Table(table)
            if table.common.numbering_type == ObjectNumberingType::None
                && (table.common.attr >> 26) & 7 == 6 =>
        {
            Some(ObjectNumberingType::Table)
        }
        Control::Table(table) => Some(table.common.numbering_type),
        Control::Picture(picture) => Some(picture.common.numbering_type),
        Control::Shape(shape) => Some(shape.common().numbering_type),
        Control::Equation(equation) => Some(equation.common.numbering_type),
        _ => None,
    };
    match category {
        Some(ObjectNumberingType::Picture) => counters[3] = counters[3].saturating_add(1),
        Some(ObjectNumberingType::Table) => counters[4] = counters[4].saturating_add(1),
        Some(ObjectNumberingType::Equation) => counters[5] = counters[5].saturating_add(1),
        _ => {}
    }
    let caption = |caption: &Option<rhwp::model::shape::Caption>, counters: &mut Counters| {
        if let Some(caption) = caption {
            count(&caption.paragraphs, counters, body);
        }
    };
    let drawing = |drawing: &rhwp::model::shape::DrawingObjAttr, counters: &mut Counters| {
        caption(&drawing.caption, counters);
        if let Some(text) = &drawing.text_box {
            count(&text.paragraphs, counters, body);
        }
    };
    match control {
        Control::AutoNumber(number) => {
            let index = match number.number_type {
                // Caption numbers display their object's number; no counter moves.
                AutoNumberType::Picture
                | AutoNumberType::Table
                | AutoNumberType::Equation
                | AutoNumberType::TotalPage => return,
                AutoNumberType::Page => 0,
                AutoNumberType::Footnote => {
                    if !matches!(body, Body::Footnote) && number.number > 0 {
                        return;
                    }
                    1
                }
                AutoNumberType::Endnote => {
                    if !matches!(body, Body::Endnote) && number.number > 0 {
                        return;
                    }
                    2
                }
            };
            counters[index] = counters[index].saturating_add(1);
        }
        Control::NewNumber(number) => {
            let index = match number.number_type {
                AutoNumberType::Page => 0,
                AutoNumberType::Footnote => 1,
                AutoNumberType::Endnote => 2,
                AutoNumberType::Picture => 3,
                AutoNumberType::Table => 4,
                AutoNumberType::Equation => 5,
                AutoNumberType::TotalPage => return,
            };
            counters[index] = number.number.saturating_sub(1);
        }
        Control::Table(table) => {
            for cell in &table.cells {
                count(&cell.paragraphs, counters, body);
            }
            caption(&table.caption, counters);
        }
        Control::Picture(picture) => caption(&picture.caption, counters),
        Control::Shape(shape) => match shape.as_ref() {
            ShapeObject::Line(shape) => drawing(&shape.drawing, counters),
            ShapeObject::Rectangle(shape) => drawing(&shape.drawing, counters),
            ShapeObject::Ellipse(shape) => drawing(&shape.drawing, counters),
            ShapeObject::Arc(shape) => drawing(&shape.drawing, counters),
            ShapeObject::Polygon(shape) => drawing(&shape.drawing, counters),
            ShapeObject::Curve(shape) => drawing(&shape.drawing, counters),
            ShapeObject::Group(shape) => caption(&shape.caption, counters),
            ShapeObject::Picture(shape) => caption(&shape.caption, counters),
            ShapeObject::Chart(shape) => {
                if shape.caption.is_some() {
                    caption(&shape.caption, counters);
                } else {
                    caption(&shape.drawing.caption, counters);
                }
                if let Some(text) = &shape.drawing.text_box {
                    count(&text.paragraphs, counters, body);
                }
            }
            ShapeObject::Ole(shape) => {
                if shape.caption.is_some() {
                    caption(&shape.caption, counters);
                } else {
                    caption(&shape.drawing.caption, counters);
                }
                if let Some(text) = &shape.drawing.text_box {
                    count(&text.paragraphs, counters, body);
                }
            }
        },
        Control::Header(header) => count(&header.paragraphs, counters, body),
        Control::Footer(footer) => count(&footer.paragraphs, counters, body),
        Control::Footnote(note) => count(&note.paragraphs, counters, Body::Footnote),
        Control::Endnote(note) => count(&note.paragraphs, counters, Body::Endnote),
        _ => {}
    }
}

/// Numbers rhwp shows in a paragraph and the paragraphs nested in it.
#[derive(Clone, PartialEq)]
enum Mark {
    Marker(NumberingMarker),
    Number(AutoNumberType, u16),
}

fn marks(paragraphs: &[Paragraph], out: &mut Vec<Mark>) {
    for paragraph in paragraphs {
        out.push(Mark::Marker(paragraph.numbering_marker.clone()));
        for control in &paragraph.controls {
            control_marks(control, out);
        }
    }
}

fn caption_marks(caption: &Option<rhwp::model::shape::Caption>, out: &mut Vec<Mark>) {
    if let Some(caption) = caption {
        marks(&caption.paragraphs, out);
    }
}

fn drawing_marks(drawing: &rhwp::model::shape::DrawingObjAttr, out: &mut Vec<Mark>) {
    caption_marks(&drawing.caption, out);
    if let Some(text) = &drawing.text_box {
        marks(&text.paragraphs, out);
    }
}

fn control_marks(control: &Control, out: &mut Vec<Mark>) {
    match control {
        Control::AutoNumber(number) => {
            out.push(Mark::Number(number.number_type, number.assigned_number));
        }
        Control::Table(table) => {
            for cell in &table.cells {
                marks(&cell.paragraphs, out);
            }
            caption_marks(&table.caption, out);
        }
        Control::Picture(picture) => caption_marks(&picture.caption, out),
        Control::Shape(shape) => match shape.as_ref() {
            ShapeObject::Line(shape) => drawing_marks(&shape.drawing, out),
            ShapeObject::Rectangle(shape) => drawing_marks(&shape.drawing, out),
            ShapeObject::Ellipse(shape) => drawing_marks(&shape.drawing, out),
            ShapeObject::Arc(shape) => drawing_marks(&shape.drawing, out),
            ShapeObject::Polygon(shape) => drawing_marks(&shape.drawing, out),
            ShapeObject::Curve(shape) => drawing_marks(&shape.drawing, out),
            ShapeObject::Group(shape) => caption_marks(&shape.caption, out),
            ShapeObject::Picture(shape) => caption_marks(&shape.caption, out),
            ShapeObject::Chart(shape) => {
                caption_marks(&shape.caption, out);
                drawing_marks(&shape.drawing, out);
            }
            ShapeObject::Ole(shape) => {
                caption_marks(&shape.caption, out);
                drawing_marks(&shape.drawing, out);
            }
        },
        Control::Header(header) => marks(&header.paragraphs, out),
        Control::Footer(footer) => marks(&footer.paragraphs, out),
        Control::Footnote(note) => marks(&note.paragraphs, out),
        Control::Endnote(note) => marks(&note.paragraphs, out),
        _ => {}
    }
}

fn paragraph_marks(paragraph: &Paragraph) -> Vec<Mark> {
    let mut out = Vec::new();
    marks(std::slice::from_ref(paragraph), &mut out);
    out
}

fn has_endnote(paragraphs: &[Paragraph]) -> bool {
    paragraphs
        .iter()
        .flat_map(|p| &p.controls)
        .any(|control| match control {
            Control::Endnote(_) => true,
            Control::Table(table) => table.cells.iter().any(|cell| has_endnote(&cell.paragraphs)),
            _ => false,
        })
}

/// Windows over the verification copy. Its root paragraphs must not change
/// between windows except for the page breaks the caller adds.
pub(crate) struct Windows {
    shell: Document,
    section: Paragraph,
    reference: Vec<Vec<Mark>>,
    counted: usize,
    counters: Counters,
}

impl Windows {
    /// None when the document has state a window cannot reproduce: content before
    /// the section paragraph, several columns, body page-number fields, endnotes,
    /// or rhwp's shape-ratio tolerance (`uses_hwp3_origin_page_tolerance`), which
    /// a shorter document could switch off. `full` is the first whole-document
    /// reading of `copy`, whose numbers every window must reproduce.
    pub(crate) fn prepare(copy: &mut Document, full: &Document) -> Option<Self> {
        let [section] = copy.sections.as_slice() else {
            return None;
        };
        let paragraphs = &section.paragraphs;
        let first = paragraphs.first()?;
        let definition = first.controls.iter().find_map(|control| match control {
            Control::SectionDef(definition) => Some(definition),
            _ => None,
        })?;
        if !first.text.is_empty()
            || first.controls.iter().any(|control| match control {
                Control::SectionDef(_) => false,
                Control::ColumnDef(columns) => columns.column_count > 1,
                _ => true,
            })
        {
            return None;
        }
        let page_fields = paragraphs.iter().flat_map(|p| &p.controls).any(|control| {
            matches!(control, Control::AutoNumber(number)
                if matches!(number.number_type, AutoNumberType::Page | AutoNumberType::TotalPage))
        });
        let count = paragraphs.len() as f64;
        let tolerance = paragraphs.len() > 50
            && (copy.doc_info.para_shapes.len() as f64) / count < 0.05
            && (copy.doc_info.char_shapes.len() as f64) / count < 0.15;
        let [full_section] = full.sections.as_slice() else {
            return None;
        };
        if page_fields
            || tolerance
            || has_endnote(paragraphs)
            || full_section.paragraphs.len() != paragraphs.len()
        {
            return None;
        }
        let properties = &copy.doc_properties;
        let mut counters = [
            properties.page_start_num.saturating_sub(1),
            properties.footnote_start_num.saturating_sub(1),
            properties.endnote_start_num.saturating_sub(1),
            properties.picture_start_num.saturating_sub(1),
            properties.table_start_num.saturating_sub(1),
            properties.equation_start_num.saturating_sub(1),
        ];
        for (index, start) in [
            (0, definition.page_num),
            (3, definition.picture_num),
            (4, definition.table_num),
            (5, definition.equation_num),
        ] {
            if start > 0 {
                counters[index] = start - 1;
            }
        }
        let reference = full_section
            .paragraphs
            .iter()
            .map(paragraph_marks)
            .collect();
        let section = first.clone();
        let taken = std::mem::take(&mut copy.sections[0].paragraphs);
        let shell = copy.clone();
        copy.sections[0].paragraphs = taken;
        Some(Self {
            shell,
            section,
            reference,
            counted: 0,
            counters,
        })
    }

    /// The section paragraph followed by root paragraphs `start..end`, numbered as
    /// in the full document, with the page that holds `start` numbered
    /// `page_number`. `start` must not decrease between calls.
    pub(crate) fn document(
        &mut self,
        copy: &Document,
        start: usize,
        end: usize,
        page_number: usize,
    ) -> Option<Document> {
        let paragraphs = &copy.sections.first()?.paragraphs;
        if start < self.counted || start == 0 || start >= end || end > paragraphs.len() {
            return None;
        }
        count(
            &paragraphs[self.counted..start],
            &mut self.counters,
            Body::Main,
        );
        self.counted = start;
        let next = |value: u16| value.checked_add(1);
        let page = u16::try_from(page_number)
            .ok()?
            .checked_sub(1)
            .filter(|n| *n > 0)?;
        let (picture, table, equation) = (
            next(self.counters[3])?,
            next(self.counters[4])?,
            next(self.counters[5])?,
        );
        let seed = |definition: &mut SectionDef| {
            definition.page_num = page;
            definition.picture_num = picture;
            definition.table_num = table;
            definition.equation_num = equation;
        };
        let mut document = self.shell.clone();
        seed(&mut document.sections[0].section_def);
        let mut section = self.section.clone();
        for control in &mut section.controls {
            if let Control::SectionDef(definition) = control {
                seed(definition);
            }
        }
        // Note numbers start from document properties, which the copy keeps in its
        // stored DocInfo; restart them in the section paragraph instead.
        for (number_type, counter) in [
            (AutoNumberType::Footnote, self.counters[1]),
            (AutoNumberType::Endnote, self.counters[2]),
        ] {
            section
                .ctrl_data_records
                .resize(section.controls.len(), None);
            section.controls.push(Control::NewNumber(NewNumber {
                number_type,
                number: next(counter)?,
            }));
            section.ctrl_data_records.push(None);
            section.char_count = section.char_count.checked_add(8)?;
            section.control_mask |= 1 << 18;
        }
        let window = &mut document.sections[0].paragraphs;
        window.reserve(end - start + 1);
        window.push(section);
        window.extend_from_slice(&paragraphs[start..end]);
        Some(document)
    }

    /// Whether a reopened window that started at root paragraph `start` shows the
    /// same numbers and list markers as the full reading.
    pub(crate) fn numbers_match(&self, window: &Document, start: usize) -> bool {
        let Some(section) = window.sections.first() else {
            return false;
        };
        let Some(paragraphs) = section.paragraphs.get(1..) else {
            return false;
        };
        self.reference
            .get(start..start + paragraphs.len())
            .is_some_and(|reference| {
                paragraphs
                    .iter()
                    .zip(reference)
                    .all(|(paragraph, reference)| &paragraph_marks(paragraph) == reference)
            })
    }
}
