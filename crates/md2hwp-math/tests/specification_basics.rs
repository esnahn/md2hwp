// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::{hml_to_tex, tex_to_hml};

fn math_without_size(tex: &str) -> String {
    tex.replace(r"\scriptscriptstyle", "")
        .replace(r"\scriptstyle", "")
        .replace(r"\textstyle", "")
        .chars()
        .filter(|c| !c.is_whitespace() && !matches!(c, '{' | '}'))
        .collect()
}

#[test]
fn scale_ranges_map_to_the_nearest_supported_math_style() {
    for (scale, style) in [
        (0.25, "scriptscriptstyle"),
        (50.0, "scriptscriptstyle"),
        (59.9, "scriptscriptstyle"),
        (60.0, "scriptstyle"),
        (70.0, "scriptstyle"),
        (84.9, "scriptstyle"),
        (85.0, "textstyle"),
        (100.0, "textstyle"),
        (200.0, "textstyle"),
    ] {
        assert_eq!(
            hml_to_tex(&format!("scale {scale} a")).unwrap(),
            format!("\\{style} a")
        );
    }
    assert_eq!(hml_to_tex("a scale 100").unwrap(), r"a \textstyle");
}

#[test]
fn scale_preserves_the_full_math_with_style_declarations() {
    for (script, plain) in [
        (
            "유리수={scale 70 분자 over 분모 }={ scale 90 m over n }",
            "유리수={분자 over 분모}={m over n}",
        ),
        ("scale 70 a + b over c", "a + b over c"),
        ("scale 0.25 {a+b} c_{d}", "{a+b} c_{d}"),
        ("{scale 90 a} b", "{a} b"),
        ("a_{scale 70 b+c}", "a_{b+c}"),
        ("matrix {scale 70 a&scale 90 b#c&d}", "matrix {a&b#c&d}"),
        ("scale 70 scale 90 a", "a"),
    ] {
        assert_eq!(
            math_without_size(&hml_to_tex(script).unwrap()),
            math_without_size(&hml_to_tex(plain).unwrap()),
            "{script}"
        );
    }
}

#[test]
fn scale_preserves_persistent_native_font_state() {
    for (script, plain) in [
        ("rm scale 70 Equation it test", "rm Equation it test"),
        (
            "scale 90 bold Equation rm font it test",
            "bold Equation rm font it test",
        ),
        ("{scale 70 rm a} b it c", "{rm a} b it c"),
        ("a_{scale 70 bold b} c it d", "a_{bold b} c it d"),
    ] {
        assert_eq!(
            math_without_size(&hml_to_tex(script).unwrap()),
            math_without_size(&hml_to_tex(plain).unwrap()),
            "{script}"
        );
    }
    assert_eq!(hml_to_tex(r#""scale""#).unwrap(), "scale");
}

#[test]
fn scale_rejects_missing_malformed_or_nonpositive_sizes_and_bodies() {
    for script in [
        "scale",
        "scale a",
        "scale 0 a",
        "scale 0.0 a",
        "scale -1 a",
        "scale +1 a",
        "scale .5 a",
        "scale 1.2.3 a",
        r#"scale "70" a"#,
        "scale 70",
        "scale 70 rm",
        "{scale 70}",
        "matrix{scale 70&a}",
        "scale 70 ^a",
        "a_{scale 70}",
        "Scale 70 a",
        "SCALE 70 a",
    ] {
        assert!(hml_to_tex(script).is_err(), "accepted {script:?}");
    }
    assert!(hml_to_tex(&format!("scale {} a", "9".repeat(400))).is_err());
}

#[test]
fn ordinary_capitalized_words_keep_the_specification_font_examples() {
    for (script, tex) in [
        ("Equation font test", r"Equation font test"),
        ("Equation rm font it test", r"Equation \mathrm{font} test"),
        (
            "rm Equation~ bold Editor",
            r"\mathrm{Equation} \  \mathbf{Editor}",
        ),
        ("aBc xyZ", "aBc xyZ"),
    ] {
        let actual = hml_to_tex(script).unwrap();
        assert_eq!(actual, tex, "{script}");
        let reconstructed = tex_to_hml(&actual).unwrap();
        let second = hml_to_tex(&reconstructed).unwrap();
        // TeX parsing splits a word into letters; Roman/Bold scopes may then
        // export as adjacent per-letter commands. Compare the retained text,
        // while the exact first export above verifies the specified font scope.
        let letters = |value: &str| {
            value
                .replace("\\mathrm", "")
                .replace("\\mathbf", "")
                .replace("\\boldsymbol", "")
                .replace("\\mathit", "")
                .chars()
                .filter(|c| c.is_alphabetic())
                .collect::<String>()
        };
        assert_eq!(
            letters(&second),
            letters(&actual),
            "{script}: {reconstructed}"
        );
    }
}

#[test]
fn ordinary_words_do_not_hide_reserved_commands_or_wrong_case_symbols() {
    for script in [
        "MYSTERY{a}",
        "UNSUPPORTED {a}",
        "Mystery{a}",
        "AlPhA",
        "Vartheta",
        "Sqrt",
        "Sin",
        "Rm a",
        "Scale 70 a",
        "Box {a}",
    ] {
        assert!(hml_to_tex(script).is_err(), "accepted {script:?}");
    }
    assert_eq!(hml_to_tex("alpha").unwrap(), "\\alpha");
    assert_eq!(hml_to_tex("LRARROW").unwrap(), "\\Leftrightarrow");
}

#[test]
fn scale_declaration_runs_and_nested_bodies_keep_existing_resource_bounds() {
    assert!(hml_to_tex(&format!("{}a", "scale 70 ".repeat(16_384))).is_err());
    assert!(hml_to_tex(&format!("{}a{}", "{scale 70 ".repeat(100), "}".repeat(100))).is_err());
    assert!(hml_to_tex(&format!("scale 70 \"{}\"", "^".repeat(262_144))).is_err());
}

#[test]
fn specification_function_names_keep_native_spelling_and_script_positions() {
    for name in [
        "sin", "cos", "tan", "cot", "sec", "csc", "log", "ln", "exp", "sinh", "cosh", "tanh",
        "coth", "lg", "max", "min", "arcsin", "arccos", "arctan", "det", "gcd", "deg", "arg",
        "dim", "hom", "ker", "Pr",
    ] {
        assert_eq!(hml_to_tex(name).unwrap(), format!(r"\{name}"));
        assert_eq!(
            hml_to_tex(&format!("{name}_{{a}}^{{b}} c")).unwrap(),
            format!(
                r"\{name}{}_{{a}}^{{b}} c",
                if matches!(name, "max" | "min" | "det" | "gcd" | "Pr") {
                    r"\limits"
                } else {
                    ""
                }
            )
        );
        assert_eq!(
            hml_to_tex(&format!("bold {name}_{{a}} it c")).unwrap(),
            format!(
                r"\{name}{}_{{a}} c",
                if matches!(name, "max" | "min" | "det" | "gcd" | "Pr") {
                    r"\limits"
                } else {
                    ""
                }
            )
        );
    }
    for name in ["cosec", "Exp", "arc", "mod", "if", "for", "and"] {
        assert_eq!(
            hml_to_tex(name).unwrap(),
            format!(r"\operatorname{{{name}}}")
        );
        assert_eq!(
            hml_to_tex(&format!("{name}_{{a}}^{{b}} c")).unwrap(),
            format!(r"\operatorname{{{name}}}_{{a}}^{{b}} c")
        );
        assert_eq!(hml_to_tex(&format!(r#""{name}""#)).unwrap(), name);
    }
    assert_eq!(
        hml_to_tex("Lim_{a} c").unwrap(),
        "\\operatorname*{Lim}\\limits_{a} c"
    );
    for name in [
        "Coth", "LG", "Max", "ArcSin", "Det", "Gcd", "Hom", "Ker", "pr", "COSEC", "Mod", "If",
    ] {
        assert!(
            hml_to_tex(name).is_err(),
            "accepted wrong-case function {name}"
        );
    }
}

#[test]
fn korean_words_bind_as_complete_native_fraction_operands() {
    assert_eq!(
        hml_to_tex("유리수={scale 70 분자 over 분모 }={ scale 90 m over n }").unwrap(),
        "유리수 = {\\scriptstyle \\frac{분자}{분모}} = {\\textstyle \\frac{m}{n}}"
    );
    for (script, expected) in [
        ("분자 over 분모", r"\frac{분자}{분모}"),
        ("분자over분모", r"\frac{분자}{분모}"),
        (
            "앞말+분자 over 분모+뒷말",
            r"\frac{앞말 + 분자}{분모 + 뒷말}",
        ),
        (
            "rm 분자 over 분모 it 끝",
            r"\frac{\mathrm{분자}}{\mathrm{분모}} 끝",
        ),
        ("한글alpha+한글2", r"한글 \alpha + 한글 2"),
        ("한글{a}", "한글 a"),
        ("αβ over γδ", r"\frac{α β}{γ δ}"),
        ("분자 over 분모", r"\frac{분자}{분모}"),
    ] {
        assert_eq!(hml_to_tex(script).unwrap(), expected, "{script}");
    }
}
