// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Euisoon Ahn
//! Template-only heading blocks and direct-child repetition.

use crate::{
    conditional_ranges,
    heading_plan::HeadingPlan,
    model_tools::map_nested_containers,
    template::{flatten, replace, replace_metadata, replace_rich, require_plain, tag},
};
use md2hwp_core::{ValidatedDocument, ir::Block};
use rhwp::model::{
    document::DocInfo,
    paragraph::{ColumnBreakType, Paragraph},
};

struct Sample {
    paragraphs: Vec<Paragraph>,
    after: bool,
}
pub(crate) struct Samples {
    samples: Vec<Sample>,
    marker_prefix: String,
}

const TITLE_MARKER_PREFIX: &str = "MD2HWP_RHWP_TITLE_84A17_";

struct TitleScope<'a> {
    slots: usize,
    marker_prefix: &'a str,
    ordinal: u32,
}

pub(crate) fn page_break(p: &mut Paragraph) {
    p.column_type = ColumnBreakType::Page;
    p.raw_break_type |= 4;
    p.page_break_synthesized = false;
}

impl Samples {
    pub(crate) fn empty_tail(&self, level: u8) -> Result<Paragraph, String> {
        let own = tag(&format!("slot:heading{level}"));
        fn find(paragraphs: &mut [Paragraph], own: &str) -> Result<Option<Paragraph>, String> {
            for p in paragraphs {
                if p.text.contains(own) {
                    return empty_sample(p, own).map(Some);
                }
                let mut result = None;
                map_nested_containers(p, &mut |nested| {
                    if result.is_none() {
                        result = find(nested, own)?;
                    }
                    Ok(())
                })?;
                if result.is_some() {
                    return Ok(result);
                }
            }
            Ok(None)
        }
        let mut sample = conditional_ranges::apply(
            self.samples[usize::from(level - 1)].paragraphs.clone(),
            true,
        )?;
        find(&mut sample, &own)?.ok_or("Missing heading title format".into())
    }
    pub(crate) fn read(
        roots: &[Paragraph],
        info: &DocInfo,
    ) -> Result<(Self, Vec<Paragraph>), String> {
        let mut lowered = roots.to_vec();
        let mut samples = Vec::new();
        for level in 1..=6 {
            let role = format!("heading{level}");
            let plain: Vec<_> = lowered
                .iter()
                .enumerate()
                .filter(|(_, p)| p.text.contains(&tag(&role)))
                .map(|(i, _)| i)
                .collect();
            let begins: Vec<_> = lowered
                .iter()
                .enumerate()
                .filter(|(_, p)| p.text == tag(&format!("begin:{role}")))
                .map(|(i, _)| i)
                .collect();
            let ends: Vec<_> = lowered
                .iter()
                .enumerate()
                .filter(|(_, p)| p.text == tag(&format!("end:{role}")))
                .map(|(i, _)| i)
                .collect();
            let (start, end, mut paragraphs, after) = match (
                plain.as_slice(),
                begins.as_slice(),
                ends.as_slice(),
            ) {
                ([index], [], []) => {
                    require_plain(&lowered[*index])?;
                    if lowered[*index].text.matches(&tag(&role)).count() != 1 {
                        return Err(format!("Duplicate {role} title slot"));
                    }
                    let mut p = lowered[*index].clone();
                    replace(&mut p, &tag(&role), &tag(&format!("slot:{role}")))?;
                    (*index, *index, vec![p], false)
                }
                ([], [start], [end]) if end > start && end - start <= 65 && end > &(start + 1) => {
                    require_plain(&lowered[*start])?;
                    require_plain(&lowered[*end])?;
                    let mut range = lowered[start + 1..*end].to_vec();
                    if is_page_break(&lowered[*start]) {
                        page_break(&mut range[0]);
                    }
                    (*start, *end, range, is_page_break(&lowered[*end]))
                }
                _ => {
                    return Err(format!(
                        "{role} requires either one paragraph declaration or one 1–64 paragraph block, not both"
                    ));
                }
            };
            let value = serde_json::to_value(&paragraphs).map_err(|e| e.to_string())?;
            check_controls(&value)?;
            for first in [true, false] {
                let checked = conditional_ranges::apply(paragraphs.clone(), first)?;
                let mut scratch = info.clone();
                rewrite(
                    checked,
                    level,
                    None,
                    None,
                    TITLE_MARKER_PREFIX,
                    0,
                    &mut scratch,
                )?;
            }
            // Keep the existing full-template validator authoritative for the
            // remaining roles, replacing only the independently checked block.
            let mut declaration = Paragraph::new_empty();
            declaration.insert_text_at(0, &tag(&role));
            lowered.splice(start..=end, [declaration]);
            samples.push(Sample {
                paragraphs: std::mem::take(&mut paragraphs),
                after,
            });
        }
        Ok((
            Self {
                samples,
                marker_prefix: TITLE_MARKER_PREFIX.into(),
            },
            lowered,
        ))
    }

    pub(crate) fn prepare(&mut self, ir: &ValidatedDocument) -> Result<(), String> {
        let ir_text = serde_json::to_string(ir.as_document()).map_err(|e| e.to_string())?;
        let paragraphs: Vec<_> = self
            .samples
            .iter()
            .map(|sample| &sample.paragraphs)
            .collect();
        let sample_text = serde_json::to_string(&paragraphs).map_err(|e| e.to_string())?;
        self.marker_prefix = TITLE_MARKER_PREFIX.into();
        while ir_text.contains(&self.marker_prefix) || sample_text.contains(&self.marker_prefix) {
            self.marker_prefix.push('_');
        }
        Ok(())
    }

    pub(crate) fn expand(
        &self,
        level: u8,
        operation: usize,
        ir: &ValidatedDocument,
        plan: &HeadingPlan,
        info: &mut DocInfo,
    ) -> Result<(Vec<Paragraph>, bool), String> {
        let sample = &self.samples[usize::from(level - 1)];
        let context = plan.context(operation).ok_or("Missing heading context")?;
        let paragraphs = conditional_ranges::apply(sample.paragraphs.clone(), context.first)?;
        let mut output = rewrite(
            paragraphs,
            level,
            Some((operation, ir)),
            Some(plan),
            &self.marker_prefix,
            0,
            info,
        )?;
        for p in &mut output {
            metadata(p, &ir.as_document().metadata)?;
        }
        // Title insertion is delayed until declarations and metadata have all
        // been consumed, so manuscript tags remain literal.
        fill_pending(&mut output, ir, &self.marker_prefix, info)?;
        Ok((output, sample.after))
    }
}

fn is_page_break(p: &Paragraph) -> bool {
    p.column_type == ColumnBreakType::Page || p.raw_break_type & 4 != 0
}

fn set_repeated_number(p: &mut Paragraph, ordinal: u32, info: &mut DocInfo) -> Result<(), String> {
    use rhwp::model::{
        raw_provenance::{record_digest, record_raw_permitted},
        style::HeadType,
    };
    let shape_index = usize::from(p.para_shape_id);
    let mut shape = info
        .para_shapes
        .get(shape_index)
        .ok_or("Repeated heading references an absent paragraph format")?
        .clone();
    if shape.head_type != HeadType::Number {
        return Ok(());
    }
    let number_index = usize::from(
        shape
            .numbering_id
            .checked_sub(1)
            .ok_or("Native numbered heading requires a numbering definition")?,
    );
    let mut numbering = info
        .numberings
        .get(number_index)
        .ok_or("Repeated heading references an absent numbering definition")?
        .clone();
    let level = usize::from(shape.para_level);
    if level >= numbering.level_start_numbers.len() {
        return Err("Unsupported native repeated heading numbering level".into());
    }
    let start =
        u16::try_from(ordinal).map_err(|_| "Native repeated heading numbering exceeds 65535")?;
    let numbering_id =
        u16::try_from(info.numberings.len() + 1).map_err(|_| "Too many numbering definitions")?;
    let shape_id =
        u16::try_from(info.para_shapes.len()).map_err(|_| "Too many paragraph formats")?;
    let seals = info.raw_provenance.as_ref().map(|seal| &seal.record_seals);
    if !record_raw_permitted(
        seals.map(|seals| &seals.numberings[..]),
        number_index,
        &numbering,
    ) {
        numbering.raw_data = None;
    }
    if !record_raw_permitted(
        seals.map(|seals| &seals.para_shapes[..]),
        shape_index,
        &shape,
    ) {
        shape.raw_data = None;
    }
    // Change only the two native start fields and the paragraph's numbering
    // reference. Preserve every unmodeled byte in valid source records.
    if let Some(raw) = &mut numbering.raw_data {
        let mut offset = 0_usize;
        for _ in 0..7 {
            let count_at = offset
                .checked_add(12)
                .ok_or("Invalid native numbering record")?;
            let bytes = raw
                .get(count_at..count_at + 2)
                .ok_or("Truncated native numbering record")?;
            let count = usize::from(u16::from_le_bytes([bytes[0], bytes[1]]));
            offset = count_at + 2 + count * 2;
            if offset > raw.len() {
                return Err("Truncated native numbering format".into());
            }
        }
        raw.get_mut(offset..offset + 2)
            .ok_or("Truncated native numbering start")?
            .copy_from_slice(&start.to_le_bytes());
        let level_at = offset + 2 + level * 4;
        raw.get_mut(level_at..level_at + 4)
            .ok_or("Native numbering requires per-level starts")?
            .copy_from_slice(&ordinal.to_le_bytes());
    }
    if let Some(raw) = &mut shape.raw_data {
        raw.get_mut(30..32)
            .ok_or("Truncated native paragraph numbering reference")?
            .copy_from_slice(&numbering_id.to_le_bytes());
    }
    numbering.start_number = start;
    numbering.level_start_numbers[level] = ordinal;
    shape.numbering_id = numbering_id;
    if let Some(seal) = &mut info.raw_provenance {
        seal.record_seals
            .numberings
            .resize(info.numberings.len(), [0; 32]);
        seal.record_seals.numberings.push(record_digest(&numbering));
        seal.record_seals
            .para_shapes
            .resize(info.para_shapes.len(), [0; 32]);
        seal.record_seals.para_shapes.push(record_digest(&shape));
    }
    info.numberings.push(numbering);
    info.para_shapes.push(shape);
    info.raw_stream_dirty = true;
    p.para_shape_id = shape_id;
    Ok(())
}

fn check_controls(value: &serde_json::Value) -> Result<(), String> {
    match value {
        serde_json::Value::Object(fields) => {
            for (name, value) in fields {
                if ["SectionDef", "ColumnDef", "Picture", "Ole"].contains(&name.as_str()) {
                    return Err(format!(
                        "Heading blocks do not support native {name} controls"
                    ));
                }
                check_controls(value)?;
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                check_controls(value)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn marker(p: &Paragraph, boundary: &str) -> Option<u8> {
    let digit = p
        .text
        .strip_prefix(&format!("{{{{md2hwp:{boundary}:each.child:heading"))?
        .strip_suffix("}}")?;
    heading_digit(digit)
}

fn heading_digit(digit: &str) -> Option<u8> {
    match digit.as_bytes() {
        [value @ b'1'..=b'6'] => Some(value - b'0'),
        _ => None,
    }
}

fn rewrite(
    paragraphs: Vec<Paragraph>,
    level: u8,
    current: Option<(usize, &ValidatedDocument)>,
    plan: Option<&HeadingPlan>,
    marker_prefix: &str,
    ordinal: u32,
    info: &mut DocInfo,
) -> Result<Vec<Paragraph>, String> {
    let mut scope = TitleScope {
        slots: 0,
        marker_prefix,
        ordinal,
    };
    let result = rewrite_container(paragraphs, level, current, plan, info, &mut scope)?;
    if scope.slots == 0 {
        return Err(format!("heading{level} scope requires its own title slot"));
    }
    Ok(result)
}

fn rewrite_container(
    paragraphs: Vec<Paragraph>,
    level: u8,
    current: Option<(usize, &ValidatedDocument)>,
    plan: Option<&HeadingPlan>,
    info: &mut DocInfo,
    scope: &mut TitleScope<'_>,
) -> Result<Vec<Paragraph>, String> {
    let mut output = Vec::new();
    let mut index = 0;
    let mut next_break = false;
    while index < paragraphs.len() {
        if let Some(child) = marker(&paragraphs[index], "begin") {
            require_plain(&paragraphs[index])?;
            if child != level + 1 || child > 6 {
                return Err("each.child must name the immediately deeper heading level".into());
            }
            let mut stack = vec![child];
            let mut end = index + 1;
            while end < paragraphs.len() {
                if let Some(n) = marker(&paragraphs[end], "begin") {
                    stack.push(n);
                }
                if let Some(n) = marker(&paragraphs[end], "end") {
                    if stack.pop() != Some(n) {
                        return Err("Mismatched each.child boundaries".into());
                    }
                    if stack.is_empty() {
                        break;
                    }
                }
                end += 1;
            }
            if end == paragraphs.len() {
                return Err("each.child must end in the same paragraph container".into());
            }
            require_plain(&paragraphs[end])?;
            let sample = paragraphs[index + 1..end].to_vec();
            // Validate every child prototype even if this manuscript has none.
            rewrite(
                sample.clone(),
                child,
                None,
                None,
                scope.marker_prefix,
                0,
                &mut info.clone(),
            )?;
            if let (Some((operation, ir)), Some(plan)) = (current, plan) {
                for (ordinal, &child_operation) in plan
                    .context(operation)
                    .ok_or("Missing parent context")?
                    .direct_children
                    .iter()
                    .enumerate()
                {
                    let mut expanded = rewrite(
                        sample.clone(),
                        child,
                        Some((child_operation, ir)),
                        Some(plan),
                        scope.marker_prefix,
                        u32::try_from(ordinal + 1).map_err(|_| "Too many repeated headings")?,
                        info,
                    )?;
                    if (is_page_break(&paragraphs[index]) || next_break) && !expanded.is_empty() {
                        page_break(&mut expanded[0]);
                        next_break = false;
                    }
                    output.extend(expanded);
                }
            }
            next_break |= is_page_break(&paragraphs[end]);
            index = end + 1;
            continue;
        }
        if marker(&paragraphs[index], "end").is_some() {
            return Err("Orphan each.child end".into());
        }
        let mut p = paragraphs[index].clone();
        if next_break {
            page_break(&mut p);
            next_break = false;
        }
        let own = tag(&format!("slot:heading{level}"));
        if p.text.contains(&own) {
            require_plain(&p)?;
            if p.text.matches(&own).count() != 1 {
                return Err("Each title paragraph requires exactly one own slot".into());
            }
            scope.slots += 1;
            if current.is_some() && scope.ordinal > 0 {
                set_repeated_number(&mut p, scope.ordinal, info)?;
            }
        }
        // Resolve only template-owned tags, before any manuscript insertion.
        let tags = template_tags(&p.text)?;
        for needle in tags {
            if needle == own {
                if let Some((operation, _)) = current {
                    replace(&mut p, &needle, &pending(scope.marker_prefix, operation))?;
                } else {
                    replace(&mut p, &needle, "")?;
                }
            } else if let Some(n) = needle
                .strip_prefix("{{md2hwp:num:heading")
                .and_then(|s| s.strip_suffix("}}"))
                .and_then(heading_digit)
            {
                if n == 0 || n > level {
                    return Err("Heading number tags may name only own or ancestor levels".into());
                }
                let number = current
                    .and_then(|(operation, _)| plan?.context(operation))
                    .map_or(0, |c| c.numbers[usize::from(n - 1)]);
                replace(&mut p, &needle, &number.to_string())?;
            } else if !needle.starts_with("{{md2hwp:meta:") {
                return Err(format!(
                    "Unexpected declaration in heading{level}: {needle}"
                ));
            }
        }
        map_nested_containers(&mut p, &mut |nested| {
            let empty = nested.first().map(|p| empty_sample(p, "")).transpose()?;
            let rewritten =
                rewrite_container(std::mem::take(nested), level, current, plan, info, scope)?;
            *nested = rewritten;
            if nested.is_empty() {
                nested.push(empty.unwrap_or_else(Paragraph::new_empty));
            }
            Ok(())
        })?;
        output.push(p);
        index += 1;
    }
    if next_break {
        return Err("each.child ending page break requires a following paragraph".into());
    }
    Ok(output)
}

pub(crate) fn empty_sample(sample: &Paragraph, slot: &str) -> Result<Paragraph, String> {
    let byte = sample.text.find(slot).ok_or("Missing sample format slot")?;
    let offset = sample.text[..byte].chars().count();
    let format = sample
        .char_shape_id_at(offset)
        .ok_or("Sample has no character format")?;
    let mut p = Paragraph::new_empty();
    p.apply_meta(sample.capture_meta());
    p.column_type = ColumnBreakType::None;
    p.raw_break_type = 0;
    p.tab_extended.clear();
    p.set_single_char_shape(format);
    Ok(p)
}

fn template_tags(text: &str) -> Result<Vec<String>, String> {
    let mut result = Vec::new();
    let mut remaining = text;
    while let Some(start) = remaining.find("{{md2hwp:") {
        remaining = &remaining[start..];
        let end = remaining
            .find("}}")
            .ok_or("Unclosed heading template tag")?
            + 2;
        result.push(remaining[..end].into());
        remaining = &remaining[end..];
    }
    Ok(result)
}

// Marker includes operation index and is adapter-private, never accepted as
// manuscript syntax. It is replaced once from the precomputed heading plan.
fn pending(marker_prefix: &str, operation: usize) -> String {
    format!("{marker_prefix}{operation}_END")
}
fn metadata(p: &mut Paragraph, metadata: &md2hwp_core::ir::Metadata) -> Result<(), String> {
    replace_metadata(p, metadata)?;
    map_nested_containers(p, &mut |nested| {
        for p in nested {
            self::metadata(p, metadata)?;
        }
        Ok(())
    })
}
fn fill_pending(
    paragraphs: &mut [Paragraph],
    ir: &ValidatedDocument,
    marker_prefix: &str,
    info: &mut DocInfo,
) -> Result<(), String> {
    for p in paragraphs {
        // Determine the operation before replacing any source text.
        if let Some(start) = p.text.find(marker_prefix) {
            let suffix = &p.text[start + marker_prefix.len()..];
            let operation: usize = suffix
                .split_once("_END")
                .ok_or("Malformed private heading marker")?
                .0
                .parse()
                .map_err(|_| "Malformed private heading operation")?;
            let Block::Heading { inlines, .. } = &ir.as_document().blocks[operation] else {
                return Err("Private heading target is not a heading".into());
            };
            let needle = pending(marker_prefix, operation);
            let offset = p.text[..start].chars().count();
            let base = p
                .char_shape_id_at(offset)
                .ok_or("Heading title has no character format")?;
            let mut spans = Vec::new();
            flatten(inlines, false, false, &mut spans);
            replace_rich(p, &needle, &spans, base, info)?;
        }
        map_nested_containers(p, &mut |nested| {
            fill_pending(nested, ir, marker_prefix, info)
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use md2hwp_core::ir::{IR_VERSION, Inline, Metadata, SCHEMA_NAME};
    use md2hwp_core::{Document, ValidationLimits, validate};
    use rhwp::model::{
        control::Control,
        table::{Cell, Table},
    };

    fn paragraph(text: &str) -> Paragraph {
        let mut paragraph = Paragraph::new_empty();
        paragraph.set_single_char_shape(0);
        paragraph.insert_text_at(0, text);
        paragraph
    }

    fn doc_info() -> DocInfo {
        use rhwp::model::style::{CharShape, Font, ParaShape, Style};
        DocInfo {
            char_shapes: vec![CharShape::default()],
            para_shapes: vec![ParaShape::default()],
            styles: vec![Style::default()],
            font_faces: vec![vec![Font::default()]; 7],
            ..DocInfo::default()
        }
    }

    fn heading(level: u8, text: &str) -> Block {
        Block::Heading {
            id: None,
            level,
            inlines: vec![Inline::Strong {
                inlines: vec![Inline::Text { value: text.into() }],
            }],
        }
    }

    fn ir(blocks: Vec<Block>) -> ValidatedDocument {
        validate(
            Document {
                schema: SCHEMA_NAME.into(),
                ir_version: IR_VERSION.into(),
                metadata: Metadata::default(),
                blocks,
            },
            &ValidationLimits::default(),
        )
        .unwrap()
    }

    fn block_roots(level: u8, contents: Vec<Paragraph>) -> Vec<Paragraph> {
        let mut roots = Vec::new();
        for current in 1..=6 {
            if current == level {
                roots.push(paragraph(&tag(&format!("begin:heading{current}"))));
                roots.extend(contents.clone());
                roots.push(paragraph(&tag(&format!("end:heading{current}"))));
            } else {
                roots.push(paragraph(&tag(&format!("heading{current}"))));
            }
        }
        roots
    }

    #[test]
    fn multiple_title_paragraphs_in_root_and_table_cells_clone_rich_titles() {
        let mut anchor = paragraph("");
        anchor.controls.push(Control::Table(Box::new(Table {
            cells: vec![
                Cell {
                    paragraphs: vec![paragraph("표 {{md2hwp:slot:heading1}}")],
                    ..Default::default()
                },
                Cell {
                    paragraphs: vec![paragraph("재현 {{md2hwp:slot:heading1}} 끝")],
                    ..Default::default()
                },
            ],
            ..Default::default()
        })));
        let roots = block_roots(1, vec![paragraph("밖 {{md2hwp:slot:heading1}}"), anchor]);
        let source_snapshot = format!("{roots:?}");
        let (samples, _) = Samples::read(&roots, &doc_info()).unwrap();
        let ir = ir(vec![heading(1, "한😀제목")]);
        let plan = HeadingPlan::new(&ir).unwrap();
        let mut info = doc_info();
        let (output, _) = samples.expand(1, 0, &ir, &plan, &mut info).unwrap();
        assert_eq!(output[0].text, "밖 한😀제목");
        let Control::Table(table) = &output[1].controls[0] else {
            panic!("table clone lost")
        };
        assert_eq!(table.cells[0].paragraphs[0].text, "표 한😀제목");
        assert_eq!(table.cells[1].paragraphs[0].text, "재현 한😀제목 끝");
        for p in [
            &output[0],
            &table.cells[0].paragraphs[0],
            &table.cells[1].paragraphs[0],
        ] {
            let title = p.text.find('한').unwrap();
            let title = p.text[..title].chars().count();
            assert!(info.char_shapes[p.char_shape_id_at(title).unwrap() as usize].bold);
            assert!(!info.char_shapes[p.char_shape_id_at(0).unwrap() as usize].bold);
        }
        assert_eq!(format!("{roots:?}"), source_snapshot);
    }

    #[test]
    fn nested_child_repetitions_use_each_child_operation_and_counters() {
        let contents = [
            "{{md2hwp:num:heading1}} {{md2hwp:slot:heading1}}",
            "{{md2hwp:begin:each.child:heading2}}",
            "{{md2hwp:num:heading1}}.{{md2hwp:num:heading2}} {{md2hwp:slot:heading2}}",
            "{{md2hwp:begin:each.child:heading3}}",
            "{{md2hwp:num:heading1}}.{{md2hwp:num:heading2}}.{{md2hwp:num:heading3}} {{md2hwp:slot:heading3}}",
            "{{md2hwp:end:each.child:heading3}}",
            "{{md2hwp:end:each.child:heading2}}",
        ].map(paragraph).to_vec();
        let (samples, _) = Samples::read(&block_roots(1, contents), &doc_info()).unwrap();
        let ir = ir(vec![
            heading(1, "부모"),
            heading(2, "자식A"),
            heading(3, "손자A"),
            heading(2, "자식B"),
            heading(3, "손자B"),
            heading(1, "다음부모"),
            heading(2, "범위밖"),
        ]);
        let plan = HeadingPlan::new(&ir).unwrap();
        let (output, _) = samples.expand(1, 0, &ir, &plan, &mut doc_info()).unwrap();
        assert_eq!(
            output.iter().map(|p| p.text.as_str()).collect::<Vec<_>>(),
            [
                "1 부모",
                "1.1 자식A",
                "1.1.1 손자A",
                "1.2 자식B",
                "1.2.1 손자B"
            ]
        );
    }

    #[test]
    fn own_heading_role_first_flag_selects_once_and_except_once() {
        let roots = block_roots(
            2,
            vec![paragraph(concat!(
                "{{md2hwp:begin:once}}최초{{md2hwp:end:once}}",
                "{{md2hwp:begin:except.once}}후속{{md2hwp:end:except.once}}",
                " {{md2hwp:num:heading2}} {{md2hwp:slot:heading2}}"
            ))],
        );
        let (samples, _) = Samples::read(&roots, &doc_info()).unwrap();
        let ir = ir(vec![
            heading(1, "A"),
            heading(2, "첫"),
            heading(1, "B"),
            heading(2, "둘"),
        ]);
        let plan = HeadingPlan::new(&ir).unwrap();
        let (first, _) = samples.expand(2, 1, &ir, &plan, &mut doc_info()).unwrap();
        let (second, _) = samples.expand(2, 3, &ir, &plan, &mut doc_info()).unwrap();
        assert_eq!(first[0].text, "최초 1 첫");
        assert_eq!(second[0].text, "후속 1 둘");
    }

    #[test]
    fn plain_heading_number_affixes_keep_their_formatting() {
        let mut info = doc_info();
        let mut italic = info.char_shapes[0].clone();
        italic.italic = true;
        info.char_shapes.push(italic);
        let mut plain = paragraph("제 {{md2hwp:num:heading1}}장 {{md2hwp:heading1}} 끝");
        plain.apply_char_shape_range(0, 2, 1);
        let mut roots = vec![plain];
        roots.extend((2..=6).map(|level| paragraph(&tag(&format!("heading{level}")))));
        let (samples, _) = Samples::read(&roots, &info).unwrap();
        let ir = ir(vec![heading(1, "제목")]);
        let plan = HeadingPlan::new(&ir).unwrap();
        let (output, _) = samples.expand(1, 0, &ir, &plan, &mut info).unwrap();
        assert_eq!(output[0].text, "제 1장 제목 끝");
        assert!(info.char_shapes[output[0].char_shape_id_at(0).unwrap() as usize].italic);
        assert!(!info.char_shapes[output[0].char_shape_id_at(2).unwrap() as usize].italic);
        assert!(info.char_shapes[output[0].char_shape_id_at(5).unwrap() as usize].bold);
    }

    #[test]
    fn unknown_deeper_and_unused_child_declarations_are_rejected() {
        for bad in [
            "{{md2hwp:unknown}}",
            "{{md2hwp:num:heading2}}",
            "{{md2hwp:slot:heading2}}",
        ] {
            let roots = block_roots(
                1,
                vec![paragraph(&format!("{{{{md2hwp:slot:heading1}}}} {bad}"))],
            );
            assert!(Samples::read(&roots, &doc_info()).is_err(), "{bad}");
        }
        let roots = block_roots(
            1,
            [
                "{{md2hwp:slot:heading1}}",
                "{{md2hwp:begin:each.child:heading2}}",
                "{{md2hwp:slot:heading2}} {{md2hwp:num:heading3}}",
                "{{md2hwp:end:each.child:heading2}}",
            ]
            .map(paragraph)
            .to_vec(),
        );
        assert!(Samples::read(&roots, &doc_info()).is_err());
    }

    #[test]
    fn private_marker_literals_in_metadata_titles_and_templates_are_preserved() {
        let metadata_literal = "MD2HWP_RHWP_TITLE_84A17_0_END";
        let title_literal = "MD2HWP_RHWP_TITLE_84A17__0_END{{md2hwp:num:heading1}}";
        let template_literal = "MD2HWP_RHWP_TITLE_84A17___0_END";
        let roots = block_roots(
            1,
            vec![paragraph(&format!(
                "{template_literal} / {{{{md2hwp:meta:title}}}} / {{{{md2hwp:slot:heading1}}}}"
            ))],
        );
        let (mut samples, _) = Samples::read(&roots, &doc_info()).unwrap();
        let mut document = ir(vec![heading(1, title_literal)]).into_document();
        document.metadata.0.insert(
            "title".into(),
            md2hwp_core::ir::MetadataValue::Text(metadata_literal.into()),
        );
        let ir = validate(document, &ValidationLimits::default()).unwrap();
        samples.prepare(&ir).unwrap();
        let plan = HeadingPlan::new(&ir).unwrap();
        let (output, _) = samples.expand(1, 0, &ir, &plan, &mut doc_info()).unwrap();
        assert_eq!(
            output[0].text,
            format!("{template_literal} / {metadata_literal} / {title_literal}")
        );
    }

    #[test]
    fn repeated_native_numbering_uses_sibling_ordinals_and_preserves_raw_records() {
        use rhwp::model::{
            raw_provenance::{
                DocInfoRecordSeals, DocInfoSeal, record_digest, record_raw_permitted,
            },
            style::{HeadType, Numbering, ParaShape},
        };
        let mut info = doc_info();
        let mut numbering_raw = vec![0_u8; 98]; // Seven empty native format strings.
        numbering_raw.extend_from_slice(&9_u16.to_le_bytes());
        for _ in 0..7 {
            numbering_raw.extend_from_slice(&9_u32.to_le_bytes());
        }
        numbering_raw.extend_from_slice(&[0xab, 0xcd]);
        info.numberings.push(Numbering {
            raw_data: Some(numbering_raw.clone()),
            start_number: 9,
            level_start_numbers: [9; 7],
            ..Default::default()
        });
        let mut shape_raw = vec![0_u8; 60];
        let attr1 = (2_u32 << 23) | (2 << 25);
        shape_raw[..4].copy_from_slice(&attr1.to_le_bytes());
        shape_raw[30..32].copy_from_slice(&1_u16.to_le_bytes());
        shape_raw[58..].copy_from_slice(&[0xab, 0xcd]);
        info.para_shapes.push(ParaShape {
            raw_data: Some(shape_raw.clone()),
            attr1,
            head_type: HeadType::Number,
            para_level: 2,
            numbering_id: 1,
            ..Default::default()
        });
        info.raw_provenance = Some(DocInfoSeal {
            model_digest: [0; 32],
            raw_digest: [0; 32],
            record_seals: DocInfoRecordSeals {
                numberings: info.numberings.iter().map(record_digest).collect(),
                para_shapes: info.para_shapes.iter().map(record_digest).collect(),
                ..Default::default()
            },
        });
        let mut child = paragraph("{{md2hwp:slot:heading2}}");
        child.para_shape_id = 1;
        let roots = block_roots(
            1,
            vec![
                paragraph("{{md2hwp:slot:heading1}}"),
                paragraph("{{md2hwp:begin:each.child:heading2}}"),
                child,
                paragraph("{{md2hwp:end:each.child:heading2}}"),
            ],
        );
        let (mut samples, _) = Samples::read(&roots, &info).unwrap();
        let ir = ir(vec![
            heading(1, "A"),
            heading(2, "A1"),
            heading(2, "A2"),
            heading(1, "B"),
            heading(2, "B1"),
        ]);
        samples.prepare(&ir).unwrap();
        let plan = HeadingPlan::new(&ir).unwrap();
        let (first, _) = samples.expand(1, 0, &ir, &plan, &mut info).unwrap();
        let (second, _) = samples.expand(1, 3, &ir, &plan, &mut info).unwrap();
        let paragraphs = [&first[1], &first[2], &second[1]];
        for (p, ordinal) in paragraphs.into_iter().zip([1_u32, 2, 1]) {
            let shape = &info.para_shapes[usize::from(p.para_shape_id)];
            assert_eq!(shape.head_type, HeadType::Number);
            let numbering = &info.numberings[usize::from(shape.numbering_id - 1)];
            assert_eq!(numbering.start_number, ordinal as u16);
            assert_eq!(numbering.level_start_numbers[2], ordinal);
            let mut expected = numbering_raw.clone();
            expected[98..100].copy_from_slice(&(ordinal as u16).to_le_bytes());
            expected[108..112].copy_from_slice(&ordinal.to_le_bytes());
            assert_eq!(numbering.raw_data.as_ref().unwrap(), &expected);
            let mut expected = shape_raw.clone();
            expected[30..32].copy_from_slice(&shape.numbering_id.to_le_bytes());
            assert_eq!(shape.raw_data.as_ref().unwrap(), &expected);
            let seals = &info.raw_provenance.as_ref().unwrap().record_seals;
            assert!(record_raw_permitted(
                Some(&seals.numberings),
                usize::from(shape.numbering_id - 1),
                numbering
            ));
            assert!(record_raw_permitted(
                Some(&seals.para_shapes),
                usize::from(p.para_shape_id),
                shape
            ));
        }
        assert_ne!(first[1].para_shape_id, first[2].para_shape_id);
        assert_ne!(first[1].para_shape_id, second[1].para_shape_id);
        assert_eq!(
            info.numberings[0].raw_data.as_ref().unwrap(),
            &numbering_raw
        );
        assert_eq!(info.para_shapes[1].raw_data.as_ref().unwrap(), &shape_raw);
    }

    #[test]
    fn noncanonical_heading_digits_are_rejected() {
        for text in [
            "{{md2hwp:num:heading01}}",
            "{{md2hwp:num:heading+1}}",
            "{{md2hwp:begin:each.child:heading02}}",
        ] {
            let roots = block_roots(
                1,
                vec![paragraph(&format!("{{{{md2hwp:slot:heading1}}}} {text}"))],
            );
            assert!(Samples::read(&roots, &doc_info()).is_err(), "{text}");
        }
        assert!(marker(&paragraph("{{md2hwp:begin:each.child:heading02}}"), "begin").is_none());
    }

    #[test]
    fn conditional_ranges_reject_hidden_declarations_and_malformed_boundaries() {
        for bad in [
            "{{md2hwp:begin:once}}{{md2hwp:slot:heading1}}{{md2hwp:end:once}}",
            "{{md2hwp:slot:heading1}}{{md2hwp:begin:once}}x{{md2hwp:end:except.once}}",
            "{{md2hwp:slot:heading1}}{{md2hwp:begin:except.once}}{{md2hwp:unknown}}{{md2hwp:end:except.once}}",
        ] {
            assert!(
                Samples::read(&block_roots(1, vec![paragraph(bad)]), &doc_info()).is_err(),
                "{bad}"
            );
        }
    }
}
