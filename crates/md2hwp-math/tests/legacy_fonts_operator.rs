// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::tex_to_hml;

#[test]
fn legacy_font_and_operator_examples_convert() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/katex/cases.json")).unwrap();
    for id in ["katex-083", "katex-084"] {
        let case = cases["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|case| case["id"] == id)
            .unwrap();
        let tex = case["tex"].as_str().unwrap();
        tex_to_hml(tex).unwrap_or_else(|err| panic!("{id}: {err}"));
    }
}

#[test]
fn legacy_fonts_restore_and_operator_limits_are_not_duplicated() {
    for tex in [
        r"\rm rm & it & \it it & \bf bf & \sf sf & \tt tt",
        r"\text{\rm rm} + \text{\it it} + \text{\bf bf} + \text{\sf sf} + \text{\tt tt}",
        r"i\rm r\it i + \text{r\it i\rm r}",
    ] {
        let tex = tex.replace('&', "+");
        assert!(tex_to_hml(&tex).is_ok(), "{tex}");
    }
    assert_eq!(tex_to_hml(r"{\rm a}b").unwrap(), "{rm a it} b");
    assert_eq!(
        tex_to_hml(r"\operatorname g").unwrap(),
        tex_to_hml(r"\operatorname{g}").unwrap()
    );
    assert_eq!(
        tex_to_hml(r"\operatorname*{asin}\limits_y").unwrap(),
        tex_to_hml(r"\displaystyle \operatorname*{asin}_y").unwrap()
    );
    assert!(tex_to_hml(r"\operatorname{\Gamma}x+\operatorname{}x").is_ok());
}

#[test]
fn operator_names_validate_tokens_and_empty_scripts() {
    for tex in [
        r"\operatorname{\GammaUnknown}",
        r"\operatorname^x",
        r"\operatorname_x",
        r"\operatorname&",
        r"\operatorname{}^x",
        r"\operatorname{\frac}",
    ] {
        assert!(tex_to_hml(tex).is_err(), "{tex}");
    }
    assert_eq!(
        tex_to_hml(r"\operatorname{\Gamma}x").unwrap(),
        tex_to_hml(r"\operatorname Γx").unwrap()
    );
    assert_eq!(
        tex_to_hml(r"\operatorname{foo}\limits_y").unwrap(),
        tex_to_hml(r"\displaystyle \operatorname*{foo}_y").unwrap()
    );
    assert_eq!(
        tex_to_hml(r"\operatorname{}x").unwrap(),
        tex_to_hml("x").unwrap()
    );
    assert_eq!(tex_to_hml(r"\operatorname{   }x").unwrap(), "x");
    let bold = tex_to_hml(r"\text{a\bf bold\rm normal}").unwrap();
    assert!(bold.contains("bold {\"bold\"}"), "{bold}");
    assert!(!bold.contains(" bf "), "{bold}");
}

#[test]
fn legacy_math_bold_covers_the_whole_declaration_scope() {
    use md2hwp_math::hml_to_tex;
    let native = tex_to_hml(r"\bf a+b\rm c").unwrap();
    assert!(native.contains("bold {a + b}"), "{native}");
    let tex = hml_to_tex(&native).unwrap();
    assert!(
        tex.contains(r"\mathbf{a}") && tex.contains(r"\mathbf{b}"),
        "{tex}"
    );
}
