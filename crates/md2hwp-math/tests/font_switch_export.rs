// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::hml_to_tex;

#[test]
fn native_font_switches_persist_across_braces() {
    assert_eq!(hml_to_tex("rm {a it b} c").unwrap(), "{\\mathrm{a} b} c");
    assert_eq!(
        hml_to_tex("{rm a} b it c").unwrap(),
        "\\mathrm{a} \\mathrm{b} c"
    );
    assert_eq!(hml_to_tex("rm ABC it d").unwrap(), "\\mathrm{ABC} d");
    assert_eq!(hml_to_tex("rm a b").unwrap(), "\\mathrm{a} \\mathrm{b}");
    assert_eq!(hml_to_tex("a rm").unwrap(), "a");
}

#[test]
fn native_font_switches_apply_in_scripts_and_other_containers() {
    assert_eq!(
        hml_to_tex("a_{rm b} c it d").unwrap(),
        "a_{\\mathrm{b}} \\mathrm{c} d"
    );
    assert_eq!(hml_to_tex("rm a it _{b} c").unwrap(), "\\mathrm{a}_{b} c");
    assert_eq!(
        hml_to_tex("matrix {rm a & b # it c & d}").unwrap(),
        "\\begin{matrix}\\mathrm{a}&\\mathrm{b}\\\\c&d\\end{matrix}"
    );
    assert_eq!(
        hml_to_tex("sqrt {rm a} b it c").unwrap(),
        "\\sqrt{\\mathrm{a}} \\mathrm{b} c"
    );
}

#[test]
fn bold_applies_to_one_primary_while_shape_persists() {
    assert_eq!(
        hml_to_tex("bold {a rm {b it c} d} e").unwrap(),
        "{\\boldsymbol{a} {\\mathbf{b} \\boldsymbol{c}} \\boldsymbol{d}} e"
    );
    assert_eq!(hml_to_tex("bold a b it c").unwrap(), "\\boldsymbol{a} b c");
    assert_eq!(hml_to_tex("bold {a} it b").unwrap(), "\\boldsymbol{a} b");
    assert_eq!(
        hml_to_tex(r#"bold "foo" rm "bar" it c"#).unwrap(),
        "\\boldsymbol{foo} \\mathrm{bar} c"
    );
    assert_eq!(
        hml_to_tex(r#"underover {bold "foo" it}"#).unwrap(),
        "\\mathop{\\boldsymbol{foo}}\\limits"
    );
}

#[test]
fn paired_roman_quote_is_a_general_operator_name() {
    for name in ["Lim", "foo", "FROWN", "λ", "log10"] {
        assert_eq!(
            hml_to_tex(&format!("underover {{rm \"{name}\" it}} _{{a}} c")).unwrap(),
            format!(r"\operatorname*{{{name}}}\limits_{{a}} c")
        );
    }
    // Old native equations may leave Roman active through the end.
    assert_eq!(
        hml_to_tex(r#"underover {rm "foo"} _{a} c"#).unwrap(),
        "\\operatorname*{foo}\\limits_{\\mathrm{a}} \\mathrm{c}"
    );
}

#[test]
fn mixed_operator_bodies_and_inner_scripts_keep_all_content() {
    assert_eq!(
        hml_to_tex(r#"underover {rm "foo" + x it} _{a} c"#).unwrap(),
        "\\mathop{\\mathrm{foo} + \\mathrm{x}}\\limits_{a} c"
    );
    assert_eq!(
        hml_to_tex(r#"underover {rm "foo" _{a} it} c"#).unwrap(),
        "\\mathop{\\mathrm{foo}_{\\mathrm{a}}}\\limits c"
    );
}

#[test]
fn quotation_keeps_literal_content_and_obeys_the_current_font() {
    assert_eq!(
        hml_to_tex(r#""foo" rm "foo bar" it "rm""#).unwrap(),
        "foo \\mathrm{foo\\ bar} rm"
    );
    assert_eq!(hml_to_tex(r#""over" "it" "rm""#).unwrap(), "over it rm");
    assert_eq!(
        hml_to_tex(r#"LEFT ( "RIGHT" RIGHT )"#).unwrap(),
        "\\left(RIGHT\\right)"
    );
}

#[test]
fn native_shape_and_weight_transitions_are_independent() {
    for (script, expected) in [
        // Exact inputs grouped by native rendered-pixel evidence in
        // artifacts/math-font-combinations/native/gallery.json.
        ("abc", "abc"),
        ("bold it abc", "abc"),
        ("rm abc", r"\mathrm{abc}"),
        ("rm bold rm abc", r"\mathrm{abc}"),
        ("bold abc", r"\boldsymbol{abc}"),
        ("rm bold abc", r"\mathbf{abc}"),
        ("it bold abc", r"\boldsymbol{abc}"),
        ("rm it bold abc", r"\boldsymbol{abc}"),
        ("bold bold abc", r"\boldsymbol{abc}"),
        ("bold rm bold abc", r"\mathbf{abc}"),
        ("rm bold it abc", "abc"),
        ("it bold rm abc", r"\mathrm{abc}"),
    ] {
        assert_eq!(hml_to_tex(script).unwrap(), expected, "{script}");
    }
}

#[test]
fn quoted_names_keep_both_font_axes_and_do_not_become_plain_operators() {
    for (script, expected) in [
        (r#"rm bold "abc""#, r"\mathbf{abc}"),
        (r#"bold "abc""#, r"\boldsymbol{abc}"),
        (r#"rm bold it "abc""#, "abc"),
        (r#"it bold rm "abc""#, r"\mathrm{abc}"),
        (
            r#"underover {rm bold "Lim" it}"#,
            r"\mathop{\mathbf{Lim}}\limits",
        ),
        (
            r#"underover {bold "Lim" it}"#,
            r"\mathop{\boldsymbol{Lim}}\limits",
        ),
    ] {
        assert_eq!(hml_to_tex(script).unwrap(), expected, "{script}");
    }
}

#[test]
fn quoted_words_at_and_beyond_nine_characters_keep_all_letters() {
    use md2hwp_math::tex_to_hml;
    for word in ["abcdefghi", "abcdefghij", "abcdefghijk"] {
        let tex = hml_to_tex(&format!("\"{word}\"")).unwrap();
        assert_eq!(tex, word);
        let expected = if word.len() > 9 {
            format!("\"{word}\"")
        } else {
            word.into()
        };
        assert_eq!(tex_to_hml(&tex).unwrap(), expected);
    }
}

#[test]
fn bold_roman_applies_to_one_primary_and_preserves_shape() {
    assert_eq!(
        hml_to_tex("{rm bold a} bold b it c").unwrap(),
        "\\mathbf{a} \\mathbf{b} c"
    );
    assert_eq!(
        hml_to_tex("rm bold a_{b} c it d").unwrap(),
        "\\mathbf{a}_{\\mathrm{b}} \\mathrm{c} d"
    );
    assert_eq!(
        hml_to_tex("rm bold matrix{a&b#c&d} it e").unwrap(),
        "\\begin{matrix}\\mathbf{a}&\\mathbf{b}\\\\\\mathbf{c}&\\mathbf{d}\\end{matrix} e"
    );
}

#[test]
fn combined_bold_roman_commands_apply_to_one_primary() {
    for command in ["boldrm", "rmbold"] {
        for (suffix, expected) in [
            ("a b", r"\mathbf{a} \mathrm{b}"),
            ("a rm b", r"\mathbf{a} \mathrm{b}"),
            ("a it b", r"\mathbf{a} b"),
            ("a bold b", r"\mathbf{a} \mathbf{b}"),
            ("a_{b} c it d", r"\mathbf{a}_{\mathrm{b}} \mathrm{c} d"),
        ] {
            let script = format!("{command} {suffix}");
            assert_eq!(hml_to_tex(&script).unwrap(), expected, "{script}");
        }
        assert_eq!(
            hml_to_tex(&format!("{{{command} a}} b it c")).unwrap(),
            r"\mathbf{a} \mathrm{b} c"
        );
        assert_eq!(
            hml_to_tex(&format!("{command} a {{rm b}} c")).unwrap(),
            r"\mathbf{a} \mathrm{b} \mathrm{c}"
        );
    }
}

#[test]
fn bold_binding_matches_native_primary_and_group_scope() {
    for (script, expected) in [
        ("bold a b", r"\boldsymbol{a} b"),
        ("bold abc d", r"\boldsymbol{abc} d"),
        ("bold {a b} c", r"{\boldsymbol{a} \boldsymbol{b}} c"),
        ("bold {rm a} b", r"\mathbf{a} \mathrm{b}"),
        ("bold {a rm b} c", r"{\boldsymbol{a} \mathbf{b}} \mathrm{c}"),
        ("bold a {rm b} c", r"\boldsymbol{a} \mathrm{b} \mathrm{c}"),
        ("bold rm a", r"\mathrm{a}"),
        ("rm bold a", r"\mathbf{a}"),
        ("bold a_b^c d", r"\boldsymbol{a}_{b}^{c} d"),
        (
            "bold {a_b^c} d",
            r"{\boldsymbol{a}_{\boldsymbol{b}}^{\boldsymbol{c}}} d",
        ),
        ("bold a over b", r"\frac{\boldsymbol{a}}{b}"),
        ("bold {a over b}", r"\frac{\boldsymbol{a}}{\boldsymbol{b}}"),
    ] {
        assert_eq!(hml_to_tex(script).unwrap(), expected, "{script}");
    }
}
