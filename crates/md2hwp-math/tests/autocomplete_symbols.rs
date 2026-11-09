// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::{hml_to_tex, tex_to_hml};

const SYMBOLS: &[(&str, &str)] = &[
    ("ASSERT", r"\vdash"),
    ("BIGVEE", r"\bigvee"),
    ("BIGWEDGE", r"\bigwedge"),
    ("CUP", r"\cup"),
    ("DASHV", r"\dashv"),
    ("del", r"\nabla"),
    ("dline", r"\Vert"),
    ("DOTSAXIS", r"\cdots"),
    ("DOTSDIAG", r"\ddots"),
    ("DOTSLOW", r"\ldots"),
    ("DOTSVERT", r"\vdots"),
    ("GGG", r"\ggg"),
    ("HCROSS", "+"),
    ("HDOWN", r"\top"),
    ("HRIGHT", r"\vdash"),
    ("HUP", r"\bot"),
    ("INFINITY", r"\infty"),
    ("INFTY", r"\infty"),
    ("LAND", r"\wedge"),
    ("MASSERT", r"\dashv"),
    ("MHO", r"\mho"),
    ("NEG", r"\lnot"),
    ("NI", r"\ni"),
    ("NIN", r"\notin"),
    ("NOWNS", r"\not\ni"),
    ("NSUBSET", r"\not\subset"),
    ("NSUBSETEQ", r"\not\subseteq"),
    ("NSUPSET", r"\not\supset"),
    ("NSUPSETEQ", r"\not\supseteq"),
    ("PHOR", "="),
    ("PREP", r"\bot"),
    ("PVER", r"\Vert"),
    ("ROUND", r"\partial"),
    ("SDOTS", "⋰"),
    ("SMALLPROD", r"{\textstyle\prod}"),
    ("SMALLSUM", r"{\textstyle\sum}"),
    ("SMALLCOPROD", r"{\textstyle\coprod}"),
    ("TRIANGLEL", r"\triangleleft"),
    ("TRIANGLER", r"\triangleright"),
];

#[test]
fn verified_native_symbols_roundtrip_semantically() {
    for &(native, expected) in SYMBOLS {
        assert_eq!(hml_to_tex(native).unwrap(), expected, "{native}");
        let canonical = tex_to_hml(expected).unwrap();
        assert_eq!(
            hml_to_tex(&canonical).unwrap(),
            expected,
            "{native}: {canonical}"
        );
        let input = format!("{native} _{{i}}^{{n}} + x");
        let tex = hml_to_tex(&input).unwrap();
        let roundtrip = tex_to_hml(&tex).unwrap();
        assert_eq!(hml_to_tex(&roundtrip).unwrap(), tex, "{input}: {roundtrip}");
        assert_eq!(hml_to_tex(&format!("\"{native}\"")).unwrap(), native);
    }
}

#[test]
fn line_aliases_are_decorations_but_dline_is_an_independent_symbol() {
    for (alias, canonical) in [("overline", "bar"), ("underline", "under")] {
        let tex = hml_to_tex(&format!("{alias} {{a+b}}")).unwrap();
        assert_eq!(tex, hml_to_tex(&format!("{canonical} {{a+b}}")).unwrap());
        assert_eq!(hml_to_tex(&tex_to_hml(&tex).unwrap()).unwrap(), tex);
    }
    assert_eq!(
        hml_to_tex("dline {a}").unwrap(),
        hml_to_tex("VERT {a}").unwrap()
    );
}

#[test]
fn combined_bold_roman_commands_preserve_the_intended_style() {
    for command in ["boldrm", "rmbold"] {
        for prefix in ["", "bold ", "rm bold "] {
            let native = format!("{prefix}{command} abc");
            let tex = hml_to_tex(&native).unwrap();
            assert_eq!(tex, hml_to_tex("rm bold abc").unwrap());
            let reverse = tex_to_hml(&tex).unwrap();
            assert!(reverse.contains("rm bold"), "{reverse}");
            assert_eq!(hml_to_tex(&reverse).unwrap(), tex);
        }
        assert_eq!(
            hml_to_tex(&format!("{command} abc it x")).unwrap(),
            hml_to_tex("rm bold abc it x").unwrap()
        );
        assert_eq!(hml_to_tex(&format!("\"{command}\"")).unwrap(), command);
    }
}
