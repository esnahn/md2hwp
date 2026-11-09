// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::{hml_to_tex, tex_to_hml};

#[test]
fn native_big_symbols_use_math_operators_without_text_size_commands() {
    for (native, tex, scalar) in [
        ("BIGSQCAP", r"\sqcap", "SQCAP"),
        ("BIGOMINUS", r"\ominus", "OMINUS"),
        ("BIGODIV", r"\oslash", "ODIV"),
    ] {
        let exported = hml_to_tex(&format!("{native} _{{a}}^{{b}}")).unwrap();
        assert_eq!(exported, format!(r"\mathop{{{tex}}}\limits_{{a}}^{{b}}"));
        assert_eq!(
            tex_to_hml(&exported).unwrap(),
            format!("{native} _{{a}} ^{{b}}")
        );
        assert_eq!(tex_to_hml(tex).unwrap(), scalar);
        assert_eq!(hml_to_tex(scalar).unwrap(), tex);
        let sized = tex_to_hml(&format!(r"\mathop{{\Large{tex}}}\limits_{{a}}^{{b}}")).unwrap();
        assert!(sized.contains("scale 144"));
        assert!(sized.starts_with("underover"));
        assert!(!sized.contains(native));
    }
}

#[test]
fn promoting_a_symbol_does_not_promote_text_with_the_same_letters() {
    assert_eq!(
        tex_to_hml(r"\mathop{\text{SQCAP}}\limits_a^b").unwrap(),
        "underover {{rm \"SQCAP\" it}} _{a} ^{b}"
    );
    assert_eq!(
        tex_to_hml(r"\mathop{\Large x}\limits").unwrap(),
        "underover {scale 144 x scale 100}"
    );
    assert_eq!(tex_to_hml(r"\mathop{\sqcap}").unwrap(), "{SQCAP}");
}
