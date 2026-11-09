// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::tex_to_hml;

#[test]
fn ordinary_math_group_breaks_do_not_make_vertical_rows() {
    assert_eq!(
        tex_to_hml(r"{d+\\e}").unwrap(),
        tex_to_hml("{d+e}").unwrap()
    );
    assert_eq!(
        tex_to_hml(r"{d+\newline e}").unwrap(),
        tex_to_hml("{d+e}").unwrap()
    );
    assert_eq!(tex_to_hml(r"{a\quad\\b}").unwrap(), "{a b}");
    assert!(tex_to_hml(r"{a\kern1em\\b}").unwrap().contains('~'));
    assert!(tex_to_hml(r"a\\b").unwrap().contains("pile"));
    assert!(
        tex_to_hml(r"\begin{gathered}a\\b\end{gathered}")
            .unwrap()
            .contains('#')
    );
    assert!(
        tex_to_hml(r"\begin{matrix}a\\b\end{matrix}")
            .unwrap()
            .contains('#')
    );
    for tex in [r"{a\\[1em]b}", r"{a\\*b}"] {
        assert!(tex_to_hml(tex).is_err(), "{tex}");
    }
}

#[test]
fn ignored_grouped_break_does_not_cut_fraction_style_lookahead() {
    let a = tex_to_hml(r"{\mathchoice{D}{T}{S}{Q}\\a\over b}").unwrap();
    let b = tex_to_hml(r"{\mathchoice{D}{T}{S}{Q}a\over b}").unwrap();
    assert_eq!(a, b);
}

#[test]
fn poor_mans_bold_normalizes_to_regular_math_bold() {
    for body in [r"\mu", "=", "+", "a+b"] {
        assert_eq!(
            tex_to_hml(&format!(r"\pmb{{{body}}}")).unwrap(),
            tex_to_hml(&format!(r"\boldsymbol{{{body}}}")).unwrap()
        );
    }
}
