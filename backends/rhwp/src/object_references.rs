// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Euisoon Ahn
//! Deferred object fields: native targets and counters are known after composition.

use crate::model_tools::IdentityAllocator;
use md2hwp_core::{
    ValidatedDocument, ValidationLimits,
    ir::{Block, CrossReferenceKind, FootnoteBlock, Inline, ListItem, ListItemBlock},
    validate,
};
use rhwp::model::{
    control::{AutoNumberType, Control},
    document::Document,
    paragraph::Paragraph,
};
use std::collections::{BTreeMap, BTreeSet};

struct Reference {
    target: String,
    required: bool,
}
struct Target {
    kind: &'static str,
    chapter: Option<u32>,
    instance: Option<u32>,
    number: Option<u16>,
}
pub(crate) struct Plan {
    prefix: String,
    samples: BTreeMap<&'static str, String>,
    prototypes: BTreeMap<&'static str, Paragraph>,
    references: BTreeMap<String, Reference>,
    targets: BTreeMap<String, Target>,
}

impl Plan {
    pub(crate) fn has_references(&self) -> bool {
        !self.references.is_empty()
    }
    pub(crate) fn prepare(
        ir: &ValidatedDocument,
        roots: &[Paragraph],
    ) -> Result<(Self, ValidatedDocument), String> {
        let mut samples = BTreeMap::new();
        let mut prototypes = BTreeMap::new();
        let find = |token: &str| -> Result<usize, String> {
            let hits: Vec<_> = roots
                .iter()
                .enumerate()
                .filter(|(_, p)| p.text == crate::template::tag(token))
                .map(|(i, _)| i)
                .collect();
            match hits.as_slice() {
                [i] => Ok(*i),
                _ => Err(format!("Expected one root declaration {token}")),
            }
        };
        let begin = find("begin:template")?;
        let end = find("end:template")?;
        for kind in ["figure", "table"] {
            let role = format!("ref.{kind}.number");
            let first = find(&format!("begin:{role}"))?;
            let last = find(&format!("end:{role}"))?;
            if first <= begin || last >= end || last != first + 2 {
                return Err(format!(
                    "{role} requires one plain root paragraph inside template definitions"
                ));
            }
            for p in &roots[first..=last] {
                crate::template::require_plain(p)?;
                if p.text.chars().any(char::is_control)
                    || p.column_type != rhwp::model::paragraph::ColumnBreakType::None
                    || p.raw_break_type & 0xf != 0
                {
                    return Err(format!(
                        "{role} must be a single-line plain paragraph without breaks"
                    ));
                }
            }
            let text = roots[first + 1].text.clone();
            let slot = crate::template::tag(&format!("slot:{role}"));
            if text.matches(&slot).count() != 1 {
                return Err(format!("{role} requires exactly one slot:{role}"));
            }
            let remainder = text
                .replace(&slot, "")
                .replace(&crate::template::tag("num:heading1"), "");
            if remainder.contains("{{md2hwp:") {
                return Err(format!("Unsupported tag in {role}"));
            }
            samples.insert(kind, text);
            prototypes.insert(kind, roots[first + 1].clone());
        }
        let mut literal = serde_json::to_string(ir.as_document()).map_err(|e| e.to_string())?;
        literal.push_str(&serde_json::to_string(roots).map_err(|e| e.to_string())?);
        fn visible(value: &serde_json::Value, out: &mut String) {
            match value {
                serde_json::Value::String(s) => out.push_str(s),
                serde_json::Value::Array(a) => {
                    for v in a {
                        visible(v, out)
                    }
                }
                serde_json::Value::Object(o) => {
                    for v in o.values() {
                        visible(v, out)
                    }
                }
                _ => {}
            }
        }
        visible(
            &serde_json::to_value(ir.as_document()).map_err(|e| e.to_string())?,
            &mut literal,
        );
        visible(
            &serde_json::to_value(roots).map_err(|e| e.to_string())?,
            &mut literal,
        );
        let mut generation = 0u64;
        let prefix = loop {
            let s = format!("MD2HWP_XREF_{generation}_");
            if !literal.contains(&s) {
                break s;
            }
            generation = generation
                .checked_add(1)
                .ok_or("Reference namespace overflow")?;
        };
        let mut plan = Self {
            prefix,
            samples,
            prototypes,
            references: BTreeMap::new(),
            targets: BTreeMap::new(),
        };
        let headings = crate::heading_plan::HeadingPlan::new(ir)?;
        let mut chapter = None;
        for (operation, block) in ir.as_document().blocks.iter().enumerate() {
            match block {
                Block::Heading { level: 1, .. } => {
                    chapter = Some(
                        headings
                            .context(operation)
                            .ok_or("Missing heading context")?
                            .numbers[0],
                    )
                }
                Block::Figure { id: Some(id), .. } => {
                    plan.targets.insert(
                        id.clone(),
                        Target {
                            kind: "figure",
                            chapter,
                            instance: None,
                            number: None,
                        },
                    );
                }
                Block::Table { id: Some(id), .. } => {
                    plan.targets.insert(
                        id.clone(),
                        Target {
                            kind: "table",
                            chapter,
                            instance: None,
                            number: None,
                        },
                    );
                }
                _ => {}
            }
        }
        let mut document = ir.as_document().clone();
        walk_document(&mut document, &mut |inlines, required| {
            plan.mark(inlines, required)
        })?;
        // A referenced table must own a native numbered caption.
        for reference in plan.references.values() {
            if ir.as_document().blocks.iter().any(
                |b| matches!(b,Block::Table{id:Some(id),caption:None,..} if id==&reference.target),
            ) {
                return Err(
                    "Table number references require a native numbered table caption".into(),
                );
            }
        }
        let expansion = plan.references.keys().try_fold(0usize, |sum, m| {
            sum.checked_add(m.len())
                .ok_or("Reference marker budget overflow")
        })?;
        let mut limits = ValidationLimits::default();
        if expansion > limits.max_text_bytes {
            return Err("Reference marker expansion exceeds adapter budget".into());
        }
        limits.max_text_bytes = limits
            .max_text_bytes
            .checked_add(expansion)
            .ok_or("Reference text budget overflow")?;
        let prepared = validate(document, &limits).map_err(|e| e.to_string())?;
        Ok((plan, prepared))
    }

    fn mark(&mut self, inlines: &mut [Inline], required: bool) -> Result<(), String> {
        for inline in inlines {
            match inline {
                Inline::CrossReference { kind, target } => {
                    let name = match kind {
                        CrossReferenceKind::FigureNumber => "figure",
                        CrossReferenceKind::TableNumber => "table",
                        CrossReferenceKind::HeadingNumber => {
                            return Err("Unresolved heading reference".into());
                        }
                    };
                    let object = self
                        .targets
                        .get(target)
                        .ok_or("Missing object reference target")?;
                    if object.kind != name {
                        return Err("Object reference target kind mismatch".into());
                    }
                    if object.chapter.is_none()
                        && self.samples[name].contains(&crate::template::tag("num:heading1"))
                    {
                        return Err("num:heading1 in object reference requires the target to follow heading1".into());
                    }
                    let marker = format!("{}{}_END", self.prefix, self.references.len());
                    self.references.insert(
                        marker.clone(),
                        Reference {
                            target: target.clone(),
                            required,
                        },
                    );
                    *inline = Inline::Text { value: marker };
                }
                Inline::Strong { inlines }
                | Inline::Emph { inlines }
                | Inline::Link { inlines, .. } => self.mark(inlines, required)?,
                Inline::Footnote { blocks } => {
                    for FootnoteBlock::Paragraph { inlines } in blocks {
                        self.mark(inlines, required)?
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub(crate) fn bind(&mut self, id: &str, instance: u32) -> Result<(), String> {
        let object = self
            .targets
            .get_mut(id)
            .ok_or("Unknown generated object ID")?;
        if object.instance.replace(instance).is_some() {
            return Err("Duplicate generated object target".into());
        }
        Ok(())
    }

    pub(crate) fn resolve(&mut self, document: &Document) -> Result<(), String> {
        if self.references.is_empty() {
            return Ok(());
        }
        let wanted: BTreeSet<_> = self
            .references
            .values()
            .map(|reference| {
                self.targets[&reference.target]
                    .instance
                    .ok_or("Referenced native target was not generated")
            })
            .collect::<Result<_, _>>()?;
        fn numbers(
            p: &Paragraph,
            wanted: &BTreeSet<u32>,
            values: &mut BTreeMap<u32, (&'static str, Option<u16>)>,
        ) -> Result<(), String> {
            for c in &p.controls {
                let found = match c {
                    Control::Picture(p) => Some((
                        p.common.instance_id,
                        "figure",
                        p.caption.as_ref(),
                        AutoNumberType::Picture,
                    )),
                    Control::Table(t) => Some((
                        t.common.instance_id,
                        "table",
                        t.caption.as_ref(),
                        AutoNumberType::Table,
                    )),
                    _ => None,
                };
                if let Some((id, kind, caption, number_kind)) = found
                    && wanted.contains(&id)
                {
                    let counters: Vec<_> = caption
                        .iter()
                        .flat_map(|c| &c.paragraphs)
                        .flat_map(|p| &p.controls)
                        .filter_map(|c| match c {
                            Control::AutoNumber(n) if n.number_type == number_kind => {
                                Some(n.number)
                            }
                            _ => None,
                        })
                        .collect();
                    let number = match counters.as_slice() {
                        [n] => Some(*n),
                        [] => None,
                        _ => return Err("Ambiguous native caption counter".into()),
                    };
                    if values.insert(id, (kind, number)).is_some() {
                        return Err("Duplicate native object identity".into());
                    }
                }
            }
            crate::model_tools::map_nested_containers(&mut p.clone(), &mut |children| {
                for p in children {
                    numbers(p, wanted, values)?;
                }
                Ok(())
            })
        }
        let mut values = BTreeMap::new();
        for section in &document.sections {
            for p in &section.paragraphs {
                numbers(p, &wanted, &mut values)?;
            }
        }
        for reference in self.references.values() {
            let object = self.targets.get_mut(&reference.target).unwrap();
            let instance = object
                .instance
                .ok_or("Referenced native target was not generated")?;
            let (kind, number) = values
                .get(&instance)
                .ok_or("Referenced native target was lost")?;
            if *kind != object.kind {
                return Err("Native target kind changed".into());
            }
            object.number =
                Some(number.filter(|n| *n > 0).ok_or(
                    "Object number reference requires one positive native caption counter",
                )?);
        }
        Ok(())
    }

    fn fragments(&self, marker: &str, native_marker: Option<&str>) -> Result<String, String> {
        let reference = &self.references[marker];
        let object = &self.targets[&reference.target];
        let slot = crate::template::tag(&format!("slot:ref.{}.number", object.kind));
        let chapter = object.chapter.map(|n| n.to_string()).unwrap_or_default();
        let number = object
            .number
            .ok_or("Reference counter not resolved")?
            .to_string();
        Ok(self.samples[object.kind]
            .replace(&crate::template::tag("num:heading1"), &chapter)
            .replace(&slot, native_marker.unwrap_or(&number)))
    }

    pub(crate) fn measurement_inlines(&self, inlines: &[Inline]) -> Result<Vec<Inline>, String> {
        let mut result = Vec::new();
        for inline in inlines {
            match inline {
                Inline::Text { value } if value.contains(&self.prefix) => {
                    let reference = self
                        .references
                        .get(value)
                        .ok_or("Unrecognized reference marker in measurement input")?;
                    result.push(Inline::CrossReference {
                        kind: if self.targets[&reference.target].kind == "figure" {
                            CrossReferenceKind::FigureNumber
                        } else {
                            CrossReferenceKind::TableNumber
                        },
                        target: reference.target.clone(),
                    });
                }
                Inline::Strong { inlines } => result.push(Inline::Strong {
                    inlines: self.measurement_inlines(inlines)?,
                }),
                Inline::Emph { inlines } => result.push(Inline::Emph {
                    inlines: self.measurement_inlines(inlines)?,
                }),
                Inline::Link {
                    inlines,
                    target,
                    title,
                } => result.push(Inline::Link {
                    inlines: self.measurement_inlines(inlines)?,
                    target: target.clone(),
                    title: title.clone(),
                }),
                _ => result.push(inline.clone()),
            }
        }
        Ok(result)
    }

    /// Match the existing backend's conservative, indivisible native field
    /// reservation using each template fragment's own character format.
    pub(crate) fn reserved_fragments(
        &self,
        kind: &CrossReferenceKind,
    ) -> Result<Vec<(String, u32)>, String> {
        let kind = match kind {
            CrossReferenceKind::FigureNumber => "figure",
            CrossReferenceKind::TableNumber => "table",
            _ => return Err("Heading references must resolve before width measurement".into()),
        };
        let mut sample = self.prototypes[kind].clone();
        let chapter = crate::template::tag("num:heading1");
        while sample.text.contains(&chapter) {
            crate::template::replace(&mut sample, &chapter, "999")?;
        }
        crate::template::replace(
            &mut sample,
            &crate::template::tag(&format!("slot:ref.{kind}.number")),
            "999",
        )?;
        let mut fragments: Vec<(String, u32)> = Vec::new();
        for (index, ch) in sample.text.chars().enumerate() {
            let shape = sample
                .char_shape_id_at(index)
                .ok_or("Reference prototype has no character format")?;
            if let Some((text, id)) = fragments.last_mut()
                && *id == shape
            {
                text.push(ch);
            } else {
                fragments.push((ch.to_string(), shape));
            }
        }
        Ok(fragments)
    }

    pub(crate) fn apply(
        &self,
        paragraphs: &mut [Paragraph],
        identity: &mut IdentityAllocator,
    ) -> Result<BTreeSet<u32>, String> {
        fn visit(
            plan: &Plan,
            p: &mut Paragraph,
            identity: &mut IdentityAllocator,
            found: &mut BTreeSet<String>,
            fields: &mut BTreeSet<u32>,
            expanded_bytes: &mut usize,
        ) -> Result<(), String> {
            while let Some(start) = p.text.find(&plan.prefix) {
                let marker = plan
                    .references
                    .keys()
                    .find(|m| p.text[start..].starts_with(m.as_str()))
                    .ok_or("Unrecognized private reference marker")?
                    .clone();
                let target = &plan.targets[&plan.references[&marker].target];
                // Expand affixes before attaching the native field; all inherit source format.
                let expanded = plan.fragments(&marker, Some(&marker))?;
                *expanded_bytes = expanded_bytes
                    .checked_add(expanded.len())
                    .ok_or("Reference expansion text budget overflow")?;
                if *expanded_bytes > ValidationLimits::default().max_text_bytes {
                    return Err("Generated reference text exceeds the adapter text budget".into());
                }
                crate::reference_insert::expand(p, &marker, &expanded)?;
                let field = crate::reference_insert::insert(
                    p,
                    &marker,
                    &target.number.unwrap().to_string(),
                    target.instance.unwrap(),
                    if target.kind == "figure" {
                        "figure_number"
                    } else {
                        "table_number"
                    },
                    identity,
                )
                .map_err(|error| {
                    format!(
                        "{error} (reference target '{}')",
                        plan.references[&marker].target
                    )
                })?;
                if !fields.insert(field) {
                    return Err("Duplicate generated reference field identity".into());
                }
                found.insert(marker);
            }
            crate::model_tools::map_nested_containers(p, &mut |children| {
                for p in children {
                    visit(plan, p, identity, found, fields, expanded_bytes)?;
                }
                Ok(())
            })
        }
        let mut found = BTreeSet::new();
        let mut fields = BTreeSet::new();
        let mut expanded_bytes = 0;
        for p in paragraphs {
            visit(
                self,
                p,
                identity,
                &mut found,
                &mut fields,
                &mut expanded_bytes,
            )?;
        }
        if self
            .references
            .iter()
            .any(|(m, r)| r.required && !found.contains(m))
        {
            return Err("A generated reference was lost during native composition".into());
        }
        Ok(fields)
    }
}

fn walk_document(
    document: &mut md2hwp_core::ir::Document,
    visit: &mut impl FnMut(&mut [Inline], bool) -> Result<(), String>,
) -> Result<(), String> {
    fn items(
        values: &mut [ListItem],
        visit: &mut impl FnMut(&mut [Inline], bool) -> Result<(), String>,
    ) -> Result<(), String> {
        for item in values {
            for block in &mut item.blocks {
                match block {
                    ListItemBlock::Paragraph { inlines } => visit(inlines, true)?,
                    ListItemBlock::List {
                        items: children, ..
                    } => items(children, visit)?,
                }
            }
        }
        Ok(())
    }
    for block in &mut document.blocks {
        match block {
            Block::Paragraph { inlines } | Block::Heading { inlines, .. } => visit(inlines, true)?,
            Block::List {
                items: children, ..
            } => items(children, visit)?,
            Block::VerbatimBlock { source, .. } => {
                for p in source.iter_mut().flatten() {
                    visit(&mut p.inlines, true)?
                }
            }
            Block::Figure {
                image,
                caption,
                source,
                ..
            } => {
                visit(&mut image.alt, false)?;
                visit(caption, true)?;
                for p in source.iter_mut().flatten() {
                    visit(&mut p.inlines, true)?
                }
            }
            Block::Table {
                header,
                rows,
                caption,
                source,
                ..
            } => {
                for cell in header.iter_mut().chain(rows.iter_mut().flatten()) {
                    visit(cell, true)?;
                }
                if let Some(caption) = caption {
                    visit(caption, true)?;
                }
                for p in source.iter_mut().flatten() {
                    visit(&mut p.inlines, true)?
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use md2hwp_core::ir::{
        IR_VERSION, ImageRef, Metadata, MetadataValue, SCHEMA_NAME, TableAlignment,
    };
    use rhwp::model::{control::AutoNumber, image::Picture, shape::Caption, table::Table};

    fn paragraph(text: &str) -> Paragraph {
        let mut p = Paragraph::new_empty();
        p.set_single_char_shape(0);
        p.insert_text_at(0, text);
        p
    }

    fn roots(chapter: bool) -> Vec<Paragraph> {
        let mut roots = vec![paragraph("{{md2hwp:begin:template}}")];
        for (kind, label) in [("figure", "그림"), ("table", "표")] {
            roots.push(paragraph(&format!(
                "{{{{md2hwp:begin:ref.{kind}.number}}}}"
            )));
            roots.push(paragraph(&format!(
                "[{label} {}{{{{md2hwp:slot:ref.{kind}.number}}}}번]",
                if chapter {
                    "{{md2hwp:num:heading1}}-"
                } else {
                    ""
                }
            )));
            roots.push(paragraph(&format!("{{{{md2hwp:end:ref.{kind}.number}}}}")));
        }
        roots.push(paragraph("{{md2hwp:end:template}}"));
        roots
    }

    fn text(value: &str) -> Inline {
        Inline::Text {
            value: value.into(),
        }
    }
    fn reference(kind: CrossReferenceKind, target: &str) -> Inline {
        Inline::CrossReference {
            kind,
            target: target.into(),
        }
    }
    fn heading() -> Block {
        Block::Heading {
            id: None,
            level: 1,
            inlines: vec![text("제목")],
        }
    }
    fn figure(id: &str, alt: Vec<Inline>) -> Block {
        Block::Figure {
            id: Some(id.into()),
            image: ImageRef {
                path: "image.png".into(),
                alt,
                title: None,
            },
            caption: vec![text("설명")],
            source: None,
        }
    }
    fn table(caption: bool) -> Block {
        Block::Table {
            id: Some("표".into()),
            columns: vec![TableAlignment::Default],
            header: vec![vec![text("머리")]],
            rows: vec![],
            caption: caption.then(|| vec![text("설명")]),
            source: None,
        }
    }
    fn ir(blocks: Vec<Block>) -> ValidatedDocument {
        let mut metadata = Metadata::default();
        metadata.0.insert(
            "md2hwp-heading1-start".into(),
            MetadataValue::Text("7".into()),
        );
        validate(
            md2hwp_core::Document {
                schema: SCHEMA_NAME.into(),
                ir_version: IR_VERSION.into(),
                metadata,
                blocks,
            },
            &ValidationLimits::default(),
        )
        .unwrap()
    }
    fn caption(kind: AutoNumberType, number: u16) -> Caption {
        let mut p = paragraph("설명");
        p.controls.push(Control::AutoNumber(AutoNumber {
            number_type: kind,
            number,
            assigned_number: number,
            ..Default::default()
        }));
        Caption {
            paragraphs: vec![p],
            ..Default::default()
        }
    }
    fn native_document() -> Document {
        let mut core = rhwp::DocumentCore::new_empty();
        core.create_blank_document_native().unwrap();
        core.document().clone()
    }
    fn native_figure(id: u32, number: u16) -> Control {
        let mut p = Picture::default();
        p.common.instance_id = id;
        p.caption = Some(caption(AutoNumberType::Picture, number));
        Control::Picture(Box::new(p))
    }
    fn native_table(id: u32, number: u16) -> Control {
        let mut t = Table::default();
        t.common.instance_id = id;
        t.caption = Some(caption(AutoNumberType::Table, number));
        Control::Table(Box::new(t))
    }

    #[test]
    fn forward_references_use_target_chapters_and_resolved_native_counters() {
        let input = ir(vec![
            Block::Paragraph {
                inlines: vec![
                    reference(CrossReferenceKind::FigureNumber, "다음"),
                    Inline::Space,
                    reference(CrossReferenceKind::TableNumber, "표"),
                    Inline::Space,
                    reference(CrossReferenceKind::FigureNumber, "이전"),
                ],
            },
            heading(),
            figure("이전", vec![]),
            table(true),
            heading(),
            figure("다음", vec![]),
        ]);
        let (mut plan, prepared) = Plan::prepare(&input, &roots(true)).unwrap();
        assert_eq!(plan.targets["이전"].chapter, Some(7));
        assert_eq!(plan.targets["다음"].chapter, Some(8));
        plan.bind("이전", 101).unwrap();
        plan.bind("다음", 102).unwrap();
        plan.bind("표", 103).unwrap();
        let mut native = native_document();
        let mut anchor = paragraph("");
        anchor.controls = vec![
            native_figure(101, 12),
            native_table(103, 35),
            native_figure(102, 1),
        ];
        native.sections[0].paragraphs = vec![anchor];
        plan.resolve(&native).unwrap();
        let Block::Paragraph { inlines } = &prepared.as_document().blocks[0] else {
            panic!()
        };
        let measured = plan.measurement_inlines(inlines).unwrap();
        let expected = vec![
            reference(CrossReferenceKind::FigureNumber, "다음"),
            Inline::Space,
            reference(CrossReferenceKind::TableNumber, "표"),
            Inline::Space,
            reference(CrossReferenceKind::FigureNumber, "이전"),
        ];
        assert_eq!(measured, expected);
        let source = inlines
            .iter()
            .map(|i| match i {
                Inline::Text { value } => value.as_str(),
                Inline::Space => " ",
                _ => panic!(),
            })
            .collect::<String>();
        let mut output = vec![paragraph(&source)];
        let mut allocator = crate::model_tools::identity_seed(&native).unwrap();
        assert_eq!(plan.apply(&mut output, &mut allocator).unwrap().len(), 3);
        assert_eq!(output[0].text, "[그림 8-1번] [표 7-35번] [그림 7-12번]");
        let commands = output[0]
            .controls
            .iter()
            .filter_map(|c| match c {
                Control::Field(f) => Some(f.command.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            commands,
            ["?#102;1;1;0;0", "?#103;0;1;0;0", "?#101;1;1;0;0"]
        );
    }

    #[test]
    fn alt_only_reference_resolves_without_requiring_a_rendered_field() {
        let input = ir(vec![
            heading(),
            figure(
                "대상",
                vec![reference(CrossReferenceKind::FigureNumber, "대상")],
            ),
        ]);
        let (mut plan, _) = Plan::prepare(&input, &roots(true)).unwrap();
        assert!(
            plan.references
                .values()
                .all(|reference| !reference.required)
        );
        plan.bind("대상", 101).unwrap();
        let mut native = native_document();
        let mut anchor = paragraph("");
        anchor.controls = vec![native_figure(101, 4)];
        native.sections[0].paragraphs = vec![anchor];
        plan.resolve(&native).unwrap();
        let mut allocator = crate::model_tools::identity_seed(&native).unwrap();
        assert!(
            plan.apply(&mut [paragraph("이미지")], &mut allocator)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn literal_marker_prefixes_and_template_tag_text_remain_literal() {
        let input = ir(vec![
            heading(),
            Block::Paragraph {
                inlines: vec![
                    text("MD2HWP_XREF_0_"),
                    text("MD2HWP_XREF_1_"),
                    text("{{md2hwp:num:heading1}}"),
                    reference(CrossReferenceKind::FigureNumber, "대상"),
                ],
            },
            figure("대상", vec![]),
        ]);
        let mut template = roots(true);
        template.push(paragraph("MD2HWP_XREF_2_"));
        let (plan, prepared) = Plan::prepare(&input, &template).unwrap();
        assert_eq!(plan.prefix, "MD2HWP_XREF_3_");
        let Block::Paragraph { inlines } = &prepared.as_document().blocks[1] else {
            panic!()
        };
        assert_eq!(
            &inlines[..3],
            &[
                text("MD2HWP_XREF_0_"),
                text("MD2HWP_XREF_1_"),
                text("{{md2hwp:num:heading1}}")
            ]
        );
    }

    #[test]
    fn prototypes_are_mandatory_and_captionless_tables_fail_only_when_referenced() {
        let input = ir(vec![heading(), table(false)]);
        assert!(Plan::prepare(&input, &roots(true)).is_ok());
        let mut missing = roots(true);
        missing.drain(4..7);
        assert!(Plan::prepare(&input, &missing).is_err());
        let input = ir(vec![
            heading(),
            table(false),
            Block::Paragraph {
                inlines: vec![reference(CrossReferenceKind::TableNumber, "표")],
            },
        ]);
        let error = Plan::prepare(&input, &roots(true)).err().unwrap();
        assert!(error.contains("native numbered table caption"));
        let no_chapter = ir(vec![
            Block::Paragraph {
                inlines: vec![reference(CrossReferenceKind::FigureNumber, "대상")],
            },
            figure("대상", vec![]),
        ]);
        assert!(Plan::prepare(&no_chapter, &roots(true)).is_err());
        assert!(Plan::prepare(&no_chapter, &roots(false)).is_ok());
    }

    #[test]
    fn unresolved_or_ambiguous_native_targets_fail_before_field_insertion() {
        let input = ir(vec![
            heading(),
            figure("대상", vec![]),
            Block::Paragraph {
                inlines: vec![reference(CrossReferenceKind::FigureNumber, "대상")],
            },
        ]);
        let (mut plan, _) = Plan::prepare(&input, &roots(true)).unwrap();
        let mut native = native_document();
        assert!(plan.resolve(&native).is_err());
        plan.bind("대상", 101).unwrap();
        let mut anchor = paragraph("");
        anchor.controls = vec![native_figure(101, 0)];
        native.sections[0].paragraphs = vec![anchor.clone()];
        assert!(plan.resolve(&native).is_err());
        anchor.controls = vec![native_figure(101, 1), native_figure(101, 2)];
        native.sections[0].paragraphs = vec![anchor];
        assert!(plan.resolve(&native).is_err());
        assert!(plan.bind("대상", 102).is_err());
    }

    #[test]
    fn adjacent_references_with_affixes_and_cloned_occurrences_keep_distinct_fields() {
        use rhwp::model::paragraph::CharShapeRef;
        let input = ir(vec![
            heading(),
            Block::Paragraph {
                inlines: vec![
                    reference(CrossReferenceKind::FigureNumber, "대상"),
                    reference(CrossReferenceKind::TableNumber, "표"),
                ],
            },
            figure("대상", vec![]),
            table(true),
        ]);
        let (mut plan, prepared) = Plan::prepare(&input, &roots(true)).unwrap();
        plan.bind("대상", 101).unwrap();
        plan.bind("표", 102).unwrap();
        let mut native = native_document();
        let mut anchor = paragraph("");
        anchor.controls = vec![native_figure(101, 12), native_table(102, 35)];
        native.sections[0].paragraphs = vec![anchor];
        plan.resolve(&native).unwrap();
        let Block::Paragraph { inlines } = &prepared.as_document().blocks[1] else {
            panic!()
        };
        let markers = inlines
            .iter()
            .map(|inline| match inline {
                Inline::Text { value } => value.as_str(),
                _ => panic!(),
            })
            .collect::<Vec<_>>();
        let mut p = paragraph(&format!("앞😀{}{}끝", markers[0], markers[1]));
        p.set_single_char_shape(3);
        let second_start = "앞😀".encode_utf16().count() + markers[0].encode_utf16().count();
        p.char_shapes.push(CharShapeRef {
            start_pos: second_start as u32,
            char_shape_id: 7,
        });
        let mut output = vec![p.clone(), p];
        let mut allocator = crate::model_tools::identity_seed(&native).unwrap();
        let fields = plan.apply(&mut output, &mut allocator).unwrap();
        assert_eq!(fields.len(), 4);
        let expected = "앞😀[그림 7-12번][표 7-35번]끝";
        for paragraph in &output {
            assert_eq!(paragraph.text, expected);
            assert_eq!(paragraph.field_ranges.len(), 2);
            assert!(
                paragraph.field_ranges[0].end_char_idx < paragraph.field_ranges[1].start_char_idx
            );
            assert!(
                paragraph
                    .char_offsets
                    .windows(2)
                    .all(|pair| pair[0] < pair[1])
            );
            assert_eq!(paragraph.char_shape_id_at(2), Some(3));
            let second_label = expected.chars().position(|ch| ch == '표').unwrap();
            assert_eq!(paragraph.char_shape_id_at(second_label), Some(7));
            assert_eq!(
                paragraph
                    .controls
                    .iter()
                    .filter(|c| matches!(c, Control::Field(_)))
                    .count(),
                2
            );
        }
    }

    #[test]
    fn reservation_uses_first_tag_character_format_and_keeps_literal_fragment_formats() {
        let mut template = roots(true);
        let sample = &mut template[2];
        let chapter = crate::template::tag("num:heading1");
        let slot = crate::template::tag("slot:ref.figure.number");
        let chapter_start = sample.text[..sample.text.find(&chapter).unwrap()]
            .chars()
            .count();
        let chapter_end = chapter_start + chapter.chars().count();
        let slot_start = sample.text[..sample.text.find(&slot).unwrap()]
            .chars()
            .count();
        let slot_end = slot_start + slot.chars().count();
        sample.set_single_char_shape(11);
        sample.apply_char_shape_range(chapter_start, chapter_start + 1, 21);
        sample.apply_char_shape_range(chapter_start + 1, chapter_end, 22);
        sample.apply_char_shape_range(chapter_end, slot_start, 31);
        sample.apply_char_shape_range(slot_start, slot_start + 1, 41);
        sample.apply_char_shape_range(slot_start + 1, slot_end, 42);
        sample.apply_char_shape_range(slot_end, sample.text.chars().count(), 51);
        let (plan, _) = Plan::prepare(&ir(vec![]), &template).unwrap();
        assert_eq!(
            plan.reserved_fragments(&CrossReferenceKind::FigureNumber)
                .unwrap(),
            vec![
                ("[그림 ".into(), 11),
                ("999".into(), 21),
                ("-".into(), 31),
                ("999".into(), 41),
                ("번]".into(), 51),
            ]
        );
        assert!(
            plan.reserved_fragments(&CrossReferenceKind::HeadingNumber)
                .is_err()
        );
    }

    #[test]
    fn measurement_restores_symbolic_references_inside_original_format_wrappers() {
        let original = vec![Inline::Strong {
            inlines: vec![Inline::Emph {
                inlines: vec![Inline::Link {
                    target: "https://example.org".into(),
                    title: Some("외부".into()),
                    inlines: vec![
                        text("참조"),
                        reference(CrossReferenceKind::FigureNumber, "대상"),
                    ],
                }],
            }],
        }];
        let input = ir(vec![
            heading(),
            Block::Paragraph {
                inlines: original.clone(),
            },
            figure("대상", vec![]),
        ]);
        let (plan, prepared) = Plan::prepare(&input, &roots(true)).unwrap();
        let Block::Paragraph { inlines } = &prepared.as_document().blocks[1] else {
            panic!()
        };
        // Reservation must work before native target identities/counters exist.
        assert_eq!(plan.measurement_inlines(inlines).unwrap(), original);
    }
}
