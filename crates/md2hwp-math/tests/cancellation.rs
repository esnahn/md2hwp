// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::{hml_to_tex, tex_to_hml};

#[test]
fn cancellation_directions_normalize_to_native_not_and_keep_the_body() {
    for name in ["cancel", "bcancel", "xcancel"] {
        for body in [
            "x",
            "2B",
            "5ay",
            "B^2",
            r"\oint_S{\vec E\cdot\hat n\,\mathrm d a}",
        ] {
            let native = tex_to_hml(&format!(r"\{name}{{{body}}}")).unwrap();
            assert!(native.contains("not "), "{native}");
            let tex = hml_to_tex(&native).unwrap();
            assert!(tex_to_hml(&tex).is_ok(), "{tex}");
        }
    }
    assert_eq!(tex_to_hml(r"\cancel{abc}").unwrap(), "{not a bc}");
    assert!(tex_to_hml(r"\cancel{\unknown}").is_err());
}

#[test]
fn integral_negation_uses_unicode_glyphs_that_hancom_slashes() {
    // Hancom draws no NOT slash over integral keywords, only over the glyphs.
    for (command, glyph) in [
        ("int", "∫"),
        ("iint", "∬"),
        ("iiint", "∭"),
        ("oint", "∮"),
        ("oiint", "∯"),
        ("oiiint", "∰"),
    ] {
        for tex in [
            format!(r"\cancel{{\{command} x}}"),
            format!(r"\xcancel{{\{command} x}}"),
            format!(r"\not\{command} x"),
            format!(r"\not{{\{command}}} x"),
            format!(r"\not {glyph} x"),
        ] {
            let native = tex_to_hml(&tex).unwrap();
            assert!(
                native.contains(&format!("not {glyph} x")),
                "{tex}: {native}"
            );
            let exported = hml_to_tex(&native).unwrap();
            assert!(
                exported.contains(&format!(r"\not\{command} x")),
                "{tex}: {exported}"
            );
            assert_eq!(tex_to_hml(&exported).unwrap(), native, "{tex}");
        }
        assert_eq!(
            hml_to_tex(&format!("not {glyph} _{{S}} x")).unwrap(),
            format!(r"\not\{command}_{{S}} x")
        );
    }
    assert_eq!(
        tex_to_hml(r"\xcancel{\oint_S{\vec E\cdot\hat n\,\mathrm d a}}").unwrap(),
        "{not ∮ _{S} {{vec {E}} CDOT {hat {n}} ` {rm {d} it} a}}"
    );
    for keyword in [
        "int", "INT", "oint", "dint", "tint", "odint", "otint", "sum",
    ] {
        assert!(
            hml_to_tex(&format!("not {keyword} x")).is_err(),
            "{keyword}"
        );
    }
    assert!(tex_to_hml(r"\cancel{\sum x}").is_err());
    assert!(tex_to_hml(r"\cancel{\int\limits_a^b x}").is_err());
}

#[test]
fn strikeout_text_preserves_upright_letters_and_does_not_slash_quotes() {
    let native = tex_to_hml(r"\text{\sout{5ABC}}").unwrap();
    assert!(native.contains("not 5 \"ABC\""), "{native}");
    assert!(!native.contains("not \""), "{native}");
    assert!(hml_to_tex(&native).is_ok());
}
