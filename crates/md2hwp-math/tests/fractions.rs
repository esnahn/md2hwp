// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::tex_to_hml;

#[test]
fn fraction_commands_normalize_without_losing_operands_or_scripts() {
    for command in ["frac", "dfrac", "tfrac", "cfrac"] {
        assert_eq!(
            tex_to_hml(&format!(r"\{command}{{a_b+1}}{{x^2}}_k")).unwrap(),
            "{{a _{b} + 1} over {x ^{2}}} _{k}",
            "{command}"
        );
        assert_eq!(
            tex_to_hml(&format!(r"\{command}12")).unwrap(),
            "{{1} over {2}}"
        );
    }
    assert_eq!(
        tex_to_hml(r"\cfrac{1}{1+\dfrac{x}{\tfrac{y}{z}}}").unwrap(),
        "{{1} over {1 + {{x} over {{{y} over {z}}}}}}"
    );
    assert_eq!(
        tex_to_hml(r"\frac{{a+b}}{\frac{}{x}}").unwrap(),
        "{{{a + b}} over {{{} over {x}}}}"
    );
}

#[test]
fn continued_fractions_preserve_font_scopes() {
    assert_eq!(
        tex_to_hml(r"\mathbf{\cfrac{\text{a}}{\mathrm{x}}}\text{z}").unwrap(),
        "{rm bold {{{{rm \"a\" rm bold}} over {{rm {x} rm bold}}}} it} {rm \"z\" it}"
    );
}

#[test]
fn optional_continued_fraction_alignment_is_accepted_and_ignored() {
    for option in ["[l]", "[r]", "[]", "[ l ]"] {
        let input = format!(r"\cfrac{option}{{a_b+1}}{{x^2}}_k");
        assert_eq!(
            tex_to_hml(&input).unwrap(),
            "{{a _{b} + 1} over {x ^{2}}} _{k}"
        );
    }
    assert_eq!(
        tex_to_hml(r"\cfrac[l]{1}{1+\cfrac[r]{a}{b}}").unwrap(),
        tex_to_hml(r"\cfrac{1}{1+\cfrac{a}{b}}").unwrap()
    );
}

#[test]
fn malformed_continued_fraction_alignment_is_rejected() {
    for input in [
        r"\cfrac[c]{a}{b}",
        r"\cfrac[lr]{a}{b}",
        r"\cfrac[\text{l}]{a}{b}",
        r"\cfrac[l{a}{b}",
    ] {
        assert!(tex_to_hml(input).unwrap_err().message.contains("alignment"));
    }
}

#[test]
fn malformed_fraction_arguments_and_unsupported_forms_are_rejected() {
    for input in [
        r"\frac",
        r"\dfrac{a}",
        r"\tfrac{a}{",
        r"\cfrac{a}",
        r"\cfrac{a}{b}^",
        r"\cfrac{a}{b}}",
    ] {
        assert!(tex_to_hml(input).is_err(), "accepted {input}");
    }
    assert!(tex_to_hml(r#"\text{\cfrac{a}{b}}"#).is_err());
    assert!(tex_to_hml(r#""\cfrac{a}{b}""#).is_err());
}

#[test]
fn continued_fractions_retain_resource_limits() {
    let input = format!("{}x{}", r"\cfrac{1}{".repeat(120), "}".repeat(120));
    assert!(
        tex_to_hml(&input)
            .unwrap_err()
            .message
            .contains("nesting limit")
    );
    let input = format!(r"\cfrac{{{}}}{{b}}", "a+".repeat(17_000));
    assert!(
        tex_to_hml(&input)
            .unwrap_err()
            .message
            .contains("atom limit")
    );
}
