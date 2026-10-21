// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Euisoon Ahn

//! Bounded image-container inspection. Drawing commands and compressed pixels
//! are preserved verbatim; this is not a complete image decoder.

use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};

const MAX_BYTES: u64 = 64 * 1024 * 1024;
const MAX_RECORDS: u32 = 1_000_000;

#[derive(Debug)]
pub(crate) struct Resource {
    pub bytes: Vec<u8>,
    pub extension: String,
    /// Raster pixels, or EMF physical frame units (0.01 mm).
    pub width: f64,
    pub height: f64,
    /// Zero for vector EMF resources, which have no intrinsic pixel dimensions.
    pub pixel_width: u32,
    pub pixel_height: u32,
    source: Option<SourceIdentity>,
}

#[derive(Debug)]
struct SourceIdentity {
    requested: PathBuf,
    resolved: PathBuf,
    root: PathBuf,
    resolved_root: PathBuf,
}

impl Resource {
    pub fn read(base: &Path, relative: &str) -> Result<Self, String> {
        Self::read_with_root(base, base, relative)
    }

    pub fn read_with_root(base: &Path, root: &Path, relative: &str) -> Result<Self, String> {
        if relative.is_empty()
            || relative.starts_with('/')
            || relative.contains(['\\', ':'])
            || relative.chars().any(char::is_control)
            || Path::new(relative).is_absolute()
        {
            return Err("image path must be a relative local path using / separators".into());
        }
        let root = std::path::absolute(root).map_err(|e| e.to_string())?;
        let resolved_root = fs::canonicalize(&root)
            .map_err(|e| format!("cannot resolve image resource root: {e}"))?;
        let requested = std::path::absolute(base.join(relative)).map_err(|e| e.to_string())?;
        let path = fs::canonicalize(&requested)
            .map_err(|e| format!("cannot resolve image {relative}: {e}"))?;
        if !path.starts_with(&resolved_root) || path == resolved_root {
            return Err("image must resolve inside the resource root".into());
        }
        let extension = Path::new(relative)
            .extension()
            .and_then(|x| x.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if !matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "emf") {
            return Err("figures support PNG, JPG, JPEG and EMF files only".into());
        }
        let bytes = read_bounded(&path)?;
        let mut resource = Self::from_bytes(bytes, extension)
            .map_err(|e| format!("invalid or unsupported image {relative}: {e}"))?;
        resource.source = Some(SourceIdentity {
            requested,
            resolved: path,
            root,
            resolved_root,
        });
        resource.verify_unchanged()?;
        Ok(resource)
    }

    /// Recheck source identity and bytes before publishing a generated document.
    pub fn verify_unchanged(&self) -> Result<(), String> {
        let source = self
            .source
            .as_ref()
            .ok_or("image resource has no source identity")?;
        let resolved = fs::canonicalize(&source.requested)
            .map_err(|e| format!("image source is no longer available: {e}"))?;
        let root = fs::canonicalize(&source.root)
            .map_err(|e| format!("image resource root is no longer available: {e}"))?;
        if resolved != source.resolved
            || root != source.resolved_root
            || !resolved.starts_with(&root)
            || resolved == root
        {
            return Err("image source path or resource root changed during generation".into());
        }
        if read_bounded(&resolved)? != self.bytes {
            return Err(format!(
                "image source changed during generation: {}",
                source.requested.display()
            ));
        }
        Ok(())
    }

    fn from_bytes(bytes: Vec<u8>, extension: String) -> Result<Self, String> {
        if bytes.len() as u64 > MAX_BYTES {
            return Err("image exceeds 64 MiB".into());
        }
        let (width, height, pixel_width, pixel_height) = match extension.as_str() {
            "png" => {
                let (w, h) = png_dimensions(&bytes)?;
                (f64::from(w), f64::from(h), w, h)
            }
            "jpg" | "jpeg" => {
                let (w, h) = jpeg_dimensions(&bytes)?;
                (f64::from(w), f64::from(h), w, h)
            }
            "emf" => {
                let (w, h) = emf_dimensions(&bytes)?;
                (w as f64, h as f64, 0, 0)
            }
            _ => return Err("unsupported image extension".into()),
        };
        Ok(Self {
            bytes,
            extension,
            width,
            height,
            pixel_width,
            pixel_height,
            source: None,
        })
    }
}

fn read_bounded(path: &Path) -> Result<Vec<u8>, String> {
    let file =
        File::open(path).map_err(|e| format!("cannot open image {}: {e}", path.display()))?;
    let metadata = file.metadata().map_err(|e| e.to_string())?;
    if !metadata.is_file() || metadata.len() > MAX_BYTES {
        return Err("image must be a regular file of at most 64 MiB".into());
    }
    // Also bound the read if a file grows after the metadata check.
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("cannot read image {}: {e}", path.display()))?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("image exceeds 64 MiB".into());
    }
    Ok(bytes)
}

fn png_dimensions(bytes: &[u8]) -> Result<(u32, u32), String> {
    if bytes.len() < 24 || bytes[..8] != [137, 80, 78, 71, 13, 10, 26, 10] {
        return Err("invalid PNG header".into());
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
    let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
    if width == 0 || height == 0 || width > i32::MAX as u32 || height > i32::MAX as u32 {
        return Err("invalid PNG dimensions".into());
    }
    Ok((width, height))
}

fn jpeg_dimensions(bytes: &[u8]) -> Result<(u32, u32), String> {
    fn byte(bytes: &[u8], cursor: &mut usize) -> Result<u8, String> {
        let value = bytes.get(*cursor).copied().ok_or("truncated JPEG header")?;
        *cursor += 1;
        Ok(value)
    }
    fn word(bytes: &[u8], cursor: &mut usize) -> Result<usize, String> {
        Ok((usize::from(byte(bytes, cursor)?) << 8) | usize::from(byte(bytes, cursor)?))
    }
    let mut cursor = 0;
    if word(bytes, &mut cursor)? != 0xffd8 {
        return Err("missing JPEG SOI signature".into());
    }
    while cursor < bytes.len() {
        if byte(bytes, &mut cursor)? != 0xff {
            return Err("missing JPEG segment marker".into());
        }
        let mut marker = byte(bytes, &mut cursor)?;
        while marker == 0xff {
            marker = byte(bytes, &mut cursor)?;
        }
        if matches!(marker, 0 | 0xd8 | 0xd0..=0xd7) {
            return Err("unexpected JPEG marker before frame".into());
        }
        if matches!(marker, 0xda | 0xd9) {
            return Err("missing JPEG frame dimensions".into());
        }
        if marker == 1 {
            continue;
        }
        let length = word(bytes, &mut cursor)?;
        if length < 2 || length - 2 > bytes.len() - cursor {
            return Err("invalid JPEG segment length".into());
        }
        if matches!(marker, 0xc0..=0xc2) {
            if length < 8 {
                return Err("truncated JPEG frame header".into());
            }
            let precision = byte(bytes, &mut cursor)?;
            let height = word(bytes, &mut cursor)?;
            let width = word(bytes, &mut cursor)?;
            let components = usize::from(byte(bytes, &mut cursor)?);
            if precision != 8
                || width == 0
                || height == 0
                || !(1..=4).contains(&components)
                || length != 8 + 3 * components
            {
                return Err(
                    "invalid or unsupported JPEG frame dimensions/precision/components".into(),
                );
            }
            return Ok((width as u32, height as u32));
        }
        if (0xc0..=0xcf).contains(&marker) && !matches!(marker, 0xc4 | 0xc8 | 0xcc) {
            return Err(
                "only 8-bit sequential and progressive JPEG DCT frames are supported".into(),
            );
        }
        cursor += length - 2;
    }
    Err("missing JPEG frame header".into())
}

fn emf_dimensions(bytes: &[u8]) -> Result<(i64, i64), String> {
    fn u(bytes: &[u8], offset: usize) -> u32 {
        u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
    }
    fn i(bytes: &[u8], offset: usize) -> i64 {
        i64::from(i32::from_le_bytes(
            bytes[offset..offset + 4].try_into().unwrap(),
        ))
    }
    if bytes.len() < 108 || bytes.len() as u64 > MAX_BYTES {
        return Err("EMF requires a complete header/EOF and at most 64 MiB".into());
    }
    if u(bytes, 0) != 1 || u(bytes, 40) != 0x464d4520 || u(bytes, 44) != 0x00010000 {
        return Err("missing EMR_HEADER/EMF signature or unsupported version".into());
    }
    let header_size = u(bytes, 4) as usize;
    if header_size < 88 || !header_size.is_multiple_of(4) || header_size > bytes.len() - 20 {
        return Err("invalid EMF header record size".into());
    }
    if u(bytes, 48) as usize != bytes.len() {
        return Err("EMF declared byte count does not match the file".into());
    }
    let expected = u(bytes, 52);
    if !(2..=MAX_RECORDS).contains(&expected) || expected as usize > bytes.len() / 8 {
        return Err("invalid EMF record count".into());
    }
    let description_characters = u(bytes, 60);
    let description_offset = u(bytes, 64);
    if description_characters != 0
        && (description_offset < 88
            || !description_offset.is_multiple_of(2)
            || u64::from(description_offset) + 2 * u64::from(description_characters)
                > header_size as u64)
    {
        return Err("EMF description exceeds the header record".into());
    }
    let width = i(bytes, 32) - i(bytes, 24);
    let height = i(bytes, 36) - i(bytes, 28);
    if width <= 0 || height <= 0 {
        return Err("EMF physical frame must have positive width and height".into());
    }
    let mut cursor = header_size;
    let mut count = 1;
    while cursor < bytes.len() {
        if bytes.len() - cursor < 8 {
            return Err("truncated EMF record header".into());
        }
        let kind = u(bytes, cursor);
        let size = u(bytes, cursor + 4) as usize;
        if matches!(kind, 0 | 1)
            || size < 8
            || !size.is_multiple_of(4)
            || size > bytes.len() - cursor
        {
            return Err("invalid EMF record type, size or boundary".into());
        }
        count += 1;
        if count > expected {
            return Err("too many EMF records".into());
        }
        if kind == 14 {
            if size < 20 || cursor + size != bytes.len() || count != expected {
                return Err("invalid EMF EOF position, size or record count".into());
            }
            let entries = u(bytes, cursor + 8);
            let offset = u(bytes, cursor + 12);
            if u(bytes, cursor + 16) as usize != size
                || (entries != 0
                    && (offset < 20 || u64::from(offset) + 4 * u64::from(entries) > size as u64))
            {
                return Err("invalid EMF EOF palette or size".into());
            }
            return Ok((width, height));
        }
        cursor += size;
    }
    Err("missing EMR_EOF".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png() -> Vec<u8> {
        let mut bytes = vec![0; 24];
        bytes[..8].copy_from_slice(&[137, 80, 78, 71, 13, 10, 26, 10]);
        bytes[16..20].copy_from_slice(&640_u32.to_be_bytes());
        bytes[20..24].copy_from_slice(&480_u32.to_be_bytes());
        bytes
    }

    fn jpeg(marker: u8) -> Vec<u8> {
        // APP metadata contains a false SOF; it must be skipped by length.
        vec![
            0xff, 0xd8, 0xff, 0xe1, 0, 6, 0xff, 0xc0, 0, 0, 0xff, marker, 0, 11, 8, 0, 32, 0, 64,
            1, 1, 0x11, 0,
        ]
    }

    fn put(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn emf() -> Vec<u8> {
        let mut bytes = vec![0; 108];
        for (offset, value) in [
            (0, 1),
            (4, 88),
            (24, 100),
            (28, 200),
            (32, 2100),
            (36, 1200),
            (40, 0x464d4520),
            (44, 0x10000),
            (48, 108),
            (52, 2),
            (88, 14),
            (92, 20),
            (104, 20),
        ] {
            put(&mut bytes, offset, value);
        }
        bytes
    }

    #[test]
    fn raster_headers_retain_bytes_and_dimensions() {
        let bytes = png();
        let resource = Resource::from_bytes(bytes.clone(), "png".into()).unwrap();
        assert_eq!(resource.bytes, bytes);
        assert_eq!(resource.extension, "png");
        assert_eq!((resource.width, resource.height), (640.0, 480.0));
        assert_eq!((resource.pixel_width, resource.pixel_height), (640, 480));
        for marker in [0xc0, 0xc1, 0xc2] {
            assert_eq!(jpeg_dimensions(&jpeg(marker)).unwrap(), (64, 32));
        }
    }

    #[test]
    fn rejects_malformed_raster_headers() {
        for length in 0..24 {
            assert!(png_dimensions(&png()[..length]).is_err());
        }
        let mut zero = png();
        zero[16..20].fill(0);
        assert!(png_dimensions(&zero).is_err());
        for length in 0..jpeg(0xc0).len() {
            assert!(jpeg_dimensions(&jpeg(0xc0)[..length]).is_err());
        }
        assert!(jpeg_dimensions(&jpeg(0xc3)).is_err());
        let mut precision = jpeg(0xc0);
        precision[14] = 12;
        assert!(jpeg_dimensions(&precision).is_err());
        let mut segment = jpeg(0xc0);
        segment[5] = 255;
        assert!(jpeg_dimensions(&segment).is_err());
    }

    #[test]
    fn emf_uses_physical_frame_and_checks_container_boundaries() {
        let bytes = emf();
        let resource = Resource::from_bytes(bytes.clone(), "emf".into()).unwrap();
        assert_eq!((resource.width, resource.height), (2000.0, 1000.0));
        assert_eq!((resource.pixel_width, resource.pixel_height), (0, 0));
        assert_eq!(resource.bytes, bytes);
        for (offset, value) in [
            (4, 87),
            (32, 100),
            (40, 0),
            (44, 2),
            (48, 109),
            (52, 3),
            (52, MAX_RECORDS + 1),
            (60, u32::MAX),
            (88, 1),
            (88, 2),
            (92, 24),
            (96, 1),
            (104, 16),
        ] {
            let mut invalid = emf();
            put(&mut invalid, offset, value);
            assert!(
                emf_dimensions(&invalid).is_err(),
                "offset {offset}, value {value}"
            );
        }
        for length in 0..108 {
            assert!(emf_dimensions(&bytes[..length]).is_err());
        }
    }

    #[test]
    fn file_loading_enforces_resource_root_and_size() {
        let root = tempfile::tempdir().unwrap();
        let base = root.path().join("resources");
        fs::create_dir(&base).unwrap();
        fs::write(base.join("image.PNG"), png()).unwrap();
        fs::write(root.path().join("outside.png"), png()).unwrap();
        let resource = Resource::read(&base, "image.PNG").unwrap();
        assert_eq!(resource.extension, "png");
        for relative in [
            "../outside.png",
            "/absolute.png",
            "C:/absolute.png",
            "file:outside.png",
            "folder\\image.png",
            "",
            "image\n.png",
        ] {
            assert!(Resource::read(&base, relative).is_err(), "{relative:?}");
        }
        let oversized = File::create(base.join("large.png")).unwrap();
        oversized.set_len(MAX_BYTES + 1).unwrap();
        assert!(Resource::read(&base, "large.png").is_err());
        assert!(Resource::read(&base, "missing.png").is_err());
    }

    #[test]
    fn separate_resource_root_allows_parent_paths_inside_root() {
        let root = tempfile::tempdir().unwrap();
        let base = root.path().join("manuscript");
        fs::create_dir(&base).unwrap();
        fs::write(root.path().join("image.png"), png()).unwrap();
        let resource = Resource::read_with_root(&base, root.path(), "../image.png").unwrap();
        resource.verify_unchanged().unwrap();
        assert!(Resource::read(&base, "../image.png").is_err());
    }

    #[test]
    fn detects_changed_or_removed_source_before_publication() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("image.png");
        fs::write(&path, png()).unwrap();
        let resource = Resource::read(root.path(), "image.png").unwrap();
        resource.verify_unchanged().unwrap();
        let mut changed = png();
        changed[19] ^= 1;
        fs::write(&path, changed).unwrap();
        assert!(resource.verify_unchanged().is_err());
        fs::write(&path, png()).unwrap();
        resource.verify_unchanged().unwrap();
        fs::remove_file(path).unwrap();
        assert!(resource.verify_unchanged().is_err());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlink_escape() {
        let root = tempfile::tempdir().unwrap();
        let base = root.path().join("resources");
        fs::create_dir(&base).unwrap();
        fs::write(root.path().join("outside.png"), png()).unwrap();
        std::os::unix::fs::symlink(root.path().join("outside.png"), base.join("image.png"))
            .unwrap();
        assert!(Resource::read(&base, "image.png").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn detects_source_symlink_retarget_even_with_identical_bytes() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("first.png"), png()).unwrap();
        fs::write(root.path().join("second.png"), png()).unwrap();
        let link = root.path().join("image.png");
        std::os::unix::fs::symlink(root.path().join("first.png"), &link).unwrap();
        let resource = Resource::read(root.path(), "image.png").unwrap();
        fs::remove_file(&link).unwrap();
        std::os::unix::fs::symlink(root.path().join("second.png"), &link).unwrap();
        assert!(resource.verify_unchanged().is_err());
    }
}
