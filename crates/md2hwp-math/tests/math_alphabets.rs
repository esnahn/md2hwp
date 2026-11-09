// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::{hml_to_tex, tex_to_hml};

#[test]
fn font_scopes_preserve_constructors_and_restore_outer_alphabets() {
    let mono = tex_to_hml(r"\mathtt{a\mathnormal{x}b}").unwrap();
    assert!(mono.contains('𝚊') && mono.contains('𝚋'));
    assert!(!mono.contains('𝚡'), "{mono}");
    let native = tex_to_hml(r"\mathfrak{A\mathrm{x}B}x").unwrap();
    assert!(native.contains('𝔄') && native.contains('𝔅'));
    assert!(!native.contains('𝔵'), "{native}");
    for tex in [
        r"\mathbb{Ax2k\breve{a}\omega\Omega\imath}",
        r"\mathcal{Ax2k\breve{a}\omega\Omega\imath}",
        r"\mathfrak{Ax2k\breve{a}\omega\Omega\imath}",
        r"\mathnormal{Ax2k\breve{a}\omega\Omega\imath}",
        r"\frac{\mathfrak{x}}{\mathbb{k}}",
        r"\mathnormal{\mathcal{A}x}",
    ] {
        let script = tex_to_hml(tex).unwrap();
        let exported = hml_to_tex(&script).unwrap();
        assert!(tex_to_hml(&exported).is_ok(), "{tex}: {exported}");
    }
    assert!(tex_to_hml(r"\mathfrak{\unknown}").is_err());
}

#[test]
fn styled_letters_keep_their_unicode_identity_on_export() {
    for (tex, expected) in [
        (r"\mathfrak{x}", r"\mathfrak{x}"),
        (r"\mathbb{k}", r"\mathbb{k}"),
    ] {
        assert!(
            hml_to_tex(&tex_to_hml(tex).unwrap())
                .unwrap()
                .contains(expected)
        );
    }
    let script = tex_to_hml(r"\mathfrak{\breve{a}}").unwrap();
    assert!(script.contains('𝔞') && script.contains('˘'));
    assert!(!script.contains('ă'));
}

#[test]
fn all_font_examples_through_066_convert_and_export() {
    let f: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/katex/cases.json")).unwrap();
    for c in f["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| (60..=66).contains(&c["id"].as_str().unwrap()[6..].parse::<u32>().unwrap()))
    {
        let native = tex_to_hml(c["tex"].as_str().unwrap()).unwrap();
        assert!(hml_to_tex(&native).is_ok(), "{}", c["id"]);
    }
}
