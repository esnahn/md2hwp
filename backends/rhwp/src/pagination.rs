// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Euisoon Ahn
//! Keep a generated floating table's first header with its first body row.
//! Positions belong to rhwp's paginator; they do not prove Hancom layout fidelity.

use std::collections::BTreeSet;

use rhwp::{
    DocumentCore,
    model::{control::Control, document::Document, paragraph::ColumnBreakType},
};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Anchor {
    paragraph: usize,
    control: usize,
    rows: usize,
    columns: usize,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Plan {
    anchors: Vec<Anchor>,
}

type Position = (usize, usize);

impl Plan {
    pub(crate) fn resolve(document: &Document, generated: &BTreeSet<u32>) -> Result<Self, String> {
        let mut seen = BTreeSet::new();
        let mut anchors = Vec::new();
        if document.sections.len() != 1 {
            return Err("Table pagination requires one section".into());
        }
        for (paragraph, p) in document.sections[0].paragraphs.iter().enumerate() {
            for (control, c) in p.controls.iter().enumerate() {
                if let Control::Table(table) = c
                    && generated.contains(&table.common.instance_id)
                {
                    if !seen.insert(table.common.instance_id)
                        || p.controls
                            .iter()
                            .filter(|c| matches!(c, Control::Table(_)))
                            .count()
                            != 1
                    {
                        return Err("Ambiguous generated table pagination anchor".into());
                    }
                    // Inline tables are indivisible in this renderer and may clip.
                    // Adding a page break cannot establish their layout safety.
                    if table.common.treat_as_char || table.row_count < 2 {
                        continue;
                    }
                    if table.col_count == 0 {
                        return Err("Generated table has no pagination columns".into());
                    }
                    anchors.push(Anchor {
                        paragraph,
                        control,
                        rows: usize::from(table.row_count),
                        columns: usize::from(table.col_count),
                    });
                }
            }
        }
        if &seen != generated {
            return Err("Missing generated table pagination anchor".into());
        }
        Ok(Self { anchors })
    }

    pub(crate) fn correct(&self, core: &mut DocumentCore) -> Result<usize, String> {
        self.correct_with(core, |core, anchor| {
            core.repaginate_if_needed();
            positions(&core.dump_page_items_json(None), anchor)
        })
    }

    fn correct_with(
        &self,
        core: &mut DocumentCore,
        mut read: impl FnMut(&mut DocumentCore, &Anchor) -> Result<(Position, Position), String>,
    ) -> Result<usize, String> {
        let mut corrected = 0;
        for anchor in &self.anchors {
            let (header, body) = read(core, anchor)?;
            if header == body {
                continue;
            }
            let mut document = core.document().clone();
            let paragraph = document.sections[0]
                .paragraphs
                .get_mut(anchor.paragraph)
                .ok_or("Missing generated table pagination paragraph")?;
            if paragraph.raw_break_type & 4 != 0 || paragraph.column_type == ColumnBreakType::Page {
                return Err(cannot_fit(anchor));
            }
            paragraph.column_type = ColumnBreakType::Page;
            paragraph.raw_break_type = (paragraph.raw_break_type & !8) | 4;
            paragraph.page_break_synthesized = false;
            core.set_document(document);
            let (header, body) = read(core, anchor)?;
            if header != body {
                return Err(cannot_fit(anchor));
            }
            corrected += 1;
        }
        Ok(corrected)
    }

    pub(crate) fn verify(&self, core: &mut DocumentCore) -> Result<(), String> {
        if self.anchors.is_empty() {
            return Ok(());
        }
        core.repaginate_if_needed();
        let pages = core.dump_page_items_json(None);
        for anchor in &self.anchors {
            let (header, body) = positions(&pages, anchor)?;
            if header != body {
                return Err(cannot_fit(anchor));
            }
        }
        Ok(())
    }
}

fn cannot_fit(anchor: &Anchor) -> String {
    format!(
        "The table header and first body row cannot start on the same page/column at root paragraph {}. Reduce the header/caption size or the first row's formatting in the template.",
        anchor.paragraph
    )
}

fn invalid() -> String {
    "Missing, ambiguous or unsupported generated table pagination positions".into()
}
fn number(value: &Value, name: &str) -> Result<usize, String> {
    value
        .get(name)
        .and_then(Value::as_u64)
        .and_then(|n| usize::try_from(n).ok())
        .ok_or_else(invalid)
}
fn array<'a>(value: &'a Value, name: &str) -> Result<&'a Vec<Value>, String> {
    value
        .get(name)
        .and_then(Value::as_array)
        .ok_or_else(invalid)
}
fn cut(value: &Value, name: &str) -> Result<(usize, bool), String> {
    let values = array(value, name)?;
    if values.iter().any(|n| n.as_u64().is_none()) {
        return Err(invalid());
    }
    Ok((
        values.len(),
        values.iter().any(|n| n.as_u64().unwrap_or(0) > 0),
    ))
}

fn positions(pages: &Value, anchor: &Anchor) -> Result<(Position, Position), String> {
    let mut header = None;
    let mut body = None;
    let mut original = false;
    let mut previous_end = None;
    for (page_index, page) in pages.as_array().ok_or_else(invalid)?.iter().enumerate() {
        if number(page, "pageIndex")? != page_index || number(page, "section")? != 0 {
            return Err(invalid());
        }
        for (column_index, column) in array(page, "columns")?.iter().enumerate() {
            if number(column, "index")? != column_index {
                return Err(invalid());
            }
            let position = (page_index, column_index);
            let mut matched = false;
            for item in array(column, "items")? {
                let kind = item
                    .get("kind")
                    .and_then(Value::as_str)
                    .ok_or_else(invalid)?;
                if kind != "table" && kind != "partialTable" {
                    continue;
                }
                if number(item, "paraIndex")? != anchor.paragraph
                    || number(item, "controlIndex")? != anchor.control
                {
                    continue;
                }
                if matched {
                    return Err(invalid());
                }
                matched = true;
                let table = item.get("table").ok_or_else(invalid)?;
                if number(table, "rowCount")? != anchor.rows
                    || number(table, "colCount")? != anchor.columns
                {
                    return Err(invalid());
                }
                if kind == "table" {
                    if header.is_some() || body.is_some() {
                        return Err(invalid());
                    }
                    header = Some(position);
                    body = Some(position);
                    original = true;
                    previous_end = Some((anchor.rows, false));
                    continue;
                }
                let start = number(item, "startRow")?;
                let end = number(item, "endRow")?;
                let continuation = item
                    .get("isContinuation")
                    .and_then(Value::as_bool)
                    .ok_or_else(invalid)?;
                let start_cut = cut(item, "startCut")?;
                let end_cut = cut(item, "endCut")?;
                if start >= end || end > anchor.rows || continuation != original {
                    return Err(invalid());
                }
                if !original {
                    if start != 0 {
                        return Err(invalid());
                    }
                    original = true;
                }
                if let Some((previous, split)) = previous_end
                    && start != if split { previous - 1 } else { previous }
                {
                    return Err(invalid());
                }
                previous_end = Some((end, end_cut.0 != 0));
                // Generated initial rows have one unmerged cell per column.
                // Cuts count consumed content units per cell. At least one
                // positive end cut proves the row has actually started here;
                // a zero cut merely reserves a band before any cell content.
                if (start < 2 && start_cut.0 != 0 && start_cut.0 != anchor.columns)
                    || (end <= 2 && end_cut.0 != 0 && end_cut.0 != anchor.columns)
                    || (start == 0 && header.is_none() && start_cut.1)
                    || (start == 1 && body.is_none() && start_cut.1)
                {
                    return Err("Generated table pagination cannot determine initial cell starts from row cuts".into());
                }
                if start == 0 && header.is_none() && (end > 1 || end_cut.0 == 0 || end_cut.1) {
                    header = Some(position);
                }
                if start <= 1
                    && end > 1
                    && body.is_none()
                    && (end > 2 || end_cut.0 == 0 || end_cut.1)
                {
                    body = Some(position);
                }
            }
        }
    }
    match (header, body) {
        (Some(header), Some(body)) if header <= body => Ok((header, body)),
        _ => Err(invalid()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rhwp::model::{document::Document, paragraph::Paragraph};
    use serde_json::json;

    fn anchor() -> Anchor {
        Anchor {
            paragraph: 1,
            control: 0,
            rows: 3,
            columns: 2,
        }
    }
    fn fragment(start: usize, end: usize, continuation: bool) -> Value {
        json!({"kind":"partialTable", "paraIndex":1,"controlIndex":0,"table":{"rowCount":3,"colCount":2},"startRow":start,"endRow":end,"isContinuation":continuation,"startCut":[],"endCut":[]})
    }
    fn pages(items: Vec<Vec<Value>>) -> Value {
        Value::Array(items.into_iter().enumerate().map(|(index, items)| json!({"pageIndex":index,"section":0,"columns":[{"index":0,"items":items}]})).collect())
    }
    #[test]
    fn detects_original_rows_ignoring_repeated_header() {
        assert_eq!(
            positions(
                &pages(vec![
                    vec![fragment(0, 1, false)],
                    vec![fragment(1, 3, true)]
                ]),
                &anchor()
            )
            .unwrap(),
            ((0, 0), (1, 0))
        );
        assert_eq!(
            positions(&pages(vec![vec![fragment(0, 3, false)]]), &anchor()).unwrap(),
            ((0, 0), (0, 0))
        );
    }
    #[test]
    fn rejects_missing_duplicate_malformed_and_ambiguous_cuts() {
        for bad in [
            pages(vec![]),
            pages(vec![vec![fragment(1, 3, true)]]),
            pages(vec![vec![fragment(0, 3, false), fragment(0, 3, false)]]),
        ] {
            assert!(positions(&bad, &anchor()).is_err());
        }
        for key in ["startCut", "endCut"] {
            let mut item = fragment(0, 2, false);
            item[key] = json!([1]);
            assert!(positions(&pages(vec![vec![item]]), &anchor()).is_err());
        }
        let mut item = fragment(0, 3, false);
        item["endRow"] = json!(4);
        assert!(positions(&pages(vec![vec![item]]), &anchor()).is_err());
    }
    fn core() -> DocumentCore {
        let mut doc = Document::default();
        doc.sections.push(Default::default());
        doc.sections[0].paragraphs = vec![
            Paragraph::default(),
            Paragraph::default(),
            Paragraph::default(),
        ];
        let mut core = DocumentCore::new_empty();
        core.set_document(doc);
        core
    }
    #[test]
    fn split_body_content_starts_on_header_page_but_empty_band_does_not() {
        let mut item = fragment(0, 2, false);
        item["endCut"] = json!([1, 0]);
        let mut next = fragment(1, 3, true);
        next["startCut"] = json!([1, 0]);
        assert_eq!(
            positions(
                &pages(vec![vec![item.clone()], vec![next.clone()]]),
                &anchor()
            )
            .unwrap(),
            ((0, 0), (0, 0))
        );
        item["endCut"] = json!([0, 0]);
        next["startCut"] = json!([0, 0]);
        assert_eq!(
            positions(&pages(vec![vec![item], vec![next]]), &anchor()).unwrap(),
            ((0, 0), (1, 0))
        );
        let mut empty_header = fragment(0, 1, false);
        empty_header["endCut"] = json!([0, 0]);
        let mut next = fragment(0, 3, true);
        next["startCut"] = json!([0, 0]);
        assert_eq!(
            positions(&pages(vec![vec![empty_header], vec![next]]), &anchor()).unwrap(),
            ((1, 0), (1, 0))
        );
    }
    #[test]
    fn detects_column_boundary_and_rejects_fragment_gaps() {
        let columns = json!([{"pageIndex":0,"section":0,"columns":[
            {"index":0,"items":[fragment(0,1,false)]},
            {"index":1,"items":[fragment(1,3,true)]}
        ]}]);
        assert_eq!(positions(&columns, &anchor()).unwrap(), ((0, 0), (0, 1)));
        assert!(
            positions(
                &pages(vec![
                    vec![fragment(0, 1, false)],
                    vec![fragment(2, 3, true)]
                ]),
                &anchor()
            )
            .is_err()
        );
    }
    #[test]
    fn corrects_only_anchor_and_rereads_later_tables() {
        let mut core = core();
        core.document_mut().sections[0].paragraphs[1].raw_break_type = 8;
        let mut second = anchor();
        second.paragraph = 2;
        let plan = Plan {
            anchors: vec![anchor(), second],
        };
        let before = core.document().sections[0].paragraphs[0].clone();
        let mut calls = 0;
        assert_eq!(
            plan.correct_with(&mut core, |_, _| {
                calls += 1;
                Ok(if calls == 1 {
                    ((0, 0), (0, 1))
                } else {
                    ((1, 0), (1, 0))
                })
            })
            .unwrap(),
            1
        );
        assert_eq!(calls, 3);
        let ps = &core.document().sections[0].paragraphs;
        assert_eq!(ps[1].raw_break_type, 4);
        assert_eq!(ps[1].column_type, ColumnBreakType::Page);
        assert_eq!(format!("{:?}", ps[0]), format!("{before:?}"));
        assert_eq!(ps[2].raw_break_type, 0);
    }
    #[test]
    fn unchanged_and_cannot_fit_are_bounded() {
        let plan = Plan {
            anchors: vec![anchor()],
        };
        let mut core = core();
        assert_eq!(
            plan.correct_with(&mut core, |_, _| Ok(((0, 0), (0, 0))))
                .unwrap(),
            0
        );
        let mut calls = 0;
        assert!(
            plan.correct_with(&mut core, |_, _| {
                calls += 1;
                Ok(((0, 0), (1, 0)))
            })
            .is_err()
        );
        assert_eq!(calls, 2);
        calls = 0;
        assert!(
            plan.correct_with(&mut core, |_, _| {
                calls += 1;
                Ok(((0, 0), (1, 0)))
            })
            .is_err()
        );
        assert_eq!(calls, 1);
    }

    #[test]
    fn resolves_generated_roots_only_and_skips_inline_and_header_only_tables() {
        use rhwp::model::table::Table;
        fn table(id: u32, inline: bool, rows: u16) -> Control {
            let mut table = Table {
                row_count: rows,
                col_count: 2,
                ..Default::default()
            };
            table.common.instance_id = id;
            table.common.treat_as_char = inline;
            Control::Table(Box::new(table))
        }
        let mut core = core();
        let doc = core.document_mut();
        doc.sections[0].paragraphs[0]
            .controls
            .push(table(10, false, 3));
        doc.sections[0].paragraphs[1]
            .controls
            .push(table(11, false, 3));
        doc.sections[0].paragraphs[2]
            .controls
            .push(table(12, true, 3));
        let generated = BTreeSet::from([11, 12]);
        assert_eq!(
            Plan::resolve(doc, &generated).unwrap().anchors,
            vec![anchor()]
        );
        assert!(Plan::resolve(doc, &BTreeSet::from([99])).is_err());
        doc.sections[0].paragraphs[2].controls = vec![table(11, false, 3)];
        assert!(Plan::resolve(doc, &BTreeSet::from([11])).is_err());
        doc.sections[0].paragraphs[2].controls = vec![table(12, false, 1)];
        assert_eq!(
            Plan::resolve(doc, &generated).unwrap().anchors,
            vec![anchor()]
        );
    }
}
