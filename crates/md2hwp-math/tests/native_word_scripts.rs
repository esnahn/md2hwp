// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::hml_to_tex;

#[test]
fn ungrouped_letter_words_script_the_last_letter() {
    for (native, tex) in [
        ("E=mc^2", "E = mc^{2}"),
        ("abc_i^2", "abc_{i}^{2}"),
        ("abc^2_i", "abc_{i}^{2}"),
        ("ab sub i sup {n+1}", "ab_{i}^{n + 1}"),
        ("mc^{a+b}+ab_i", "mc^{a + b} + ab_{i}"),
        ("mc^2 over y", r"\frac{mc^{2}}{y}"),
        ("a over mc^2", r"\frac{a}{mc^{2}}"),
    ] {
        assert_eq!(hml_to_tex(native).unwrap(), tex, "{native}");
    }
}

#[test]
fn explicit_groups_numbers_quoted_words_and_hangul_keep_their_scope() {
    for (native, tex) in [
        ("{mc}^2", "{mc}^{2}"),
        ("{a+b}^2", "{a + b}^{2}"),
        ("{mc^2}^3", "{mc^{2}}^{3}"),
        ("12^2", "{12}^{2}"),
        ("12.5_i", "{12.5}_{i}"),
        (r#""mc"^2"#, "{mc}^{2}"),
        ("한글^2", "{한글}^{2}"),
        ("mc lsub i ^2", "{}_{i}{mc}^{2}"),
    ] {
        assert_eq!(hml_to_tex(native).unwrap(), tex, "{native}");
    }
}

#[test]
fn font_wrapped_letters_preserve_the_last_letter_script_scope() {
    for (native, tex) in [
        ("rm mc^2", r"\mathrm{m} \mathrm{c}^{2}"),
        ("bold mc_i^2", r"\boldsymbol{m} \boldsymbol{c}_{i}^{2}"),
        ("rm bold mc^2", r"\mathbf{m} \mathbf{c}^{2}"),
        ("rm {mc}^2", r"\mathrm{mc}^{2}"),
        (r#"rm "mc"^2"#, r"\mathrm{mc}^{2}"),
    ] {
        assert_eq!(hml_to_tex(native).unwrap(), tex, "{native}");
    }
}

#[test]
fn compact_and_split_functions_script_their_final_argument_or_operator() {
    for (native, tex) in [
        ("sinx^2", r"\sin x^{2}"),
        ("sinxy_i^2", r"\sin xy_{i}^{2}"),
        ("si nx^2", r"\sin x^{2}"),
        ("si nxy^2", r"\sin x y^{2}"),
        ("sin^2 x", r"\sin^{2} x"),
        ("si n^2 x", r"\sin^{2} x"),
        ("sincos^2 x", r"\sin \cos^{2} x"),
        ("{sinx}^2", r"{\sin x}^{2}"),
        ("rm sinxy^2", r"\sin \mathrm{x} \mathrm{y}^{2}"),
        ("rm si nxy^2", r"\sin \mathrm{x} \mathrm{y}^{2}"),
    ] {
        assert_eq!(hml_to_tex(native).unwrap(), tex, "{native}");
    }
}
