// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Euisoon Ahn
#![forbid(unsafe_code)]

mod code;
mod conditional_ranges;
mod figures;
mod heading_plan;
mod headings;
mod image_resource;
mod lists;
mod model_tools;
mod table_layout;
mod table_widths;
mod tables;
mod template;

use md2hwp_core::{ValidationLimits, ir::Block, read_ir};
use rhwp::{DocumentCore, serializer::hwpx::roundtrip::diff_documents};
use std::{
    collections::BTreeMap,
    env,
    ffi::OsString,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

const USAGE: &str = "usage: md2hwp-rhwp <source.ir.json> [[--output] <source.output.hwp>] [--template <template.hwp>] [--verbose]\n       md2hwp-rhwp ir2hwp --ir <source.ir.json> --output <source.output.hwp> [--template <template.hwp>] [--verbose]\n       md2hwp-rhwp --version\nExperimental HWP-only backend: paragraphs, heading blocks, lists, code boxes, figures, tables and metadata.\nFigures accept PNG, JPG, JPEG and EMF; shorthand resources stay within the IR directory, ir2hwp resources within the working directory.\nTables require installed Windows TrueType/OpenType fonts.\nDefault template: template.hwp beside this EXE. HWPX requires md2hwp-backend.exe.";

#[derive(Debug)]
struct Options {
    input: PathBuf,
    output: PathBuf,
    template: PathBuf,
    verbose: bool,
    explicit: bool,
}

fn hwp_path(path: &Path, role: &str) -> Result<(), String> {
    if !path
        .extension()
        .is_some_and(|x| x.eq_ignore_ascii_case("hwp"))
    {
        return Err(format!(
            "{role} must have .hwp extension. md2hwp-rhwp does not support HWPX; select md2hwp-backend.exe for HWPX."
        ));
    }
    Ok(())
}

fn default_output(input: &Path) -> PathBuf {
    let mut stem = input.file_stem().unwrap_or_default().to_os_string();
    if let Some(text) = stem.to_str().and_then(|s| s.strip_suffix(".ir")) {
        stem = OsString::from(text);
    }
    stem.push(".output.hwp");
    input.with_file_name(stem)
}

fn parse(args: Vec<OsString>, adjacent: &Path) -> Result<Options, String> {
    let explicit = args.first().is_some_and(|v| v == "ir2hwp");
    let mut iter = args.into_iter().skip(usize::from(explicit));
    let (mut input, mut output, mut template, mut verbose) = (None, None, None, false);
    while let Some(arg) = iter.next() {
        match arg.to_str() {
            Some("--ir" | "--output" | "--template") => {
                let value = iter
                    .next()
                    .filter(|v| !v.to_string_lossy().starts_with('-'))
                    .ok_or_else(|| format!("Missing value for {}", arg.to_string_lossy()))?;
                let slot = match arg.to_str() {
                    Some("--ir") => &mut input,
                    Some("--output") => &mut output,
                    _ => &mut template,
                };
                if slot.replace(PathBuf::from(value)).is_some() {
                    return Err(format!("Duplicate argument: {}", arg.to_string_lossy()));
                }
            }
            Some("--verbose") => {
                if verbose {
                    return Err("Duplicate --verbose".into());
                }
                verbose = true;
            }
            _ if arg.to_string_lossy().starts_with('-') => {
                return Err(format!(
                    "Unsupported option: {}\n{USAGE}",
                    arg.to_string_lossy()
                ));
            }
            _ if explicit => {
                return Err(format!(
                    "Unexpected positional argument: {}",
                    arg.to_string_lossy()
                ));
            }
            _ if input.is_none() => input = Some(PathBuf::from(arg)),
            _ if output.is_none() => output = Some(PathBuf::from(arg)),
            _ => return Err("Too many positional arguments".into()),
        }
    }
    let input = input.ok_or_else(|| USAGE.to_owned())?;
    if explicit && output.is_none() {
        return Err("ir2hwp requires --output".into());
    }
    let output = output.unwrap_or_else(|| default_output(&input));
    let template = template.unwrap_or_else(|| adjacent.join("template.hwp"));
    hwp_path(&template, "Template")?;
    hwp_path(&output, "Output")?;
    Ok(Options {
        input,
        output,
        template,
        verbose,
        explicit,
    })
}

fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|e| format!("{}: {e}", path.display()))?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > limit {
        return Err(format!("{} exceeds the {limit}-byte limit", path.display()));
    }
    Ok(bytes)
}

fn run(options: Options) -> Result<(), String> {
    let input = fs::canonicalize(&options.input)
        .map_err(|e| format!("{}: {e}", options.input.display()))?;
    let template_path = fs::canonicalize(&options.template)
        .map_err(|e| format!("{}: {e}", options.template.display()))?;
    let output = std::path::absolute(&options.output).map_err(|e| e.to_string())?;
    if output.exists() {
        let existing = fs::canonicalize(&output).map_err(|e| e.to_string())?;
        if existing == input
            || existing == template_path
            || existing
                == fs::canonicalize(env::current_exe().map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?
        {
            return Err("Output must not replace the source, template or executable".into());
        }
    }
    let limits = ValidationLimits::default();
    let ir_bytes = read_bounded(&input, limits.max_json_bytes as u64)?;
    let ir = read_ir(&ir_bytes, &limits).map_err(|e| e.to_string())?;
    let base = input.parent().ok_or("Source IR has no parent directory")?;
    let root = if options.explicit {
        env::current_dir().map_err(|e| e.to_string())?
    } else {
        base.to_path_buf()
    };
    let mut resources = BTreeMap::new();
    let mut image_paths = Vec::new();
    for (index, block) in ir.as_document().blocks.iter().enumerate() {
        if let Block::Figure { image, .. } = block {
            let resource = if options.explicit {
                image_resource::Resource::read_with_root(base, &root, &image.path)?
            } else {
                image_resource::Resource::read(base, &image.path)?
            };
            image_paths.push(fs::canonicalize(base.join(&image.path)).map_err(|e| e.to_string())?);
            resources.insert(index, resource);
        }
    }
    protect_image_sources(&output, &image_paths)?;
    let template_bytes = read_bounded(&template_path, 64 * 1024 * 1024)?;
    if !template_bytes.starts_with(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]) {
        return Err(
            "Template must be an HWP compound document; renamed HWPX is not supported".into(),
        );
    }
    let mut doc = DocumentCore::from_bytes(&template_bytes)
        .map_err(|e| format!("Could not read HWP template: {e:?}"))?;
    template::render(&ir, &mut doc, &resources)?;
    let expected = doc.document().clone();
    let serialized = doc
        .export_hwp_with_adapter_snapshot_with_report()
        .map_err(|e| format!("HWP serialization failed: {e:?}"))?;
    if !serialized.content_loss().is_empty() {
        return Err(format!(
            "HWP serialization reported content loss: {}",
            serialized.content_loss().to_json()
        ));
    }
    let parent = output.parent().ok_or("Output has no parent directory")?;
    let mut temporary = tempfile::Builder::new()
        .prefix(".md2hwp-rhwp-")
        .suffix(".hwp")
        .tempfile_in(parent)
        .map_err(|e| e.to_string())?;
    temporary
        .write_all(serialized.bytes())
        .map_err(|e| e.to_string())?;
    temporary.as_file().sync_all().map_err(|e| e.to_string())?;
    let saved = fs::read(temporary.path()).map_err(|e| e.to_string())?;
    if saved != serialized.bytes() {
        return Err("Saved HWP bytes differ from serialized document".into());
    }
    let reopened = DocumentCore::from_bytes(&saved)
        .map_err(|e| format!("Saved HWP could not be reopened: {e:?}"))?;
    verify_embedded_bytes(&expected, reopened.document())?;
    let diff = diff_documents(&expected, reopened.document());
    if !diff.is_empty() {
        return Err(format!(
            "Saved HWP differs from the composed document: {:?}",
            diff.differences.iter().take(5).collect::<Vec<_>>()
        ));
    }
    if read_bounded(&input, limits.max_json_bytes as u64)? != ir_bytes
        || read_bounded(&template_path, 64 * 1024 * 1024)? != template_bytes
    {
        return Err(
            "Source IR or template changed during generation; output was not replaced".into(),
        );
    }
    for resource in resources.values() {
        resource.verify_unchanged()?;
    }
    protect_image_sources(&output, &image_paths)?;
    temporary
        .persist(&output)
        .map_err(|e| format!("Could not publish {}: {}", output.display(), e.error))?;
    if options.verbose {
        eprintln!("rhwp: saved and reopened HWP; model comparison passed");
    }
    println!("{}", output.display());
    Ok(())
}

fn protect_image_sources(output: &Path, image_paths: &[PathBuf]) -> Result<(), String> {
    if output.exists() {
        let output = fs::canonicalize(output).map_err(|e| e.to_string())?;
        if image_paths.contains(&output) {
            return Err("Output must not replace a source image".into());
        }
    }
    Ok(())
}

fn verify_embedded_bytes(
    expected: &rhwp::model::document::Document,
    actual: &rhwp::model::document::Document,
) -> Result<(), String> {
    if expected.bin_data_content.len() != actual.bin_data_content.len()
        || expected.bin_data_content.iter().any(|source| {
            !actual.bin_data_content.iter().any(|saved| {
                saved.id == source.id
                    && saved.extension == source.extension
                    && saved.data.load() == source.data.load()
            })
        })
    {
        return Err("Saved HWP changed embedded resource identities or original bytes".into());
    }
    Ok(())
}

fn main() {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() == 1 && args[0] == "--version" {
        println!(
            "md2hwp-rhwp {} (experimental; IR {}; rhwp 0.8.7)\nLicense: AGPL-3.0-only",
            env!("CARGO_PKG_VERSION"),
            md2hwp_core::ir::IR_VERSION
        );
        return;
    }
    if args.is_empty() || (args.len() == 1 && (args[0] == "--help" || args[0] == "-h")) {
        println!("{USAGE}");
        return;
    }
    let result = env::current_exe()
        .map_err(|e| e.to_string())
        .and_then(|exe| {
            parse(
                args,
                exe.parent().ok_or("Executable has no parent directory")?,
            )
        })
        .and_then(run);
    if let Err(error) = result {
        eprintln!("md2hwp-rhwp: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn options(args: &[&str]) -> Result<Options, String> {
        parse(
            args.iter().map(OsString::from).collect(),
            Path::new("deploy"),
        )
    }
    #[test]
    fn defaults_and_explicit_mode() {
        let a = options(&["source.ir.json"]).unwrap();
        assert_eq!(a.output, PathBuf::from("source.output.hwp"));
        assert_eq!(a.template, PathBuf::from("deploy/template.hwp"));
        assert!(!a.explicit);
        let a = options(&[
            "ir2hwp",
            "--output",
            "out.hwp",
            "--ir",
            "in.json",
            "--template",
            "custom.hwp",
        ])
        .unwrap();
        assert_eq!(a.input, PathBuf::from("in.json"));
        assert!(a.explicit);
        assert!(!options(&["--ir", "in.json"]).unwrap().explicit);
    }
    #[test]
    fn hwpx_and_duplicate_arguments_are_rejected() {
        for args in [
            vec!["a.json", "a.HWPX"],
            vec!["a.json", "--template", "a.hwpx"],
            vec!["a.json", "b.hwp", "--output", "c.hwp"],
            vec!["--ir", "a.json", "--ir", "b.json"],
        ] {
            assert!(options(&args).is_err());
        }
        assert!(options(&["a.json", "a.HWP"]).is_ok());
    }

    #[test]
    fn output_cannot_replace_a_source_image() {
        let root = tempfile::tempdir().unwrap();
        let image = root.path().join("image.png");
        fs::write(&image, b"image bytes").unwrap();
        let protected = vec![fs::canonicalize(&image).unwrap()];
        assert!(protect_image_sources(&image, &protected).is_err());
        assert!(protect_image_sources(&root.path().join("result.hwp"), &protected).is_ok());
    }
}
