// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::tex_to_hml;

#[test]
fn above_dimensions_select_native_fraction_line_presence() {
    for unit in [
        "pt", "mm", "cm", "in", "bp", "pc", "dd", "cc", "sp", "em", "ex", "mu",
    ] {
        for (amount, operation) in [
            ("0", "atop"),
            ("-0.25", "atop"),
            (".5", "over"),
            ("+1.0", "over"),
        ] {
            for thickness in [format!("{amount}{unit}"), format!("{{{amount}{unit}}}")] {
                let tex = format!(r"a\above{thickness} b");
                assert_eq!(
                    tex_to_hml(&tex).unwrap(),
                    format!("{{{{a}} {operation} {{b}}}}"),
                    "{tex}"
                );
            }
        }
    }
    for tex in [r"a\above1pt b", r"a\above{1pt} b"] {
        assert_eq!(tex_to_hml(tex).unwrap(), "{{a} over {b}}");
    }
}

#[test]
fn above_scopes_the_current_group_cell_and_annotation() {
    for (above, equivalent) in [
        (r"z+{a+b\above1pt c+d}+w", r"z+{a+b\over c+d}+w"),
        (
            r"{{a\above0pt b}\above1pt {c\above-1pt d}}",
            r"{{a\atop b}\over {c\atop d}}",
        ),
        (r"\frac{a\above1pt b}{c}", r"\frac{a\over b}{c}"),
        (r"\left[a\above1pt b\right]", r"\left[a\over b\right]"),
        (
            r"\begin{matrix}a\above1pt b&c\\d&e\above0pt f\end{matrix}",
            r"\begin{matrix}a\over b&c\\d&e\atop f\end{matrix}",
        ),
        (r"\overset{x}{a\above1pt b}", r"\overset{x}{a\over b}"),
        (r"\underset{a\above0pt b}{x}", r"\underset{a\atop b}{x}"),
        (
            r"\overset{x}{\to\above1pt\leftarrow}",
            r"\overset{x}{\to\over\leftarrow}",
        ),
    ] {
        assert_eq!(
            tex_to_hml(above).unwrap(),
            tex_to_hml(equivalent).unwrap(),
            "{above}"
        );
    }
}

#[test]
fn above_rejects_duplicate_infix_and_missing_operands() {
    for tex in [
        r"a\above1pt b\above1pt c",
        r"a\above1pt b\over c",
        r"a\atop b\above1pt c",
        r"a\above1pt b\brace c",
        r"\overset{x}{a\above1pt b\atop c}",
        r"\above1pt b",
        r"a\above1pt",
        r"\begin{matrix}a&\above1pt b\end{matrix}",
        r"\overset{x}{a\above1pt}",
    ] {
        assert!(tex_to_hml(tex).is_err(), "accepted {tex}");
    }
}

#[test]
fn above_rejects_malformed_thickness_and_units() {
    for tex in [
        r"a\above b",
        r"a\above{} b",
        r"a\above{1} b",
        r"a\above{pt} b",
        r"a\above1px b",
        r"a\above{1px} b",
        r"a\above1e2pt b",
        r"a\above{1e2pt} b",
        r"a\above1..2pt b",
        r"a\above{1..2pt} b",
        r"a\above{\textwidth} b",
        r"a\above{1pt b",
    ] {
        assert!(tex_to_hml(tex).is_err(), "accepted {tex}");
    }
}

#[test]
fn original_katex_fraction_case_converts_without_fixture_rewriting() {
    let fixtures: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/katex/cases.json")).unwrap();
    let case = fixtures["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["id"] == "katex-032")
        .unwrap();
    let tex = case["tex"].as_str().unwrap();
    assert!(tex.contains(r"\above1.0pt"));
    let native = tex_to_hml(tex).unwrap();
    assert!(native.contains("over"), "{native}");
    assert!(!native.contains('\\'), "{native}");
}
