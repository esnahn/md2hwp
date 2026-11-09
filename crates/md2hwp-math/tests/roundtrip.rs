// SPDX-License-Identifier: AGPL-3.0-only
//! TeX emitted by the native-script exporter must remain in the bounded
//! TeX-to-SCRIPT importer's supported subset.
use md2hwp_math::{hml_to_tex, tex_to_hml};
use serde_json::Value;

fn assert_pipeline(script: &str, label: &str) -> String {
    let tex = hml_to_tex(script).unwrap_or_else(|e| panic!("{label}, HML {script:?}: {e}"));
    assert!(!tex.trim().is_empty(), "{label}: empty TeX output");
    let reconstructed = tex_to_hml(&tex)
        .unwrap_or_else(|e| panic!("{label}, TeX {tex:?} from HML {script:?}: {e}"));
    assert!(
        !reconstructed.trim().is_empty(),
        "{label}: empty SCRIPT output for TeX {tex:?}"
    );
    reconstructed
}

fn normalized_script(script: &str) -> String {
    script.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn assert_hml(tex: &str, expected: &str) {
    let actual = tex_to_hml(tex).unwrap_or_else(|e| panic!("TeX {tex:?}: {e}"));
    assert_eq!(
        normalized_script(&actual),
        normalized_script(expected),
        "TeX {tex:?}"
    );
}

#[test]
fn every_toolbar_export_roundtrips_through_tex_to_hml() {
    let toolbar: Value =
        serde_json::from_str(include_str!("fixtures/toolbar-equations.json")).unwrap();
    let toolbar_cases = toolbar["cases"].as_array().unwrap();
    assert_eq!(toolbar_cases.len(), 76);
    for case in toolbar_cases {
        let label = format!("toolbar {} ({})", case["case_id"], case["name"]);
        assert_pipeline(case["hancom_script"].as_str().unwrap(), &label);
    }
}

#[test]
fn aligned_tex_stays_identical_after_eqalign_roundtrip() {
    // Menu example #4: adding the explicit native container must not change
    // the TeX alignment points, row order, or subsequent exports.
    for script in [
        "420 = & 2 TIMES 210# = & 2 TIMES 2 TIMES 105",
        "eqalign{a &= b+c#x+y &= z}",
        "eqalign{a &= b#c &= d#e &= f}",
    ] {
        let expected = hml_to_tex(script).unwrap();
        assert!(expected.starts_with(r"\begin{aligned}"));
        let mut tex = expected.clone();
        for _ in 0..3 {
            let native = tex_to_hml(&tex).unwrap();
            assert!(native.contains("eqalign"));
            tex = hml_to_tex(&native).unwrap();
            assert_eq!(tex, expected, "original SCRIPT: {script}");
        }
    }
}

#[test]
fn inverse_scripts_attach_to_their_own_base_and_preserve_constructor_order() {
    assert_hml(r"{}^{s}a", "a lsup {s}");
    assert_hml(r"{}_{t}a", "a lsub {t}");
    assert_hml(r"\mathop{f}\limits_{b}^{c}", "underover {f} _{b} ^{c}");

    assert_hml(r"\xrightarrow[d]{u}", "{rel rarrow {u} {d}}");
    assert_hml(r"\xleftarrow[d]{u}", "{rel larrow {u} {d}}");
    assert_hml(r"\overbrace{x}^{y}", "{OVERBRACE {x} {y}}");
    // Native UNDERBRACE stores its label first, unlike TeX's body-first form.
    assert_hml(r"\underbrace{x}_{y}", "{UNDERBRACE {y} {x}}");
}

#[test]
fn toolbar_accents_limits_and_delimiter_constructors_have_native_inverses() {
    for (tex, expected) in [
        (r"\overleftrightarrow{x}", "{dyad {x}}"),
        (r"\acute{x}", "{acute {x}}"),
        (r"\grave{x}", "{grave {x}}"),
        (r"\check{x}", "{check {x}}"),
        (r"\overset{\frown}{x}", "{arch {x}}"),
        (r"\boxed{x}", "{box {x}}"),
        (r"\left\{x\right\}", "{LEFT { x RIGHT }}"),
        (r"\left\lbrace x\right\rbrace", "{LEFT { x RIGHT }}"),
        (r"\left\lVert x\right\rVert", "{LEFT ∥ x RIGHT ∥}"),
        (r"\lceil x\rceil", "LCEIL x RCEIL"),
        (r"\lfloor x\rfloor", "LFLOOR x RFLOOR"),
        (r"\operatorname*{Lim}\limits_{x\to0} f", "Lim _{x -> 0} f"),
        (r"\begin{gathered}a\\b\end{gathered}", "{pile {a # b}}"),
    ] {
        assert_hml(tex, expected);
    }
}

#[test]
fn standard_big_operators_and_integrals_keep_their_native_names_and_bounds() {
    for (tex, expected) in [
        (r"\sum\limits_{i=1}^{n}", "sum _{i = 1} ^{n}"),
        (r"\prod\limits_{i=1}^{n}", "prod _{i = 1} ^{n}"),
        (r"\coprod\limits_{i=1}^{n}", "coprod _{i = 1} ^{n}"),
        (r"\bigcap\limits_{i=1}^{n}", "inter _{i = 1} ^{n}"),
        (r"\bigcup\limits_{i=1}^{n}", "union _{i = 1} ^{n}"),
        (
            r"\mathop{\sqcap}\limits_{i=1}^{n}",
            "BIGSQCAP _{i = 1} ^{n}",
        ),
        (
            r"\mathop{\ominus}\limits_{i=1}^{n}",
            "BIGOMINUS _{i = 1} ^{n}",
        ),
        (
            r"\mathop{\oslash}\limits_{i=1}^{n}",
            "BIGODIV _{i = 1} ^{n}",
        ),
        (r"\bigsqcup\limits_{i=1}^{n}", "BIGSQCUP _{i = 1} ^{n}"),
        (r"\bigoplus\limits_{i=1}^{n}", "BIGOPLUS _{i = 1} ^{n}"),
        (r"\bigotimes\limits_{i=1}^{n}", "BIGOTIMES _{i = 1} ^{n}"),
        (r"\bigodot\limits_{i=1}^{n}", "BIGODOT _{i = 1} ^{n}"),
        (r"\biguplus\limits_{i=1}^{n}", "BIGUPLUS _{i = 1} ^{n}"),
        (r"\bigvee\limits_{i=1}^{n}", "BIGVEE _{i = 1} ^{n}"),
        (r"\bigwedge\limits_{i=1}^{n}", "BIGWEDGE _{i = 1} ^{n}"),
        (r"\int_{a}^{b} f", "int _{a} ^{b} f"),
        (r"\iint_{a}^{b} f", "dint _{a} ^{b} f"),
        (r"\iiint_{a}^{b} f", "tint _{a} ^{b} f"),
        (r"\oint_{a}^{b} f", "oint _{a} ^{b} f"),
        (r"\oiint_{a}^{b} f", "odint _{a} ^{b} f"),
        (r"\oiiint_{a}^{b} f", "otint _{a} ^{b} f"),
    ] {
        assert_hml(tex, expected);
    }
}
