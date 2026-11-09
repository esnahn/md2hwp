// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::{hml_to_tex, tex_to_hml};

#[test]
fn over_scopes_groups_cells_and_annotation_arguments() {
    for (tex, expected) in [
        (r"1\over2", r"\frac{1}{2}"),
        (r"a+b\over c+d", r"\frac{a + b}{c + d}"),
        (r"x+{a\over b}+y", r"x + \frac{a}{b} + y"),
        (
            r"\begin{matrix}a\over b&c\end{matrix}",
            r"\begin{matrix}\frac{a}{b}&c\end{matrix}",
        ),
    ] {
        assert_eq!(hml_to_tex(&tex_to_hml(tex).unwrap()).unwrap(), expected);
    }
    assert!(tex_to_hml(r"\overset{a}{b\over c}").is_ok());
    for tex in [
        r"\over b",
        r"a\over",
        r"a\over b\over c",
        r"a\atop b\over c",
        r"a\over b\atop c",
        r"a\overlinebad b",
    ] {
        assert!(tex_to_hml(tex).is_err(), "{tex}");
    }
}

#[test]
fn brace_and_brack_work_in_annotation_groups() {
    for (tex, expected) in [
        (
            r"\overset{x}{a\brace b}",
            "{underover {{LEFT { {{a} atop {b}} RIGHT }}}^{x}}",
        ),
        (
            r"\overset{a\brack b}{x}",
            "{underover {x}^{{LEFT [ {{a} atop {b}} RIGHT ]}}}",
        ),
        (
            r"\overset{x}{{a\brace b}\brack c}",
            "{underover {{LEFT [ {{{{LEFT { {{a} atop {b}} RIGHT }}}} atop {c}} RIGHT ]}}^{x}}",
        ),
    ] {
        assert_eq!(tex_to_hml(tex).unwrap(), expected, "{tex}");
    }
    for tex in [
        r"\overset{x}{\brace b}",
        r"\overset{a\brack}{x}",
        r"\overset{x}{a\brace b\over c}",
        r"\overset{a\atop b\brack c}{x}",
    ] {
        assert!(tex_to_hml(tex).is_err(), "{tex}");
    }
}
