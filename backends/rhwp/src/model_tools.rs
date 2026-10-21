// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Euisoon Ahn
//! Native paragraph-container traversal shared by template operations.

use rhwp::model::control::Control;
use rhwp::model::paragraph::Paragraph;
use rhwp::model::raw_provenance::record_digest;
use rhwp::model::shape::{Caption, ShapeObject};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) struct IdentityAllocator {
    used: BTreeSet<u32>,
    next: u64,
}

fn alias(id: u32) -> u32 {
    if id == 0 { 0 } else { (id & 0x3fff_ffff) + 1 }
}

impl IdentityAllocator {
    fn id(&mut self) -> Result<u32, String> {
        while self.next <= u64::from(u32::MAX) {
            let id = self.next as u32;
            self.next += 1;
            if self.used.insert(id) {
                return Ok(id);
            }
        }
        Err("Native object identity space exhausted".into())
    }
    fn common(&mut self) -> Result<u32, String> {
        loop {
            let id = self.id()?;
            if self.used.insert(alias(id)) {
                return Ok(id);
            }
        }
    }
}

/// Reserve modeled and retained native identities without changing the template.
pub(crate) fn identity_seed(
    document: &rhwp::model::document::Document,
) -> Result<IdentityAllocator, String> {
    fn raw_id(value: &serde_json::Value, start: usize) -> Option<u32> {
        let bytes = value.as_array()?.get(start..start + 4)?;
        let mut raw = [0; 4];
        for (to, from) in raw.iter_mut().zip(bytes) {
            *to = u8::try_from(from.as_u64()?).ok()?;
        }
        Some(u32::from_le_bytes(raw))
    }
    fn reserve(value: &serde_json::Value, used: &mut BTreeSet<u32>) {
        match value {
            serde_json::Value::Object(fields) => {
                for (name, value) in fields {
                    if matches!(
                        name.as_str(),
                        "instance_id"
                            | "inst_id"
                            | "field_id"
                            | "end_field_id"
                            | "begin_id_ref"
                            | "start_subject_id"
                            | "end_subject_id"
                            | "hwpx_ole_id"
                    ) && let Some(id) = value.as_u64().and_then(|n| u32::try_from(n).ok())
                    {
                        used.insert(id);
                        used.insert(alias(id));
                    }
                    let offset = match name.as_str() {
                        "raw_header_extra" => Some(6),
                        "raw_ctrl_data" => Some(32),
                        "raw_picture_extra" => Some(1),
                        _ => None,
                    };
                    if let Some(id) = offset.and_then(|offset| raw_id(value, offset)) {
                        used.insert(id);
                        used.insert(alias(id));
                    }
                    reserve(value, used);
                }
            }
            serde_json::Value::Array(values) => {
                for value in values {
                    reserve(value, used);
                }
            }
            _ => {}
        }
    }
    let mut used = BTreeSet::new();
    for section in &document.sections {
        reserve(
            &serde_json::to_value(&section.paragraphs).map_err(|e| e.to_string())?,
            &mut used,
        );
        reserve(
            &serde_json::to_value(&section.section_def).map_err(|e| e.to_string())?,
            &mut used,
        );
    }
    Ok(IdentityAllocator { used, next: 1 })
}

#[derive(Default)]
struct ReferenceMap(BTreeMap<u32, Option<u32>>);
impl ReferenceMap {
    fn add(&mut self, old: u32, new: u32) {
        if old == 0 {
            return;
        }
        self.0
            .entry(old)
            .and_modify(|value| {
                if *value != Some(new) {
                    *value = None;
                }
            })
            .or_insert(Some(new));
    }
    fn rewrite(&self, id: &mut u32) -> Result<(), String> {
        match self.0.get(id) {
            Some(Some(new)) => *id = *new,
            Some(None) => return Err(format!("Ambiguous identity {id} in copied heading block")),
            None => {}
        }
        Ok(())
    }
    fn subject(&mut self, old: u32, new: u32) {
        self.add(old, new);
        self.add(alias(old), new);
    }
}

fn remap_common(
    common: &mut rhwp::model::shape::CommonObjAttr,
    raw: &mut [u8],
    seal: &mut Option<[u8; 32]>,
    ids: &mut IdentityAllocator,
) -> Result<(), String> {
    let valid_seal = seal.is_some_and(|digest| digest == record_digest(common));
    common.instance_id = ids.common()?;
    if !raw.is_empty() {
        raw.get_mut(32..36)
            .ok_or("Truncated native common identity payload")?
            .copy_from_slice(&common.instance_id.to_le_bytes());
    }
    if valid_seal {
        *seal = Some(record_digest(common));
    }
    Ok(())
}

fn remap_picture(
    pic: &mut rhwp::model::image::Picture,
    ids: &mut IdentityAllocator,
    subjects: &mut ReferenceMap,
) -> Result<(), String> {
    let old = pic.common.instance_id;
    pic.common.instance_id = ids.common()?;
    subjects.subject(old, pic.common.instance_id);
    pic.instance_id = ids.id()?;
    if !pic.raw_picture_extra.is_empty() {
        pic.raw_picture_extra
            .get_mut(1..5)
            .ok_or("Truncated native picture identity payload")?
            .copy_from_slice(&pic.instance_id.to_le_bytes());
    }
    Ok(())
}

fn remap_shape(
    shape: &mut ShapeObject,
    ids: &mut IdentityAllocator,
    subjects: &mut ReferenceMap,
) -> Result<(), String> {
    if let ShapeObject::Picture(pic) = shape {
        return remap_picture(pic, ids, subjects);
    }
    let old = shape.common().instance_id;
    let new = ids.common()?;
    shape.common_mut().instance_id = new;
    let target = if let Some(drawing) = shape.drawing_mut() {
        let old_drawing = drawing.inst_id;
        drawing.inst_id = ids.id()?;
        subjects.add(old_drawing, drawing.inst_id);
        drawing.inst_id
    } else {
        new
    };
    subjects.subject(old, target);
    match shape {
        ShapeObject::Group(group) => {
            for child in &mut group.children {
                remap_shape(child, ids, subjects)?;
            }
        }
        // OLE fallback stores opaque native identity-bearing content; fail closed.
        ShapeObject::Ole(_) => {
            return Err("Copied heading blocks do not support OLE objects".into());
        }
        _ => {}
    }
    Ok(())
}

fn assign_identities(
    paragraph: &mut Paragraph,
    ids: &mut IdentityAllocator,
    subjects: &mut ReferenceMap,
    fields: &mut ReferenceMap,
) -> Result<(), String> {
    if let Some(raw) = paragraph.raw_header_extra.get_mut(6..10) {
        raw.copy_from_slice(&ids.id()?.to_le_bytes());
    }
    for control in &mut paragraph.controls {
        match control {
            Control::Table(table) => remap_common(
                &mut table.common,
                &mut table.raw_ctrl_data,
                &mut table.raw_ctrl_seal,
                ids,
            )?,
            Control::Equation(eq) => remap_common(
                &mut eq.common,
                &mut eq.raw_ctrl_data,
                &mut eq.raw_ctrl_seal,
                ids,
            )?,
            Control::Shape(shape) => remap_shape(shape, ids, subjects)?,
            Control::Picture(pic) => remap_picture(pic, ids, subjects)?,
            Control::Form(form) => form.common.instance_id = ids.common()?,
            Control::Footnote(note) => note.instance_id = ids.id()?,
            Control::Endnote(note) => note.instance_id = ids.id()?,
            Control::Field(field) => {
                let old = field.field_id;
                field.field_id = ids.id()?;
                fields.add(old, field.field_id);
            }
            _ => {}
        }
    }
    map_nested_containers(paragraph, &mut |paragraphs| {
        for paragraph in paragraphs {
            assign_identities(paragraph, ids, subjects, fields)?;
        }
        Ok(())
    })
}

fn fix_shape_refs(shape: &mut ShapeObject, subjects: &ReferenceMap) -> Result<(), String> {
    match shape {
        ShapeObject::Line(line) => {
            if let Some(connector) = &mut line.connector {
                subjects.rewrite(&mut connector.start_subject_id)?;
                subjects.rewrite(&mut connector.end_subject_id)?;
            }
        }
        ShapeObject::Group(group) => {
            for child in &mut group.children {
                fix_shape_refs(child, subjects)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn fix_references(
    paragraph: &mut Paragraph,
    subjects: &ReferenceMap,
    fields: &ReferenceMap,
) -> Result<(), String> {
    for end in &mut paragraph.orphan_field_ends {
        fields.rewrite(&mut end.begin_id_ref)?;
    }
    for control in &mut paragraph.controls {
        if let Control::Shape(shape) = control {
            fix_shape_refs(shape, subjects)?;
        }
    }
    map_nested_containers(paragraph, &mut |paragraphs| {
        for paragraph in paragraphs {
            fix_references(paragraph, subjects, fields)?;
        }
        Ok(())
    })
}

/// Assign all destinations before fixing cross-paragraph and connector refs.
/// Operates on detached generated paragraphs only; original template IDs remain.
pub(crate) fn refresh_block_identities(
    paragraphs: &mut [Paragraph],
    ids: &mut IdentityAllocator,
) -> Result<(), String> {
    let mut subjects = ReferenceMap::default();
    let mut fields = ReferenceMap::default();
    for paragraph in &mut *paragraphs {
        assign_identities(paragraph, ids, &mut subjects, &mut fields)?;
    }
    for paragraph in paragraphs {
        fix_references(paragraph, &subjects, &fields)?;
    }
    Ok(())
}

/// Visit each immediate nested paragraph list without descending into its
/// paragraphs. The callback owns paragraph recursion and can replace a list.
/// Grouped drawing objects are traversed to reach their paragraph containers.
pub(crate) fn map_nested_containers(
    paragraph: &mut Paragraph,
    callback: &mut impl FnMut(&mut Vec<Paragraph>) -> Result<(), String>,
) -> Result<(), String> {
    for control in &mut paragraph.controls {
        match control {
            Control::SectionDef(section) => {
                for master in &mut section.master_pages {
                    callback(&mut master.paragraphs)?;
                }
            }
            Control::Table(table) => {
                for cell in &mut table.cells {
                    callback(&mut cell.paragraphs)?;
                }
                map_caption(&mut table.caption, callback)?;
            }
            Control::Shape(shape) => map_shape(shape, callback)?,
            Control::Picture(picture) => map_caption(&mut picture.caption, callback)?,
            Control::Header(header) => callback(&mut header.paragraphs)?,
            Control::Footer(footer) => callback(&mut footer.paragraphs)?,
            Control::Footnote(note) => callback(&mut note.paragraphs)?,
            Control::Endnote(note) => callback(&mut note.paragraphs)?,
            Control::HiddenComment(comment) => callback(&mut comment.paragraphs)?,
            Control::Field(field) => callback(&mut field.memo_paragraphs)?,
            _ => {}
        }
    }
    Ok(())
}

fn map_caption(
    caption: &mut Option<Caption>,
    callback: &mut impl FnMut(&mut Vec<Paragraph>) -> Result<(), String>,
) -> Result<(), String> {
    if let Some(caption) = caption {
        callback(&mut caption.paragraphs)?;
    }
    Ok(())
}

fn map_shape(
    shape: &mut ShapeObject,
    callback: &mut impl FnMut(&mut Vec<Paragraph>) -> Result<(), String>,
) -> Result<(), String> {
    match shape {
        ShapeObject::Group(group) => {
            for child in &mut group.children {
                map_shape(child, callback)?;
            }
            map_caption(&mut group.caption, callback)?;
        }
        ShapeObject::Picture(picture) => map_caption(&mut picture.caption, callback)?,
        ShapeObject::Chart(chart) => {
            map_caption(&mut chart.caption, callback)?;
            map_drawing(&mut chart.drawing, callback)?;
        }
        ShapeObject::Ole(ole) => {
            map_ole(ole, callback)?;
        }
        _ => {
            if let Some(drawing) = shape.drawing_mut() {
                map_drawing(drawing, callback)?;
            }
        }
    }
    Ok(())
}

fn map_ole(
    ole: &mut rhwp::model::shape::OleShape,
    callback: &mut impl FnMut(&mut Vec<Paragraph>) -> Result<(), String>,
) -> Result<(), String> {
    map_caption(&mut ole.caption, callback)?;
    map_drawing(&mut ole.drawing, callback)?;
    if let Some(fallback) = &mut ole.chart_switch_fallback {
        map_ole(fallback, callback)?;
    }
    Ok(())
}

fn map_drawing(
    drawing: &mut rhwp::model::shape::DrawingObjAttr,
    callback: &mut impl FnMut(&mut Vec<Paragraph>) -> Result<(), String>,
) -> Result<(), String> {
    if let Some(text_box) = &mut drawing.text_box {
        callback(&mut text_box.paragraphs)?;
    }
    map_caption(&mut drawing.caption, callback)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rhwp::model::shape::{GroupShape, RectangleShape, TextBox};
    use rhwp::model::table::{Cell, Table};

    #[test]
    fn reserves_static_master_page_and_mismatched_retained_identities() {
        use rhwp::model::header_footer::MasterPage;
        let core = rhwp::DocumentCore::new_empty();
        let mut document = core.document().clone();
        document.sections.push(Default::default());
        let mut table = Table::default();
        table.common.instance_id = 301;
        table.raw_ctrl_data = vec![0; 36];
        table.raw_ctrl_data[32..36].copy_from_slice(&401u32.to_le_bytes());
        let mut raw_header_extra = vec![0; 10];
        raw_header_extra[6..10].copy_from_slice(&501u32.to_le_bytes());
        document.sections[0]
            .section_def
            .master_pages
            .push(MasterPage {
                paragraphs: vec![Paragraph {
                    raw_header_extra,
                    controls: vec![Control::Table(Box::new(table))],
                    ..Default::default()
                }],
                ..Default::default()
            });
        let ids = identity_seed(&document).unwrap();
        for id in [301, 401, 501] {
            assert!(ids.used.contains(&id));
            assert!(ids.used.contains(&alias(id)));
        }
    }

    #[test]
    fn preserves_shared_field_metadata_and_remaps_cross_paragraph_begin_ref() {
        use rhwp::model::control::Field;
        use rhwp::model::paragraph::OrphanFieldEnd;
        let core = rhwp::DocumentCore::new_empty();
        let mut ids = identity_seed(core.document()).unwrap();
        let mut paragraphs = vec![
            Paragraph {
                orphan_field_ends: vec![OrphanFieldEnd {
                    char_idx: 0,
                    begin_id_ref: 991,
                    field_id: 87,
                    begin_ctrl_id: 0,
                }],
                ..Default::default()
            },
            Paragraph {
                controls: vec![Control::Field(Field {
                    field_id: 991,
                    instance_id: Some(87),
                    ..Default::default()
                })],
                ..Default::default()
            },
        ];
        refresh_block_identities(&mut paragraphs, &mut ids).unwrap();
        let Control::Field(field) = &paragraphs[1].controls[0] else {
            panic!()
        };
        assert_ne!(field.field_id, 991);
        assert_eq!(
            paragraphs[0].orphan_field_ends[0].begin_id_ref,
            field.field_id
        );
        assert_eq!(paragraphs[0].orphan_field_ends[0].field_id, 87);
        assert_eq!(field.instance_id, Some(87));
    }

    #[test]
    fn stale_common_provenance_seal_stays_stale() {
        let core = rhwp::DocumentCore::new_empty();
        let mut ids = identity_seed(core.document()).unwrap();
        let mut table = Table::default();
        table.raw_ctrl_data = vec![0; 36];
        table.raw_ctrl_seal = Some(record_digest(&table.common));
        table.common.width = 1234;
        let old_seal = table.raw_ctrl_seal;
        let mut paragraphs = vec![Paragraph {
            controls: vec![Control::Table(Box::new(table))],
            ..Default::default()
        }];
        refresh_block_identities(&mut paragraphs, &mut ids).unwrap();
        let Control::Table(table) = &paragraphs[0].controls[0] else {
            panic!()
        };
        assert_eq!(table.raw_ctrl_seal, old_seal);
        assert_ne!(table.raw_ctrl_seal, Some(record_digest(&table.common)));
    }

    #[test]
    fn refreshes_nested_table_raw_identity_and_valid_seal() {
        let core = rhwp::DocumentCore::new_empty();
        let mut document = core.document().clone();
        document.sections.push(Default::default());
        let mut table = Table::default();
        table.common.instance_id = 1;
        table.common.width = 1234;
        table.raw_ctrl_data = vec![0; 36];
        table.raw_ctrl_data[32..36].copy_from_slice(&1u32.to_le_bytes());
        table.raw_ctrl_seal = Some(record_digest(&table.common));
        let paragraph = Paragraph {
            raw_header_extra: vec![0; 10],
            controls: vec![Control::Table(Box::new(table))],
            ..Default::default()
        };
        document.sections[0].paragraphs = vec![paragraph.clone()];
        let mut ids = identity_seed(&document).unwrap();
        let mut generated = vec![paragraph.clone(), paragraph];
        refresh_block_identities(&mut generated, &mut ids).unwrap();
        let mut allocated = BTreeSet::new();
        for paragraph in &generated {
            let para_id = u32::from_le_bytes(paragraph.raw_header_extra[6..10].try_into().unwrap());
            assert!(allocated.insert(para_id));
            let Control::Table(table) = &paragraph.controls[0] else {
                panic!()
            };
            assert_ne!(table.common.instance_id, 1);
            assert!(allocated.insert(table.common.instance_id));
            assert_eq!(table.common.width, 1234);
            assert_eq!(
                table.raw_ctrl_data[32..36],
                table.common.instance_id.to_le_bytes()
            );
            assert_eq!(table.raw_ctrl_seal, Some(record_digest(&table.common)));
        }
        let Control::Table(original) = &document.sections[0].paragraphs[0].controls[0] else {
            panic!()
        };
        assert_eq!(original.common.instance_id, 1);
    }

    #[test]
    fn visits_grouped_shape_text_and_captions_without_paragraph_recursion() {
        let mut rectangle = RectangleShape::default();
        rectangle.drawing.text_box = Some(TextBox {
            paragraphs: vec![Paragraph {
                controls: vec![Control::Table(Box::new(Table {
                    cells: vec![Cell::default()],
                    ..Default::default()
                }))],
                ..Default::default()
            }],
            ..Default::default()
        });
        rectangle.drawing.caption = Some(Caption::default());
        let mut paragraph = Paragraph {
            controls: vec![Control::Shape(Box::new(ShapeObject::Group(GroupShape {
                children: vec![ShapeObject::Rectangle(rectangle)],
                caption: Some(Caption::default()),
                ..Default::default()
            })))],
            ..Default::default()
        };
        let mut visits = 0;
        map_nested_containers(&mut paragraph, &mut |_| {
            visits += 1;
            Ok(())
        })
        .unwrap();
        assert_eq!(visits, 3);
    }

    #[test]
    fn propagates_callback_failure() {
        let mut paragraph = Paragraph {
            controls: vec![Control::Table(Box::new(Table {
                cells: vec![Cell::default(), Cell::default()],
                ..Default::default()
            }))],
            ..Default::default()
        };
        let mut visits = 0;
        let error = map_nested_containers(&mut paragraph, &mut |_| {
            visits += 1;
            Err("stop".into())
        })
        .unwrap_err();
        assert_eq!(error, "stop");
        assert_eq!(visits, 1);
    }
}
