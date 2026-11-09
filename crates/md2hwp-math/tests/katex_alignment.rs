// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::{hml_to_tex, tex_to_hml};

fn fixture_tex(id: &str) -> String {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/katex/cases.json")).unwrap();
    fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["id"] == id)
        .unwrap()["tex"]
        .as_str()
        .unwrap()
        .to_owned()
}

#[test]
fn rcases_uses_an_invisible_left_and_visible_right_curly_delimiter() {
    let native = tex_to_hml(r"\begin{rcases}a&b\\c&d\end{rcases}").unwrap();
    assert_eq!(native, "{LEFT . matrix {a & b # c & d} RIGHT }}");
    let tex = hml_to_tex(&native).unwrap();
    assert!(tex.contains(r"\left.") && tex.contains(r"\right\}"));
    assert_eq!(
        tex_to_hml(&tex).unwrap(),
        "{LEFT . {matrix {a & b # c & d}} RIGHT }}"
    );
    let complete = tex_to_hml(&fixture_tex("katex-017")).unwrap();
    assert!(complete.contains("LEFT . matrix"));
    assert!(hml_to_tex(&complete).is_ok(), "{complete}");
    assert!(tex_to_hml(r"\begin{rcases}a&b\\c\end{rcases}").is_err());
    assert!(tex_to_hml(r"\begin{rcases}a&b\end{cases}").is_err());
    assert!(tex_to_hml(r"\left[a\right\}").is_err());
}

#[test]
fn array_horizontal_rules_do_not_change_cell_contents() {
    let plain = tex_to_hml(r"\begin{array}{cc}a&b\\c&d\end{array}").unwrap();
    let ruled =
        tex_to_hml(r"\begin{array}{|c:c|}\hline a&b\\\hline\hdashline c&d\\\hline\end{array}")
            .unwrap();
    assert_eq!(ruled, plain);
}

#[test]
fn row_spacing_is_discarded_without_parsing_its_contents() {
    for env in ["matrix", "aligned", "array"] {
        let columns = if env == "array" { "{c}" } else { "" };
        let plain = format!(r"\begin{{{env}}}{columns}a\\b\end{{{env}}}");
        for spacing in ["1ex", "-2pt", r"{\unknownDimension}"] {
            let input = format!(r"\begin{{{env}}}{columns}a\\[{spacing}]b\end{{{env}}}");
            assert_eq!(tex_to_hml(&input).unwrap(), tex_to_hml(&plain).unwrap());
        }
    }
    assert!(tex_to_hml(r"\begin{matrix}a\\[1ex b\end{matrix}").is_err());
}

#[test]
fn small_arrays_keep_cells_alignment_and_restore_size() {
    let small = tex_to_hml(r"\begin{smallmatrix}a&b\\c&d\end{smallmatrix}x").unwrap();
    assert_eq!(small, "{scale 70 {matrix {a & b # c & d}} scale 100} x");
    assert!(hml_to_tex(&small).is_ok());
    assert_eq!(
        tex_to_hml(r"\begin{smallmatrix}a&b\\\end{smallmatrix}").unwrap(),
        tex_to_hml(r"\begin{smallmatrix}a&b\end{smallmatrix}").unwrap()
    );
    let nested = tex_to_hml(
        r"\scriptstyle\begin{smallmatrix}\begin{subarray}{l}a\\b\end{subarray}&c\end{smallmatrix}x",
    )
    .unwrap();
    assert!(nested.contains("{scale 70 {lpile {a # b}} scale 70}"));
    assert!(nested.ends_with("scale 70} x scale 100"));
    for (align, native) in [("c", "pile"), ("l", "lpile")] {
        let tex = format!(r"\begin{{subarray}}{{{align}}}a\\b\end{{subarray}}");
        assert_eq!(
            tex_to_hml(&tex).unwrap(),
            format!("{{scale 70 {{{native} {{a # b}}}} scale 100}}")
        );
    }
    assert!(tex_to_hml(r"\begin{subarray}{cc}a\end{subarray}").is_err());
    assert!(tex_to_hml(r"\begin{subarray}{c}a&b\end{subarray}").is_err());
    let tex = fixture_tex("katex-006");
    let native = tex_to_hml(&tex).unwrap();
    assert!(native.starts_with("{pile {"));
    assert!(native.contains("scale 70 {matrix {a & b # c & d}} scale 100"));
    assert!(native.contains("scale 70 {pile {a # b}} scale 100"));
    assert!(hml_to_tex(&native).is_ok(), "{native}");
}

#[test]
fn equation_line_breaks_keep_nested_arrays_and_fraction_scopes() {
    assert_eq!(tex_to_hml(r"a\\[1ex]b").unwrap(), "{pile {a # b}}");
    assert_eq!(tex_to_hml(r"a\\").unwrap(), "{pile {a}}");
    let native = tex_to_hml(r"a\over b\\c\over d").unwrap();
    assert_eq!(native, "{pile {{{a} over {b}} # {{c} over {d}}}}");
    assert!(hml_to_tex(&native).is_ok());
    let native = tex_to_hml(r"\begin{matrix}a&b\\c&d\end{matrix}\\x").unwrap();
    assert_eq!(native, "{pile {{matrix {a & b # c & d}} # x}}");
    assert!(tex_to_hml(r"\\a").is_err());
}

#[test]
fn cr_is_a_row_separator_with_a_control_word_boundary() {
    for env in ["matrix", "aligned", "array"] {
        let argument = if env == "array" { "{cc}" } else { "" };
        let input = format!(r"\begin{{{env}}}{argument}a&b\cr c&d\end{{{env}}}");
        let standard = input.replace(r"\cr ", r"\\");
        assert_eq!(tex_to_hml(&input).unwrap(), tex_to_hml(&standard).unwrap());
    }
    assert!(tex_to_hml(r"\begin{matrix}a\crane b\end{matrix}").is_err());
    assert!(tex_to_hml(r"a\cr b").is_err());
}

#[test]
fn katex_align_keeps_math_while_omitting_equation_numbering() {
    let native = tex_to_hml(&fixture_tex("katex-003")).unwrap();
    assert!(native.starts_with("{eqalign {"), "{native}");
    assert!(native.contains("NEQ"), "{native}");
    assert!(!native.contains("tag"), "{native}");
    assert!(!native.contains("nonumber"), "{native}");
    assert!(!native.contains("AST"), "{native}");
    assert!(hml_to_tex(&native).is_ok(), "{native}");

    for command in [
        r"\tag{1}",
        r"\tag*{A}",
        r"\notag",
        r"\nonumber",
        r"\label{eq:a}",
    ] {
        let tex = format!(r"\begin{{align}}a&=b{command}\end{{align}}");
        assert_eq!(
            tex_to_hml(&tex).unwrap(),
            tex_to_hml(r"\begin{aligned}a&=b\end{aligned}").unwrap(),
            "{tex}"
        );
    }
}

#[test]
fn katex_alignedat_uses_the_same_native_alignment_cells() {
    let tex = fixture_tex("katex-005");
    let aligned = tex
        .replace(r"\begin{alignedat}{3}", r"\begin{aligned}")
        .replace(r"\end{alignedat}", r"\end{aligned}");
    let native = tex_to_hml(&tex).unwrap();
    assert_eq!(native, tex_to_hml(&aligned).unwrap());
    assert!(hml_to_tex(&native).is_ok(), "{native}");

    let with_empty = tex_to_hml(r"\begin{alignedat}{2}a&=b&&c\end{alignedat}").unwrap();
    assert!(with_empty.contains(" &  & "), "{with_empty}");
    assert!(hml_to_tex(&with_empty).is_ok(), "{with_empty}");

    let expected = tex_to_hml(r"\begin{aligned}a&=b\end{aligned}").unwrap();
    for (env, argument) in [("align*", ""), ("alignat", "{1}"), ("alignat*", "{1}")] {
        let input = format!(r"\begin{{{env}}}{argument}a&=b\end{{{env}}}");
        assert_eq!(tex_to_hml(&input).unwrap(), expected, "{input}");
    }
}
