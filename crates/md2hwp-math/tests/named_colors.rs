// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::{hml_to_tex, tex_to_hml};
use std::collections::HashSet;

fn fixture() -> serde_json::Value {
    serde_json::from_str(include_str!("../../../tests/katex/named-colors.json")).unwrap()
}

fn color_tag(case: &serde_json::Value) -> String {
    let rgb = case["rgb"].as_array().unwrap();
    format!("color {{{}, {}, {}}}", rgb[0], rgb[1], rgb[2])
}

#[test]
fn every_css_name_matches_independent_browser_rgb_and_accepts_ascii_case() {
    let data = fixture();
    let cases = data["css"].as_array().unwrap();
    assert_eq!(cases.len(), 148);
    let mut names = HashSet::new();
    for case in cases {
        let name = case["name"].as_str().unwrap();
        assert!(names.insert(name));
        for spelling in [name.to_owned(), name.to_ascii_uppercase()] {
            for tex in [
                format!(r"\textcolor{{{spelling}}}{{x}}"),
                format!(r"\color{{{spelling}}}x"),
            ] {
                let native = tex_to_hml(&tex).unwrap();
                assert!(native.contains(&color_tag(case)), "{tex}: {native}");
                assert!(hml_to_tex(&native).is_ok(), "{native}");
            }
        }
        assert!(tex_to_hml(&format!(r"\colorbox{{{name}}}{{x}}")).is_ok());
        assert!(tex_to_hml(&format!(r"\fcolorbox{{{name}}}{{red}}{{x}}")).is_ok());
    }
}

#[test]
fn every_katex_palette_command_preserves_its_distinct_rgb_and_argument_scope() {
    let data = fixture();
    let cases = data["commands"].as_array().unwrap();
    assert_eq!(cases.len(), 56);
    let mut names = HashSet::new();
    for case in cases {
        let name = case["name"].as_str().unwrap();
        assert!(names.insert(name));
        let tex = format!("a\\{name}{{x}}b");
        let native = tex_to_hml(&tex).unwrap();
        assert_eq!(
            native,
            format!("a {{{} {{x}}}} b", color_tag(case)),
            "{tex}"
        );
        assert!(hml_to_tex(&native).is_ok());
    }
    assert!(tex_to_hml(r"\BlueA{x}").is_err());
    assert!(tex_to_hml(r"\textcolor{blueA}{x}").is_err());
}

#[test]
fn color_values_requiring_alpha_or_context_and_unknown_names_fail_explicitly() {
    for name in [
        "transparent",
        "currentColor",
        "CanvasText",
        "unknown",
        "red;display:none",
    ] {
        assert!(
            tex_to_hml(&format!(r"\textcolor{{{name}}}{{x}}")).is_err(),
            "{name}"
        );
    }
    assert!(tex_to_hml(r"\newcommand{\red}{x}\red").is_err());
}
