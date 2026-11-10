// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Euisoon Ahn
//! IR math as native equations with Hancom-like stored geometry.
//!
//! Hancom keeps an equation's stored size and baseline when it opens a file,
//! so the geometry comes from `equation_metrics`, a calculator fitted to Hancom
//! measurements. It never rejects a script; unmeasured parts are estimated.
//! Native fields follow equations that Hancom 2020 itself saved: inline
//! equations use numbering type None, display equations type Equation.

use std::collections::{BTreeMap, BTreeSet};

use md2hwp_core::{
    ir::{Block, FootnoteBlock, Inline, ListItem, ListItemBlock},
    validate::{ValidatedDocument, ValidationLimits, validate},
};
use rhwp::model::{
    Padding,
    control::{AutoNumberType, Control, Equation},
    document::{DocInfo, Document},
    paragraph::Paragraph,
    shape::{CommonObjAttr, HorzRelTo, ObjectNumberingType, SizeCriterion, TextWrap, VertRelTo},
};

use crate::model_tools::{IdentityAllocator, ObjectOrders};
use crate::template::tag;

pub(crate) const UNSUPPORTED_LIST_MATH: &str =
    "rhwp generation does not yet support display math in list items";

/// Paragraph control-mask bit of an equation (HWP5 control code 11).
const EQUATION_MASK: u32 = 1 << 11;
const SIZE_PREFIX: &str = "display_math.fontsize-pt:";

/// Inline math replaced by adapter-private text markers until native insertion.
pub(crate) struct Plan {
    prefix: String,
    scripts: BTreeMap<String, String>,
}

impl Plan {
    pub(crate) fn prepare(
        ir: &ValidatedDocument,
        roots: &[Paragraph],
    ) -> Result<(Self, ValidatedDocument), String> {
        // ASCII prefixes survive JSON quoting, so a plain search covers the
        // manuscript, metadata, verbatim literals and every template container.
        let original = serde_json::to_string(ir.as_document()).map_err(|e| e.to_string())?;
        let template = serde_json::to_string(roots).map_err(|e| e.to_string())?;
        let mut generation = 0_u64;
        let prefix = loop {
            let candidate = format!("MD2HWP_MATH_{generation}_");
            if !original.contains(&candidate) && !template.contains(&candidate) {
                break candidate;
            }
            generation = generation
                .checked_add(1)
                .ok_or("Math marker namespace exhausted")?;
        };
        let mut plan = Self {
            prefix,
            scripts: BTreeMap::new(),
        };
        let mut document = ir.as_document().clone();
        for block in &mut document.blocks {
            match block {
                Block::Paragraph { inlines } | Block::Heading { inlines, .. } => {
                    plan.mark(inlines);
                }
                Block::List { items, .. } => plan.mark_items(items),
                _ => {}
            }
        }
        let marker_bytes: usize = plan.scripts.keys().map(String::len).sum();
        let mut limits = ValidationLimits::default();
        // Scripts already paid for their text; only marker expansion is extra.
        limits.max_text_bytes = limits
            .max_text_bytes
            .checked_add(marker_bytes)
            .ok_or("Math marker text limit overflow")?;
        let prepared = validate(document, &limits).map_err(|e| e.to_string())?;
        Ok((plan, prepared))
    }

    fn mark_items(&mut self, items: &mut [ListItem]) {
        for item in items {
            for block in &mut item.blocks {
                match block {
                    ListItemBlock::Paragraph { inlines } => self.mark(inlines),
                    ListItemBlock::List { items, .. } => self.mark_items(items),
                    ListItemBlock::DisplayMath { .. } => {}
                }
            }
        }
    }

    fn mark(&mut self, inlines: &mut [Inline]) {
        for inline in inlines {
            match inline {
                Inline::InlineMath { script } => {
                    let marker = format!("{}{}_END", self.prefix, self.scripts.len());
                    self.scripts.insert(marker.clone(), script.clone());
                    *inline = Inline::Text { value: marker };
                }
                Inline::Footnote { blocks } => {
                    for FootnoteBlock::Paragraph { inlines } in blocks {
                        self.mark(inlines);
                    }
                }
                Inline::Strong { inlines }
                | Inline::Emph { inlines }
                | Inline::Strikeout { inlines }
                | Inline::Superscript { inlines }
                | Inline::Subscript { inlines }
                | Inline::Link { inlines, .. } => self.mark(inlines),
                _ => {}
            }
        }
    }

    /// Replace every marker, including those in generated footnote bodies.
    pub(crate) fn apply(
        &self,
        paragraphs: &mut [Paragraph],
        info: &DocInfo,
        orders: &mut ObjectOrders,
        identity: &mut IdentityAllocator,
    ) -> Result<(), String> {
        if self.scripts.is_empty() {
            return Ok(());
        }
        let model = serde_json::to_value(&*paragraphs).map_err(|e| e.to_string())?;
        self.check_contexts(&model)?;
        let mut found = BTreeSet::new();
        for paragraph in &mut *paragraphs {
            self.attach(paragraph, info, orders, identity, &mut found)?;
        }
        if found.len() != self.scripts.len() {
            return Err("A generated inline math marker was lost before native insertion".into());
        }
        let model = serde_json::to_value(&*paragraphs).map_err(|e| e.to_string())?;
        if contains(&model, &self.prefix) {
            return Err("An unresolved private math marker remains after native insertion".into());
        }
        Ok(())
    }

    fn check_contexts(&self, value: &serde_json::Value) -> Result<(), String> {
        match value {
            serde_json::Value::Object(fields) => {
                for (name, value) in fields {
                    if matches!(
                        name.as_str(),
                        "caption"
                            | "cells"
                            | "Header"
                            | "Footer"
                            | "master_pages"
                            | "HiddenComment"
                            | "memo_paragraphs"
                    ) && contains(value, &self.prefix)
                    {
                        return Err("Inline math cannot be placed in headers, footers, master pages, captions, comments or table cells".into());
                    }
                    self.check_contexts(value)?;
                }
            }
            serde_json::Value::Array(values) => {
                for value in values {
                    self.check_contexts(value)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn attach(
        &self,
        paragraph: &mut Paragraph,
        info: &DocInfo,
        orders: &mut ObjectOrders,
        identity: &mut IdentityAllocator,
        found: &mut BTreeSet<String>,
    ) -> Result<(), String> {
        while let Some(start) = paragraph.text.find(&self.prefix) {
            let (marker, script) = self
                .scripts
                .iter()
                .find(|(marker, _)| paragraph.text[start..].starts_with(marker.as_str()))
                .ok_or("Unrecognized private math marker in generated content")?;
            let unit = style_base_unit(paragraph, info)?;
            let control = fresh(equation(script, unit, false), orders, identity)?;
            crate::note_insert::insert_control(
                paragraph,
                marker,
                control,
                EQUATION_MASK,
                "Inline math",
            )?;
            found.insert(marker.clone());
        }
        crate::model_tools::map_nested_containers(paragraph, &mut |paragraphs| {
            for paragraph in paragraphs {
                self.attach(paragraph, info, orders, identity, found)?;
            }
            Ok(())
        })
    }
}

/// The template's display paragraph and its editor font size.
pub(crate) struct Display {
    sample: Paragraph,
    base_unit: u32,
}

impl Display {
    /// Reads the sample the template validator has already checked.
    pub(crate) fn read(definitions: &[Paragraph]) -> Result<Self, String> {
        let begin = definitions
            .iter()
            .position(|p| p.text == tag("begin:display_math"))
            .ok_or("Missing root begin:display_math")?;
        let sample = definitions
            .get(begin + 1)
            .filter(|p| p.text.matches(&tag("slot:display_math")).count() == 1)
            .ok_or("display_math sample must contain one slot:display_math")?
            .clone();
        let setting = definitions
            .iter()
            .find_map(|p| {
                p.text
                    .strip_prefix(&format!("{{{{md2hwp:{SIZE_PREFIX}"))
                    .and_then(|rest| rest.strip_suffix("}}"))
            })
            .ok_or("Missing root display_math.fontsize-pt setting")?;
        let mut counts = BTreeMap::new();
        counts.insert(format!("{SIZE_PREFIX}{setting}"), 1);
        let base_unit = u32::try_from(crate::template::display_math_points(&counts)?)
            .map_err(|_| "display_math.fontsize-pt is out of range")?;
        Ok(Self { sample, base_unit })
    }

    pub(crate) fn render(
        &self,
        script: &str,
        orders: &mut ObjectOrders,
        identity: &mut IdentityAllocator,
    ) -> Result<Paragraph, String> {
        let mut paragraph = self.sample.clone();
        let control = fresh(equation(script, self.base_unit, true), orders, identity)?;
        crate::note_insert::insert_control(
            &mut paragraph,
            &tag("slot:display_math"),
            control,
            EQUATION_MASK,
            "Display math",
        )?;
        Ok(paragraph)
    }
}

/// Inline equations use the character size of their paragraph style, not
/// direct character formatting (same rule as the Hancom worker).
fn style_base_unit(paragraph: &Paragraph, info: &DocInfo) -> Result<u32, String> {
    let style = info
        .styles
        .get(usize::from(paragraph.style_id))
        .ok_or("Inline math paragraph has no native paragraph style")?;
    let shape = info
        .char_shapes
        .get(usize::from(style.char_shape_id))
        .ok_or("Inline math paragraph style has no character format")?;
    u32::try_from(shape.base_size)
        .ok()
        .filter(|size| *size > 0)
        .ok_or_else(|| "Inline math paragraph style has no positive character size".into())
}

/// A native equation as Hancom 2020 saves `EquationCreate` results.
fn equation(script: &str, base_unit: u32, numbered: bool) -> Equation {
    let geometry = crate::equation_metrics::measure(script, base_unit);
    Equation {
        common: CommonObjAttr {
            // Treat as character, paragraph-relative, absolute size,
            // flow with text; bits 26-27 select the Equation number category.
            attr: if numbered { 0x0C2A_2311 } else { 0x002A_2311 },
            treat_as_char: true,
            flow_with_text: true,
            width: geometry.width,
            height: geometry.height,
            margin: Padding {
                left: 56,
                right: 56,
                top: 0,
                bottom: 0,
            },
            vert_rel_to: VertRelTo::Para,
            horz_rel_to: HorzRelTo::Para,
            text_wrap: TextWrap::TopAndBottom,
            width_criterion: SizeCriterion::Absolute,
            height_criterion: SizeCriterion::Absolute,
            hwp5_gen_shape_attr_bit26: numbered,
            numbering_type: if numbered {
                ObjectNumberingType::Equation
            } else {
                ObjectNumberingType::None
            },
            description: "수식입니다.".into(),
            ..Default::default()
        },
        script: script.into(),
        font_size: base_unit,
        baseline: geometry.baseline,
        version_info: "Equation Version 60".into(),
        // EquationCreate's default font. Hancom stores the name as given; an
        // HWPML round trip replaces it with HYhwpEQ, which lays out the same.
        font_name: "HancomEQN".into(),
        ..Default::default()
    }
}

/// Give a generated equation a fresh identity and a stacking order above all
/// existing objects.
fn fresh(
    equation: Equation,
    orders: &mut ObjectOrders,
    identity: &mut IdentityAllocator,
) -> Result<Control, String> {
    let mut detached = vec![Paragraph {
        controls: vec![Control::Equation(Box::new(equation))],
        ..Default::default()
    }];
    crate::model_tools::refresh_block_identities(&mut detached, identity)?;
    orders.assign(&mut detached)?;
    Ok(detached[0].controls.remove(0))
}

/// Hancom's Equation AUTONUM shows the counter of the latest preceding
/// Equation-numbered object; NEWNUM sets the next object's number.
pub(crate) fn recalculate_numbers(document: &mut Document) -> Result<(), String> {
    fn walk(paragraphs: &mut [Paragraph], current: &mut u32) -> Result<(), String> {
        for paragraph in paragraphs {
            for control in &mut paragraph.controls {
                match control {
                    Control::SectionDef(definition) if definition.equation_num > 0 => {
                        *current = u32::from(definition.equation_num) - 1;
                    }
                    Control::NewNumber(number)
                        if number.number_type == AutoNumberType::Equation =>
                    {
                        *current = u32::from(number.number).saturating_sub(1);
                    }
                    Control::Equation(equation)
                        if equation.common.numbering_type == ObjectNumberingType::Equation =>
                    {
                        *current = current
                            .checked_add(1)
                            .ok_or("Equation numbering overflow")?;
                    }
                    Control::AutoNumber(number)
                        if number.number_type == AutoNumberType::Equation =>
                    {
                        let value = u16::try_from(*current)
                            .map_err(|_| "Equation numbering exceeds the supported u16 range")?;
                        number.number = value;
                        number.assigned_number = value;
                    }
                    // Body containers count in document order; headers,
                    // footers and master pages do not.
                    Control::Footnote(note) => walk(&mut note.paragraphs, current)?,
                    Control::Endnote(note) => walk(&mut note.paragraphs, current)?,
                    Control::Table(table) => {
                        for cell in &mut table.cells {
                            walk(&mut cell.paragraphs, current)?;
                        }
                        if let Some(caption) = &mut table.caption {
                            walk(&mut caption.paragraphs, current)?;
                        }
                    }
                    Control::Picture(picture) => {
                        if let Some(caption) = &mut picture.caption {
                            walk(&mut caption.paragraphs, current)?;
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
    let mut current = u32::from(document.doc_properties.equation_start_num.max(1)) - 1;
    for section in &mut document.sections {
        if section.section_def.equation_num > 0 {
            current = u32::from(section.section_def.equation_num) - 1;
        }
        walk(&mut section.paragraphs, &mut current)?;
    }
    Ok(())
}

fn contains(value: &serde_json::Value, prefix: &str) -> bool {
    match value {
        serde_json::Value::String(text) => text.contains(prefix),
        serde_json::Value::Array(values) => values.iter().any(|v| contains(v, prefix)),
        serde_json::Value::Object(values) => values.values().any(|v| contains(v, prefix)),
        _ => false,
    }
}
