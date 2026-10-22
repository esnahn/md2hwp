// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Euisoon Ahn
//! Inspect metadata before disposable definitions or unused heading branches disappear.

use std::collections::BTreeSet;

use md2hwp_core::{
    ValidationLimits,
    ir::{Document as IrDocument, IR_VERSION, Metadata, MetadataValue, SCHEMA_NAME},
    validate,
};
use rhwp::model::{document::Document, paragraph::Paragraph};

const PREFIX: &str = "{{md2hwp:meta:";
const PLACEHOLDER: &str = "MD2HWP_METADATA_VALIDATION";
const MAX_DEPTH: usize = 128;

pub(crate) struct Inspection {
    pub document: Document,
    pub keys: Vec<String>,
    pub requires_recognized_date: bool,
}

pub(crate) fn inspect(source: &Document) -> Result<Inspection, String> {
    let mut document = source.clone();
    let mut keys = BTreeSet::new();
    let mut requires_recognized_date = false;
    for section in &mut document.sections {
        inspect_paragraphs(
            &mut section.paragraphs,
            0,
            &mut keys,
            &mut requires_recognized_date,
        )?;
        for master in &mut section.section_def.master_pages {
            inspect_paragraphs(
                &mut master.paragraphs,
                0,
                &mut keys,
                &mut requires_recognized_date,
            )?;
        }
    }
    validate_keys(&keys)?;
    let mut keys: Vec<_> = keys
        .into_iter()
        .map(|key| {
            if key == "date-meta" {
                "date".into()
            } else {
                key
            }
        })
        .collect();
    // Match StringComparer.Ordinal, including supplementary Unicode in custom keys.
    keys.sort_by(|left, right| left.encode_utf16().cmp(right.encode_utf16()));
    keys.dedup();
    Ok(Inspection {
        document,
        keys,
        requires_recognized_date,
    })
}

fn inspect_paragraphs(
    paragraphs: &mut [Paragraph],
    depth: usize,
    keys: &mut BTreeSet<String>,
    requires_recognized_date: &mut bool,
) -> Result<(), String> {
    if depth > MAX_DEPTH {
        return Err("Template metadata container nesting exceeds 128".into());
    }
    for paragraph in paragraphs {
        inspect_paragraph(paragraph, keys, requires_recognized_date)?;
        crate::model_tools::map_nested_containers(paragraph, &mut |children| {
            inspect_paragraphs(children, depth + 1, keys, requires_recognized_date)
        })?;
    }
    Ok(())
}

fn inspect_paragraph(
    paragraph: &mut Paragraph,
    keys: &mut BTreeSet<String>,
    requires_recognized_date: &mut bool,
) -> Result<(), String> {
    while let Some(start) = paragraph.text.find(PREFIX) {
        let end = paragraph.text[start + PREFIX.len()..]
            .find("}}")
            .map(|offset| start + PREFIX.len() + offset)
            .ok_or("Malformed metadata tag: missing closing boundary")?;
        let name = &paragraph.text[start + PREFIX.len()..end];
        if name.is_empty() || name.contains(['{', '}']) {
            return Err("Malformed metadata tag".into());
        }
        let (key, format) = match name.split_once(":fmt:") {
            Some((key, format)) => (key, Some(format)),
            None => (name, None),
        };
        if !matches!(
            key,
            "title" | "subtitle" | "author" | "date" | "date-meta" | "publisher"
        ) && !(key.starts_with("md2hwp-") && key.len() > 7)
        {
            return Err(format!("Unsupported metadata key: {key}"));
        }
        if key.chars().any(char::is_control) {
            return Err("Metadata key contains a control character".into());
        }
        if let Some(format) = format {
            if key != "date-meta" {
                return Err("fmt is supported only for date-meta".into());
            }
            crate::template::format_date("2000-01-01", format)?;
        }
        keys.insert(key.to_owned());
        *requires_recognized_date |= key == "date-meta";
        let token = paragraph.text[start..end + 2].to_owned();
        if paragraph.char_offsets.len() < paragraph.text.chars().count() {
            return Err("Metadata paragraph has incomplete character offsets".into());
        }
        crate::template::replace(paragraph, &token, PLACEHOLDER)?;
    }
    Ok(())
}

fn validate_keys(keys: &BTreeSet<String>) -> Result<(), String> {
    // Reuse the core's NFC and metadata-key contract without adding a normalizer
    // dependency or pretending to render an artificial manuscript.
    let mut metadata = Metadata::default();
    for key in keys
        .iter()
        .map(String::as_str)
        .chain(["date", "date-meta", "md2hwp-heading1-start"])
    {
        let value = match key {
            "date" | "date-meta" => "2000-01-01",
            "md2hwp-heading1-start" => "1",
            _ => "metadata validation",
        };
        metadata
            .0
            .insert(key.into(), MetadataValue::Text(value.into()));
    }
    validate(
        IrDocument {
            schema: SCHEMA_NAME.into(),
            ir_version: IR_VERSION.into(),
            metadata,
            blocks: Vec::new(),
        },
        &ValidationLimits::default(),
    )
    .map(|_| ())
    .map_err(|error| format!("Invalid template metadata key: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rhwp::model::{
        control::Control,
        header_footer::MasterPage,
        table::{Cell, Table},
    };

    fn paragraph(text: &str) -> Paragraph {
        let mut paragraph = Paragraph::new_empty();
        paragraph.set_single_char_shape(0);
        paragraph.insert_text_at(0, text);
        paragraph
    }

    fn document(text: &str) -> Document {
        let mut document = rhwp::DocumentCore::new_empty().document().clone();
        document.sections.push(Default::default());
        document.sections[0].paragraphs = vec![paragraph(text)];
        document
    }

    #[test]
    fn reports_yaml_requirements_and_preserves_source_and_other_tags() {
        let source = document(
            "{{md2hwp:meta:title}} {{md2hwp:meta:date-meta:fmt:%Y년 %-m월}} {{md2hwp:meta:date}} {{md2hwp:meta:title}} {{md2hwp:body}}",
        );
        let before = source.clone();
        let result = inspect(&source).unwrap();
        assert_eq!(result.keys, ["date", "title"]);
        assert!(result.requires_recognized_date);
        assert!(rhwp::serializer::hwpx::roundtrip::diff_documents(&before, &source).is_empty());
        let text = &result.document.sections[0].paragraphs[0].text;
        assert_eq!(text.matches(PLACEHOLDER).count(), 4);
        assert!(text.ends_with("{{md2hwp:body}}"));
    }

    #[test]
    fn accepts_custom_colons_and_uses_utf16_ordinal_key_order() {
        let source = document(
            "{{md2hwp:meta:md2hwp-name:detail}} {{md2hwp:meta:md2hwp-😀}} {{md2hwp:meta:md2hwp-\u{e000}}} {{md2hwp:meta:md2hwp-heading1-start}}",
        );
        let result = inspect(&source).unwrap();
        assert_eq!(
            result.keys,
            [
                "md2hwp-heading1-start",
                "md2hwp-name:detail",
                "md2hwp-😀",
                "md2hwp-\u{e000}"
            ]
        );
        assert!(!result.requires_recognized_date);
    }

    #[test]
    fn rejects_invalid_keys_formats_and_malformed_metadata() {
        for name in [
            "unknown",
            "md2hwp-",
            "title:detail",
            "title:fmt:%Y",
            "date-meta:fmt:%Q",
            "date-meta:fmt:",
            "date-meta:fmt:%",
            "date-meta:fmt:%-Y",
            "date-meta:fmt:line\nbreak",
            "md2hwp-e\u{301}",
            "md2hwp-control\tkey",
            "",
            "title{suffix",
        ] {
            let source = document(&format!("{{{{md2hwp:meta:{name}}}}}"));
            assert!(inspect(&source).is_err(), "accepted {name:?}");
        }
        assert!(inspect(&document("{{md2hwp:meta:title")).is_err());
    }

    #[test]
    fn accepts_supported_date_formats_and_split_character_runs() {
        let mut source = document("앞 {{md2hwp:meta:date-meta:fmt:%Y %y %m %-m %d %-d %F %%}} 뒤");
        source.sections[0].paragraphs[0].apply_char_shape_range(2, 9, 7);
        source.sections[0].paragraphs[0].apply_char_shape_range(9, 15, 8);
        let result = inspect(&source).unwrap();
        let replaced = &result.document.sections[0].paragraphs[0];
        assert_eq!(replaced.text, format!("앞 {PLACEHOLDER} 뒤"));
        assert_eq!(replaced.char_shape_id_at(2), Some(7));
    }

    #[test]
    fn rejects_tag_crossing_native_character_offset_gap() {
        let mut source = document("{{md2hwp:meta:title}}");
        for offset in &mut source.sections[0].paragraphs[0].char_offsets[10..] {
            *offset += 8;
        }
        let before = source.clone();
        assert!(inspect(&source).is_err());
        assert!(rhwp::serializer::hwpx::roundtrip::diff_documents(&before, &source).is_empty());
    }

    #[test]
    fn visits_nested_cells_and_section_master_pages() {
        let mut source = document("{{md2hwp:begin:template}}");
        let mut table = Table::default();
        table.cells.push(Cell {
            paragraphs: vec![paragraph("{{md2hwp:meta:publisher}}")],
            ..Default::default()
        });
        source.sections[0].paragraphs[0]
            .controls
            .push(Control::Table(Box::new(table)));
        source.sections[0]
            .section_def
            .master_pages
            .push(MasterPage {
                paragraphs: vec![paragraph("{{md2hwp:meta:author}}")],
                ..Default::default()
            });
        let before = source.clone();
        let result = inspect(&source).unwrap();
        assert_eq!(result.keys, ["author", "publisher"]);
        assert!(rhwp::serializer::hwpx::roundtrip::diff_documents(&before, &source).is_empty());
        source.sections[0].section_def.master_pages[0].paragraphs[0] =
            paragraph("{{md2hwp:meta:date-meta:fmt:%Q}}");
        assert!(inspect(&source).is_err());
    }

    #[test]
    fn bounds_recursive_container_inspection() {
        let mut source = document("outer");
        let mut nested = paragraph("{{md2hwp:meta:title}}");
        for _ in 0..=MAX_DEPTH {
            let mut table = Table::default();
            table.cells.push(Cell {
                paragraphs: vec![nested],
                ..Default::default()
            });
            nested = paragraph("container");
            nested.controls.push(Control::Table(Box::new(table)));
        }
        source.sections[0].paragraphs = vec![nested];
        let error = match inspect(&source) {
            Ok(_) => panic!("expected metadata inspection failure"),
            Err(error) => error,
        };
        assert!(error.contains("nesting"));
    }
}
