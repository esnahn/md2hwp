// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::{hml_to_tex, tex_to_hml};

#[test]
fn operator_nolimits_uses_grouped_native_base_for_side_scripts() {
    for (tex, native) in [
        ("sum", "sum"),
        ("prod", "prod"),
        ("coprod", "coprod"),
        ("int", "int"),
        ("iint", "dint"),
        ("lim", "lim"),
        ("bigcup", "union"),
    ] {
        let script = tex_to_hml(&format!(r"\{tex}\nolimits^n_{{i=1}}")).unwrap();
        assert_eq!(script, format!("{{{native}}} ^{{n}} _{{i = 1}}"));
        let exported = hml_to_tex(&script).unwrap();
        assert!(!exported.contains(r"\nolimits"), "{exported}");
        assert!(exported.starts_with('{'), "{exported}");
        let roundtrip = tex_to_hml(&exported).unwrap();
        assert_eq!(roundtrip, format!("{{{native}}} _{{i = 1}} ^{{n}}"));
    }
    for tex in [
        r"x\nolimits_i",
        r"\nolimits_i",
        r"\sum\nolimitsbad_i",
        r"\sum\nolimits_i_i",
    ] {
        assert!(tex_to_hml(tex).is_err(), "{tex}");
    }
}

#[test]
fn original_limit_controls_fixture_converts() {
    let f: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/katex/cases.json")).unwrap();
    let case = f["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "katex-053")
        .unwrap();
    let script = tex_to_hml(case["tex"].as_str().unwrap()).unwrap();
    assert!(script.contains("{sum} ^{n} _{i = 1}"));
    assert!(script.contains("underover {int}"));
}
