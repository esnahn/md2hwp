#![forbid(unsafe_code)]
//! User-facing `md2hwp` command-line application.

use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{self, Command, Stdio};

use md2hwp_core::{
    ValidationLimits, load_builtin_rules, normalize_pandoc, read_pandoc_json, write_ir,
};

const PANDOC_VERSION_LINE: &str = "pandoc 3.10.1";
const FAILURE_EXIT_CODE: i32 = 1;

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
    let options = Options::parse(arguments)?;
    let source = fs::read(&options.input)
        .map_err(|error| format!("could not read {}: {error}", options.input.display()))?;
    std::str::from_utf8(&source)
        .map_err(|error| format!("source is not UTF-8 at byte {}", error.valid_up_to()))?;
    ensure_distinct_paths(&options.input, &options.output)?;
    verify_pandoc(&options.pandoc)?;
    let pandoc_json = invoke_pandoc(&options.pandoc, &source)?;

    let limits = ValidationLimits::default();
    let pandoc = read_pandoc_json(&pandoc_json, &limits).map_err(|error| error.to_string())?;
    let rules = load_builtin_rules().map_err(|error| error.to_string())?;
    let ir = normalize_pandoc(pandoc, &rules, "commonmark", &limits)
        .map_err(|error| error.to_string())?;
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

fn verify_pandoc(path: &Path) -> Result<(), AppError> {
    let output = Command::new(path)
        .arg("--version")
        .output()
        .map_err(|error| format!("could not launch pinned Pandoc {}: {error}", path.display()))?;
    if !output.status.success() {
        return Err(AppError::pandoc(
            format!("Pandoc --version failed with {}", output.status),
            output.status.code(),
        ));
    }
    let stdout = String::from_utf8(output.stdout)
        .map_err(|_| "Pandoc --version output was not UTF-8".to_owned())?;
    let actual = stdout.lines().next().unwrap_or("");
    if actual != PANDOC_VERSION_LINE {
        return Err(format!(
            "unsupported Pandoc executable: expected {PANDOC_VERSION_LINE:?}, got {actual:?}"
        )
        .into());
    }
    Ok(())
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
    pandoc: PathBuf,
    force: bool,
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
        let mut reader_selected = false;
        let mut force = false;
        while let Some(argument) = arguments.next() {
            match argument.to_str() {
                Some("--from") => {
                    let reader = arguments.next().ok_or_else(usage)?;
                    if reader != "commonmark" {
                        return Err("only --from commonmark is implemented".to_owned());
                    }
                    reader_selected = true;
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
        if !reader_selected {
            return Err(usage());
        }
        Ok(Self {
            input: input.ok_or_else(usage)?,
            output: output.ok_or_else(usage)?,
            pandoc: pandoc.unwrap_or_else(default_pandoc_path),
            force,
        })
    }
}

fn default_pandoc_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join(".local")
        .join("dependencies")
        .join("pandoc")
        .join("3.10.1")
        .join("pandoc.exe")
}

fn usage() -> String {
    "usage: md2hwp md2ir --from commonmark --input <file.md> --output <file.ir.json> [--pandoc <pandoc.exe>] [--force]".to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

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
