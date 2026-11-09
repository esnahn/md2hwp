// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::{hml_to_tex, tex_to_hml};

#[test]
fn script_lowercase_roundtrips_including_letterlike_symbol_exceptions() {
    for (glyph, letter) in "𝒶𝒷𝒸𝒹ℯ𝒻ℊ𝒽𝒾𝒿𝓀𝓁𝓂𝓃ℴ𝓅𝓆𝓇𝓈𝓉𝓊𝓋𝓌𝓍𝓎𝓏".chars().zip('a'..='z')
    {
        let tex = format!("\\mathscr{{{letter}}}");
        let native = tex_to_hml(&tex).unwrap();
        assert!(native.contains(glyph), "{letter}: {native}");
        assert_eq!(hml_to_tex(&format!("\"{glyph}\"")).unwrap(), tex);
        assert!(hml_to_tex(&native).unwrap().contains(&tex));
    }
    let native = tex_to_hml(r"\mathscr{aBceGo}").unwrap();
    assert!(
        native.contains('𝒶')
            && native.contains('ℬ')
            && native.contains('ℯ')
            && native.contains('ℴ')
    );
    assert!(tex_to_hml(r"\mathscr{123}").unwrap().contains("123"));
}

#[test]
fn quoted_script_capitals_export_as_mathscr() {
    for (glyph, letter) in "𝒜ℬ𝒞𝒟ℰℱ𝒢ℋℐ𝒥𝒦ℒℳ𝒩𝒪𝒫𝒬ℛ𝒮𝒯𝒰𝒱𝒲𝒳𝒴𝒵".chars().zip('A'..='Z')
    {
        assert_eq!(
            hml_to_tex(&format!("\"{glyph}\"")).unwrap(),
            format!("\\mathscr{{{letter}}}"),
            "{glyph}"
        );
    }
}

#[test]
fn native_keywords_and_mixed_quoted_text_keep_their_meaning() {
    assert_eq!(hml_to_tex("LAPLACE").unwrap(), r"\mathcal{L}");
    assert_eq!(hml_to_tex("\"𝒞x\"").unwrap(), r"\mathit{𝒞x}");
}

#[test]
fn quoted_monospace_alphabet_and_punctuation_export_as_mathtt() {
    let glyphs: String = (0x1d670..=0x1d6a3)
        .chain(0x1d7f6..=0x1d7ff)
        .map(|value| char::from_u32(value).unwrap())
        .collect();
    assert_eq!(
        hml_to_tex(&format!("\"{glyphs} -_!\"")).unwrap(),
        r"\mathtt{ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789\ -\_!}"
    );
    assert_eq!(
        hml_to_tex("\"𝚊𝒞𝟷\"").unwrap(),
        r"\mathtt{a}\mathit{𝒞}\mathtt{1}"
    );
}

#[test]
fn script_accepts_nested_math_and_restores_alphabet_scopes() {
    let native = tex_to_hml(r"\mathscr{a+\frac{b}{\mathfrak{c}}d}e").unwrap();
    for glyph in ['𝒶', '𝒷', '𝔠', '𝒹'] {
        assert!(native.contains(glyph), "{glyph}: {native}");
    }
    assert!(!native.contains('ℯ'), "{native}");
    let native = tex_to_hml(r"\mathscr{A+123+\alpha}").unwrap();
    assert!(
        native.contains('𝒜') && native.contains("123") && native.contains("alpha"),
        "{native}"
    );
    assert!(tex_to_hml(r"\mathscr{}").is_err());
}
