// SPDX-License-Identifier: AGPL-3.0-only
//! Unicode letter mappings for the supported mathematical alphabets.
//!
//! The letterlike symbols in these tables fill the historical holes in the
//! Mathematical Alphanumeric Symbols block; they are not contiguous ranges.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MathAlphabet {
    Normal,
    Blackboard,
    Calligraphic,
    Script,
    Fraktur,
}

const SCRIPT_CAPITALS: [char; 26] = [
    '𝒜', 'ℬ', '𝒞', '𝒟', 'ℰ', 'ℱ', '𝒢', 'ℋ', 'ℐ', '𝒥', '𝒦', 'ℒ', 'ℳ', '𝒩', '𝒪', '𝒫', '𝒬', 'ℛ', '𝒮',
    '𝒯', '𝒰', '𝒱', '𝒲', '𝒳', '𝒴', '𝒵',
];

const SCRIPT_LOWERCASE: [char; 26] = [
    '𝒶', '𝒷', '𝒸', '𝒹', 'ℯ', '𝒻', 'ℊ', '𝒽', '𝒾', '𝒿', '𝓀', '𝓁', '𝓂', '𝓃', 'ℴ', '𝓅', '𝓆', '𝓇', '𝓈',
    '𝓉', '𝓊', '𝓋', '𝓌', '𝓍', '𝓎', '𝓏',
];

const FRAKTUR_CAPITALS: [char; 26] = [
    '𝔄', '𝔅', 'ℭ', '𝔇', '𝔈', '𝔉', '𝔊', 'ℌ', 'ℑ', '𝔍', '𝔎', '𝔏', '𝔐', '𝔑', '𝔒', '𝔓', '𝔔', 'ℜ', '𝔖',
    '𝔗', '𝔘', '𝔙', '𝔚', '𝔛', '𝔜', 'ℨ',
];

impl MathAlphabet {
    pub(crate) fn glyph(self, c: char) -> Option<char> {
        match self {
            Self::Normal => Some(c),
            Self::Blackboard => (c == 'k').then_some('𝕜'),
            Self::Calligraphic | Self::Script if c.is_ascii_uppercase() => {
                Some(SCRIPT_CAPITALS[(c as u8 - b'A') as usize])
            }
            Self::Script if c.is_ascii_lowercase() => {
                Some(SCRIPT_LOWERCASE[(c as u8 - b'a') as usize])
            }
            Self::Fraktur if c.is_ascii_uppercase() => {
                Some(FRAKTUR_CAPITALS[(c as u8 - b'A') as usize])
            }
            Self::Fraktur if c.is_ascii_lowercase() => {
                char::from_u32(0x1D51E + u32::from(c as u8 - b'a'))
            }
            _ => None,
        }
    }
}

/// Return the TeX command name (without a backslash) and its ASCII argument.
pub(crate) fn tex_glyph(c: char) -> Option<(&'static str, char)> {
    if c == '𝕜' {
        return Some(("mathbb", 'k'));
    }
    if let Some(index) = FRAKTUR_CAPITALS.iter().position(|&glyph| glyph == c) {
        return Some(("mathfrak", char::from(b'A' + index as u8)));
    }
    let offset = u32::from(c).checked_sub(0x1D51E)?;
    (offset < 26).then(|| ("mathfrak", char::from(b'a' + offset as u8)))
}

#[cfg(test)]
mod tests {
    use super::{MathAlphabet, tex_glyph};
    use std::collections::HashSet;

    #[test]
    fn script_capitals_are_unique_and_include_letterlike_symbols() {
        let glyphs: Vec<_> = ('A'..='Z')
            .map(|c| MathAlphabet::Calligraphic.glyph(c).unwrap())
            .collect();
        assert_eq!(glyphs.iter().copied().collect::<HashSet<_>>().len(), 26);
        assert_eq!(MathAlphabet::Calligraphic.glyph('B'), Some('ℬ'));
        assert_eq!(MathAlphabet::Calligraphic.glyph('H'), Some('ℋ'));
        assert_eq!(MathAlphabet::Calligraphic.glyph('L'), Some('ℒ'));
        assert_eq!(MathAlphabet::Calligraphic.glyph('R'), Some('ℛ'));
        assert_eq!(MathAlphabet::Calligraphic.glyph('a'), None);
    }

    #[test]
    fn fraktur_letters_round_trip_bijectively() {
        let mut glyphs = HashSet::new();
        for letter in ('A'..='Z').chain('a'..='z') {
            let glyph = MathAlphabet::Fraktur.glyph(letter).unwrap();
            assert!(glyphs.insert(glyph));
            assert_eq!(tex_glyph(glyph), Some(("mathfrak", letter)));
        }
        for (letter, glyph) in [('C', 'ℭ'), ('H', 'ℌ'), ('I', 'ℑ'), ('R', 'ℜ'), ('Z', 'ℨ')]
        {
            assert_eq!(MathAlphabet::Fraktur.glyph(letter), Some(glyph));
        }
        assert_eq!(MathAlphabet::Fraktur.glyph('0'), None);
        // Unassigned code points at the historical capital-letter holes.
        for hole in [
            '\u{1D506}',
            '\u{1D50B}',
            '\u{1D50C}',
            '\u{1D515}',
            '\u{1D51D}',
        ] {
            assert_eq!(tex_glyph(hole), None);
        }
    }

    #[test]
    fn blackboard_support_is_limited_to_k() {
        assert_eq!(MathAlphabet::Blackboard.glyph('k'), Some('𝕜'));
        assert_eq!(tex_glyph('𝕜'), Some(("mathbb", 'k')));
        for unsupported in ['A', 'a', 'j', 'l', '0', 'ℝ', '𝕒'] {
            assert_eq!(MathAlphabet::Blackboard.glyph(unsupported), None);
            assert_eq!(tex_glyph(unsupported), None);
        }
    }

    #[test]
    fn normal_preserves_unicode() {
        for c in ['A', 'é', '한', '𝔄', '😀'] {
            assert_eq!(MathAlphabet::Normal.glyph(c), Some(c));
        }
    }
}
