// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::{hml_to_tex, tex_to_hml};

#[test]
fn buildrel_has_one_label_and_does_not_consume_the_following_body() {
    for command in ["BUILDREL", "buildrel"] {
        let tex = hml_to_tex(&format!("A {command} <-> {{+2}} B")).unwrap();
        assert_eq!(tex, r"A \overset{+ 2}{\longleftrightarrow} B");
        let native = tex_to_hml(&tex).unwrap();
        assert_eq!(
            hml_to_tex(&native).unwrap(),
            r"A \overset{+ 2}{\underset{}{\longleftrightarrow}} B"
        );
    }
    for arrow in [
        "LRARROW", "lrarrow", "RARROW", "LARROW", "rarrow", "larrow", "EXARROW",
    ] {
        let tex = hml_to_tex(&format!("BUILDREL {arrow} {{a}} x")).unwrap();
        assert!(tex.ends_with(" x"), "{arrow}: {tex}");
        assert!(tex_to_hml(&tex).is_ok(), "{arrow}: {tex}");
    }
    for invalid in [
        "BUILDREL",
        "BUILDREL RARROW",
        "BUILDREL x {a}",
        "BUILDREL RARROW a",
    ] {
        assert!(hml_to_tex(invalid).is_err(), "{invalid}");
    }
}

fn compact(script: &str) -> String {
    script.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]
fn arbitrary_bodies_use_general_over_and_under_annotations() {
    for (tex, expected) in [
        (r"\overset{a}{x}", "{underover {x}^{a}}"),
        (r"\underset{b}{x}", "{underover {x}_{b}}"),
        (r"\overset{a}{x+y}", "{underover {x + y}^{a}}"),
        (r"\underset{b}{x+y}", "{underover {x + y}_{b}}"),
        (
            r"\overset{a}{\underset{b}{x+y}}",
            "{underover {x + y}_{b}^{a}}",
        ),
    ] {
        let actual = tex_to_hml(tex).unwrap_or_else(|e| panic!("{tex:?}: {e}"));
        assert_eq!(compact(&actual), compact(expected), "TeX {tex:?}");
    }
}

#[test]
fn both_nesting_orders_flatten_to_the_same_two_sided_annotation() {
    let over_then_under = tex_to_hml(r"\overset{a}{\underset{b}{x+y}}").unwrap();
    let under_then_over = tex_to_hml(r"\underset{b}{\overset{a}{x+y}}").unwrap();

    assert_eq!(
        compact(&over_then_under),
        compact(&under_then_over),
        "the order of the TeX annotation commands must not change the result"
    );
    assert_eq!(compact(&over_then_under), "{underover {x + y}_{b}^{a}}");
}

#[test]
fn redundant_groups_around_annotation_arguments_are_semantically_redundant() {
    let plain = compact(&tex_to_hml(r"\overset{a}{\underset{b}{\Longleftrightarrow}}").unwrap());
    for tex in [
        r"\overset{a}{{\underset{b}{\Longleftrightarrow}}}",
        r"\overset{a}{\underset{b}{{\Longleftrightarrow}}}",
        r"\underset{b}{{\overset{a}{{\Longleftrightarrow}}}}",
    ] {
        assert_eq!(compact(&tex_to_hml(tex).unwrap()), plain, "TeX {tex:?}");
    }
}

#[test]
fn annotated_native_arrows_keep_their_relation_and_both_label_slots() {
    for (command, native) in [
        ("Longleftrightarrow", "LRARROW"),
        ("longleftrightarrow", "lrarrow"),
        ("Longrightarrow", "RARROW"),
        ("Longleftarrow", "LARROW"),
        ("rightarrow", "rarrow"),
        ("leftarrow", "larrow"),
        ("rightleftarrows", "EXARROW"),
    ] {
        let over_then_under = format!(r"\overset{{a}}{{\underset{{b}}{{\{command}}}}}");
        let under_then_over = format!(r"\underset{{b}}{{\overset{{a}}{{\{command}}}}}");
        let expected = format!("{{rel {native} {{a}} {{b}}}}");

        for tex in [&over_then_under, &under_then_over] {
            let actual = tex_to_hml(tex).unwrap_or_else(|e| panic!("{tex:?}: {e}"));
            assert_eq!(compact(&actual), compact(&expected), "TeX {tex:?}");
        }
    }

    for (tex, expected) in [
        (r"\overset{a}{\rightarrow}", "{rel rarrow {a} {}}"),
        (r"\underset{b}{\rightarrow}", "{rel rarrow {} {b}}"),
    ] {
        assert_eq!(
            compact(&tex_to_hml(tex).unwrap()),
            compact(expected),
            "TeX {tex:?}"
        );
    }
}

#[test]
fn scalar_leftarrow_is_native_but_leq_is_not_an_arrow() {
    assert_eq!(
        compact(&tex_to_hml(r"\overset{a}{\Leftarrow}").unwrap()),
        "{rel LARROW {a} {}}"
    );

    let leq = tex_to_hml(r"\overset{a}{\leq}").unwrap();
    assert!(leq.contains("underover"), "unexpected TeX lowering: {leq}");
    assert!(leq.contains("LEQ"), "relation body was lost: {leq}");
    assert!(
        !leq.contains("{rel "),
        "LEQ was mistaken for an arrow: {leq}"
    );
}

#[test]
fn repeated_annotations_scripts_and_sums_stay_general() {
    let repeated_above = tex_to_hml(r"\overset{o}{\overset{i}{x}}").unwrap();
    assert!(repeated_above.contains("underover"), "{repeated_above}");
    assert!(repeated_above.contains("^{o}"), "{repeated_above}");
    assert!(repeated_above.contains("^{i}"), "{repeated_above}");

    let repeated_below = tex_to_hml(r"\underset{o}{\underset{i}{x}}").unwrap();
    assert!(repeated_below.contains("underover"), "{repeated_below}");
    assert!(repeated_below.contains("_{o}"), "{repeated_below}");
    assert!(repeated_below.contains("_{i}"), "{repeated_below}");

    let scripted = tex_to_hml(r"\overset{a}{x_i^2}").unwrap();
    assert!(scripted.contains("underover"), "{scripted}");
    assert!(
        scripted.contains("_{i}"),
        "base subscript was lost: {scripted}"
    );
    assert!(
        scripted.contains("^{2}"),
        "base superscript was lost: {scripted}"
    );
    assert!(scripted.contains("^{a}"), "annotation was lost: {scripted}");
    assert!(
        !scripted.contains("{rel "),
        "non-arrow became an arrow: {scripted}"
    );

    let sum = tex_to_hml(r"\overset{a}{\sum}").unwrap();
    assert!(sum.contains("underover"), "{sum}");
    assert!(sum.contains("sum"), "sum base was lost: {sum}");
    assert!(sum.contains("^{a}"), "annotation was lost: {sum}");
    assert!(!sum.contains("{rel "), "sum became an arrow: {sum}");
}

#[test]
fn arrow_names_in_text_are_not_native_arrow_bases() {
    for tex in [r"\overset{a}{LRARROW}", r"\overset{a}{\text{LRARROW}}"] {
        let actual = tex_to_hml(tex).unwrap();
        assert!(!actual.contains("{rel "), "TeX {tex:?}: {actual}");
    }

    let text = tex_to_hml(r"\overset{a}{\text{LRARROW}}").unwrap();
    assert!(text.contains("LRARROW"), "text label was lost: {text}");
}

#[test]
fn malformed_annotation_arguments_and_groups_are_rejected() {
    for tex in [
        r"\overset",
        r"\overset{a}",
        r"\overset{a}{x",
        r"\overset{a}}{x}",
        r"\underset",
        r"\underset{b}",
        r"\underset{b}{x",
    ] {
        assert!(
            tex_to_hml(tex).is_err(),
            "malformed TeX was accepted: {tex:?}"
        );
    }
}

#[test]
fn annotation_parsing_obeys_depth_and_atom_resource_limits() {
    let nested = (0..100).fold("x".to_owned(), |body, _| {
        format!(r"\overset{{a}}{{{body}}}")
    });
    assert!(
        tex_to_hml(&nested).is_err(),
        "excessive nesting was accepted"
    );

    let long_label = format!(r"\overset{{{}}}{{x}}", "a".repeat(32_769));
    assert!(
        tex_to_hml(&long_label).is_err(),
        "an annotation exceeded the atom limit without an error"
    );
}

#[test]
fn additional_terms_and_scripted_arrows_cannot_be_collapsed_to_rel() {
    for tex in [
        r"\overset{a}{\Longleftrightarrow_i}",
        r"\overset{a}{\underset{b}{\Longleftrightarrow}z}",
        r"\overset{a}{\underset{b}{x}z}",
    ] {
        let actual = tex_to_hml(tex).unwrap();
        assert!(actual.starts_with("{underover "), "{tex}: {actual}");
        assert!(actual.contains("^{a}"), "{tex}: {actual}");
        if tex.ends_with("z}") {
            assert!(actual.contains('z'), "{tex}: {actual}");
            assert!(actual.contains("{b}"), "{tex}: {actual}");
        } else {
            assert!(actual.contains("_{i}"), "{tex}: {actual}");
        }
    }
}
