// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::{hml_to_tex, tex_to_hml};

#[test]
fn starred_operator_names_are_generic_limit_operators() {
    for (tex, expected) in [
        (
            r"\operatorname*{foo}_{i}^{n}",
            "underover {rm \"foo\" it} _{i} ^{n}",
        ),
        (r"\operatorname*{Lim}_{x\to0}", "Lim _{x -> 0}"),
        (
            r"\operatorname*{lowerlim}_{a}^{b}",
            "underover {rm \"lowerlim\" it} _{a} ^{b}",
        ),
        (r"\operatorname*{log10}", "underover {rm \"log10\" it}"),
        (r"\operatorname*{λ}_{k}", "underover {rm \"λ\" it} _{k}"),
        (r"\operatorname*{FROWN}", "underover {rm \"FROWN\" it}"),
    ] {
        assert_eq!(
            tex_to_hml(&format!(r"\displaystyle {tex}")).unwrap(),
            expected,
            "TeX {tex:?}"
        );
    }
    assert_eq!(tex_to_hml(r"\operatorname*{Lim}").unwrap(), "Lim");
}

#[test]
fn unstarred_operator_names_remain_ordinary_text_with_scripts() {
    assert_eq!(
        tex_to_hml(r"\operatorname{foo}_{i}^{n}").unwrap(),
        "{rm \"foo\" it} _{i} ^{n}"
    );
    assert_eq!(
        tex_to_hml(r"\operatorname{Lim}").unwrap(),
        "{rm \"Lim\" it}"
    );
    assert_eq!(tex_to_hml(r"\text{FROWN}").unwrap(), "{rm \"FROWN\" it}");
    assert_eq!(
        tex_to_hml(r"\overset{FROWN}{a}").unwrap(),
        "{underover {a}^{\"FROWN\"}}"
    );
}

#[test]
fn native_text_operators_export_and_roundtrip_as_operatorname_star() {
    for name in ["foo", "lowerlim", "log10", "λ", "FROWN"] {
        let hml = format!("underover {{rm \"{name}\" it}} _{{i}} ^{{n}}");
        let expected_tex = format!(r"\operatorname*{{{name}}}\limits_{{i}}^{{n}}");
        let tex = hml_to_tex(&hml).unwrap();
        assert_eq!(tex, expected_tex, "HML {hml:?}");
        assert_eq!(
            tex_to_hml(&tex).unwrap(),
            format!("underover {{rm \"{name}\" it}} _{{i}} ^{{n}}"),
            "TeX {tex:?}"
        );
    }
}

#[test]
fn only_the_exact_starred_lim_spelling_collapses_to_native_lim() {
    assert_eq!(tex_to_hml(r"\operatorname*{Lim}").unwrap(), "Lim");
    assert_eq!(
        tex_to_hml(r"\operatorname{Lim}").unwrap(),
        "{rm \"Lim\" it}"
    );
    assert_eq!(tex_to_hml(r"\text{Lim}").unwrap(), "{rm \"Lim\" it}");

    for (tex, expected) in [
        (r"\operatorname*{lim}", "underover {rm \"lim\" it}"),
        (r"\operatorname*{LIM}", "underover {rm \"LIM\" it}"),
        (r"\operatorname*{Limx}", "underover {rm \"Limx\" it}"),
    ] {
        assert_eq!(tex_to_hml(tex).unwrap(), expected, "TeX {tex:?}");
    }

    for tex in [
        r"\mathrm{\operatorname*{Lim}}",
        r"\mathbf{\operatorname*{Lim}}",
    ] {
        let hml = tex_to_hml(tex).unwrap();
        assert_ne!(hml, "Lim", "styled TeX {tex:?}");
        assert!(hml.contains("underover"), "styled TeX {tex:?}: {hml}");
    }

    assert_eq!(hml_to_tex("Lim").unwrap(), "\\operatorname*{Lim}");
    assert_eq!(tex_to_hml(&hml_to_tex("Lim").unwrap()).unwrap(), "Lim");

    let generic_lim = "underover {rm \"Lim\" it}";
    let tex = hml_to_tex(generic_lim).unwrap();
    assert_eq!(tex, r"\operatorname*{Lim}");
    assert_eq!(tex_to_hml(&tex).unwrap(), "Lim");
}

#[test]
fn only_a_plain_upright_name_becomes_operatorname_star() {
    assert_eq!(
        hml_to_tex(r#"underover {rm "foo" it + x}"#).unwrap(),
        "\\mathop{\\mathrm{foo} + x}\\limits"
    );
    assert_eq!(
        hml_to_tex(r#"underover {{rm "foo" it}_{a}}"#).unwrap(),
        "\\mathop{\\mathrm{foo}_{a}}\\limits"
    );
    assert_eq!(
        hml_to_tex(r#"underover {{rm "foo" it}}"#).unwrap(),
        "\\operatorname*{foo}"
    );
}

#[test]
fn starred_operator_names_reject_empty_or_unsupported_literal_contents() {
    for tex in [
        r"\operatorname*{}",
        r"\operatorname*{   }",
        r"\operatorname*{foo{bar}}",
        r#"\operatorname*{foo"bar}"#,
        r#"\operatorname*{foo\bar}"#,
        r"\operatorname*{foo$bar}",
    ] {
        assert!(tex_to_hml(tex).is_err(), "unexpected success: {tex:?}");
    }

    let control = "\u{0007}";
    let tex = format!(r"\operatorname*{{foo{control}bar}}");
    assert!(tex_to_hml(&tex).is_err(), "unexpected success: {tex:?}");
}
