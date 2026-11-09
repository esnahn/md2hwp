// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::hml_to_tex;

#[test]
fn ordinary_symbols_and_fraction_operands_have_token_boundaries() {
    assert_eq!(
        hml_to_tex("sin alpha `cos beta").unwrap(),
        "\\sin \\alpha \\, \\cos \\beta"
    );
    assert_eq!(hml_to_tex("2pi").unwrap(), "2 \\pi");
    assert_eq!(hml_to_tex("C over d").unwrap(), "\\frac{C}{d}");
}
