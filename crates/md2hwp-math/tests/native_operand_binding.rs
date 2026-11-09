// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::hml_to_tex;

#[test]
fn unbraced_fractions_bind_contiguous_lexical_operands() {
    for (script, expected) in [
        ("10a^3 over b^2", r"\frac{10 a^{3}}{b^{2}}"),
        ("10 a^3 over b^2", r"10 \frac{a^{3}}{b^{2}}"),
        ("b^3 over 2a", r"\frac{b^{3}}{2 a}"),
        ("b^3 over 2 a", r"\frac{b^{3}}{2} a"),
        ("ab over cd", r"\frac{ab}{cd}"),
        ("a b over c d", r"a \frac{b}{c} d"),
        ("a+b over c+d", r"\frac{a + b}{c + d}"),
        ("a over b/c", r"\frac{a}{b / c}"),
        ("x+{a} over {b}+y", r"x + \frac{a}{b} + y"),
        ("2a^2 over b", r"\frac{2 a^{2}}{b}"),
    ] {
        assert_eq!(hml_to_tex(script).unwrap(), expected, "{script}");
    }
}

#[test]
fn unbraced_scripts_bind_until_whitespace_sibling_script_or_container_end() {
    for (script, expected) in [
        ("lim_N->inf", r"\lim\limits_{N \to \infty}"),
        ("sum_n=1^N", r"\sum\limits_{n = 1}^{N}"),
        ("sum_n =1^N", r"\sum\limits_{n} = 1^{N}"),
        ("x_n+1 y", r"x_{n + 1} y"),
        ("x^n+1 y", r"x^{n + 1} y"),
        ("x_n +1", r"x_{n} + 1"),
        ("x^n +1", r"x^{n} + 1"),
        ("x^2a", r"x^{2 a}"),
        ("x^{2}a", r"x^{2} a"),
        ("x^C`=y", r"x^{C} \, = y"),
        ("x^C~=y", r"x^{C} \  = y"),
        ("`^3sqrt{x^2 +1}dx", r"\,^{3} \sqrt{x^{2} + 1} dx"),
        (
            "(SUM_k=1^n 1 over 2^k)",
            r"(\sum\limits_{k = 1}^{n} \frac{1}{2^{k}})",
        ),
    ] {
        assert_eq!(hml_to_tex(script).unwrap(), expected, "{script}");
    }
}

#[test]
fn printed_nested_limit_example_keeps_each_unbraced_index_and_exponent() {
    assert_eq!(
        hml_to_tex("lim_N->inf 1 over N sum_n=1^N LEFT(SUM_k=1^n 1 over 2^k right)").unwrap(),
        r"\lim\limits_{N \to \infty} \frac{1}{N} \sum\limits_{n = 1}^{N} \left(\sum\limits_{k = 1}^{n} \frac{1}{2^{k}}\right)"
    );
}

#[test]
fn italic_quoted_spaces_do_not_need_a_math_alphabet() {
    assert_eq!(hml_to_tex(r#"" ""#).unwrap(), r"\ ");
    assert_eq!(hml_to_tex(r#""  ""#).unwrap(), r"\ \ ");
    assert_eq!(hml_to_tex(r#"rm " ""#).unwrap(), r"\mathrm{\ }");
    assert_eq!(hml_to_tex(r#"bold " ""#).unwrap(), r"\boldsymbol{\ }");
    assert_eq!(hml_to_tex(r#""a b""#).unwrap(), r"\mathit{a\ b}");
}
