// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::{hml_to_tex, tex_to_hml};

#[test]
fn breve_prefers_precomposed_characters() {
    for (tex, expected) in [
        (r"\breve a", "\"ă\""),
        (r"\breve{{A}}", "\"Ă\""),
        (r"\breve{e}", "\"ĕ\""),
        (r"\breve{\alpha}", "\"ᾰ\""),
        (r"\breve{И}", "\"Й\""),
    ] {
        assert_eq!(tex_to_hml(tex).unwrap(), expected, "{tex}");
    }
}

#[test]
fn breve_uses_adjacent_spacing_mark_when_composition_is_unavailable() {
    assert_eq!(tex_to_hml(r"\breve x").unwrap(), "{x \"˘\"}");
    let native = tex_to_hml(r"\breve{AB}").unwrap();
    assert!(native.contains('˘'));
    assert!(!native.contains("underover"));
    assert!(!native.contains('\u{0306}'));
}

#[test]
fn breve_keeps_formatting_and_outer_scripts() {
    for tex in [
        r"\breve{AB}",
        r"\breve{a}^2",
        r"\frac{\breve{a}}{b}",
        r"\mathbf{\breve{a}}",
    ] {
        let native = tex_to_hml(tex).unwrap();
        assert!(!native.contains("underover"));
        assert!(!native.contains('\u{0306}'));
        assert!(!native.contains("arch"));
        let exported = hml_to_tex(&native).unwrap();
        assert!(tex_to_hml(&exported).is_ok(), "{exported}");
    }
    for tex in [r"\breve", r"\breve{a", r"\brevebad{a}"] {
        assert!(tex_to_hml(tex).is_err(), "{tex}");
    }
}
