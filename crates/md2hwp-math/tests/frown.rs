// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::{hml_to_tex, tex_to_hml};

#[test]
fn frown_is_only_an_overset_arc_argument() {
    for input in [
        r"\overset{\frown}{AB}",
        r"\overset{{{ \frown }}}{AB}",
        r"\overset\frown{AB}",
    ] {
        let script = tex_to_hml(input).unwrap();
        assert_eq!(script, "{arch {\"AB\"}}");
        assert!(hml_to_tex(&script).is_ok());
    }
    for input in [
        r"\frown",
        r"a \frown b",
        r"\underset{\frown}{a}",
        r"\overset{a \frown}{b}",
        r"\overset{\frown a}{b}",
        r"\overset{\frown}{\underset{x}{a}}",
    ] {
        assert!(tex_to_hml(input).is_err(), "accepted {input}");
    }
}

#[test]
fn literal_frown_text_does_not_become_an_arc() {
    assert_eq!(
        tex_to_hml(r"\overset{FROWN}{a}").unwrap(),
        "{underover {a}^{\"FROWN\"}}"
    );
    assert_eq!(
        tex_to_hml(r"\overset{\text{FROWN}}{a}").unwrap(),
        "{underover {a}^{{rm \"FROWN\" it}}}"
    );
    assert_eq!(
        tex_to_hml(r"\overset{b}{a}").unwrap(),
        "{underover {a}^{b}}"
    );
}
