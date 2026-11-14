// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Euisoon Ahn
//! Standalone SVG converter using the same code as md2hwp's SVG figures.

#![forbid(unsafe_code)]

use md2hwp_svg::Target;
use std::{
    ffi::OsString,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

const USAGE: &str = "usage: svg2emf <source.svg> [[--output] <file>]
Converts an SVG drawing. The output extension selects the format:
  .emf          EMF+ Dual vector metafile (default: source.emf beside the SVG)
  .png          PNG, transparent background
  .jpg, .jpeg   JPEG on a white background
An .emf output fails when the drawing needs something EMF cannot hold here
(gradients, masks, filters, opacity over several differently coloured shapes);
choose a .png or .jpg output for such drawings.
The source is never modified. An existing output is replaced only on success.
  --version     Show the version.
  -h, --help    Show this help.
svg2emf is free software under the GNU AGPL-3.0-only license; see LICENSE.";

struct Options {
    input: PathBuf,
    output: PathBuf,
    target: Target,
}

fn parse(arguments: Vec<OsString>) -> Result<Options, String> {
    let (mut input, mut output) = (None, None);
    let mut arguments = arguments.into_iter();
    while let Some(argument) = arguments.next() {
        if argument == "--output" {
            let value = arguments.next().ok_or("missing value for --output")?;
            if output.replace(PathBuf::from(value)).is_some() {
                return Err("duplicate output".into());
            }
        } else if argument.to_string_lossy().starts_with('-') {
            return Err(format!(
                "unsupported option: {}\n{USAGE}",
                argument.to_string_lossy()
            ));
        } else if input.is_none() {
            input = Some(PathBuf::from(argument));
        } else if output.replace(PathBuf::from(argument)).is_some() {
            return Err("duplicate output".into());
        }
    }
    let input = input.ok_or(USAGE)?;
    if !extension(&input).is_some_and(|x| x == "svg") {
        return Err("source must have .svg extension".into());
    }
    let output = output.unwrap_or_else(|| input.with_extension("emf"));
    let target = match extension(&output).as_deref() {
        Some("emf") => Target::Emf,
        Some("png") => Target::Png,
        Some("jpg" | "jpeg") => Target::Jpg,
        _ => return Err("output must have .emf, .png, .jpg or .jpeg extension".into()),
    };
    Ok(Options {
        input,
        output,
        target,
    })
}

fn extension(path: &Path) -> Option<String> {
    Some(path.extension()?.to_str()?.to_ascii_lowercase())
}

fn run(options: Options) -> Result<PathBuf, String> {
    let file = fs::File::open(&options.input)
        .map_err(|e| format!("cannot open {}: {e}", options.input.display()))?;
    let mut svg = Vec::new();
    file.take(md2hwp_svg::MAX_BYTES + 1)
        .read_to_end(&mut svg)
        .map_err(|e| format!("cannot read {}: {e}", options.input.display()))?;
    let converted = md2hwp_svg::convert(&svg, options.target).map_err(|error| {
        if options.target == Target::Emf && error.contains("cannot be stored as EMF") {
            format!("{error}; choose a .png or .jpg output")
        } else {
            error
        }
    })?;
    let output = std::path::absolute(&options.output).map_err(|e| e.to_string())?;
    let same = |a: &Path, b: &Path| matches!((fs::canonicalize(a), fs::canonicalize(b)), (Ok(a), Ok(b)) if a == b);
    if same(&options.input, &output) {
        return Err("output would replace the source".into());
    }
    let parent = output.parent().ok_or("output has no parent directory")?;
    let mut temporary = tempfile::Builder::new()
        .prefix(".svg2emf-")
        .tempfile_in(parent)
        .map_err(|e| format!("cannot write in {}: {e}", parent.display()))?;
    temporary
        .write_all(&converted.bytes)
        .map_err(|e| e.to_string())?;
    temporary.as_file().sync_all().map_err(|e| e.to_string())?;
    temporary
        .persist(&output)
        .map_err(|e| format!("could not write {}: {}", output.display(), e.error))?;
    Ok(output)
}

fn main() {
    let arguments: Vec<OsString> = std::env::args_os().skip(1).collect();
    match arguments.first().and_then(|a| a.to_str()) {
        Some("--help" | "-h") if arguments.len() == 1 => return println!("{USAGE}"),
        Some("--version") if arguments.len() == 1 => {
            return println!("svg2emf {}", env!("CARGO_PKG_VERSION"));
        }
        _ => {}
    }
    match parse(arguments).and_then(run) {
        Ok(output) => println!("{}", output.display()),
        Err(error) => {
            eprintln!("svg2emf: {error}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(arguments: &[&str]) -> Result<Options, String> {
        parse(arguments.iter().map(Into::into).collect())
    }

    #[test]
    fn output_extension_selects_the_format() {
        let default = options(&["a/b.SVG"]).unwrap();
        assert_eq!(default.output, PathBuf::from("a/b.emf"));
        assert_eq!(default.target, Target::Emf);
        assert_eq!(options(&["b.svg", "c.PNG"]).unwrap().target, Target::Png);
        let named = options(&["--output", "c.jpeg", "b.svg"]).unwrap();
        assert_eq!((named.target, named.output), (Target::Jpg, "c.jpeg".into()));
        for bad in [
            vec![],
            vec!["b.png"],
            vec!["b.svg", "c.gif"],
            vec!["b.svg", "c.emf", "d.emf"],
            vec!["b.svg", "--output"],
            vec!["b.svg", "--svg", "png"],
        ] {
            assert!(options(&bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn converts_refuses_unsupported_emf_and_keeps_existing_output() {
        let directory = tempfile::tempdir().unwrap();
        let svg = directory.path().join("d.svg");
        let shape = r##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="5" height="5" fill="#123456" EXTRA/></svg>"##;
        fs::write(&svg, shape.replace(" EXTRA", "")).unwrap();
        let path = |name: &str| directory.path().join(name).to_str().unwrap().to_owned();
        let emf = run(options(&[&path("d.svg")]).unwrap()).unwrap();
        assert_eq!(emf, directory.path().join("d.emf"));
        let first = fs::read(&emf).unwrap();
        assert!(md2hwp_svg::is_generated(&first));
        run(options(&[&path("d.svg"), &path("d.jpg")]).unwrap()).unwrap();
        assert!(
            fs::read(directory.path().join("d.jpg"))
                .unwrap()
                .starts_with(&[0xff, 0xd8])
        );
        // A drawing EMF cannot hold fails and leaves the earlier output alone.
        let blurred = shape.replace("EXTRA", r#"filter="url(#f)""#).replace(
            "<rect",
            r#"<filter id="f"><feGaussianBlur stdDeviation="1"/></filter><rect"#,
        );
        fs::write(&svg, blurred).unwrap();
        let error = run(options(&[&path("d.svg")]).unwrap()).unwrap_err();
        assert!(error.contains("a filter"), "{error}");
        assert_eq!(fs::read(&emf).unwrap(), first);
        run(options(&[&path("d.svg"), &path("d.png")]).unwrap()).unwrap();
        assert!(run(options(&[&path("missing.svg")]).unwrap()).is_err());
    }
}
