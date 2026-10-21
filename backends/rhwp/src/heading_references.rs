// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Euisoon Ahn
//! Resolve heading references before note extraction and width measurement.

use std::collections::{BTreeMap, BTreeSet};

use md2hwp_core::{
    ValidatedDocument, ValidationLimits,
    ir::{
        Block, CrossReferenceKind, FootnoteBlock, Inline, ListItem, ListItemBlock, SourceParagraph,
    },
    validate,
};
use rhwp::model::paragraph::{ColumnBreakType, Paragraph};

use crate::{
    heading_plan::HeadingPlan,
    model_tools::map_nested_containers,
    template::{require_plain, tag},
};

pub(crate) struct Samples {
    texts: Vec<String>,
}

impl Samples {
    pub(crate) fn read(roots: &[Paragraph]) -> Result<Self, String> {
        let find = |name: &str| -> Result<usize, String> {
            let indices: Vec<_> = roots
                .iter()
                .enumerate()
                .filter(|(_, p)| p.text == tag(name))
                .map(|(i, _)| i)
                .collect();
            match indices.as_slice() {
                [index] => Ok(*index),
                _ => Err(format!(
                    "Expected one root {name}; regenerate the template or explicitly adopt all six heading reference blocks"
                )),
            }
        };
        let begin = find("begin:template")?;
        let end = find("end:template")?;
        let mut accepted = BTreeSet::new();
        let mut texts = Vec::new();
        for level in 1..=6 {
            let role = format!("ref.heading{level}.number");
            let first = find(&format!("begin:{role}"))?;
            let last = find(&format!("end:{role}"))?;
            if first <= begin || last >= end || last != first + 2 {
                return Err(format!(
                    "{role} requires exactly one root sample paragraph inside template definitions"
                ));
            }
            for (index, p) in roots.iter().enumerate().take(last + 1).skip(first) {
                require_plain(p)?;
                if p.column_type != ColumnBreakType::None || p.raw_break_type != 0 {
                    return Err(format!(
                        "{role} boundaries and sample cannot contain page/column breaks"
                    ));
                }
                accepted.insert(index);
            }
            let text = &roots[first + 1].text;
            if !text.contains(&tag(&format!("num:heading{level}"))) {
                return Err(format!("{role} requires num:heading{level}"));
            }
            let mut remainder = text.clone();
            for ancestor in 1..=level {
                remainder = remainder.replace(&tag(&format!("num:heading{ancestor}")), "");
            }
            if remainder.contains("{{md2hwp:") {
                return Err(format!(
                    "{role} permits only its own and ancestor heading number tags"
                ));
            }
            texts.push(text.clone());
        }
        // A misplaced declaration in a native container must not be mistaken for
        // an editable reference prototype or silently discarded.
        fn check(p: &mut Paragraph, accepted: bool) -> Result<(), String> {
            if !accepted && p.text.contains("ref.heading") {
                let mut remaining = p.text.as_str();
                while let Some((_, suffix)) = remaining.split_once("{{md2hwp:") {
                    let name = suffix.split_once("}}").map_or(suffix, |(name, _)| name);
                    if name.split(':').any(|part| part.starts_with("ref.heading")) {
                        return Err("Heading reference declarations are allowed only in their six root prototypes; replace old ref.heading.number declarations".into());
                    }
                    remaining = suffix;
                }
            }
            map_nested_containers(p, &mut |children| {
                for child in children {
                    check(child, false)?;
                }
                Ok(())
            })
        }
        for (index, p) in roots.iter().enumerate() {
            check(&mut p.clone(), accepted.contains(&index))?;
        }
        Ok(Self { texts })
    }

    pub(crate) fn text(&self, level: u8, numbers: &[u32; 6]) -> Result<String, String> {
        let mut text = self
            .texts
            .get(usize::from(level.saturating_sub(1)))
            .filter(|_| (1..=6).contains(&level))
            .ok_or("Missing heading reference prototype")?
            .clone();
        for ancestor in 1..=level {
            text = text.replace(
                &tag(&format!("num:heading{ancestor}")),
                &numbers[usize::from(ancestor - 1)].to_string(),
            );
        }
        Ok(text)
    }

    pub(crate) fn prepare(&self, ir: &ValidatedDocument) -> Result<ValidatedDocument, String> {
        let plan = HeadingPlan::new(ir)?;
        let mut targets = BTreeMap::new();
        for (operation, block) in ir.as_document().blocks.iter().enumerate() {
            if let Block::Heading { id: Some(id), .. } = block {
                let context = plan
                    .context(operation)
                    .ok_or("Missing heading counter context")?;
                targets.insert(id.clone(), self.text(context.level, &context.numbers)?);
            }
        }
        let mut document = ir.as_document().clone();
        for block in &mut document.blocks {
            match block {
                Block::Paragraph { inlines } | Block::Heading { inlines, .. } => {
                    bind(inlines, &targets)?
                }
                Block::List { items, .. } => bind_items(items, &targets)?,
                Block::VerbatimBlock { source, .. } => bind_sources(source, &targets)?,
                Block::Figure {
                    image,
                    caption,
                    source,
                    ..
                } => {
                    bind(&mut image.alt, &targets)?;
                    bind(caption, &targets)?;
                    bind_sources(source, &targets)?;
                }
                Block::Table {
                    header,
                    rows,
                    caption,
                    source,
                    ..
                } => {
                    for cell in header.iter_mut().chain(rows.iter_mut().flatten()) {
                        bind(cell, &targets)?;
                    }
                    if let Some(caption) = caption {
                        bind(caption, &targets)?;
                    }
                    bind_sources(source, &targets)?;
                }
            }
        }
        validate(document, &ValidationLimits::default()).map_err(|e| e.to_string())
    }
}

fn bind_sources(
    source: &mut Option<Vec<SourceParagraph>>,
    targets: &BTreeMap<String, String>,
) -> Result<(), String> {
    for paragraph in source.iter_mut().flatten() {
        bind(&mut paragraph.inlines, targets)?;
    }
    Ok(())
}

fn bind_items(items: &mut [ListItem], targets: &BTreeMap<String, String>) -> Result<(), String> {
    for item in items {
        for block in &mut item.blocks {
            match block {
                ListItemBlock::Paragraph { inlines } => bind(inlines, targets)?,
                ListItemBlock::List { items, .. } => bind_items(items, targets)?,
            }
        }
    }
    Ok(())
}

fn bind(inlines: &mut Vec<Inline>, targets: &BTreeMap<String, String>) -> Result<(), String> {
    let mut result = Vec::new();
    for mut inline in std::mem::take(inlines) {
        match &mut inline {
            Inline::CrossReference {
                kind: CrossReferenceKind::HeadingNumber,
                target,
            } => {
                let text = targets
                    .get(target)
                    .ok_or_else(|| format!("Missing heading reference target '{target}'"))?;
                for (index, word) in text.split(' ').enumerate() {
                    if index > 0 {
                        result.push(Inline::Space);
                    }
                    if !word.is_empty() {
                        result.push(Inline::Text { value: word.into() });
                    }
                }
                continue;
            }
            Inline::Strong { inlines }
            | Inline::Emph { inlines }
            | Inline::Link { inlines, .. } => bind(inlines, targets)?,
            Inline::Footnote { blocks } => {
                for FootnoteBlock::Paragraph { inlines } in blocks {
                    bind(inlines, targets)?;
                }
            }
            _ => {}
        }
        result.push(inline);
    }
    *inlines = result;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use md2hwp_core::{
        Document,
        ir::{IR_VERSION, Metadata, MetadataValue, SCHEMA_NAME},
    };

    fn paragraph(text: &str) -> Paragraph {
        let mut p = Paragraph::new_empty();
        p.set_single_char_shape(0);
        p.insert_text_at(0, text);
        p
    }

    fn prototypes() -> Vec<Paragraph> {
        let mut roots = vec![paragraph(&tag("begin:template"))];
        for level in 1..=6 {
            roots.push(paragraph(&tag(&format!("begin:ref.heading{level}.number"))));
            let numbers = (1..=level)
                .map(|n| tag(&format!("num:heading{n}")))
                .collect::<Vec<_>>()
                .join(".");
            roots.push(paragraph(&format!("제 {numbers} 절")));
            roots.push(paragraph(&tag(&format!("end:ref.heading{level}.number"))));
        }
        roots.push(paragraph(&tag("end:template")));
        roots
    }

    fn reference() -> Inline {
        Inline::CrossReference {
            kind: CrossReferenceKind::HeadingNumber,
            target: "세부".into(),
        }
    }

    #[test]
    fn forward_references_preserve_wrappers_and_count_idless_skipped_headings() {
        let heading = |level, id: Option<&str>| Block::Heading {
            id: id.map(str::to_owned),
            level,
            inlines: vec![Inline::Text {
                value: "제목".into(),
            }],
        };
        let mut metadata = Metadata::default();
        metadata.0.insert(
            "md2hwp-heading1-start".into(),
            MetadataValue::Text("7".into()),
        );
        let ir = validate(
            Document {
                schema: SCHEMA_NAME.into(),
                ir_version: IR_VERSION.into(),
                metadata,
                blocks: vec![
                    Block::Paragraph {
                        inlines: vec![
                            Inline::Strong {
                                inlines: vec![Inline::Emph {
                                    inlines: vec![Inline::Link {
                                        target: "https://example.org".into(),
                                        title: None,
                                        inlines: vec![reference()],
                                    }],
                                }],
                            },
                            Inline::Footnote {
                                blocks: vec![FootnoteBlock::Paragraph {
                                    inlines: vec![reference()],
                                }],
                            },
                        ],
                    },
                    heading(1, None),
                    heading(1, None),
                    heading(3, Some("세부")),
                ],
            },
            &ValidationLimits::default(),
        )
        .unwrap();
        let prepared = Samples::read(&prototypes()).unwrap().prepare(&ir).unwrap();
        let expected = vec![
            Inline::Text {
                value: "제".into()
            },
            Inline::Space,
            Inline::Text {
                value: "8.0.1".into(),
            },
            Inline::Space,
            Inline::Text {
                value: "절".into()
            },
        ];
        let Block::Paragraph { inlines } = &prepared.as_document().blocks[0] else {
            panic!()
        };
        let Inline::Strong { inlines: strong } = &inlines[0] else {
            panic!()
        };
        let Inline::Emph { inlines: emph } = &strong[0] else {
            panic!()
        };
        let Inline::Link { inlines: label, .. } = &emph[0] else {
            panic!()
        };
        assert_eq!(label, &expected);
        let Inline::Footnote { blocks } = &inlines[1] else {
            panic!()
        };
        let FootnoteBlock::Paragraph { inlines: body } = &blocks[0];
        assert_eq!(body, &expected);
        assert_eq!(
            ir.as_document().blocks[0],
            Block::Paragraph {
                inlines: vec![
                    Inline::Strong {
                        inlines: vec![Inline::Emph {
                            inlines: vec![Inline::Link {
                                target: "https://example.org".into(),
                                title: None,
                                inlines: vec![reference()],
                            }]
                        }]
                    },
                    Inline::Footnote {
                        blocks: vec![FootnoteBlock::Paragraph {
                            inlines: vec![reference()]
                        }]
                    }
                ]
            }
        );
    }

    #[test]
    fn invalid_prototypes_fail_even_without_any_manuscript_references() {
        let roots = prototypes();
        assert!(Samples::read(&roots).is_ok());
        for replacement in [
            "literal",
            "{{md2hwp:num:heading1}} {{md2hwp:num:heading2}}",
            "{{md2hwp:num:heading1}} {{md2hwp:meta:title}}",
            "{{md2hwp:num:heading1}} {{md2hwp:broken",
        ] {
            let mut bad = roots.clone();
            bad[2] = paragraph(replacement);
            assert!(Samples::read(&bad).is_err(), "{replacement}");
        }
        let mut bad = roots.clone();
        bad[2].column_type = ColumnBreakType::Page;
        assert!(Samples::read(&bad).is_err());
        let mut bad = roots.clone();
        bad.push(paragraph(&tag("slot:ref.heading1.number")));
        assert!(Samples::read(&bad).is_err());
        let mut bad = roots.clone();
        bad.push(paragraph(&tag("begin:ref.heading.number")));
        assert!(Samples::read(&bad).is_err());
    }

    #[test]
    fn source_literals_and_other_reference_kinds_are_unchanged() {
        let mut inlines = vec![
            Inline::Text {
                value: "{{md2hwp:num:heading1}}".into(),
            },
            Inline::CrossReference {
                kind: CrossReferenceKind::FigureNumber,
                target: "그림".into(),
            },
            reference(),
        ];
        let original = inlines[..2].to_vec();
        bind(
            &mut inlines,
            &BTreeMap::from([("세부".into(), " 1  절 ".into())]),
        )
        .unwrap();
        assert_eq!(&inlines[..2], original.as_slice());
        assert_eq!(
            &inlines[2..],
            &[
                Inline::Space,
                Inline::Text { value: "1".into() },
                Inline::Space,
                Inline::Space,
                Inline::Text {
                    value: "절".into()
                },
                Inline::Space
            ]
        );
    }
}
