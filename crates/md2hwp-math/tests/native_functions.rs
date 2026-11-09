// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::{hml_to_tex, tex_to_hml};

#[test]
fn standard_named_operators_import_without_becoming_ordinary_letters() {
    for name in [
        "coth", "lg", "max", "min", "arcsin", "arccos", "arctan", "det", "gcd", "deg", "arg",
        "dim", "hom", "ker", "Pr",
    ] {
        assert_eq!(tex_to_hml(&format!(r"\{name}")).unwrap(), name);
        assert_eq!(hml_to_tex(name).unwrap(), format!(r"\{name}"));
    }
}

#[test]
fn already_exported_native_relation_symbols_have_matching_imports() {
    for (native, tex, canonical) in [
        ("WEDGE", "\\wedge", "WEDGE"),
        ("<<", "\\ll", "<<"),
        (">>", "\\gg", ">>"),
        (">>>", "\\ggg", "GGG"),
    ] {
        assert_eq!(hml_to_tex(native).unwrap(), tex);
        assert_eq!(tex_to_hml(tex).unwrap(), canonical);
    }
}

#[test]
fn native_named_operators_keep_their_keywords_on_roundtrip() {
    for name in ["cosec", "Exp", "arc", "mod", "if", "for", "and"] {
        let tex = format!(r"\operatorname{{{name}}}");
        assert_eq!(hml_to_tex(name).unwrap(), tex);
        assert_eq!(tex_to_hml(&tex).unwrap(), name);
        assert_eq!(hml_to_tex(&tex_to_hml(&tex).unwrap()).unwrap(), tex);
    }
}

#[test]
fn keyword_normalization_keeps_literal_limits_and_font_boundaries() {
    for name in ["cosec", "Exp", "arc", "mod", "if", "for", "and"] {
        assert_eq!(
            tex_to_hml(&format!(r"\text{{{name}}}")).unwrap(),
            format!("{{rm \"{name}\" it}}")
        );
        let starred = tex_to_hml(&format!(r"\operatorname*{{{name}}}")).unwrap();
        assert!(starred.starts_with("underover "), "{starred}");
        let styled = tex_to_hml(&format!(r"\mathbf{{\operatorname{{{name}}}}}")).unwrap();
        assert!(styled.contains(&format!("\"{name}\"")), "{styled}");
        let upper = tex_to_hml(&format!(r"\operatorname{{{}}}", name.to_uppercase())).unwrap();
        assert!(upper.contains('"'), "{upper}");
    }
}

#[test]
fn only_the_exact_plain_exp_operator_returns_to_the_native_keyword() {
    assert_eq!(hml_to_tex("Exp").unwrap(), "\\operatorname{Exp}");
    assert_eq!(
        tex_to_hml(r"\operatorname{Exp}_{a}^{b}").unwrap(),
        "Exp _{a} ^{b}"
    );
    for tex in [r"\operatorname{exp}", r"\operatorname{EXP}", r"\text{Exp}"] {
        assert_ne!(tex_to_hml(tex).unwrap(), "Exp", "{tex}");
    }
    assert_eq!(
        tex_to_hml(r"\operatorname*{Exp}").unwrap(),
        "underover {rm \"Exp\" it}"
    );
    for tex in [
        r"\mathrm{\operatorname{Exp}}",
        r"\mathbf{\operatorname{Exp}}",
        r"\boldsymbol{\operatorname{Exp}}",
    ] {
        let native = tex_to_hml(tex).unwrap();
        assert!(native.contains("\"Exp\""), "{tex}: {native}");
    }
}

#[test]
fn font_examples_restore_the_surrounding_default_after_their_scope() {
    for (native, expected) in [
        ("rm abc", "{rm {abc} it}"),
        ("it abc", "abc"),
        ("bold abc", "{it bold {abc} it}"),
    ] {
        let tex = hml_to_tex(native).unwrap();
        assert_eq!(tex_to_hml(&tex).unwrap(), expected);
    }
    // An italic scope inside Roman text returns to Roman, then to the default.
    assert_eq!(
        tex_to_hml(r"\mathrm{x \mathit{abc} y}z").unwrap(),
        "{rm {x {it {abc} rm} y} it} z"
    );
}
