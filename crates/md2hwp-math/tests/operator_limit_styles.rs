// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::{hml_to_tex, tex_to_hml, tex_to_hml_with_display};

#[test]
fn operator_limits_follow_math_style_and_explicit_controls() {
    let side = tex_to_hml(r"\operatorname*{asin}_y x").unwrap();
    assert!(!side.contains("underover"), "{side}");
    let below = tex_to_hml(r"\operatorname*{asin}\limits_y x").unwrap();
    assert!(below.contains("underover"), "{below}");
    assert_eq!(
        below,
        tex_to_hml(r"\displaystyle \operatorname*{asin}_y x").unwrap()
    );
    assert_eq!(
        side,
        tex_to_hml(r"\displaystyle \operatorname*{asin}\nolimits_y x").unwrap()
    );
    for name in ["lim", "sum", "intop", "limsup", "liminf"] {
        let plain = tex_to_hml(&format!(r"\{name}_a^b")).unwrap();
        let forced_side = tex_to_hml(&format!(r"\displaystyle \{name}\nolimits_a^b")).unwrap();
        assert_eq!(plain, forced_side, "{name}");
        let display = tex_to_hml(&format!(r"\displaystyle \{name}_a^b")).unwrap();
        let forced_below = tex_to_hml(&format!(r"\{name}\limits_a^b")).unwrap();
        assert_eq!(display, forced_below, "{name}");
        assert_ne!(plain, display, "{name}");
    }
    assert_eq!(
        tex_to_hml(r"\int_a^b").unwrap(),
        tex_to_hml(r"\displaystyle \int_a^b").unwrap()
    );
}

#[test]
fn original_operator_fixtures_convert() {
    let fixtures: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/katex/cases.json")).unwrap();
    for id in ["katex-084", "katex-085"] {
        let case = fixtures["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|case| case["id"] == id)
            .unwrap();
        tex_to_hml(case["tex"].as_str().unwrap()).unwrap_or_else(|error| panic!("{id}: {error}"));
    }
}

#[test]
fn native_limits_export_explicit_placement_and_substack_is_bounded() {
    for native in [
        "sum _{a} ^{b}",
        "lim _{a}",
        "Lim _{a}",
        r#"underover {rm "foo" it} _{a}"#,
    ] {
        let tex = hml_to_tex(native).unwrap();
        assert!(tex.contains(r"\limits"), "{native}: {tex}");
        assert_eq!(tex_to_hml(&tex).unwrap(), native);
    }
    let pile = tex_to_hml(r"\substack{{a}\\b}").unwrap();
    assert!(pile.contains("pile"), "{pile}");
    assert!(tex_to_hml(r"\substack{a\\}").is_err());
    assert!(tex_to_hml(&format!(r"\substack{{{}}}", "a\\\\".repeat(20_000))).is_err());
}

#[test]
fn initial_display_style_applies_to_fixture_without_changing_inline_default() {
    let fixtures: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/katex/cases.json")).unwrap();
    let case = fixtures["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["id"] == "katex-026")
        .unwrap();
    assert_eq!(case["options"]["displayMode"], true);
    let tex = case["tex"].as_str().unwrap();
    let display = tex_to_hml_with_display(tex, true).unwrap();
    let inline = tex_to_hml(tex).unwrap();
    assert_ne!(display, inline);
    assert_eq!(inline, tex_to_hml_with_display(tex, false).unwrap());
    assert_eq!(
        display,
        tex_to_hml(&format!(r"\displaystyle {tex}")).unwrap()
    );
    assert_eq!(
        tex_to_hml_with_display(r"\sum_i", true).unwrap(),
        "sum _{i}"
    );
    assert_eq!(
        tex_to_hml_with_display(r"\sum_i", false).unwrap(),
        "{sum} _{i}"
    );
}
