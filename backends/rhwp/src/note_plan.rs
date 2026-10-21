// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Euisoon Ahn
//! Adapter-private note placeholders survive ordinary heading/table composition.

use std::collections::{BTreeMap, BTreeSet};

use md2hwp_core::{
    ir::{Block, FootnoteBlock, Inline, ListItem, ListItemBlock},
    validate::{ValidatedDocument, ValidationLimits, validate},
};
use rhwp::model::{
    control::Control, document::DocInfo, footnote::FootnoteShape, paragraph::Paragraph,
};

use crate::{footnotes::Sample, model_tools::IdentityAllocator};

pub(crate) struct Plan {
    prefix: String,
    notes: BTreeMap<String, Vec<FootnoteBlock>>,
}

impl Plan {
    pub(crate) fn prepare(
        ir: &ValidatedDocument,
        roots: &[Paragraph],
    ) -> Result<(Self, ValidatedDocument), String> {
        // ASCII prefixes survive JSON quoting unchanged. This includes metadata,
        // verbatim literals, link labels and every native template container.
        let original = serde_json::to_string(ir.as_document()).map_err(|e| e.to_string())?;
        let template = serde_json::to_string(roots).map_err(|e| e.to_string())?;
        let mut visible = String::new();
        collect_visible(
            &serde_json::to_value(ir.as_document()).map_err(|e| e.to_string())?,
            &mut visible,
        );
        let mut generation = 0_u64;
        let prefix = loop {
            let candidate = format!("MD2HWP_NOTE_{generation}_");
            if !original.contains(&candidate)
                && !template.contains(&candidate)
                && !visible.contains(&candidate)
            {
                break candidate;
            }
            generation = generation
                .checked_add(1)
                .ok_or("Footnote marker namespace exhausted")?;
        };
        let mut plan = Self {
            prefix,
            notes: BTreeMap::new(),
        };
        let mut document = ir.as_document().clone();
        for block in &mut document.blocks {
            match block {
                Block::Paragraph { inlines } | Block::Heading { inlines, .. } => {
                    plan.mark(inlines)?
                }
                Block::List { items, .. } => plan.mark_items(items)?,
                Block::Table { header, rows, .. } => {
                    for cell in header.iter_mut().chain(rows.iter_mut().flatten()) {
                        plan.mark(cell)?;
                    }
                }
                _ => {}
            }
        }
        let marker_bytes = plan.notes.keys().try_fold(0_usize, |sum, marker| {
            sum.checked_add(marker.len())
                .ok_or("Footnote marker text limit overflow")
        })?;
        let mut limits = ValidationLimits::default();
        // The original document has already paid for all note blocks, runs and
        // text. Only private marker expansion may exceed its original text cap.
        if marker_bytes > limits.max_text_bytes {
            return Err("Footnote marker expansion exceeds the adapter text budget".into());
        }
        limits.max_text_bytes = limits
            .max_text_bytes
            .checked_add(marker_bytes)
            .ok_or("Footnote marker text limit overflow")?;
        let prepared = validate(document, &limits).map_err(|e| e.to_string())?;
        Ok((plan, prepared))
    }

    fn mark_items(&mut self, items: &mut [ListItem]) -> Result<(), String> {
        for item in items {
            for block in &mut item.blocks {
                match block {
                    ListItemBlock::Paragraph { inlines } => self.mark(inlines)?,
                    ListItemBlock::List { items, .. } => self.mark_items(items)?,
                }
            }
        }
        Ok(())
    }

    fn mark(&mut self, inlines: &mut [Inline]) -> Result<(), String> {
        for inline in inlines {
            match inline {
                Inline::Footnote { blocks } => {
                    for FootnoteBlock::Paragraph { inlines } in blocks.iter() {
                        crate::template::check_inlines(inlines)?;
                    }
                    let marker = format!("{}{}_END", self.prefix, self.notes.len());
                    self.notes.insert(marker.clone(), blocks.clone());
                    *inline = Inline::Text { value: marker };
                }
                Inline::Strong { inlines }
                | Inline::Emph { inlines }
                | Inline::Link { inlines, .. } => self.mark(inlines)?,
                _ => {}
            }
        }
        Ok(())
    }

    /// Width measurement sees the real note reference; marker strings never
    /// contribute their arbitrary ASCII width to native column allocation.
    pub(crate) fn measurement_inlines(&self, inlines: &[Inline]) -> Vec<Inline> {
        inlines
            .iter()
            .map(|inline| match inline {
                Inline::Text { value } if self.notes.contains_key(value) => Inline::Footnote {
                    blocks: self.notes[value].clone(),
                },
                Inline::Strong { inlines } => Inline::Strong {
                    inlines: self.measurement_inlines(inlines),
                },
                Inline::Emph { inlines } => Inline::Emph {
                    inlines: self.measurement_inlines(inlines),
                },
                Inline::Link {
                    target,
                    title,
                    inlines,
                } => Inline::Link {
                    target: target.clone(),
                    title: title.clone(),
                    inlines: self.measurement_inlines(inlines),
                },
                _ => inline.clone(),
            })
            .collect()
    }

    pub(crate) fn apply(
        &self,
        paragraphs: &mut [Paragraph],
        sample: &Sample,
        info: &mut DocInfo,
        shape: &FootnoteShape,
        identity: &mut IdentityAllocator,
    ) -> Result<BTreeSet<u32>, String> {
        let mut generated = BTreeSet::new();
        let mut found = BTreeSet::new();
        // Validate contexts before making any edits. The field names belong to
        // the pinned native model, not manuscript syntax or user-authored text.
        let model = serde_json::to_value(&*paragraphs).map_err(|e| e.to_string())?;
        self.check_contexts(&model)?;
        for paragraph in &mut *paragraphs {
            self.attach_paragraph(
                paragraph,
                sample,
                info,
                shape,
                identity,
                &mut generated,
                &mut found,
            )?;
        }
        if found.len() != self.notes.len() {
            return Err(
                "A generated footnote reference was lost during manuscript composition".into(),
            );
        }
        let final_model = serde_json::to_value(&*paragraphs).map_err(|e| e.to_string())?;
        if contains_prefix(&final_model, &self.prefix) {
            return Err(
                "An unresolved private footnote marker remains after native insertion".into(),
            );
        }
        Ok(generated)
    }

    fn check_contexts(&self, value: &serde_json::Value) -> Result<(), String> {
        match value {
            serde_json::Value::Object(fields) => {
                for (name, value) in fields {
                    if matches!(
                        name.as_str(),
                        "caption"
                            | "Header"
                            | "Footer"
                            | "Footnote"
                            | "Endnote"
                            | "master_pages"
                            | "HiddenComment"
                            | "memo_paragraphs"
                    ) && contains_prefix(value, &self.prefix)
                    {
                        return Err("Footnotes cannot be placed in native captions, headers, footers, master pages, comments or other notes".into());
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

    #[allow(clippy::too_many_arguments)]
    fn attach_paragraph(
        &self,
        paragraph: &mut Paragraph,
        sample: &Sample,
        info: &mut DocInfo,
        shape: &FootnoteShape,
        identity: &mut IdentityAllocator,
        generated: &mut BTreeSet<u32>,
        found: &mut BTreeSet<String>,
    ) -> Result<(), String> {
        while let Some(start) = paragraph.text.find(&self.prefix) {
            let (marker, blocks) = self
                .notes
                .iter()
                .find(|(marker, _)| paragraph.text[start..].starts_with(marker.as_str()))
                .ok_or("Unrecognized private footnote marker in generated content")?;
            let note = sample.render(blocks, info, shape)?;
            let mut detached = Paragraph {
                controls: vec![Control::Footnote(Box::new(note))],
                ..Default::default()
            };
            crate::model_tools::refresh_block_identities(
                std::slice::from_mut(&mut detached),
                identity,
            )?;
            let Control::Footnote(note) = detached.controls.remove(0) else {
                unreachable!()
            };
            if !generated.insert(note.instance_id) {
                return Err("Generated footnotes share a native instance identity".into());
            }
            crate::note_insert::insert(paragraph, marker, *note)?;
            found.insert(marker.clone());
        }
        crate::model_tools::map_nested_containers(paragraph, &mut |paragraphs| {
            for paragraph in paragraphs {
                self.attach_paragraph(paragraph, sample, info, shape, identity, generated, found)?;
            }
            Ok(())
        })
    }
}

fn contains_prefix(value: &serde_json::Value, prefix: &str) -> bool {
    match value {
        serde_json::Value::String(text) => text.contains(prefix),
        serde_json::Value::Array(values) => values.iter().any(|v| contains_prefix(v, prefix)),
        serde_json::Value::Object(values) => values.values().any(|v| contains_prefix(v, prefix)),
        _ => false,
    }
}

// Rich spans concatenate without JSON delimiters when rendered. Scan that
// projection too, so a literal prefix split across emphasis or links is reserved.
fn collect_visible(value: &serde_json::Value, out: &mut String) {
    match value {
        serde_json::Value::String(text) => out.push_str(text),
        serde_json::Value::Array(values) => {
            for value in values {
                collect_visible(value, out);
            }
        }
        serde_json::Value::Object(fields) => {
            match fields.get("type").and_then(serde_json::Value::as_str) {
                Some("text") => {
                    if let Some(value) = fields.get("value") {
                        collect_visible(value, out);
                    }
                }
                Some("space") => out.push(' '),
                Some("line_break") => out.push('\n'),
                Some("strong" | "emph" | "link") => {
                    if let Some(value) = fields.get("inlines") {
                        collect_visible(value, out);
                    }
                }
                _ => {
                    for (name, value) in fields {
                        if name != "type" {
                            collect_visible(value, out);
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use md2hwp_core::ir::{Document, IR_VERSION, Metadata, MetadataValue, SCHEMA_NAME};
    use rhwp::model::{
        style::{CharShape, ParaShape},
        table::{Cell, Table},
    };

    fn note() -> Inline {
        Inline::Footnote {
            blocks: vec![FootnoteBlock::Paragraph {
                inlines: vec![
                    Inline::Text {
                        value: "주석".into(),
                    },
                    Inline::Space,
                    Inline::Text {
                        value: "본문".into(),
                    },
                ],
            }],
        }
    }
    fn ir(inlines: Vec<Inline>) -> ValidatedDocument {
        validate(
            Document {
                schema: SCHEMA_NAME.into(),
                ir_version: IR_VERSION.into(),
                metadata: Metadata::default(),
                blocks: vec![Block::Paragraph { inlines }],
            },
            &ValidationLimits::default(),
        )
        .unwrap()
    }
    fn paragraph(text: &str) -> Paragraph {
        let mut p = Paragraph::new_empty();
        p.set_single_char_shape(0);
        p.insert_text_at(0, text);
        p
    }
    fn sample(info: &DocInfo) -> Sample {
        Sample::read(
            &[
                paragraph("{{md2hwp:begin:template}}"),
                paragraph("{{md2hwp:footnote}}"),
                paragraph("{{md2hwp:end:template}}"),
            ],
            info,
        )
        .unwrap()
    }
    #[test]
    fn marker_namespace_scans_metadata_template_and_original_literals() {
        let mut original = ir(vec![
            Inline::Text {
                value: "MD2HWP_NOTE_0_0_END".into(),
            },
            Inline::Strong {
                inlines: vec![Inline::Text {
                    value: "MD2HWP_NOTE_".into(),
                }],
            },
            Inline::Emph {
                inlines: vec![Inline::Text { value: "3_".into() }],
            },
            note(),
        ])
        .into_document();
        original
            .metadata
            .0
            .insert("title".into(), MetadataValue::Text("MD2HWP_NOTE_1_".into()));
        let original = validate(original, &ValidationLimits::default()).unwrap();
        let (plan, prepared) = Plan::prepare(&original, &[paragraph("MD2HWP_NOTE_2_")]).unwrap();
        assert_eq!(plan.prefix, "MD2HWP_NOTE_4_");
        let Block::Paragraph { inlines } = &prepared.as_document().blocks[0] else {
            panic!()
        };
        assert_eq!(
            plan.measurement_inlines(inlines),
            match &original.as_document().blocks[0] {
                Block::Paragraph { inlines } => inlines.clone(),
                _ => unreachable!(),
            }
        );
        assert_eq!(
            inlines[0],
            Inline::Text {
                value: "MD2HWP_NOTE_0_0_END".into()
            }
        );
    }
    #[test]
    fn repeated_native_occurrences_get_fresh_note_ids_without_changing_table_identity() {
        let (plan, _) = Plan::prepare(&ir(vec![note()]), &[]).unwrap();
        let marker = plan.notes.keys().next().unwrap();
        let mut info = DocInfo {
            char_shapes: vec![CharShape::default()],
            para_shapes: vec![ParaShape::default()],
            ..Default::default()
        };
        let sample = sample(&info);
        let mut table = Table {
            cells: vec![Cell {
                paragraphs: vec![paragraph(&format!("{marker} / {marker}"))],
                ..Default::default()
            }],
            ..Default::default()
        };
        table.common.instance_id = 500;
        let mut output = vec![Paragraph {
            controls: vec![Control::Table(Box::new(table))],
            ..Default::default()
        }];
        let mut document = rhwp::DocumentCore::new_empty().document().clone();
        document.sections.push(Default::default());
        document.sections[0].paragraphs = output.clone();
        let mut identities = crate::model_tools::identity_seed(&document).unwrap();
        let ids = plan
            .apply(
                &mut output,
                &sample,
                &mut info,
                &FootnoteShape::default(),
                &mut identities,
            )
            .unwrap();
        assert_eq!(ids.len(), 2);
        let Control::Table(table) = &output[0].controls[0] else {
            panic!()
        };
        assert_eq!(table.common.instance_id, 500);
        assert_eq!(table.cells[0].paragraphs[0].text, " / ");
        assert_eq!(table.cells[0].paragraphs[0].controls.len(), 2);
    }
    #[test]
    fn rejects_references_copied_to_forbidden_native_contexts_and_missing_markers() {
        use rhwp::model::{header_footer::Header, shape::Caption};
        let (plan, _) = Plan::prepare(&ir(vec![note()]), &[]).unwrap();
        let marker = plan.notes.keys().next().unwrap();
        let mut info = DocInfo {
            char_shapes: vec![CharShape::default()],
            para_shapes: vec![ParaShape::default()],
            ..Default::default()
        };
        let sample = sample(&info);
        let document = rhwp::DocumentCore::new_empty().document().clone();
        let mut identities = crate::model_tools::identity_seed(&document).unwrap();
        for control in [
            Control::Header(Box::new(Header {
                paragraphs: vec![paragraph(marker)],
                ..Default::default()
            })),
            Control::Table(Box::new(Table {
                caption: Some(Caption {
                    paragraphs: vec![paragraph(marker)],
                    ..Default::default()
                }),
                ..Default::default()
            })),
        ] {
            let mut output = vec![Paragraph {
                controls: vec![control],
                ..Default::default()
            }];
            let before = serde_json::to_value(&output).unwrap();
            assert!(
                plan.apply(
                    &mut output,
                    &sample,
                    &mut info,
                    &FootnoteShape::default(),
                    &mut identities
                )
                .is_err()
            );
            assert_eq!(serde_json::to_value(output).unwrap(), before);
        }
        assert!(
            plan.apply(
                &mut [paragraph("lost")],
                &sample,
                &mut info,
                &FootnoteShape::default(),
                &mut identities
            )
            .is_err()
        );
    }
}
