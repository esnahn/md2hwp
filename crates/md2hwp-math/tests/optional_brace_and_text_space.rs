// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::{hml_to_tex, tex_to_hml};

#[test]
fn braces_without_labels_keep_the_following_expression() {
    for (tex, expected) in [
        (r"\underbrace{x}+y", "{UNDERBRACE {} {x}} + y"),
        (r"\overbrace{x}+y", "{OVERBRACE {x} {}} + y"),
    ] {
        let native = tex_to_hml(tex).unwrap();
        assert_eq!(native, expected);
        let exported = hml_to_tex(&native).unwrap();
        assert!(tex_to_hml(&exported).is_ok(), "{exported}");
    }
    assert!(tex_to_hml(r"\underbrace{x}_").is_err());
    assert!(tex_to_hml(r"\underbrace{x}_a_b").is_err());
}

#[test]
fn escaped_text_space_is_preserved_as_native_space() {
    let native = tex_to_hml(r"\text{a\ b}").unwrap();
    assert!(native.contains('~'), "{native}");
    assert!(hml_to_tex(&native).is_ok());
    assert!(tex_to_hml(r"\text{a\unknown b}").is_err());
}
