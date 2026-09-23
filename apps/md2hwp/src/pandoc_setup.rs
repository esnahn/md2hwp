use std::{env, fs, path::PathBuf, process::Command};

pub const GUIDE: &str = "Pandoc이 필요합니다. md2hwp setup-pandoc 명령으로 공식 배포본을 다운로드하거나 직접 설치하세요.\nhttps://pandoc.org/installing.html\n별도 설치 경로는 --pandoc <pandoc.exe>로 지정할 수 있습니다.";

pub fn find() -> Option<PathBuf> {
    if let Some(paths) = env::var_os("PATH") {
        for root in env::split_paths(&paths).filter(|p| p.is_absolute()) {
            let path = root.join("pandoc.exe");
            if path.is_file() {
                return Some(path);
            }
        }
    }
    let root = PathBuf::from(env::var_os("LOCALAPPDATA")?).join("md2hwp/pandoc");
    let relative = fs::read_to_string(root.join("current.txt")).ok()?;
    managed_path(&root, relative.trim())
}

fn managed_path(root: &std::path::Path, relative: &str) -> Option<PathBuf> {
    let relative = std::path::Path::new(relative);
    if relative
        .components()
        .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        return None;
    }
    let path = root.join(relative);
    if path.file_name()? != "pandoc.exe" || !path.is_file() {
        return None;
    }
    Some(path)
}

pub fn install() -> Result<(), String> {
    if !cfg!(target_os = "windows") {
        return Err(GUIDE.into());
    }
    println!(
        "Pandoc 공식 배포본 다운로드: https://github.com/jgm/pandoc/releases\nPandoc: John MacFarlane 및 기여자, GPL-2.0-or-later. 원본 배포물과 저작권 안내를 보존합니다."
    );
    let windows = env::var_os("SystemRoot").ok_or("SystemRoot is missing")?;
    let host = PathBuf::from(windows).join("System32/WindowsPowerShell/v1.0/powershell.exe");
    // Build-time defaults only: no runtime lock.json file and no installed-version gate.
    let lock: serde_json::Value =
        serde_json::from_str(include_str!("../../../dependencies/lock.json"))
            .map_err(|e| e.to_string())?;
    let pin = lock["dependencies"]
        .as_array()
        .ok_or("Invalid embedded dependency metadata")?
        .iter()
        .find(|d| d["name"] == "pandoc")
        .ok_or("Missing Pandoc download default")?["pins"][0]
        .clone();
    let status = Command::new(host)
        // Do not inherit PowerShell 7's module paths into Windows PowerShell 5.1.
        .env_remove("PSModulePath")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            include_str!("../setup-pandoc.ps1"),
        ])
        .env(
            "MD2HWP_PANDOC_VERSION",
            pin["version"].as_str().ok_or("Missing Pandoc version")?,
        )
        .env(
            "MD2HWP_PANDOC_URL",
            pin["url"].as_str().ok_or("Missing Pandoc URL")?,
        )
        .env(
            "MD2HWP_PANDOC_SHA256",
            pin["sha256"].as_str().ok_or("Missing Pandoc digest")?,
        )
        .status()
        .map_err(|e| format!("Could not start Pandoc setup: {e}"))?;
    if !status.success() {
        return Err(format!("Pandoc 다운로드 실패: {status}\n{GUIDE}"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn managed_pointer_cannot_escape_download_root() {
        let root = std::path::Path::new("C:/cache");
        for pointer in [
            "../pandoc.exe",
            "C:/other/pandoc.exe",
            "\\\\host\\share\\pandoc.exe",
        ] {
            assert!(managed_path(root, pointer).is_none());
        }
    }
}
