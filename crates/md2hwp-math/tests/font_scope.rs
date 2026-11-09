// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::{hml_to_tex, tex_to_hml};

#[test]
fn tex_font_scopes_restore_the_native_font_before_following_math() {
    for (tex, native) in [
        (r"\mathrm{a} c", "{rm {a} it} c"),
        (r"\operatorname*{Lim}\limits_{a} c", "Lim _{a} c"),
        (r"\mathrm{a \mathit{b} c} d", "{rm {a {it {b} rm} c} it} d"),
        (r"\mathit{a \mathrm{b} c} d", "{a {rm {b} it} c} d"),
        (r"\mathrm{a \text{B} c} d", "{rm {a {\"B\"} c} it} d"),
        (
            r"\mathbf{a \mathrm{b} c} d",
            "{rm bold {a {rm {b} rm bold} c} it} d",
        ),
        (
            r"\mathrm{a \mathbf{b} c} d",
            "{rm {a {rm bold {b} rm} c} it} d",
        ),
    ] {
        assert_eq!(tex_to_hml(tex).unwrap(), native, "{tex}");
        let exported = hml_to_tex(native).unwrap();
        assert!(tex_to_hml(&exported).is_ok(), "{exported}");
    }
}

#[test]
fn restoring_italic_changes_both_the_limit_and_the_following_variable() {
    assert_eq!(
        hml_to_tex(r#"underover {rm "Lim" it} _{a} c"#).unwrap(),
        "\\operatorname*{Lim}\\limits_{a} c"
    );
    assert_eq!(
        hml_to_tex(r#"underover {rm "Lim"} _{a} c"#).unwrap(),
        "\\operatorname*{Lim}\\limits_{\\mathrm{a}} \\mathrm{c}"
    );
}
