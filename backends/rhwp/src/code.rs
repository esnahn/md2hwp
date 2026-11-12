// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Euisoon Ahn
//! Code boxes retain the template's native table, cell and caption formats.

use md2hwp_core::ir::SourceParagraph;
use rhwp::model::{control::Control, document::DocInfo, paragraph::Paragraph, style::HeadType};

const CONTENT: &str = "{{md2hwp:slot:code.content}}";
const TITLE: &str = "{{md2hwp:slot:code.title}}";
const SOURCE: &str = "{{md2hwp:slot:code.source}}";
const PREFIX: &str = "{{md2hwp:slot:code.source.prefix}}";

pub(crate) struct Prototype {
    anchor: Paragraph,
    before: Vec<Paragraph>,
    after: Vec<Paragraph>,
    content: Paragraph,
    title: Option<Paragraph>,
    source: Paragraph,
}

impl Prototype {
    pub(crate) fn read(
        roots: &[Paragraph],
        _body: &Paragraph,
        info: &DocInfo,
    ) -> Result<Self, String> {
        let boundary = |name: &str| -> Result<usize, String> {
            let token = crate::template::tag(name);
            let found: Vec<_> = roots
                .iter()
                .enumerate()
                .filter(|(_, p)| p.text == token)
                .collect();
            if found.len() != 1 {
                return Err(format!("Code requires exactly one plain root {token}"));
            }
            plain(found[0].1, info)?;
            Ok(found[0].0)
        };
        let begin = boundary("begin:code")?;
        let end = boundary("end:code")?;
        if end <= begin + 1 || end - begin > 65 {
            return Err(
                "Code range requires one sample box with 0–63 empty paragraphs before/after it"
                    .into(),
            );
        }
        let range = &roots[begin + 1..end];
        let anchors: Vec<_> = range
            .iter()
            .enumerate()
            .filter(|(_, p)| !p.controls.is_empty())
            .collect();
        if anchors.len() != 1 {
            return Err("Code requires one native single-cell table anchor and only empty spacing paragraphs".into());
        }
        let (index, anchor) = anchors[0];
        for p in range[..index].iter().chain(&range[index + 1..]) {
            plain(p, info)?;
            if !p.text.trim().is_empty() || shape(p, info)?.head_type != HeadType::None {
                return Err(
                    "Code spacing must be empty plain paragraphs without native numbering".into(),
                );
            }
        }
        if !anchor.text.trim().is_empty()
            || anchor.controls.len() != 1
            || !anchor.field_ranges.is_empty()
            || !anchor.orphan_field_ends.is_empty()
            || !anchor.range_tags.is_empty()
            || !anchor.title_marks.is_empty()
            || !anchor.markpen_marks.is_empty()
            || anchor.text.chars().any(|c| c.is_control() && c != '\n')
        {
            return Err(
                "Code anchor requires only its native table and optional native line break".into(),
            );
        }
        // code.anchor is independently editable; retain its own paragraph format.
        shape(anchor, info)?;
        let Control::Table(table) = &anchor.controls[0] else {
            return Err("Code requires a native table".into());
        };
        if table.row_count != 1
            || table.col_count != 1
            || table.cells.len() != 1
            || table.cells[0].row != 0
            || table.cells[0].col != 0
            || table.cells[0].row_span != 1
            || table.cells[0].col_span != 1
            || !table.common.treat_as_char
        {
            return Err("Code requires one single-cell table with TreatAsChar enabled".into());
        }
        let paragraphs = &table.cells[0].paragraphs;
        if !(1..=2).contains(&paragraphs.len())
            || paragraphs.last().map(|p| p.text.as_str()) != Some(CONTENT)
            || (paragraphs.len() == 2 && paragraphs[0].text != TITLE)
        {
            return Err(
                "Code cell requires an optional standalone title slot followed by one content slot"
                    .into(),
            );
        }
        for p in paragraphs {
            plain(p, info)?;
        }
        let caption = table
            .caption
            .as_ref()
            .ok_or("Code requires its native source caption")?;
        if caption.paragraphs.len() != 1 {
            return Err("Code native caption requires exactly one source paragraph".into());
        }
        let source = &caption.paragraphs[0];
        plain(source, info)?;
        if source.text.matches(PREFIX).count() != 1
            || source.text.matches(SOURCE).count() != 1
            || !source.text.contains(&format!("{PREFIX}: {SOURCE}"))
            || source
                .text
                .replace(PREFIX, "")
                .replace(SOURCE, "")
                .contains("{{md2hwp:")
        {
            return Err("Code source requires slot:code.source.prefix followed by ': ' and slot:code.source in one plain native caption paragraph".into());
        }
        // Validate each slot globally, including unused prototypes and nested containers.
        let mut counts = [0; 4];
        fn count(p: &Paragraph, counts: &mut [usize; 4]) -> Result<(), String> {
            for (i, tag) in [CONTENT, TITLE, SOURCE, PREFIX].iter().enumerate() {
                counts[i] += p.text.matches(tag).count();
            }
            let mut copy = p.clone();
            crate::model_tools::map_nested_containers(&mut copy, &mut |ps| {
                for p in ps {
                    count(p, counts)?;
                }
                Ok(())
            })
        }
        for p in roots {
            count(p, &mut counts)?;
        }
        if counts != [1, usize::from(paragraphs.len() == 2), 1, 1] {
            return Err("Duplicate or misplaced code slots in template".into());
        }
        Ok(Self {
            anchor: anchor.clone(),
            before: range[..index].to_vec(),
            after: range[index + 1..].to_vec(),
            content: paragraphs.last().unwrap().clone(),
            title: (paragraphs.len() == 2).then(|| paragraphs[0].clone()),
            source: source.clone(),
        })
    }

    pub(crate) fn render(
        &self,
        lines: &[String],
        source: Option<&[SourceParagraph]>,
        info: &mut DocInfo,
    ) -> Result<Vec<Paragraph>, String> {
        if lines.is_empty() || lines.iter().any(|s| s.contains(['\r', '\n'])) {
            return Err("Code requires at least one CR/LF-free logical line".into());
        }
        let mut paragraphs = Vec::new();
        let mut start = 0;
        if let Some(sample) = &self.title
            && let Some(title) = lines[0].strip_prefix("제목:")
        {
            let title = title.strip_prefix(' ').unwrap_or(title);
            if title.trim().is_empty() {
                return Err("Code 제목: requires a nonempty title".into());
            }
            paragraphs.push(fill(sample, TITLE, title)?);
            start = 1;
            while start < lines.len() && lines[start].trim().is_empty() {
                start += 1;
            }
        }
        if lines.len() - start > usize::from(u16::MAX) - paragraphs.len() {
            return Err("Code cell exceeds the native 65535 paragraph limit".into());
        }
        for line in &lines[start..] {
            paragraphs.push(fill(&self.content, CONTENT, line)?);
        }
        let mut anchor = self.anchor.clone();
        let Control::Table(table) = &mut anchor.controls[0] else {
            unreachable!()
        };
        table.cells[0].paragraphs = paragraphs;
        if let Some(sources) = source {
            if sources.is_empty() {
                return Err("Code source paragraphs must not be empty".into());
            }
            if sources.len() > usize::from(u16::MAX) {
                return Err("Code caption exceeds the native 65535 paragraph limit".into());
            }
            let mut generated = Vec::new();
            for source in sources {
                crate::template::check_inlines(&source.inlines)?;
                let mut p = self.source.clone();
                let byte = p.text.find(SOURCE).unwrap();
                let base = p
                    .char_shape_id_at(p.text[..byte].chars().count())
                    .ok_or("Code source has no character format")?;
                let mut spans = Vec::new();
                crate::template::flatten(&source.inlines, &mut spans);
                // Content first keeps literal tag-shaped manuscript text opaque.
                crate::template::replace_rich(&mut p, SOURCE, &spans, base, info)?;
                crate::template::replace(&mut p, PREFIX, &source.prefix)?;
                generated.push(p);
            }
            table.caption.as_mut().unwrap().paragraphs = generated;
        } else {
            table.caption = None;
        }
        crate::template::discard_stale_layout(&mut anchor);
        let mut output = self.before.clone();
        output.push(anchor);
        output.extend(self.after.clone());
        Ok(output)
    }
}

fn shape<'a>(
    p: &Paragraph,
    info: &'a DocInfo,
) -> Result<&'a rhwp::model::style::ParaShape, String> {
    info.para_shapes
        .get(usize::from(p.para_shape_id))
        .ok_or("Code references an absent paragraph format".into())
}

fn plain(p: &Paragraph, info: &DocInfo) -> Result<(), String> {
    crate::template::require_plain(p)?;
    if !p.orphan_field_ends.is_empty() {
        return Err("Code samples cannot contain field ends".into());
    }
    shape(p, info)?;
    if p.char_shapes.is_empty()
        || p.char_shapes
            .iter()
            .any(|s| info.char_shapes.get(s.char_shape_id as usize).is_none())
    {
        return Err("Code sample references an absent character format".into());
    }
    let mut copy = p.clone();
    crate::template::replace(&mut copy, &p.text, "")?;
    Ok(())
}

fn fill(sample: &Paragraph, slot: &str, text: &str) -> Result<Paragraph, String> {
    let mut p = sample.clone();
    let byte = p
        .text
        .find(slot)
        .ok_or("Code substitution target disappeared")?;
    let format = p
        .char_shape_id_at(p.text[..byte].chars().count())
        .ok_or("Code slot has no character format")?;
    crate::template::replace(&mut p, slot, text)?;
    if p.text.is_empty() {
        // A zero-length apply_char_shape_range cannot bind a format. Empty raw
        // lines still inherit the slot's first character, including its font size.
        p.set_single_char_shape(format);
    }
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use md2hwp_core::ir::Inline;
    use rhwp::model::{
        shape::Caption,
        style::{CharShape, ParaShape},
        table::{Cell, Table},
    };

    fn p(text: &str) -> Paragraph {
        let mut p = Paragraph::new_empty();
        p.set_single_char_shape(0);
        p.insert_text_at(0, text);
        p
    }

    fn fixture(title: bool) -> (Vec<Paragraph>, Paragraph, DocInfo) {
        let mut cell = Cell {
            row_span: 1,
            col_span: 1,
            width: 12345,
            height: 678,
            paragraphs: Vec::new(),
            ..Default::default()
        };
        if title {
            cell.paragraphs.push(p(TITLE));
        }
        cell.paragraphs.push(p(CONTENT));
        let mut table = Table {
            row_count: 1,
            col_count: 1,
            cells: vec![cell],
            caption: Some(Caption {
                spacing: 333,
                paragraphs: vec![p(&format!("({PREFIX}: {SOURCE})"))],
                ..Default::default()
            }),
            ..Default::default()
        };
        table.common.treat_as_char = true;
        let mut anchor = p("\n");
        anchor.controls.push(Control::Table(Box::new(table)));
        let mut before = p("");
        before.raw_break_type = 4;
        (
            vec![
                p("{{md2hwp:begin:code}}"),
                before,
                anchor,
                p(""),
                p("{{md2hwp:end:code}}"),
            ],
            p("{{md2hwp:body}}"),
            DocInfo {
                para_shapes: vec![ParaShape::default()],
                char_shapes: vec![CharShape::default()],
                ..Default::default()
            },
        )
    }

    fn table(output: &[Paragraph]) -> &Table {
        let Control::Table(table) = &output[1].controls[0] else {
            panic!()
        };
        table
    }

    #[test]
    fn title_body_blank_tabs_literals_and_native_geometry_survive() {
        let (roots, body, mut info) = fixture(true);
        let prototype = Prototype::read(&roots, &body, &info).unwrap();
        let lines = [
            "제목: title",
            " ",
            "\t",
            "\t  x ",
            "",
            "{{md2hwp:slot:code.title}}",
            "제목: literal",
        ]
        .map(String::from);
        let output = prototype.render(&lines, None, &mut info).unwrap();
        assert_eq!(output[0].raw_break_type, 4);
        let table = table(&output);
        assert_eq!(table.cells[0].width, 12345);
        assert_eq!(table.cells[0].height, 678);
        assert!(table.caption.is_none());
        assert_eq!(
            table.cells[0]
                .paragraphs
                .iter()
                .map(|p| p.text.as_str())
                .collect::<Vec<_>>(),
            [
                "title",
                "\t  x ",
                "",
                "{{md2hwp:slot:code.title}}",
                "제목: literal"
            ]
        );
        assert_eq!(table.cells[0].paragraphs[1].char_offsets[1], 8);
        assert_eq!(roots[2].controls.len(), 1);
    }

    #[test]
    fn no_title_slot_preserves_all_raw_lines() {
        let (roots, body, mut info) = fixture(false);
        let prototype = Prototype::read(&roots, &body, &info).unwrap();
        let lines = ["제목: ", "", "  "].map(String::from);
        let output = prototype.render(&lines, None, &mut info).unwrap();
        assert_eq!(
            table(&output).cells[0]
                .paragraphs
                .iter()
                .map(|p| &p.text)
                .collect::<Vec<_>>(),
            lines.iter().collect::<Vec<_>>()
        );
    }

    #[test]
    fn sources_clone_caption_options_affixes_and_rich_formats_without_reparsing() {
        let (roots, body, mut info) = fixture(false);
        let prototype = Prototype::read(&roots, &body, &info).unwrap();
        let sources = vec![
            SourceParagraph {
                prefix: "출처".into(),
                inlines: vec![Inline::Strong {
                    inlines: vec![Inline::Text {
                        value: PREFIX.into(),
                    }],
                }],
            },
            SourceParagraph {
                prefix: "주1".into(),
                inlines: vec![Inline::Emph {
                    inlines: vec![Inline::Text {
                        value: "둘".into()
                    }],
                }],
            },
        ];
        let output = prototype
            .render(&["body".into()], Some(&sources), &mut info)
            .unwrap();
        let caption = table(&output).caption.as_ref().unwrap();
        assert_eq!(caption.spacing, 333);
        assert_eq!(caption.paragraphs[0].text, format!("(출처: {PREFIX})"));
        assert_eq!(caption.paragraphs[1].text, "(주1: 둘)");
        let id = caption.paragraphs[0].char_shape_id_at(5).unwrap();
        assert!(info.char_shapes[id as usize].bold);
        let id = caption.paragraphs[1].char_shape_id_at(5).unwrap();
        assert!(info.char_shapes[id as usize].italic);
    }

    #[test]
    fn rejects_bad_unused_samples_and_native_counter_overflow() {
        let (mut roots, body, mut info) = fixture(false);
        roots.push(p(TITLE));
        assert!(Prototype::read(&roots, &body, &info).is_err());
        roots.pop();
        let prototype = Prototype::read(&roots, &body, &info).unwrap();
        assert!(
            prototype
                .render(&vec![String::new(); 65536], None, &mut info)
                .unwrap_err()
                .contains("65535")
        );
        let Control::Table(table) = &mut roots[2].controls[0] else {
            panic!()
        };
        table.cells[0].paragraphs.push(p("extra"));
        assert!(Prototype::read(&roots, &body, &info).is_err());
    }

    #[test]
    fn source_rejects_unsupported_inlines_and_empty_titles_fail() {
        let (roots, body, mut info) = fixture(true);
        let prototype = Prototype::read(&roots, &body, &info).unwrap();
        assert!(
            prototype
                .render(&["제목: \t".into()], None, &mut info)
                .is_err()
        );
        let source = SourceParagraph {
            prefix: "출처".into(),
            inlines: vec![Inline::Footnote { blocks: Vec::new() }],
        };
        assert!(
            prototype
                .render(&["text".into()], Some(&[source]), &mut info)
                .is_err()
        );
    }

    #[test]
    fn anchor_preserves_independent_format_and_spacing_rejects_numbering() {
        let (mut roots, body, mut info) = fixture(false);
        info.para_shapes.push(info.para_shapes[0].clone());
        roots[2].para_shape_id = 1;
        info.para_shapes[1].margin_left = 1;
        let prototype = Prototype::read(&roots, &body, &info).unwrap();
        let output = prototype.render(&["body".into()], None, &mut info).unwrap();
        assert_eq!(output[1].para_shape_id, 1);
        assert_eq!(
            info.para_shapes[usize::from(output[1].para_shape_id)].margin_left,
            1
        );
        roots[1].para_shape_id = 1;
        info.para_shapes[1].head_type = HeadType::Number;
        assert!(Prototype::read(&roots, &body, &info).is_err());
    }

    #[test]
    fn empty_code_cell_paragraphs_store_no_line_records() {
        use rhwp::{
            DocumentCore,
            model::{document::Section, style::Font},
        };
        let (roots, body, mut info) = fixture(false);
        info.font_faces = vec![vec![Font::default()]; 7];
        let prototype = Prototype::read(&roots, &body, &info).unwrap();
        let output = prototype
            .render(
                &["".into(), "text".into(), "".into(), "".into()],
                None,
                &mut info,
            )
            .unwrap();
        let expected = table(&output).cells[0].paragraphs.clone();
        // Line layout is the reader's derived cache: generate none of it.
        for index in [0, 2, 3] {
            assert!(expected[index].line_segs.is_empty());
            assert!(expected[index].stored_text_partition_dirty);
        }
        let mut core = DocumentCore::new_empty();
        let mut doc = core.document().clone();
        doc.doc_info = info;
        doc.sections = vec![Section {
            paragraphs: output,
            ..Default::default()
        }];
        core.set_document(doc);
        let bytes = core.export_hwp_with_adapter_snapshot_with_report().unwrap();
        assert!(bytes.content_loss().is_empty());
        let stored = rhwp::parser::parse_hwp(bytes.bytes()).unwrap();
        let Control::Table(actual) = &stored.sections[0].paragraphs[1].controls[0] else {
            panic!()
        };
        for index in [0, 2, 3] {
            assert!(actual.cells[0].paragraphs[index].line_segs.is_empty());
        }
    }
}
