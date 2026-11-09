// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::{hml_to_tex, tex_to_hml};

#[test]
fn native_blanks_export_as_explicit_tex_spaces_and_roundtrip() {
    for (native, tex) in [("`", "\\,"), ("~", "\\ "), ("~~", "\\quad")] {
        assert_eq!(hml_to_tex(native).unwrap(), tex);
        assert_eq!(tex_to_hml(tex).unwrap(), native);
    }
}

fn normalize_script_whitespace(script: &str) -> String {
    script.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]
fn mkern_quantizes_mu_lengths_to_native_blank_units() {
    let cases = [
        ("0", ""),
        ("1", ""),
        ("1.125", "`"), // Halfway values round up.
        ("2", "`"),
        ("2.25", "`"),
        ("3", "`"),
        ("4", "``"),
        ("4.5", "``"),
        ("6.75", "```"),
        ("9", "~"),
        ("18", "~~"),
        ("36", "~~~~"),
        ("72", "~~~~~~~~"),
    ];

    for (length, blank) in cases {
        let tex = format!(r"a\mkern{length}mu b");
        let actual = tex_to_hml(&tex).unwrap_or_else(|error| panic!("{tex}: {error}"));
        let expected = if blank.is_empty() {
            "a b".to_owned()
        } else {
            format!("a {blank} b")
        };
        assert_eq!(
            normalize_script_whitespace(&actual),
            expected,
            "TeX {tex:?}"
        );
    }
}

#[test]
fn mkern_accepts_mu_only_decimal_spellings() {
    for (tex, expected) in [
        (r"a\mkern+.5mu b", "a b"),
        (r"a\mkern1.mu b", "a b"),
        (r"a\mkern+2mu b", "a ` b"),
        (r"a\mkern 1.125   mu b", "a ` b"),
        ("a\\mkern\t2.25 mu b", "a ` b"),
    ] {
        let actual = tex_to_hml(tex).unwrap();
        assert_eq!(
            normalize_script_whitespace(&actual),
            expected,
            "TeX {tex:?}"
        );
    }
}

#[test]
fn mkern_rejects_negative_nonfinite_exponent_non_mu_and_control_inputs() {
    for tex in [
        r"a\mkern-1mu b",
        r"a\mkern-.5mu b",
        r"a\mkern-0mu b",
        r"a\mkern73mu b",
        r"a\mkern72.0001mu b",
        r"a\mkern1e2mu b",
        r"a\mkernNaNmu b",
        r"a\mkerninfmu b",
        r"a\mkernmu b",
        r"a\mkern.mu b",
        r"a\mkern1..0mu b",
        r"a\mkern2pt b",
        r"a\mkern2MU b",
        r"a\mkern{2mu} b",
        r"a\mkern\quad b",
    ] {
        assert!(tex_to_hml(tex).is_err(), "unexpected success: {tex:?}");
    }
}
