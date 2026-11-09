// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::{hml_to_tex, tex_to_hml};

#[test]
fn binary_class_adds_medium_spacing_except_in_scripts() {
    assert_eq!(
        tex_to_hml(r"a\mathbin{\mathrm{mod}}b").unwrap(),
        "a `` {{rm {\"mod\"} it}} `` b"
    );
    let native = tex_to_hml(r"\scriptstyle a\mathbin{x}b").unwrap();
    assert!(!native.contains('`'), "{native}");
}

#[test]
fn grouped_not_letters_keep_native_first_character_overlay() {
    assert_eq!(tex_to_hml(r"\not{abc}").unwrap(), "not abc");
    assert_eq!(hml_to_tex("not abc").unwrap(), r"\not a bc");
    assert_eq!(tex_to_hml(r"\not{ab}").unwrap(), "not ab");
    for input in [r"\not{}", r"\not{a+b}", r"\not{sin}", r"\not{\unknown}"] {
        assert!(tex_to_hml(input).is_err(), "{input}");
    }
}

#[test]
fn omitted_matrix_cells_are_padded_without_changing_existing_cells() {
    assert_eq!(
        tex_to_hml(r"\begin{matrix}a&b\\c\end{matrix}").unwrap(),
        "{matrix {a & b # c & }}"
    );
    assert_eq!(
        tex_to_hml(r"\begin{pmatrix}a\\b&c\end{pmatrix}").unwrap(),
        "{pmatrix {a &  # b & c}}"
    );
}
