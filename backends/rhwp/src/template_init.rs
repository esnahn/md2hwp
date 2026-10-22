// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Euisoon Ahn
//! Author a complete, unstyled IR template without loading a template document.

use std::collections::BTreeMap;

use rhwp::{
    DocumentCore,
    model::{
        Padding,
        control::{AutoNumber, AutoNumberType, Control},
        document::{DocInfo, DocProperties, Document, FileHeader, HwpVersion, Section, SectionDef},
        footnote::{Footnote, FootnoteShape},
        page::{PageBorderFill, PageBorderUiBasis, PageDef},
        paragraph::Paragraph,
        shape::{
            Caption, CaptionDirection, CommonObjAttr, HorzRelTo, ObjectNumberingType, TextWrap,
            VertRelTo,
        },
        style::{
            BorderFill, BorderLine, BorderLineType, Bullet, CharShape, DiagonalLine, Fill,
            FillType, Font, HeadType, Numbering, NumberingHead, ParaShape, SolidFill, Style,
            TabDef,
        },
        table::{Cell, Table, TablePageBreak, VerticalAlign},
    },
};

const WIDTH: u32 = 40_252; // 142 mm, rounded to HWPUNIT.
// A generated one-pixel sample, also used by Hancom's default-template authoring.
// This is image data only; no document/template bytes are embedded here.
const SAMPLE_PNG: &[u8] = &[
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 2, 0,
    0, 0, 144, 119, 83, 222, 0, 0, 0, 12, 73, 68, 65, 84, 120, 156, 99, 184, 123, 247, 46, 0, 5,
    50, 2, 152, 195, 13, 250, 143, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
];

fn paragraph(text: &str, style: u8, shape: u16) -> Paragraph {
    let mut paragraph = Paragraph::new_empty();
    paragraph.set_single_char_shape(0);
    paragraph.style_id = style;
    paragraph.para_shape_id = shape;
    paragraph.insert_text_at(0, text);
    crate::template::discard_stale_layout(&mut paragraph);
    paragraph
}

/// Put one native extended control between two plain text fragments. Each HWP5
/// extended control consumes eight UTF-16 units and is absent from plain text.
fn controlled(before: &str, after: &str, control: Control, style: u8) -> Paragraph {
    let mut paragraph = paragraph(&format!("{before}{after}"), style, 0);
    for position in paragraph
        .char_offsets
        .iter_mut()
        .skip(before.chars().count())
    {
        *position += 8;
    }
    paragraph.char_count += 8;
    paragraph.has_para_text = true;
    paragraph.controls.push(control);
    paragraph.align_ctrl_data_records();
    paragraph
}

fn number(kind: AutoNumberType) -> Control {
    Control::AutoNumber(AutoNumber {
        number_type: kind,
        number: 1,
        assigned_number: 1,
        ..Default::default()
    })
}

fn caption(paragraphs: Vec<Paragraph>, direction: CaptionDirection) -> Caption {
    Caption {
        direction,
        width: WIDTH,
        max_width: WIDTH,
        spacing: 283,
        paragraphs,
        ..Default::default()
    }
}

fn blank() -> Document {
    let none = BorderLine {
        line_type: BorderLineType::None,
        ..Default::default()
    };
    let solid = BorderLine {
        line_type: BorderLineType::Solid,
        width: 1,
        color: 0,
    };
    let diagonal = DiagonalLine {
        diagonal_type: 1,
        ..Default::default()
    };
    let note_shape = FootnoteShape {
        start_number: 1,
        suffix_char: ')',
        separator_length: -1,
        separator_margin_top: -1,
        separator_margin_bottom: 850,
        note_spacing: 567,
        separator_line_type: 1,
        separator_line_width: 1,
        raw_unknown: 283,
        ..Default::default()
    };
    let endnote_shape = FootnoteShape {
        separator_length: 12280,
        separator_margin_top: 224,
        raw_unknown: 0,
        ..note_shape.clone()
    };
    // Hancom requires the both/even/odd PAGE_BORDER_FILL records, including a
    // valid one-based border reference, even when the visible border is empty.
    let page_border = PageBorderFill {
        attr: 1,
        spacing_left: 1417,
        spacing_right: 1417,
        spacing_top: 1417,
        spacing_bottom: 1417,
        border_fill_id: 1,
        ui_basis: PageBorderUiBasis::Page,
        ..Default::default()
    };
    let base = ParaShape {
        line_spacing: 160,
        line_spacing_v2: 160,
        ..Default::default()
    };
    let mut bullet_shape = base.clone();
    bullet_shape.head_type = HeadType::Bullet;
    bullet_shape.numbering_id = 1;
    bullet_shape.attr1 |= (HeadType::Bullet as u32) << 23;
    let mut ordered_shape = base.clone();
    ordered_shape.head_type = HeadType::Number;
    ordered_shape.numbering_id = 1;
    ordered_shape.attr1 |= (HeadType::Number as u32) << 23;
    Document {
        header: FileHeader {
            version: HwpVersion {
                major: 5,
                minor: 1,
                build: 0,
                revision: 0,
            },
            compressed: true,
            flags: 1,
            ..Default::default()
        },
        doc_properties: DocProperties {
            section_count: 1,
            page_start_num: 1,
            footnote_start_num: 1,
            endnote_start_num: 1,
            picture_start_num: 1,
            table_start_num: 1,
            equation_start_num: 1,
            ..Default::default()
        },
        doc_info: DocInfo {
            font_faces: vec![
                vec![Font {
                    name: "함초롬바탕".into(),
                    alt_type: 1,
                    ..Default::default()
                }];
                7
            ],
            char_shapes: vec![CharShape {
                base_size: 1000,
                border_fill_id: 4,
                ratios: [100; 7],
                relative_sizes: [100; 7],
                ..Default::default()
            }],
            para_shapes: vec![base, bullet_shape, ordered_shape],
            tab_defs: vec![TabDef::default()],
            styles: vec![Style {
                local_name: "바탕글".into(),
                english_name: "Normal".into(),
                lang_id: 1042,
                ..Default::default()
            }],
            border_fills: vec![
                BorderFill {
                    borders: [none; 4],
                    diagonal,
                    ..Default::default()
                },
                BorderFill {
                    borders: [solid; 4],
                    diagonal,
                    ..Default::default()
                },
                BorderFill {
                    borders: [none, none, solid, none],
                    diagonal,
                    ..Default::default()
                },
                // A native CharShape reapplication materializes a transparent
                // brush. Author it explicitly so inserting reference fields
                // preserves the source character formatting exactly.
                BorderFill {
                    borders: [none; 4],
                    diagonal,
                    fill: Fill {
                        fill_type: FillType::Solid,
                        solid: Some(SolidFill {
                            background_color: u32::MAX,
                            pattern_color: 0x0099_9999,
                            pattern_type: -1,
                        }),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ],
            bullets: vec![Bullet {
                bullet_char: '●',
                text_distance: 50,
                char_shape_id: 0,
                ..Default::default()
            }],
            numberings: vec![Numbering {
                heads: [NumberingHead {
                    text_distance: 50,
                    char_shape_id: 0,
                    ..Default::default()
                }; 7],
                level_formats: std::array::from_fn(|level| format!("^{}.", level + 1)),
                start_number: 1,
                level_start_numbers: [1; 7],
                ..Default::default()
            }],
            ..Default::default()
        },
        sections: vec![Section {
            section_def: SectionDef {
                page_def: PageDef {
                    height: 84186,
                    margin_top: 5668,
                    ..PageDef::a4_default()
                },
                // Hancom supplies 1134 when an omitted zero is reimported.
                // Author its fresh-document defaults so strict XML imports do
                // not change section geometry during native generation.
                column_spacing: 1134,
                default_tab_spacing: 8000,
                outline_numbering_id: 1,
                page_border_fill: page_border.clone(),
                extra_page_border_fills: vec![page_border; 2],
                footnote_shape: note_shape.clone(),
                endnote_shape,
                ..Default::default()
            },
            paragraphs: vec![paragraph("", 0, 0)],
            ..Default::default()
        }],
        ..Default::default()
    }
}

fn cell(row: u16, col: u16, width: u32, text: &str, style: u8, border: u16) -> Cell {
    let mut cell = Cell::new_empty(col, row, width, 1282, border);
    cell.padding = Padding {
        left: 510,
        right: 510,
        top: 141,
        bottom: 141,
    };
    cell.vertical_align = VerticalAlign::Center;
    let mut content = paragraph(text, style, 0);
    content.char_count_msb = true;
    cell.paragraphs = vec![content];
    cell
}

fn table(cells: Vec<Cell>, rows: u16, columns: u16, id: u32, inline: bool) -> Table {
    let mut table = Table {
        row_count: rows,
        col_count: columns,
        row_sizes: if columns == 2 { vec![2, 2, 1] } else { vec![1] },
        border_fill_id: 2,
        cells,
        page_break: if inline {
            TablePageBreak::None
        } else {
            TablePageBreak::CellBreak
        },
        repeat_header: !inline,
        common: CommonObjAttr {
            ctrl_id: u32::from_be_bytes(*b"tbl "),
            instance_id: id,
            z_order: id as i32,
            width: WIDTH,
            height: 1282 * u32::from(rows),
            treat_as_char: inline,
            vert_rel_to: VertRelTo::Para,
            horz_rel_to: HorzRelTo::Para,
            text_wrap: TextWrap::TopAndBottom,
            numbering_type: if inline {
                ObjectNumberingType::None
            } else {
                ObjectNumberingType::Table
            },
            ..Default::default()
        },
        ..Default::default()
    };
    table.rebuild_grid();
    table
}

pub(crate) fn create() -> Result<Document, String> {
    let mut core = DocumentCore::new_empty();
    core.set_document(blank());
    // The native picture constructor supplies HWP geometry and resource records;
    // its input is the generated image above, never an existing document.
    core.insert_picture_native(
        0,
        0,
        0,
        &[],
        SAMPLE_PNG,
        WIDTH,
        3402,
        1,
        1,
        "png",
        "서식 표본",
        None,
        None,
    )
    .map_err(|error| format!("Could not author the template picture: {error:?}"))?;
    let mut document = core.document().clone();
    let mut picture = document.sections[0]
        .paragraphs
        .iter()
        .flat_map(|p| &p.controls)
        .find_map(|control| match control {
            Control::Picture(picture) => Some((**picture).clone()),
            _ => None,
        })
        .ok_or("Native picture constructor produced no picture")?;

    let mut roles: Vec<String> = [
        "body",
        "heading1",
        "heading2",
        "heading3",
        "heading4",
        "heading5",
        "heading6",
        "footnote",
        "footnote.next",
        "code",
        "code.title",
        "code.source",
        "figure.caption",
        "figure.source",
        "table.header",
        "table.content",
        "table.source",
        "table.caption",
        "ref.figure.number",
        "ref.table.number",
        "reset",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    roles.extend((1..=6).map(|level| format!("ref.heading{level}.number")));
    let mut styles = BTreeMap::new();
    for role in &roles {
        let id =
            u8::try_from(document.doc_info.styles.len()).map_err(|_| "Too many template styles")?;
        styles.insert(role.clone(), id);
        document.doc_info.styles.push(Style {
            local_name: format!("md2hwp.{role}"),
            english_name: format!("md2hwp.{role}"),
            next_style_id: 1,
            lang_id: 1042,
            ..Default::default()
        });
    }
    let text = |role: &str, value: &str| paragraph(value, styles[role], 0);
    let declaration = |name: &str| text("body", &crate::template::tag(name));
    let source = |role: &str| {
        text(
            &format!("{role}.source"),
            &format!(
                "{}: {}",
                crate::template::tag(&format!("slot:{role}.source.prefix")),
                crate::template::tag(&format!("slot:{role}.source"))
            ),
        )
    };

    let mut roots = vec![
        controlled(
            "",
            &crate::template::tag("begin:template"),
            Control::SectionDef(Box::new(document.sections[0].section_def.clone())),
            styles["body"],
        ),
        declaration("ir-version:0.4"),
    ];
    for role in [
        "body", "heading1", "heading2", "heading3", "heading4", "heading5", "heading6", "reset",
    ] {
        roots.push(text(role, &crate::template::tag(role)));
    }
    let first_note = controlled(
        "",
        &format!(" {}", crate::template::tag("footnote")),
        number(AutoNumberType::Footnote),
        styles["footnote"],
    );
    let note = Footnote {
        number: 1,
        after_decoration_letter: u16::from(b')'),
        instance_id: 1_000_000,
        paragraphs: vec![
            first_note,
            text("footnote.next", &crate::template::tag("footnote.next")),
        ],
        ..Default::default()
    };
    roots.push(controlled(
        "각주 서식 표본",
        "",
        Control::Footnote(Box::new(note)),
        styles["body"],
    ));
    roots.extend([
        declaration("list.max-depth:6"),
        declaration("list.indent-hwp:1000"),
        paragraph(&crate::template::tag("list.bullet"), styles["body"], 1),
        paragraph(&crate::template::tag("list.ordered"), styles["body"], 2),
    ]);

    let mut code_cell = cell(
        0,
        0,
        WIDTH,
        &crate::template::tag("slot:code.title"),
        styles["code.title"],
        2,
    );
    let mut content = text("code", &crate::template::tag("slot:code.content"));
    content.char_count_msb = true;
    code_cell.paragraphs.push(content);
    let mut code = table(vec![code_cell], 1, 1, 100, true);
    code.caption = Some(caption(vec![source("code")], CaptionDirection::Bottom));
    roots.extend([
        declaration("begin:code"),
        controlled("", "", Control::Table(Box::new(code)), styles["body"]),
        declaration("end:code"),
    ]);

    let numbered_caption = |kind: AutoNumberType, role: &str, label: &str| {
        controlled(
            &format!("[{label} {}-", crate::template::tag("num:heading1")),
            &format!("] {}", crate::template::tag(&format!("slot:{role}"))),
            number(kind),
            styles[role],
        )
    };
    picture.common.attr = 0x240a_2211;
    picture.common.hwp5_gen_shape_attr_bit26 = true;
    picture.common.affect_line_spacing = false;
    picture.common.flow_with_text = true;
    picture.common.vert_rel_to = VertRelTo::Para;
    picture.common.horz_rel_to = HorzRelTo::Column;
    picture.common.treat_as_char = true;
    picture.common.numbering_type = ObjectNumberingType::Picture;
    picture.caption = Some(caption(
        vec![
            numbered_caption(AutoNumberType::Picture, "figure.caption", "그림"),
            source("figure"),
        ],
        CaptionDirection::Bottom,
    ));
    roots.extend([
        declaration("figure.max-width-mm:142"),
        declaration("begin:figure"),
        controlled("", "", Control::Picture(Box::new(picture)), styles["body"]),
        declaration("end:figure"),
    ]);

    let mut cells = Vec::new();
    for row in 0..2 {
        for col in 0..2 {
            let role = if row == 0 {
                "table.header"
            } else {
                "table.content"
            };
            let mut sample = cell(
                row,
                col,
                WIDTH / 2,
                &crate::template::tag(&format!("slot:{role}")),
                styles[role],
                2,
            );
            sample.set_header(row == 0);
            cells.push(sample);
        }
    }
    let mut source_cell = cell(2, 0, WIDTH, "", styles["table.source"], 3);
    source_cell.col_span = 2;
    let mut source_paragraph = source("table");
    source_paragraph.char_count_msb = true;
    source_cell.paragraphs = vec![source_paragraph];
    cells.push(source_cell);
    let mut manuscript_table = table(cells, 3, 2, 200, false);
    manuscript_table.caption = Some(caption(
        vec![numbered_caption(
            AutoNumberType::Table,
            "table.caption",
            "표",
        )],
        CaptionDirection::Top,
    ));
    roots.extend([
        declaration("table.width-mm:142"),
        declaration("begin:table"),
        controlled(
            "",
            "",
            Control::Table(Box::new(manuscript_table)),
            styles["body"],
        ),
        declaration("end:table"),
    ]);

    for (kind, label) in [("figure", "그림"), ("table", "표")] {
        let role = format!("ref.{kind}.number");
        roots.extend([
            declaration(&format!("begin:{role}")),
            text(
                &role,
                &format!(
                    "{label} {}-{}",
                    crate::template::tag("num:heading1"),
                    crate::template::tag(&format!("slot:{role}"))
                ),
            ),
            declaration(&format!("end:{role}")),
        ]);
    }
    for level in 1..=6 {
        let role = format!("ref.heading{level}.number");
        let numbers = (1..=level)
            .map(|number| crate::template::tag(&format!("num:heading{number}")))
            .collect::<Vec<_>>()
            .join(".");
        let value = match level {
            1 => format!("제{numbers}장"),
            2 => format!("{numbers}절"),
            _ => format!("{numbers}항"),
        };
        roots.extend([
            declaration(&format!("begin:{role}")),
            text(&role, &value),
            declaration(&format!("end:{role}")),
        ]);
    }
    roots.extend([
        declaration("end:template"),
        declaration("content"),
        text("body", ""),
    ]);
    document.sections[0].paragraphs = roots;
    document.sections[0].raw_stream = None;
    document.sections[0].raw_provenance = None;
    document.doc_info.raw_stream = None;
    document.doc_info.raw_stream_dirty = true;
    document.doc_info.raw_provenance = None;
    Ok(document)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authors_default_formats_and_complete_native_samples() {
        let document = create().unwrap();
        assert_eq!(document.sections.len(), 1);
        assert_eq!(
            document.sections[0]
                .section_def
                .extra_page_border_fills
                .len(),
            2
        );
        assert_eq!(
            document.sections[0]
                .section_def
                .page_border_fill
                .border_fill_id,
            1
        );
        assert_eq!(document.doc_info.font_faces[0][0].name, "함초롬바탕");
        assert_eq!(document.doc_info.char_shapes[0].base_size, 1000);
        assert!(
            document
                .doc_info
                .font_faces
                .iter()
                .flatten()
                .all(|font| font.alt_type == 1)
        );
        let roots = &document.sections[0].paragraphs;
        crate::footnotes::Sample::read(roots, &document.doc_info).unwrap();
        crate::lists::Lists::read(roots, &document.doc_info).unwrap();
        crate::code::Prototype::read(roots, &roots[2], &document.doc_info).unwrap();
        crate::figures::Prototype::read(roots, &document.doc_info).unwrap();
        crate::tables::Prototype::read(roots, &document.doc_info).unwrap();
        crate::heading_references::Samples::read(roots).unwrap();
    }
}
