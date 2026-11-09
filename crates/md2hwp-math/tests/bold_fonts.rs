// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::tex_to_hml;

#[test]
fn math_alphabets_select_shape_and_weight_and_restore_both() {
    for (tex, native) in [
        (r"\mathbf{abc}z", "{rm bold {abc} it} z"),
        (r"\boldsymbol{abc}z", "{it bold {abc} it} z"),
        (
            r"\mathrm{\boldsymbol{abc}d}z",
            "{rm {{rm bold {abc} rm} d} it} z",
        ),
        (
            r"\mathbf{a\mathit{b}c}z",
            "{rm bold {a {it {b} rm bold} c} it} z",
        ),
        (
            r"\boldsymbol{a\mathrm{b}c}z",
            "{it bold {a {rm bold {b} it bold} c} it} z",
        ),
        (r"\boldsymbol{a\mathit{b}c}z", "{it bold {a {b} c} it} z"),
        (
            r"\boldsymbol{\mathbf{a\mathit{b}c}d}z",
            "{it bold {{rm bold {a {it bold {b} rm bold} c} it bold} d} it} z",
        ),
        (r"\mathbf{\boldsymbol{a}b}z", "{rm bold {{a} b} it} z"),
        (
            r"\mathbf{a\text{B}c}z",
            "{rm bold {a {rm \"B\" rm bold} c} it} z",
        ),
    ] {
        assert_eq!(tex_to_hml(tex).unwrap(), native, "{tex}");
    }
}

#[test]
fn bold_operator_names_retain_the_complete_font_restore() {
    assert_eq!(
        tex_to_hml(r"\mathbf{\operatorname*{Lim}x}z").unwrap(),
        "{rm bold {underover {rm \"Lim\" rm bold} x} it} z"
    );
    assert_eq!(
        tex_to_hml(r"\boldsymbol{\operatorname*{Lim}x}z").unwrap(),
        "{it bold {underover {rm bold \"Lim\" it bold} x} it} z"
    );
    assert_eq!(tex_to_hml(r"\operatorname*{Lim}").unwrap(), "Lim");
}
