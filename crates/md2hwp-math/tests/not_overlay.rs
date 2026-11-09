// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::{hml_to_tex, tex_to_hml};

#[test]
fn native_not_overlays_one_supported_character_or_symbol() {
    for (native, tex) in [
        ("not =", r"\not="),
        ("NOT =", r"\not="),
        ("not <", r"\not<"),
        ("not >", r"\not>"),
        ("not a", r"\not a"),
        ("not 한", r"\not 한"),
        ("not 2", r"\not2"),
        ("not ab", r"\not a b"),
        ("not abc^2", r"\not a bc^{2}"),
        ("not IN", r"\not\in"),
        ("not alpha", r"\not\alpha"),
        ("not <=", r"\not\leq"),
        ("not GEQ", r"\not\geq"),
        ("a not = b", r"a \not= b"),
        ("{not =}^2", r"\not=^{2}"),
        ("not =^2", r"\not=^{2}"),
        ("rm not x", r"\mathrm{\not x}"),
        ("rm not ab", r"\mathrm{\not a} \mathrm{b}"),
        ("not 12", r"\not1 2"),
        ("not 12 over c", r"\not1 \frac{2}{c}"),
        ("not ab over c", r"\not a \frac{b}{c}"),
        ("{not ab} over c", r"\frac{\not a b}{c}"),
        ("not LEQ", r"\not\leq"),
        ("bold not a", r"\boldsymbol{\not a}"),
        ("not LCEIL", r"\not\lceil"),
        ("not RFLOOR", r"\not\rfloor"),
    ] {
        assert_eq!(hml_to_tex(native).unwrap(), tex, "{native}");
    }
}

#[test]
fn tex_not_consumes_one_symbol_and_keeps_scripts_and_neighbors() {
    for (tex, native) in [
        (r"\not=", "not ="),
        (r"\not =", "not ="),
        (r"\not{=}", "not ="),
        (r"\not a", "not a"),
        (r"\not 한", "not 한"),
        (r"\not12", "not 1 2"),
        (r"\not\alpha", "not alpha"),
        (r"\not\in", "not IN"),
        (r"\not a nd", "not a nd"),
        (r"\mathrm{\not x}", "{rm {not x} it}"),
        (r"\not\leq", "not <="),
        (r"\not=^2", "not = ^{2}"),
        (r"\not ab", "not a b"),
        (r"a\not=b", "a not = b"),
        (r"\not\lceil", "not LCEIL"),
        (r"\not\rceil", "not RCEIL"),
        (r"\not\lfloor", "not LFLOOR"),
        (r"\not\rfloor", "not RFLOOR"),
    ] {
        assert_eq!(tex_to_hml(tex).unwrap(), native, "{tex}");
        let roundtrip = hml_to_tex(native).unwrap();
        assert!(roundtrip.contains(r"\not"), "{tex}: {roundtrip}");
    }
}

#[test]
fn not_preserves_quoted_literals_and_existing_negated_symbol_mappings() {
    assert_eq!(tex_to_hml("not").unwrap(), "\"not\"");
    assert_eq!(tex_to_hml("n o t").unwrap(), "\"not\"");
    assert_eq!(tex_to_hml("NOT").unwrap(), "\"NOT\"");
    assert_eq!(hml_to_tex("\"not\"").unwrap(), "not");
    assert_eq!(hml_to_tex("notin").unwrap(), r"\notin");
    assert_eq!(hml_to_tex("!=").unwrap(), r"\neq");
    assert_eq!(tex_to_hml(r"\neq\notin").unwrap(), "NEQ notin");
}

#[test]
fn compound_or_missing_not_operands_fail_explicitly() {
    for native in [
        "not",
        "NOT",
        "not }",
        "not ^2",
        "not {=",
        "not {}",
        "not {a+b}",
        "not {a}",
        "not rm a",
        "not sqrt x",
        "not matrix {a}",
        "not sin",
        "not not =",
        "Not =",
        "not \"a\"",
        "not α",
        "not é",
    ] {
        assert!(hml_to_tex(native).is_err(), "accepted {native}");
    }
    for tex in [
        r"\not",
        r"\not{}",
        r"\not{a+b}",
        r"\not\frac12",
        r"\not\sqrt{x}",
        r"\not\sin",
        r"\not\not=",
        r"\not^2",
        r"\not α",
    ] {
        assert!(tex_to_hml(tex).is_err(), "accepted {tex}");
    }
    assert_eq!(tex_to_hml(r"x+\not\sqrt{x}").unwrap_err().offset, 6);
}

#[test]
fn not_retains_shared_parser_resource_limits() {
    assert!(hml_to_tex(&"not a ".repeat(16_385)).is_err());
    assert!(tex_to_hml(&r"\not a ".repeat(16_385)).is_err());
}

#[test]
fn standard_ascii_scalar_expansions_keep_postfix_and_not_scope() {
    for scalar in [
        "OHM",
        "varupsilon",
        "DEG",
        "LAPLACE",
        "CENTIGRADE",
        "FAHRENHEIT",
        "ANGSTROM",
        "att",
        "hund",
        "thou",
        "IDENTICAL",
    ] {
        for native in [
            format!("{scalar}^2"),
            format!("{scalar}_i"),
            format!("not {scalar}"),
            format!("not {scalar}^2"),
        ] {
            let tex = hml_to_tex(&native).unwrap();
            let inverse = tex_to_hml(&tex).unwrap_or_else(|e| panic!("{native}: {tex}: {e}"));
            assert_eq!(hml_to_tex(&inverse).unwrap(), tex, "{native}: {inverse}");
            assert!(inverse.contains(scalar), "{native}: {inverse}");
        }
    }
    assert_eq!(hml_to_tex("DEG^2").unwrap(), r"{{}^{\circ}}^{2}");
    assert_eq!(
        hml_to_tex("CENTIGRADE^2").unwrap(),
        r"{{}^{\circ}\mathrm{C}}^{2}"
    );
    assert_eq!(tex_to_hml(r"\not{{}^{\circ}}").unwrap(), "not DEG");
    for tex in [
        r"\not{\mathcal{L}x}",
        r"\not{\text{\AA}x}",
        r"\not{{}^{\circ}x}",
        r"\not{{}^{\circ}\mathrm{CF}}",
        r"\not{{}^{\circ}\mathrm{C}^2}",
        r"\not{\text{DEG}}",
    ] {
        assert!(tex_to_hml(tex).is_err(), "accepted {tex}");
    }
}
