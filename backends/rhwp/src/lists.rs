// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Euisoon Ahn
//! Native list definitions belong to the marker samples; text belongs to body.

use md2hwp_core::ir::{Inline, ListItem, ListItemBlock, ListKind};
use rhwp::model::{
    document::DocInfo,
    paragraph::Paragraph,
    raw_provenance::{record_digest, record_raw_permitted},
    style::{Bullet, HeadType, Numbering, NumberingHead, ParaShape},
};

pub(crate) struct Lists {
    max_depth: usize,
    indent: i32,
    bullet: Bullet,
    ordered: Numbering,
    bullet_id: Option<u16>,
}

impl Lists {
    pub(crate) fn read(roots: &[Paragraph], info: &DocInfo) -> Result<Self, String> {
        let sample = |name: &str| -> Result<&Paragraph, String> {
            let tag = crate::template::tag(name);
            let matches: Vec<_> = roots.iter().filter(|p| p.text == tag).collect();
            if matches.len() != 1 {
                return Err(format!("Template requires exactly one plain root {tag}"));
            }
            crate::template::require_plain(matches[0])?;
            Ok(matches[0])
        };
        let setting = |name: &str, maximum: usize| -> Result<usize, String> {
            let prefix = format!("{{{{md2hwp:{name}:");
            let matches: Vec<_> = roots
                .iter()
                .filter(|p| p.text.starts_with(&prefix))
                .collect();
            if matches.len() != 1 {
                return Err(format!("Template requires exactly one {name} setting"));
            }
            crate::template::require_plain(matches[0])?;
            let text = matches[0]
                .text
                .strip_prefix(&prefix)
                .unwrap()
                .strip_suffix("}}")
                .ok_or("Invalid list setting")?;
            if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
                return Err(format!("Invalid positive integer {name}"));
            }
            let value: usize = text.parse().map_err(|_| format!("Invalid {name}"))?;
            if value == 0 || value > maximum {
                return Err(format!("Invalid {name}; expected 1..={maximum}"));
            }
            Ok(value)
        };
        let max_depth = setting("list.max-depth", 6)?;
        let indent = setting("list.indent-hwp", 10000)? as i32;
        let bullet_sample = sample("list.bullet")?;
        let ordered_sample = sample("list.ordered")?;
        let shape = |p: &Paragraph, expected: HeadType| -> Result<usize, String> {
            let shape = info
                .para_shapes
                .get(usize::from(p.para_shape_id))
                .ok_or("List sample references an absent paragraph format")?;
            if shape.head_type != expected {
                return Err("List sample requires its native bullet/number marker; regenerate or explicitly adopt the template declarations".into());
            }
            shape
                .numbering_id
                .checked_sub(1)
                .map(usize::from)
                .ok_or("List sample requires a native definition".into())
        };
        let bullet_index = shape(bullet_sample, HeadType::Bullet)?;
        let ordered_index = shape(ordered_sample, HeadType::Number)?;
        let mut bullet = info
            .bullets
            .get(bullet_index)
            .ok_or("Missing native bullet definition")?
            .clone();
        let mut ordered = info
            .numberings
            .get(ordered_index)
            .ok_or("Missing native numbering definition")?
            .clone();
        let seals = info.raw_provenance.as_ref().map(|p| &p.record_seals);
        if !record_raw_permitted(seals.map(|p| p.bullets.as_slice()), bullet_index, &bullet) {
            bullet.raw_data = None;
        }
        if !record_raw_permitted(
            seals.map(|p| p.numberings.as_slice()),
            ordered_index,
            &ordered,
        ) {
            ordered.raw_data = None;
        }
        let fallback = |p: &Paragraph| {
            p.char_shape_id_at(0)
                .ok_or("List sample has no character format".to_string())
        };
        if bullet.char_shape_id == u32::MAX {
            bullet.char_shape_id = fallback(bullet_sample)?;
            if let Some(raw) = &mut bullet.raw_data {
                patch(raw, 8, &bullet.char_shape_id.to_le_bytes())?;
            }
        }
        valid_character(info, bullet.char_shape_id)?;
        let offsets = number_offsets(&ordered)?;
        for (level, offset) in offsets.iter().copied().enumerate().take(7) {
            if level <= max_depth && ordered.level_formats[level].is_empty() {
                return Err(format!(
                    "list.ordered requires a native number format at level {}",
                    level + 1
                ));
            }
            if ordered.heads[level].char_shape_id == u32::MAX {
                ordered.heads[level].char_shape_id = fallback(ordered_sample)?;
                if let Some(raw) = &mut ordered.raw_data {
                    patch(
                        raw,
                        offset + 8,
                        &ordered.heads[level].char_shape_id.to_le_bytes(),
                    )?;
                }
            }
            valid_character(info, ordered.heads[level].char_shape_id)?;
        }
        Ok(Self {
            max_depth,
            indent,
            bullet,
            ordered,
            bullet_id: None,
        })
    }

    pub(crate) fn render(
        &mut self,
        kind: &ListKind,
        start: Option<u64>,
        items: &[ListItem],
        body: &Paragraph,
        info: &mut DocInfo,
    ) -> Result<Vec<Paragraph>, String> {
        let mut output = Vec::new();
        self.render_at(kind, start, items, body, info, 0, &mut output)?;
        Ok(output)
    }

    #[allow(clippy::too_many_arguments)]
    fn render_at(
        &mut self,
        kind: &ListKind,
        start: Option<u64>,
        items: &[ListItem],
        body: &Paragraph,
        info: &mut DocInfo,
        depth: usize,
        output: &mut Vec<Paragraph>,
    ) -> Result<(), String> {
        if depth > self.max_depth {
            return Err(format!(
                "List depth {depth} exceeds template list.max-depth {}",
                self.max_depth
            ));
        }
        let start = match (kind, start) {
            (ListKind::Ordered, Some(n)) if n > 0 => n,
            (ListKind::Bullet, None) => 1,
            _ => {
                return Err(
                    "Ordered lists require a positive start; bullet lists cannot carry start"
                        .into(),
                );
            }
        };
        let mut active = None;
        for (index, item) in items.iter().enumerate() {
            let number = start
                .checked_add(index as u64)
                .ok_or("List number overflow")?;
            let mut first = true;
            let mut continuation_left = None;
            for block in &item.blocks {
                match block {
                    ListItemBlock::List {
                        kind, start, items, ..
                    } => {
                        self.render_at(kind, *start, items, body, info, depth + 1, output)?;
                        active = None;
                    }
                    ListItemBlock::Paragraph { inlines } => {
                        crate::template::check_inlines(inlines)?;
                        let mut paragraph = body_paragraph(body, inlines, info)?;
                        let shape_index = usize::from(body.para_shape_id);
                        let mut shape = copy_shape(info, shape_index)?;
                        shape.margin_left = shape
                            .margin_left
                            .checked_add(
                                (depth as i32)
                                    .checked_mul(self.indent)
                                    .ok_or("List indentation overflow")?,
                            )
                            .ok_or("List indentation overflow")?;
                        if first {
                            let definition = match active {
                                Some(id) => id,
                                None => {
                                    let id = self.definition(kind, number, depth, info)?;
                                    active = Some(id);
                                    id
                                }
                            };
                            shape.head_type = if *kind == ListKind::Bullet {
                                HeadType::Bullet
                            } else {
                                HeadType::Number
                            };
                            shape.numbering_id = definition;
                            shape.para_level = depth as u8;
                            // Calculate only for items which actually need continuations.
                            if item
                                .blocks
                                .iter()
                                .filter(|b| matches!(b, ListItemBlock::Paragraph { .. }))
                                .count()
                                > 1
                            {
                                continuation_left =
                                    Some(self.continuation_margin(kind, depth, &shape, info)?);
                            }
                            first = false;
                        } else {
                            shape.head_type = HeadType::None;
                            shape.numbering_id = 0;
                            shape.para_level = 0;
                            shape.margin_left =
                                continuation_left.ok_or("Missing list marker for continuation")?;
                            shape.indent = 0;
                            active = None;
                        }
                        paragraph.para_shape_id = append_shape(info, shape)?;
                        output.push(paragraph);
                    }
                }
            }
            if first {
                return Err("Each list item requires a paragraph to carry its marker".into());
            }
        }
        Ok(())
    }

    fn definition(
        &mut self,
        kind: &ListKind,
        number: u64,
        depth: usize,
        info: &mut DocInfo,
    ) -> Result<u16, String> {
        if *kind == ListKind::Bullet {
            if let Some(id) = self.bullet_id {
                return Ok(id);
            }
            let id =
                u16::try_from(info.bullets.len() + 1).map_err(|_| "Too many bullet definitions")?;
            if let Some(seal) = &mut info.raw_provenance {
                seal.record_seals
                    .bullets
                    .resize(info.bullets.len(), [0; 32]);
                seal.record_seals.bullets.push(record_digest(&self.bullet));
            }
            info.bullets.push(self.bullet.clone());
            info.raw_stream_dirty = true;
            self.bullet_id = Some(id);
            return Ok(id);
        }
        let start = u16::try_from(number)
            .map_err(|_| "Native ordered list start exceeds 65535; use the Hancom backend")?;
        let id = u16::try_from(info.numberings.len() + 1)
            .map_err(|_| "Too many numbering definitions")?;
        let mut numbering = self.ordered.clone();
        let offsets = number_offsets(&numbering)?;
        if let Some(raw) = &mut numbering.raw_data {
            patch(raw, offsets[7], &start.to_le_bytes())?;
            patch(
                raw,
                offsets[7] + 2 + depth * 4,
                &u32::from(start).to_le_bytes(),
            )?;
        }
        numbering.start_number = start;
        numbering.level_start_numbers[depth] = u32::from(start);
        if let Some(seal) = &mut info.raw_provenance {
            seal.record_seals
                .numberings
                .resize(info.numberings.len(), [0; 32]);
            seal.record_seals.numberings.push(record_digest(&numbering));
        }
        info.numberings.push(numbering);
        info.raw_stream_dirty = true;
        Ok(id)
    }

    fn continuation_margin(
        &self,
        kind: &ListKind,
        depth: usize,
        shape: &ParaShape,
        info: &DocInfo,
    ) -> Result<i32, String> {
        let head = if *kind == ListKind::Bullet {
            if self.bullet.image_bullet != 0 {
                return Err(
                    "Image bullet continuation measurement is unsupported; use the Hancom backend"
                        .into(),
                );
            }
            NumberingHead {
                attr: self.bullet.attr,
                width_adjust: self.bullet.width_adjust,
                text_distance: self.bullet.text_distance,
                char_shape_id: self.bullet.char_shape_id,
                number_format: 0,
            }
        } else {
            self.ordered.heads[depth]
        };
        if head.attr & (1 << 2) != 0 {
            return Err("List continuations with UseInstWidth=true require native marker font measurement; use the Hancom backend or explicitly set the template list marker UseInstWidth=false".into());
        }
        let height = f64::from(
            info.char_shapes
                .get(head.char_shape_id as usize)
                .ok_or("Missing marker character format")?
                .base_size,
        );
        let offset = if head.attr & (1 << 4) != 0 {
            f64::from(head.text_distance)
        } else {
            height * f64::from(head.text_distance) / 100.0
        };
        // Margins are URC; width/offset/adjust are HWPUNIT. Match .NET's ties-to-even rounding.
        let distance = (2.0 * (height + offset + f64::from(head.width_adjust))).round_ties_even();
        let margin = i64::from(shape.margin_left) + i64::from(shape.indent) + distance as i64;
        i32::try_from(margin).map_err(|_| "List continuation indentation overflow".into())
    }
}

fn valid_character(info: &DocInfo, id: u32) -> Result<(), String> {
    info.char_shapes
        .get(id as usize)
        .map(|_| ())
        .ok_or("Native marker references an absent character format".into())
}

fn body_paragraph(
    body: &Paragraph,
    inlines: &[Inline],
    info: &mut DocInfo,
) -> Result<Paragraph, String> {
    let mut paragraph = body.clone();
    let base = paragraph
        .char_shape_id_at(0)
        .ok_or("Body sample has no character format")?;
    let mut spans = Vec::new();
    crate::template::flatten(inlines, false, false, &mut spans);
    crate::template::replace_rich(
        &mut paragraph,
        &crate::template::tag("body"),
        &spans,
        base,
        info,
    )?;
    Ok(paragraph)
}

fn patch(raw: &mut [u8], offset: usize, bytes: &[u8]) -> Result<(), String> {
    raw.get_mut(offset..offset + bytes.len())
        .ok_or("Truncated native list record")?
        .copy_from_slice(bytes);
    Ok(())
}

// Seven variable-length heads followed by global/per-level starts. Raw tails stay intact.
fn number_offsets(numbering: &Numbering) -> Result<[usize; 8], String> {
    let mut result = [0; 8];
    if let Some(raw) = &numbering.raw_data {
        let mut offset = 0;
        for slot in result.iter_mut().take(7) {
            *slot = offset;
            let bytes = raw
                .get(offset + 12..offset + 14)
                .ok_or("Truncated native numbering head")?;
            offset += 14 + usize::from(u16::from_le_bytes([bytes[0], bytes[1]])) * 2;
            if offset > raw.len() {
                return Err("Truncated native numbering format".into());
            }
        }
        result[7] = offset;
    }
    Ok(result)
}

fn copy_shape(info: &DocInfo, index: usize) -> Result<ParaShape, String> {
    let mut shape = info
        .para_shapes
        .get(index)
        .ok_or("Body sample references an absent paragraph format")?
        .clone();
    if !record_raw_permitted(
        info.raw_provenance
            .as_ref()
            .map(|s| s.record_seals.para_shapes.as_slice()),
        index,
        &shape,
    ) {
        shape.raw_data = None;
    }
    Ok(shape)
}

fn append_shape(info: &mut DocInfo, mut shape: ParaShape) -> Result<u16, String> {
    let id = u16::try_from(info.para_shapes.len()).map_err(|_| "Too many paragraph formats")?;
    shape.attr1 = (shape.attr1 & !((3 << 23) | (7 << 25)))
        | ((shape.head_type as u32) << 23)
        | (u32::from(shape.para_level) << 25);
    if let Some(raw) = &mut shape.raw_data {
        patch(raw, 0, &shape.attr1.to_le_bytes())?;
        patch(raw, 4, &shape.margin_left.to_le_bytes())?;
        patch(raw, 12, &shape.indent.to_le_bytes())?;
        patch(raw, 30, &shape.numbering_id.to_le_bytes())?;
        // Hancom reads the extended native level at payload 54..58. Updating
        // attr1 alone leaves the old body's level authoritative on reopen.
        // Older records omit this field; preserve their original length.
        if raw.len() >= 58 {
            patch(raw, 54, &u32::from(shape.para_level).to_le_bytes())?;
        }
    }
    if let Some(seal) = &mut info.raw_provenance {
        seal.record_seals
            .para_shapes
            .resize(info.para_shapes.len(), [0; 32]);
        seal.record_seals.para_shapes.push(record_digest(&shape));
    }
    info.para_shapes.push(shape);
    info.raw_stream_dirty = true;
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rhwp::model::{
        raw_provenance::{DocInfoRecordSeals, DocInfoSeal},
        style::CharShape,
    };

    fn p(text: &str, shape: u16) -> Paragraph {
        let mut p = Paragraph::new_empty();
        p.set_single_char_shape(0);
        p.insert_text_at(0, text);
        p.para_shape_id = shape;
        p
    }

    fn fixture() -> (Vec<Paragraph>, DocInfo, Paragraph) {
        let formats = std::array::from_fn(|level| format!("^{}. ", level + 1));
        let mut raw = Vec::new();
        for format in &formats {
            raw.extend_from_slice(&[0; 8]);
            raw.extend_from_slice(&u32::MAX.to_le_bytes());
            raw.extend_from_slice(&(format.encode_utf16().count() as u16).to_le_bytes());
            for unit in format.encode_utf16() {
                raw.extend_from_slice(&unit.to_le_bytes());
            }
        }
        raw.extend_from_slice(&9_u16.to_le_bytes());
        for _ in 0..7 {
            raw.extend_from_slice(&9_u32.to_le_bytes());
        }
        raw.extend_from_slice(&[0xab, 0xcd]);
        let mut shape_raw = vec![0; 60];
        shape_raw[58..].copy_from_slice(&[0xef, 0xab]);
        let mut bullet_raw = vec![0; 24];
        bullet_raw[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
        bullet_raw[22..].copy_from_slice(&[0x34, 0x12]);
        let mut info = DocInfo {
            char_shapes: vec![CharShape::default()],
            para_shapes: vec![
                ParaShape {
                    margin_left: 100,
                    indent: -20,
                    spacing_after: 421,
                    raw_data: Some(shape_raw),
                    ..Default::default()
                },
                ParaShape {
                    head_type: HeadType::Bullet,
                    numbering_id: 1,
                    ..Default::default()
                },
                ParaShape {
                    head_type: HeadType::Number,
                    numbering_id: 1,
                    ..Default::default()
                },
            ],
            bullets: vec![Bullet {
                char_shape_id: u32::MAX,
                bullet_char: '★',
                raw_data: Some(bullet_raw),
                ..Default::default()
            }],
            numberings: vec![Numbering {
                heads: [NumberingHead {
                    char_shape_id: u32::MAX,
                    ..Default::default()
                }; 7],
                level_formats: formats,
                start_number: 9,
                level_start_numbers: [9; 7],
                raw_data: Some(raw),
                ..Default::default()
            }],
            ..Default::default()
        };
        info.char_shapes[0].base_size = 1000;
        info.raw_provenance = Some(DocInfoSeal {
            model_digest: [0; 32],
            raw_digest: [0; 32],
            record_seals: DocInfoRecordSeals {
                para_shapes: info.para_shapes.iter().map(record_digest).collect(),
                bullets: info.bullets.iter().map(record_digest).collect(),
                numberings: info.numberings.iter().map(record_digest).collect(),
                ..Default::default()
            },
        });
        (
            vec![
                p(&crate::template::tag("list.max-depth:6"), 0),
                p(&crate::template::tag("list.indent-hwp:800"), 0),
                p(&crate::template::tag("list.bullet"), 1),
                p(&crate::template::tag("list.ordered"), 2),
            ],
            info,
            p(&crate::template::tag("body"), 0),
        )
    }

    fn text(value: &str) -> ListItemBlock {
        ListItemBlock::Paragraph {
            inlines: vec![Inline::Text {
                value: value.into(),
            }],
        }
    }
    fn item(value: &str) -> ListItem {
        ListItem {
            blocks: vec![text(value)],
        }
    }
    fn shape<'a>(p: &Paragraph, info: &'a DocInfo) -> &'a ParaShape {
        &info.para_shapes[usize::from(p.para_shape_id)]
    }

    #[test]
    fn native_restarts_preserve_formats_raw_tails_and_original_records() {
        let (roots, mut info, body) = fixture();
        let original = serde_json::json!({"numberings": info.numberings, "bullets": info.bullets, "para_shapes": info.para_shapes});
        let source = serde_json::to_value(&roots).unwrap();
        let mut lists = Lists::read(&roots, &info).unwrap();
        let output = lists
            .render(
                &ListKind::Ordered,
                Some(7),
                &[item("one"), item("two")],
                &body,
                &mut info,
            )
            .unwrap();
        assert_eq!(output[0].text, "one");
        let first = shape(&output[0], &info);
        let second = shape(&output[1], &info);
        assert_eq!(first.head_type, HeadType::Number);
        assert_eq!(first.numbering_id, second.numbering_id);
        assert_eq!(
            (first.margin_left, first.indent, first.spacing_after),
            (100, -20, 421)
        );
        assert_eq!(&first.raw_data.as_ref().unwrap()[58..], &[0xef, 0xab]);
        let numbering = &info.numberings[usize::from(first.numbering_id - 1)];
        assert_eq!(numbering.start_number, 7);
        assert_eq!(numbering.level_start_numbers[0], 7);
        assert_eq!(numbering.level_formats, info.numberings[0].level_formats);
        assert!(numbering.heads.iter().all(|h| h.char_shape_id == 0));
        assert_eq!(
            &numbering.raw_data.as_ref().unwrap()[numbering.raw_data.as_ref().unwrap().len() - 2..],
            &[0xab, 0xcd]
        );
        let seals = &info.raw_provenance.as_ref().unwrap().record_seals;
        assert!(record_raw_permitted(
            Some(&seals.numberings),
            usize::from(first.numbering_id - 1),
            numbering
        ));
        assert!(record_raw_permitted(
            Some(&seals.para_shapes),
            usize::from(output[0].para_shape_id),
            first
        ));
        let first_id = first.numbering_id;
        let again = lists
            .render(
                &ListKind::Ordered,
                Some(7),
                &[item("new")],
                &body,
                &mut info,
            )
            .unwrap();
        assert_ne!(shape(&again[0], &info).numbering_id, first_id);
        assert_eq!(
            serde_json::to_value(&info.numberings[0]).unwrap(),
            original["numberings"][0]
        );
        assert_eq!(
            serde_json::to_value(&info.bullets[0]).unwrap(),
            original["bullets"][0]
        );
        assert_eq!(
            serde_json::to_value(&info.para_shapes[0]).unwrap(),
            original["para_shapes"][0]
        );
        assert_eq!(serde_json::to_value(&roots).unwrap(), source);
    }

    #[test]
    fn nested_resume_and_continuations_start_fresh_native_segments() {
        let (roots, mut info, body) = fixture();
        let mut lists = Lists::read(&roots, &info).unwrap();
        let items = vec![
            ListItem {
                blocks: vec![
                    text("parent"),
                    ListItemBlock::List {
                        kind: ListKind::Ordered,
                        start: Some(99),
                        tight: false,
                        items: vec![item("child")],
                    },
                    text("continuation"),
                ],
            },
            item("next"),
        ];
        let output = lists
            .render(&ListKind::Ordered, Some(7), &items, &body, &mut info)
            .unwrap();
        let shapes: Vec<_> = output.iter().map(|p| shape(p, &info)).collect();
        assert_eq!(shapes[1].para_level, 1);
        assert_eq!(shapes[1].margin_left, 900);
        assert_eq!(shapes[2].head_type, HeadType::None);
        assert_eq!(shapes[2].numbering_id, 0);
        assert_eq!((shapes[2].margin_left, shapes[2].indent), (2080, 0));
        let starts: Vec<_> = [0, 1, 3]
            .into_iter()
            .map(|i| info.numberings[usize::from(shapes[i].numbering_id - 1)].start_number)
            .collect();
        assert_eq!(starts, [7, 99, 8]);
        assert_ne!(shapes[0].numbering_id, shapes[3].numbering_id);
    }

    #[test]
    fn validates_unused_samples_and_rejects_unmeasured_continuations() {
        let (roots, mut info, _) = fixture();
        info.numberings[0].level_formats[6].clear();
        assert!(
            Lists::read(&roots, &info)
                .err()
                .unwrap()
                .contains("level 7")
        );
        let (roots, mut info, body) = fixture();
        info.numberings[0].heads[0].attr |= 1 << 2;
        let mut lists = Lists::read(&roots, &info).unwrap();
        // Ordinary markers do not require font measurement.
        assert!(
            lists
                .render(
                    &ListKind::Ordered,
                    Some(1),
                    &[item("single")],
                    &body,
                    &mut info
                )
                .is_ok()
        );
        let error = lists
            .render(
                &ListKind::Ordered,
                Some(1),
                &[ListItem {
                    blocks: vec![text("one"), text("two")],
                }],
                &body,
                &mut info,
            )
            .unwrap_err();
        assert!(error.contains("UseInstWidth=true") && error.contains("Hancom"));
    }

    #[test]
    fn all_seven_native_levels_and_depth_limit_are_authoritative() {
        let (roots, mut info, body) = fixture();
        let mut items = vec![item("deep")];
        for _ in 0..6 {
            items = vec![ListItem {
                blocks: vec![
                    text("level"),
                    ListItemBlock::List {
                        kind: ListKind::Ordered,
                        start: Some(99),
                        tight: true,
                        items,
                    },
                ],
            }];
        }
        let mut lists = Lists::read(&roots, &info).unwrap();
        let output = lists
            .render(&ListKind::Ordered, Some(7), &items, &body, &mut info)
            .unwrap();
        assert_eq!(output.len(), 7);
        for (depth, paragraph) in output.iter().enumerate() {
            let shape = shape(paragraph, &info);
            assert_eq!(usize::from(shape.para_level), depth);
            assert_eq!(shape.margin_left, 100 + depth as i32 * 800);
            let raw = shape.raw_data.as_ref().unwrap();
            assert_eq!(&raw[54..58], &(depth as u32).to_le_bytes());
            assert_eq!(&raw[58..], &[0xef, 0xab]);
            let attr = u32::from_le_bytes(raw[..4].try_into().unwrap());
            assert_eq!((attr >> 25) & 7, depth as u32);
            assert_eq!((attr >> 23) & 3, 2);
            let n = &info.numberings[usize::from(shape.numbering_id - 1)];
            assert_eq!(
                n.level_start_numbers[depth],
                if depth == 0 { 7 } else { 99 }
            );
        }
        let too_deep = vec![ListItem {
            blocks: vec![
                text("extra"),
                ListItemBlock::List {
                    kind: ListKind::Ordered,
                    start: Some(1),
                    tight: true,
                    items,
                },
            ],
        }];
        assert!(
            lists
                .render(&ListKind::Ordered, Some(1), &too_deep, &body, &mut info)
                .unwrap_err()
                .contains("depth 7")
        );
    }

    #[test]
    fn bullet_definition_is_private_and_reused_with_body_rich_text() {
        let (roots, mut info, body) = fixture();
        let mut lists = Lists::read(&roots, &info).unwrap();
        let items = vec![ListItem {
            blocks: vec![ListItemBlock::Paragraph {
                inlines: vec![
                    Inline::Strong {
                        inlines: vec![Inline::Text {
                            value: "bold".into(),
                        }],
                    },
                    Inline::LineBreak,
                    Inline::Text {
                        value: "tail".into(),
                    },
                ],
            }],
        }];
        let output = lists
            .render(&ListKind::Bullet, None, &items, &body, &mut info)
            .unwrap();
        let second = lists
            .render(&ListKind::Bullet, None, &[item("other")], &body, &mut info)
            .unwrap();
        let id = shape(&output[0], &info).numbering_id;
        assert_eq!(id, shape(&second[0], &info).numbering_id);
        assert_ne!(id, 1);
        assert_eq!(info.bullets[usize::from(id - 1)].bullet_char, '★');
        assert_eq!(info.bullets[usize::from(id - 1)].char_shape_id, 0);
        assert_eq!(info.bullets[0].char_shape_id, u32::MAX);
        assert!(info.char_shapes[output[0].char_shape_id_at(0).unwrap() as usize].bold);
        assert!(output[0].text.contains("tail"));
    }

    #[test]
    fn continuation_offset_units_and_rounding_match_native_contract() {
        let (roots, mut info, _) = fixture();
        info.char_shapes[0].base_size = 1001;
        info.bullets[0].text_distance = 25;
        info.bullets[0].width_adjust = -3;
        let lists = Lists::read(&roots, &info).unwrap();
        assert_eq!(
            lists
                .continuation_margin(&ListKind::Bullet, 0, &info.para_shapes[0], &info)
                .unwrap(),
            2576
        );
        info.bullets[0].attr |= 1 << 4;
        info.bullets[0].text_distance = 50;
        let lists = Lists::read(&roots, &info).unwrap();
        assert_eq!(
            lists
                .continuation_margin(&ListKind::Bullet, 0, &info.para_shapes[0], &info)
                .unwrap(),
            2176
        );
    }

    #[test]
    fn stale_record_seals_cannot_reauthorize_obsolete_native_bytes() {
        let (roots, mut info, body) = fixture();
        info.numberings[0].start_number = 12; // Deliberately invalidate source seal.
        let mut lists = Lists::read(&roots, &info).unwrap();
        assert!(lists.ordered.raw_data.is_none());
        let output = lists
            .render(
                &ListKind::Ordered,
                Some(7),
                &[item("fresh")],
                &body,
                &mut info,
            )
            .unwrap();
        let index = usize::from(shape(&output[0], &info).numbering_id - 1);
        assert!(info.numberings[index].raw_data.is_none());
        assert!(record_raw_permitted(
            Some(
                &info
                    .raw_provenance
                    .as_ref()
                    .unwrap()
                    .record_seals
                    .numberings
            ),
            index,
            &info.numberings[index]
        ));
        assert!(!record_raw_permitted(
            Some(
                &info
                    .raw_provenance
                    .as_ref()
                    .unwrap()
                    .record_seals
                    .numberings
            ),
            0,
            &info.numberings[0]
        ));
    }
}
