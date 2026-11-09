// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::hml_to_tex;

#[test]
fn ordinary_delimiters_can_be_unmatched() {
    for script in ["3 TIMES )", "( x", "[ x )", "( [ x ) ]", "x ]"] {
        hml_to_tex(script).unwrap_or_else(|error| panic!("{script}: {error}"));
    }
}

#[test]
fn ordinary_pairs_retain_binomials_and_native_pairs_remain_strict() {
    let tex = hml_to_tex("( a atop b )").unwrap();
    assert!(tex.contains("\\binom{a}{b}"), "{tex}");
    for script in ["LEFT ( x", "LEFT ( x RIGHT", "x RIGHT )"] {
        assert!(hml_to_tex(script).is_err(), "{script}");
    }
}
