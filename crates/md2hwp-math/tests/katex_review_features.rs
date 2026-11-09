// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::{hml_to_tex, tex_to_hml};

#[test]
fn colors_follow_group_scope_and_keep_explicit_nested_colors() {
    assert_eq!(
        tex_to_hml(r"a{b\color{red}cd}e").unwrap(),
        "a {b {color {255, 0, 0} {c}} {color {255, 0, 0} {d}}} e"
    );
    let native = tex_to_hml(r"\blue{a}\textcolor{#0f0}{b}\textcolor{red}{c}").unwrap();
    assert!(native.contains("color {100, 149, 237} {a}"));
    assert!(native.contains("color {0, 255, 0} {b}"));
    assert!(hml_to_tex(&native).is_ok());
    let native = tex_to_hml(r"{\color{red}a\textcolor{green}{b}c\color{blue}d}e").unwrap();
    assert!(native.contains("color {0, 128, 0} {b}"));
    assert!(native.contains("color {0, 0, 255} {d}"));
    assert!(native.ends_with("} e"));
    let native = tex_to_hml(r"{a\color{red}b\over c}").unwrap();
    assert!(
        native.contains("{a {color {255, 0, 0} {b}}} over {{color {255, 0, 0} {c}}}"),
        "{native}"
    );
    for tex in [
        r"\color{unknown}x",
        r"\color[RGB]{256,0,0}x",
        r"\textcolor{#zzf}{x}",
    ] {
        assert!(tex_to_hml(tex).is_err(), "{tex}");
    }
}

#[test]
fn background_color_is_omitted_but_frame_is_retained() {
    assert_eq!(
        tex_to_hml(r"\colorbox{teal}B").unwrap(),
        "{box {{rm {B} it}}}"
    );
    assert_eq!(
        tex_to_hml(r"\fcolorbox{blue}{red}{C}").unwrap(),
        "{box {{rm {C} it}}}"
    );
}

#[test]
fn literal_star_is_not_native_times_and_font_commands_are_scoped() {
    assert_eq!(tex_to_hml("a*b").unwrap(), "a \"*\" b");
    assert_eq!(tex_to_hml(r"\mathbf{a}").unwrap(), "{rm bold {a} it}");
    assert!(
        tex_to_hml(r"\text{\it abc}")
            .unwrap()
            .contains("it \"abc\"")
    );
}

#[test]
fn equation_split_and_gather_preserve_math_without_numbering() {
    for tex in [
        r"\begin{equation}\begin{split}a&=b\\&=c\end{split}\end{equation}",
        r"\begin{gather}a=b\tag{1}\\c=d\label{eq:x}\end{gather}",
    ] {
        let native = tex_to_hml(tex).unwrap();
        assert!(!native.contains("tag"));
        assert!(hml_to_tex(&native).is_ok(), "{native}");
    }
    for tex in [
        r"\begin{CD}a\end{CD}",
        r"\newcommand{\x}{a}\x",
        r"\startExp a\endExp",
    ] {
        assert!(tex_to_hml(tex).is_err());
    }
}

#[test]
fn fixed_sizes_restore_scope_and_infix_brackets_keep_stacked_operands() {
    assert_eq!(tex_to_hml(r"a\bigl.b").unwrap(), "a b");
    for delimiter in ["(", ")", "[", "]", "|"] {
        let native = tex_to_hml(&format!(r"\big{delimiter}")).unwrap();
        assert!(hml_to_tex(&native).is_ok(), "{native}");
    }
    assert_eq!(
        tex_to_hml(r"{\Huge x}y").unwrap(),
        "{scale 249 x scale 100} y"
    );
    assert_eq!(
        tex_to_hml(r"\bigl\uparrow").unwrap(),
        "{scale 120 uparrow scale 100}"
    );
    for tex in [r"{a\brace b}", r"{a\brack b}"] {
        let native = tex_to_hml(tex).unwrap();
        assert!(native.contains("atop"));
        assert!(hml_to_tex(&native).is_ok(), "{native}");
    }
}
