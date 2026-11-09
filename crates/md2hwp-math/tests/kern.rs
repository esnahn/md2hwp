// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::tex_to_hml;

#[test]
fn kern_normalizes_positive_lengths_and_omits_negative_lengths() {
    for (length, expected) in [
        ("1em", "a ~~ b"),
        ("1ex", "a ~ b"),
        ("1.25pt", "a ` b"),
        ("0pt", "a b"),
        ("-0.25em", "a b"),
    ] {
        for tex in [format!(r"a\kern{{{length}}}b"), format!(r"a\kern{length}b")] {
            assert_eq!(tex_to_hml(&tex).unwrap(), expected, "{tex}");
        }
    }
    assert_eq!(
        tex_to_hml(r"\frac{a\kern{1em}b}{c}").unwrap(),
        "{{a ~~ b} over {c}}"
    );
}

#[test]
fn kern_rejects_invalid_or_excessive_dimensions() {
    for tex in [
        r"\kern{}",
        r"\kern{NaNem}",
        r"\kern{1e2pt}",
        r"\kern{1px}",
        r"\kern{1..2em}",
        r"\kern{5em}",
        r"\kern{-5em}",
        r"\kern",
        r"\kern{1em",
    ] {
        assert!(tex_to_hml(tex).is_err(), "{tex}");
    }
}

#[test]
fn original_katex_kern_fixture_converts() {
    let fixtures: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/katex/cases.json")).unwrap();
    let case = fixtures["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "katex-045")
        .unwrap();
    assert!(tex_to_hml(case["tex"].as_str().unwrap()).is_ok());
}

#[test]
fn horizontal_spaces_share_kern_approximation_with_or_without_star() {
    for command in ["hspace", "hspace*"] {
        for length in ["1em", "1ex", "0pt", "-0.25em"] {
            assert_eq!(
                tex_to_hml(&format!(r"a\{command}{{{length}}}b")).unwrap(),
                tex_to_hml(&format!(r"a\kern{{{length}}}b")).unwrap()
            );
        }
        for length in ["", "bad", "1px", "5em"] {
            assert!(tex_to_hml(&format!(r"a\{command}{{{length}}}b")).is_err());
        }
    }
    let fixtures: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/katex/cases.json")).unwrap();
    let case = fixtures["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "katex-054")
        .unwrap();
    let script = tex_to_hml(case["tex"].as_str().unwrap()).unwrap();
    assert!(!script.contains('#'));
    assert!(md2hwp_math::hml_to_tex(&script).is_ok());
}
