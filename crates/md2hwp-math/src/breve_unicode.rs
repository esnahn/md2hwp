// SPDX-License-Identifier: AGPL-3.0-only
// Canonical composition facts generated with Python unicodedata NFC,
// Unicode 17.0.0: each Unicode scalar followed by U+0306 COMBINING BREVE.

/// Returns the precomposed scalar for a base followed by a breve, when available.
pub(crate) fn compose_breve(base: char) -> Option<char> {
    match base {
        '\u{0041}' => Some('\u{0102}'),
        '\u{0045}' => Some('\u{0114}'),
        '\u{0047}' => Some('\u{011e}'),
        '\u{0049}' => Some('\u{012c}'),
        '\u{004f}' => Some('\u{014e}'),
        '\u{0055}' => Some('\u{016c}'),
        '\u{0061}' => Some('\u{0103}'),
        '\u{0065}' => Some('\u{0115}'),
        '\u{0067}' => Some('\u{011f}'),
        '\u{0069}' => Some('\u{012d}'),
        '\u{006f}' => Some('\u{014f}'),
        '\u{0075}' => Some('\u{016d}'),
        '\u{0228}' => Some('\u{1e1c}'),
        '\u{0229}' => Some('\u{1e1d}'),
        '\u{0391}' => Some('\u{1fb8}'),
        '\u{0399}' => Some('\u{1fd8}'),
        '\u{03a5}' => Some('\u{1fe8}'),
        '\u{03b1}' => Some('\u{1fb0}'),
        '\u{03b9}' => Some('\u{1fd0}'),
        '\u{03c5}' => Some('\u{1fe0}'),
        '\u{0410}' => Some('\u{04d0}'),
        '\u{0415}' => Some('\u{04d6}'),
        '\u{0416}' => Some('\u{04c1}'),
        '\u{0418}' => Some('\u{0419}'),
        '\u{0423}' => Some('\u{040e}'),
        '\u{0430}' => Some('\u{04d1}'),
        '\u{0435}' => Some('\u{04d7}'),
        '\u{0436}' => Some('\u{04c2}'),
        '\u{0438}' => Some('\u{0439}'),
        '\u{0443}' => Some('\u{045e}'),
        '\u{1ea0}' => Some('\u{1eb6}'),
        '\u{1ea1}' => Some('\u{1eb7}'),
        '\u{1fbe}' => Some('\u{1fd0}'),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::compose_breve;

    #[test]
    fn composes_supported_latin_greek_and_cyrillic_letters() {
        assert_eq!(compose_breve('A'), Some('Ă'));
        assert_eq!(compose_breve('a'), Some('ă'));
        assert_eq!(compose_breve('α'), Some('ᾰ'));
        assert_eq!(compose_breve('И'), Some('Й'));
    }

    #[test]
    fn leaves_unsupported_bases_uncomposed() {
        assert_eq!(compose_breve('x'), None);
    }
}
