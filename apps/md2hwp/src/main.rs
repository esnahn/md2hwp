#![forbid(unsafe_code)]
//! User-facing `md2hwp` command-line application.

use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{self, Command, Stdio};

use md2hwp_core::ir::Block;
use md2hwp_core::validate::validate;
use md2hwp_core::{
    ValidationLimits, load_builtin_rules, normalize_pandoc, read_pandoc_json, write_ir,
};

const FAILURE_EXIT_CODE: i32 = 1;
mod pandoc_setup;

#[path = "../../../backends/hancom-automation/launcher.rs"]
mod hancom;

fn main() {
    if let Err(error) = run(env::args_os().skip(1).collect()) {
        eprintln!("md2hwp: {}", error.message);
        process::exit(error.exit_code);
    }
}

struct AppError {
    message: String,
    exit_code: i32,
}

impl AppError {
    fn pandoc(message: impl Into<String>, exit_code: Option<i32>) -> Self {
        Self {
            message: message.into(),
            exit_code: child_exit_code(exit_code),
        }
    }
}

impl From<String> for AppError {
    fn from(message: String) -> Self {
        Self {
            message,
            exit_code: FAILURE_EXIT_CODE,
        }
    }
}

fn child_exit_code(exit_code: Option<i32>) -> i32 {
    exit_code
        .filter(|exit_code| *exit_code != 0)
        .unwrap_or(FAILURE_EXIT_CODE)
}

fn run(arguments: Vec<std::ffi::OsString>) -> Result<(), AppError> {
    if arguments.first().is_some_and(|a| a == "setup-pandoc") {
        if arguments.len() != 1 {
            return Err("usage: md2hwp setup-pandoc".to_owned().into());
        }
        return pandoc_setup::install().map_err(AppError::from);
    }
    if arguments
        .first()
        .is_some_and(|a| a == "check-runtime" || a == "render-hwp")
    {
        return hancom::run(arguments).map_err(AppError::from);
    }
    let options = Options::parse(arguments)?;
    let source = fs::read(&options.input)
        .map_err(|error| format!("could not read {}: {error}", options.input.display()))?;
    ensure_distinct_paths(&options.input, &options.output)?;
    let (pandoc_json, reader) = match options.input_format {
        InputFormat::CommonMark => {
            std::str::from_utf8(&source)
                .map_err(|error| format!("source is not UTF-8 at byte {}", error.valid_up_to()))?;
            let pandoc = options
                .pandoc
                .as_deref()
                .ok_or_else(|| "internal error: CommonMark requires Pandoc".to_owned())?;
            verify_pandoc(pandoc)?;
            (invoke_pandoc(pandoc, &source)?, "commonmark")
        }
        InputFormat::PandocJson => (source, "pandoc-json"),
    };

    let limits = ValidationLimits::default();
    let pandoc = read_pandoc_json(&pandoc_json, &limits).map_err(|error| error.to_string())?;
    let rules = load_builtin_rules().map_err(|error| error.to_string())?;
    let ir =
        normalize_pandoc(pandoc, &rules, reader, &limits).map_err(|error| error.to_string())?;
    let mut document = ir.into_document();
    for block in &mut document.blocks {
        if let Block::Figure { image, .. } = block {
            image.path = rebase_image_path(&image.path, &options.input, &options.output)?;
        }
    }
    let ir = validate(document, &limits).map_err(|error| error.to_string())?;
    let serialized = write_ir(&ir).map_err(|error| error.to_string())?;

    if options.output.exists() && !options.force {
        return Err(format!(
            "output already exists: {} (pass --force to replace it)",
            options.output.display()
        )
        .into());
    }
    if let Some(parent) = options.output.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
    }
    fs::write(&options.output, serialized)
        .map_err(|error| format!("could not write {}: {error}", options.output.display()))?;
    println!("{}", options.output.display());
    Ok(())
}

fn rebase_image_path(resource: &str, input: &Path, output: &Path) -> Result<String, String> {
    fn absolute(path: &Path) -> Result<PathBuf, String> {
        let absolute = std::path::absolute(path).map_err(|error| error.to_string())?;
        let mut normalized = PathBuf::new();
        for component in absolute.components() {
            match component {
                std::path::Component::CurDir => {}
                std::path::Component::ParentDir => {
                    normalized.pop();
                }
                _ => normalized.push(component.as_os_str()),
            }
        }
        Ok(normalized)
    }
    let input = absolute(input)?;
    let output = absolute(output)?;
    let target = absolute(&input.parent().ok_or("input has no parent")?.join(resource))?;
    let base = output.parent().ok_or("output has no parent")?;
    let target_parts: Vec<_> = target.components().collect();
    let base_parts: Vec<_> = base.components().collect();
    if target_parts.first() != base_parts.first() {
        return Err("image and IR output must be on the same filesystem root".to_owned());
    }
    let common = target_parts
        .iter()
        .zip(&base_parts)
        .take_while(|(a, b)| a == b)
        .count();
    let mut relative = PathBuf::new();
    for _ in common..base_parts.len() {
        relative.push("..");
    }
    for component in &target_parts[common..] {
        relative.push(component.as_os_str());
    }
    relative
        .to_str()
        .map(|path| path.replace('\\', "/"))
        .ok_or_else(|| "image path is not Unicode".to_owned())
}

fn verify_pandoc(path: &Path) -> Result<(), AppError> {
    let output = Command::new(path)
        .arg("--version")
        .output()
        .map_err(|error| {
            format!(
                "could not launch Pandoc {}: {error}\n{}",
                path.display(),
                pandoc_setup::GUIDE
            )
        })?;
    if !output.status.success() {
        return Err(AppError::pandoc(
            format!("Pandoc --version failed with {}", output.status),
            output.status.code(),
        ));
    }
    let stdout = String::from_utf8(output.stdout)
        .map_err(|_| "Pandoc --version output was not UTF-8".to_owned())?;
    let actual = stdout.lines().next().unwrap_or("");
    if !is_pandoc_version_line(actual) {
        return Err(
            format!("expected a Pandoc executable, got version response {actual:?}").into(),
        );
    }
    Ok(())
}

fn is_pandoc_version_line(line: &str) -> bool {
    line.strip_prefix("pandoc ")
        .is_some_and(|version| version.starts_with(|c: char| c.is_ascii_digit()))
}

fn invoke_pandoc(path: &Path, source: &[u8]) -> Result<Vec<u8>, AppError> {
    let mut child = Command::new(path)
        .args(["--from=commonmark", "--to=json"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("could not launch Pandoc {}: {error}", path.display()))?;
    let write_result = child
        .stdin
        .take()
        .ok_or_else(|| "Pandoc stdin was not available".to_owned())?
        .write_all(source);
    let output = child
        .wait_with_output()
        .map_err(|error| format!("could not wait for Pandoc: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::pandoc(
            format!("Pandoc failed with {}: {}", output.status, stderr.trim()),
            output.status.code(),
        ));
    }
    write_result.map_err(|error| format!("could not send source to Pandoc: {error}"))?;
    Ok(output.stdout)
}

fn ensure_distinct_paths(input: &Path, output: &Path) -> Result<(), String> {
    if input == output {
        return Err("input and output paths must differ".to_owned());
    }
    if input.exists() && output.exists() {
        let input = fs::canonicalize(input).map_err(|error| error.to_string())?;
        let output = fs::canonicalize(output).map_err(|error| error.to_string())?;
        if input == output {
            return Err("input and output paths resolve to the same file".to_owned());
        }
    }
    Ok(())
}

struct Options {
    input: PathBuf,
    output: PathBuf,
    input_format: InputFormat,
    pandoc: Option<PathBuf>,
    force: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InputFormat {
    CommonMark,
    PandocJson,
}

impl Options {
    fn parse(arguments: Vec<std::ffi::OsString>) -> Result<Self, String> {
        let mut arguments = arguments.into_iter();
        if arguments.next().as_deref() != Some(std::ffi::OsStr::new("md2ir")) {
            return Err(usage());
        }
        let mut input = None;
        let mut output = None;
        let mut pandoc = None;
        let mut input_format = None;
        let mut force = false;
        while let Some(argument) = arguments.next() {
            match argument.to_str() {
                Some("--from") => {
                    let reader = arguments.next().ok_or_else(usage)?;
                    input_format = Some(match reader.to_str() {
                        Some("commonmark") => InputFormat::CommonMark,
                        Some("pandoc-json") => InputFormat::PandocJson,
                        Some(reader) => {
                            return Err(format!(
                                "unsupported --from value {reader:?}; expected commonmark or pandoc-json"
                            ));
                        }
                        None => return Err(usage()),
                    });
                }
                Some("--input") => input = Some(PathBuf::from(arguments.next().ok_or_else(usage)?)),
                Some("--output") => {
                    output = Some(PathBuf::from(arguments.next().ok_or_else(usage)?))
                }
                Some("--pandoc") => {
                    pandoc = Some(PathBuf::from(arguments.next().ok_or_else(usage)?))
                }
                Some("--force") => force = true,
                _ => return Err(usage()),
            }
        }
        let input_format = input_format.ok_or_else(usage)?;
        if input_format == InputFormat::PandocJson && pandoc.is_some() {
            return Err("--pandoc is only valid with --from commonmark".to_owned());
        }
        Ok(Self {
            input: input.ok_or_else(usage)?,
            output: output.ok_or_else(usage)?,
            input_format,
            pandoc: match input_format {
                InputFormat::CommonMark => Some(pandoc.unwrap_or_else(default_pandoc_path)),
                InputFormat::PandocJson => None,
            },
            force,
        })
    }
}

fn default_pandoc_path() -> PathBuf {
    pandoc_setup::find().unwrap_or_else(|| PathBuf::from("pandoc.exe"))
}

fn usage() -> String {
    "usage: md2hwp md2ir --from <commonmark|pandoc-json> --input <file> --output <file.ir.json> [--pandoc <pandoc.exe>] [--force]\n       md2hwp setup-pandoc\n       md2hwp check-runtime [--dotnet <dotnet.exe>]\n       md2hwp render-hwp --worker <worker.exe> --ir <file.ir.json> --template <template.hwp> --output <new.hwp> [--dotnet <dotnet.exe>]".to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pandoc_releases_are_not_an_exact_version_gate() {
        for version in [
            "pandoc 3.10.1",
            "pandoc 3.11",
            "pandoc 4.0",
            "pandoc 3.10.1-nightly",
        ] {
            assert!(is_pandoc_version_line(version));
        }
        assert!(!is_pandoc_version_line("other 3.10.1"));
        assert!(!is_pandoc_version_line("pandoc "));
        // Actual JSON/API compatibility is checked by read_pandoc_json, not this banner.
    }

    #[test]
    fn image_paths_follow_input_when_ir_moves() {
        assert_eq!(
            rebase_image_path(
                "../assets/image.png",
                Path::new("examples/source.md"),
                Path::new("artifacts/nested/out.json")
            )
            .unwrap(),
            "../../assets/image.png"
        );
        assert_eq!(
            rebase_image_path(
                "한글.png",
                Path::new("examples/source.md"),
                Path::new("examples/out.json")
            )
            .unwrap(),
            "한글.png"
        );
    }

    fn arguments(values: &[&str]) -> Vec<std::ffi::OsString> {
        values.iter().map(std::ffi::OsString::from).collect()
    }

    #[test]
    fn direct_pandoc_json_does_not_select_an_executable() {
        let options = Options::parse(arguments(&[
            "md2ir",
            "--from",
            "pandoc-json",
            "--input",
            "input.json",
            "--output",
            "output.json",
        ]))
        .unwrap();

        assert_eq!(options.input_format, InputFormat::PandocJson);
        assert!(options.pandoc.is_none());
    }

    #[test]
    fn direct_pandoc_json_rejects_a_pandoc_override() {
        let error = Options::parse(arguments(&[
            "md2ir",
            "--from",
            "pandoc-json",
            "--input",
            "input.json",
            "--output",
            "output.json",
            "--pandoc",
            "pandoc.exe",
        ]))
        .err()
        .unwrap();

        assert_eq!(error, "--pandoc is only valid with --from commonmark");
    }

    #[test]
    fn preserves_a_pandoc_exit_code() {
        let error = AppError::pandoc("Pandoc failed", Some(23));
        assert_eq!(error.exit_code, 23);
    }

    #[test]
    fn falls_back_when_a_child_has_no_failure_code() {
        assert_eq!(child_exit_code(None), FAILURE_EXIT_CODE);
        assert_eq!(child_exit_code(Some(0)), FAILURE_EXIT_CODE);
    }

    #[test]
    fn application_errors_use_the_failure_exit_code() {
        let error = AppError::from("application failed".to_owned());
        assert_eq!(error.exit_code, FAILURE_EXIT_CODE);
    }
}
