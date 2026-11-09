// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::{hml_to_tex, tex_to_hml};

#[test]
fn middle_preserves_visible_delimiters_without_native_auto_sizing() {
    for (tex, expected) in [
        (r"\left(x\middle/y\right)", "{LEFT ( x / y RIGHT )}"),
        (
            r"\left(x\middle|y\middle|z\right)",
            "{LEFT ( x | y | z RIGHT )}",
        ),
        (r"\left(x\middle.y\right)", "{LEFT ( x y RIGHT )}"),
        (r"\left(x\middle\{y\right)", "{LEFT ( x \"{\" y RIGHT )}"),
    ] {
        let native = tex_to_hml(tex).unwrap();
        assert_eq!(native, expected);
        assert!(hml_to_tex(&native).is_ok());
    }
    for tex in [
        r"\middle|",
        r"\left(x\middle\alpha y\right)",
        r"\left(x\middle",
    ] {
        assert!(tex_to_hml(tex).is_err(), "{tex}");
    }
}

#[test]
fn hidden_right_curly_delimiter_roundtrips() {
    let native = tex_to_hml(r"\left\{x\right.").unwrap();
    assert_eq!(native, "{LEFT { x RIGHT .}");
    let tex = hml_to_tex(&native).unwrap();
    assert!(tex.contains(r"\right."), "{tex}");
    assert!(tex_to_hml(&tex).is_ok());
}

#[test]
fn original_middle_fixture_converts_without_changing_rule_policy() {
    let f: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/katex/cases.json")).unwrap();
    let case = f["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "katex-051")
        .unwrap();
    let native = tex_to_hml(case["tex"].as_str().unwrap()).unwrap();
    assert!(native.contains("RIGHT ."));
    assert!(hml_to_tex(&native).is_ok());
    assert!(tex_to_hml(r"\rule{0.1em}{1em}").is_err());
    assert_eq!(tex_to_hml(r"\mathrlap{x}").unwrap(), "{x}");
}
