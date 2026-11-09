// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::{hml_to_tex, tex_to_hml};

#[test]
fn script_and_typewriter_letters_roundtrip_without_losing_the_alphabet() {
    let native = tex_to_hml(r"\mathscr{C}").unwrap();
    assert!(native.contains('𝒞'));
    assert!(hml_to_tex(&native).unwrap().contains(r"\mathscr{C}"));
    for tex in [r"\text{\tt abc123}", r"\texttt{abc123}", r"\mathtt{abc123}"] {
        let native = tex_to_hml(tex).unwrap();
        assert!(native.contains('𝚊') && native.contains('𝟷'), "{native}");
        assert!(
            hml_to_tex(&native).unwrap().contains(r"\mathtt"),
            "{native}"
        );
    }
    assert!(tex_to_hml(r"\mathscr{a+b}").is_ok());
    let native = tex_to_hml(r"\text{a{\tt b}c}").unwrap();
    assert!(native.contains("\"𝚋\"") && native.contains("\"c\""));
}

#[test]
fn set_and_braket_use_native_scalable_delimiters() {
    for (tex, left, right) in [(r"\Set{x|x>0}", "{", "}"), (r"\Braket{x|y}", "<", ">")] {
        let native = tex_to_hml(tex).unwrap();
        assert!(native.contains(&format!("LEFT {left}")));
        assert!(native.contains(&format!("RIGHT {right}")));
        assert!(hml_to_tex(&native).is_ok());
    }
}

#[test]
fn a_color_declaration_includes_the_right_delimiter_but_textcolor_does_not() {
    let native = tex_to_hml(r"\left(black\color{red}red\right)black").unwrap();
    assert!(native.contains("RIGHT )}"), "{native}");
    assert!(native.contains("color {255, 0, 0}"));
    let exported = hml_to_tex(&native).unwrap();
    assert!(exported.contains(r"\color[RGB]{255,0,0}"), "{exported}");
    assert!(tex_to_hml(&exported).is_ok(), "{exported}");
    let native = tex_to_hml(r"\left(x\textcolor{red}{y}\right)").unwrap();
    assert!(native.ends_with("RIGHT )}"));
    assert!(!native.ends_with("RIGHT )}}}"));
}

#[test]
fn arrows_never_generate_buildrel() {
    for tex in [
        r"\xrightarrow{ABC}",
        r"\xrightarrow[ab]{ABC}",
        r"\overset{ABC}{\rightarrow}",
    ] {
        let native = tex_to_hml(tex).unwrap();
        assert!(native.contains("rel rarrow"));
        assert!(!native.to_lowercase().contains("buildrel"));
    }
}
