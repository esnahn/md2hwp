// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::{hml_to_tex, tex_to_hml};

#[test]
fn mapsfrom_command_and_unicode_keep_direction_in_nested_scripts() {
    assert_eq!(
        tex_to_hml(r"a\mapsfrom b").unwrap(),
        tex_to_hml("a↤b").unwrap()
    );
    assert_eq!(
        tex_to_hml("a↦b").unwrap(),
        tex_to_hml(r"a\mapsto b").unwrap()
    );
    assert_eq!(hml_to_tex("\"↤\"").unwrap(), r"\mapsfrom");
    for tex in [r"\mapsfrom-x", r"x_{a\mapsfrom b}", r"x_{y_{a\mapsfrom b}}"] {
        let native = tex_to_hml(tex).unwrap();
        let exported = hml_to_tex(&native).unwrap();
        assert!(exported.contains(r"\mapsfrom"), "{exported}");
        assert_eq!(tex_to_hml(&exported).unwrap(), native);
    }
    assert!(tex_to_hml(r"\mapsfrombad").is_err());
}

#[test]
fn original_mapsfrom_fixture_converts_and_exports() {
    let f: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/katex/cases.json")).unwrap();
    let case = f["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "katex-056")
        .unwrap();
    assert!(hml_to_tex(&tex_to_hml(case["tex"].as_str().unwrap()).unwrap()).is_ok());
}
