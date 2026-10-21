// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Euisoon Ahn
//! Native tables are composed from the template's editable two-column samples.

use md2hwp_core::ir::{Inline, SourceParagraph, TableAlignment};
use rhwp::model::{
    control::{AutoNumberType, Control},
    document::DocInfo,
    paragraph::Paragraph,
    raw_provenance::record_digest,
    shape::{CaptionDirection, ObjectNumberingType, SizeCriterion},
    style::{Alignment, BorderFill, BorderLineType, CenterLine, FillType, HeadType},
    table::{Cell, Table},
};

const HEADER: &str = "{{md2hwp:slot:table.header}}";
const CONTENT: &str = "{{md2hwp:slot:table.content}}";
const SOURCE: &str = "{{md2hwp:slot:table.source}}";
const PREFIX: &str = "{{md2hwp:slot:table.source.prefix}}";
const CAPTION: &str = "{{md2hwp:slot:table.caption}}";
const CHAPTER: &str = "{{md2hwp:num:heading1}}";

pub(crate) struct Prototype {
    anchor: Paragraph,
    before: Vec<Paragraph>,
    after: Vec<Paragraph>,
    width: i32,
}

impl Prototype {
    pub(crate) fn table(&self) -> &Table {
        let Control::Table(t) = &self.anchor.controls[0] else {
            unreachable!()
        };
        t
    }

    pub(crate) fn header_sample(&self) -> &Cell {
        &self.table().cells[0]
    }
    pub(crate) fn body_sample(&self) -> &Cell {
        &self.table().cells[2]
    }
    pub(crate) fn width_hwp_units(&self) -> i32 {
        self.width
    }

    pub(crate) fn read(roots: &[Paragraph], info: &DocInfo) -> Result<Self, String> {
        let boundary = |name: &str| -> Result<usize, String> {
            let token = crate::template::tag(name);
            let matches: Vec<_> = roots
                .iter()
                .enumerate()
                .filter(|(_, p)| p.text == token)
                .collect();
            if matches.len() != 1 {
                return Err(format!("Table requires exactly one plain root {token}"));
            }
            if name != "begin:template" {
                plain(matches[0].1, info)?;
            }
            Ok(matches[0].0)
        };
        let begin = boundary("begin:table")?;
        let end = boundary("end:table")?;
        let template_begin = boundary("begin:template")?;
        let template_end = boundary("end:template")?;
        if begin <= template_begin || end >= template_end || end <= begin + 1 || end - begin > 65 {
            return Err("Table requires one native sample with 0–63 empty spacing paragraphs inside template definitions".into());
        }
        let settings: Vec<_> = roots
            .iter()
            .enumerate()
            .filter_map(|(i, p)| {
                p.text
                    .strip_prefix("{{md2hwp:table.width-mm:")
                    .and_then(|s| s.strip_suffix("}}"))
                    .map(|s| (i, p, s))
            })
            .collect();
        if settings.len() != 1 || settings[0].0 <= template_begin || settings[0].0 >= begin {
            return Err(
                "Table requires exactly one table.width-mm before begin:table inside definitions"
                    .into(),
            );
        }
        plain(settings[0].1, info)?;
        let text = settings[0].2;
        let mm = text.parse::<f64>().map_err(|_| "Invalid table.width-mm")?;
        let hu = (mm * 7200.0 / 25.4).round();
        if !text.chars().all(|c| c.is_ascii_digit() || c == '.')
            || !hu.is_finite()
            || hu < 1.0
            || hu > f64::from(i32::MAX)
        {
            return Err(
                "table.width-mm must be a positive decimal width within native HWPUNIT limits"
                    .into(),
            );
        }
        let range = &roots[begin + 1..end];
        let anchors: Vec<_> = range
            .iter()
            .enumerate()
            .filter(|(_, p)| !p.controls.is_empty())
            .collect();
        if anchors.len() != 1 {
            return Err(
                "Table range requires one native table anchor and only empty spacing paragraphs"
                    .into(),
            );
        }
        let (index, anchor) = anchors[0];
        for p in range[..index].iter().chain(&range[index + 1..]) {
            plain(p, info)?;
            if !p.text.is_empty()
                || info.para_shapes[usize::from(p.para_shape_id)].head_type != HeadType::None
            {
                return Err("Table spacing requires empty unnumbered paragraphs".into());
            }
        }
        formats(anchor, info)?;
        if !anchor.text.is_empty()
            || anchor.controls.len() != 1
            || !anchor.field_ranges.is_empty()
            || !anchor.orphan_field_ends.is_empty()
            || !anchor.range_tags.is_empty()
            || !anchor.title_marks.is_empty()
            || !anchor.markpen_marks.is_empty()
        {
            return Err("Table anchor requires only its native table".into());
        }
        let Control::Table(table) = &anchor.controls[0] else {
            return Err("Table sample must be a native table".into());
        };
        if table.row_count != 3 || table.col_count != 2 || table.cells.len() != 5 {
            return Err("Table requires a three-row, two-column prototype: two header cells, two content cells, one merged source cell. Regenerate with init-template or explicitly adopt the two-column prototype".into());
        }
        if !table.zones.is_empty() {
            return Err("Table prototype cell-zone overlays are unsupported; use each sample cell's border/fill".into());
        }
        for (i, cell) in table.cells.iter().enumerate() {
            let (row, col, span, slot) = if i < 4 {
                (
                    (i / 2) as u16,
                    (i % 2) as u16,
                    1,
                    if i < 2 { HEADER } else { CONTENT },
                )
            } else {
                (2, 0, 2, SOURCE)
            };
            if cell.row != row
                || cell.col != col
                || cell.row_span != 1
                || cell.col_span != span
                || cell.paragraphs.len() != 1
            {
                return Err("Table sample addresses/spans must match the three-row two-column grid, with one paragraph per cell".into());
            }
            border(cell, table, info)?;
            let p = &cell.paragraphs[0];
            plain(p, info)?;
            if i < 4 && p.text != slot {
                return Err("Table header/content slots must occupy their whole paragraph".into());
            }
            if i == 4
                && (p.text.matches(SOURCE).count() != 1
                    || p.text.matches(PREFIX).count() != 1
                    || !p.text.contains(&format!("{PREFIX}: {SOURCE}"))
                    || p.text
                        .replace(SOURCE, "")
                        .replace(PREFIX, "")
                        .contains("{{md2hwp:"))
            {
                return Err("Table source requires slot:table.source.prefix followed by ': ' and slot:table.source".into());
            }
        }
        let source_border = border(&table.cells[4], table, info)?;
        if source_border.borders[2].line_type != BorderLineType::None
            && source_border.borders[2] != border(&table.cells[2], table, info)?.borders[3]
        {
            return Err(
                "Table source top border must match the content sample bottom border exactly"
                    .into(),
            );
        }
        let transparent_fill = source_border.fill.fill_type == FillType::None
            || (source_border.fill.fill_type == FillType::Solid
                && source_border
                    .fill
                    .solid
                    .is_some_and(|s| s.background_color == u32::MAX && s.pattern_type == -1));
        if [0, 1, 3]
            .iter()
            .any(|&i| source_border.borders[i].line_type != BorderLineType::None)
            || source_border.attr != 0
            || source_border.three_d
            || source_border.center_line != CenterLine::None
            || !transparent_fill
            || source_border.fill.gradient.is_some()
            || source_border.fill.image.is_some()
        {
            return Err("Table source requires no side/bottom borders, diagonal lines, background or hatch fill".into());
        }
        let caption = table
            .caption
            .as_ref()
            .ok_or("Table requires a native numbered caption prototype")?;
        if caption.paragraphs.len() != 1 {
            return Err("Table native caption requires exactly one paragraph".into());
        }
        let p = &caption.paragraphs[0];
        formats(p, info)?;
        if p.controls.len() != 1
            || !matches!(&p.controls[0], Control::AutoNumber(n) if n.number_type == AutoNumberType::Table)
            || p.text.matches(CAPTION).count() != 1
            || p.text
                .replace(CAPTION, "")
                .replace(CHAPTER, "")
                .contains("{{md2hwp:")
            || p.raw_break_type & 0x0c != 0
            || !p.field_ranges.is_empty()
            || !p.orphan_field_ends.is_empty()
        {
            return Err("Table caption requires one Table AUTONUM, one caption slot and optional num:heading1, without other controls or page breaks".into());
        }
        let mut probe = p.clone();
        crate::template::replace(&mut probe, CAPTION, "")?;
        while probe.text.contains(CHAPTER) {
            crate::template::replace(&mut probe, CHAPTER, "")?;
        }
        let mut counts = [0; 8];
        fn count(p: &Paragraph, counts: &mut [usize; 8]) -> Result<(), String> {
            for (i, slot) in [
                HEADER,
                CONTENT,
                SOURCE,
                PREFIX,
                CAPTION,
                "{{md2hwp:begin:table}}",
                "{{md2hwp:end:table}}",
                "{{md2hwp:table.width-mm:",
            ]
            .iter()
            .enumerate()
            {
                counts[i] += p.text.matches(slot).count();
            }
            let mut cursor = p.text.as_str();
            while let Some((_, tail)) = cursor.split_once("{{md2hwp:") {
                let Some((token, rest)) = tail.split_once("}}") else {
                    if tail.starts_with("table.") || tail.starts_with("slot:table.") {
                        return Err("Malformed table declaration".into());
                    }
                    break;
                };
                if (token.starts_with("table.") || token.starts_with("slot:table."))
                    && !matches!(
                        token,
                        "slot:table.header"
                            | "slot:table.content"
                            | "slot:table.source"
                            | "slot:table.source.prefix"
                            | "slot:table.caption"
                    )
                    && !token.starts_with("table.width-mm:")
                {
                    return Err(format!("Unsupported table declaration {token}"));
                }
                cursor = rest;
            }
            let mut p = p.clone();
            crate::model_tools::map_nested_containers(&mut p, &mut |ps| {
                for p in ps {
                    count(p, counts)?;
                }
                Ok(())
            })
        }
        for p in roots {
            count(p, &mut counts)?;
        }
        if counts != [2, 2, 1, 1, 1, 1, 1, 1] {
            return Err("Duplicate or misplaced table declarations/slots".into());
        }
        Ok(Self {
            anchor: anchor.clone(),
            before: range[..index].to_vec(),
            after: range[index + 1..].to_vec(),
            width: hu as i32,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render(
        &self,
        columns: &[TableAlignment],
        header: &[Vec<Inline>],
        rows: &[Vec<Vec<Inline>>],
        caption: Option<&[Inline]>,
        source: Option<&[SourceParagraph]>,
        chapter: Option<u32>,
        number: u32,
        widths: &[i32],
        info: &mut DocInfo,
    ) -> Result<Vec<Paragraph>, String> {
        let count = columns.len();
        let row_count = 1 + rows.len() + usize::from(source.is_some());
        if count == 0
            || count > i16::MAX as usize
            || row_count > u16::MAX as usize
            || row_count
                .checked_mul(count)
                .is_none_or(|n| n > rhwp::model::table::MAX_TABLE_GRID_CELLS)
            || header.len() != count
            || rows.iter().any(|r| r.len() != count)
            || widths.len() != count
            || widths.iter().any(|w| *w <= 0)
            || widths.iter().map(|w| i64::from(*w)).sum::<i64>() != i64::from(self.width)
        {
            return Err("Table grid/positive column widths must fit native limits and sum to table.width-mm".into());
        }
        let mut anchor = self.anchor.clone();
        let Control::Table(table) = &mut anchor.controls[0] else {
            unreachable!()
        };
        let samples = table.cells.clone();
        let mut cells = Vec::new();
        for (row, values) in std::iter::once(header)
            .chain(rows.iter().map(Vec::as_slice))
            .enumerate()
        {
            let sample = &samples[if row == 0 { 0 } else { 2 }];
            let right = &samples[if row == 0 { 1 } else { 3 }];
            for (col, value) in values.iter().enumerate() {
                let mut cell = sample.clone();
                let mut edges = border(sample, table, info)?.clone();
                let rb = border(right, table, info)?;
                if col != 0 {
                    edges.borders[0] = rb.borders[0];
                }
                if col == count - 1 {
                    edges.borders[1] = rb.borders[1];
                }
                cell.border_fill_id = intern_border(edges, info)?;
                cell.col = col as u16;
                cell.row = row as u16;
                cell.col_span = 1;
                cell.row_span = 1;
                resize_cell(&mut cell, widths[col] as u32);
                cell.height = 1;
                cell.set_header(row == 0);
                fill(
                    &mut cell.paragraphs[0],
                    if row == 0 { HEADER } else { CONTENT },
                    value,
                    info,
                )?;
                align(&mut cell.paragraphs[0], &columns[col], info)?;
                cells.push(cell);
            }
        }
        if let Some(sources) = source {
            if sources.is_empty() || sources.len() > u16::MAX as usize {
                return Err("Table source requires 1–65535 paragraphs".into());
            }
            let mut cell = samples[4].clone();
            cell.col = 0;
            cell.row = (row_count - 1) as u16;
            cell.row_span = 1;
            cell.col_span = count as u16;
            resize_cell(&mut cell, self.width as u32);
            cell.height = 1;
            cell.set_header(false);
            if rows.is_empty() {
                let mut edges = border(&cell, table, info)?.clone();
                if edges.borders[2].line_type != BorderLineType::None {
                    edges.borders[2] = border(&samples[0], table, info)?.borders[3];
                    cell.border_fill_id = intern_border(edges, info)?;
                }
            }
            let sample = cell.paragraphs[0].clone();
            cell.paragraphs.clear();
            for source in sources {
                let mut p = sample.clone();
                // The original prefix precedes the source slot. Substituting
                // content first keeps both arbitrary literals opaque.
                fill(&mut p, SOURCE, &source.inlines, info)?;
                crate::template::replace(&mut p, PREFIX, &source.prefix)?;
                cell.paragraphs.push(p);
            }
            cells.push(cell);
        }
        table.cells = cells;
        table.row_count = row_count as u16;
        table.col_count = count as u16;
        table.row_sizes = vec![count as i16; row_count];
        if source.is_some() {
            *table.row_sizes.last_mut().unwrap() = 1;
        }
        table.rebuild_grid();
        if let Some(caption) = caption {
            let number = u16::try_from(number)
                .ok()
                .filter(|n| *n > 0)
                .ok_or("Table number exceeds native 1–65535 range")?;
            let native = table.caption.as_mut().unwrap();
            let p = &mut native.paragraphs[0];
            while p.text.contains(CHAPTER) {
                crate::template::replace(
                    p,
                    CHAPTER,
                    &chapter
                        .ok_or("Table caption num:heading1 requires a preceding heading1")?
                        .to_string(),
                )?;
            }
            fill(p, CAPTION, caption, info)?;
            let Control::AutoNumber(n) = &mut p.controls[0] else {
                unreachable!()
            };
            n.number = number;
            n.assigned_number = number;
            native.max_width = if matches!(
                native.direction,
                CaptionDirection::Left | CaptionDirection::Right
            ) {
                native.width
            } else {
                self.width as u32
            };
        } else {
            table.caption = None;
        }
        patch_common(table, self.width as u32, row_count as u32)?;
        crate::code::compose_empty_cell_lines(table, info)?;
        crate::template::discard_stale_layout(&mut anchor);
        let mut output = self.before.clone();
        output.push(anchor);
        output.extend(self.after.clone());
        Ok(output)
    }
}

fn resize_cell(cell: &mut Cell, width: u32) {
    cell.shift_text_area_width(i64::from(width) - i64::from(cell.width));
    cell.width = width;
}

fn patch_common(table: &mut Table, width: u32, height: u32) -> Result<(), String> {
    let valid = table
        .raw_ctrl_seal
        .is_some_and(|s| s == record_digest(&table.common));
    let numbered = table.caption.is_some();
    table.common.width = width;
    table.common.height = height;
    table.common.width_criterion = SizeCriterion::Absolute;
    table.common.height_criterion = SizeCriterion::Absolute;
    table.common.size_protect = false;
    table.common.numbering_type = if numbered {
        ObjectNumberingType::Table
    } else {
        ObjectNumberingType::None
    };
    let mask = (7 << 15) | (3 << 18) | (1 << 20) | (7 << 26) | (1 << 29);
    let bits = (4 << 15) | (2 << 18) | if numbered { (2 << 26) | (1 << 29) } else { 0 };
    table.common.attr = (table.common.attr & !mask) | bits;
    if !table.raw_ctrl_data.is_empty() {
        if table.raw_ctrl_data.len() < 36 {
            return Err("Truncated table native object header".into());
        }
        let old = u32::from_le_bytes(table.raw_ctrl_data[..4].try_into().unwrap());
        table.raw_ctrl_data[..4].copy_from_slice(&((old & !mask) | bits).to_le_bytes());
        table.raw_ctrl_data[12..16].copy_from_slice(&width.to_le_bytes());
        table.raw_ctrl_data[16..20].copy_from_slice(&height.to_le_bytes());
    }
    if valid {
        table.raw_ctrl_seal = Some(record_digest(&table.common));
    }
    Ok(())
}

fn border<'a>(cell: &Cell, table: &Table, info: &'a DocInfo) -> Result<&'a BorderFill, String> {
    let id = if cell.border_fill_id == 0 {
        table.border_fill_id
    } else {
        cell.border_fill_id
    };
    id.checked_sub(1)
        .and_then(|i| info.border_fills.get(usize::from(i)))
        .ok_or("Table sample references an absent border/fill".into())
}

fn intern_border(mut value: BorderFill, info: &mut DocInfo) -> Result<u16, String> {
    // Only the four edge records change. Preserve all other opaque native bytes.
    if let Some(raw) = &mut value.raw_data {
        for (i, b) in value.borders.iter().enumerate() {
            let offset = 2 + i * 6;
            let dest = raw
                .get_mut(offset..offset + 6)
                .ok_or("Truncated table border/fill record")?;
            dest[0] = b.line_type as u8;
            dest[1] = b.width;
            dest[2..].copy_from_slice(&b.color.to_le_bytes());
        }
    }
    if let Some(index) = info.border_fills.iter().position(|b| *b == value) {
        return u16::try_from(index + 1)
            .map_err(|_| "Too many table border/fill definitions".into());
    }
    let id = u16::try_from(info.border_fills.len() + 1)
        .map_err(|_| "Too many table border/fill definitions")?;
    info.border_fills.push(value);
    info.raw_stream_dirty = true;
    Ok(id)
}

fn fill(
    p: &mut Paragraph,
    slot: &str,
    inlines: &[Inline],
    info: &mut DocInfo,
) -> Result<(), String> {
    crate::template::check_inlines(inlines)?;
    let byte = p.text.find(slot).ok_or("Table slot disappeared")?;
    let base = p
        .char_shape_id_at(p.text[..byte].chars().count())
        .ok_or("Table slot has no character format")?;
    let mut spans = Vec::new();
    crate::template::flatten(inlines, false, false, &mut spans);
    crate::template::replace_rich(p, slot, &spans, base, info)?;
    if p.text.is_empty() {
        p.set_single_char_shape(base);
    }
    Ok(())
}

fn align(p: &mut Paragraph, value: &TableAlignment, info: &mut DocInfo) -> Result<(), String> {
    let (alignment, bits) = match value {
        TableAlignment::Default => return Ok(()),
        TableAlignment::Left => (Alignment::Left, 1),
        TableAlignment::Right => (Alignment::Right, 2),
        TableAlignment::Center => (Alignment::Center, 3),
    };
    let mut shape = info
        .para_shapes
        .get(usize::from(p.para_shape_id))
        .ok_or("Table has an absent paragraph format")?
        .clone();
    if shape.alignment == alignment {
        return Ok(());
    }
    shape.alignment = alignment;
    shape.attr1 = (shape.attr1 & !(7 << 2)) | (bits << 2);
    if let Some(raw) = &mut shape.raw_data {
        raw.get_mut(..4)
            .ok_or("Truncated table paragraph format")?
            .copy_from_slice(&shape.attr1.to_le_bytes());
    }
    let key = serde_json::to_value(&shape).map_err(|e| e.to_string())?;
    let index = info
        .para_shapes
        .iter()
        .position(|s| serde_json::to_value(s).is_ok_and(|v| v == key))
        .unwrap_or(info.para_shapes.len());
    p.para_shape_id = u16::try_from(index).map_err(|_| "Too many table paragraph formats")?;
    if index == info.para_shapes.len() {
        info.para_shapes.push(shape);
        info.raw_stream_dirty = true;
    }
    Ok(())
}

fn formats(p: &Paragraph, info: &DocInfo) -> Result<(), String> {
    if info.para_shapes.get(usize::from(p.para_shape_id)).is_none()
        || p.char_shapes.is_empty()
        || p.char_shapes
            .iter()
            .any(|s| info.char_shapes.get(s.char_shape_id as usize).is_none())
    {
        return Err("Table sample references an absent paragraph/character format".into());
    }
    Ok(())
}

fn plain(p: &Paragraph, info: &DocInfo) -> Result<(), String> {
    crate::template::require_plain(p)?;
    formats(p, info)?;
    if !p.orphan_field_ends.is_empty() || p.raw_break_type & 0x0c != 0 {
        return Err("Table sample has orphan field ends or page/column breaks".into());
    }
    let mut probe = p.clone();
    crate::template::replace(&mut probe, &p.text, "")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rhwp::model::{
        control::AutoNumber,
        shape::Caption,
        style::{CharShape, ParaShape},
        table::TablePageBreak,
    };

    fn p(text: &str) -> Paragraph {
        let mut p = Paragraph::new_empty();
        p.set_single_char_shape(0);
        p.insert_text_at(0, text);
        p
    }
    fn text(value: &str) -> Vec<Inline> {
        vec![Inline::Text {
            value: value.into(),
        }]
    }
    fn fixture() -> (Vec<Paragraph>, DocInfo) {
        let mut left = BorderFill::default();
        left.borders[0].color = 11;
        left.borders[1].color = 12;
        left.borders[3].color = 13;
        let mut right = left.clone();
        right.borders[0].color = 21;
        right.borders[1].color = 22;
        let mut source = BorderFill::default();
        for b in &mut source.borders {
            b.line_type = BorderLineType::None;
        }
        source.borders[2] = left.borders[3];
        let mut cells = Vec::new();
        for i in 0..5 {
            let mut cell = Cell {
                row: if i < 4 { i / 2 } else { 2 },
                col: if i < 4 { i % 2 } else { 0 },
                row_span: 1,
                col_span: if i < 4 { 1 } else { 2 },
                width: 1000,
                height: 2345,
                border_fill_id: if i == 4 {
                    3
                } else if i % 2 == 0 {
                    1
                } else {
                    2
                },
                paragraphs: vec![p(if i < 2 {
                    HEADER
                } else if i < 4 {
                    CONTENT
                } else {
                    SOURCE
                })],
                raw_list_extra: vec![0; 13],
                ..Default::default()
            };
            cell.raw_list_extra[..2].copy_from_slice(&1030u16.to_le_bytes());
            cells.push(cell);
        }
        cells[4].paragraphs = vec![p(&format!("({PREFIX}: {SOURCE})"))];
        let mut caption = p(&format!("표 {CHAPTER}-{CAPTION}"));
        caption.controls.push(Control::AutoNumber(AutoNumber {
            number_type: AutoNumberType::Table,
            ..Default::default()
        }));
        let mut table = Table {
            row_count: 3,
            col_count: 2,
            cells,
            repeat_header: true,
            page_break: TablePageBreak::CellBreak,
            caption: Some(Caption {
                spacing: 123,
                paragraphs: vec![caption],
                ..Default::default()
            }),
            ..Default::default()
        };
        table.raw_ctrl_data = vec![0xa5; 40];
        table.raw_ctrl_seal = Some(record_digest(&table.common));
        let mut anchor = p("");
        anchor.controls.push(Control::Table(Box::new(table)));
        (
            vec![
                p("{{md2hwp:begin:template}}"),
                p("{{md2hwp:table.width-mm:10}}"),
                p("{{md2hwp:begin:table}}"),
                p(""),
                anchor,
                p(""),
                p("{{md2hwp:end:table}}"),
                p("{{md2hwp:end:template}}"),
            ],
            DocInfo {
                para_shapes: vec![ParaShape::default()],
                char_shapes: vec![CharShape::default()],
                border_fills: vec![left, right, source],
                ..Default::default()
            },
        )
    }
    fn output_table(ps: &[Paragraph]) -> &Table {
        let Control::Table(t) = &ps[1].controls[0] else {
            panic!()
        };
        t
    }

    #[test]
    fn asymmetric_edges_header_flags_source_rows_and_native_options() {
        let (roots, mut info) = fixture();
        let proto = Prototype::read(&roots, &info).unwrap();
        let widths = [900, 900, proto.width_hwp_units() - 1800];
        let sources = [
            SourceParagraph {
                prefix: "출처".into(),
                inlines: text(PREFIX),
            },
            SourceParagraph {
                prefix: SOURCE.into(),
                inlines: text("둘"),
            },
        ];
        let ps = proto
            .render(
                &[
                    TableAlignment::Default,
                    TableAlignment::Center,
                    TableAlignment::Right,
                ],
                &[text("a"), text("b"), vec![]],
                &[vec![text("c"), text("d"), text("e")]],
                Some(&text("제목")),
                Some(&sources),
                Some(4),
                7,
                &widths,
                &mut info,
            )
            .unwrap();
        let t = output_table(&ps);
        assert_eq!(t.row_count, 3);
        assert_eq!(t.col_count, 3);
        assert_eq!(t.row_sizes, [3, 3, 1]);
        assert!(t.repeat_header);
        assert_eq!(t.page_break, TablePageBreak::CellBreak);
        for (col, width) in widths.iter().enumerate() {
            let b = border(&t.cells[col], t, &info).unwrap();
            assert_eq!(b.borders[0].color, if col == 0 { 11 } else { 21 });
            assert_eq!(b.borders[1].color, if col == 2 { 22 } else { 12 });
            assert!(t.cells[col].is_header);
            assert_ne!(t.cells[col].list_header_width_ref & 4, 0);
            assert!(!t.cells[col + 3].is_header);
            assert_eq!(
                u16::from_le_bytes(t.cells[col].raw_list_extra[..2].try_into().unwrap()),
                (width + 30) as u16
            );
        }
        assert_eq!(t.cells[6].col_span, 3);
        assert_eq!(t.cells[6].paragraphs[0].text, format!("(출처: {PREFIX})"));
        assert_eq!(t.cells[6].paragraphs[1].text, format!("({SOURCE}: 둘)"));
        let caption = t.caption.as_ref().unwrap();
        assert_eq!(caption.spacing, 123);
        assert_eq!(caption.paragraphs[0].text, "표 4-제목");
        let Control::AutoNumber(n) = &caption.paragraphs[0].controls[0] else {
            panic!()
        };
        assert_eq!(n.number, 7);
        assert_eq!(t.common.numbering_type, ObjectNumberingType::Table);
        assert_eq!(t.raw_ctrl_seal, Some(record_digest(&t.common)));
        assert_eq!(
            &t.raw_ctrl_data[12..16],
            &(proto.width_hwp_units() as u32).to_le_bytes()
        );
        assert_eq!(
            info.para_shapes[usize::from(t.cells[1].paragraphs[0].para_shape_id)].alignment,
            Alignment::Center
        );
        assert_eq!(t.cells[2].paragraphs[0].line_segs.len(), 1);
        assert_eq!(proto.table().cells[0].width, 1000);
    }

    #[test]
    fn one_column_combines_outer_edges_omits_caption_and_adjusts_source_boundary() {
        let (roots, mut info) = fixture();
        let proto = Prototype::read(&roots, &info).unwrap();
        let sources = [SourceParagraph {
            prefix: "출처".into(),
            inlines: text("자료"),
        }];
        let ps = proto
            .render(
                &[TableAlignment::Default],
                &[text("a")],
                &[],
                None,
                Some(&sources),
                None,
                1,
                &[proto.width_hwp_units()],
                &mut info,
            )
            .unwrap();
        let t = output_table(&ps);
        let b = border(&t.cells[0], t, &info).unwrap();
        assert_eq!(b.borders[0].color, 11);
        assert_eq!(b.borders[1].color, 22);
        assert!(t.caption.is_none());
        assert_eq!(t.common.numbering_type, ObjectNumberingType::None);
        assert_eq!(t.row_sizes, [1, 1]);
        assert_eq!(
            border(&t.cells[1], t, &info).unwrap().borders[2],
            b.borders[3]
        );
    }

    #[test]
    fn alignment_patches_only_native_attr_bits_and_interns() {
        let mut info = DocInfo {
            para_shapes: vec![ParaShape {
                attr1: 0xff00_0000,
                raw_data: Some(vec![0xa5; 60]),
                ..Default::default()
            }],
            ..Default::default()
        };
        let mut p = p("x");
        align(&mut p, &TableAlignment::Right, &mut info).unwrap();
        let id = p.para_shape_id;
        let shape = &info.para_shapes[usize::from(id)];
        assert_eq!(shape.attr1, 0xff00_0008);
        assert_eq!(
            &shape.raw_data.as_ref().unwrap()[..4],
            &shape.attr1.to_le_bytes()
        );
        assert!(
            shape.raw_data.as_ref().unwrap()[4..]
                .iter()
                .all(|b| *b == 0xa5)
        );
        let mut second = p.clone();
        second.para_shape_id = 0;
        align(&mut second, &TableAlignment::Right, &mut info).unwrap();
        assert_eq!(second.para_shape_id, id);
    }

    #[test]
    fn invalid_unused_prototypes_widths_and_unsupported_inlines_fail() {
        let (mut roots, mut info) = fixture();
        let proto = Prototype::read(&roots, &info).unwrap();
        assert!(
            proto
                .render(
                    &[TableAlignment::Default],
                    &[text("x")],
                    &[],
                    None,
                    None,
                    None,
                    1,
                    &[1],
                    &mut info
                )
                .is_err()
        );
        assert!(
            proto
                .render(
                    &[TableAlignment::Default],
                    &[vec![Inline::Footnote { blocks: vec![] }]],
                    &[],
                    None,
                    None,
                    None,
                    1,
                    &[proto.width_hwp_units()],
                    &mut info
                )
                .is_err()
        );
        roots.push(p(HEADER));
        assert!(Prototype::read(&roots, &info).is_err());
        roots.pop();
        let Control::Table(t) = &mut roots[4].controls[0] else {
            panic!()
        };
        t.cells[4].col_span = 1;
        assert!(Prototype::read(&roots, &info).is_err());
        let (roots, mut info) = fixture();
        info.border_fills[2].borders[0].line_type = BorderLineType::Solid;
        assert!(Prototype::read(&roots, &info).is_err());
    }

    #[test]
    fn authoritative_template_supports_rich_tables_and_preserves_prototype() {
        let core =
            rhwp::DocumentCore::from_bytes(include_bytes!("../../../templates/template.hwp"))
                .unwrap();
        let doc = core.document();
        let original = format!("{doc:?}");
        let proto = Prototype::read(&doc.sections[0].paragraphs, &doc.doc_info).unwrap();
        let mut info = doc.doc_info.clone();
        let width = proto.width_hwp_units();
        let sources = [SourceParagraph {
            prefix: "출처".into(),
            inlines: text("자료"),
        }];
        let output = proto
            .render(
                &[TableAlignment::Default, TableAlignment::Center],
                &[text("항목"), text("수치")],
                &[vec![
                    vec![Inline::Strong {
                        inlines: text("굵게"),
                    }],
                    vec![],
                ]],
                Some(&text("원래 서식")),
                Some(&sources),
                Some(3),
                1,
                &[width / 2, width - width / 2],
                &mut info,
            )
            .unwrap();
        let table = output
            .iter()
            .flat_map(|p| &p.controls)
            .find_map(|c| {
                if let Control::Table(t) = c {
                    Some(t)
                } else {
                    None
                }
            })
            .unwrap();
        assert_eq!(table.cells.len(), 5);
        assert_eq!(table.cells[2].paragraphs[0].text, "굵게");
        assert!(
            info.char_shapes[table.cells[2].paragraphs[0].char_shape_id_at(0).unwrap() as usize]
                .bold
        );
        assert_eq!(format!("{:?}", core.document()), original);
        // Exercise native storage as well as the in-memory cell grid. Build
        // the expectation from template formats and assigned widths, never
        // from the serializer's output or the reopened model.
        let mut expected = doc.clone();
        expected.doc_info = info;
        let mut output = output;
        crate::model_tools::ObjectOrders::new(doc)
            .unwrap()
            .assign(&mut output)
            .unwrap();
        let mut identities = crate::model_tools::identity_seed(doc).unwrap();
        crate::model_tools::refresh_block_identities(&mut output, &mut identities).unwrap();
        let token = crate::template::tag("begin:template");
        let mut section_anchor = doc.sections[0]
            .paragraphs
            .iter()
            .find(|p| p.text == token)
            .unwrap()
            .clone();
        crate::template::replace(&mut section_anchor, &token, "").unwrap();
        expected.sections[0].paragraphs = vec![section_anchor];
        expected.sections[0].paragraphs.extend(output);
        expected.sections[0]
            .paragraphs
            .push(doc.sections[0].paragraphs.last().unwrap().clone());
        expected.sections[0].raw_stream = None;
        expected.sections[0].raw_provenance = None;
        let mut writer = rhwp::DocumentCore::new_empty();
        writer.set_document(expected.clone());
        let saved = writer
            .export_hwp_with_adapter_snapshot_with_report()
            .unwrap();
        assert!(saved.content_loss().is_empty());
        let reopened = rhwp::DocumentCore::from_bytes(saved.bytes()).unwrap();
        let diff =
            rhwp::serializer::hwpx::roundtrip::diff_documents(&expected, reopened.document());
        assert!(diff.is_empty(), "{:?}", diff.differences);
    }
}
