// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::{hml_to_tex, tex_to_hml};

#[test]
fn tex_text_quotes_become_visible_quote_glyphs_in_both_text_paths() {
    assert_eq!(
        tex_to_hml(r"\text{``a'' `b'}").unwrap(),
        "{rm \"“a” ‘b’\" it}"
    );
    let native = tex_to_hml(r"\text{\it ``a'' `b'}").unwrap();
    assert!(native.contains("“a” ‘b’"), "{native}");
    assert!(!native.contains('`'));
    assert!(hml_to_tex(&native).is_ok());
    let separated = tex_to_hml(r"\text{`{`}x''}").unwrap();
    assert_eq!(separated.matches('‘').count(), 2);
    assert!(!separated.contains('“'));
}

#[test]
fn text_dash_ligatures_keep_tex_group_boundaries_and_typewriter_exceptions() {
    let native = tex_to_hml(r"\text{a---b -- c----d-{-}-e}").unwrap();
    assert!(native.contains("a—b – c—-d-"), "{native}");
    assert_eq!(native.matches('—').count(), 2);
    assert_eq!(native.matches('–').count(), 1);
    for tex in [r"\text{\tt ``a''---}", r"\texttt{``a''---}"] {
        let native = tex_to_hml(tex).unwrap();
        assert!(native.contains("“𝚊”---"), "{native}");
        assert!(!native.contains("‘‘") && !native.contains("’’"));
        assert!(!native.contains('`'));
        assert!(hml_to_tex(&native).is_ok());
    }
}

#[test]
fn math_backticks_primes_spacing_and_grave_accents_remain_distinct() {
    let native = tex_to_hml("``x''").unwrap();
    assert_eq!(native.matches('“').count(), 1);
    assert_eq!(native.matches('”').count(), 1);
    assert!(!native.contains('`'));
    assert_eq!(tex_to_hml(r"f''\,g").unwrap().matches('`').count(), 1);
    assert_eq!(tex_to_hml("f''").unwrap().matches('\'').count(), 2);
    let native = tex_to_hml(r"\mathtt{``a''}").unwrap();
    assert_eq!(native.matches('“').count(), 1);
    assert_eq!(native.matches('”').count(), 1);
    let literal = tex_to_hml(r"\text{‘‘a’’}").unwrap();
    assert!(literal.contains("‘‘a’’"));
    let annotation = tex_to_hml(r"\overset{a}{``x''}").unwrap();
    assert_eq!(annotation.matches('“').count(), 1);
    assert_eq!(annotation.matches('”').count(), 1);
    assert_eq!(
        tex_to_hml(r"\text{\`a}").unwrap(),
        "{rm {grave {\"a\"}} it}"
    );
}

#[test]
fn named_text_punctuation_uses_the_same_native_glyphs() {
    let native = tex_to_hml(r"\text{\textquotedblleft a\textquotedblright\textquoteleft b\textquoteright\textendash\textemdash}").unwrap();
    for glyph in ['“', '”', '‘', '’', '–', '—'] {
        assert!(native.contains(glyph), "{native}");
    }
    assert!(hml_to_tex(&native).is_ok());
}
