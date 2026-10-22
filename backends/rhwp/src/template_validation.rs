// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Euisoon Ahn
//! Strict, manuscript-independent validation of every editable template prototype.

use rhwp::model::{
    control::Control,
    document::{DocInfo, Document},
    paragraph::Paragraph,
};

use crate::template::{require_plain, tag};

const PREFIX: &str = "{{md2hwp:";
const MAX_DEPTH: usize = 128;

pub(crate) struct Inspection {
    pub required_metadata: Vec<String>,
    pub requires_recognized_date: bool,
}

pub(crate) fn inspect(source: &Document) -> Result<Inspection, String> {
    if source.sections.len() != 1 {
        return Err("rhwp templates require exactly one section".into());
    }
    // Inspect original samples, including every discarded conditional branch,
    // before lowerers replace them with synthetic declaration paragraphs.
    check_formats(&source.sections[0].paragraphs, &source.doc_info, 0)?;
    for master in &source.sections[0].section_def.master_pages {
        check_formats(&master.paragraphs, &source.doc_info, 0)?;
    }
    let metadata = crate::template_metadata::inspect(source)?;
    let document = &metadata.document;
    let original = &document.sections[0].paragraphs;
    crate::heading_references::Samples::read(original)?;
    crate::object_references::Plan::read_samples(original)?;
    let note = crate::footnotes::Sample::read(original, &document.doc_info)?;
    let roots = note.lower_sample(original);
    let begin = single(&roots, "begin:template")?;
    let end = single(&roots, "end:template")?;
    let content = single(&roots, "content")?;
    if begin >= end
        || end + 1 != content
        || content + 2 != roots.len()
        || !roots[content + 1].text.is_empty()
    {
        return Err(
            "Template definitions must precede the terminal content target and its empty successor"
                .into(),
        );
    }
    require_plain(&roots[end])?;
    require_plain(&roots[content])?;
    require_plain(&roots[content + 1])?;
    if roots[begin]
        .controls
        .iter()
        .any(|control| !matches!(control, Control::SectionDef(_) | Control::ColumnDef(_)))
        || !roots[begin].field_ranges.is_empty()
        || !roots[begin].range_tags.is_empty()
        || !roots[begin].title_marks.is_empty()
        || !roots[begin].markpen_marks.is_empty()
    {
        return Err("begin:template may carry only native section/column controls".into());
    }
    for (index, paragraph) in roots.iter().enumerate() {
        if index < begin || index > end && index != content {
            reject_tags(std::slice::from_ref(paragraph), 0)?;
        } else if index == begin || index == end || index == content {
            let mut paragraph = paragraph.clone();
            crate::model_tools::map_nested_containers(&mut paragraph, &mut |children| {
                reject_tags(children, 1)
            })?;
        }
    }
    for master in &document.sections[0].section_def.master_pages {
        reject_tags(&master.paragraphs, 0)?;
    }
    let version = single(&roots, "ir-version:0.4")
        .map_err(|_| "Template IR must be exactly 0.4; regenerate or explicitly adopt the current declarations".to_string())?;
    let body = inside_plain(&roots, "body", begin, end)?;
    let reset = inside_plain(&roots, "reset", begin, end)?;
    if version <= begin || version >= end {
        return Err("Template IR version must be inside template definitions".into());
    }
    require_plain(&roots[version])?;
    if style_name(&roots[body], &document.doc_info)?
        == style_name(&roots[reset], &document.doc_info)?
    {
        return Err("Reset and body styles must differ".into());
    }
    // All six heading readers validate both once/except.once variants and all
    // nested each.child prototypes even without manuscript headings.
    let (_, definitions) =
        crate::headings::Samples::read(&roots[begin + 1..end], &document.doc_info)?;
    crate::template::validate_definitions(&definitions)?;
    check_number_ownership(&definitions)?;
    crate::lists::Lists::read(&roots[begin + 1..end], &document.doc_info)?;
    crate::code::Prototype::read(&roots[begin + 1..end], &roots[body], &document.doc_info)?;
    crate::figures::Prototype::read(&roots[begin + 1..end], &document.doc_info)?;
    crate::tables::Prototype::read(&roots, &document.doc_info)?;
    Ok(Inspection {
        required_metadata: metadata.keys,
        requires_recognized_date: metadata.requires_recognized_date,
    })
}

fn check_number_ownership(roots: &[Paragraph]) -> Result<(), String> {
    let mut role: Option<&str> = None;
    for paragraph in roots {
        if let Some(name) = paragraph
            .text
            .strip_prefix("{{md2hwp:begin:")
            .and_then(|s| s.strip_suffix("}}"))
        {
            role = Some(name);
        }
        let allowed = role
            .is_some_and(|name| name == "figure" || name == "table" || name.starts_with("ref."));
        fn check(paragraph: &Paragraph, allowed: bool, depth: usize) -> Result<(), String> {
            if depth > MAX_DEPTH {
                return Err("Template number container nesting exceeds 128".into());
            }
            if !allowed && paragraph.text.contains("{{md2hwp:num:") {
                return Err(
                    "Heading number tag requires a heading, caption or number-reference scope"
                        .into(),
                );
            }
            crate::model_tools::map_nested_containers(&mut paragraph.clone(), &mut |children| {
                for child in children {
                    check(child, allowed, depth + 1)?;
                }
                Ok(())
            })
        }
        check(paragraph, allowed, 0)?;
        if paragraph.text.starts_with("{{md2hwp:end:") {
            role = None;
        }
    }
    Ok(())
}

fn single(roots: &[Paragraph], name: &str) -> Result<usize, String> {
    let token = tag(name);
    let indices: Vec<_> = roots
        .iter()
        .enumerate()
        .filter(|(_, paragraph)| paragraph.text == token)
        .map(|(index, _)| index)
        .collect();
    match indices.as_slice() {
        [index] => Ok(*index),
        _ => Err(format!("Expected exactly one root declaration {name}")),
    }
}

fn inside_plain(
    roots: &[Paragraph],
    name: &str,
    begin: usize,
    end: usize,
) -> Result<usize, String> {
    let index = single(roots, name)?;
    if index <= begin || index >= end {
        return Err(format!("{name} must be inside template definitions"));
    }
    require_plain(&roots[index])?;
    Ok(index)
}

fn style_name<'a>(paragraph: &Paragraph, info: &'a DocInfo) -> Result<&'a str, String> {
    let style = info
        .styles
        .get(usize::from(paragraph.style_id))
        .ok_or("Template paragraph references an absent native style")?;
    if style.style_type != 0 || style.local_name.trim().is_empty() {
        return Err("Template paragraph requires a named native paragraph style".into());
    }
    if info
        .styles
        .iter()
        .filter(|candidate| candidate.style_type == 0 && candidate.local_name == style.local_name)
        .count()
        != 1
    {
        return Err("Template paragraph style name must be unique".into());
    }
    Ok(&style.local_name)
}

fn check_formats(paragraphs: &[Paragraph], info: &DocInfo, depth: usize) -> Result<(), String> {
    if depth > MAX_DEPTH {
        return Err("Template format container nesting exceeds 128".into());
    }
    for paragraph in paragraphs {
        style_name(paragraph, info)?;
        let style = &info.styles[usize::from(paragraph.style_id)];
        if info
            .para_shapes
            .get(usize::from(paragraph.para_shape_id))
            .is_none()
            || info
                .para_shapes
                .get(usize::from(style.para_shape_id))
                .is_none()
            || info
                .char_shapes
                .get(usize::from(style.char_shape_id))
                .is_none()
            || paragraph.char_shapes.is_empty()
            || paragraph.char_shapes.iter().any(|shape| {
                usize::try_from(shape.char_shape_id)
                    .ok()
                    .and_then(|index| info.char_shapes.get(index))
                    .is_none()
            })
        {
            return Err(
                "Template paragraph references an absent paragraph/character format".into(),
            );
        }
        crate::model_tools::map_nested_containers(&mut paragraph.clone(), &mut |children| {
            check_formats(children, info, depth + 1)
        })?;
    }
    Ok(())
}

fn reject_tags(paragraphs: &[Paragraph], depth: usize) -> Result<(), String> {
    if depth > MAX_DEPTH {
        return Err("Template declaration container nesting exceeds 128".into());
    }
    for paragraph in paragraphs {
        if paragraph.text.contains(PREFIX) {
            return Err(format!(
                "Unbound declaration outside template definitions: {}",
                paragraph.text
            ));
        }
        crate::model_tools::map_nested_containers(&mut paragraph.clone(), &mut |children| {
            reject_tags(children, depth + 1)
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tracked() -> Document {
        let mut core =
            rhwp::DocumentCore::from_bytes(include_bytes!("../../../templates/template.hwp"))
                .unwrap();
        crate::note_storage::restore_source_autonums(core.document_mut()).unwrap();
        core.document().clone()
    }

    fn edit(paragraph: &mut Paragraph, text: &str) {
        paragraph.delete_text_at(0, paragraph.text.chars().count());
        paragraph.insert_text_at(0, text);
    }

    #[test]
    fn tracked_template_passes_without_mutating_source() {
        let document = tracked();
        let before = document.clone();
        let result = inspect(&document).unwrap();
        assert!(rhwp::serializer::hwpx::roundtrip::diff_documents(&before, &document).is_empty());
        assert!(
            result
                .required_metadata
                .windows(2)
                .all(|pair| pair[0] != pair[1])
        );
    }

    #[test]
    fn rejects_orphan_heading_number_in_unused_definitions() {
        let mut document = tracked();
        let roots = &mut document.sections[0].paragraphs;
        let index = single(roots, "reset").unwrap();
        let mut paragraph = roots[index].clone();
        edit(&mut paragraph, "{{md2hwp:num:heading1}}");
        roots.insert(index, paragraph);
        assert!(inspect(&document).is_err());
    }

    #[test]
    fn rejects_reset_body_style_alias_and_nonplain_reset() {
        let mut document = tracked();
        let roots = &mut document.sections[0].paragraphs;
        let body = single(roots, "body").unwrap();
        let reset = single(roots, "reset").unwrap();
        roots[reset].style_id = roots[body].style_id;
        assert!(inspect(&document).is_err());
        let mut document = tracked();
        let roots = &mut document.sections[0].paragraphs;
        let reset = single(roots, "reset").unwrap();
        roots[reset]
            .controls
            .push(Control::ColumnDef(Default::default()));
        assert!(inspect(&document).is_err());
    }

    #[test]
    fn rejects_unused_role_with_missing_style_or_formats() {
        for kind in 0..3 {
            let mut document = tracked();
            let roots = &mut document.sections[0].paragraphs;
            let reset = single(roots, "reset").unwrap();
            match kind {
                0 => roots[reset].style_id = u8::MAX,
                1 => roots[reset].para_shape_id = u16::MAX,
                _ => roots[reset].char_shapes[0].char_shape_id = u32::MAX,
            }
            assert!(inspect(&document).is_err());
        }
    }

    #[test]
    fn rejects_unused_native_prototype_and_static_marker_mutations() {
        let mut document = tracked();
        let roots = &mut document.sections[0].paragraphs;
        let start = single(roots, "begin:code").unwrap();
        let end = single(roots, "end:code").unwrap();
        let anchor = roots[start + 1..end]
            .iter_mut()
            .find(|paragraph| {
                paragraph
                    .controls
                    .iter()
                    .any(|control| matches!(control, Control::Table(_)))
            })
            .unwrap();
        let table = anchor
            .controls
            .iter_mut()
            .find_map(|control| match control {
                Control::Table(table) => Some(table),
                _ => None,
            })
            .unwrap();
        table.cells.clear();
        assert!(inspect(&document).is_err());
        let mut document = tracked();
        let paragraph = &mut document.sections[0].paragraphs[0];
        paragraph.insert_text_at(0, "{{md2hwp:unknown}}");
        assert!(inspect(&document).is_err());
    }

    #[test]
    fn detects_metadata_in_unused_samples_and_static_native_containers() {
        use rhwp::model::header_footer::MasterPage;
        let mut document = tracked();
        let roots = &mut document.sections[0].paragraphs;
        let reset = single(roots, "reset").unwrap();
        let mut sample = roots[reset].clone();
        edit(
            &mut sample,
            "{{md2hwp:meta:md2hwp-name:detail}} {{md2hwp:meta:date-meta:fmt:%F}}",
        );
        document.sections[0]
            .section_def
            .master_pages
            .push(MasterPage {
                paragraphs: vec![sample],
                ..Default::default()
            });
        let result = inspect(&document).unwrap();
        assert!(
            result
                .required_metadata
                .contains(&"md2hwp-name:detail".into())
        );
        assert!(result.required_metadata.contains(&"date".into()));
        assert!(result.requires_recognized_date);
        let master = document.sections[0]
            .section_def
            .master_pages
            .last_mut()
            .unwrap();
        edit(
            &mut master.paragraphs[0],
            "{{md2hwp:meta:date-meta:fmt:%Q}}",
        );
        assert!(inspect(&document).is_err());
    }

    #[test]
    fn inspects_unused_conditional_heading_branch_before_lowering() {
        let mut document = tracked();
        let roots = &mut document.sections[0].paragraphs;
        let body = single(roots, "body").unwrap();
        let sample = roots[body].clone();
        let plain = roots
            .iter()
            .position(|paragraph| paragraph.text.contains(&tag("heading6")));
        let (start, end) = match plain {
            Some(index) => (index, index),
            None => (
                single(roots, "begin:heading6").unwrap(),
                single(roots, "end:heading6").unwrap(),
            ),
        };
        let texts = [
            tag("begin:heading6"),
            tag("slot:heading6"),
            format!(
                "{}{}{}",
                tag("begin:except.once"),
                tag("meta:md2hwp-unused"),
                tag("end:except.once")
            ),
            tag("end:heading6"),
        ];
        let replacement = texts.iter().map(|text| {
            let mut paragraph = sample.clone();
            edit(&mut paragraph, text);
            paragraph
        });
        roots.splice(start..=end, replacement);
        let result = inspect(&document).unwrap();
        assert!(result.required_metadata.contains(&"md2hwp-unused".into()));
        edit(
            &mut document.sections[0].paragraphs[start + 2],
            &format!(
                "{}{}{}",
                tag("begin:except.once"),
                tag("meta:date-meta:fmt:%Q"),
                tag("end:except.once")
            ),
        );
        assert!(inspect(&document).is_err());
    }
}
