// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::{hml_to_tex, tex_to_hml};

const CHOICE: &str = r"\mathchoice{D}{T}{S}{SS}";

// Uppercase markers distinguish selected branches without depending on native
// scale decorations, grouping or whitespace.
fn assert_markers(tex: &str, expected: &str) {
    let native = tex_to_hml(tex).unwrap_or_else(|err| panic!("{tex}: {err:?}"));
    let markers: String = native
        .replace("LEFT", "")
        .replace("RIGHT", "")
        .chars()
        .filter(char::is_ascii_uppercase)
        .collect();
    assert_eq!(markers, expected, "{tex}: {native}");
    let exported = hml_to_tex(&native).unwrap();
    assert!(!exported.contains("mathchoice"), "{exported}");
}

#[test]
fn standalone_defaults_to_text_and_explicit_styles_select_each_branch() {
    assert_markers(CHOICE, "T");
    for (style, expected) in [
        ("displaystyle", "D"),
        ("textstyle", "T"),
        ("scriptstyle", "S"),
        ("scriptscriptstyle", "SS"),
    ] {
        assert_markers(&format!(r"\{style} {CHOICE}"), expected);
    }
}

#[test]
fn fixture_059_selects_styles_and_nested_script_styles() {
    assert_markers(
        &format!(
            r"{{\displaystyle {CHOICE}}} {{\textstyle {CHOICE}}} {{\scriptstyle {CHOICE}}} {{\scriptscriptstyle {CHOICE}}} X_{{{CHOICE}_{{{CHOICE}}}}}"
        ),
        "DTSSSXSSS",
    );
}

#[test]
fn groups_restore_styles_and_named_sizes_do_not_change_them() {
    assert_markers(
        &format!(r"{CHOICE} {{\displaystyle {CHOICE}}} {CHOICE}"),
        "TDT",
    );
    for size in [
        "tiny",
        "scriptsize",
        "footnotesize",
        "small",
        "normalsize",
        "large",
        "Large",
        "LARGE",
        "huge",
        "Huge",
    ] {
        assert_markers(&format!(r"\{size} {CHOICE}"), "T");
        assert_markers(&format!(r"\displaystyle \{size} {CHOICE}"), "D");
    }
}

#[test]
fn subscript_and_superscript_styles_step_down_and_saturate() {
    for (style, expected) in [
        ("displaystyle", "XSSSSSS"),
        ("textstyle", "XSSSSSS"),
        ("scriptstyle", "XSSSSSSSS"),
        ("scriptscriptstyle", "XSSSSSSSS"),
    ] {
        assert_markers(
            &format!(r"\{style} X_{{{CHOICE}_{{{CHOICE}}}}}^{{{CHOICE}^{{{CHOICE}}}}}"),
            expected,
        );
    }
}

#[test]
fn fractions_use_child_styles_in_both_operands() {
    for (style, expected) in [
        ("displaystyle", "TT"),
        ("textstyle", "SS"),
        ("scriptstyle", "SSSS"),
        ("scriptscriptstyle", "SSSS"),
    ] {
        assert_markers(
            &format!(r"\{style} \frac{{{CHOICE}}}{{{CHOICE}}}"),
            expected,
        );
        assert_markers(&format!(r"\{style} \dfrac{{{CHOICE}}}{{{CHOICE}}}"), "TT");
        assert_markers(&format!(r"\{style} \tfrac{{{CHOICE}}}{{{CHOICE}}}"), "SS");
    }
    for (style, expected) in [(0, "TT"), (1, "SS"), (2, "SSSS"), (3, "SSSS")] {
        assert_markers(
            &format!(r"\genfrac{{}}{{}}{{}}{{{style}}}{{{CHOICE}}}{{{CHOICE}}}"),
            expected,
        );
    }
    assert_markers(&format!(r"\displaystyle {{{CHOICE}\over {CHOICE}}}"), "TT");
    assert_markers(&format!(r"\textstyle {{{CHOICE}\over {CHOICE}}}"), "SS");
}

#[test]
fn radical_body_inherits_style_and_index_uses_scriptscript() {
    for (style, expected) in [
        ("displaystyle", "SSD"),
        ("textstyle", "SST"),
        ("scriptstyle", "SSS"),
        ("scriptscriptstyle", "SSSS"),
    ] {
        assert_markers(&format!(r"\{style} \sqrt[{CHOICE}]{{{CHOICE}}}"), expected);
    }
}

#[test]
fn lookahead_respects_nested_math_lists_cells_and_fences() {
    assert_markers(&format!(r"{CHOICE}+\sqrt[a\over b]{{x}}"), "T");
    assert_markers(&format!(r"{CHOICE}+\sqrt[\sqrt[a\over b]{{x}}]{{x}}"), "T");
    assert_markers(&format!(r"{CHOICE}+{{{CHOICE}\over {CHOICE}}}"), "TSS");
    assert_markers(
        &format!(r"\left({CHOICE}\right)+{CHOICE}\over {CHOICE}"),
        "SSS",
    );
    assert_markers(
        &format!(r"\begin{{matrix}}{CHOICE}&{CHOICE}\over {CHOICE}\end{{matrix}}"),
        "TSS",
    );
    assert_markers(
        &format!(r"\begin{{smallmatrix}}{CHOICE}\end{{smallmatrix}}{CHOICE}"),
        "ST",
    );
    assert_markers(
        &format!(r"\begin{{aligned}}{CHOICE}&{CHOICE}\end{{aligned}}{CHOICE}"),
        "DDT",
    );
    assert_markers(&format!(r"\overset{{{CHOICE}}}{{{CHOICE}}}"), "TS");
    assert_markers(&format!(r"\overset{{x}}{{{CHOICE}\over {CHOICE}}}"), "SS");
    assert_markers(&format!(r"{{\displaystyle {CHOICE}\over {CHOICE}}}"), "DS");
}

#[test]
fn nested_choices_emit_only_selected_material_and_do_not_leak_style() {
    assert_markers(r"\mathchoice{D}{\mathchoice{D}{T}{S}{SS}}{S}{SS}", "T");
    assert_markers(
        &format!(r"\mathchoice{{D}}{{\displaystyle X}}{{S}}{{SS}}{CHOICE}"),
        "XT",
    );
}

#[test]
fn all_branches_must_be_supported_and_well_formed() {
    for tex in [
        r"\mathchoice D T S Q",
        r"\mathchoice",
        r"\mathchoice{D}{T}{S}",
        r"\mathchoice{D}{T}{S}{",
        r"\mathchoice{\unsupported}{T}{S}{SS}",
        r"\mathchoice{D}{T}{\unsupported}{SS}",
        r"\mathchoice{D}{T}{S}{\frac{x}}",
    ] {
        assert!(tex_to_hml(tex).is_err(), "accepted {tex}");
    }
}

#[test]
fn unselected_branches_still_count_toward_resource_limits() {
    let nested = format!("{}D{}", "{".repeat(100), "}".repeat(100));
    let atoms = "a+".repeat(32_768);
    let bytes = " ".repeat(262_145);
    for branch in [nested, atoms, bytes] {
        let tex = format!(r"\mathchoice{{{branch}}}{{T}}{{S}}{{SS}}");
        assert!(
            tex_to_hml(&tex).is_err(),
            "accepted oversized unselected branch"
        );
    }
}
