// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::tex_to_hml;

#[test]
fn all_named_font_sizes_use_rounded_ten_point_scales_in_math_and_text() {
    for (name, scale) in [
        ("tiny", 50),
        ("scriptsize", 70),
        ("footnotesize", 80),
        ("small", 90),
        ("normalsize", 100),
        ("large", 120),
        ("Large", 144),
        ("LARGE", 173),
        ("huge", 207),
        ("Huge", 249),
    ] {
        let tail = if scale == 100 {
            String::new()
        } else {
            " scale 100".into()
        };
        let prefix = if scale == 100 {
            String::new()
        } else {
            format!("scale {scale} ")
        };
        assert_eq!(
            tex_to_hml(&format!("\\{name} x")).unwrap(),
            format!("{prefix}x{tail}")
        );
        let text = tex_to_hml(&format!("\\text{{\\{name} abc}}")).unwrap();
        assert!(text.contains(&format!("{prefix}\"abc\"")), "{text}");
        if scale == 100 {
            assert!(!text.contains("scale"), "{text}");
        }
        assert!(md2hwp_math::hml_to_tex(&text).is_ok(), "{text}");
    }
    assert_eq!(
        tex_to_hml(r"{\LARGE a{\tiny b}c}d").unwrap(),
        "{scale 173 a {scale 50 b scale 173} c scale 100} d"
    );
    let text = tex_to_hml(r"\text{\Huge a{\tiny b}c}d").unwrap();
    assert!(text.contains("scale 50 \"b\" scale 249"), "{text}");
    assert!(text.contains("\"c\" scale 100"), "{text}");
    let colored = tex_to_hml(r"\color{red}\Huge x").unwrap();
    assert!(!colored.contains("{scale 249}"), "{colored}");
    assert!(md2hwp_math::hml_to_tex(&colored).is_ok());
}

#[test]
fn default_scale_declarations_are_omitted_but_restore_smaller_scales() {
    for command in ["displaystyle", "textstyle", "normalsize"] {
        assert_eq!(tex_to_hml(&format!(r"\{command} x")).unwrap(), "x");
        assert_eq!(
            tex_to_hml(&format!(r"\scriptstyle a {{\{command} b}} c")).unwrap(),
            "scale 70 a {scale 100 b scale 70} c scale 100",
            "{command}"
        );
    }
    let text = tex_to_hml(r"\text{\normalsize abc}").unwrap();
    assert!(!text.contains("scale"), "{text}");
    assert!(text.contains("\"abc\""), "{text}");
    let text = tex_to_hml(r"\text{\scriptsize a{\normalsize b}c}").unwrap();
    assert!(text.contains("scale 100 \"b\" scale 70"), "{text}");
}

#[test]
fn math_styles_emit_absolute_native_scales_and_restore_group_scope() {
    for (tex, native) in [
        (r"\scriptstyle a", "scale 70 a scale 100"),
        (r"\scriptscriptstyle a", "scale 50 a scale 100"),
        (r"a {\scriptstyle b} c", "a {scale 70 b scale 100} c"),
        (
            r"\scriptstyle a {\scriptscriptstyle b} c",
            "scale 70 a {scale 50 b scale 70} c scale 100",
        ),
        (
            r"\scriptstyle\scriptstyle a",
            "scale 70 scale 70 a scale 100",
        ),
        (r"\scriptstyle a \textstyle b", "scale 70 a scale 100 b"),
        (
            r"\scriptscriptstyle a \displaystyle b",
            "scale 50 a scale 100 b",
        ),
        (
            r"\scriptstyle a {\textstyle\sum} b",
            "scale 70 a {scale 100 SMALLSUM scale 70} b scale 100",
        ),
    ] {
        assert_eq!(tex_to_hml(tex).unwrap(), native, "{tex}");
    }
}

#[test]
fn styles_restore_cell_and_annotation_scopes_and_preserve_small_operators() {
    assert_eq!(
        tex_to_hml(r"\begin{matrix}\scriptstyle a&b\end{matrix}").unwrap(),
        "{matrix {scale 70 a scale 100 & b}}"
    );
    assert_eq!(
        tex_to_hml(r"\overset{a}{\scriptstyle b}").unwrap(),
        "{underover {scale 70 b scale 100}^{a}}"
    );
    assert_eq!(
        tex_to_hml(r"{\textstyle\sum}_{i}").unwrap(),
        "{SMALLSUM} _{i}"
    );
    assert_eq!(
        tex_to_hml(r"\textstyle\frac{a}{b}").unwrap(),
        "{{a} over {b}}"
    );
    for tex in [
        r"\scriptstylebad a",
        r"\scriptscriptstylebad a",
        r"\textstyle\sumthing",
    ] {
        assert!(tex_to_hml(tex).is_err(), "{tex}");
    }
    assert!(
        tex_to_hml(&format!(
            "{}a{}",
            r"{\scriptstyle ".repeat(100),
            "}".repeat(100)
        ))
        .is_err()
    );
}
