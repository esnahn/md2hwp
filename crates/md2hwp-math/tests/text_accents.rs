// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::{hml_to_tex, tex_to_hml};

#[test]
fn katex_text_accent_array_accepts_omitted_trailing_cells() {
    let fixtures: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/katex/cases.json")).unwrap();
    let tex = fixtures["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "katex-002")
        .unwrap()["tex"]
        .as_str()
        .unwrap();
    let native = tex_to_hml(tex).unwrap();
    assert!(!native.contains("col {"));
    assert!(native.contains(" & "));
    assert!(native.contains("\"ı\" \"˝\""));
    assert!(native.contains("\"ı\" \"˚\""));
}

#[test]
fn text_accents_use_native_commands_and_restore_font() {
    for (command, letter) in [(r"\text{\i}", "ı"), (r"\text{\j}", "ȷ")] {
        let native = tex_to_hml(command).unwrap();
        assert_eq!(
            hml_to_tex(&native).unwrap(),
            format!("\\mathrm{{{letter}}}")
        );
    }
    for (tex, native) in [
        (r"\text{\'a}", r#"{rm {acute {"a"}} it}"#),
        (r"\text{\'\i}", "{rm {acute {\"ı\"}} it}"),
        (r"\text{\.\j}", "{rm {dot {\"ȷ\"}} it}"),
        (r#"\text{\"{a}}"#, r#"{rm {ddot {"a"}} it}"#),
        (r"\text{\`A}", r#"{rm {grave {"A"}} it}"#),
        (r"\text{\H a}", r#"{rm {"a" "˝"} it}"#),
        (r"\text{\r a}", r#"{rm {"a" "˚"} it}"#),
    ] {
        let actual = tex_to_hml(tex).unwrap();
        assert_eq!(actual, native);
        assert!(hml_to_tex(&actual).is_ok(), "{actual}");
    }
    for tex in [
        r"\text{a\AA}",
        r"\text{\AA a}",
        r"\text{\AA\AA}",
        r"\text{\textreferencemark{}}",
    ] {
        assert!(tex_to_hml(tex).is_ok(), "{tex}");
    }
    for tex in [r"\text{\input{file}}", r"\text{\' }"] {
        assert!(tex_to_hml(tex).is_err(), "{tex}");
    }
}
