// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Euisoon Ahn
//! Keep a generated floating table's first header with its first body row.
//! Positions belong to rhwp's paginator; they do not prove Hancom layout fidelity.

use std::collections::{BTreeMap, BTreeSet};

use rhwp::{
    DocumentCore,
    model::{
        control::Control,
        document::Document,
        paragraph::{ColumnBreakType, Paragraph},
    },
};

use crate::page_window;
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Anchor {
    paragraph: usize,
    control: usize,
    rows: usize,
    columns: usize,
}

#[derive(Debug, Default, PartialEq)]
pub(crate) struct Plan {
    anchors: Vec<Anchor>,
    /// The page breaks the last correction added to the document, with the
    /// flags each anchor paragraph had before.
    added: Vec<Added>,
    method: Method,
}

/// How the last correction read the copy.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Method {
    /// Whole-copy readings only.
    #[default]
    Full,
    /// Windows after the first break, certified by one whole-copy reading.
    Windows,
    /// Windows until one could not be used or the certification failed, then
    /// whole-copy readings from the start.
    WindowsThenFull,
}

/// A page break added to root paragraph `paragraph`, and the flags it replaced.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Added {
    paragraph: usize,
    column_type: ColumnBreakType,
    raw_break_type: u8,
    page_break_synthesized: bool,
}

type Position = (usize, usize);

/// Add an anchor page break to `paragraph`, clearing its column break, and return
/// what it replaced. None when the paragraph already starts a page.
fn add_page_break(paragraph: &mut Paragraph, index: usize) -> Option<Added> {
    if paragraph.raw_break_type & 4 != 0 || paragraph.column_type == ColumnBreakType::Page {
        return None;
    }
    let added = Added {
        paragraph: index,
        column_type: paragraph.column_type,
        raw_break_type: paragraph.raw_break_type,
        page_break_synthesized: paragraph.page_break_synthesized,
    };
    paragraph.column_type = ColumnBreakType::Page;
    paragraph.raw_break_type = (paragraph.raw_break_type & !8) | 4;
    paragraph.page_break_synthesized = false;
    Some(added)
}

/// Page items of a document written and opened again, and the opened core.
/// Root paragraph `p` of the copy is paragraph `p - offset` here.
struct Reading {
    fragments: Fragments,
    core: DocumentCore,
    offset: usize,
}

impl Reading {
    fn of(document: &Document, offset: usize) -> Result<Self, String> {
        let saved = rhwp::serializer::serialize_document(document)
            .map_err(|e| format!("HWP serialization failed: {e:?}"))?;
        let mut core = DocumentCore::from_bytes(&saved)
            .map_err(|e| format!("Serialized HWP could not be reopened: {e:?}"))?;
        core.repaginate_if_needed();
        Ok(Self {
            fragments: Fragments::read(core.dump_page_items_json(None))?,
            core,
            offset,
        })
    }

    fn positions(&self, anchor: &Anchor) -> Result<(Position, Position), String> {
        let local = Anchor {
            paragraph: anchor
                .paragraph
                .checked_sub(self.offset)
                .ok_or_else(invalid)?,
            ..anchor.clone()
        };
        self.fragments.positions(&local)
    }

    /// Whether page `page` holds anything from before root paragraph `paragraph`:
    /// floats carried over or footnotes moved from an earlier page.
    fn carries_into(&self, page: usize, paragraph: usize) -> bool {
        let Some(local) = paragraph.checked_sub(self.offset) else {
            return true;
        };
        if self
            .fragments
            .pages
            .get(page)
            .and_then(|page| page.first_paragraph)
            .is_some_and(|first| first < local)
        {
            return true;
        }
        let Ok(page) = u32::try_from(page) else {
            return true;
        };
        (0..)
            .map_while(|index| self.core.get_page_footnote_info_native(page, index).ok())
            .take(100_000)
            .any(|info| {
                serde_json::from_str::<Value>(&info)
                    .ok()
                    .and_then(|info| paragraph_index(&info, "paraIdx"))
                    .is_none_or(|source| source < local)
            })
    }
}

/// The composed document exported once and parsed again: the verification copy.
/// It is read only to find and check page breaks and never published. Its section
/// is written again from the model, so the breaks added to it are read with it.
struct Copy {
    document: Document,
}

impl Copy {
    fn new(saved: &[u8]) -> Result<Self, String> {
        let mut document = rhwp::parser::parse_hwp(saved)
            .map_err(|e| format!("Serialized HWP could not be parsed: {e:?}"))?;
        if let Some(section) = document.sections.first_mut() {
            section.raw_stream = None;
        }
        Ok(Self { document })
    }
}

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
        Ok(Self {
            anchors,
            ..Self::default()
        })
    }

    /// Positions come from rhwp's own layout of the verification copy: the
    /// composed document exported once and parsed again, since rhwp 0.9 derives
    /// line layout only when it opens a file and the model in memory can paginate
    /// differently. The copy is read whole once. A page break starts a fresh page,
    /// so later tables are then read from windows of the copy that start at the
    /// last break and end after the next 1, 2, 4, … candidates until one splits.
    /// One whole reading of the copy with every break certifies the result; a
    /// window that cannot be used, or a failed certification, starts over on whole
    /// readings. The breaks found go into the document at once.
    pub(crate) fn correct(&mut self, core: &mut DocumentCore) -> Result<usize, String> {
        self.added.clear();
        self.method = Method::Full;
        if self.anchors.is_empty() {
            return Ok(0);
        }
        let saved = core
            .export_hwp_with_adapter_snapshot_with_report()
            .map_err(|e| format!("HWP serialization failed: {e:?}"))?;
        let broken = self.correct_copy(Copy::new(saved.bytes())?)?;
        if broken.is_empty() {
            return Ok(0);
        }
        let mut document = core.document().clone();
        for index in broken {
            let anchor = &self.anchors[index];
            let paragraph = document.sections[0]
                .paragraphs
                .get_mut(anchor.paragraph)
                .ok_or("Missing generated table pagination paragraph")?;
            let added =
                add_page_break(paragraph, anchor.paragraph).ok_or_else(|| cannot_fit(anchor))?;
            self.added.push(added);
        }
        core.set_document(document);
        Ok(self.added.len())
    }

    /// The anchors, by index, that need a page break, found and added on `copy`.
    fn correct_copy(&mut self, mut copy: Copy) -> Result<Vec<usize>, String> {
        let first = Reading::of(&copy.document, 0)?;
        let mut current = None;
        for (index, anchor) in self.anchors.iter().enumerate() {
            let (header, body) = first.positions(anchor)?;
            if header != body {
                current = Some(index);
                break;
            }
        }
        let Some(current) = current else {
            return Ok(Vec::new());
        };
        let pristine = copy.document.clone();
        let Some(mut windows) =
            page_window::Windows::prepare(&mut copy.document, first.core.document())
        else {
            return self.on_full_readings(&mut copy.document, Some(first));
        };
        if let Some(broken) = self.windows(&mut copy.document, &mut windows, first, current)?
            && self.fits(&copy.document)
        {
            self.method = Method::Windows;
            return Ok(broken);
        }
        self.method = Method::WindowsThenFull;
        copy.document = pristine;
        self.on_full_readings(&mut copy.document, None)
    }

    /// Breaks found from windows after the split of anchor `current` in
    /// `found_in`, added to `copy`; None when a window cannot be used.
    fn windows(
        &self,
        copy: &mut Document,
        windows: &mut page_window::Windows,
        mut found_in: Reading,
        mut current: usize,
    ) -> Result<Option<Vec<usize>>, String> {
        let mut broken = Vec::new();
        loop {
            let anchor = &self.anchors[current];
            let (header, _) = found_in.positions(anchor)?;
            let Some(page_number) = found_in
                .fragments
                .pages
                .get(header.0)
                .and_then(|page| page.number)
                .and_then(|number| number.checked_add(1))
            else {
                return Ok(None);
            };
            if found_in.carries_into(header.0 + 1, anchor.paragraph) {
                return Ok(None);
            }
            let paragraph = copy.sections[0]
                .paragraphs
                .get_mut(anchor.paragraph)
                .ok_or("Missing generated table pagination paragraph")?;
            if add_page_break(paragraph, anchor.paragraph).is_none() {
                // A split table whose anchor already starts a page cannot be fixed;
                // a window's verdict on it is left to whole readings.
                if found_in.offset == 0 {
                    return Err(cannot_fit(anchor));
                }
                return Ok(None);
            }
            broken.push(current);
            let start = anchor.paragraph;
            let mut settled = current + 1;
            let mut step = 1usize;
            let next = loop {
                let last = settled.saturating_add(step).min(self.anchors.len());
                let end = self.anchors[last - 1].paragraph + 1;
                let Some(document) = windows.document(copy, start, end, page_number) else {
                    return Ok(None);
                };
                let Ok(window) = Reading::of(&document, start - 1) else {
                    return Ok(None);
                };
                let fits = window.positions(anchor).ok().and_then(|(header, body)| {
                    let number = window.fragments.pages.get(header.0)?.number?;
                    Some(header == body && number == page_number)
                });
                if fits != Some(true) || !windows.numbers_match(window.core.document(), start) {
                    return Ok(None);
                }
                let mut split = None;
                for index in settled..last {
                    let Ok((header, body)) = window.positions(&self.anchors[index]) else {
                        return Ok(None);
                    };
                    if header != body {
                        split = Some(index);
                        break;
                    }
                }
                if let Some(index) = split {
                    break Some((index, window));
                }
                settled = last;
                if settled == self.anchors.len() {
                    break None;
                }
                step = step.saturating_mul(2);
            };
            match next {
                Some((index, window)) => {
                    current = index;
                    found_in = window;
                }
                None => return Ok(Some(broken)),
            }
        }
    }

    /// Whether every anchor's header and first body row start together in one
    /// whole reading of `copy`.
    fn fits(&self, copy: &Document) -> bool {
        Reading::of(copy, 0).is_ok_and(|reading| {
            self.anchors.iter().all(|anchor| {
                reading
                    .positions(anchor)
                    .is_ok_and(|(header, body)| header == body)
            })
        })
    }

    /// The correction on whole readings of `copy`. `reading` is a reading of
    /// `copy` as it is, if one exists.
    fn on_full_readings(
        &self,
        copy: &mut Document,
        mut reading: Option<Reading>,
    ) -> Result<Vec<usize>, String> {
        self.correct_with(copy, |copy, anchor, changed| {
            if changed || reading.is_none() {
                reading = Some(Reading::of(copy, 0)?);
            }
            reading.as_ref().ok_or_else(invalid)?.positions(anchor)
        })
    }

    /// `read` receives true when the copy changed since its previous call.
    fn correct_with(
        &self,
        copy: &mut Document,
        mut read: impl FnMut(&Document, &Anchor, bool) -> Result<(Position, Position), String>,
    ) -> Result<Vec<usize>, String> {
        let mut broken = Vec::new();
        for (index, anchor) in self.anchors.iter().enumerate() {
            let (header, body) = read(copy, anchor, false)?;
            if header == body {
                continue;
            }
            let paragraph = copy.sections[0]
                .paragraphs
                .get_mut(anchor.paragraph)
                .ok_or("Missing generated table pagination paragraph")?;
            add_page_break(paragraph, anchor.paragraph).ok_or_else(|| cannot_fit(anchor))?;
            broken.push(index);
            let (header, body) = read(copy, anchor, true)?;
            if header != body {
                return Err(cannot_fit(anchor));
            }
        }
        Ok(broken)
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
fn array_mut<'a>(value: &'a mut Value, name: &str) -> Result<&'a mut Vec<Value>, String> {
    value
        .get_mut(name)
        .and_then(Value::as_array_mut)
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

/// Table items of one page-items reading in page and column order, grouped by
/// root paragraph and control. The page and column structure is checked once per
/// reading, so each anchor reads only its own items: O(items · log tables) per
/// reading instead of O(items) per anchor.
struct Fragments {
    tables: BTreeMap<(usize, usize), Vec<(Position, Value)>>,
    pages: Vec<PageSummary>,
}

/// What the window checks need from a page: its displayed number and the
/// smallest root paragraph index of its items and extras.
struct PageSummary {
    number: Option<usize>,
    first_paragraph: Option<usize>,
}

fn paragraph_index(value: &Value, name: &str) -> Option<usize> {
    value
        .get(name)
        .and_then(Value::as_u64)
        .and_then(|n| usize::try_from(n).ok())
}

impl Fragments {
    fn read(mut pages: Value) -> Result<Self, String> {
        let mut tables = BTreeMap::<_, Vec<_>>::new();
        let mut summaries = Vec::new();
        for (page_index, page) in pages
            .as_array_mut()
            .ok_or_else(invalid)?
            .iter_mut()
            .enumerate()
        {
            if number(page, "pageIndex")? != page_index || number(page, "section")? != 0 {
                return Err(invalid());
            }
            let mut first_paragraph = page
                .get("extras")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .flat_map(|extra| {
                    [
                        paragraph_index(extra, "paraIndex"),
                        paragraph_index(extra, "tableParaIndex"),
                    ]
                })
                .flatten()
                .min();
            let page_number = paragraph_index(page, "pageNumber");
            for (column_index, column) in array_mut(page, "columns")?.iter_mut().enumerate() {
                if number(column, "index")? != column_index {
                    return Err(invalid());
                }
                for item in array_mut(column, "items")? {
                    if let Some(paragraph) = paragraph_index(item, "paraIndex") {
                        first_paragraph =
                            Some(first_paragraph.map_or(paragraph, |first| first.min(paragraph)));
                    }
                    let kind = item
                        .get("kind")
                        .and_then(Value::as_str)
                        .ok_or_else(invalid)?;
                    if kind != "table" && kind != "partialTable" {
                        continue;
                    }
                    let key = (number(item, "paraIndex")?, number(item, "controlIndex")?);
                    tables
                        .entry(key)
                        .or_default()
                        .push(((page_index, column_index), item.take()));
                }
            }
            summaries.push(PageSummary {
                number: page_number,
                first_paragraph,
            });
        }
        Ok(Self {
            tables,
            pages: summaries,
        })
    }

    fn positions(&self, anchor: &Anchor) -> Result<(Position, Position), String> {
        let mut header = None;
        let mut body = None;
        let mut original = false;
        let mut previous_end = None;
        let mut previous_position = None;
        let items = self
            .tables
            .get(&(anchor.paragraph, anchor.control))
            .map_or(&[][..], Vec::as_slice);
        for (position, item) in items {
            let position = *position;
            // One table item per column; items arrive in reading order.
            if previous_position == Some(position) {
                return Err(invalid());
            }
            previous_position = Some(position);
            let kind = item
                .get("kind")
                .and_then(Value::as_str)
                .ok_or_else(invalid)?;
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
                return Err(
                    "Generated table pagination cannot determine initial cell starts from row cuts"
                        .into(),
                );
            }
            if start == 0 && header.is_none() && (end > 1 || end_cut.0 == 0 || end_cut.1) {
                header = Some(position);
            }
            if start <= 1 && end > 1 && body.is_none() && (end > 2 || end_cut.0 == 0 || end_cut.1) {
                body = Some(position);
            }
        }
        match (header, body) {
            (Some(header), Some(body)) if header <= body => Ok((header, body)),
            _ => Err(invalid()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rhwp::model::{document::Document, paragraph::Paragraph};
    use serde_json::json;

    fn positions(pages: &Value, anchor: &Anchor) -> Result<(Position, Position), String> {
        Fragments::read(pages.clone())?.positions(anchor)
    }

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
    fn one_reading_serves_every_anchor() {
        let mut other = fragment(0, 3, false);
        other["paraIndex"] = json!(2);
        let fragments = Fragments::read(pages(vec![
            vec![json!({"kind":"text"}), fragment(0, 1, false), other.clone()],
            vec![fragment(1, 3, true)],
        ]))
        .unwrap();
        let mut second = anchor();
        second.paragraph = 2;
        assert_eq!(fragments.positions(&anchor()).unwrap(), ((0, 0), (1, 0)));
        assert_eq!(fragments.positions(&second).unwrap(), ((0, 0), (0, 0)));
        second.paragraph = 3;
        assert!(fragments.positions(&second).is_err());
        // Every table item names its root paragraph and control, as before.
        let mut unnamed = other;
        unnamed.as_object_mut().unwrap().remove("paraIndex");
        assert!(Fragments::read(pages(vec![vec![fragment(0, 3, false), unnamed]])).is_err());
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
    fn document() -> Document {
        let mut doc = Document::default();
        doc.sections.push(Default::default());
        doc.sections[0].paragraphs = vec![
            Paragraph::default(),
            Paragraph::default(),
            Paragraph::default(),
        ];
        doc
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
        let mut copy = document();
        copy.sections[0].paragraphs[1].raw_break_type = 8;
        let mut second = anchor();
        second.paragraph = 2;
        let plan = Plan {
            anchors: vec![anchor(), second],
            ..Plan::default()
        };
        let before = copy.sections[0].paragraphs[0].clone();
        let mut calls = 0;
        assert_eq!(
            plan.correct_with(&mut copy, |_, _, _| {
                calls += 1;
                Ok(if calls == 1 {
                    ((0, 0), (0, 1))
                } else {
                    ((1, 0), (1, 0))
                })
            })
            .unwrap(),
            vec![0]
        );
        assert_eq!(calls, 3);
        let ps = &copy.sections[0].paragraphs;
        assert_eq!(ps[1].raw_break_type, 4);
        assert_eq!(ps[1].column_type, ColumnBreakType::Page);
        assert_eq!(format!("{:?}", ps[0]), format!("{before:?}"));
        assert_eq!(ps[2].raw_break_type, 0);
    }
    #[test]
    fn unchanged_and_cannot_fit_are_bounded() {
        let plan = Plan {
            anchors: vec![anchor()],
            ..Plan::default()
        };
        let mut copy = document();
        assert!(
            plan.correct_with(&mut copy, |_, _, _| Ok(((0, 0), (0, 0))))
                .unwrap()
                .is_empty()
        );
        let mut calls = 0;
        assert!(
            plan.correct_with(&mut copy, |_, _, _| {
                calls += 1;
                Ok(((0, 0), (1, 0)))
            })
            .is_err()
        );
        assert_eq!(calls, 2);
        calls = 0;
        assert!(
            plan.correct_with(&mut copy, |_, _, _| {
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
        let mut doc = document();
        let doc = &mut doc;
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

    /// The all-features manuscript rendered into a default (`init-template`)
    /// template, where one generated table needs a page break.
    fn all_features() -> (DocumentCore, Plan) {
        let root =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/all-features");
        let bytes = std::fs::read(root.join("all-features.ir.json")).unwrap();
        let ir = md2hwp_core::read_ir(&bytes, &md2hwp_core::ValidationLimits::default()).unwrap();
        let mut resources = BTreeMap::new();
        for (index, block) in ir.as_document().blocks.iter().enumerate() {
            if let md2hwp_core::ir::Block::Figure { image, .. } = block {
                resources.insert(
                    index,
                    crate::image_resource::Resource::read(&root, &image.path).unwrap(),
                );
            }
        }
        let mut core = DocumentCore::new_empty();
        core.set_document(crate::template_init::create().unwrap());
        let plan = {
            // Table widths measured with Windows fonts in two test threads at once
            // came out too wide; render one manuscript at a time.
            let _rendering = RENDERING.lock().unwrap_or_else(|e| e.into_inner());
            crate::template::render(&ir, &mut core, &resources)
                .unwrap()
                .1
        };
        (core, plan)
    }

    static RENDERING: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn export(core: &DocumentCore) -> Vec<u8> {
        core.export_hwp_with_adapter_snapshot_with_report()
            .unwrap()
            .bytes()
            .to_vec()
    }

    /// The copy of `core`'s document with the breaks of `plan` undone.
    fn copy_before(core: &DocumentCore, plan: &Plan) -> Document {
        let mut document = core.document().clone();
        for added in &plan.added {
            let paragraph = &mut document.sections[0].paragraphs[added.paragraph];
            paragraph.column_type = added.column_type;
            paragraph.raw_break_type = added.raw_break_type;
            paragraph.page_break_synthesized = added.page_break_synthesized;
        }
        let mut writer = DocumentCore::new_empty();
        writer.set_document(document);
        Copy::new(&export(&writer)).unwrap().document
    }

    #[test]
    fn windows_find_the_breaks_of_whole_copy_readings() {
        let (core, plan) = all_features();
        assert_eq!(plan.method, Method::Windows);
        assert_eq!(plan.added.len(), 1);
        // The same document corrected on whole readings of its copy only.
        let mut copy = copy_before(&core, &plan);
        let broken: Vec<_> = plan
            .on_full_readings(&mut copy, None)
            .unwrap()
            .into_iter()
            .map(|index| plan.anchors[index].paragraph)
            .collect();
        let windowed: Vec<_> = plan.added.iter().map(|added| added.paragraph).collect();
        assert_eq!(broken, windowed);
    }

    #[test]
    fn windows_number_like_the_whole_copy_at_every_table() {
        let (core, plan) = all_features();
        let mut copy = Copy::new(&export(&core)).unwrap().document;
        let first = Reading::of(&copy, 0).unwrap();
        let mut windows = page_window::Windows::prepare(&mut copy, first.core.document()).unwrap();
        let end = copy.sections[0].paragraphs.len();
        assert!(!plan.anchors.is_empty());
        for anchor in &plan.anchors {
            let document = windows.document(&copy, anchor.paragraph, end, 2).unwrap();
            let window = Reading::of(&document, anchor.paragraph - 1).unwrap();
            assert!(windows.numbers_match(window.core.document(), anchor.paragraph));
        }
    }

    fn blank() -> Document {
        let mut core = DocumentCore::new_empty();
        core.create_blank_document_native().unwrap();
        core.document().clone()
    }

    fn plain(controls: Vec<Control>) -> Paragraph {
        Paragraph {
            controls,
            ..Default::default()
        }
    }

    #[test]
    fn windows_need_a_document_they_can_reproduce() {
        use rhwp::model::{
            control::{AutoNumber, AutoNumberType},
            page::ColumnDef,
        };
        let ready = |document: &Document| {
            page_window::Windows::prepare(&mut document.clone(), document).is_some()
        };
        let base = blank();
        assert!(ready(&base));
        let mut columns = base.clone();
        columns.sections[0].paragraphs[0]
            .controls
            .retain(|control| !matches!(control, Control::ColumnDef(_)));
        columns.sections[0].paragraphs[0]
            .controls
            .push(Control::ColumnDef(ColumnDef {
                column_count: 2,
                ..Default::default()
            }));
        assert!(!ready(&columns));
        let mut page_field = base.clone();
        page_field.sections[0]
            .paragraphs
            .push(plain(vec![Control::AutoNumber(AutoNumber {
                number_type: AutoNumberType::TotalPage,
                ..Default::default()
            })]));
        assert!(!ready(&page_field));
        let mut cover = base.clone();
        cover.sections[0].paragraphs.insert(0, plain(Vec::new()));
        assert!(!ready(&cover));
    }

    #[test]
    fn window_starts_with_the_section_paragraph_and_the_counters_there() {
        use rhwp::model::control::{AutoNumberType, NewNumber};
        let restart = |number_type, number| {
            Control::NewNumber(NewNumber {
                number_type,
                number,
            })
        };
        let mut document = blank();
        document.sections[0].paragraphs.extend([
            plain(vec![restart(AutoNumberType::Table, 5)]),
            plain(vec![restart(AutoNumberType::Footnote, 7)]),
            plain(Vec::new()),
            plain(Vec::new()),
        ]);
        let mut windows = page_window::Windows::prepare(&mut document.clone(), &document).unwrap();
        let window = windows.document(&document, 3, 5, 10).unwrap();
        let paragraphs = &window.sections[0].paragraphs;
        assert_eq!(paragraphs.len(), 3);
        let definition = paragraphs[0]
            .controls
            .iter()
            .find_map(|control| match control {
                Control::SectionDef(definition) => Some(definition),
                _ => None,
            })
            .unwrap();
        assert_eq!((definition.page_num, definition.table_num), (9, 5));
        assert_eq!(window.sections[0].section_def.table_num, 5);
        assert!(paragraphs[0].controls.iter().any(|control| matches!(
            control,
            Control::NewNumber(NewNumber {
                number_type: AutoNumberType::Footnote,
                number: 7
            })
        )));
        // Counting only moves forward, and the first page cannot be numbered 0.
        assert!(windows.document(&document, 2, 5, 10).is_none());
        assert!(windows.document(&document, 3, 5, 1).is_none());
    }
}
