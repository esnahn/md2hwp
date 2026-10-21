// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Euisoon Ahn
//! Native pictures use new resources and geometry, with template-owned captions.

use md2hwp_core::ir::{Inline, SourceParagraph};
use rhwp::model::{
    bin_data::{BinData, BinDataCompression, BinDataContent, BinDataType},
    control::{AutoNumberType, Control, NewNumber},
    document::{DocInfo, Document},
    image::{CropInfo, Picture},
    paragraph::Paragraph,
    shape::{
        Caption, CaptionDirection, HorzRelTo, ObjectNumberingType, ShapeComponentAttr,
        SizeCriterion, VertRelTo,
    },
    style::HeadType,
};

const CAPTION: &str = "{{md2hwp:slot:figure.caption}}";
const SOURCE: &str = "{{md2hwp:slot:figure.source}}";
const PREFIX: &str = "{{md2hwp:slot:figure.source.prefix}}";
const CHAPTER: &str = "{{md2hwp:num:heading1}}";

pub(crate) struct Prototype {
    anchor: Paragraph,
    before: Vec<Paragraph>,
    after: Vec<Paragraph>,
    caption: Caption,
    width: u32,
}

impl Prototype {
    pub(crate) fn read(roots: &[Paragraph], info: &DocInfo) -> Result<Self, String> {
        let boundary = |name: &str| -> Result<usize, String> {
            let token = crate::template::tag(name);
            let found: Vec<_> = roots
                .iter()
                .enumerate()
                .filter(|(_, p)| p.text == token)
                .collect();
            if found.len() != 1 {
                return Err(format!("Figure requires exactly one plain root {token}"));
            }
            plain(found[0].1, info)?;
            Ok(found[0].0)
        };
        let begin = boundary("begin:figure")?;
        let end = boundary("end:figure")?;
        if end <= begin + 1 || end - begin > 65 {
            return Err("Figure range requires one sample picture with 0–63 empty paragraphs before/after it".into());
        }
        let settings: Vec<_> = roots
            .iter()
            .enumerate()
            .filter_map(|(i, p)| {
                p.text
                    .strip_prefix("{{md2hwp:figure.max-width-mm:")
                    .and_then(|s| s.strip_suffix("}}"))
                    .map(|s| (i, p, s))
            })
            .collect();
        if settings.len() != 1 || settings[0].0 >= begin {
            return Err(
                "Figure requires exactly one figure.max-width-mm setting before begin:figure"
                    .into(),
            );
        }
        plain(settings[0].1, info)?;
        let text = settings[0].2;
        let mm = text
            .parse::<f64>()
            .map_err(|_| "Invalid figure.max-width-mm")?;
        if !text.chars().all(|c| c.is_ascii_digit() || c == '.')
            || !mm.is_finite()
            || mm <= 0.0
            || mm > 142.0
        {
            return Err("Figure width must be greater than zero and at most 142 mm".into());
        }
        let width = (mm * 7200.0 / 25.4).round() as u32;
        if width == 0 {
            return Err("Figure width is smaller than one native HWPUNIT".into());
        }
        let range = &roots[begin + 1..end];
        let anchors: Vec<_> = range
            .iter()
            .enumerate()
            .filter(|(_, p)| !p.controls.is_empty())
            .collect();
        if anchors.len() != 1 {
            return Err(
                "Figure requires one native picture and only empty spacing paragraphs".into(),
            );
        }
        let (index, anchor) = anchors[0];
        for p in range[..index].iter().chain(&range[index + 1..]) {
            plain(p, info)?;
            if !p.text.is_empty()
                || info.para_shapes[usize::from(p.para_shape_id)].head_type != HeadType::None
            {
                return Err(
                    "Figure spacing must be empty plain paragraphs without native numbering".into(),
                );
            }
        }
        if !anchor.text.is_empty() || anchor.controls.len() != 1 {
            return Err("Figure anchor requires only its native picture".into());
        }
        formats(anchor, info)?;
        let Control::Picture(picture) = &anchor.controls[0] else {
            return Err("Figure requires one native picture".into());
        };
        let caption = picture
            .caption
            .as_ref()
            .ok_or("Figure requires a native caption")?;
        if caption.paragraphs.len() != 2 {
            return Err(
                "Figure caption requires caption and source paragraphs, in that order".into(),
            );
        }
        let title = &caption.paragraphs[0];
        formats(title, info)?;
        if title.controls.len() != 1
            || !matches!(&title.controls[0], Control::AutoNumber(n) if n.number_type == AutoNumberType::Picture)
        {
            return Err("Figure caption requires exactly one native Figure AUTONUM".into());
        }
        let mut allowed = title.text.replace(CAPTION, "").replace(CHAPTER, "");
        if title.text.matches(CAPTION).count() != 1 || allowed.contains("{{md2hwp:") {
            return Err(
                "Figure caption requires one caption slot and permits only num:heading1".into(),
            );
        }
        // A slot must not span the native automatic number's control gap.
        let mut probe = title.clone();
        crate::template::replace(&mut probe, CAPTION, "")?;
        while probe.text.contains(CHAPTER) {
            crate::template::replace(&mut probe, CHAPTER, "")?;
        }
        let source = &caption.paragraphs[1];
        plain(source, info)?;
        if source.text.matches(PREFIX).count() != 1
            || source.text.matches(SOURCE).count() != 1
            || !source.text.contains(&format!("{PREFIX}: {SOURCE}"))
        {
            return Err("Figure source requires slot:figure.source.prefix followed by ': ' and slot:figure.source".into());
        }
        allowed = source.text.replace(PREFIX, "").replace(SOURCE, "");
        if allowed.contains("{{md2hwp:") {
            return Err("Figure source contains unsupported template tags".into());
        }
        let mut counts = [0; 3];
        fn count(p: &Paragraph, counts: &mut [usize; 3]) -> Result<(), String> {
            for (index, token) in [CAPTION, SOURCE, PREFIX].iter().enumerate() {
                counts[index] += p.text.matches(token).count();
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
        if counts != [1, 1, 1] {
            return Err("Duplicate or misplaced figure slots in template".into());
        }
        Ok(Self {
            anchor: anchor.clone(),
            before: range[..index].to_vec(),
            after: range[index + 1..].to_vec(),
            caption: caption.clone(),
            width,
        })
    }

    pub(crate) fn render(
        &self,
        caption: &[Inline],
        source: Option<&[SourceParagraph]>,
        chapter: Option<u32>,
        number: u32,
        resource: &crate::image_resource::Resource,
        document: &mut Document,
    ) -> Result<Vec<Paragraph>, String> {
        crate::template::check_inlines(caption)?;
        let number = u16::try_from(number)
            .ok()
            .filter(|n| *n > 0)
            .ok_or("Figure number exceeds the native 1–65535 range")?;
        if !resource.width.is_finite()
            || !resource.height.is_finite()
            || resource.width <= 0.0
            || resource.height <= 0.0
        {
            return Err("Figure resource has invalid dimensions".into());
        }
        let h = (f64::from(self.width) * resource.height / resource.width).round();
        if h < 1.0 || h > f64::from(i32::MAX) {
            return Err("Figure height is outside native HWPUNIT limits".into());
        }
        let height = h as u32;
        let mut native_caption = self.caption.clone();
        let mut title = native_caption.paragraphs[0].clone();
        // Resolve template tags before inserting any manuscript literals.
        while title.text.contains(CHAPTER) {
            let chapter =
                chapter.ok_or("Figure caption num:heading1 requires a preceding heading1")?;
            crate::template::replace(&mut title, CHAPTER, &chapter.to_string())?;
        }
        fill_rich(&mut title, CAPTION, caption, &mut document.doc_info)?;
        let Control::AutoNumber(n) = &mut title.controls[0] else {
            unreachable!()
        };
        n.number = number;
        n.assigned_number = number;
        let mut paragraphs = vec![title];
        if let Some(sources) = source {
            if sources.is_empty() || sources.len() >= usize::from(u16::MAX) {
                return Err("Figure source requires 1–65534 paragraphs".into());
            }
            for source in sources {
                crate::template::check_inlines(&source.inlines)?;
                let mut p = native_caption.paragraphs[1].clone();
                crate::template::replace(&mut p, PREFIX, &source.prefix)?;
                fill_rich(&mut p, SOURCE, &source.inlines, &mut document.doc_info)?;
                paragraphs.push(p);
            }
        }
        native_caption.paragraphs = paragraphs;
        native_caption.max_width = if matches!(
            native_caption.direction,
            CaptionDirection::Left | CaptionDirection::Right
        ) {
            native_caption.width
        } else {
            self.width
        };
        // Use fresh model geometry: sample crop, transforms, borders and embedded
        // bytes are not manuscript defaults. The native crop axis is independent
        // of display extent; raster inputs follow the reader's 75 HU/pixel axis.
        let natural = if resource.pixel_width > 0 && resource.pixel_height > 0 {
            (
                f64::from(resource.pixel_width) * 75.0,
                f64::from(resource.pixel_height) * 75.0,
            )
        } else {
            (
                resource.width * 7200.0 / 2540.0,
                resource.height * 7200.0 / 2540.0,
            )
        };
        if natural.0 < 1.0
            || natural.1 < 1.0
            || natural.0 > f64::from(i32::MAX)
            || natural.1 > f64::from(i32::MAX)
        {
            return Err("Figure intrinsic dimensions exceed native crop-coordinate limits".into());
        }
        let id = embed(resource, document)?;
        let mut picture = Picture::default();
        picture.common.ctrl_id = 0x6773_6f20;
        picture.common.attr = 0x240a_2211;
        picture.common.hwp5_gen_shape_attr_bit26 = true;
        picture.common.numbering_type = ObjectNumberingType::Picture;
        picture.common.treat_as_char = true;
        picture.common.affect_line_spacing = false;
        picture.common.flow_with_text = true;
        picture.common.vert_rel_to = VertRelTo::Para;
        picture.common.horz_rel_to = HorzRelTo::Column;
        picture.common.width_criterion = SizeCriterion::Absolute;
        picture.common.height_criterion = SizeCriterion::Absolute;
        picture.common.width = self.width;
        picture.common.height = height;
        picture.shape_attr = ShapeComponentAttr {
            original_width: self.width,
            original_height: height,
            current_width: self.width,
            current_height: height,
            local_file_version: 1,
            ..Default::default()
        };
        // The pinned HWP5 reader/writer exposes two consecutive four-i32
        // storage chunks under these names, while Hancom consumes interleaved
        // (x,y) pairs. Supplying Cartesian x[]/y[] arrays produces a bow tie.
        picture.border_x = [0, 0, self.width as i32, 0];
        picture.border_y = [self.width as i32, height as i32, 0, height as i32];
        picture.crop = CropInfo {
            left: 0,
            top: 0,
            right: natural.0.round() as i32,
            bottom: natural.1.round() as i32,
        };
        picture.img_dim = (picture.crop.right as u32, picture.crop.bottom as u32);
        picture.image_attr.bin_data_id = id;
        picture.caption = Some(native_caption);
        let mut anchor = self.anchor.clone();
        anchor.controls = vec![Control::Picture(Box::new(picture))];
        anchor.ctrl_data_records = vec![None];
        anchor.control_mask = 1 << 11;
        if number == 1 && chapter.is_some() {
            anchor.controls.insert(
                0,
                Control::NewNumber(NewNumber {
                    number_type: AutoNumberType::Picture,
                    number: 1,
                }),
            );
            anchor.ctrl_data_records.insert(0, None);
            // Both controls precede the anchor's paragraph-end marker.
            anchor.char_count = anchor
                .char_count
                .checked_add(8)
                .ok_or("Figure anchor character-count overflow")?;
            anchor.control_mask |= 1 << 18;
        }
        crate::template::discard_stale_layout(&mut anchor);
        let mut output = self.before.clone();
        output.push(anchor);
        output.extend(self.after.clone());
        Ok(output)
    }
}

fn fill_rich(
    p: &mut Paragraph,
    token: &str,
    inlines: &[Inline],
    info: &mut DocInfo,
) -> Result<(), String> {
    let byte = p.text.find(token).ok_or("Figure slot disappeared")?;
    let base = p
        .char_shape_id_at(p.text[..byte].chars().count())
        .ok_or("Figure slot has no character format")?;
    let mut spans = Vec::new();
    crate::template::flatten(inlines, false, false, &mut spans);
    crate::template::replace_rich(p, token, &spans, base, info)
}

fn formats(p: &Paragraph, info: &DocInfo) -> Result<(), String> {
    if info.para_shapes.get(usize::from(p.para_shape_id)).is_none()
        || p.char_shapes.is_empty()
        || p.char_shapes
            .iter()
            .any(|s| info.char_shapes.get(s.char_shape_id as usize).is_none())
    {
        return Err("Figure references an absent paragraph/character format".into());
    }
    if !p.field_ranges.is_empty()
        || !p.orphan_field_ends.is_empty()
        || !p.range_tags.is_empty()
        || !p.title_marks.is_empty()
        || !p.markpen_marks.is_empty()
    {
        return Err("Figure sample contains unsupported fields or marks".into());
    }
    Ok(())
}

fn plain(p: &Paragraph, info: &DocInfo) -> Result<(), String> {
    crate::template::require_plain(p)?;
    formats(p, info)
}

fn embed(
    resource: &crate::image_resource::Resource,
    document: &mut Document,
) -> Result<u16, String> {
    let position = u16::try_from(document.doc_info.bin_data_list.len() + 1)
        .map_err(|_| "Too many embedded resources")?;
    let maximum = document
        .doc_info
        .bin_data_list
        .iter()
        .map(|d| d.storage_id)
        .chain(document.bin_data_content.iter().map(|d| d.id))
        .max()
        .unwrap_or(0);
    let storage_id = maximum
        .checked_add(1)
        .ok_or("Embedded resource storage IDs exhausted")?;
    document.doc_info.bin_data_list.push(BinData {
        attr: 0x0021,
        data_type: BinDataType::Embedding,
        compression: BinDataCompression::NoCompress,
        storage_id,
        extension: Some(resource.extension.clone()),
        ..Default::default()
    });
    document.bin_data_content.push(BinDataContent {
        id: storage_id,
        data: resource.bytes.clone().into(),
        extension: resource.extension.clone(),
    });
    document.doc_info.raw_stream_dirty = true;
    Ok(position)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rhwp::model::{
        control::AutoNumber,
        style::{CharShape, ParaShape},
    };

    fn p(text: &str) -> Paragraph {
        let mut p = Paragraph::new_empty();
        p.set_single_char_shape(0);
        p.insert_text_at(0, text);
        p
    }

    fn fixture() -> (Vec<Paragraph>, Document) {
        let mut title = p(&format!("그림 {CHAPTER}- {CAPTION}"));
        title.controls.push(Control::AutoNumber(AutoNumber {
            number_type: AutoNumberType::Picture,
            format: 3,
            number: 9,
            assigned_number: 9,
            prefix_char: '(',
            suffix_char: ')',
            ..Default::default()
        }));
        title.char_count += 8;
        for offset in &mut title.char_offsets {
            *offset += 8;
        }
        let mut picture = Picture::default();
        picture.common.width = 99;
        picture.common.height = 123;
        picture.reverse = true;
        picture.crop.right = 42;
        picture.image_attr.bin_data_id = 77;
        picture.caption = Some(Caption {
            spacing: 123,
            include_margin: true,
            paragraphs: vec![title, p(&format!("({PREFIX}: {SOURCE})"))],
            ..Default::default()
        });
        let mut anchor = p("");
        anchor.char_count += 8;
        anchor.control_mask = 0x800;
        anchor.controls.push(Control::Picture(Box::new(picture)));
        let mut before = p("");
        before.raw_break_type = 4;
        let mut document = Document::default();
        document.doc_info.para_shapes = vec![ParaShape::default()];
        document.doc_info.char_shapes = vec![CharShape::default()];
        (
            vec![
                p("{{md2hwp:figure.max-width-mm:142}}"),
                p("{{md2hwp:begin:figure}}"),
                before,
                anchor,
                p(""),
                p("{{md2hwp:end:figure}}"),
            ],
            document,
        )
    }

    fn resource(extension: &str) -> crate::image_resource::Resource {
        let root =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/all-features");
        let mut resource = crate::image_resource::Resource::read(&root, "image.png").unwrap();
        resource.extension = extension.into();
        resource.width = 200.0;
        resource.height = 100.0;
        resource.pixel_width = 200;
        resource.pixel_height = 100;
        resource
    }

    fn picture(output: &[Paragraph]) -> &Picture {
        let Control::Picture(p) = output[1].controls.last().unwrap() else {
            panic!()
        };
        p
    }

    #[test]
    fn uses_resource_aspect_and_exact_bytes_with_template_caption_and_spacing() {
        let (roots, mut document) = fixture();
        let prototype = Prototype::read(&roots, &document.doc_info).unwrap();
        let bytes = resource("jpeg");
        let output = prototype
            .render(
                &[Inline::Strong {
                    inlines: vec![Inline::Text {
                        value: CHAPTER.into(),
                    }],
                }],
                Some(&[
                    SourceParagraph {
                        prefix: "출처".into(),
                        inlines: vec![Inline::Text {
                            value: PREFIX.into(),
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
                ]),
                Some(3),
                1,
                &bytes,
                &mut document,
            )
            .unwrap();
        assert_eq!(output[0].raw_break_type, 4);
        assert_eq!(output.len(), 3);
        assert!(matches!(
            output[1].controls[0],
            Control::NewNumber(NewNumber {
                number_type: AutoNumberType::Picture,
                number: 1
            })
        ));
        let picture = picture(&output);
        assert_eq!(
            picture.common.width,
            (142.0_f64 * 7200.0 / 25.4).round() as u32
        );
        assert_eq!(
            picture.common.height,
            (f64::from(picture.common.width) / 2.0).round() as u32
        );
        assert!(!picture.reverse);
        assert_eq!(picture.crop.right, 15000);
        assert_eq!(picture.img_dim, (15000, 7500));
        let caption = picture.caption.as_ref().unwrap();
        assert_eq!(caption.spacing, 123);
        assert!(caption.include_margin);
        assert_eq!(caption.paragraphs[0].text, format!("그림 3- {CHAPTER}"));
        assert_eq!(caption.paragraphs[1].text, format!("(출처: {PREFIX})"));
        assert_eq!(caption.paragraphs[2].text, "(주1: 둘)");
        let Control::AutoNumber(n) = &caption.paragraphs[0].controls[0] else {
            panic!()
        };
        assert_eq!(
            (
                n.number,
                n.assigned_number,
                n.format,
                n.prefix_char,
                n.suffix_char
            ),
            (1, 1, 3, '(', ')')
        );
        let bold = caption.paragraphs[0].char_shape_id_at(6).unwrap();
        assert!(document.doc_info.char_shapes[bold as usize].bold);
        assert_eq!(document.bin_data_content[0].data.load(), bytes.bytes);
        assert_eq!(document.bin_data_content[0].extension, "jpeg");
        assert_eq!(document.doc_info.bin_data_list[0].attr, 0x21);
        assert_eq!(roots[3].controls.len(), 1);
        let native = picture;
        assert_eq!(native.border_x, [0, 0, native.common.width as i32, 0]);
        assert_eq!(
            native.border_y,
            [
                native.common.width as i32,
                native.common.height as i32,
                0,
                native.common.height as i32
            ]
        );
        assert_eq!(output[1].control_mask, (1 << 11) | (1 << 18));
    }

    #[test]
    fn continuation_retains_native_number_and_omits_absent_source() {
        let (roots, mut document) = fixture();
        let prototype = Prototype::read(&roots, &document.doc_info).unwrap();
        let output = prototype
            .render(
                &[Inline::Text {
                    value: "caption".into(),
                }],
                None,
                Some(0),
                2,
                &resource("png"),
                &mut document,
            )
            .unwrap();
        assert_eq!(output[1].controls.len(), 1);
        let caption = picture(&output).caption.as_ref().unwrap();
        assert_eq!(caption.paragraphs.len(), 1);
        let Control::AutoNumber(n) = &caption.paragraphs[0].controls[0] else {
            panic!()
        };
        assert_eq!(n.number, 2);
    }

    #[test]
    fn rejects_duplicate_misplaced_and_invalid_prototypes() {
        let (mut roots, document) = fixture();
        roots.push(p(CAPTION));
        assert!(Prototype::read(&roots, &document.doc_info).is_err());
        roots.pop();
        roots[0] = p("{{md2hwp:figure.max-width-mm:143}}");
        assert!(Prototype::read(&roots, &document.doc_info).is_err());
        roots[0] = p("{{md2hwp:figure.max-width-mm:142}}");
        let Control::Picture(picture) = &mut roots[3].controls[0] else {
            panic!()
        };
        picture.caption.as_mut().unwrap().paragraphs[0]
            .controls
            .clear();
        assert!(Prototype::read(&roots, &document.doc_info).is_err());
    }

    #[test]
    fn rejects_unsupported_content_counter_overflow_and_degenerate_geometry() {
        let (roots, mut document) = fixture();
        let prototype = Prototype::read(&roots, &document.doc_info).unwrap();
        let text = [Inline::Text {
            value: "title".into(),
        }];
        assert!(
            prototype
                .render(&text, None, None, 1, &resource("png"), &mut document)
                .unwrap_err()
                .contains("preceding heading1")
        );
        assert!(
            prototype
                .render(
                    &[Inline::Footnote { blocks: vec![] }],
                    None,
                    Some(1),
                    1,
                    &resource("png"),
                    &mut document
                )
                .is_err()
        );
        assert!(
            prototype
                .render(&text, None, Some(1), 65536, &resource("png"), &mut document)
                .is_err()
        );
        let mut bad = resource("png");
        bad.height = 0.0;
        assert!(
            prototype
                .render(&text, None, Some(1), 1, &bad, &mut document)
                .is_err()
        );
        assert!(
            prototype
                .render(
                    &text,
                    Some(&[]),
                    Some(1),
                    1,
                    &resource("png"),
                    &mut document
                )
                .is_err()
        );
    }

    #[test]
    fn tracked_template_figures_survive_strict_serialization_with_exact_resources() {
        use md2hwp_core::{
            ValidationLimits,
            ir::{Block, Document, IR_VERSION, ImageRef, Metadata, SCHEMA_NAME},
            validate,
        };
        use std::collections::BTreeMap;
        let mut core =
            rhwp::DocumentCore::from_bytes(include_bytes!("../../../templates/template.hwp"))
                .unwrap();
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
        let mut resources = BTreeMap::new();
        let mut blocks = vec![Block::Heading {
            level: 1,
            id: None,
            inlines: vec![Inline::Text {
                value: "장제목".into(),
            }],
        }];
        for path in [
            "all-features/image.png",
            "jpeg/image.jpg",
            "emf/EmfOnly.emf",
        ] {
            resources.insert(
                blocks.len(),
                crate::image_resource::Resource::read(&root, path).unwrap(),
            );
            blocks.push(Block::Figure {
                id: None,
                image: ImageRef {
                    title: None,
                    path: path.into(),
                    alt: vec![Inline::Text {
                        value: "그림".into(),
                    }],
                },
                caption: vec![Inline::Text {
                    value: "{{md2hwp:num:heading1}}리터럴".into(),
                }],
                source: Some(vec![SourceParagraph {
                    prefix: "주1".into(),
                    inlines: vec![Inline::Text {
                        value: "설명".into(),
                    }],
                }]),
            });
        }
        let ir = validate(
            Document {
                schema: SCHEMA_NAME.into(),
                ir_version: IR_VERSION.into(),
                metadata: Metadata(BTreeMap::from([(
                    "title".into(),
                    md2hwp_core::ir::MetadataValue::Text("그림시험".into()),
                )])),
                blocks,
            },
            &ValidationLimits::default(),
        )
        .unwrap();
        crate::template::render(&ir, &mut core, &resources).unwrap();
        let expected = core.document().clone();
        let result = core.export_hwp_with_adapter_snapshot_with_report().unwrap();
        assert!(result.content_loss().is_empty());
        let reopened = rhwp::DocumentCore::from_bytes(result.bytes()).unwrap();
        let diff =
            rhwp::serializer::hwpx::roundtrip::diff_documents(&expected, reopened.document());
        assert!(diff.is_empty(), "{:?}", diff.differences);
        crate::verify_embedded_bytes(&expected, reopened.document()).unwrap();
        let mut corrupted = reopened.document().clone();
        corrupted.bin_data_content.last_mut().unwrap().data = vec![0].into();
        assert!(crate::verify_embedded_bytes(&expected, &corrupted).is_err());
    }
}
