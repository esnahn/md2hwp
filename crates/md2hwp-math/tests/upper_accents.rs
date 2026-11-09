// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::{hml_to_tex, tex_to_hml};

#[test]
fn upper_decorations_keep_bodies_and_outer_scripts() {
    for (command, decoration) in [
        ("overleftarrow", "underover {\"AB\"}^{larrow}"),
        ("Overrightarrow", "underover {\"AB\"}^{RARROW}"),
        ("overleftharpoon", "underover {\"AB\"}^{\"↼\"}"),
        ("overrightharpoon", "underover {\"AB\"}^{\"⇀\"}"),
        ("overlinesegment", "underover {\"AB\"}^{\"─\"}"),
        ("overgroup", "arch {\"AB\"}"),
        ("widehat", "hat {\"AB\"}"),
        ("widetilde", "tilde {\"AB\"}"),
        ("widecheck", "check {\"AB\"}"),
        ("overrightarrow", "vec {\"AB\"}"),
        ("overleftrightarrow", "dyad {\"AB\"}"),
    ] {
        for (scripts, expected_scripts) in [
            ("", ""),
            ("^2", " ^{2}"),
            ("_1", " _{1}"),
            ("_1^2", " _{1} ^{2}"),
        ] {
            let input = format!(r"\{command}{{AB}}{scripts}");
            let native = tex_to_hml(&input).unwrap();
            assert_eq!(
                native,
                format!("{{{decoration}}}{expected_scripts}"),
                "{input}"
            );
            let exported = hml_to_tex(&native).unwrap();
            assert!(tex_to_hml(&exported).is_ok(), "{input}: {exported}");
        }
        assert!(tex_to_hml(&format!(r"\{command}")).is_err(), "{command}");
    }
}

#[test]
fn original_upper_accent_fixtures_convert_and_export() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/katex/cases.json")).unwrap();
    for id in ["katex-106", "katex-107"] {
        let case = fixture["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|case| case["id"] == id)
            .unwrap();
        let native = tex_to_hml(case["tex"].as_str().unwrap())
            .unwrap_or_else(|error| panic!("{id}: {error}"));
        let exported = hml_to_tex(&native).unwrap_or_else(|error| panic!("{id}: {error}"));
        assert!(tex_to_hml(&exported).is_ok(), "{id}: {exported}");
    }
}
