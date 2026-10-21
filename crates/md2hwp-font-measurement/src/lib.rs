// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Euisoon Ahn
//! Safe, thread-confined Windows GDI font measurements in scaled logical units.

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FontSpec {
    pub face: String,
    pub height: i32,
    pub bold: bool,
    pub italic: bool,
}

#[cfg(windows)]
mod platform {
    use super::FontSpec;
    use std::{collections::HashMap, ffi::c_void, marker::PhantomData, ptr, rc::Rc};

    type Handle = *mut c_void;
    #[repr(C)]
    struct Size {
        width: i32,
        height: i32,
    }

    #[link(name = "gdi32")]
    unsafe extern "system" {
        fn CreateCompatibleDC(device: Handle) -> Handle;
        fn GetCurrentObject(device: Handle, kind: u32) -> Handle;
        fn CreateFontW(
            height: i32,
            width: i32,
            escapement: i32,
            orientation: i32,
            weight: i32,
            italic: u32,
            underline: u32,
            strike: u32,
            charset: u32,
            output: u32,
            clip: u32,
            quality: u32,
            pitch: u32,
            face: *const u16,
        ) -> Handle;
        fn SelectObject(device: Handle, object: Handle) -> Handle;
        fn GetTextFaceW(device: Handle, count: i32, face: *mut u16) -> i32;
        fn GetTextExtentPoint32W(
            device: Handle,
            text: *const u16,
            count: i32,
            size: *mut Size,
        ) -> i32;
        fn GetFontData(
            device: Handle,
            table: u32,
            offset: u32,
            buffer: *mut c_void,
            length: u32,
        ) -> u32;
        fn DeleteObject(object: Handle) -> i32;
        fn DeleteDC(device: Handle) -> i32;
    }

    fn error(action: &str) -> String {
        format!("{action}: {}", std::io::Error::last_os_error())
    }

    /// Owns its DC and created fonts; never shares GDI handles across threads.
    pub struct Measurement {
        device: Handle,
        original_font: Handle,
        fonts: HashMap<FontSpec, Handle>,
        _thread_confined: PhantomData<Rc<()>>,
    }

    impl Measurement {
        pub fn new() -> Result<Self, String> {
            // SAFETY: null requests a private memory DC. On success this instance
            // exclusively owns it until Drop; the stock original font is borrowed.
            let device = unsafe { CreateCompatibleDC(ptr::null_mut()) };
            if device.is_null() {
                return Err(error("Could not create font measurement context"));
            }
            // SAFETY: device is the valid, newly owned DC; OBJ_FONT is 6.
            let original_font = unsafe { GetCurrentObject(device, 6) };
            if original_font.is_null() {
                let message = error("Could not read original measurement font");
                // SAFETY: releasing the just-created DC, no created fonts exist.
                unsafe {
                    DeleteDC(device);
                }
                return Err(message);
            }
            Ok(Self {
                device,
                original_font,
                fonts: HashMap::new(),
                _thread_confined: PhantomData,
            })
        }

        pub fn extent(&mut self, spec: &FontSpec, text: &str) -> Result<i32, String> {
            let units: Vec<u16> = text.encode_utf16().collect();
            let count =
                i32::try_from(units.len()).map_err(|_| "Font measurement text is too long")?;
            let face: Vec<u16> = spec.face.encode_utf16().chain(Some(0)).collect();
            if spec.height <= 0 || face.len() <= 1 || face.len() > 32 || spec.face.contains('\0') {
                return Err("Invalid font measurement face or height".into());
            }
            let font = if let Some(font) = self.fonts.get(spec) {
                *font
            } else {
                // SAFETY: face is terminated UTF-16 valid for the call. All other
                // parameters are documented scalar GDI font selection values.
                let font = unsafe {
                    CreateFontW(
                        -spec.height,
                        0,
                        0,
                        0,
                        if spec.bold { 700 } else { 400 },
                        u32::from(spec.italic),
                        0,
                        0,
                        1,
                        4,
                        0,
                        2,
                        0,
                        face.as_ptr(),
                    )
                };
                if font.is_null() {
                    return Err(error("Could not create table measurement font"));
                }
                self.fonts.insert(spec.clone(), font);
                font
            };
            // SAFETY: both handles belong to this instance and remain alive;
            // the original font is restored before any font is deleted in Drop.
            let previous = unsafe { SelectObject(self.device, font) };
            if previous.is_null() || previous as isize == -1 {
                return Err(error("Could not select table font"));
            }
            let mut selected = [0u16; 32];
            // SAFETY: writable buffer has exactly the capacity passed to GDI.
            if unsafe { GetTextFaceW(self.device, 32, selected.as_mut_ptr()) } == 0 {
                return Err(error("Could not identify table measurement font"));
            }
            let end = selected
                .iter()
                .position(|v| *v == 0)
                .unwrap_or(selected.len());
            let selected = String::from_utf16_lossy(&selected[..end]);
            if selected.to_lowercase() != spec.face.to_lowercase() {
                return Err(format!(
                    "Table font {} is unavailable to Windows; GDI selected {selected}. Install the font or change the table sample font.",
                    spec.face
                ));
            }
            // SAFETY: with a null output buffer GetFontData only queries the
            // installed sfnt font data size. Raster/device fonts return GDI_ERROR.
            if unsafe { GetFontData(self.device, 0, 0, ptr::null_mut(), 0) } == u32::MAX {
                return Err(
                    "Table automatic widths require an installed Windows TrueType/OpenType font"
                        .into(),
                );
            }
            let mut size = Size {
                width: 0,
                height: 0,
            };
            // SAFETY: UTF-16 buffer remains alive, count matches its checked
            // length (not Unicode scalar count), and size is a valid output.
            if unsafe { GetTextExtentPoint32W(self.device, units.as_ptr(), count, &mut size) } == 0
            {
                return Err(error("Could not measure table text"));
            }
            Ok(size.width)
        }
    }

    impl Drop for Measurement {
        fn drop(&mut self) {
            // SAFETY: restore the borrowed stock font before deleting any owned
            // fonts; all handles were created by this instance on this thread.
            unsafe {
                SelectObject(self.device, self.original_font);
                // Delete the DC first: even if restoring the stock object failed,
                // no owned font remains selected when DeleteObject is called.
                DeleteDC(self.device);
                for font in self.fonts.values() {
                    DeleteObject(*font);
                }
            }
        }
    }
}

#[cfg(not(windows))]
mod platform {
    use super::FontSpec;
    pub struct Measurement;
    impl Measurement {
        pub fn new() -> Result<Self, String> {
            Err(
                "Table automatic widths require Windows GDI and installed TrueType/OpenType fonts"
                    .into(),
            )
        }
        pub fn extent(&mut self, _: &FontSpec, _: &str) -> Result<i32, String> {
            Self::new().map(|_| 0)
        }
    }
}

pub use platform::Measurement;

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn installed_font_measures_utf16_without_face_buffer_length_limit() {
        let mut measurement = Measurement::new().unwrap();
        let font = FontSpec {
            face: "Arial".into(),
            height: 4000,
            bold: false,
            italic: false,
        };
        let narrow = measurement.extent(&font, "i").unwrap();
        let wide = measurement.extent(&font, "W").unwrap();
        assert!(wide > narrow && narrow > 0);
        // A text buffer is independent of LOGFONT's 32 UTF-16-unit face buffer.
        assert!(measurement.extent(&font, &"a\u{301}😀".repeat(40)).unwrap() > wide);
    }

    #[test]
    fn unavailable_face_fails_instead_of_silently_measuring_a_substitute() {
        let mut measurement = Measurement::new().unwrap();
        let font = FontSpec {
            face: "md2hwp missing font 934173".into(),
            height: 4000,
            bold: false,
            italic: false,
        };
        assert!(
            measurement
                .extent(&font, "text")
                .unwrap_err()
                .contains("unavailable to Windows")
        );
    }
}
