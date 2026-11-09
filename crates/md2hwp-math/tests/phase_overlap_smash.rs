// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::{hml_to_tex, tex_to_hml};

#[test]
fn phase_keeps_angle_and_argument_but_omits_long_rule() {
    let native = tex_to_hml(r"\phase{-78.2^\circ}").unwrap();
    assert!(native.starts_with("ANGLE {"), "{native}");
    assert!(!native.contains("under"), "{native}");
    assert!(hml_to_tex(&native).unwrap().contains(r"\angle"));
    assert!(tex_to_hml(r"\phase\unknown").is_err());
}

#[test]
fn lap_and_smash_preserve_contents_without_layout_side_effects() {
    for cmd in [
        "mathllap", "mathrlap", "mathclap", "llap", "rlap", "clap", "smash",
    ] {
        let native = tex_to_hml(&format!(r"\{cmd}{{\frac{{a}}{{b}}}}^2+c")).unwrap();
        assert!(native.contains("over"), "{cmd}: {native}");
        assert!(native.ends_with("^{2} + c"), "{cmd}: {native}");
        assert!(hml_to_tex(&native).is_ok(), "{cmd}: {native}");
        assert!(tex_to_hml(&format!(r"\{cmd}{{\unknown}}")).is_err());
    }
    assert_eq!(
        tex_to_hml(r"\smash[b]{y=}").unwrap(),
        tex_to_hml(r"\smash[t]{y=}").unwrap()
    );
    assert_eq!(tex_to_hml(r"\smash 2").unwrap(), "{2}");
    for tex in [r"\smash[z]{x}", r"\smash[b", r"\mathrlap", r"\phantom{x}"] {
        assert!(tex_to_hml(tex).is_err(), "{tex}");
    }
}
