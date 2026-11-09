// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::hml_to_tex;

#[test]
fn radicals_accept_native_arguments_without_mandatory_braces() {
    for (script, expected) in [
        ("sqrt 2", r"\sqrt{2}"),
        ("root 3 of 27", r"\sqrt[3]{27}"),
        ("sqrt 2 + 3", r"\sqrt{2} + 3"),
        ("root 3 of 27 + 4", r"\sqrt[3]{27} + 4"),
        ("sqrt {2+3}", r"\sqrt{2 + 3}"),
        ("root {n+1} of {a+b}", r"\sqrt[n + 1]{a + b}"),
        ("sqrt sqrt 2", r"\sqrt{\sqrt{2}}"),
        ("root 3 of sqrt 2", r"\sqrt[3]{\sqrt{2}}"),
        ("sqrt rm x it y", r"\sqrt{\mathrm{x}} y"),
        ("sqrt scale 70 2", r"\sqrt{\scriptstyle 2}"),
    ] {
        assert_eq!(hml_to_tex(script).unwrap(), expected, "{script}");
    }
}

#[test]
fn radical_arguments_still_reject_missing_or_malformed_structure() {
    for script in [
        "sqrt",
        "sqrt rm",
        "sqrt }",
        "sqrt ^2",
        "sqrt {2",
        "root",
        "root of 27",
        "root 3 27",
        "root 3 of",
        "root 3 of }",
        "root 3 of ^2",
        "matrix{sqrt &a}",
        "root 3 of {27",
    ] {
        assert!(hml_to_tex(script).is_err(), "accepted {script:?}");
    }
}

#[test]
fn unbraced_radical_nesting_retains_resource_bounds() {
    assert!(hml_to_tex(&format!("{}2", "sqrt ".repeat(100))).is_err());
    assert!(hml_to_tex(&format!("{}2", "root 3 of ".repeat(100))).is_err());
}

#[test]
fn accents_and_boxes_accept_one_native_operand_and_preserve_groups() {
    for (native, command) in [
        ("vec", "vec"),
        ("dyad", "overleftrightarrow"),
        ("acute", "acute"),
        ("grave", "grave"),
        ("dot", "dot"),
        ("ddot", "ddot"),
        ("under", "underline"),
        ("bar", "bar"),
        ("hat", "hat"),
        ("check", "check"),
        ("arch", r"overset{\frown}"),
        ("tilde", "tilde"),
        ("BOX", "boxed"),
        ("box", "boxed"),
    ] {
        for operand in ["A", "AA", "AAA", "abc"] {
            let plain = format!("{native} {operand}");
            let braced = format!("{native} {{{operand}}}");
            let expected = format!(r"\{command}{{{operand}}}");
            assert_eq!(hml_to_tex(&plain).unwrap(), expected, "{plain}");
            assert_eq!(hml_to_tex(&braced).unwrap(), expected, "{braced}");
        }
        assert_eq!(
            hml_to_tex(&format!("{native} a + b")).unwrap(),
            format!(r"\{command}{{a}} + b")
        );
        assert_eq!(
            hml_to_tex(&format!("{native} {{a+b}} c")).unwrap(),
            format!(r"\{command}{{a + b}} c")
        );
    }
}

#[test]
fn over_and_underbraces_accept_separate_unbraced_operands() {
    for (script, expected) in [
        ("OVERBRACE a b", r"\overbrace{a}^{b}"),
        ("UNDERBRACE a b", r"\underbrace{b}_{a}"),
        ("OVERBRACE AA BB", r"\overbrace{AA}^{BB}"),
        ("UNDERBRACE {AA} {BB}", r"\underbrace{BB}_{AA}"),
        ("OVERBRACE {a+b} c + d", r"\overbrace{a + b}^{c} + d"),
        ("UNDERBRACE a {b+c} + d", r"\underbrace{b + c}_{a} + d"),
        ("vec hat AA", r"\vec{\hat{AA}}"),
    ] {
        assert_eq!(hml_to_tex(script).unwrap(), expected, "{script}");
    }
}

#[test]
fn specification_decoration_examples_accept_the_original_unbraced_scripts() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/specification-equations.json")).unwrap();
    let mut count = 0;
    for case in fixture["cases"].as_array().unwrap() {
        let id = case["id"].as_str().unwrap();
        if case["category"] == "글자 장식 명령어" && case["source_kind"] == "explicit_script"
        {
            let script = case["hancom_script"].as_str().unwrap();
            assert!(hml_to_tex(script).is_ok(), "{id}: {script}");
            count += 1;
        }
    }
    assert_eq!(count, 20);
}

#[test]
fn constructor_operands_keep_reserved_commands_and_structure_validation() {
    for script in [
        "vec",
        "bar rm",
        "hat }",
        "vec ^a",
        "OVERBRACE a",
        "UNDERBRACE {a}",
        "OVERBRACE a }",
        "vec MYSTERY{a}",
        "hat {MYSTERY{a}}",
        "vec SQRT",
        "vec ladder",
        "vec {longdiv}",
        "hat AA MYSTERY{a}",
        "hat AA BB",
        "AA",
        "MYSTERY{a}",
        "REL rarrow a b",
        "COLOR {255,0,0} a",
        "matrix a",
    ] {
        assert!(hml_to_tex(script).is_err(), "accepted {script:?}");
    }
    assert_eq!(hml_to_tex("vec NIN").unwrap(), r"\vec{\notin}");
    assert_eq!(hml_to_tex("vec {NSUBSET}").unwrap(), r"\vec{\not\subset}");
    assert!(hml_to_tex(&format!("{}A", "vec ".repeat(100))).is_err());
    assert!(hml_to_tex(&format!("{}A", "OVERBRACE A ".repeat(100))).is_err());
}
