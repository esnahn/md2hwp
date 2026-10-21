// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Euisoon Ahn
//! Deliberately small HWP template renderer. Unsupported structures fail before
//! the caller's model is changed; template and manuscript text are never parsed
//! recursively after substitution.

use std::collections::BTreeMap;

use md2hwp_core::ir::{Block, Inline, Metadata, MetadataValue};
use md2hwp_core::validate::ValidatedDocument;
use rhwp::DocumentCore;
use rhwp::model::control::Control;
use rhwp::model::document::DocInfo;
use rhwp::model::paragraph::Paragraph;

const PREFIX: &str = "{{md2hwp:";

fn tag(name: &str) -> String {
    format!("{PREFIX}{name}}}}}")
}

/// Compose into a private clone and publish only a complete supported document.
/// `set_document` also invalidates the upstream layout and pagination caches.
pub fn render(ir: &ValidatedDocument, core: &mut DocumentCore) -> Result<(), String> {
    let input = ir.as_document();
    for block in &input.blocks {
        match block {
            Block::Paragraph { inlines } | Block::Heading { inlines, .. } => {
                check_inlines(inlines)?;
            }
            Block::VerbatimBlock { .. } => return Err(unsupported("verbatim blocks")),
            Block::List { .. } => return Err(unsupported("lists")),
            Block::Table { .. } => return Err(unsupported("tables")),
            Block::Figure { .. } => return Err(unsupported("figures")),
        }
    }
    let mut document = core.document().clone();
    if document.sections.len() != 1 {
        return Err(unsupported("templates with multiple sections"));
    }
    let roots = &document.sections[0].paragraphs;
    if roots
        .iter()
        .any(|p| p.text.contains(&format!("{PREFIX}begin:heading")))
    {
        return Err(unsupported(
            "heading blocks (once/except/each.child and native title objects); use a separate template with six plain heading samples",
        ));
    }
    let single = |name: &str| -> Result<usize, String> {
        let needle = tag(name);
        let found: Vec<_> = roots
            .iter()
            .enumerate()
            .filter(|(_, p)| p.text == needle)
            .map(|(i, _)| i)
            .collect();
        if found.len() != 1 {
            return Err(format!(
                "Template requires exactly one plain root {needle}; found {}",
                found.len()
            ));
        }
        Ok(found[0])
    };
    let begin = single("begin:template")?;
    let end = single("end:template")?;
    let content = single("content")?;
    if begin >= end
        || end + 1 != content
        || content + 2 != roots.len()
        || !roots[content + 1].text.is_empty()
        || !roots[content + 1].controls.is_empty()
    {
        return Err(
            "Template definitions must precede the terminal content target and its empty successor"
                .into(),
        );
    }
    let version = single("ir-version:0.4")
        .map_err(|_| "Template IR must be exactly 0.4; regenerate or explicitly adopt the current declarations".to_string())?;
    let body = single("body")?;
    let mut sample_indices = vec![body];
    for level in 1..=6 {
        sample_indices.push(single(&format!("heading{level}"))?);
    }
    for &index in sample_indices.iter().chain(std::iter::once(&version)) {
        if index <= begin || index >= end {
            return Err(
                "Template style samples and IR version must be inside begin:template/end:template"
                    .into(),
            );
        }
        require_plain(&roots[index])?;
    }
    require_plain(&roots[end])?;
    require_plain(&roots[content])?;
    // Only begin:template may own section/column controls. Keep its native
    // paragraph rather than moving control identities or manufacturing options.
    if roots[begin]
        .controls
        .iter()
        .any(|c| !matches!(c, Control::SectionDef(_) | Control::ColumnDef(_)))
    {
        return Err("begin:template may carry only native section/column controls".into());
    }
    validate_definitions(&roots[begin + 1..end])?;
    let samples: Vec<_> = sample_indices.iter().map(|&i| roots[i].clone()).collect();
    let mut output = roots[..begin].to_vec();
    if !roots[begin].controls.is_empty() {
        let mut retained = roots[begin].clone();
        let len = retained.text.chars().count();
        retained.delete_text_at(0, len);
        discard_stale_layout(&mut retained);
        output.push(retained);
    }
    // Substitute static text before inserting manuscript text so literal tags in
    // either metadata or the manuscript remain literal.
    for paragraph in &mut output {
        replace_metadata(paragraph, &input.metadata)?;
        reject_control_tags(paragraph)?;
    }
    for block in &input.blocks {
        let (sample, name, inlines) = match block {
            Block::Paragraph { inlines } => (&samples[0], "body".to_string(), inlines),
            Block::Heading { level, inlines, .. } => (
                &samples[usize::from(*level)],
                format!("heading{level}"),
                inlines,
            ),
            _ => unreachable!("unsupported blocks were checked before composition"),
        };
        let mut paragraph = sample.clone();
        let base = paragraph
            .char_shape_id_at(0)
            .ok_or("Template sample has no character format")?;
        let mut spans = Vec::new();
        flatten(inlines, false, false, &mut spans);
        replace_rich(
            &mut paragraph,
            &tag(&name),
            &spans,
            base,
            &mut document.doc_info,
        )?;
        output.push(paragraph);
    }
    output.push(roots[content + 1].clone());
    let section = &mut document.sections[0];
    section.paragraphs = output;
    section.raw_stream = None;
    section.raw_provenance = None;
    core.set_document(document);
    Ok(())
}

fn unsupported(feature: &str) -> String {
    format!("rhwp generation does not yet support {feature}; use the Hancom backend")
}

fn validate_definitions(roots: &[Paragraph]) -> Result<(), String> {
    let value = serde_json::to_value(roots).map_err(|e| e.to_string())?;
    if count_model_kind(&value, "SectionDef") != 0 || count_model_kind(&value, "ColumnDef") != 0 {
        return Err("Only begin:template may carry native section/column controls".into());
    }
    let mut texts = Vec::new();
    collect_texts(&value, &mut texts);
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for text in texts {
        let mut cursor = 0;
        while let Some(relative) = text[cursor..].find(PREFIX) {
            let start = cursor + relative;
            let end = text[start..]
                .find("}}")
                .map(|n| start + n)
                .ok_or("Unclosed template definition tag")?;
            *counts
                .entry(text[start + PREFIX.len()..end].to_string())
                .or_default() += 1;
            cursor = end + 2;
        }
    }
    let mut required: BTreeMap<String, usize> = [
        "ir-version:0.4",
        "body",
        "reset",
        "footnote",
        "list.bullet",
        "list.ordered",
        "begin:code",
        "end:code",
        "slot:code.content",
        "slot:code.source.prefix",
        "slot:code.source",
        "begin:figure",
        "end:figure",
        "slot:figure.caption",
        "slot:figure.source.prefix",
        "slot:figure.source",
        "begin:table",
        "end:table",
        "slot:table.caption",
        "slot:table.source.prefix",
        "slot:table.source",
        "begin:ref.figure.number",
        "end:ref.figure.number",
        "slot:ref.figure.number",
        "begin:ref.table.number",
        "end:ref.table.number",
        "slot:ref.table.number",
    ]
    .into_iter()
    .map(|s| (s.to_string(), 1))
    .collect();
    for level in 1..=6 {
        required.insert(format!("heading{level}"), 1);
        required.insert(format!("begin:ref.heading{level}.number"), 1);
        required.insert(format!("end:ref.heading{level}.number"), 1);
    }
    required.insert("slot:table.header".into(), 2);
    required.insert("slot:table.content".into(), 2);
    for (name, expected) in &required {
        let actual = counts.get(name).copied().unwrap_or(0);
        if actual != *expected {
            return Err(format!(
                "Template requires {expected} {{{{md2hwp:{name}}}}} declaration(s); found {actual}. Regenerate or explicitly adopt current IR 0.4 prototypes"
            ));
        }
    }
    for setting in [
        "list.max-depth",
        "list.indent-hwp",
        "figure.max-width-mm",
        "table.width-mm",
    ] {
        let matches: Vec<_> = counts
            .iter()
            .filter(|(key, _)| key.starts_with(&format!("{setting}:")))
            .collect();
        if matches.len() != 1 || *matches[0].1 != 1 {
            return Err(format!("Template requires exactly one {setting} setting"));
        }
        let n = matches[0].0[setting.len() + 1..]
            .parse::<f64>()
            .map_err(|_| format!("Invalid {setting}"))?;
        if !n.is_finite()
            || n <= 0.0
            || (setting.contains("width") && n > 142.0)
            || (setting == "list.max-depth" && (n.fract() != 0.0 || n > 6.0))
            || (setting == "list.indent-hwp" && (n.fract() != 0.0 || n > 10000.0))
        {
            return Err(format!("Invalid {setting}"));
        }
    }
    for (key, count) in &counts {
        if required.contains_key(key)
            || (1..=6).any(|n| key == &format!("num:heading{n}"))
            || [
                "list.max-depth:",
                "list.indent-hwp:",
                "figure.max-width-mm:",
                "table.width-mm:",
            ]
            .iter()
            .any(|prefix| key.starts_with(prefix))
        {
            continue;
        }
        if ["footnote.next", "slot:code.title"].contains(&key.as_str()) && *count == 1 {
            continue;
        }
        return Err(format!(
            "Unknown, duplicate or unsupported template declaration: {key}"
        ));
    }
    let mut ranges = Vec::new();
    for role in [
        "code",
        "figure",
        "table",
        "ref.figure.number",
        "ref.table.number",
        "ref.heading1.number",
        "ref.heading2.number",
        "ref.heading3.number",
        "ref.heading4.number",
        "ref.heading5.number",
        "ref.heading6.number",
    ] {
        let start = roots
            .iter()
            .position(|p| p.text == tag(&format!("begin:{role}")))
            .ok_or_else(|| format!("Missing root begin:{role}"))?;
        let end = roots
            .iter()
            .position(|p| p.text == tag(&format!("end:{role}")))
            .ok_or_else(|| format!("Missing root end:{role}"))?;
        require_plain(&roots[start])?;
        require_plain(&roots[end])?;
        if end <= start {
            return Err(format!("Mismatched {role} range"));
        }
        if ranges.iter().any(|&(a, b)| start <= b && a <= end) {
            return Err("Template prototype ranges must not overlap or nest".into());
        }
        ranges.push((start, end));
        if role.starts_with("ref.") {
            if end != start + 2 {
                return Err(format!("Reference prototype {role} requires one paragraph"));
            }
            require_plain(&roots[start + 1])?;
            if let Some(level) = role
                .strip_prefix("ref.heading")
                .and_then(|s| s.strip_suffix(".number"))
                .and_then(|s| s.parse::<u8>().ok())
            {
                let text = &roots[start + 1].text;
                if !text.contains(&tag(&format!("num:heading{level}")))
                    || ((level + 1)..=6).any(|n| text.contains(&tag(&format!("num:heading{n}"))))
                {
                    return Err(format!(
                        "{role} must contain its own number and may not reference deeper heading levels"
                    ));
                }
            }
        } else {
            let kind = if role == "figure" { "Picture" } else { "Table" };
            let native = serde_json::to_value(&roots[start + 1..end]).map_err(|e| e.to_string())?;
            if count_model_kind(&native, kind) != 1 {
                return Err(format!(
                    "Template {role} prototype requires exactly one native {kind}"
                ));
            }
            let mut local_texts = Vec::new();
            collect_texts(&native, &mut local_texts);
            let slots: &[&str] = match role {
                "code" => &[
                    "slot:code.content",
                    "slot:code.source.prefix",
                    "slot:code.source",
                ],
                "figure" => &[
                    "slot:figure.caption",
                    "slot:figure.source.prefix",
                    "slot:figure.source",
                ],
                _ => &[
                    "slot:table.caption",
                    "slot:table.header",
                    "slot:table.content",
                    "slot:table.source.prefix",
                    "slot:table.source",
                ],
            };
            for slot in slots {
                if !local_texts.iter().any(|text| text.contains(&tag(slot))) {
                    return Err(format!(
                        "Template {role} prototype is missing its local {slot}"
                    ));
                }
            }
        }
    }
    Ok(())
}

fn count_model_kind(value: &serde_json::Value, name: &str) -> usize {
    match value {
        serde_json::Value::Object(fields) => fields
            .iter()
            .map(|(key, value)| usize::from(key == name) + count_model_kind(value, name))
            .sum(),
        serde_json::Value::Array(values) => values
            .iter()
            .map(|value| count_model_kind(value, name))
            .sum(),
        _ => 0,
    }
}

fn collect_texts<'a>(value: &'a serde_json::Value, texts: &mut Vec<&'a str>) {
    match value {
        serde_json::Value::Object(fields) => {
            for (key, value) in fields {
                if key == "text" {
                    if let Some(text) = value.as_str() {
                        texts.push(text);
                    }
                } else {
                    collect_texts(value, texts);
                }
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                collect_texts(value, texts);
            }
        }
        _ => {}
    }
}

fn require_plain(p: &Paragraph) -> Result<(), String> {
    if !p.controls.is_empty()
        || !p.field_ranges.is_empty()
        || !p.range_tags.is_empty()
        || !p.title_marks.is_empty()
        || !p.markpen_marks.is_empty()
        || p.text.chars().any(char::is_control)
    {
        return Err("Template declarations and samples must be plain single-line root paragraphs without controls or ranges".into());
    }
    Ok(())
}

fn check_inlines(inlines: &[Inline]) -> Result<(), String> {
    for inline in inlines {
        match inline {
            Inline::Text { .. } | Inline::Space | Inline::LineBreak => {}
            Inline::Strong { inlines }
            | Inline::Emph { inlines }
            | Inline::Link { inlines, .. } => check_inlines(inlines)?,
            Inline::Footnote { .. } => return Err(unsupported("footnotes")),
            Inline::CrossReference { .. } => return Err(unsupported("cross references")),
        }
    }
    Ok(())
}

#[derive(Debug)]
struct Span {
    text: String,
    bold: bool,
    italic: bool,
}

fn flatten(inlines: &[Inline], bold: bool, italic: bool, out: &mut Vec<Span>) {
    for inline in inlines {
        let text = match inline {
            Inline::Text { value } => value.clone(),
            Inline::Space => " ".into(),
            Inline::LineBreak => "\n".into(),
            Inline::Strong { inlines } => {
                flatten(inlines, true, italic, out);
                continue;
            }
            Inline::Emph { inlines } => {
                flatten(inlines, bold, true, out);
                continue;
            }
            Inline::Link { inlines, .. } => {
                flatten(inlines, bold, italic, out);
                continue;
            }
            _ => unreachable!("inlines were validated before composition"),
        };
        out.push(Span { text, bold, italic });
    }
}

fn replace_rich(
    p: &mut Paragraph,
    needle: &str,
    spans: &[Span],
    base: u32,
    info: &mut DocInfo,
) -> Result<(), String> {
    let text: String = spans.iter().map(|s| s.text.as_str()).collect();
    let start = replace(p, needle, &text)?;
    let mut offset = start;
    let mut formats = BTreeMap::new();
    for span in spans {
        let len = span.text.chars().count();
        let id = if !span.bold && !span.italic {
            base
        } else if let Some(id) = formats.get(&(span.bold, span.italic)) {
            *id
        } else {
            let mut shape = info
                .char_shapes
                .get(base as usize)
                .ok_or("Template references an absent character format")?
                .clone();
            shape.bold |= span.bold;
            shape.italic |= span.italic;
            shape.attr = (shape.attr & !3) | u32::from(shape.italic) | (u32::from(shape.bold) << 1);
            shape.raw_data = None;
            let id =
                u32::try_from(info.char_shapes.len()).map_err(|_| "Too many character formats")?;
            info.char_shapes.push(shape);
            info.raw_stream_dirty = true;
            formats.insert((span.bold, span.italic), id);
            id
        };
        p.apply_char_shape_range(offset, offset + len, id);
        offset += len;
    }
    discard_stale_layout(p);
    Ok(())
}

/// Upstream editing methods take scalar-character offsets and maintain their
/// corresponding UTF-16 HWP positions, including supplementary Unicode scalars.
fn replace(p: &mut Paragraph, needle: &str, value: &str) -> Result<usize, String> {
    let byte = p
        .text
        .find(needle)
        .ok_or("Template substitution target disappeared")?;
    let start = p.text[..byte].chars().count();
    let end = start + needle.chars().count();
    let offsets = &p.char_offsets;
    for pair in offsets[start..end].windows(2) {
        // Plain ASCII tags cannot span native controls, tabs or field markers.
        if pair[1] != pair[0] + 1 {
            return Err("Template tag crosses a native control".into());
        }
    }
    let format = p
        .char_shape_id_at(start)
        .ok_or("Template tag has no character format")?;
    p.delete_text_at(start, needle.chars().count());
    p.insert_text_at(start, value);
    p.apply_char_shape_range(start, start + value.chars().count(), format);
    discard_stale_layout(p);
    Ok(start)
}

fn replace_metadata(p: &mut Paragraph, metadata: &Metadata) -> Result<(), String> {
    let mut replacements = Vec::new();
    let mut cursor = 0;
    while let Some(relative) = p.text[cursor..].find(PREFIX) {
        let start = cursor + relative;
        let end = p.text[start..]
            .find("}}")
            .map(|x| start + x + 2)
            .ok_or("Unclosed template tag")?;
        let full = &p.text[start..end];
        let key = full
            .strip_prefix(&format!("{PREFIX}meta:"))
            .and_then(|s| s.strip_suffix("}}"))
            .ok_or_else(|| format!("Unbound template declaration outside definitions: {full}"))?;
        let (key, format) = match key.split_once(":fmt:") {
            Some(("date-meta", format)) => ("date-meta", Some(format)),
            Some(_) => return Err("fmt is supported only for date-meta".into()),
            None if key.contains(':') => return Err("Malformed metadata tag".into()),
            None => (key, None),
        };
        let value = match metadata.0.get(key) {
            Some(MetadataValue::Text(value)) => value.clone(),
            Some(MetadataValue::Authors(values)) => values.join(", "),
            None => return Err(format!("Template references missing metadata: {key}")),
        };
        if value.trim().is_empty() {
            return Err(format!("Template references empty metadata: {key}"));
        }
        let value = match format {
            Some(format) => format_date(&value, format)?,
            None => value,
        };
        replacements.push((p.text[..start].chars().count(), full.chars().count(), value));
        cursor = end;
    }
    // Right-to-left edits keep tag-shaped inserted values out of later searches.
    for (start, length, value) in replacements.into_iter().rev() {
        let format = p
            .char_shape_id_at(start)
            .ok_or("Metadata tag has no character format")?;
        let end = start + length;
        if p.char_offsets[start..end]
            .windows(2)
            .any(|pair| pair[1] != pair[0] + 1)
        {
            return Err("Metadata tag crosses a native control".into());
        }
        p.delete_text_at(start, length);
        p.insert_text_at(start, &value);
        p.apply_char_shape_range(start, start + value.chars().count(), format);
        discard_stale_layout(p);
    }
    Ok(())
}

/// Template rows describe the sample text, not the generated text. Drop the
/// entire row cache on edited paragraphs so the expected model and saved HWP
/// both contain an uncomposed paragraph. Untouched static paragraphs retain all
/// original rows and provenance. This mirrors upstream replace_line_segs' cache
/// reset while keeping the public dirty flag set until actual composition.
fn discard_stale_layout(p: &mut Paragraph) {
    p.line_segs.clear();
    p.layout_space_metrics.clear();
    p.layout_only_fill_lines = 0;
    p.source_line_seg_vertical_pos = None;
    p.hwpx_axis_shift = 0;
    p.invalidate_layout_inputs();
}

fn format_date(iso: &str, format: &str) -> Result<String, String> {
    // The core owns date interpretation. Here accept only its exact derived ISO
    // representation, without reparsing the original display date or using today.
    if iso.len() != 10 || md2hwp_core::metadata::normalize_date(iso).as_deref() != Some(iso) {
        return Err("date-meta must be an ISO date".into());
    }
    if format.is_empty() || format.chars().any(char::is_control) {
        return Err("fmt must be nonempty single-line text".into());
    }
    let year = &iso[..4];
    let month = &iso[5..7];
    let day = &iso[8..];
    let mut output = String::new();
    let mut chars = format.chars();
    while let Some(ch) = chars.next() {
        if ch != '%' {
            output.push(ch);
            continue;
        }
        let mut token = chars.next().ok_or("Incomplete fmt specifier")?;
        let unpadded = token == '-';
        if unpadded {
            token = chars.next().ok_or("Incomplete fmt specifier")?;
        }
        output.push_str(match (token, unpadded) {
            ('Y', false) => year,
            ('y', false) => &year[2..],
            ('m', false) => month,
            ('d', false) => day,
            ('m', true) => month.trim_start_matches('0'),
            ('d', true) => day.trim_start_matches('0'),
            ('F', false) => iso,
            ('%', false) => "%",
            _ => {
                return Err(format!(
                    "Unsupported fmt specifier %{}{token}",
                    if unpadded { "-" } else { "" }
                ));
            }
        });
    }
    Ok(output)
}

fn reject_control_tags(p: &Paragraph) -> Result<(), String> {
    for control in &p.controls {
        let encoded = serde_json::to_string(control).map_err(|e| e.to_string())?;
        if encoded.contains(PREFIX) {
            return Err(unsupported(
                "template tags inside native controls (tables, headers, footers, master pages or shapes)",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use md2hwp_core::ir::{Document, IR_VERSION, SCHEMA_NAME};
    use md2hwp_core::{ValidationLimits, validate};

    fn paragraph(text: &str) -> Paragraph {
        let mut p = Paragraph::new_empty();
        p.set_single_char_shape(0);
        p.insert_text_at(0, text);
        p
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

    fn template() -> DocumentCore {
        let mut core = DocumentCore::new_empty();
        let mut doc = core.document().clone();
        doc.doc_info = doc_info();
        doc.sections.push(rhwp::model::document::Section::default());
        let mut names = vec![
            "begin:template".to_string(),
            "ir-version:0.4".into(),
            "body".into(),
        ];
        names.extend((1..=6).map(|n| format!("heading{n}")));
        names.extend(["end:template".into(), "content".into()]);
        doc.sections[0].paragraphs = names.iter().map(|s| paragraph(&tag(s))).collect();
        doc.sections[0].paragraphs.push(paragraph(""));
        core.set_document(doc);
        core
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

    #[test]
    fn unicode_rich_text_keeps_utf16_offsets_and_literal_tags() {
        let mut info = doc_info();
        let mut p = paragraph(&tag("body"));
        let mut spans = Vec::new();
        flatten(
            &[
                Inline::Text {
                    value: "한😀".into(),
                },
                Inline::Strong {
                    inlines: vec![Inline::Text {
                        value: "굵게".into(),
                    }],
                },
                Inline::Link {
                    target: "https://example.com".into(),
                    title: None,
                    inlines: vec![Inline::Text {
                        value: "{{md2hwp:body}}".into(),
                    }],
                },
            ],
            false,
            false,
            &mut spans,
        );
        replace_rich(&mut p, &tag("body"), &spans, 0, &mut info).unwrap();
        assert_eq!(p.text, "한😀굵게{{md2hwp:body}}");
        assert_eq!(&p.char_offsets[..5], &[0, 1, 3, 4, 5]);
        let bold = p.char_shape_id_at(2).unwrap() as usize;
        assert!(info.char_shapes[bold].bold);
        assert!(p.controls.is_empty());
        assert!(p.line_segs.is_empty());
    }

    #[test]
    fn metadata_substitution_does_not_reparse_inserted_identical_tags() {
        let mut p = paragraph("{{md2hwp:meta:title}} / {{md2hwp:meta:title}}");
        let mut metadata = Metadata::default();
        metadata.0.insert(
            "title".into(),
            MetadataValue::Text("😀{{md2hwp:meta:title}}".into()),
        );
        replace_metadata(&mut p, &metadata).unwrap();
        assert_eq!(p.text, "😀{{md2hwp:meta:title}} / 😀{{md2hwp:meta:title}}");
        assert_eq!(p.char_offsets[1], 2);
    }

    #[test]
    fn unsupported_content_and_bad_templates_leave_model_unchanged() {
        let mut core = template();
        let before = format!("{:?}", core.document());
        assert!(
            render(
                &ir(vec![Block::VerbatimBlock {
                    lines: vec!["x".into()],
                    source: None
                }]),
                &mut core
            )
            .is_err()
        );
        assert_eq!(format!("{:?}", core.document()), before);
        core.document_mut().sections[0]
            .paragraphs
            .insert(2, paragraph(&tag("body")));
        let before = format!("{:?}", core.document());
        assert!(
            render(&ir(vec![]), &mut core)
                .unwrap_err()
                .contains("exactly one")
        );
        assert_eq!(format!("{:?}", core.document()), before);
    }

    #[test]
    fn missing_metadata_is_an_error() {
        let mut p = paragraph("{{md2hwp:meta:missing}}");
        assert!(
            replace_metadata(&mut p, &Metadata::default())
                .unwrap_err()
                .contains("missing metadata")
        );
    }

    #[test]
    fn date_metadata_formats_match_the_shared_contract() {
        assert_eq!(
            format_date("2024-02-09", "%Y %y %m %-m %d %-d %F %%").unwrap(),
            "2024 24 02 2 09 9 2024-02-09 %"
        );
        assert_eq!(
            format_date("2024-02-29", "%Y년 %-m월 %-d일").unwrap(),
            "2024년 2월 29일"
        );
        for format in ["", "%", "%-", "%q", "%-Y", "%-%", "line\nbreak", "tab\t"] {
            assert!(format_date("2024-02-09", format).is_err(), "{format:?}");
        }
        assert!(format_date("2023-02-29", "%F").is_err());
        assert!(format_date("2024-2-09", "%F").is_err());
        let mut p = paragraph("날짜: {{md2hwp:meta:date-meta:fmt:%Y년 %-m월 %-d일}}");
        let mut metadata = Metadata::default();
        metadata
            .0
            .insert("date-meta".into(), MetadataValue::Text("2024-02-09".into()));
        replace_metadata(&mut p, &metadata).unwrap();
        assert_eq!(p.text, "날짜: 2024년 2월 9일");
        assert!(p.stored_text_partition_is_dirty());
    }

    #[test]
    fn metadata_inherits_first_tag_character_across_runs() {
        let mut p = paragraph("앞 {{md2hwp:meta:title}} 뒤");
        p.apply_char_shape_range(2, 5, 7);
        p.apply_char_shape_range(5, 22, 8);
        let mut metadata = Metadata::default();
        metadata
            .0
            .insert("title".into(), MetadataValue::Text("😀제목".into()));
        replace_metadata(&mut p, &metadata).unwrap();
        assert_eq!(p.text, "앞 😀제목 뒤");
        assert_eq!(p.char_shape_id_at(2), Some(7));
        assert_eq!(p.char_shape_id_at(3), Some(7));
        assert_eq!(p.char_shape_id_at(4), Some(7));
        assert_eq!(p.char_shape_id_at(6), Some(0));
        metadata
            .0
            .insert("title".into(), MetadataValue::Text("  ".into()));
        assert!(replace_metadata(&mut paragraph("{{md2hwp:meta:title}}"), &metadata).is_err());
        assert!(
            replace_metadata(&mut paragraph("{{md2hwp:meta:title:fmt:%Y}}"), &metadata).is_err()
        );
    }

    #[test]
    fn explicit_line_break_remains_inside_the_formatted_paragraph() {
        let mut p = paragraph(&tag("body"));
        let mut info = doc_info();
        let mut spans = Vec::new();
        flatten(
            &[
                Inline::Text {
                    value: "첫😀".into(),
                },
                Inline::LineBreak,
                Inline::Emph {
                    inlines: vec![Inline::Text {
                        value: "둘째".into(),
                    }],
                },
            ],
            false,
            false,
            &mut spans,
        );
        replace_rich(&mut p, &tag("body"), &spans, 0, &mut info).unwrap();
        assert_eq!(p.text, "첫😀\n둘째");
        assert_eq!(p.char_offsets, vec![0, 1, 3, 4, 5]);
        assert!(info.char_shapes[p.char_shape_id_at(3).unwrap() as usize].italic);
        assert!(p.stored_text_partition_is_dirty());
        assert!(p.line_segs.is_empty());
    }

    #[test]
    fn metadata_edits_discard_sample_rows_but_static_paragraphs_keep_them() {
        use rhwp::model::paragraph::SpaceMetric;
        let mut edited = paragraph("제목: {{md2hwp:meta:title}}");
        assert!(!edited.line_segs.is_empty());
        edited.layout_space_metrics = vec![(0, SpaceMetric::Stored)];
        edited.layout_only_fill_lines = 1;
        edited.source_line_seg_vertical_pos = Some(vec![1234]);
        edited.hwpx_axis_shift = 8;
        let mut static_paragraph = paragraph("고정 텍스트");
        static_paragraph.layout_space_metrics = vec![(0, SpaceMetric::Stored)];
        static_paragraph.source_line_seg_vertical_pos = Some(vec![1234]);
        static_paragraph.hwpx_axis_shift = 8;
        let before = format!("{static_paragraph:?}");
        let mut metadata = Metadata::default();
        metadata
            .0
            .insert("title".into(), MetadataValue::Text("새 제목".into()));
        replace_metadata(&mut edited, &metadata).unwrap();
        assert!(edited.line_segs.is_empty());
        assert!(edited.layout_space_metrics.is_empty());
        assert_eq!(edited.layout_only_fill_lines, 0);
        assert!(edited.source_line_seg_vertical_pos.is_none());
        assert_eq!(edited.hwpx_axis_shift, 0);
        assert!(edited.stored_text_partition_is_dirty());
        replace_metadata(&mut static_paragraph, &metadata).unwrap();
        assert_eq!(format!("{static_paragraph:?}"), before);
    }
}
