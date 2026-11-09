// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::{hml_to_tex, tex_to_hml};

#[test]
fn explicit_integral_limits_use_underover_and_keep_both_scripts() {
    for (tex, native) in [
        ("int", "int"),
        ("oint", "oint"),
        ("iint", "dint"),
        ("iiint", "tint"),
        ("oiint", "odint"),
        ("oiiint", "otint"),
    ] {
        let explicit = format!(r"\{tex}\limits^x_{{y+4-a}}");
        let script = tex_to_hml(&explicit).unwrap();
        assert_eq!(
            script,
            format!("underover {{{native}}} ^{{x}} _{{y + 4 - a}}")
        );
        assert_eq!(
            script,
            tex_to_hml(&format!(r"\mathop{{\{tex}}}\limits^x_{{y+4-a}}")).unwrap()
        );
        assert!(hml_to_tex(&script).is_ok());
        assert_eq!(
            tex_to_hml(&format!(r"\{tex}_0^1")).unwrap(),
            format!("{native} _{{0}} ^{{1}}")
        );
    }
}

#[test]
fn integral_limits_work_inside_annotations_and_arrays() {
    for tex in [
        r"\overset{a}{\int\limits_0^1}",
        r"\begin{matrix}\oint\limits_0^1&x\end{matrix}",
    ] {
        assert!(tex_to_hml(tex).unwrap().contains("underover"));
    }
    for tex in [
        r"x\limits_0",
        r"\limits_0",
        r"\int\limitsbad",
        r"\int\limits_0_1",
    ] {
        assert!(tex_to_hml(tex).is_err(), "{tex}");
    }
}

#[test]
fn original_integrands_fixture_converts() {
    let f: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/katex/cases.json")).unwrap();
    let case = f["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "katex-043")
        .unwrap();
    assert!(tex_to_hml(case["tex"].as_str().unwrap()).is_ok());
}
