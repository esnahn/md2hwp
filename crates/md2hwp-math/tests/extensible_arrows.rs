// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::{hml_to_tex, tex_to_hml};

#[test]
fn extensible_bidirectional_and_crossed_arrows_use_selected_native_relations() {
    for glyph in ["🣒", "🣓"] {
        assert_eq!(tex_to_hml(glyph).unwrap(), "EXARROW");
    }
    for (command, arrow) in [
        ("xrightarrow", "rarrow"),
        ("xleftarrow", "larrow"),
        ("xRightarrow", "RARROW"),
        ("xLeftarrow", "LARROW"),
        ("xleftrightarrow", "lrarrow"),
        ("xLeftrightarrow", "LRARROW"),
        ("xrightleftharpoons", "EXARROW"),
        ("xhookrightarrow", "hookright"),
        ("xhookleftarrow", "hookleft"),
        ("xtwoheadrightarrow", "↠"),
        ("xtwoheadleftarrow", "↞"),
        ("xrightleftarrows", "EXARROW"),
        ("xrightequilibrium", "EXARROW"),
        ("xleftequilibrium", "EXARROW"),
    ] {
        for (optional, below) in [("", ""), ("[ab]", "ab")] {
            let tex = format!("\\{command}{optional}{{ABC}}");
            let native = tex_to_hml(&tex).unwrap();
            assert_eq!(native, format!("{{rel {arrow} {{\"ABC\"}} {{{below}}}}}"));
            assert!(!native.to_lowercase().contains("buildrel"));
            let exported = hml_to_tex(&native).unwrap();
            let reparsed = tex_to_hml(&exported).unwrap();
            assert!(reparsed.contains(&format!("rel {arrow}")), "{exported}");
            assert!(reparsed.contains("ABC") && reparsed.contains(below));
        }
    }
}

#[test]
fn twohead_relations_preserve_direction_unicode_and_nested_labels() {
    for (command, scalar, glyph) in [
        ("xtwoheadrightarrow", "twoheadrightarrow", "↠"),
        ("xtwoheadleftarrow", "twoheadleftarrow", "↞"),
    ] {
        let native = tex_to_hml(&format!("\\{command}[{{[a]}}]{{\\frac{{A}}{{B}}}}")).unwrap();
        assert!(native.contains(&format!("rel {glyph}")));
        let exported = hml_to_tex(&native).unwrap();
        assert!(
            exported.starts_with(&format!("\\{command}[{{")),
            "{exported}"
        );
        assert!(tex_to_hml(&exported).is_ok());
        assert_eq!(tex_to_hml(&format!("\\{scalar}")).unwrap(), glyph);
        assert_eq!(tex_to_hml(glyph).unwrap(), glyph);
        assert_eq!(hml_to_tex(glyph).unwrap(), format!("\\{scalar}"));
        assert_eq!(
            hml_to_tex(&format!("\"{glyph}\"")).unwrap(),
            format!("\\{scalar}")
        );
        assert!(
            tex_to_hml(&format!("\\overset{{A}}{{\\{scalar}}}"))
                .unwrap()
                .contains(&format!("rel {glyph}"))
        );
    }
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/katex/cases.json")).unwrap();
    let tex = fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "katex-031")
        .unwrap()["tex"]
        .as_str()
        .unwrap();
    let native = tex_to_hml(tex).unwrap();
    assert!(native.contains("rel hookright") && native.contains("rel ↠"));
    assert!(hml_to_tex(&native).is_ok(), "{native}");
}

#[test]
fn hooked_relation_roundtrips_complex_labels_and_preserves_the_scalar_hook() {
    for (command, scalar, native_name) in [
        ("xhookrightarrow", "hookrightarrow", "hookright"),
        ("xhookleftarrow", "hookleftarrow", "hookleft"),
    ] {
        let native = tex_to_hml(&format!("\\{command}[{{[a]}}]{{\\frac{{A}}{{B}}}}")).unwrap();
        assert!(native.contains(&format!("rel {native_name} {{{{{{A}} over {{B}}}}}}")));
        let exported = hml_to_tex(&native).unwrap();
        assert!(
            exported.starts_with(&format!("\\{command}[{{")),
            "{exported}"
        );
        assert!(tex_to_hml(&exported).is_ok(), "{exported}");
        assert_eq!(tex_to_hml(&format!("\\{scalar}")).unwrap(), native_name);
        let above = tex_to_hml(&format!("\\overset{{ABC}}{{\\{scalar}}}")).unwrap();
        assert!(above.contains(&format!("rel {native_name}")));
        assert!(tex_to_hml(&format!("\\{command}[ab]")).is_err());
    }
}

#[test]
fn labels_preserve_nested_fractions_and_brackets_inside_larger_expressions() {
    let native =
        tex_to_hml(r"\frac{\xleftrightarrow[ab]{\frac{A}{B}}}{\xLeftrightarrow[C]{D}}").unwrap();
    assert!(native.contains("rel lrarrow {{{A} over {B}}} {ab}"));
    assert!(native.contains("rel LRARROW {\"D\"} {\"C\"}"));
    assert!(hml_to_tex(&native).is_ok());
    assert!(tex_to_hml(r"\xleftrightarrow[ab]{ABC").is_err());
}
