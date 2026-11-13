// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Euisoon Ahn
//! HWPX at the file boundary. Composition always works on an HWP model: an HWPX
//! template is lowered to HWP first, and HWPX output is exported from the HWP
//! that already passed the stored-record comparison.

use rhwp::model::{
    control::Control, document::SectionDef, header_footer::HeaderFooterApply, paragraph::Paragraph,
    shape::ObjectNumberingType,
};
use rhwp::{
    DocumentCore,
    model::document::Document,
    parser::{FileFormat, detect_format},
    serializer::hwpx::{
        package_check::check_package,
        roundtrip::{
            diff_exported_hwp_documents, diff_exported_hwpx_documents,
            hwpx_export_expected_document, strip_hwp_to_hwpx_noise, strip_hwpx_to_hwp_noise,
        },
    },
};
use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Hwp,
    Hwpx,
}

impl Format {
    pub fn of(path: &Path, role: &str) -> Result<Self, String> {
        match path.extension() {
            Some(x) if x.eq_ignore_ascii_case("hwp") => Ok(Self::Hwp),
            Some(x) if x.eq_ignore_ascii_case("hwpx") => Ok(Self::Hwpx),
            _ => Err(format!("{role} must have .hwp or .hwpx extension")),
        }
    }

    pub fn suffix(self) -> &'static str {
        match self {
            Self::Hwp => ".hwp",
            Self::Hwpx => ".hwpx",
        }
    }
}

/// Embedded resources as (extension, bytes), ignoring storage IDs: HWPX renumbers them.
fn embedded(document: &Document) -> Vec<(String, Vec<u8>)> {
    let mut items: Vec<_> = document
        .bin_data_content
        .iter()
        .map(|item| {
            (
                item.extension.to_ascii_lowercase(),
                item.data.load().to_vec(),
            )
        })
        .collect();
    items.sort();
    items
}

fn first_differences(diff: &rhwp::serializer::hwpx::roundtrip::IrDiff) -> String {
    format!("{:?}", diff.differences.iter().take(5).collect::<Vec<_>>())
}

fn base_kinds(definition: &SectionDef) -> Vec<HeaderFooterApply> {
    definition
        .master_pages
        .iter()
        .filter(|master| !master.is_extension)
        .map(|master| master.apply_to)
        .collect()
}

fn master_page_kinds(document: &Document) -> Vec<Vec<HeaderFooterApply>> {
    document
        .sections
        .iter()
        .map(|section| base_kinds(&section.section_def))
        .collect()
}

/// Lowers an HWPX template to HWP bytes, proving the HWP keeps what was read.
pub fn lower_template(bytes: &[u8]) -> Result<Vec<u8>, String> {
    if detect_format(bytes) != FileFormat::Hwpx {
        return Err("Template with .hwpx extension must be an HWPX package".into());
    }
    let source = DocumentCore::from_bytes(bytes)
        .map_err(|e| format!("Could not read HWPX template: {e:?}"))?;
    let expected = source.prepare_hwp_export_snapshot().document().clone();
    let lowered = rhwp::serializer::serialize_document_with_report(&expected)
        .map_err(|e| format!("HWPX template could not be converted to HWP: {e}"))?;
    if !lowered.content_loss().is_empty() {
        return Err(format!(
            "Converting the HWPX template to HWP reported content loss: {}",
            lowered.content_loss().to_json()
        ));
    }
    let reopened = DocumentCore::from_bytes(lowered.bytes())
        .map_err(|e| format!("Converted HWPX template could not be reopened: {e:?}"))?;
    let diff = strip_hwpx_to_hwp_noise(diff_exported_hwp_documents(&expected, reopened.document()));
    // HWP stores the base master pages in Both, Even, Odd order and keeps their kinds.
    let mut kinds = master_page_kinds(source.document());
    for section in &mut kinds {
        section.sort_by_key(|kind| match kind {
            HeaderFooterApply::Both => 0,
            HeaderFooterApply::Even => 1,
            HeaderFooterApply::Odd => 2,
        });
    }
    if kinds != master_page_kinds(&expected) || kinds != master_page_kinds(reopened.document()) {
        return Err("Converted HWPX template changed master page kinds".into());
    }
    if !diff.is_empty() {
        return Err(format!(
            "Converted HWPX template differs from its source: {}",
            first_differences(&diff)
        ));
    }
    if embedded(source.document()) != embedded(reopened.document()) {
        return Err("Converting the HWPX template changed embedded resource bytes".into());
    }
    Ok(lowered.into_bytes())
}

/// Instance id and numbering category of every equation, in model order.
fn equation_numbering(document: &Document) -> Vec<(u32, ObjectNumberingType)> {
    fn collect(paragraphs: &mut [Paragraph], found: &mut Vec<(u32, ObjectNumberingType)>) {
        for paragraph in paragraphs {
            for control in &paragraph.controls {
                if let Control::Equation(equation) = control {
                    found.push((equation.common.instance_id, equation.common.numbering_type));
                }
            }
            let _ = crate::model_tools::map_nested_containers(paragraph, &mut |list| {
                collect(list, found);
                Ok(())
            });
        }
    }
    let mut document = document.clone();
    let mut found = Vec::new();
    for section in &mut document.sections {
        collect(&mut section.paragraphs, &mut found);
    }
    found
}

/// rhwp writes numberingType="EQUATION" on every equation, so Hancom also numbers
/// inline equations and shifts the displayed equation numbers. Rewrite the attribute
/// from the model, matching elements by instance id. Remove when rhwp writes the
/// stored category itself.
fn restore_equation_numbering(source: &Document, hwpx: Vec<u8>) -> Result<Vec<u8>, String> {
    const WRITTEN: &str = r#"numberingType="EQUATION""#;
    let equations = equation_numbering(source);
    let mut wanted = BTreeMap::new();
    for (id, kind) in &equations {
        if *kind != ObjectNumberingType::Equation {
            wanted.insert(*id, *kind);
        }
    }
    if wanted.is_empty() {
        return Ok(hwpx);
    }
    if equations
        .iter()
        .any(|(id, kind)| wanted.get(id).is_some_and(|other| other != kind))
    {
        return Err("Equations share an instance id with different numbering categories".into());
    }
    let zip_error = |e: zip::result::ZipError| format!("HWPX package could not be rewritten: {e}");
    let mut archive = zip::ZipArchive::new(Cursor::new(hwpx.as_slice())).map_err(zip_error)?;
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let mut rewritten = 0;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(zip_error)?;
        let name = entry.name().map_err(zip_error)?.into_owned();
        let mut text = String::new();
        if !name.starts_with("Contents/")
            || !name.ends_with(".xml")
            || entry.read_to_string(&mut text).is_err()
            || !text.contains("<hp:equation ")
        {
            drop(entry);
            let raw = archive.by_index_raw(index).map_err(zip_error)?;
            writer.raw_copy_file(raw).map_err(zip_error)?;
            continue;
        }
        let mut patched = String::with_capacity(text.len());
        let mut rest = text.as_str();
        while let Some(start) = rest.find(r#"<hp:equation id=""#) {
            let end = start
                + rest[start..]
                    .find('>')
                    .ok_or("Unclosed hp:equation element")?;
            let tag = &rest[start..end];
            let id = tag[r#"<hp:equation id=""#.len()..]
                .split('"')
                .next()
                .and_then(|id| id.parse::<u32>().ok())
                .ok_or("hp:equation element without a numeric id")?;
            patched.push_str(&rest[..start]);
            match wanted.get(&id) {
                Some(kind) if tag.contains(WRITTEN) => {
                    let value = match kind {
                        ObjectNumberingType::None => "NONE",
                        ObjectNumberingType::Picture => "PICTURE",
                        ObjectNumberingType::Table => "TABLE",
                        ObjectNumberingType::Equation => "EQUATION",
                    };
                    patched.push_str(&tag.replacen(
                        WRITTEN,
                        &format!(r#"numberingType="{value}""#),
                        1,
                    ));
                    rewritten += 1;
                }
                _ => patched.push_str(tag),
            }
            rest = &rest[end..];
        }
        patched.push_str(rest);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(entry.compression())
            .last_modified_time(zip::DateTime::default())
            .system(zip::System::Unix);
        drop(entry);
        writer.start_file(name, options).map_err(zip_error)?;
        writer
            .write_all(patched.as_bytes())
            .map_err(|e| e.to_string())?;
    }
    if rewritten
        != equations
            .iter()
            .filter(|(id, _)| wanted.contains_key(id))
            .count()
    {
        return Err("Saved HWPX does not contain every equation of the verified HWP".into());
    }
    Ok(writer.finish().map_err(zip_error)?.into_inner())
}

/// Exports verified HWP bytes as HWPX and checks the package against that HWP.
pub fn export(hwp: &[u8]) -> Result<Vec<u8>, String> {
    let source = DocumentCore::from_bytes(hwp)
        .map_err(|e| format!("Verified HWP could not be reopened: {e:?}"))?;
    let exported = source
        .export_hwpx_native_with_report()
        .map_err(|e| format!("HWPX serialization failed: {e:?}"))?;
    if !exported.content_loss().is_empty() {
        return Err(format!(
            "HWPX serialization reported content loss: {}",
            exported.content_loss().to_json()
        ));
    }
    let bytes = restore_equation_numbering(source.document(), exported.into_bytes())?;
    verify(&source, &bytes)?;
    Ok(bytes)
}

/// Reopens saved HWPX bytes and compares them with the HWP they were exported from.
pub fn verify(source: &DocumentCore, hwpx: &[u8]) -> Result<(), String> {
    if detect_format(hwpx) != FileFormat::Hwpx {
        return Err("Saved HWPX is not an HWPX package".into());
    }
    let package = check_package(hwpx, source.document());
    if !package.is_ok() {
        return Err(format!(
            "Saved HWPX package is incomplete: {}",
            package.summary()
        ));
    }
    let reopened = DocumentCore::from_bytes(hwpx)
        .map_err(|e| format!("Saved HWPX could not be reopened: {e:?}"))?;
    let diff = strip_hwp_to_hwpx_noise(diff_exported_hwpx_documents(
        &hwpx_export_expected_document(source.document()),
        &hwpx_export_expected_document(reopened.document()),
    ));
    if !diff.is_empty() {
        return Err(format!(
            "Saved HWPX differs from the verified HWP: {}",
            first_differences(&diff)
        ));
    }
    if embedded(source.document()) != embedded(reopened.document()) {
        return Err("Saved HWPX changed embedded resource bytes".into());
    }
    let sorted = |document: &Document| {
        let mut list = equation_numbering(document);
        list.sort_by_key(|(id, kind)| (*id, *kind as u8));
        list
    };
    if sorted(source.document()) != sorted(reopened.document()) {
        return Err("Saved HWPX changed equation numbering categories".into());
    }
    let (before, after) = (source.page_count(), reopened.page_count());
    if before != after {
        return Err(format!(
            "Saved HWPX has {after} pages; the verified HWP has {before}"
        ));
    }
    Ok(())
}
