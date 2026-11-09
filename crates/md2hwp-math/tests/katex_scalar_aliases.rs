// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::{hml_to_tex, tex_to_hml};

#[test]
fn katex_scalar_aliases_preserve_distinct_math_glyphs() {
    let native = tex_to_hml(r"\maltese\degree\pounds\$\minuso").unwrap();
    assert_eq!(native, "\"✠\" DEG \"£\" \"$\" \"⦵\"");
    assert!(!native.contains("OMINUS"));
    assert!(hml_to_tex(&native).is_ok());
}

#[test]
fn katex_scalar_aliases_work_inside_text() {
    let native = tex_to_hml(r"\text{\maltese\degree\pounds\textdollar}").unwrap();
    for glyph in ["✠", "°", "£", "$"] {
        assert!(native.contains(glyph), "{native}");
    }
    assert!(hml_to_tex(&native).is_ok());
}

#[test]
fn unknown_scalar_names_still_fail() {
    assert!(tex_to_hml(r"\malteseUnknown").is_err());
    assert!(tex_to_hml(r"\text{\textdollarUnknown}").is_err());
}
