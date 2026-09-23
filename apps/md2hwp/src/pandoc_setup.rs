use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    env, fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};

pub const GUIDE: &str = "Pandoc이 필요합니다. md2hwp setup-pandoc 명령으로 공식 배포본을 다운로드하거나 직접 설치하세요.\nhttps://pandoc.org/installing.html\n별도 설치 경로는 --pandoc <pandoc.exe>로 지정할 수 있습니다.";
const LATEST: &str = "https://api.github.com/repos/jgm/pandoc/releases/latest";
const ARCHIVE_LIMIT: u64 = 128 * 1024 * 1024;

pub fn find() -> Option<PathBuf> {
    let root = env::var_os("LOCALAPPDATA").map(|p| PathBuf::from(p).join("md2hwp/pandoc"));
    let paths = env::var_os("PATH")
        .map(|p| env::split_paths(&p).collect::<Vec<_>>())
        .unwrap_or_default();
    find_in(root.as_deref(), &paths)
}
fn find_in(root: Option<&Path>, paths: &[PathBuf]) -> Option<PathBuf> {
    if let Some(root) = root
        && let Ok(pointer) = fs::read_to_string(root.join("current.txt"))
        && let Some(path) = managed_path(root, pointer.trim())
    {
        return Some(path);
    }
    paths
        .iter()
        .filter(|p| p.is_absolute())
        .map(|p| p.join("pandoc.exe"))
        .find(|p| p.is_file())
}
fn managed_path(root: &Path, relative: &str) -> Option<PathBuf> {
    let relative = Path::new(relative);
    if relative
        .components()
        .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        return None;
    }
    let path = root.join(relative);
    (path.file_name()? == "pandoc.exe" && path.is_file()).then_some(path)
}
struct Release {
    version: String,
    url: String,
    digest: String,
}
impl Release {
    fn new(version: &str, url: &str, digest: &str) -> Result<Self, String> {
        let parts: Vec<_> = version.split('.').collect();
        if !(2..=4).contains(&parts.len())
            || parts
                .iter()
                .any(|p| p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()))
            || digest.len() != 64
            || !digest.bytes().all(|b| b.is_ascii_hexdigit())
            || url
                != format!(
                    "https://github.com/jgm/pandoc/releases/download/{version}/pandoc-{version}-windows-x86_64.zip"
                )
        {
            return Err("Invalid official Pandoc release metadata".into());
        }
        Ok(Self {
            version: version.into(),
            url: url.into(),
            digest: digest.into(),
        })
    }
    fn preferred() -> Result<Self, String> {
        let lock: Value = serde_json::from_str(include_str!("../../../dependencies/lock.json"))
            .map_err(|e| e.to_string())?;
        let pin = &lock["dependencies"]
            .as_array()
            .ok_or("Invalid embedded lock")?
            .iter()
            .find(|d| d["name"] == "pandoc")
            .ok_or("Missing Pandoc pin")?["pins"][0];
        Self::new(
            field(pin, "version")?,
            field(pin, "url")?,
            field(pin, "sha256")?,
        )
    }
    fn latest(bytes: &[u8]) -> Result<Self, String> {
        let value: Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        if value["draft"] != false || value["prerelease"] != false {
            return Err("Expected stable release".into());
        }
        let version = field(&value, "tag_name")?;
        let name = format!("pandoc-{version}-windows-x86_64.zip");
        let assets: Vec<_> = value["assets"]
            .as_array()
            .ok_or("Missing assets")?
            .iter()
            .filter(|a| a["name"] == name)
            .collect();
        if assets.len() != 1 {
            return Err("Expected unique Windows x64 archive".into());
        }
        Self::new(
            version,
            field(assets[0], "browser_download_url")?,
            field(assets[0], "digest")?
                .strip_prefix("sha256:")
                .ok_or("Missing upstream SHA-256")?,
        )
    }
}
fn field<'a>(value: &'a Value, name: &str) -> Result<&'a str, String> {
    value[name]
        .as_str()
        .ok_or_else(|| format!("Missing {name}"))
}
pub fn install() -> Result<(), String> {
    install_default().map_err(|e| format!("Pandoc 다운로드 실패: {e}\n{GUIDE}"))
}
fn install_default() -> Result<(), String> {
    if !cfg!(target_os = "windows") {
        return Err("Windows x64 is required".into());
    }
    let root = PathBuf::from(env::var_os("LOCALAPPDATA").ok_or("LOCALAPPDATA is missing")?)
        .join("md2hwp/pandoc");
    println!(
        "Pandoc 공식 배포본 다운로드. John MacFarlane 및 기여자, GPL-2.0-or-later. 원본 배포물과 저작권 안내를 보존합니다."
    );
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .tls_config(
            ureq::tls::TlsConfig::builder()
                .provider(ureq::tls::TlsProvider::NativeTls)
                .root_certs(ureq::tls::RootCerts::PlatformVerifier)
                .build(),
        )
        .https_only(true)
        .timeout_global(Some(Duration::from_secs(120)))
        .build()
        .into();
    let mut fetch = |url: &str, limit: u64| {
        let mut response = agent
            .get(url)
            .header("User-Agent", "md2hwp-setup")
            .call()
            .map_err(|e| e.to_string())?;
        let mut bytes = Vec::new();
        response
            .body_mut()
            .as_reader()
            .take(limit + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() as u64 > limit {
            return Err("Download exceeds size limit".into());
        }
        Ok(bytes)
    };
    let installed = install_with(&root, Release::preferred()?, &mut fetch, |exe| {
        let json =
            super::invoke_pandoc(exe, b"# md2hwp compatibility check").map_err(|e| e.message)?;
        let limits = super::ValidationLimits::default();
        let ast = super::read_pandoc_json(&json, &limits).map_err(|e| e.to_string())?;
        let rules = super::load_builtin_rules().map_err(|e| e.to_string())?;
        let ir = super::normalize_pandoc(ast, &rules, "commonmark", &limits)
            .map_err(|e| e.to_string())?;
        if !matches!(
            ir.into_document().blocks.as_slice(),
            [super::Block::Heading { level: 1, .. }]
        ) {
            return Err("Unexpected Pandoc compatibility response".into());
        }
        Ok(())
    })?;
    println!(
        "Installed Pandoc: {}\nOriginal ZIP, documentation, COPYRIGHT and source link retained. System PATH unchanged.",
        installed.display()
    );
    Ok(())
}
fn install_with(
    root: &Path,
    mut release: Release,
    fetch: &mut impl FnMut(&str, u64) -> Result<Vec<u8>, String>,
    probe: impl FnOnce(&Path) -> Result<(), String>,
) -> Result<PathBuf, String> {
    let archive = match fetch(&release.url, ARCHIVE_LIMIT) {
        Ok(bytes) => bytes,
        Err(_) => {
            println!("Preferred release unavailable; checking latest official stable release.");
            release = Release::latest(&fetch(LATEST, 4 * 1024 * 1024)?)?;
            fetch(&release.url, ARCHIVE_LIMIT)?
        }
    };
    if !format!("{:x}", Sha256::digest(&archive)).eq_ignore_ascii_case(&release.digest) {
        return Err("Archive integrity check failed; no fallback performed".into());
    }
    fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let stage = tempfile::Builder::new()
        .prefix(".download-")
        .tempdir_in(root)
        .map_err(|e| e.to_string())?;
    fs::write(stage.path().join("upstream.zip"), &archive).map_err(|e| e.to_string())?;
    let exe = extract(&archive, &stage.path().join("upstream"))?;
    probe(&exe)?;
    let copyright = fetch(
        &format!(
            "https://raw.githubusercontent.com/jgm/pandoc/{}/COPYRIGHT",
            release.version
        ),
        4 * 1024 * 1024,
    )?;
    fs::write(stage.path().join("COPYRIGHT"), copyright).map_err(|e| e.to_string())?;
    fs::write(stage.path().join("UPSTREAM.txt"), format!("Pandoc - John MacFarlane and contributors\nGPL-2.0-or-later; see COPYRIGHT and upstream notices.\nBinary: {}\nSource: https://github.com/jgm/pandoc/tree/{}\nInstallation: https://pandoc.org/installing.html\n", release.url, release.version)).map_err(|e| e.to_string())?;
    let relative = exe
        .strip_prefix(stage.path())
        .map_err(|e| e.to_string())?
        .to_owned();
    let name = format!(
        "{}-{}",
        release.version,
        stage
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .trim_start_matches(".download-")
    );
    let destination = root.join(&name);
    fs::rename(stage.path(), &destination).map_err(|e| e.to_string())?;
    // Finalized directory survives publication failure; the old pointer remains valid.
    let mut pointer = tempfile::NamedTempFile::new_in(root).map_err(|e| e.to_string())?;
    write!(pointer, "{}", Path::new(&name).join(&relative).display()).map_err(|e| e.to_string())?;
    pointer.as_file().sync_all().map_err(|e| e.to_string())?;
    pointer
        .persist(root.join("current.txt"))
        .map_err(|e| e.to_string())?;
    Ok(destination.join(relative))
}
fn extract(bytes: &[u8], root: &Path) -> Result<PathBuf, String> {
    let mut archive =
        zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| e.to_string())?;
    let mut executables = Vec::new();
    let mut total = 0u64;
    if archive.len() > 10000 {
        return Err("Too many archive entries".into());
    }
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|e| e.to_string())?;
        if entry
            .name()
            .split(['/', '\\'])
            .any(|part| part == ".." || part.contains(':'))
        {
            return Err("Unsafe archive path".into());
        }
        let name = entry.enclosed_name().ok_or("Unsafe archive path")?;
        if name
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
            || entry.is_symlink()
        {
            return Err("Unsafe archive entry".into());
        }
        total = total
            .checked_add(entry.size())
            .ok_or("Archive size overflow")?;
        if total > 1024 * 1024 * 1024 {
            return Err("Expanded archive exceeds limit".into());
        }
        let path = root.join(name);
        if entry.is_dir() {
            fs::create_dir_all(&path).map_err(|e| e.to_string())?;
            continue;
        }
        fs::create_dir_all(path.parent().ok_or("Missing parent")?).map_err(|e| e.to_string())?;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| e.to_string())?;
        let expected = entry.size();
        let copied = std::io::copy(&mut (&mut entry).take(expected + 1), &mut file)
            .map_err(|e| e.to_string())?;
        if copied != expected {
            return Err("Archive entry size mismatch".into());
        }
        if path.file_name().is_some_and(|n| n == "pandoc.exe") {
            executables.push(path);
        }
    }
    if executables.len() != 1 {
        return Err("Expected exactly one pandoc.exe".into());
    }
    Ok(executables.remove(0))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(name: &str) -> Vec<u8> {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        zip.start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"synthetic executable; never run").unwrap();
        zip.finish().unwrap().into_inner()
    }
    fn release(bytes: &[u8]) -> Release {
        Release::new("3.10.1", "https://github.com/jgm/pandoc/releases/download/3.10.1/pandoc-3.10.1-windows-x86_64.zip", &format!("{:x}", Sha256::digest(bytes))).unwrap()
    }
    fn latest(bytes: &[u8]) -> Vec<u8> {
        let pin = release(bytes);
        serde_json::to_vec(&serde_json::json!({"draft":false,"prerelease":false,"tag_name":pin.version,"assets":[{"name":"pandoc-3.10.1-windows-x86_64.zip","browser_download_url":pin.url,"digest":format!("sha256:{}",pin.digest)}]})).unwrap()
    }
    #[test]
    fn managed_install_precedes_path_and_invalid_pointer_falls_back() {
        let temp = tempfile::tempdir().unwrap();
        let managed = temp.path().join("managed");
        let path = temp.path().join("path");
        fs::create_dir_all(managed.join("version")).unwrap();
        fs::create_dir_all(&path).unwrap();
        fs::write(managed.join("version/pandoc.exe"), b"").unwrap();
        fs::write(path.join("pandoc.exe"), b"").unwrap();
        fs::write(managed.join("current.txt"), "version/pandoc.exe").unwrap();
        assert_eq!(
            find_in(Some(&managed), std::slice::from_ref(&path)),
            Some(managed.join("version/pandoc.exe"))
        );
        for pointer in [
            "../path/pandoc.exe",
            "C:/other/pandoc.exe",
            "\\\\host\\share\\pandoc.exe",
            "missing/pandoc.exe",
        ] {
            fs::write(managed.join("current.txt"), pointer).unwrap();
            assert_eq!(
                find_in(Some(&managed), std::slice::from_ref(&path)),
                Some(path.join("pandoc.exe"))
            );
        }
    }
    #[test]
    fn unavailable_preferred_falls_back_and_publishes_after_probe() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("current.txt"), "old/pandoc.exe").unwrap();
        let bytes = fixture("pandoc/pandoc.exe");
        let mut calls = Vec::new();
        let installed = install_with(
            temp.path(),
            release(&bytes),
            &mut |url, _| {
                calls.push(url.to_owned());
                match calls.len() {
                    1 => Err("unavailable".into()),
                    2 => {
                        assert_eq!(url, LATEST);
                        Ok(latest(&bytes))
                    }
                    3 => Ok(bytes.clone()),
                    4 => Ok(b"copyright fixture".to_vec()),
                    _ => panic!("unexpected request"),
                }
            },
            |exe| {
                assert!(exe.is_file());
                assert_eq!(
                    fs::read_to_string(temp.path().join("current.txt")).unwrap(),
                    "old/pandoc.exe"
                );
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(find_in(Some(temp.path()), &[]), Some(installed.clone()));
        let directory = installed
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        for file in ["upstream.zip", "COPYRIGHT", "UPSTREAM.txt"] {
            assert!(directory.join(file).is_file());
        }
        assert_eq!(calls.len(), 4);
    }
    #[test]
    fn failures_preserve_current_and_clean_staging() {
        for failure in ["digest", "extract", "probe", "copyright"] {
            let temp = tempfile::tempdir().unwrap();
            fs::write(temp.path().join("current.txt"), "old/pandoc.exe").unwrap();
            let bytes = if failure == "extract" {
                b"not zip".to_vec()
            } else {
                fixture("pandoc/pandoc.exe")
            };
            let pin = release(&bytes);
            let mut calls = 0;
            let result = install_with(
                temp.path(),
                pin,
                &mut |_, _| {
                    calls += 1;
                    if failure == "digest" {
                        return Ok(b"corrupted".to_vec());
                    }
                    if calls == 2 {
                        return Err("copyright unavailable".into());
                    }
                    Ok(bytes.clone())
                },
                |_| {
                    if failure == "probe" {
                        Err("incompatible JSON".into())
                    } else {
                        Ok(())
                    }
                },
            );
            assert!(result.is_err(), "{failure}");
            assert_eq!(
                fs::read_to_string(temp.path().join("current.txt")).unwrap(),
                "old/pandoc.exe"
            );
            assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1, "{failure}");
            assert_eq!(calls, if failure == "copyright" { 2 } else { 1 });
        }
    }
    #[test]
    fn unsafe_archive_and_untrusted_release_are_rejected() {
        let temp = tempfile::tempdir().unwrap();
        assert!(extract(&fixture("../escaped.exe"), temp.path()).is_err());
        assert!(Release::new("3.10.1", "https://example.com/pandoc.zip", &"0".repeat(64)).is_err());
        let bytes = fixture("pandoc.exe");
        let mut metadata: Value = serde_json::from_slice(&latest(&bytes)).unwrap();
        metadata["assets"][0]["digest"] = Value::Null;
        assert!(Release::latest(&serde_json::to_vec(&metadata).unwrap()).is_err());
        metadata["prerelease"] = Value::Bool(true);
        assert!(Release::latest(&serde_json::to_vec(&metadata).unwrap()).is_err());
        assert!(Release::preferred().is_ok());
    }
}
