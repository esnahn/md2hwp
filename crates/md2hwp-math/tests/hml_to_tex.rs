// SPDX-License-Identifier: AGPL-3.0-only
//! Structural coverage for the user-approved native toolbar grammar.
use md2hwp_math::hml_to_tex;
use serde_json::Value;

fn toolbar() -> Value {
    serde_json::from_str(include_str!("fixtures/toolbar-equations.json")).unwrap()
}

fn compact(tex: &str) -> String {
    tex.chars().filter(|c| !c.is_whitespace()).collect()
}

// A converter may retain the disposable outer SCRIPT grouping. Only remove a
// group enclosing the complete expression; never remove argument/base groups.
fn normalize(tex: &str) -> String {
    let mut out = compact(tex).replace(r"\limits", "");
    loop {
        if !out.starts_with('{') || !out.ends_with('}') {
            break;
        }
        let mut depth = 0;
        let mut escaped = false;
        let mut first_close = None;
        for (i, c) in out.char_indices() {
            if escaped {
                escaped = false;
                continue;
            }
            if c == '\\' {
                escaped = true;
                continue;
            }
            if c == '{' {
                depth += 1;
            } else if c == '}' {
                depth -= 1;
                if depth == 0 {
                    first_close = Some(i);
                    break;
                }
            }
        }
        if first_close != Some(out.len() - 1) {
            break;
        }
        out = out[1..out.len() - 1].to_owned();
    }
    out
}

fn assert_tex(script: &str, expected: &str) {
    let actual = hml_to_tex(script).unwrap_or_else(|e| panic!("{script}: {e}"));
    assert_eq!(
        normalize(&actual),
        normalize(expected),
        "SCRIPT {script}; TeX {actual}"
    );
}

fn commands(tex: &str) -> Vec<String> {
    let mut chars = tex.chars().peekable();
    let mut out = Vec::new();
    while let Some(c) = chars.next() {
        if c != '\\' {
            continue;
        }
        let mut name = String::new();
        while chars.peek().is_some_and(char::is_ascii_alphabetic) {
            name.push(chars.next().unwrap());
        }
        if !name.is_empty() && name != "limits" {
            out.push(name);
        }
    }
    out
}

#[test]
fn all_76_approved_toolbar_operations_emit_the_expected_tex_constructors() {
    let fixture = toolbar();
    let cases = fixture["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 76);
    for case in cases {
        let script = case["hancom_script"].as_str().unwrap();
        let expected = case["tex"].as_str().unwrap();
        let actual = hml_to_tex(script)
            .unwrap_or_else(|e| panic!("toolbar {} ({}): {e}", case["case_id"], case["name"]));
        assert!(!actual.trim().is_empty(), "{}", case["case_id"]);
        // Check constructor order across every sample without requiring all
        // harmless whitespace or redundant source groups to disappear.
        assert_eq!(
            commands(&actual),
            commands(expected),
            "{}: {actual}",
            case["case_id"]
        );
    }
}

#[test]
fn approved_scripts_preserve_argument_roles_and_limit_placement() {
    let fixture = toolbar();
    for id in [
        "001", "002", "003", "004", "005", "007", "013", "017", "028", "031", "033", "035", "036",
        "042", "043", "048", "049", "050", "051", "052", "053", "054", "055", "056", "057", "058",
        "061", "064", "067", "068", "070",
    ] {
        let case = fixture["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|case| case["case_id"] == id)
            .unwrap();
        assert_tex(
            case["hancom_script"].as_str().unwrap(),
            case["tex"].as_str().unwrap(),
        );
    }
}

#[test]
fn grouped_and_nested_prescripts_bind_to_their_own_base() {
    assert_tex("{a+b} LSUP {c}", r"{}^{c}{a+b}");
    assert_tex("{a+b} LSUB {c}", r"{}_{c}{a+b}");
    let paired = normalize(&hml_to_tex("a LSUP {b} LSUB {c}").unwrap());
    assert!(
        paired == r"{}_{c}^{b}a" || paired == r"{}^{b}_{c}a",
        "{paired}"
    );
    let nested = compact(&hml_to_tex("vec {{a+b} LSUP {c}} + x^{y LSUB {z}}").unwrap());
    assert!(nested.contains(r"\vec{"), "{nested}");
    assert!(nested.contains(r"{}^{c}{a+b}"), "{nested}");
    assert!(nested.contains(r"{}_{z}y"), "{nested}");
}

#[test]
fn composite_limits_relations_and_braces_keep_nested_operands() {
    let limit = compact(&hml_to_tex("UNDEROVER {a+b}_{x+y}^{z}").unwrap());
    assert!(limit.contains(r"\mathop{a+b}\limits_{x+y}^{z}"), "{limit}");
    let relation = compact(&hml_to_tex("REL rarrow {{a} over {b}} {c+d}").unwrap());
    assert!(relation.contains(r"\xrightarrow[c+d]"), "{relation}");
    assert!(relation.contains(r"\frac{a}{b}"), "{relation}");
    let brace = compact(&hml_to_tex("UNDERBRACE {x+y} {{a} over {b}}").unwrap());
    assert!(brace.contains(r"\underbrace{"), "{brace}");
    assert!(brace.contains(r"\frac{a}{b}"), "{brace}");
    assert!(brace.ends_with("_{x+y}"), "{brace}");
    assert_tex("lim _{x rarrow 0} c", r"\lim_{x\to0}c");
    assert_tex("Lim _{x rarrow 0} c", r"\operatorname*{Lim}_{x\to0}c");
}

#[test]
fn multiple_contour_integrals_keep_nested_bounds_and_bodies() {
    assert_tex(
        "odint _{x+1}^{root {3} of {y}} {{a} over {b}}",
        r"\oiint_{x+1}^{\sqrt[3]{y}}\frac{a}{b}",
    );
    assert_tex("sqrt {otint _{a}^{b} {f}}", r"\sqrt{\oiiint_{a}^{b}f}");
}

#[test]
fn fractions_do_not_capture_adjacent_addends_or_swap_nested_operands() {
    assert_tex("x+{a} over {b}+y", r"x+\frac{a}{b}+y");
    assert_tex("a over b", r"\frac{a}{b}");
    // Independently observed in Hancom: scripts bind to an unbraced denominator.
    // A script on the entire fraction requires a group around that fraction.
    assert_tex("a over b^c", r"\frac{a}{b^{c}}");
    assert_tex("a over b LSUP {c}", r"\frac{a}{{}^{c}b}");
    assert_tex("{{a} over {b}}^c", r"\frac{a}{b}^{c}");
    let nested = compact(&hml_to_tex("{{a} over {b}} over {{c} over {d}}").unwrap());
    assert_eq!(commands(&nested).iter().filter(|c| *c == "frac").count(), 3);
    assert!(
        nested.find(r"\frac{a}{b}").unwrap() < nested.find(r"\frac{c}{d}").unwrap(),
        "{nested}"
    );
}

#[test]
fn fractions_keep_context_sensitive_style_in_cases_scripts_and_nested_fractions() {
    for script in [
        "{1} over {a^{n-m}}",
        "cases{{1} over {a^{n-m}}&m<n#1&m=n}",
        "a^{{1} over {n}}",
        "{1} over {1+{1} over {a^{n-m}}}",
    ] {
        let tex = hml_to_tex(script).unwrap();
        assert!(tex.contains(r"\frac"), "{script}: {tex}");
        assert!(!commands(&tex).iter().any(|name| name == "dfrac"));
    }
}

#[test]
fn compact_vector_prefixes_preserve_native_blackboard_and_vector_forms() {
    for letter in 'A'..='Z' {
        let script = format!("Vec{letter}");
        assert_tex(&script, &format!(r"\mathbb{{{letter}}}"));
    }
    for (script, expected) in [
        ("VecAB", r"\mathbb{A}B"),
        ("VecABC", r"\mathbb{A}BC"),
        ("VecAALPHA", r"\mathbb{A}A"),
        ("VecAINF", r"\mathbb{A}\infty"),
        ("vecab", r"\vec{ab}"),
        ("vecAlpha", r"\vec{A}"),
        ("vecA", r"\vec{A}"),
        ("Veca", r"\vec{a}"),
        ("VECA", r"\vec{A}"),
        ("veCalpha", "veCalpha"),
        ("VecA_i", r"\mathbb{A}_{i}"),
        ("Veca_i", r"\vec{a_{i}}"),
        ("vec {a_i}", r"\vec{a_{i}}"),
        ("vec {a}_i", r"\vec{a_{i}}"),
        ("vecab_i", r"\vec{ab_{i}}"),
        ("{vec {a}}_i", r"\vec{a}_{i}"),
        ("sqrt {a}_i", r"\sqrt{a}_{i}"),
        ("{{VecAB} over {vecab}}", r"\frac{\mathbb{A}B}{\vec{ab}}"),
        (
            "sqrt {vec {{VecA} over {VecB}}}",
            r"\sqrt{\vec{\frac{\mathbb{A}}{\mathbb{B}}}}",
        ),
    ] {
        assert_tex(script, expected);
    }
    assert_tex("VecAMHO", r"\mathbb{A} \mho");
    assert_tex("VecASMALLCOPROD", r"\mathbb{A} {\textstyle\coprod}");
    assert!(hml_to_tex("VecALADDER").is_err());
}

#[test]
fn arrays_retain_cell_row_order_and_nested_constructors() {
    for (name, tex_env) in [
        ("matrix", "matrix"),
        ("pmatrix", "pmatrix"),
        ("dmatrix", "vmatrix"),
        ("bmatrix", "bmatrix"),
        ("cases", "cases"),
    ] {
        assert_tex(
            &format!("{name}{{a&b#c&d}}"),
            &format!(r"\begin{{{tex_env}}}a&b\\c&d\end{{{tex_env}}}"),
        );
    }
    assert_tex("pile{a#b}", r"\begin{gathered}a\\b\end{gathered}");
    let nested = compact(&hml_to_tex("matrix{{a} over {b}&vec{c}#d LSUP {e}&Lim_{x}f}").unwrap());
    for expected in [
        r"\begin{matrix}",
        r"\frac{a}{b}",
        r"\vec{c}",
        r"{}^{e}d",
        r"\operatorname*{Lim}",
        r"\end{matrix}",
    ] {
        assert!(nested.contains(expected), "missing {expected} in {nested}");
    }
    assert_eq!(nested.matches('&').count(), 2);
    assert_eq!(nested.matches(r"\\").count(), 1);
}

#[test]
fn multi_character_symbol_tokens_use_longest_match() {
    assert_tex(
        "a <-> b != c == d << e",
        r"a\leftrightarrow b\neq c\equiv d\ll e",
    );
}

#[test]
fn malformed_unknown_and_excessive_input_fails_explicitly() {
    for script in [
        "",
        "{a",
        "a}",
        "a over",
        "over b",
        "sqrt",
        "root {n} of",
        "a^",
        "a LSUP",
        "UNDEROVER",
        "REL rarrow {a}",
        "REL UNKNOWN {a}{b}",
        "UNDERBRACE {a}",
        "UNSUPPORTED {a}",
        "MYSTERY{a}",
        "bigg x",
        "a binom b",
        "LEFT (a",
        "LEFT (a RIGHT",
        "a\0b",
    ] {
        assert!(
            hml_to_tex(script).is_err(),
            "unexpectedly accepted {script:?}"
        );
    }
    let deep = format!("{}a{}", "{".repeat(512), "}".repeat(512));
    assert!(hml_to_tex(&deep).is_err());
    assert!(hml_to_tex(&"a".repeat(8 * 1024 * 1024 + 1)).is_err());
    // Each row is below the output limit, but escaping both must not bypass the
    // cumulative output budget through a root aligned environment.
    let expanded_rows = format!("\"{}\"#\"{}\"", "^".repeat(130_000), "^".repeat(130_000));
    assert!(hml_to_tex(&expanded_rows).is_err());
}
