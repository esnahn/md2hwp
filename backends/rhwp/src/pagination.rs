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
        shape::Caption,
        style::LineSpacingType,
    },
    renderer::DEFAULT_DPI,
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
/// Captions rhwp leaves out of its layout are replaced once, when it is built.
struct Copy {
    document: Document,
    /// Document root paragraphs, ascending, that got a table caption spacer
    /// before them (see [`replace_table_captions_with_spacers`]).
    spaced: Vec<usize>,
}

/// Something the correction decides in document order on a reading of the copy.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Event {
    /// The caption spacer at this copy paragraph, which needs a break when its
    /// table's first rows leave it behind.
    Spacer(usize),
    /// Anchor `index` of the plan, which needs a page break when its header row
    /// starts on an earlier page or column than its first body row.
    Anchor(usize),
}

/// The break an event needs.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Needs {
    Page,
    Column,
}

impl Copy {
    fn new(saved: &[u8]) -> Result<Self, String> {
        let mut document = rhwp::parser::parse_hwp(saved)
            .map_err(|e| format!("Serialized HWP could not be parsed: {e:?}"))?;
        replace_captions_with_picture_height(&mut document)?;
        if let Some(section) = document.sections.first_mut() {
            section.raw_stream = None;
        }
        let mut copy = Self {
            document,
            spaced: Vec::new(),
        };
        replace_table_captions_with_spacers(&mut copy)?;
        Ok(copy)
    }

    /// Where document root paragraph `paragraph` is in the copy.
    fn index(&self, paragraph: usize) -> usize {
        paragraph + self.spaced.partition_point(|&spaced| spaced <= paragraph)
    }

    /// The copy paragraph that takes document root paragraph `paragraph`'s page
    /// break: its caption spacer, if it has one.
    fn start(&self, paragraph: usize) -> usize {
        self.index(paragraph) - usize::from(self.spaced.binary_search(&paragraph).is_ok())
    }

    /// `anchor` as it is in the copy.
    fn anchor(&self, anchor: &Anchor) -> Anchor {
        Anchor {
            paragraph: self.index(anchor.paragraph),
            ..anchor.clone()
        }
    }

    /// The spacers and anchors in document order, a table's spacer before it.
    fn events(&self, anchors: &[Anchor]) -> Vec<Event> {
        let mut events: Vec<_> =
            self.spaced
                .iter()
                .enumerate()
                .map(|(inserted, &paragraph)| {
                    (paragraph + inserted, Event::Spacer(paragraph + inserted))
                })
                .chain(
                    anchors.iter().enumerate().map(|(index, anchor)| {
                        (self.index(anchor.paragraph), Event::Anchor(index))
                    }),
                )
                .collect();
        events.sort_by_key(|(position, _)| *position);
        events.into_iter().map(|(_, event)| event).collect()
    }
}

/// Heights in HWPUNIT of caption paragraph blocks as rhwp lays them out at their
/// caption text widths (HWPUNIT). Each block ends at its last caption line,
/// without that line's trailing gap or paragraph spacing, as Hancom stacks a
/// caption against its object: Hancom 2020 stores, for example, a picture line of
/// 26835 + 283 + 4 × 1000 + 3 × 600 = 32918 HU for four one-line 10 pt caption
/// paragraphs at 160% and a 283 HU caption gap.
///
/// The paragraphs are measured as body paragraphs, each caption followed by its
/// last line twice: once as stored, once without the line gap and the spacing
/// after, whose difference is the trailing part to leave out. Only the section's
/// first paragraph, which owns its definitions, stays in the measuring copy, so
/// measuring does not lay the document out again.
fn caption_blocks(document: &Document, captions: &[(&Caption, u32)]) -> Result<Vec<u32>, String> {
    let mut measuring = document.clone();
    measuring.sections[0].paragraphs.truncate(1);
    let mut flat_shapes = BTreeMap::new();
    let mut plans = Vec::with_capacity(captions.len());
    for (caption, _) in captions {
        let roots = &mut measuring.sections[0].paragraphs;
        let first = roots.len();
        for paragraph in &caption.paragraphs {
            let mut paragraph = paragraph.clone();
            crate::template::discard_stale_layout(&mut paragraph);
            roots.push(paragraph);
        }
        let last = caption.paragraphs.last().ok_or_else(invalid_caption)?;
        let line = last_line(last)?;
        let flat_shape = match flat_shapes.get(&last.para_shape_id) {
            Some(id) => *id,
            None => {
                let shapes = &mut measuring.doc_info.para_shapes;
                let mut shape = shapes
                    .get(usize::from(last.para_shape_id))
                    .ok_or_else(invalid_caption)?
                    .clone();
                shape.line_spacing_type = LineSpacingType::Percent;
                shape.line_spacing = 100;
                shape.spacing_after = 0;
                shape.raw_data = None;
                let id = u16::try_from(shapes.len()).map_err(|_| invalid_caption())?;
                shapes.push(shape);
                flat_shapes.insert(last.para_shape_id, id);
                id
            }
        };
        let mut flat = line.clone();
        flat.para_shape_id = flat_shape;
        let roots = &mut measuring.sections[0].paragraphs;
        roots.push(line);
        roots.push(flat);
        plans.push((first, caption.paragraphs.len()));
    }
    let mut core = DocumentCore::new_empty();
    core.set_document(measuring);
    core.repaginate_if_needed();
    captions
        .iter()
        .zip(plans)
        .map(|((_, width), (first, count))| {
            let width = f64::from(*width) * DEFAULT_DPI / 7200.0;
            let height = |index: usize| {
                core.production_paragraph_height(0, index, Some(width))
                    .filter(|height| height.is_finite() && *height >= 0.0)
                    .ok_or_else(invalid_caption)
            };
            let mut block = 0.0;
            for index in first..first + count {
                block += height(index)?;
            }
            let trailing = height(first + count)? - height(first + count + 1)?;
            let block = ((block - trailing) * 7200.0 / DEFAULT_DPI).round();
            if !(0.0..=f64::from(u32::MAX)).contains(&block) || trailing < 0.0 {
                return Err(invalid_caption());
            }
            Ok(block as u32)
        })
        .collect()
}

/// One line with the paragraph's own formatting and its last character's format.
fn last_line(paragraph: &Paragraph) -> Result<Paragraph, String> {
    let last = paragraph.text.chars().last();
    let format = paragraph
        .char_shape_id_at(paragraph.text.chars().count().saturating_sub(1))
        .or_else(|| {
            paragraph
                .char_shapes
                .first()
                .map(|shape| shape.char_shape_id)
        })
        .ok_or_else(invalid_caption)?;
    let mut line = Paragraph::new_empty();
    line.set_single_char_shape(format);
    line.style_id = paragraph.style_id;
    line.para_shape_id = paragraph.para_shape_id;
    if let Some(last) = last {
        line.insert_text_at(0, &last.to_string());
    }
    crate::template::discard_stale_layout(&mut line);
    Ok(line)
}

fn invalid_caption() -> String {
    "Caption could not be measured for table pagination".into()
}

/// A body picture laid out as a character whose caption sits above or below it.
struct CaptionedPicture {
    paragraph: usize,
    control: usize,
    /// Text width of the caption paragraphs, in HWPUNIT.
    width: u32,
}

/// rhwp 0.9 lays a treat-as-char picture's line out from the picture alone and
/// draws its native caption over the following text. Hancom's line holds the
/// picture, the caption gap and the caption paragraphs (see [`caption_blocks`]),
/// so rhwp fitted about one caption more on a page and split tables that Hancom
/// keeps whole.
///
/// This removes each top/bottom caption from the copy and adds its height to the
/// picture, so the picture's line reserves the caption's place: the picture grows
/// by the caption gap and the caption block. Left/right captions, which share the
/// picture's band, stay as they are.
fn replace_captions_with_picture_height(document: &mut Document) -> Result<(), String> {
    let pictures = captioned_pictures(document)?;
    if pictures.is_empty() {
        return Ok(());
    }
    let blocks = {
        let captions = pictures
            .iter()
            .map(|picture| {
                match document.sections[0]
                    .paragraphs
                    .get(picture.paragraph)
                    .and_then(|p| p.controls.get(picture.control))
                {
                    Some(Control::Picture(p)) => p
                        .caption
                        .as_ref()
                        .map(|caption| (caption, picture.width))
                        .ok_or_else(invalid_caption),
                    _ => Err(invalid_caption()),
                }
            })
            .collect::<Result<Vec<_>, String>>()?;
        caption_blocks(document, &captions)?
    };
    for (picture, block) in pictures.iter().zip(blocks) {
        let Some(Control::Picture(p)) = document.sections[0]
            .paragraphs
            .get_mut(picture.paragraph)
            .and_then(|p| p.controls.get_mut(picture.control))
        else {
            return Err(invalid_caption());
        };
        let caption = p.caption.take().ok_or_else(invalid_caption)?;
        let gap = u32::try_from(caption.spacing).map_err(|_| invalid_caption())?;
        let extent = block.checked_add(gap).ok_or_else(invalid_caption)?;
        p.common.height = p
            .common
            .height
            .checked_add(extent)
            .ok_or_else(invalid_caption)?;
        p.shape_attr.current_height = p
            .shape_attr
            .current_height
            .checked_add(extent)
            .ok_or_else(invalid_caption)?;
    }
    Ok(())
}

fn captioned_pictures(document: &Document) -> Result<Vec<CaptionedPicture>, String> {
    use rhwp::model::shape::CaptionDirection;
    if document.sections.len() != 1 {
        return Err("Table pagination requires one section".into());
    }
    let mut pictures = Vec::new();
    for (paragraph, p) in document.sections[0].paragraphs.iter().enumerate() {
        for (control, c) in p.controls.iter().enumerate() {
            if let Control::Picture(picture) = c
                && picture.common.treat_as_char
                && let Some(caption) = &picture.caption
                && matches!(
                    caption.direction,
                    CaptionDirection::Top | CaptionDirection::Bottom
                )
                && !caption.paragraphs.is_empty()
            {
                let margins = if caption.include_margin {
                    i64::from(picture.common.margin.left) + i64::from(picture.common.margin.right)
                } else {
                    0
                };
                let width = u32::try_from(i64::from(picture.common.width) + margins)
                    .map_err(|_| invalid_caption())?;
                pictures.push(CaptionedPicture {
                    paragraph,
                    control,
                    width,
                });
            }
        }
    }
    Ok(pictures)
}

/// A floating table after the section's first paragraph whose caption sits above it.
struct CaptionedTable {
    paragraph: usize,
    control: usize,
    /// Text width of the caption paragraphs, in HWPUNIT.
    width: u32,
}

/// rhwp 0.9.1 leaves a floating table's top caption out in two places. When a full
/// page pushes a captioned table to the top of the next one, rhwp draws the table
/// below its caption but advances its page cursor by the table alone: the next
/// caption on that page covers the line above it, and a table at its end keeps
/// its header row alone without room for caption and header. When a table holds
/// footnotes in its cells, rhwp decides whether its first row stays at a page end
/// without the caption and keeps the header row alone there too. Hancom places
/// the caption at the anchor line and the table below it (Hancom 2020 stores the
/// next paragraph at the anchor + caption gap + caption + table, for example
/// 13083 + 283 + 1000 + 9600 = 23966 HU) and moves the caption with the table to
/// the next page.
///
/// This replaces each such caption by a spacer paragraph of the same height before
/// the anchor: fixed line spacing of the caption gap and caption block (rhwp keeps
/// non-percent line spacing at twice its HWPUNIT value), the caption's first
/// character format and one character, so that hidden empty lines cannot drop it.
/// The anchor's page or column break moves to the spacer, and the correction
/// breaks the spacer when the table leaves it behind ([`Event::Spacer`]).
/// A table in the section's first paragraph, which holds the section definition,
/// always starts the first page, where rhwp reserves its caption: it keeps it.
fn replace_table_captions_with_spacers(copy: &mut Copy) -> Result<(), String> {
    let tables = captioned_tables(&copy.document)?;
    if tables.is_empty() {
        return Ok(());
    }
    let blocks = {
        let captions = tables
            .iter()
            .map(|table| {
                match copy.document.sections[0]
                    .paragraphs
                    .get(table.paragraph)
                    .and_then(|p| p.controls.get(table.control))
                {
                    Some(Control::Table(t)) => t
                        .caption
                        .as_ref()
                        .map(|caption| (caption, table.width))
                        .ok_or_else(invalid_caption),
                    _ => Err(invalid_caption()),
                }
            })
            .collect::<Result<Vec<_>, String>>()?;
        caption_blocks(&copy.document, &captions)?
    };
    let document = &mut copy.document;
    let mut spacer_shapes = BTreeMap::new();
    for (table, block) in tables.iter().zip(blocks).rev() {
        let anchor = document.sections[0]
            .paragraphs
            .get_mut(table.paragraph)
            .ok_or_else(invalid_caption)?;
        let Some(Control::Table(t)) = anchor.controls.get_mut(table.control) else {
            return Err(invalid_caption());
        };
        let caption = t.caption.take().ok_or_else(invalid_caption)?;
        let gap = u32::try_from(caption.spacing).map_err(|_| invalid_caption())?;
        let spacing = block
            .checked_add(gap)
            .and_then(|extent| i32::try_from(extent).ok())
            .and_then(|extent| extent.checked_mul(2))
            .ok_or_else(invalid_caption)?;
        let format = caption
            .paragraphs
            .first()
            .and_then(|paragraph| paragraph.char_shapes.first())
            .map(|shape| shape.char_shape_id)
            .ok_or_else(invalid_caption)?;
        let (shape, style) = (anchor.para_shape_id, anchor.style_id);
        let moves = matches!(
            anchor.column_type,
            ColumnBreakType::Page | ColumnBreakType::Column
        );
        let column_type = if moves {
            std::mem::take(&mut anchor.column_type)
        } else {
            ColumnBreakType::None
        };
        let synthesized = moves && std::mem::take(&mut anchor.page_break_synthesized);
        let breaks = anchor.raw_break_type & 0xC;
        anchor.raw_break_type &= !0xC;
        let id = match spacer_shapes.get(&(shape, spacing)) {
            Some(id) => *id,
            None => {
                let shapes = &mut document.doc_info.para_shapes;
                let mut fixed = shapes
                    .get(usize::from(shape))
                    .ok_or_else(invalid_caption)?
                    .clone();
                fixed.line_spacing_type = LineSpacingType::Fixed;
                fixed.line_spacing = spacing;
                fixed.spacing_before = 0;
                fixed.spacing_after = 0;
                fixed.raw_data = None;
                let id = u16::try_from(shapes.len()).map_err(|_| invalid_caption())?;
                shapes.push(fixed);
                spacer_shapes.insert((shape, spacing), id);
                id
            }
        };
        let mut spacer = Paragraph::new_empty();
        spacer.set_single_char_shape(format);
        spacer.style_id = style;
        spacer.para_shape_id = id;
        spacer.insert_text_at(0, ".");
        spacer.column_type = column_type;
        spacer.raw_break_type = breaks;
        spacer.page_break_synthesized = synthesized;
        crate::template::discard_stale_layout(&mut spacer);
        document.sections[0]
            .paragraphs
            .insert(table.paragraph, spacer);
    }
    document.doc_info.raw_stream_dirty = true;
    copy.spaced = tables.iter().map(|table| table.paragraph).collect();
    Ok(())
}

fn captioned_tables(document: &Document) -> Result<Vec<CaptionedTable>, String> {
    use rhwp::model::shape::{CaptionDirection, TextWrap, VertRelTo};
    if document.sections.len() != 1 {
        return Err("Table pagination requires one section".into());
    }
    let mut tables = Vec::new();
    for (paragraph, p) in document.sections[0].paragraphs.iter().enumerate().skip(1) {
        let mut found = p
            .controls
            .iter()
            .enumerate()
            .filter_map(|(control, c)| match c {
                Control::Table(table)
                    if !table.common.treat_as_char
                        && matches!(table.common.vert_rel_to, VertRelTo::Para)
                        && matches!(table.common.text_wrap, TextWrap::TopAndBottom) =>
                {
                    table
                        .caption
                        .as_ref()
                        .filter(|caption| {
                            caption.direction == CaptionDirection::Top
                                && !caption.paragraphs.is_empty()
                        })
                        .map(|caption| (control, table, caption))
                }
                _ => None,
            });
        // One spacer stands for one caption: a paragraph holding several such
        // tables keeps their captions.
        let (Some((control, table, caption)), None) = (found.next(), found.next()) else {
            continue;
        };
        let margins = if caption.include_margin {
            i64::from(table.common.margin.left) + i64::from(table.common.margin.right)
        } else {
            0
        };
        let width = u32::try_from(i64::from(table.common.width) + margins)
            .map_err(|_| invalid_caption())?;
        tables.push(CaptionedTable {
            paragraph,
            control,
            width,
        });
    }
    Ok(tables)
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
    /// last break and end after the next 1, 2, 4, … candidates until one needs a
    /// break. One whole reading of the copy with every break certifies the result;
    /// a window that cannot be used, or a failed certification, starts over on
    /// whole readings. The breaks found go into the document at once.
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
        let events = copy.events(&self.anchors);
        let first = Reading::of(&copy.document, 0)?;
        let mut current = None;
        for (index, &event) in events.iter().enumerate() {
            if self.needs(&copy, &first, event)?.is_some() {
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
            return self.on_full_readings(&mut copy, &events, Some(first));
        };
        if let Some(broken) = self.windows(&mut copy, &events, &mut windows, first, current)?
            && self.settled(&copy, &events)
        {
            self.method = Method::Windows;
            return Ok(broken);
        }
        self.method = Method::WindowsThenFull;
        copy.document = pristine;
        self.on_full_readings(&mut copy, &events, None)
    }

    /// The break `event` needs in `reading` of `copy`, if any.
    fn needs(&self, copy: &Copy, reading: &Reading, event: Event) -> Result<Option<Needs>, String> {
        match event {
            Event::Anchor(index) => {
                let (header, body) = reading.positions(&copy.anchor(&self.anchors[index]))?;
                Ok((header != body).then_some(Needs::Page))
            }
            Event::Spacer(spacer) => {
                let column_type = copy.document.sections[0]
                    .paragraphs
                    .get(spacer)
                    .ok_or_else(invalid)?
                    .column_type;
                let local = spacer.checked_sub(reading.offset).ok_or_else(invalid)?;
                let text = *reading
                    .fragments
                    .first_text
                    .get(&local)
                    .ok_or_else(invalid)?;
                let table = *reading
                    .fragments
                    .first_table
                    .get(&(local + 1))
                    .ok_or_else(invalid)?;
                Ok(
                    if table.0 > text.0 && column_type != ColumnBreakType::Page {
                        Some(Needs::Page)
                    } else if table.0 == text.0
                        && table.1 > text.1
                        && column_type == ColumnBreakType::None
                    {
                        Some(Needs::Column)
                    } else {
                        None
                    },
                )
            }
        }
    }

    /// The copy paragraph where `event`'s break goes, and the page in `reading`
    /// where the content it moves starts.
    fn moves(
        &self,
        copy: &Copy,
        reading: &Reading,
        event: Event,
    ) -> Result<(usize, usize), String> {
        match event {
            Event::Anchor(index) => {
                let anchor = &self.anchors[index];
                let (header, _) = reading.positions(&copy.anchor(anchor))?;
                Ok((copy.start(anchor.paragraph), header.0))
            }
            Event::Spacer(spacer) => {
                let local = spacer.checked_sub(reading.offset).ok_or_else(invalid)?;
                let text = reading
                    .fragments
                    .first_text
                    .get(&local)
                    .ok_or_else(invalid)?;
                Ok((spacer, text.0))
            }
        }
    }

    /// The copy paragraph after the table that `event` concerns.
    fn end(&self, copy: &Copy, event: Event) -> usize {
        match event {
            Event::Anchor(index) => copy.index(self.anchors[index].paragraph) + 1,
            Event::Spacer(spacer) => spacer + 2,
        }
    }

    /// Breaks found from windows after event `current` needed one in `found_in`,
    /// added to `copy`; None when a window cannot be used.
    fn windows(
        &self,
        copy: &mut Copy,
        events: &[Event],
        windows: &mut page_window::Windows,
        mut found_in: Reading,
        mut current: usize,
    ) -> Result<Option<Vec<usize>>, String> {
        let mut broken = Vec::new();
        loop {
            let event = events[current];
            if self.needs(copy, &found_in, event)? != Some(Needs::Page) {
                return Ok(None);
            }
            let (start, page) = self.moves(copy, &found_in, event)?;
            let Some(page_number) = found_in
                .fragments
                .pages
                .get(page)
                .and_then(|page| page.number)
                .and_then(|number| number.checked_add(1))
            else {
                return Ok(None);
            };
            if found_in.carries_into(page + 1, start) {
                return Ok(None);
            }
            let paragraph = copy.document.sections[0]
                .paragraphs
                .get_mut(start)
                .ok_or("Missing generated table pagination paragraph")?;
            match event {
                Event::Spacer(_) => break_spacer(paragraph, Needs::Page),
                Event::Anchor(index) => {
                    if add_page_break(paragraph, start).is_none() {
                        // A split table whose anchor already starts a page cannot be
                        // fixed; a window's verdict on it is left to whole readings.
                        if found_in.offset == 0 {
                            return Err(cannot_fit(&self.anchors[index]));
                        }
                        return Ok(None);
                    }
                    broken.push(index);
                }
            }
            let mut settled = current + 1;
            let mut step = 1usize;
            let next = loop {
                let last = settled.saturating_add(step).min(events.len());
                let end = self.end(copy, events[last - 1]);
                let Some(document) = windows.document(&copy.document, start, end, page_number)
                else {
                    return Ok(None);
                };
                let Ok(window) = Reading::of(&document, start - 1) else {
                    return Ok(None);
                };
                let fixed = matches!(self.needs(copy, &window, event), Ok(None))
                    && self
                        .moves(copy, &window, event)
                        .ok()
                        .and_then(|(_, page)| window.fragments.pages.get(page)?.number)
                        == Some(page_number);
                if !fixed || !windows.numbers_match(window.core.document(), start) {
                    return Ok(None);
                }
                let mut split = None;
                for (index, &later) in events.iter().enumerate().take(last).skip(settled) {
                    match self.needs(copy, &window, later) {
                        Ok(None) => {}
                        Ok(Some(_)) => {
                            split = Some(index);
                            break;
                        }
                        Err(_) => return Ok(None),
                    }
                }
                if let Some(index) = split {
                    break Some((index, window));
                }
                settled = last;
                if settled == events.len() {
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

    /// Whether no event needs a break in one whole reading of `copy`.
    fn settled(&self, copy: &Copy, events: &[Event]) -> bool {
        Reading::of(&copy.document, 0).is_ok_and(|reading| {
            events
                .iter()
                .all(|&event| matches!(self.needs(copy, &reading, event), Ok(None)))
        })
    }

    /// The correction on whole readings of `copy`. `reading` is a reading of
    /// `copy` as it is, if one exists.
    fn on_full_readings(
        &self,
        copy: &mut Copy,
        events: &[Event],
        mut reading: Option<Reading>,
    ) -> Result<Vec<usize>, String> {
        self.correct_with(copy, events, |copy, event, changed| {
            if changed || reading.is_none() {
                reading = Some(Reading::of(&copy.document, 0)?);
            }
            self.needs(copy, reading.as_ref().ok_or_else(invalid)?, event)
        })
    }

    /// Decides `events` in order. `check` tells what an event needs and receives
    /// true when the copy changed since its previous call.
    fn correct_with(
        &self,
        copy: &mut Copy,
        events: &[Event],
        mut check: impl FnMut(&Copy, Event, bool) -> Result<Option<Needs>, String>,
    ) -> Result<Vec<usize>, String> {
        let mut broken = Vec::new();
        for &event in events {
            let Some(needs) = check(copy, event, false)? else {
                continue;
            };
            match event {
                Event::Spacer(spacer) => {
                    let paragraph = copy.document.sections[0]
                        .paragraphs
                        .get_mut(spacer)
                        .ok_or_else(invalid)?;
                    break_spacer(paragraph, needs);
                    check(copy, event, true)?;
                }
                Event::Anchor(index) => {
                    let anchor = &self.anchors[index];
                    let start = copy.start(anchor.paragraph);
                    let paragraph = copy.document.sections[0]
                        .paragraphs
                        .get_mut(start)
                        .ok_or("Missing generated table pagination paragraph")?;
                    add_page_break(paragraph, start).ok_or_else(|| cannot_fit(anchor))?;
                    broken.push(index);
                    if check(copy, event, true)?.is_some() {
                        return Err(cannot_fit(anchor));
                    }
                }
            }
        }
        Ok(broken)
    }
}

/// Break a caption spacer that its table's first rows left behind, as Hancom moves
/// the caption with its table. Spacer breaks stay in the copy.
fn break_spacer(paragraph: &mut Paragraph, needs: Needs) {
    match needs {
        Needs::Page => {
            paragraph.column_type = ColumnBreakType::Page;
            paragraph.raw_break_type = (paragraph.raw_break_type & !8) | 4;
        }
        Needs::Column => {
            paragraph.column_type = ColumnBreakType::Column;
            paragraph.raw_break_type |= 8;
        }
    }
    paragraph.page_break_synthesized = false;
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
    /// Where each root paragraph's first text lines start.
    first_text: BTreeMap<usize, Position>,
    /// Where the first rows of each root paragraph's tables start.
    first_table: BTreeMap<usize, Position>,
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
        let (mut first_text, mut first_table) = (BTreeMap::new(), BTreeMap::new());
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
                    if kind == "fullParagraph" || kind == "partialParagraph" {
                        first_text
                            .entry(number(item, "paraIndex")?)
                            .or_insert((page_index, column_index));
                    }
                    if kind != "table" && kind != "partialTable" {
                        continue;
                    }
                    let key = (number(item, "paraIndex")?, number(item, "controlIndex")?);
                    if item.get("isContinuation").and_then(Value::as_bool) != Some(true) {
                        first_table
                            .entry(key.0)
                            .or_insert((page_index, column_index));
                    }
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
            first_text,
            first_table,
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
    use md2hwp_core::{
        ValidationLimits,
        ir::{
            Block, Document as Ir, IR_VERSION, ImageRef, Inline, Metadata, SCHEMA_NAME,
            SourceParagraph, TableAlignment,
        },
        validate,
    };
    use rhwp::model::{document::Document, paragraph::Paragraph};
    use serde_json::json;

    fn positions(pages: &Value, anchor: &Anchor) -> Result<(Position, Position), String> {
        Fragments::read(pages.clone())?.positions(anchor)
    }

    fn text(value: &str) -> Vec<Inline> {
        vec![Inline::Text {
            value: value.into(),
        }]
    }

    fn saved_figure_and_table(notes: usize, fillers: usize) -> (Vec<u8>, Plan) {
        saved_document(notes, fillers, false)
    }

    /// Heading, a picture with a caption and `notes` source paragraphs, `fillers`
    /// body paragraphs and a header/body table, captioned with `table_caption`,
    /// rendered into the default template and saved.
    fn saved_document(notes: usize, fillers: usize, table_caption: bool) -> (Vec<u8>, Plan) {
        let document = crate::template_init::create().unwrap();
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(
            directory.path().join("image.png"),
            document.bin_data_content[0].data.load(),
        )
        .unwrap();
        let mut blocks = vec![
            Block::Heading {
                id: None,
                level: 1,
                inlines: text("장"),
            },
            Block::Figure {
                id: None,
                image: ImageRef {
                    path: "image.png".into(),
                    alt: vec![],
                    title: None,
                },
                caption: text("그림"),
                source: Some(
                    (0..notes)
                        .map(|_| SourceParagraph {
                            prefix: "주".into(),
                            inlines: text("설명"),
                        })
                        .collect(),
                ),
            },
        ];
        blocks.extend((0..fillers).map(|_| Block::Paragraph {
            inlines: text("본문"),
        }));
        blocks.push(Block::Table {
            id: None,
            columns: vec![TableAlignment::Default],
            header: vec![text("머리")],
            rows: vec![vec![text("본문")]],
            caption: table_caption.then(|| text("표")),
            source: None,
        });
        rendered(blocks, &directory, "image.png")
    }

    fn hwpunit(px: f64) -> u32 {
        (px * 7200.0 / DEFAULT_DPI).round() as u32
    }

    /// The saved document with its picture `height` HWPUNIT tall.
    fn with_picture_height(
        base: &Document,
        (paragraph, control): (usize, usize),
        height: u32,
    ) -> Vec<u8> {
        let mut document = base.clone();
        let Control::Picture(p) = &mut document.sections[0].paragraphs[paragraph].controls[control]
        else {
            panic!("not a picture")
        };
        p.common.height = height;
        p.shape_attr.current_height = height;
        document.sections[0].raw_stream = None;
        rhwp::serializer::serialize_document(&document).unwrap()
    }

    /// Page items of the saved file itself, as rhwp opens it.
    fn opened_pages(saved: &[u8]) -> Value {
        let mut core = DocumentCore::from_bytes(saved).unwrap();
        core.repaginate_if_needed();
        core.dump_page_items_json(None)
    }

    /// The copy of `saved` with its caption spacers settled as the correction
    /// settles them, and its whole reading.
    fn settled_copy(saved: &[u8]) -> (Copy, Reading) {
        let mut copy = Copy::new(saved).unwrap();
        let spacers = copy.events(&[]);
        Plan::default()
            .on_full_readings(&mut copy, &spacers, None)
            .unwrap();
        let reading = Reading::of(&copy.document, 0).unwrap();
        (copy, reading)
    }

    /// Header and first body row of `anchor` in the copy, as the correction reads it.
    fn verification_positions(saved: &[u8], anchor: &Anchor) -> (Position, Position) {
        let (copy, reading) = settled_copy(saved);
        reading.positions(&copy.anchor(anchor)).unwrap()
    }

    #[test]
    fn replaces_table_caption_with_a_spacer_of_its_height() {
        let (saved, plan) = saved_document(1, 0, true);
        let [anchor] = plan.anchors.as_slice() else {
            panic!("expected one table anchor")
        };
        let mut document = rhwp::parser::parse_hwp(&saved).unwrap();
        let Control::Table(table) =
            &document.sections[0].paragraphs[anchor.paragraph].controls[anchor.control]
        else {
            panic!("not a table")
        };
        let caption = table.caption.as_ref().unwrap();
        assert_eq!(caption.direction, rhwp::model::shape::CaptionDirection::Top);
        assert_eq!(caption.paragraphs.len(), 1);

        let copy = Copy::new(&saved).unwrap();
        assert_eq!(copy.spaced, vec![anchor.paragraph]);
        let moved = copy.anchor(anchor);
        assert_eq!(moved.paragraph, anchor.paragraph + 1);
        assert_eq!(copy.start(anchor.paragraph), anchor.paragraph);
        let paragraphs = &copy.document.sections[0].paragraphs;
        assert_eq!(paragraphs[anchor.paragraph].text, ".");
        let Control::Table(table) = &paragraphs[moved.paragraph].controls[anchor.control] else {
            panic!("not a table")
        };
        assert!(table.caption.is_none());
        // One 10 pt caption line and the 283 HU caption gap, as Hancom stores it.
        let bytes = rhwp::serializer::serialize_document(&copy.document).unwrap();
        let mut core = DocumentCore::from_bytes(&bytes).unwrap();
        core.repaginate_if_needed();
        let spacer = core
            .production_paragraph_height(0, anchor.paragraph, None)
            .unwrap();
        assert_eq!(hwpunit(spacer), 283 + 1000);

        // A break the anchor has in the document moves to the spacer in the copy,
        // so that the caption's space moves with the table.
        add_page_break(
            &mut document.sections[0].paragraphs[anchor.paragraph],
            anchor.paragraph,
        )
        .unwrap();
        document.sections[0].raw_stream = None;
        let file = rhwp::serializer::serialize_document(&document).unwrap();
        let copy = Copy::new(&file).unwrap();
        let paragraphs = &copy.document.sections[0].paragraphs;
        assert_eq!(
            paragraphs[anchor.paragraph].column_type,
            ColumnBreakType::Page
        );
        assert_ne!(paragraphs[anchor.paragraph].raw_break_type & 4, 0);
        assert_eq!(
            paragraphs[moved.paragraph].column_type,
            ColumnBreakType::None
        );
        assert_eq!(paragraphs[moved.paragraph].raw_break_type & 0xC, 0);
    }

    #[test]
    fn table_moves_whole_with_its_caption_spacer() {
        let (saved, mut plan) = saved_document(3, 10, true);
        let [anchor] = plan.anchors.clone().try_into().unwrap();
        let base = rhwp::parser::parse_hwp(&saved).unwrap();
        let at = picture_at(&base);
        // Grow the picture in 3 px steps until the spacer still fits on the first
        // page but the table's first rows do not: the copy then breaks the spacer.
        let file = (hwpunit(400.0)..hwpunit(720.0))
            .step_by(225)
            .map(|height| with_picture_height(&base, at, height))
            .find(|file| {
                let (copy, _) = settled_copy(file);
                copy.document.sections[0].paragraphs[anchor.paragraph].column_type
                    == ColumnBreakType::Page
            })
            .expect("the copy moves a table with its caption spacer");
        let (copy, reading) = settled_copy(&file);
        let spacer = reading.fragments.first_text[&anchor.paragraph];
        assert_eq!(spacer, (1, 0));
        assert_eq!(
            reading.positions(&copy.anchor(&anchor)).unwrap(),
            (spacer, spacer)
        );
        // The spacer break models Hancom moving the caption with its table; the
        // document gets no break.
        let mut core = DocumentCore::from_bytes(&file).unwrap();
        assert_eq!(plan.correct(&mut core).unwrap(), 0);
    }

    fn picture(
        document: &Document,
        (paragraph, control): (usize, usize),
    ) -> &rhwp::model::image::Picture {
        let Control::Picture(picture) =
            &document.sections[0].paragraphs[paragraph].controls[control]
        else {
            panic!("not a picture")
        };
        picture
    }

    /// rhwp's own height of a root paragraph once the file is opened.
    fn opened_height(saved: &[u8], paragraph: usize) -> u32 {
        let mut core = DocumentCore::from_bytes(saved).unwrap();
        core.repaginate_if_needed();
        hwpunit(
            core.production_paragraph_height(0, paragraph, None)
                .unwrap(),
        )
    }

    #[test]
    fn replaces_caption_with_picture_height_like_hancoms_line() {
        let (saved, _) = saved_figure_and_table(1, 0);
        let document = rhwp::parser::parse_hwp(&saved).unwrap();
        let at = picture_at(&document);
        let before = picture(&document, at);
        let caption = before.caption.as_ref().unwrap();
        assert!(before.common.treat_as_char);
        assert_eq!(
            caption.direction,
            rhwp::model::shape::CaptionDirection::Bottom
        );
        assert_eq!(caption.paragraphs.len(), 2);
        // rhwp 0.9.1 derives the picture's line without its caption. A pin that
        // includes it would count the caption twice here: revert the picture
        // caption replacement then.
        assert_eq!(opened_height(&saved, at.0), before.common.height + 600);

        let copy = Copy::new(&saved).unwrap();
        // Two one-line 10 pt caption paragraphs at 160%: 1000 + 600 + 1000 HU,
        // without the last line's gap, plus the 283 HU caption gap.
        let extent = 283 + 2600;
        let after = picture(&copy.document, at);
        assert!(after.caption.is_none());
        assert_eq!(after.common.height, before.common.height + extent);
        assert_eq!(
            after.shape_attr.current_height,
            before.shape_attr.current_height + extent
        );
        let unrelated = |document: &Document| -> Vec<String> {
            document.sections[0]
                .paragraphs
                .iter()
                .enumerate()
                .filter(|(index, _)| *index != at.0)
                .map(|(_, p)| format!("{p:?}"))
                .collect()
        };
        assert_eq!(unrelated(&copy.document), unrelated(&document));
        // The copy is written and read back with the taller picture and no caption.
        let probe = rhwp::serializer::serialize_document(&copy.document).unwrap();
        let reopened = rhwp::parser::parse_hwp(&probe).unwrap();
        assert!(picture(&reopened, at).caption.is_none());
        assert_eq!(
            opened_height(&probe, at.0),
            before.common.height + extent + 600
        );
    }

    #[test]
    fn table_split_only_by_an_omitted_picture_caption_stays_whole() {
        let (saved, plan) = saved_figure_and_table(3, 10);
        let [anchor] = plan.anchors.as_slice() else {
            panic!("expected one table anchor")
        };
        let base = rhwp::parser::parse_hwp(&saved).unwrap();
        let at = picture_at(&base);
        // Grow the picture in 3 px steps until rhwp's own layout, with the
        // caption in place, leaves the header alone at the bottom of the first page.
        let (file, header, body) = (hwpunit(560.0)..hwpunit(680.0))
            .step_by(225)
            .map(|height| with_picture_height(&base, at, height))
            .find_map(|file| {
                let (header, body) = positions(&opened_pages(&file), anchor).unwrap();
                (header != body).then_some((file, header, body))
            })
            .expect("rhwp splits the table with the caption in place");
        assert_eq!((header.0 + 1, header.1), body);
        // The copy that the correction reads keeps it whole.
        assert_eq!(verification_positions(&file, anchor), (body, body));
    }

    /// The saved file with the table's header alone at the bottom of the first page
    /// in the copy, found by growing the picture in 3 px steps.
    fn saved_with_orphaned_header(saved: &[u8], anchor: &Anchor) -> Vec<u8> {
        let base = rhwp::parser::parse_hwp(saved).unwrap();
        let at = picture_at(&base);
        (hwpunit(400.0)..hwpunit(720.0))
            .step_by(225)
            .map(|height| with_picture_height(&base, at, height))
            .find(|file| {
                let (header, body) = verification_positions(file, anchor);
                header != body
            })
            .expect("the copy splits the table")
    }

    /// Heading, `before` body paragraphs, then, when `first` is set, a table with a
    /// header and three body rows, `between` body paragraphs, a table with a header
    /// and two body rows whose first body cell holds a footnote when `note` is set,
    /// and a body paragraph. Each table has a one-line top caption.
    fn saved_tables(before: usize, first: bool, between: usize, note: bool) -> (Vec<u8>, Plan) {
        use md2hwp_core::ir::FootnoteBlock;
        let body = || Block::Paragraph {
            inlines: text("본문"),
        };
        let table = |rows| Block::Table {
            id: None,
            columns: vec![TableAlignment::Default],
            header: vec![text("머리")],
            rows,
            caption: Some(text("표")),
            source: None,
        };
        let mut blocks = vec![Block::Heading {
            id: None,
            level: 1,
            inlines: text("장"),
        }];
        blocks.extend((0..before).map(|_| body()));
        if first {
            blocks.push(table((0..3).map(|_| vec![text("본문")]).collect()));
        }
        blocks.extend((0..between).map(|_| body()));
        let mut cell = text("본문");
        if note {
            cell.push(Inline::Footnote {
                blocks: vec![FootnoteBlock::Paragraph {
                    inlines: text("주석"),
                }],
            });
        }
        blocks.push(table(vec![vec![cell], vec![text("본문")]]));
        blocks.push(body());
        rendered(blocks, &tempfile::tempdir().unwrap(), "image.png")
    }

    #[test]
    fn table_after_a_page_top_table_stays_whole() {
        // 39 body lines fill the first page, so the first table starts the second;
        // 37 lines after it leave 17 px under the second table's anchor, less than
        // its caption and header row (30.4 px). Hancom 2020 moves that table whole
        // to the third page.
        let (saved, mut plan) = saved_tables(39, true, 37, false);
        let [first, second] = plan.anchors.clone().try_into().unwrap();
        let pages = opened_pages(&saved);
        assert_eq!(
            pages[1]["columns"][0]["items"][0]["paraIndex"].as_u64(),
            Some(first.paragraph as u64)
        );
        // rhwp 0.9.1 leaves the first table's caption out of its page cursor and
        // keeps the second table's header alone on the second page. A pin that
        // counts the caption moves the table whole: drop the spacers once this
        // and table_with_a_cell_footnote_stays_whole both fail.
        assert_eq!(positions(&pages, &second).unwrap(), ((1, 0), (2, 0)));
        assert_eq!(verification_positions(&saved, &second), ((2, 0), (2, 0)));
        let mut core = DocumentCore::from_bytes(&saved).unwrap();
        assert_eq!(plan.correct(&mut core).unwrap(), 0);
    }

    #[test]
    fn table_with_a_cell_footnote_stays_whole() {
        // 38 body lines leave 23.5 px under the table's anchor: room for its header
        // row (13.3 px) but not for its caption and header (30.4 px). Hancom 2020
        // moves the table whole to the second page, with or without the footnote.
        let (saved, plan) = saved_tables(38, false, 0, false);
        let [anchor] = plan.anchors.as_slice() else {
            panic!("expected one table anchor")
        };
        let pages = opened_pages(&saved);
        assert_eq!(positions(&pages, anchor).unwrap(), ((1, 0), (1, 0)));
        let (saved, mut plan) = saved_tables(38, false, 0, true);
        let [anchor] = plan.anchors.clone().try_into().unwrap();
        // With the footnote, rhwp 0.9.1 keeps the header alone on the first page:
        // its first-row check leaves the caption out. A pin that counts it moves
        // the table whole: drop the spacers once this and
        // table_after_a_page_top_table_stays_whole both fail.
        let pages = opened_pages(&saved);
        assert_eq!(positions(&pages, &anchor).unwrap(), ((0, 0), (1, 0)));
        assert_eq!(verification_positions(&saved, &anchor), ((1, 0), (1, 0)));
        let mut core = DocumentCore::from_bytes(&saved).unwrap();
        assert_eq!(plan.correct(&mut core).unwrap(), 0);
    }

    /// `blocks` validated, rendered into the default template and saved, with the
    /// figure image `image` in `directory`.
    fn rendered(blocks: Vec<Block>, directory: &tempfile::TempDir, image: &str) -> (Vec<u8>, Plan) {
        let resources: BTreeMap<_, _> = blocks
            .iter()
            .enumerate()
            .filter(|(_, block)| matches!(block, Block::Figure { .. }))
            .map(|(index, _)| {
                (
                    index,
                    crate::image_resource::Resource::read(directory.path(), image).unwrap(),
                )
            })
            .collect();
        let input = validate(
            Ir {
                schema: SCHEMA_NAME.into(),
                ir_version: IR_VERSION.into(),
                metadata: Metadata::default(),
                blocks,
            },
            &ValidationLimits::default(),
        )
        .unwrap();
        let mut core = DocumentCore::new_empty();
        core.set_document(crate::template_init::create().unwrap());
        let plan = {
            let _rendering = RENDERING.lock().unwrap_or_else(|e| e.into_inner());
            crate::template::render(&input, &mut core, &resources)
                .unwrap()
                .1
        };
        (export(&core), plan)
    }

    /// The root paragraph and control of the only picture.
    fn picture_at(document: &Document) -> (usize, usize) {
        let mut found =
            document.sections[0]
                .paragraphs
                .iter()
                .enumerate()
                .flat_map(|(paragraph, p)| {
                    p.controls
                        .iter()
                        .enumerate()
                        .filter(|(_, c)| matches!(c, Control::Picture(_)))
                        .map(move |(control, _)| (paragraph, control))
                });
        let at = found.next().expect("a picture");
        assert!(found.next().is_none(), "expected one picture");
        at
    }

    #[test]
    fn measures_caption_blocks_to_their_last_line() {
        let (saved, _) = saved_figure_and_table(1, 0);
        let document = rhwp::parser::parse_hwp(&saved).unwrap();
        let (paragraph, control) = picture_at(&document);
        let Control::Picture(picture) =
            &document.sections[0].paragraphs[paragraph].controls[control]
        else {
            panic!("not a picture")
        };
        let caption = picture.caption.as_ref().unwrap();
        // The figure caption and one source note: two one-line 10 pt caption
        // paragraphs at 160%, 1000 + 600 + 1000 HU without the last line's gap.
        assert_eq!(caption.paragraphs.len(), 2);
        assert_eq!(
            caption_blocks(&document, &[(caption, picture.common.width)]).unwrap(),
            vec![2600]
        );
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
    fn plain_copy() -> Copy {
        Copy {
            document: document(),
            spaced: Vec::new(),
        }
    }
    const BOTH: [Event; 2] = [Event::Anchor(0), Event::Anchor(1)];
    #[test]
    fn corrects_only_anchor_and_rereads_later_tables() {
        let mut copy = plain_copy();
        copy.document.sections[0].paragraphs[1].raw_break_type = 8;
        let mut second = anchor();
        second.paragraph = 2;
        let plan = Plan {
            anchors: vec![anchor(), second],
            ..Plan::default()
        };
        let before = copy.document.sections[0].paragraphs[0].clone();
        let mut calls = 0;
        assert_eq!(
            plan.correct_with(&mut copy, &BOTH, |_, _, _| {
                calls += 1;
                Ok((calls == 1).then_some(Needs::Page))
            })
            .unwrap(),
            vec![0]
        );
        assert_eq!(calls, 3);
        let ps = &copy.document.sections[0].paragraphs;
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
        let mut copy = plain_copy();
        let first = [Event::Anchor(0)];
        assert!(
            plan.correct_with(&mut copy, &first, |_, _, _| Ok(None))
                .unwrap()
                .is_empty()
        );
        let mut calls = 0;
        assert!(
            plan.correct_with(&mut copy, &first, |_, _, _| {
                calls += 1;
                Ok(Some(Needs::Page))
            })
            .is_err()
        );
        assert_eq!(calls, 2);
        calls = 0;
        assert!(
            plan.correct_with(&mut copy, &first, |_, _, _| {
                calls += 1;
                Ok(Some(Needs::Page))
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
    fn copy_before(core: &DocumentCore, plan: &Plan) -> Copy {
        let mut document = core.document().clone();
        for added in &plan.added {
            let paragraph = &mut document.sections[0].paragraphs[added.paragraph];
            paragraph.column_type = added.column_type;
            paragraph.raw_break_type = added.raw_break_type;
            paragraph.page_break_synthesized = added.page_break_synthesized;
        }
        let mut writer = DocumentCore::new_empty();
        writer.set_document(document);
        Copy::new(&export(&writer)).unwrap()
    }

    #[test]
    fn windows_find_the_breaks_of_whole_copy_readings() {
        let (saved, mut plan) = saved_figure_and_table(3, 10);
        let [anchor] = plan.anchors.clone().try_into().unwrap();
        let mut core =
            DocumentCore::from_bytes(&saved_with_orphaned_header(&saved, &anchor)).unwrap();
        assert_eq!(plan.correct(&mut core).unwrap(), 1);
        assert_eq!(plan.method, Method::Windows);
        // The same document corrected on whole readings of its copy only.
        let mut copy = copy_before(&core, &plan);
        let events = copy.events(&plan.anchors);
        let broken: Vec<_> = plan
            .on_full_readings(&mut copy, &events, None)
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
        let mut copy = Copy::new(&export(&core)).unwrap();
        let first = Reading::of(&copy.document, 0).unwrap();
        let mut windows =
            page_window::Windows::prepare(&mut copy.document, first.core.document()).unwrap();
        let end = copy.document.sections[0].paragraphs.len();
        assert!(!plan.anchors.is_empty());
        for anchor in &plan.anchors {
            let start = copy.start(anchor.paragraph);
            let document = windows.document(&copy.document, start, end, 2).unwrap();
            let window = Reading::of(&document, start - 1).unwrap();
            assert!(windows.numbers_match(window.core.document(), start));
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
