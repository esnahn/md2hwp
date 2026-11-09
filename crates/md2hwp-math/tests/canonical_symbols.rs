// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::{hml_to_tex, tex_to_hml};

#[test]
fn symbol_import_uses_the_selected_native_canonical_spellings() {
    for (tex, canonical) in [
        (r"\bigvee", "BIGVEE"),
        (r"\bigwedge", "BIGWEDGE"),
        ("⋁", "BIGVEE"),
        ("⋀", "BIGWEDGE"),
        (r"\mathop{\lor}\limits", "BIGVEE"),
        (r"\mathop{\wedge}\limits", "BIGWEDGE"),
        (r"\cup", "CUP"),
        ("∪", "CUP"),
        (r"\dashv", "DASHV"),
        ("⊣", "DASHV"),
        (r"\ggg", "GGG"),
        ("⋙", "GGG"),
        (r"\ni", "NI"),
        (r"\owns", "NI"),
        ("∋", "NI"),
        (r"\infty", "INFTY"),
        ("∞", "INFTY"),
    ] {
        let actual = tex_to_hml(tex).unwrap();
        assert_eq!(actual, canonical, "{tex}");
        let exported = hml_to_tex(&actual).unwrap();
        assert_eq!(tex_to_hml(&exported).unwrap(), canonical, "{tex}");
    }
}

#[test]
fn canonical_decoration_and_large_operators_keep_their_operands_and_scripts() {
    assert_eq!(
        tex_to_hml(r"\underline{a+b}").unwrap(),
        "{underline {a + b}}"
    );
    for (tex, canonical) in [
        (r"\bigvee\limits_{i=1}^{n}", "BIGVEE _{i = 1} ^{n}"),
        (r"\bigwedge\limits_{i=1}^{n}", "BIGWEDGE _{i = 1} ^{n}"),
        (r"\mathop{\lor}\limits_{i=1}^{n}", "BIGVEE _{i = 1} ^{n}"),
        (
            r"\mathop{\wedge}\limits_{i=1}^{n}",
            "BIGWEDGE _{i = 1} ^{n}",
        ),
        (r"\overset{a}{\lor}", "{BIGVEE^{a}}"),
        (r"\underset{b}{\wedge}", "{BIGWEDGE_{b}}"),
    ] {
        assert_eq!(tex_to_hml(tex).unwrap(), canonical, "{tex}");
    }
    for (input, canonical) in [
        ("underover {LOR}", "BIGVEE"),
        ("underover {WEDGE}", "BIGWEDGE"),
        ("SMALLUNION", "CUP"),
        ("HLEFT", "DASHV"),
        ("MASSERT", "DASHV"),
        (">>>", "GGG"),
        ("OWNS", "NI"),
        ("INF", "INFTY"),
        ("INFINITY", "INFTY"),
    ] {
        let tex = hml_to_tex(input).unwrap();
        assert_eq!(tex_to_hml(&tex).unwrap(), canonical, "{input}");
    }
    for tex in [r"\text{NI}", r"\text{INFTY}", r"\text{DASHV}"] {
        let native = tex_to_hml(tex).unwrap();
        assert!(native.contains('"'), "{tex}: {native}");
    }
}
