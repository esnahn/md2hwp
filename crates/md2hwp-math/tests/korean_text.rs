// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::tex_to_hml;

#[test]
fn plain_hangul_runs_are_unquoted_whole_words() {
    assert_eq!(tex_to_hml("한글").unwrap(), "한글");
    assert_eq!(
        tex_to_hml(r"유리수=\frac{분자}{분모}").unwrap(),
        "유리수 = {{분자} over {분모}}"
    );
}

#[test]
fn hangul_words_inside_braced_fraction_arguments_stay_whole() {
    assert_eq!(
        tex_to_hml(r"\frac{분자}{분모}").unwrap(),
        "{{분자} over {분모}}"
    );
}

#[test]
fn unbraced_hangul_arguments_consume_one_scalar_each() {
    assert_eq!(tex_to_hml(r"\frac가나").unwrap(), "{{가} over {나}}");
    assert_eq!(tex_to_hml(r"\sqrt한글").unwrap(), "{sqrt {한}} 글");
}

#[test]
fn spaces_and_script_boundaries_keep_hangul_atoms_separate() {
    assert_eq!(tex_to_hml("한 글").unwrap(), "한 글");
    // TeX scripts attach to the preceding scalar, so a word run is not merged
    // across this boundary.
    assert_eq!(tex_to_hml("한글^2").unwrap(), "한 글 ^{2}");
}

#[test]
fn hangul_font_scopes_and_explicit_text_keep_their_existing_wrappers() {
    assert_eq!(tex_to_hml(r"\mathrm{한글}").unwrap(), "{rm {한글} it}");
    assert_eq!(
        tex_to_hml(r"\text{한글 공백}").unwrap(),
        "{rm \"한글 공백\" it}"
    );
}

#[test]
fn braced_hangul_overset_base_merges_as_a_single_word() {
    assert_eq!(
        tex_to_hml(r"\overset{a}{한글}").unwrap(),
        "{underover {한글}^{a}}"
    );
}

#[test]
fn ascii_letters_adjacent_to_hangul_do_not_become_a_tex_command() {
    let actual = tex_to_hml("한글over분모").unwrap();
    assert_eq!(actual, "한글 \"over\" 분모");
    assert!(
        !actual.contains(" over "),
        "unexpected OVER constructor: {actual}"
    );
}

#[test]
fn non_hangul_unicode_keeps_its_quoted_representation() {
    assert_eq!(tex_to_hml("α").unwrap(), "\"α\"");
}
