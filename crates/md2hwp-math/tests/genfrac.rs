// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::tex_to_hml;

#[test]
fn default_generalized_fraction_preserves_operands_and_token_arguments() {
    assert_eq!(
        tex_to_hml(r"\genfrac{}{}{}{}{a}{b}").unwrap(),
        "{{a} over {b}}"
    );
    for tex in [r"\genfrac{}{}{}{}ab", r"\genfrac..{}{}{a}{b}"] {
        let native = tex_to_hml(tex).unwrap();
        assert!(native.contains("{a} over {b}"), "{tex}: {native}");
        assert!(!native.contains("LEFT"), "{tex}: {native}");
    }
    let native = tex_to_hml(r"\genfrac{}{}{}{}{a_b+1}{x^2}_k").unwrap();
    for part in ["a _{b} + 1", "x ^{2}", "_{k}"] {
        assert!(native.contains(part), "{native}");
    }
}

#[test]
fn generalized_fraction_delimiters_are_independent() {
    for tex in [r"\genfrac{[}{)}{}{}{a}{b}", r"\genfrac[){}{}ab"] {
        let native = tex_to_hml(tex).unwrap();
        assert!(native.contains("LEFT ["), "{native}");
        assert!(native.contains("RIGHT )"), "{native}");
        assert!(native.contains("over"), "{native}");
    }
    for tex in [
        r"\genfrac{\langle}{\rbrace}{}{}{a}{b}",
        r"\genfrac\langle\rbrace{}{}ab",
        r"\genfrac{}{]}{}{}{a}{b}",
        r"\genfrac{[}{.}{}{}{a}{b}",
    ] {
        let native = tex_to_hml(tex).unwrap();
        assert!(native.contains("over"), "{tex}: {native}");
        assert!(native.contains("LEFT"), "{tex}: {native}");
        assert!(native.contains("RIGHT"), "{tex}: {native}");
    }
}

#[test]
fn generalized_fraction_dimensions_select_line_presence() {
    for unit in [
        "pt", "mm", "cm", "in", "bp", "pc", "dd", "cc", "sp", "em", "ex", "mu",
    ] {
        for (amount, operation) in [
            ("0", "atop"),
            ("-0.25", "atop"),
            (".5", "over"),
            ("+1.0", "over"),
        ] {
            let tex = format!(r"\genfrac{{}}{{}}{{{amount}{unit}}}{{}}{{a}}{{b}}");
            let native = tex_to_hml(&tex).unwrap();
            assert!(native.contains(operation), "{tex}: {native}");
        }
    }
}

#[test]
fn generalized_fraction_styles_are_scoped_and_empty_style_inherits() {
    for (style, scale) in [(0, 100), (1, 100), (2, 70), (3, 50)] {
        let tex = format!(r"\genfrac{{}}{{}}{{}}{{{style}}}{{a}}{{b}}+z");
        let native = tex_to_hml(&tex).unwrap();
        if scale == 100 {
            assert_eq!(native, "{{a} over {b}} + z");
        } else {
            assert!(native.contains(&format!("scale {scale}")), "{native}");
        }
        assert!(native.ends_with("+ z"), "{native}");
        if scale != 100 {
            assert!(native.contains("scale 100"), "{native}");
        }
    }
    let native = tex_to_hml(r"\scriptstyle\genfrac{}{}{}{}{a}{b}+z").unwrap();
    assert!(native.contains("scale 70"), "{native}");
    assert!(!native.contains("scale 50"), "{native}");
    assert!(native.contains("+ z scale 100"), "{native}");
    for style in [0, 1] {
        let native = tex_to_hml(&format!(
            r"\scriptstyle\genfrac{{}}{{}}{{}}{{{style}}}{{a}}{{b}}+z"
        ))
        .unwrap();
        assert_eq!(
            native, "scale 70 {scale 100 {{a} over {b}} scale 70} + z scale 100",
            "style {style}"
        );
    }
    let native = tex_to_hml(r"\scriptstyle\genfrac{}{}{}{3}{a}{b}+z").unwrap();
    assert!(native.contains("scale 50"), "{native}");
    assert!(native.contains("scale 70"), "{native}");
    assert!(native.contains("+ z scale 100"), "{native}");
}

#[test]
fn generalized_fractions_nest_and_work_inside_annotations() {
    let native = tex_to_hml(r"\genfrac{[}{]}{}{}{\genfrac{}{}{0pt}{}{a}{b}}{c}").unwrap();
    assert!(native.contains("over"), "{native}");
    assert!(native.contains("atop"), "{native}");
    for tex in [
        r"\overset{x}{\genfrac{}{}{0pt}{}{a}{b}}",
        r"\underset{\genfrac{}{}{}{}{a}{b}}{x}",
    ] {
        let native = tex_to_hml(tex).unwrap();
        assert!(native.contains("underover"), "{native}");
        assert!(native.contains('a') && native.contains('b'), "{native}");
    }
}

#[test]
fn malformed_generalized_fraction_arguments_are_rejected() {
    for tex in [
        r"\genfrac",
        r"\genfrac{}{}{}{}{a}",
        r"\genfrac{}{}{}{}{a}{",
        r"\genfrac{ab}{}{}{}{a}{b}",
        r"\genfrac{+}{}{}{}{a}{b}",
        r"\genfrac{\alpha}{}{}{}{a}{b}",
        r"\genfrac{}{abc}{}{}{a}{b}",
        r"\genfrac{}{}{1}{}{a}{b}",
        r"\genfrac{}{}{pt}{}{a}{b}",
        r"\genfrac{}{}{1px}{}{a}{b}",
        r"\genfrac{}{}{1e2pt}{}{a}{b}",
        r"\genfrac{}{}{1..2pt}{}{a}{b}",
        r"\genfrac{}{}{\textwidth}{}{a}{b}",
        r"\genfrac{}{}{}{4}{a}{b}",
        r"\genfrac{}{}{}{-1}{a}{b}",
        r"\genfrac{}{}{}{1.0}{a}{b}",
        r"\genfrac{}{}{}{\scriptstyle}{a}{b}",
    ] {
        assert!(tex_to_hml(tex).is_err(), "accepted {tex}");
    }
}
