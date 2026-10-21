// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Euisoon Ahn
//! Ordered body-only table layout and native numbering checks.

use std::collections::BTreeSet;

use rhwp::model::{
    control::{AutoNumberType, Control},
    document::Document,
    page::{BindingMethod, ColumnDef, PageDef},
    paragraph::Paragraph,
};

/// Traverse one control at a time so nested controls precede its next sibling.
/// SECDEF master pages and header/footer content are deliberately excluded.
fn walk(
    paragraphs: &mut [Paragraph],
    visit: &mut impl FnMut(&mut Control) -> Result<(), String>,
) -> Result<(), String> {
    for paragraph in paragraphs {
        for control in &mut paragraph.controls {
            visit(control)?;
            if matches!(
                control,
                Control::SectionDef(_) | Control::Header(_) | Control::Footer(_)
            ) {
                continue;
            }
            if let Control::Table(table) = control {
                // SHAPEOBJECT/CAPTION precedes ROW/CELL in HWPML.
                if let Some(caption) = &mut table.caption {
                    walk(&mut caption.paragraphs, visit)?;
                }
                for cell in &mut table.cells {
                    walk(&mut cell.paragraphs, visit)?;
                }
                continue;
            }
            // Reuse the adapter's exhaustive nested-container traversal without
            // visiting later sibling controls before this control's children.
            let mut owner = Paragraph::default();
            owner
                .controls
                .push(Control::ColumnDef(ColumnDef::default()));
            // Move the real control into its temporary owning paragraph. The
            // placeholder is restored even when recursive validation fails.
            std::mem::swap(control, &mut owner.controls[0]);
            let result = crate::model_tools::map_nested_containers(&mut owner, &mut |children| {
                walk(children, visit)
            });
            std::mem::swap(control, &mut owner.controls[0]);
            result?;
        }
    }
    Ok(())
}

pub(crate) fn require_fits(roots: &[Paragraph], generated: &BTreeSet<u32>) -> Result<(), String> {
    if generated.is_empty() {
        return Ok(());
    }
    let mut page: Option<PageDef> = None;
    let mut columns: Option<ColumnDef> = None;
    let mut seen = BTreeSet::new();
    // Validation operates on detached clones; callers retain all native caches.
    walk(&mut roots.to_vec(), &mut |control| {
        match control {
            Control::SectionDef(section) => page = Some(section.page_def.clone()),
            Control::ColumnDef(value) => columns = Some(value.clone()),
            Control::Table(table) if generated.contains(&table.common.instance_id) => {
                if !seen.insert(table.common.instance_id) {
                    return Err(
                        "Duplicate generated table identity during width validation.".into(),
                    );
                }
                let page = page
                    .as_ref()
                    .ok_or("Generated table has no active SECDEF page definition.")?;
                let available = available_width(page, columns.as_ref())?;
                if table.common.width == 0 || table.common.width > available {
                    return Err(format!(
                        "table.width-mm exceeds the active text column: table={} HWPUNIT, available={available} HWPUNIT.",
                        table.common.width
                    ));
                }
            }
            _ => {}
        }
        Ok(())
    })?;
    if &seen != generated {
        return Err("Generated table identity is missing from the document body.".into());
    }
    Ok(())
}

fn available_width(page: &PageDef, columns: Option<&ColumnDef>) -> Result<u32, String> {
    let paper = if page.landscape {
        page.height
    } else {
        page.width
    };
    if paper == 0 || page.width == 0 || page.height == 0 {
        return Err("Invalid active table page dimensions.".into());
    }
    let gutter = if page.binding == BindingMethod::TopFlip {
        0
    } else {
        page.margin_gutter
    };
    let margins = page
        .margin_left
        .checked_add(page.margin_right)
        .and_then(|value| value.checked_add(gutter))
        .ok_or("Active table page margins overflow.")?;
    let mut usable = paper
        .checked_sub(margins)
        .filter(|value| *value > 0)
        .ok_or("Active table page has no positive text width.")?;
    if let Some(columns) = columns {
        if columns.column_count == 0 || columns.spacing < 0 {
            return Err("Invalid active table column count or gap.".into());
        }
        if columns.column_count > 1 {
            if !columns.same_width {
                return Err("Generated tables require equal-width document columns.".into());
            }
            let count = u32::from(columns.column_count);
            let gaps = u32::try_from(columns.spacing)
                .map_err(|_| "Invalid table column gap.")?
                .checked_mul(count - 1)
                .ok_or("Active table column gaps overflow.")?;
            usable = usable
                .checked_sub(gaps)
                .ok_or("Table column gaps exceed text width.")?
                / count;
            if usable == 0 {
                return Err("Active table column has no positive width.".into());
            }
        }
    }
    Ok(usable)
}

/// Match Hancom's body Table AUTONUM pass, including existing native counters.
pub(crate) fn recalculate_picture_numbers(document: &mut Document) -> Result<(), String> {
    let mut next = u32::from(document.doc_properties.picture_start_num);
    for section in &mut document.sections {
        if section.section_def.picture_num > 0 {
            next = u32::from(section.section_def.picture_num);
        }
        walk(&mut section.paragraphs, &mut |control| {
            match control {
                Control::SectionDef(definition) if definition.picture_num > 0 => {
                    next = u32::from(definition.picture_num)
                }
                Control::NewNumber(number) if number.number_type == AutoNumberType::Picture => {
                    next = u32::from(number.number)
                }
                Control::AutoNumber(number) if number.number_type == AutoNumberType::Picture => {
                    let value = u16::try_from(next)
                        .map_err(|_| "Native Picture numbering exceeds the supported u16 range")?;
                    number.number = value;
                    number.assigned_number = value;
                    next += 1;
                }
                _ => {}
            }
            Ok(())
        })?;
    }
    Ok(())
}

/// Match Hancom's body Table AUTONUM pass, including existing native counters.
/// Static Table AUTONUMs also change: leaving their stored values stale makes
/// parser-computed numbers disagree with the independently composed model.
pub(crate) fn recalculate_numbers(
    document: &mut Document,
    generated: &BTreeSet<u32>,
) -> Result<(), String> {
    if generated.is_empty() {
        return Ok(());
    }
    let mut next = u32::from(document.doc_properties.table_start_num);
    for section in &mut document.sections {
        if section.section_def.table_num > 0 {
            next = u32::from(section.section_def.table_num);
        }
        walk(&mut section.paragraphs, &mut |control| {
            match control {
                Control::SectionDef(definition) if definition.table_num > 0 => {
                    next = u32::from(definition.table_num);
                }
                Control::NewNumber(number) if number.number_type == AutoNumberType::Table => {
                    next = u32::from(number.number);
                }
                Control::AutoNumber(number) if number.number_type == AutoNumberType::Table => {
                    let value = u16::try_from(next)
                        .map_err(|_| "Native Table numbering exceeds the supported u16 range.")?;
                    number.number = value;
                    number.assigned_number = value;
                    next += 1;
                }
                _ => {}
            }
            Ok(())
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rhwp::model::{
        control::{AutoNumber, NewNumber},
        document::{Section, SectionDef},
        header_footer::Header,
        shape::Caption,
        table::{Cell, Table},
    };

    #[test]
    fn picture_reference_counters_follow_document_start_and_native_restarts() {
        let mut doc = rhwp::DocumentCore::new_empty().document().clone();
        doc.doc_properties.picture_start_num = 7;
        let number = || {
            Control::AutoNumber(AutoNumber {
                number_type: AutoNumberType::Picture,
                number: 99,
                ..Default::default()
            })
        };
        doc.sections = vec![Section {
            paragraphs: vec![root(vec![
                number(),
                Control::NewNumber(NewNumber {
                    number_type: AutoNumberType::Picture,
                    number: 1,
                }),
                number(),
                number(),
            ])],
            ..Default::default()
        }];
        recalculate_picture_numbers(&mut doc).unwrap();
        let values: Vec<_> = doc.sections[0].paragraphs[0]
            .controls
            .iter()
            .filter_map(|c| match c {
                Control::AutoNumber(n) => Some((n.number, n.assigned_number)),
                _ => None,
            })
            .collect();
        assert_eq!(values, [(7, 7), (1, 1), (2, 2)]);
    }

    fn root(controls: Vec<Control>) -> Paragraph {
        Paragraph {
            controls,
            ..Paragraph::default()
        }
    }

    fn page() -> PageDef {
        PageDef {
            width: 1000,
            height: 2000,
            margin_left: 100,
            margin_right: 100,
            margin_gutter: 100,
            ..PageDef::default()
        }
    }

    fn sec(page: PageDef) -> Control {
        Control::SectionDef(Box::new(SectionDef {
            page_def: page,
            ..SectionDef::default()
        }))
    }

    fn table(id: u32, width: u32) -> Control {
        let mut value = Table::default();
        value.common.instance_id = id;
        value.common.width = width;
        Control::Table(Box::new(value))
    }

    #[test]
    fn preceding_layout_and_unrelated_static_widths() {
        let ids = BTreeSet::from([1, 2]);
        let mut wider = page();
        wider.width = 1400;
        let roots = vec![root(vec![
            sec(page()),
            table(99, 9000),
            table(1, 700),
            sec(wider),
            table(2, 1100),
        ])];
        assert!(require_fits(&roots, &ids).is_ok());
        let roots = vec![root(vec![sec(page()), table(1, 701)])];
        assert!(require_fits(&roots, &BTreeSet::from([1])).is_err());
    }

    #[test]
    fn gutter_and_landscape_are_strict() {
        assert_eq!(available_width(&page(), None).unwrap(), 700);
        let mut top = page();
        top.binding = BindingMethod::TopFlip;
        assert_eq!(available_width(&top, None).unwrap(), 800);
        top.landscape = true;
        assert_eq!(available_width(&top, None).unwrap(), 1800);
        top.margin_left = u32::MAX;
        assert!(available_width(&top, None).is_err());
        top = page();
        top.width = 300;
        assert!(available_width(&top, None).is_err());
    }

    #[test]
    fn equal_columns_and_later_changes() {
        let columns = ColumnDef {
            column_count: 2,
            same_width: true,
            spacing: 100,
            ..ColumnDef::default()
        };
        assert_eq!(available_width(&page(), Some(&columns)).unwrap(), 300);
        let mut unequal = columns.clone();
        unequal.same_width = false;
        assert!(available_width(&page(), Some(&unequal)).is_err());
        let roots = vec![root(vec![
            sec(page()),
            table(1, 700),
            Control::ColumnDef(columns),
            table(2, 300),
        ])];
        assert!(require_fits(&roots, &BTreeSet::from([1, 2])).is_ok());
        let invalid = ColumnDef {
            column_count: 0,
            ..ColumnDef::default()
        };
        assert!(available_width(&page(), Some(&invalid)).is_err());
    }

    #[test]
    fn missing_section_and_missing_generated_identity_fail() {
        assert!(require_fits(&[root(vec![table(1, 1)])], &BTreeSet::from([1])).is_err());
        assert!(require_fits(&[root(vec![sec(page())])], &BTreeSet::from([1])).is_err());
    }

    #[test]
    fn nested_controls_follow_order_and_headers_do_not_change_layout() {
        let mut header_page = page();
        header_page.width = 1;
        let header = Control::Header(Box::new(Header {
            paragraphs: vec![root(vec![sec(header_page), table(99, 9000)])],
            ..Header::default()
        }));
        let mut outer = Table::default();
        outer.cells.push(Cell {
            paragraphs: vec![root(vec![sec(page()), table(1, 700)])],
            ..Cell::default()
        });
        let roots = vec![root(vec![
            header,
            Control::Table(Box::new(outer)),
            table(2, 700),
        ])];
        assert!(require_fits(&roots, &BTreeSet::from([1, 2])).is_ok());
    }

    fn auto() -> Control {
        Control::AutoNumber(AutoNumber {
            number_type: AutoNumberType::Table,
            number: 99,
            assigned_number: 99,
            ..AutoNumber::default()
        })
    }

    #[test]
    fn numbering_counts_static_captions_and_native_restarts() {
        let mut document = Document::default();
        document.doc_properties.table_start_num = 3;
        let native = Table {
            caption: Some(Caption {
                paragraphs: vec![root(vec![auto()])],
                ..Caption::default()
            }),
            ..Table::default()
        };
        document.sections.push(Section {
            paragraphs: vec![root(vec![
                auto(),
                Control::Table(Box::new(native)),
                Control::NewNumber(NewNumber {
                    number_type: AutoNumberType::Table,
                    number: 12,
                }),
                auto(),
                Control::SectionDef(Box::new(SectionDef {
                    table_num: 20,
                    ..SectionDef::default()
                })),
                auto(),
            ])],
            ..Section::default()
        });
        recalculate_numbers(&mut document, &BTreeSet::from([1])).unwrap();
        let mut numbers = Vec::new();
        walk(&mut document.sections[0].paragraphs, &mut |control| {
            if let Control::AutoNumber(value) = control {
                assert_eq!(value.number, value.assigned_number);
                numbers.push(value.number);
            }
            Ok(())
        })
        .unwrap();
        assert_eq!(numbers, [3, 4, 12, 20]);
    }

    #[test]
    fn numbering_excludes_headers_and_obeys_section_start() {
        let mut document = Document::default();
        document.doc_properties.table_start_num = 2;
        document.sections.push(Section {
            section_def: SectionDef {
                table_num: 7,
                ..SectionDef::default()
            },
            paragraphs: vec![root(vec![
                Control::Header(Box::new(Header {
                    paragraphs: vec![root(vec![auto()])],
                    ..Header::default()
                })),
                auto(),
            ])],
            ..Section::default()
        });
        recalculate_numbers(&mut document, &BTreeSet::from([1])).unwrap();
        let controls = &document.sections[0].paragraphs[0].controls;
        let Control::Header(header) = &controls[0] else {
            panic!("header");
        };
        let Control::AutoNumber(untouched) = &header.paragraphs[0].controls[0] else {
            panic!("number");
        };
        assert_eq!(untouched.number, 99);
        let Control::AutoNumber(body) = &controls[1] else {
            panic!("number");
        };
        assert_eq!((body.number, body.assigned_number), (7, 7));
    }

    #[test]
    fn numbering_rejects_overflow_instead_of_wrapping() {
        let mut document = Document::default();
        document.doc_properties.table_start_num = u16::MAX;
        document.sections.push(Section {
            paragraphs: vec![root(vec![auto(), auto()])],
            ..Section::default()
        });
        assert!(recalculate_numbers(&mut document, &BTreeSet::from([1])).is_err());
    }
}
