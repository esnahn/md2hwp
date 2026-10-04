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
    if arguments.iter().any(|argument| argument == "--version") {
        println!("md2hwp {}", env!("CARGO_PKG_VERSION"));
        return hancom::run_version(arguments).map_err(AppError::from);
    }
    if arguments.first().is_some_and(|a| {
        Path::new(a)
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("md"))
    }) {
        return convert_manuscript(arguments);
    }
    if arguments.first().is_some_and(|a| a == "setup-pandoc") {
        if arguments.len() != 1 {
            return Err("usage: md2hwp setup-pandoc".to_owned().into());
        }
        return pandoc_setup::install().map_err(AppError::from);
    }
    if arguments
        .first()
        .is_some_and(|a| a == "check-runtime" || a == "ir2hwp" || a == "init-template")
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
    let absolute_output = std::path::absolute(&options.output).map_err(|e| e.to_string())?;
    let mut temporary = tempfile::NamedTempFile::new_in(absolute_output.parent().unwrap())
        .map_err(|e| e.to_string())?;
    temporary
        .write_all(&serialized)
        .map_err(|e| e.to_string())?;
    temporary.as_file().sync_all().map_err(|e| e.to_string())?;
    if options.force {
        temporary
            .persist(&absolute_output)
            .map_err(|e| e.to_string())?;
    } else {
        temporary
            .persist_noclobber(&absolute_output)
            .map_err(|e| e.to_string())?;
    }
    println!("{}", options.output.display());
    Ok(())
}

struct ManuscriptOptions {
    input: PathBuf,
    ir: PathBuf,
    output: PathBuf,
    forwarded: Vec<std::ffi::OsString>,
    protected: Vec<PathBuf>,
}

fn manuscript_options(arguments: &[std::ffi::OsString]) -> Result<ManuscriptOptions, String> {
    let input = std::path::absolute(Path::new(arguments.first().ok_or_else(usage)?))
        .map_err(|e| e.to_string())?;
    let ir = input.with_extension("ir.json");
    let mut output = None;
    let mut forwarded = Vec::new();
    let mut protected = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut args = arguments.iter().skip(1);
    while let Some(key) = args.next() {
        let option = key.to_str();
        if option == Some("--verbose") {
            if !seen.insert(key.clone()) {
                return Err("Duplicate --verbose".into());
            }
            forwarded.push(key.clone());
            continue;
        }
        if matches!(
            option,
            Some("--output" | "--template" | "--worker" | "--dotnet")
        ) {
            if !seen.insert(key.clone()) {
                return Err(format!("Duplicate argument: {}", key.to_string_lossy()));
            }
            let value = args
                .next()
                .filter(|v| !v.to_string_lossy().starts_with("--"))
                .ok_or_else(usage)?;
            let path = std::path::absolute(Path::new(value)).map_err(|e| e.to_string())?;
            if option == Some("--output") {
                if output.replace(path).is_some() {
                    return Err("Duplicate output argument".into());
                }
            } else {
                protected.push(path.clone());
                forwarded.extend([key.clone(), path.into_os_string()]);
            }
        } else {
            if key.to_string_lossy().starts_with('-') {
                return Err(usage());
            }
            let path = std::path::absolute(Path::new(key)).map_err(|e| e.to_string())?;
            if output.replace(path).is_some() {
                return Err("Duplicate output argument".into());
            }
        }
    }
    let output = output.unwrap_or_else(|| input.with_extension("output.hwp"));
    if !output
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("hwp"))
    {
        return Err("Output must have .hwp extension".into());
    }
    Ok(ManuscriptOptions {
        input,
        ir,
        output,
        forwarded,
        protected,
    })
}

fn convert_manuscript(arguments: Vec<std::ffi::OsString>) -> Result<(), AppError> {
    let ManuscriptOptions {
        input,
        ir,
        output,
        forwarded,
        protected,
    } = manuscript_options(&arguments)?;
    // Protect source/template aliases before writing either generated file.
    for destination in [&ir, &output] {
        ensure_distinct_paths(&input, destination)?;
        let template = env::current_exe()
            .map_err(|e| e.to_string())?
            .with_file_name("template.hwp");
        ensure_distinct_paths(&template, destination)?;
        for path in &protected {
            ensure_distinct_paths(path, destination)?;
        }
    }
    run(vec![
        "md2ir".into(),
        "--from".into(),
        "commonmark".into(),
        "--force".into(),
        "--input".into(),
        input.as_os_str().into(),
        "--output".into(),
        ir.as_os_str().into(),
    ])?;
    // Only the backend child receives the resource cwd; never change this process's cwd.
    // IR is retained if runtime/backend/template prerequisites or rendering fail.
    let mut backend = vec![
        "ir2hwp".into(),
        "--ir".into(),
        ir.as_os_str().into(),
        "--output".into(),
        output.as_os_str().into(),
    ];
    backend.extend(forwarded);
    hancom::run_with_resource_root(backend, input.parent())
        .map_err(|e| format!("{e}\nValidated IR retained: {}", ir.display()))?;
    println!("{}", output.display());
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
        .args([
            "--from=commonmark+yaml_metadata_block+footnotes+attributes+implicit_figures",
            "--to=json",
        ])
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
        let mut seen = std::collections::HashSet::new();
        while let Some(argument) = arguments.next() {
            if argument.to_string_lossy().starts_with('-') {
                let option = argument.to_str().ok_or_else(usage)?;
                if !matches!(
                    option,
                    "--input" | "--output" | "--from" | "--pandoc" | "--force"
                ) {
                    return Err(usage());
                }
                if !seen.insert(argument.clone()) {
                    return Err(format!("duplicate option: {option}"));
                }
                if option == "--force" {
                    force = true;
                    continue;
                }
                let value = arguments.next().ok_or_else(usage)?;
                if value.to_string_lossy().starts_with('-') {
                    return Err(format!("missing value for {option}"));
                }
                match option {
                    "--input" if input.is_none() => input = Some(PathBuf::from(value)),
                    "--output" if output.is_none() => output = Some(PathBuf::from(value)),
                    "--input" | "--output" => return Err(format!("duplicate {option}")),
                    "--pandoc" => pandoc = Some(PathBuf::from(value)),
                    "--from" => {
                        input_format =
                            Some(match value.to_str() {
                                Some("commonmark") => InputFormat::CommonMark,
                                Some("pandoc-json") => InputFormat::PandocJson,
                                _ => return Err(
                                    "unsupported --from value; expected commonmark or pandoc-json"
                                        .to_owned(),
                                ),
                            })
                    }
                    _ => unreachable!(),
                }
            } else if input.is_none() {
                input = Some(PathBuf::from(argument));
            } else if output.is_none() {
                output = Some(PathBuf::from(argument));
            } else {
                return Err("unexpected argument or duplicate output".to_owned());
            }
        }
        let input = input.ok_or_else(usage)?;
        let output = output.unwrap_or_else(|| input.with_extension("ir.json"));
        let input_format = input_format.unwrap_or(InputFormat::CommonMark);
        if input_format == InputFormat::PandocJson && pandoc.is_some() {
            return Err("--pandoc is only valid with --from commonmark".to_owned());
        }
        Ok(Self {
            input,
            output,
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
    "usage: md2hwp <source.md> [[--output] <source.output.hwp>] [--template <template.hwp>] [--worker <md2hwp-backend.exe>] [--dotnet <dotnet.exe>] [--verbose]\n       md2hwp md2ir [--input] <source.md|source.json> [[--output] <source.ir.json>] [--from <commonmark|pandoc-json>] [--pandoc <pandoc.exe>] [--force]\n       md2hwp ir2hwp --ir <source.ir.json> --output <source.output.hwp> [--template <template.hwp>] [--worker <md2hwp-backend.exe>] [--dotnet <dotnet.exe>] [--verbose]\n       md2hwp setup-pandoc\n       md2hwp check-runtime [--dotnet <dotnet.exe>]\n       md2hwp init-template [[--output] <template.hwp>] [--worker <md2hwp-backend.exe>] [--dotnet <dotnet.exe>]\n       md2hwp --version [--worker <md2hwp-backend.exe>] [--dotnet <dotnet.exe>]".to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manuscript_outputs_are_beside_source_and_explicit_output_wins() {
        let input = std::ffi::OsString::from("원고 폴더/보고서.v2.md");
        let ManuscriptOptions {
            input: source,
            ir,
            output,
            ..
        } = manuscript_options(std::slice::from_ref(&input)).unwrap();
        assert_eq!(ir, source.with_file_name("보고서.v2.ir.json"));
        assert_eq!(output, source.with_file_name("보고서.v2.output.hwp"));
        let ManuscriptOptions {
            ir: explicit_ir,
            output: explicit_output,
            ..
        } = manuscript_options(&[input, "다른 폴더/결과.hwp".into()]).unwrap();
        assert_eq!(explicit_ir, ir);
        assert_eq!(
            explicit_output,
            std::path::absolute("다른 폴더/결과.hwp").unwrap()
        );
        assert!(manuscript_options(&["source.md".into(), "output.json".into()]).is_err());
    }

    #[test]
    fn shorthand_options_allow_free_order_and_resolve_from_caller() {
        let args = [
            "원고/source.md",
            "--dotnet",
            "runtime/dotnet.exe",
            "--template",
            "양식/custom.hwp",
            "--worker",
            "backend/worker.exe",
            "--output",
            "출력/output.hwp",
        ]
        .map(Into::into);
        let options = manuscript_options(&args).unwrap();
        assert_eq!(
            options.output,
            std::path::absolute("출력/output.hwp").unwrap()
        );
        assert_eq!(
            options.forwarded,
            vec![
                std::ffi::OsString::from("--dotnet"),
                std::path::absolute("runtime/dotnet.exe")
                    .unwrap()
                    .into_os_string(),
                "--template".into(),
                std::path::absolute("양식/custom.hwp")
                    .unwrap()
                    .into_os_string(),
                "--worker".into(),
                std::path::absolute("backend/worker.exe")
                    .unwrap()
                    .into_os_string()
            ]
        );
        let positional = ["source.md", "--template", "custom.hwp", "output.hwp"].map(Into::into);
        assert_eq!(
            manuscript_options(&positional).unwrap().output,
            std::path::absolute("output.hwp").unwrap()
        );
        for tail in [
            vec!["a.hwp", "--output", "b.hwp"],
            vec!["--output", "a.hwp", "b.hwp"],
            vec!["--template"],
            vec!["--template", "--output", "a.hwp"],
            vec!["--worker", "a.exe", "--worker", "b.exe"],
            vec!["--unknown", "x"],
        ] {
            let arguments = std::iter::once("source.md")
                .chain(tail)
                .map(Into::into)
                .collect::<Vec<_>>();
            assert!(manuscript_options(&arguments).is_err());
        }
    }

    #[test]
    fn shorthand_protects_explicit_template_before_writing_ir() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source.md");
        let template = root.path().join("custom.hwp");
        fs::write(&source, "manuscript").unwrap();
        fs::write(&template, "preserve template").unwrap();
        assert!(
            convert_manuscript(vec![
                source.clone().into_os_string(),
                template.clone().into_os_string(),
                "--template".into(),
                template.clone().into_os_string()
            ])
            .is_err()
        );
        assert_eq!(fs::read_to_string(template).unwrap(), "preserve template");
        assert!(!source.with_extension("ir.json").exists());
    }

    #[test]
    fn invalid_source_preserves_existing_outputs() {
        for extension in ["ir.json", "output.hwp"] {
            let root = tempfile::tempdir().unwrap();
            let source = root.path().join("원고.md");
            fs::write(&source, [0xff]).unwrap();
            let existing = source.with_extension(extension);
            fs::write(&existing, "preserve").unwrap();
            let error = run(vec![source.as_os_str().into()]).err().unwrap();
            assert!(error.message.contains("not UTF-8"));
            assert_eq!(fs::read_to_string(&existing).unwrap(), "preserve");
            assert_eq!(fs::read(&source).unwrap(), vec![0xff]);
            assert_eq!(fs::read_dir(root.path()).unwrap().count(), 2);
        }
    }

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
    fn md2ir_accepts_positional_and_named_paths() {
        for args in [
            vec!["md2ir", "원고.md", "결과.ir.json"],
            vec!["md2ir", "원고.md", "--output", "결과.ir.json"],
            vec!["md2ir", "--output", "결과.ir.json", "--input", "원고.md"],
        ] {
            let options = Options::parse(args.into_iter().map(Into::into).collect()).unwrap();
            assert_eq!(options.input, PathBuf::from("원고.md"));
            assert_eq!(options.output, PathBuf::from("결과.ir.json"));
        }
        for args in [
            vec!["md2ir", "a.md", "--input", "b.md"],
            vec!["md2ir", "a.md", "a.json", "--output", "b.json"],
            vec!["md2ir", "a.md", "--output", "a.json", "b.json"],
            vec![
                "md2ir",
                "a.md",
                "--from",
                "commonmark",
                "--from",
                "pandoc-json",
            ],
            vec!["md2ir", "--input", "--force"],
            vec!["md2ir", "a.md", "--unknown"],
        ] {
            assert!(Options::parse(args.into_iter().map(Into::into).collect()).is_err());
        }
        let options = Options::parse(
            ["md2ir", "ast.json", "--from", "pandoc-json"]
                .map(Into::into)
                .to_vec(),
        )
        .unwrap();
        assert_eq!(options.output, PathBuf::from("ast.ir.json"));
        assert_eq!(options.input_format, InputFormat::PandocJson);
    }

    #[test]
    fn md2ir_defaults_follow_source_and_commonmark() {
        let args = ["md2ir", "--input", "원고/보고서.v2.md"].map(Into::into);
        let options = Options::parse(args.to_vec()).unwrap();
        assert_eq!(options.output, PathBuf::from("원고/보고서.v2.ir.json"));
        assert_eq!(options.input_format, InputFormat::CommonMark);
        assert!(options.pandoc.is_some());
        let args = ["md2ir", "--input", "ast.json", "--from", "pandoc-json"].map(Into::into);
        let options = Options::parse(args.to_vec()).unwrap();
        assert_eq!(options.output, PathBuf::from("ast.ir.json"));
        assert!(options.pandoc.is_none());
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
