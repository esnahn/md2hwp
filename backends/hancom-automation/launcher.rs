//! Repository investigation launcher; the C# worker remains a separate process.
use std::{env, ffi::OsString, fs, path::PathBuf, process::Command};

const INSTALL: &str = ".NET 10 런타임(x64)이 필요합니다.\n공식 페이지에서 콘솔 앱용 .NET Runtime → Windows → x64를 설치한 뒤 다시 실행하세요.\nhttps://dotnet.microsoft.com/ko-kr/download/dotnet/10.0\nSDK는 필요하지 않습니다. 설치 위치가 별도라면 --dotnet <dotnet.exe>를 지정하세요.";

fn compatible_runtime(list: &str) -> bool {
    list.lines().any(|line| {
        let mut fields = line.split_whitespace();
        fields.next() == Some("Microsoft.NETCore.App")
            && fields.next().is_some_and(|version| {
                let parts: Vec<_> = version.split('.').collect();
                parts.len() == 3
                    && parts[0] == "10"
                    && parts[1] == "0"
                    && !parts[2].is_empty()
                    && parts[2].bytes().all(|b| b.is_ascii_digit())
            })
    })
}

fn is_x64_pe(bytes: &[u8]) -> bool {
    if bytes.get(..2) != Some(b"MZ") {
        return false;
    }
    let Some(offset) = bytes.get(0x3c..0x40) else {
        return false;
    };
    let offset = u32::from_le_bytes(offset.try_into().unwrap()) as usize;
    bytes.get(offset..offset.saturating_add(6)) == Some(b"PE\0\0\x64\x86")
}

fn candidates(explicit: Option<PathBuf>) -> Vec<PathBuf> {
    if let Some(path) = explicit {
        return vec![path];
    }
    let mut paths = Vec::new();
    for key in ["DOTNET_ROOT_X64", "DOTNET_ROOT"] {
        if let Some(root) = env::var_os(key) {
            paths.push(PathBuf::from(root).join("dotnet.exe"));
        }
    }
    if let Some(root) = env::var_os("ProgramW6432").or_else(|| env::var_os("ProgramFiles")) {
        paths.push(PathBuf::from(root).join("dotnet/dotnet.exe"));
    }
    if let Some(path) = env::var_os("PATH") {
        paths.extend(
            env::split_paths(&path)
                .filter(|p| p.is_absolute())
                .map(|p| p.join("dotnet.exe")),
        );
    }
    paths
}

fn resolve_runtime(explicit: Option<PathBuf>) -> Result<PathBuf, String> {
    let explicit_host = explicit.is_some();
    let mut diagnostics = Vec::new();
    let mut visited = std::collections::HashSet::new();
    for path in candidates(explicit) {
        let identity = path.to_string_lossy().replace('/', "\\").to_lowercase();
        if !visited.insert(identity) {
            continue;
        }
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if !explicit_host && error.kind() == std::io::ErrorKind::NotFound => {
                continue;
            }
            Err(error) => {
                diagnostics.push(format!("{}: {error}", path.display()));
                continue;
            }
        };
        if !is_x64_pe(&bytes) {
            diagnostics.push(format!(
                "{}: Windows x64 실행 파일이 아닙니다",
                path.display()
            ));
            continue;
        }
        let mut command = Command::new(&path);
        command.arg("--list-runtimes");
        match command.output() {
            Ok(output)
                if output.status.success()
                    && compatible_runtime(&String::from_utf8_lossy(&output.stdout)) =>
            {
                return fs::canonicalize(&path).map_err(|e| e.to_string());
            }
            Ok(_) => diagnostics.push(format!(
                "{}: 정식 Microsoft.NETCore.App 10.0.x 없음",
                path.display()
            )),
            Err(error) => diagnostics.push(format!("{}: {error}", path.display())),
        }
    }
    if diagnostics.is_empty() {
        diagnostics.push("설치된 dotnet.exe를 찾지 못했습니다.".into());
    }
    Err(format!("{INSTALL}\n확인 결과:\n{}", diagnostics.join("\n")))
}

struct VersionOptions {
    worker: Option<PathBuf>,
    dotnet: Option<PathBuf>,
}

impl VersionOptions {
    fn parse(arguments: Vec<OsString>) -> Result<Self, String> {
        let mut result = Self {
            worker: None,
            dotnet: None,
        };
        let mut version = false;
        let mut args = arguments.into_iter();
        while let Some(key) = args.next() {
            if key == "--version" {
                if version {
                    return Err("Duplicate --version".into());
                }
                version = true;
                continue;
            }
            let slot = match key.to_str() {
                Some("--worker") => &mut result.worker,
                Some("--dotnet") => &mut result.dotnet,
                _ => return Err(version_usage()),
            };
            if slot.is_some() {
                return Err(format!("Duplicate argument: {}", key.to_string_lossy()));
            }
            let value = args
                .next()
                .filter(|value| !value.to_string_lossy().starts_with('-'))
                .ok_or_else(version_usage)?;
            *slot = Some(PathBuf::from(value));
        }
        if !version {
            return Err(version_usage());
        }
        Ok(result)
    }
}

fn version_usage() -> String {
    "usage: md2hwp --version [--worker <md2hwp-backend.exe>] [--dotnet <dotnet.exe>]".into()
}

pub fn run_version(arguments: Vec<OsString>) -> Result<(), String> {
    let options = VersionOptions::parse(arguments)?;
    if !cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        return Err("Hancom requires Windows x64.".into());
    }
    launch_worker(
        adjacent_default(options.worker, "md2hwp-backend.exe")?,
        options.dotnet,
        vec!["--version".into()],
        None,
    )
}

pub fn run(arguments: Vec<OsString>) -> Result<(), String> {
    run_with_resource_root(arguments, None)
}

pub fn run_with_resource_root(
    arguments: Vec<OsString>,
    resource_root: Option<&std::path::Path>,
) -> Result<(), String> {
    if !cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        return Err("Hancom requires Windows x64.".into());
    }
    let mut args = arguments.into_iter();
    let mode = args.next().unwrap_or_default();
    let check = mode == "check-runtime";
    let init = mode == "init-template";
    if !check && !init && mode != "ir2hwp" {
        return Err(usage());
    }
    let (mut dotnet, mut worker, mut ir, mut template, mut output) = (None, None, None, None, None);
    let mut verbose = false;
    while let Some(key) = args.next() {
        if key == "--verbose" && !check && !init {
            if verbose {
                return Err("duplicate --verbose".into());
            }
            verbose = true;
            continue;
        }
        if init && !key.to_string_lossy().starts_with('-') {
            if output.replace(PathBuf::from(key)).is_some() {
                return Err("duplicate output argument".into());
            }
            continue;
        }
        let slot = match key.to_str() {
            Some("--dotnet") => &mut dotnet,
            Some("--worker") if !check => &mut worker,
            Some("--ir") if !check && !init => &mut ir,
            Some("--template") if !check && !init => &mut template,
            Some("--output") if !check => &mut output,
            _ => return Err(usage()),
        };
        if slot.is_some() {
            return Err("duplicate argument".into());
        }
        *slot = Some(PathBuf::from(args.next().ok_or_else(usage)?));
    }
    if check {
        println!(".NET 10 x64: {}", resolve_runtime(dotnet)?.display());
        return Ok(());
    }
    let worker = adjacent_default(worker, "md2hwp-backend.exe")?;
    if init {
        let output = output.unwrap_or_else(|| PathBuf::from("template.hwp"));
        if output.exists() {
            return Err(format!("Template already exists: {}", output.display()));
        }
        if !output
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("hwp"))
        {
            return Err("Template output must be .hwp".into());
        }
        return launch_worker(
            worker,
            dotnet,
            vec![
                "init-template".into(),
                "--output".into(),
                output.into_os_string(),
            ],
            None,
        );
    }
    let ir = ir.ok_or_else(usage)?;
    let template = adjacent_default(template, "template.hwp")?;
    let output = output.ok_or_else(usage)?;
    for path in [&worker, &ir, &template] {
        if !path.is_file() {
            return Err(format!("Missing input: {}", path.display()));
        }
    }
    for path in [&template, &output] {
        if !path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("hwp"))
        {
            return Err("The tagged investigation requires HWP input and output.".into());
        }
    }
    if output.exists() {
        let resolved = fs::canonicalize(&output).map_err(|e| e.to_string())?;
        for protected in [&template, &ir, &worker] {
            if resolved == fs::canonicalize(protected).map_err(|e| e.to_string())? {
                return Err(format!(
                    "Output must not replace input: {}",
                    protected.display()
                ));
            }
        }
    }
    let content = fs::read(&ir).map_err(|e| e.to_string())?;
    md2hwp_core::read_ir(&content, &md2hwp_core::ValidationLimits::default())
        .map_err(|e| e.to_string())?;
    let mut forwarded = vec![
        "ir2hwp".into(),
        "--ir".into(),
        ir.into_os_string(),
        "--template".into(),
        template.into_os_string(),
        "--output".into(),
        output.into_os_string(),
    ];
    if verbose {
        forwarded.push("--verbose".into());
    }
    launch_worker(worker, dotnet, forwarded, resource_root)
}

fn launch_worker(
    worker: PathBuf,
    dotnet: Option<PathBuf>,
    arguments: Vec<OsString>,
    resource_root: Option<&std::path::Path>,
) -> Result<(), String> {
    if !worker
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("exe"))
        || !is_x64_pe(&fs::read(&worker).map_err(|e| e.to_string())?)
    {
        return Err("Worker must be a published Windows x64 EXE.".into());
    }
    let dotnet = resolve_runtime(dotnet)?;
    let runtime_root = dotnet
        .parent()
        .ok_or("dotnet.exe has no parent directory")?;
    let mut command = Command::new(fs::canonicalize(worker).map_err(|e| e.to_string())?);
    if let Some(root) = resource_root {
        command.current_dir(root);
    }
    let status = command
        .args(arguments)
        // The published apphost searches environment variables only: use the checked runtime.
        .env("DOTNET_ROOT_X64", runtime_root)
        .env("DOTNET_ROOT", runtime_root)
        .env("DOTNET_ROLL_FORWARD", "LatestPatch")
        .env("DOTNET_ROLL_FORWARD_TO_PRERELEASE", "0")
        .status()
        .map_err(|e| format!("Could not start Hancom worker: {e}"))?;
    if !status.success() {
        return Err(format!("Hancom worker failed: {status}"));
    }
    Ok(())
}

fn adjacent_default(explicit: Option<PathBuf>, name: &str) -> Result<PathBuf, String> {
    if let Some(path) = explicit {
        return Ok(path);
    }
    let executable = env::current_exe().map_err(|e| e.to_string())?;
    Ok(executable
        .parent()
        .ok_or("Executable has no parent directory")?
        .join(name))
}

fn usage() -> String {
    "usage: md2hwp ir2hwp --ir <source.ir.json> --output <source.output.hwp> [--template <template.hwp>] [--worker <md2hwp-backend.exe>] [--dotnet <dotnet.exe>] [--verbose]\n       md2hwp check-runtime [--dotnet <dotnet.exe>]\n       md2hwp init-template [[--output] <template.hwp>] [--worker <md2hwp-backend.exe>] [--dotnet <dotnet.exe>]\nDefaults beside md2hwp.exe: md2hwp-backend.exe, template.hwp".into()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn version_options_support_free_order_and_reject_ambiguous_inputs() {
        let defaults = VersionOptions::parse(vec!["--version".into()]).unwrap();
        assert!(defaults.worker.is_none() && defaults.dotnet.is_none());
        for args in [
            vec![
                "--version",
                "--worker",
                "custom worker.exe",
                "--dotnet",
                "custom dotnet.exe",
            ],
            vec![
                "--dotnet",
                "custom dotnet.exe",
                "--worker",
                "custom worker.exe",
                "--version",
            ],
        ] {
            let options =
                VersionOptions::parse(args.into_iter().map(OsString::from).collect()).unwrap();
            assert_eq!(options.worker, Some(PathBuf::from("custom worker.exe")));
            assert_eq!(options.dotnet, Some(PathBuf::from("custom dotnet.exe")));
        }
        for args in [
            vec![],
            vec!["--version", "--version"],
            vec!["--version", "--worker"],
            vec!["--worker", "--version"],
            vec!["--version", "--dotnet", "--worker", "x.exe"],
            vec!["--version", "--worker", "a.exe", "--worker", "b.exe"],
            vec!["--version", "--dotnet", "a.exe", "--dotnet", "b.exe"],
            vec!["--version", "--template", "template.hwp"],
            vec!["source.md", "--version"],
        ] {
            assert!(VersionOptions::parse(args.into_iter().map(OsString::from).collect()).is_err());
        }
    }
    #[test]
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    fn template_creation_rejects_unsafe_targets_before_starting_worker() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("edited.hwp");
        fs::write(&output, b"user template").unwrap();
        let error = run(vec![
            "init-template".into(),
            output.clone().into_os_string(),
        ])
        .unwrap_err();
        assert!(error.contains("already exists"), "{error}");
        assert_eq!(fs::read(output).unwrap(), b"user template");
        for arguments in [
            vec!["init-template", "first.hwp", "second.hwp"],
            vec!["init-template", "first.hwp", "--output", "second.hwp"],
            vec!["init-template", "--template", "old.hwp"],
            vec!["init-template", "--ir", "source.ir.json"],
            vec!["init-template", "wrong.txt"],
        ] {
            assert!(run(arguments.into_iter().map(OsString::from).collect()).is_err());
        }
    }
    #[test]
    fn defaults_are_executable_relative_and_explicit_paths_win() {
        for name in ["template.hwp", "md2hwp-backend.exe"] {
            assert_eq!(
                adjacent_default(None, name).unwrap(),
                env::current_exe().unwrap().parent().unwrap().join(name)
            );
            let explicit = PathBuf::from("custom").join(name);
            assert_eq!(
                adjacent_default(Some(explicit.clone()), name).unwrap(),
                explicit
            );
        }
    }
    #[test]
    fn runtime_requires_stable_core_10_0() {
        assert!(compatible_runtime(
            "Microsoft.NETCore.App 10.0.11 [C:\\dotnet]"
        ));
        for line in [
            "Microsoft.NETCore.App 9.0.1 [x]",
            "Microsoft.NETCore.App 11.0.0 [x]",
            "Microsoft.NETCore.App 10.0.0-preview.1 [x]",
            "Microsoft.WindowsDesktop.App 10.0.1 [x]",
            "Microsoft.NETCore.App 10.0. [x]",
            "Microsoft.NETCore.App 10.1.0 [x]",
        ] {
            assert!(!compatible_runtime(line), "{line}");
        }
    }
    #[test]
    fn rejects_wrong_architecture_and_malformed_hosts() {
        let mut pe = vec![0; 134];
        pe[..2].copy_from_slice(b"MZ");
        pe[0x3c..0x40].copy_from_slice(&128u32.to_le_bytes());
        pe[128..].copy_from_slice(b"PE\0\0\x64\x86");
        assert!(is_x64_pe(&pe));
        pe[132..].copy_from_slice(&0x14cu16.to_le_bytes());
        assert!(!is_x64_pe(&pe));
        assert!(!is_x64_pe(&[]));
        pe[0x3c..0x40].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(!is_x64_pe(&pe));
    }
    #[test]
    fn explicit_host_does_not_fall_back() {
        assert_eq!(
            candidates(Some(PathBuf::from("missing.exe"))),
            vec![PathBuf::from("missing.exe")]
        );
    }
}
