// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::{hml_to_tex, tex_to_hml};

#[test]
fn lower_decorations_keep_bodies_and_outer_scripts() {
    for (command, mark) in [
        ("underleftarrow", "larrow"),
        ("underrightarrow", "rarrow"),
        ("underleftrightarrow", "lrarrow"),
        ("undergroup", "\"⌣\""),
        ("utilde", "SIM"),
    ] {
        let script = tex_to_hml(&format!(r"\{command}{{AB}}^2")).unwrap();
        let expected = format!("{{underover {{\"AB\"}}_{{{mark}}}}} ^{{2}}");
        assert_eq!(script, expected);
        let exported = hml_to_tex(&script).unwrap();
        assert!(tex_to_hml(&exported).is_ok(), "{exported}");
        assert!(tex_to_hml(&format!(r"\{command}")).is_err());
    }
}

#[test]
fn underline_segment_uses_underover_and_text_underline_stays_native() {
    assert_eq!(
        tex_to_hml(r"\underlinesegment{AB}").unwrap(),
        "{underover {\"AB\"}_{\"─\"}}"
    );
    let script = tex_to_hml(r"\text{\underline{text}}").unwrap();
    assert!(script.contains("under"), "{script}");
    assert!(script.contains("\"text\""), "{script}");
    assert!(hml_to_tex(&script).is_ok());
}

#[test]
fn original_lower_accent_fixture_converts_and_exports() {
    let f: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/katex/cases.json")).unwrap();
    let case = f["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "katex-055")
        .unwrap();
    let native = tex_to_hml(case["tex"].as_str().unwrap()).unwrap();
    assert!(hml_to_tex(&native).is_ok());
}
