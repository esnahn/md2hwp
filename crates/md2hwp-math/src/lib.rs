// SPDX-License-Identifier: AGPL-3.0-only
//! Direct TeX / Hancom equation SCRIPT conversion, independent of document APIs.
#![forbid(unsafe_code)]

mod breve_unicode;
pub mod hml_to_tex;
mod katex_color_commands;
mod math_alphabet;
mod tex_colors;
mod tex_style;
pub mod tex_to_hml;

pub use hml_to_tex::{PortError, hml_to_tex};
pub use tex_to_hml::{TexError, tex_to_hml, tex_to_hml_with_display};

pub(crate) fn is_hangul_letter(c: char) -> bool {
    c.is_alphabetic()
        && matches!(c,
            '\u{1100}'..='\u{11ff}' | '\u{3130}'..='\u{318f}' |
            '\u{a960}'..='\u{a97f}' | '\u{ac00}'..='\u{d7a3}' |
            '\u{d7b0}'..='\u{d7ff}' | '\u{ffa0}'..='\u{ffdc}')
}
