// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::{hml_to_tex, tex_to_hml};

#[test]
fn plain_and_explicit_italic_words_bypass_native_function_recognition() {
    for word in ["sin", "cos", "and", "sinx", "over", "it", "rm"] {
        assert_eq!(hml_to_tex(&format!("\"{word}\"")).unwrap(), word);
        assert_eq!(tex_to_hml(word).unwrap(), format!("\"{word}\""));
        let explicit = format!(r"\mathit{{{word}}}");
        let native = tex_to_hml(&explicit).unwrap();
        assert_eq!(native, format!("{{\"{word}\"}}"));
        assert_eq!(hml_to_tex(&native).unwrap(), format!("{{{word}}}"));
    }
    assert_eq!(tex_to_hml(r"\sin x").unwrap(), r#"sin x"#);
    assert_eq!(hml_to_tex(r#""sin"^2"#).unwrap(), r"{sin}^{2}");
    assert_eq!(hml_to_tex(r#"rm "sin""#).unwrap(), r"\mathrm{sin}");
    assert_eq!(hml_to_tex(r#"bold "sin""#).unwrap(), r"\boldsymbol{sin}");
}

#[test]
fn default_literals_preserve_argument_and_script_boundaries() {
    for (tex, native) in [
        ("sin^2", r#""si" "n" ^{2}"#),
        ("sin_i", r#""si" "n" _{i}"#),
        ("{sin}^2", r#"{"sin"} ^{2}"#),
        ("s i n", r#""sin""#),
        ("si{n}", r#""si" {"n"}"#),
        (r"\frac sin", r#"{{s} over {i}} n"#),
        (r"\mathrm{sin}", r#"{rm {"sin"} it}"#),
        (r"\mathbf{sin}", r#"{rm bold {"sin"} it}"#),
        (r"\overset{x}{sin}", r#"{underover {"sin"}^{x}}"#),
    ] {
        assert_eq!(tex_to_hml(tex).unwrap(), native, "{tex}");
    }
}

#[test]
fn literal_word_joining_preserves_tex_script_attachment_and_boundaries() {
    for (tex, native) in [
        (r"\mathit{sin^2}", "{\"si\" \"n\" ^{2}}"),
        (r"\mathit{sin_i}", "{\"si\" \"n\" _{i}}"),
        (r"\mathit{sin}^2", "{\"sin\"} ^{2}"),
        (r"\mathit{si^n x}", "{s i ^{n} x}"),
        (r"\mathit{s i n}", "{\"sin\"}"),
        (r"\mathit{si{n}}", "{\"si\" {\"n\"}}"),
        (r"\mathit{sin+cos}", "{\"sin\" + \"cos\"}"),
        (r"\mathit s in", "{s} \"in\""),
        (r"\mathit{\frac sin}", "{{{s} over {i}} n}"),
    ] {
        assert_eq!(tex_to_hml(tex).unwrap(), native, "{tex}");
    }
}

#[test]
fn literal_words_join_across_whitespace_and_quote_only_beyond_nine_letters() {
    for tex in ["abc", "a b c", "a\tb\tc", "a\nb\nc", " a\r\n b\t c "] {
        assert_eq!(tex_to_hml(tex).unwrap(), r#"abc"#, "{tex:?}");
        let styled = format!(r"\mathit{{{tex}}}");
        assert_eq!(tex_to_hml(&styled).unwrap(), r#"{abc}"#, "{styled:?}");
    }
    for word in [
        "abcdefgh",
        "abcdefghi",
        "abcdefghij",
        "abcdefghijk",
        "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ",
    ] {
        let expected = if word.len() > 9 {
            format!("\"{word}\"")
        } else {
            word.into()
        };
        assert_eq!(tex_to_hml(word).unwrap(), expected);
        let spaced = word
            .chars()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        assert_eq!(tex_to_hml(&spaced).unwrap(), expected);
    }
}

#[test]
fn minimal_quotes_protect_native_commands_and_transparent_group_context() {
    for (tex, native) in [
        ("abc", "abc"),
        ("abc^2", "ab c ^{2}"),
        ("sin", r#""sin""#),
        ("sinx", r#""sinx""#),
        ("xsin", "xsin"),
        ("Sinx", "Sinx"),
        ("a si n", r#""asin""#),
        ("x si n", "xsin"),
        ("s{i}n", r#""s" {"i"} "n""#),
        ("x{si}n", "x {si} n"),
        ("s i^2 n", "s i ^{2} n"),
        (r"\sin x", "sin x"),
        ("alphax", r#""alphax""#),
        ("sumx", r#""sumx""#),
        ("intx", r#""intx""#),
        ("sqrtx", r#""sqrtx""#),
        ("rmx", r#""rmx""#),
        ("boldx", r#""boldx""#),
        ("capx", r#""capx""#),
        ("oplusx", r#""oplusx""#),
        ("varepsilonx", r#""varepsilonx""#),
        ("overlinex", r#""overlinex""#),
        ("VecA", r#""VecA""#),
        ("VecAB", r#""VecAB""#),
        ("vecab", r#""vecab""#),
        ("Veca", r#""Veca""#),
        ("VECA", r#""VECA""#),
        ("veCalpha", "veCalpha"),
        ("xalpha", "xalpha"),
        ("ABC", r#""ABC""#),
        ("abc{d}", r#""abc" {d}"#),
        ("Equation{font}", r#""Equation" {font}"#),
        (r"\frac sin", "{{s} over {i}} n"),
    ] {
        assert_eq!(tex_to_hml(tex).unwrap(), native, "{tex}");
    }
    // These quotes preserve the existing native reader's ambiguity guards.
    assert!(hml_to_tex("ABC").is_err());
    assert!(hml_to_tex("unknown {x}").is_err());
    for tex in ["ABC", "abc{d}", "Equation{font}"] {
        hml_to_tex(&tex_to_hml(tex).unwrap()).unwrap();
    }
}

#[test]
fn internal_literal_delimiters_are_rejected_before_any_tex_parsing() {
    for delimiter in ['\x01', '\x02'] {
        for tex in [
            format!("a{delimiter}b"),
            format!(r"\text{{a{delimiter}b}}"),
            format!(r"\operatorname{{a{delimiter}b}}"),
            format!(r"\begin{{matrix{delimiter}}}a\end{{matrix}}"),
        ] {
            let error = tex_to_hml(&tex).unwrap_err();
            assert_eq!(error.offset, tex.find(delimiter).unwrap());
            assert!(error.message.contains("reserved internal character"));
        }
    }
}

#[test]
fn whitespace_joining_respects_scripts_commands_groups_and_explicit_spaces() {
    for (tex, native) in [
        ("s i n ^ 2", r#""si" "n" ^{2}"#),
        ("a b_c d", r#"a b _{c} d"#),
        ("a^b c d", r#"a ^{b} cd"#),
        (r"\frac a b c d", r#"{{a} over {b}} cd"#),
        (r"\sqrt a b c", r#"{sqrt {a}} bc"#),
        (r"a {b} c", r#"a {b} c"#),
        (r"a {} b", r#"a {} b"#),
        (r"a \sin b c", r#"a sin bc"#),
        (r"a \, b c", r#"a ` bc"#),
        (r"a \quad b c", r#"a ~~ bc"#),
        (r"a\ b c", r#"a ~ bc"#),
        (r"a~b c", r#"a ~ bc"#),
    ] {
        assert_eq!(tex_to_hml(tex).unwrap(), native, "{tex:?}");
    }
}

#[test]
fn annotation_literals_share_whitespace_joining_and_boundary_rules() {
    for (tex, native) in [
        ("\\overset{a \t b\nc}{x y z}", r#"{underover {xyz}^{abc}}"#),
        (
            r"\underset{abcdefghijk}{a b c}",
            r#"{underover {abc}_{"abcdefghijk"}}"#,
        ),
        (
            r"\overset{a b}{s i n^2}",
            r#"{underover {"si" "n" ^{2}}^{ab}}"#,
        ),
        (r"\overset{a {b} c}{x}", r#"{underover {x}^{a {b} c}}"#),
        (r"\overset{a\,b c}{x}", r#"{underover {x}^{a ` bc}}"#),
        (r"\overset{a\sin b c}{x}", r#"{underover {x}^{a sin bc}}"#),
        (
            r"\overset{\frac a b c d}{x}",
            r#"{underover {x}^{{{a} over {b}} cd}}"#,
        ),
    ] {
        assert_eq!(tex_to_hml(tex).unwrap(), native, "{tex:?}");
    }
}

#[test]
fn nested_fonts_restore_literal_scope_without_changing_operators() {
    for (tex, native) in [
        (r"\mathit{si\mathrm{n}cos}x", "{si {rm {n} it} \"cos\"} x"),
        (r"\mathrm{\mathit{sin}x}", "{rm {{it {\"sin\"} rm} x} it}"),
        (
            r"\mathit{\mathbf{sin}cos}",
            "{{rm bold {\"sin\"} it} \"cos\"}",
        ),
        (
            r"\mathit{\boldsymbol{sin}cos}",
            "{{it bold {\"sin\"} it} \"cos\"}",
        ),
        (r"\mathit{sin\sin cos}", "{\"sin\" sin \"cos\"}"),
        (r"\mathit{\overset{x}{sin}}", "{{underover {\"sin\"}^{x}}}"),
    ] {
        assert_eq!(tex_to_hml(tex).unwrap(), native, "{tex}");
    }
}
