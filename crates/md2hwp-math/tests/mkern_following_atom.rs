// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::tex_to_hml;

#[test]
fn mkern_unit_can_be_followed_immediately_by_a_math_atom() {
    assert_eq!(tex_to_hml(r"a\mkern18mub").unwrap(), "a ~~ b");
    assert_eq!(tex_to_hml(r"a\mkern2.25mufoo").unwrap(), "a ` foo");
    assert_eq!(tex_to_hml(r"a\mkern2.25mu{b}").unwrap(), "a ` {b}");
}

#[test]
fn mkern_rejects_a_numeric_dimension_suffix() {
    for tex in [r"a\mkern2.25mu2", r"a\mkern2.25mu.5"] {
        assert!(tex_to_hml(tex).is_err(), "unexpected success: {tex}");
    }
}
