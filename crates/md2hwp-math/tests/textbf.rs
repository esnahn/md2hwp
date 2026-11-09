// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::{hml_to_tex, tex_to_hml};

#[test]
fn top_level_textbf_uses_native_roman_bold() {
    for (tex, native, exported) in [
        (r"\textbf{ab}", r#"{rm bold {"ab"} it}"#, r"\mathbf{ab}"),
        (
            r"\textbf{a b}z",
            r#"{rm bold {"a b"} it} z"#,
            r"\mathbf{a\ b} z",
        ),
        (
            r"\textbf{가 나}",
            r#"{rm bold {"가 나"} it}"#,
            r"\mathbf{가\ 나}",
        ),
        (
            r"\mathrm{x\textbf{y}}",
            r#"{rm {x {rm bold {"y"} rm}} it}"#,
            r"{\mathrm{x} \mathbf{y}}",
        ),
    ] {
        let converted = tex_to_hml(tex).unwrap();
        assert_eq!(converted, native, "{tex}");
        assert!(!converted.contains("bf"), "{tex}: {converted}");
        assert_eq!(hml_to_tex(&converted).unwrap(), exported, "{tex}");
    }
}

#[test]
fn textbf_inside_text_commands_uses_a_local_bold_scope() {
    for command in ["text", "textrm", "textsf", "mbox"] {
        let tex = format!(r"\{command}{{x \textbf{{ab}} y}}");
        let native = tex_to_hml(&tex).unwrap();
        assert_eq!(native, r#"{rm {"x " bold {"ab"} " y"} it}"#, "{tex}");
        assert_eq!(
            hml_to_tex(&native).unwrap(),
            r"{\mathrm{x\ } \mathbf{ab} \mathrm{\ y}}"
        );
    }
    // A nested \textbf matches the existing text-local \bf declaration.
    assert_eq!(
        tex_to_hml(r"\text{x \textbf{ab} y}").unwrap(),
        tex_to_hml(r"\text{x {\bf ab} y}")
            .unwrap()
            .replace(r#"{bold {"ab"}}"#, r#"bold {"ab"}"#)
    );
    assert_eq!(
        tex_to_hml(r"\textbf{a \textbf{b}}").unwrap(),
        r#"{rm bold {"a " bold {"b"}} it}"#
    );
    assert_eq!(
        tex_to_hml(r"\text{\textbf{a \textbf{b}}}").unwrap(),
        r#"{rm {bold {"a " bold {"b"}}} it}"#
    );
}

#[test]
fn textbf_and_textit_require_one_closed_braced_argument() {
    for tex in [
        r"\textbf a",
        r"\textbf{a",
        r"\text{a \textbf b}",
        r"\text{a \textbf{b}",
        r"\text{a \textbf{$b$}}",
        r"\textit a",
        r"\textit{a",
        r"\text{a \textit b}",
        r"\text{\textrm b}",
        r"\text{\textit{$a$}}",
    ] {
        assert!(tex_to_hml(tex).is_err(), "accepted {tex}");
    }
}

#[test]
fn textit_uses_native_italic_and_restores_the_previous_font() {
    for (tex, native, exported) in [
        (
            r"\textit{abc123}",
            r#"{it {"abc123"} it}"#,
            r"\mathit{abc123}",
        ),
        // Letters alone export as default math italic.
        (r"\textit{ab}z", r#"{it {"ab"} it} z"#, r"{ab} z"),
        (
            r"\mathrm{x\textit{ab}}",
            r#"{rm {x {it {"ab"} rm}} it}"#,
            r"{\mathrm{x} {ab}}",
        ),
        (
            r"\text{a \textit{bc} d}",
            r#"{rm {"a " {it {"bc"} rm} " d"} it}"#,
            r"{\mathrm{a\ } {bc} \mathrm{\ d}}",
        ),
    ] {
        let converted = tex_to_hml(tex).unwrap();
        assert_eq!(converted, native, "{tex}");
        assert_eq!(hml_to_tex(&converted).unwrap(), exported, "{tex}");
    }
}

#[test]
fn nested_text_commands_follow_the_enclosing_text_shape() {
    for (tex, native) in [
        // Bold inside italic text stays italic; textsf/textrm reset the shape.
        (
            r"\textit{a \textbf{b} \textsf{c}}",
            r#"{it {"a " bold {"b"} " " {rm {"c"} it}} it}"#,
        ),
        (
            r"\text{\it a \textrm{b} c}",
            r#"{rm {it "a " {rm {"b"} it} " c"} it}"#,
        ),
        // text/mbox keep the enclosing shape; repeated shapes add no switch.
        (r"\textit{a \text{b} c}", r#"{it {"a " {"b"} " c"} it}"#),
        (r"\textit{a \mbox{b}}", r#"{it {"a " {"b"}} it}"#),
        (r"\textit{a \textit{b}}", r#"{it {"a " {"b"}} it}"#),
        (r"\text{a \textrm{b}}", r#"{rm {"a " {"b"}} it}"#),
        // A text-local \it ends with its group in the shape bookkeeping.
        (
            r"\text{{\it a} \textit{b}}",
            r#"{rm {{it "a"} " " {it {"b"} rm}} it}"#,
        ),
        // textrm leaves typewriter text; textit keeps the typewriter family.
        (
            r"\texttt{a \textrm{b} \textit{c}}",
            r#"{rm {"𝚊 " {"b"} " " {it {"𝚌"} rm}} it}"#,
        ),
    ] {
        let converted = tex_to_hml(tex).unwrap();
        assert_eq!(converted, native, "{tex}");
        assert!(hml_to_tex(&converted).is_ok(), "{tex}: {converted}");
    }
}

#[test]
fn katex_text_stacked_case_converts_every_row() {
    let fixtures: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/katex/cases.json")).unwrap();
    let tex = fixtures["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "katex-122")
        .unwrap()["tex"]
        .as_str()
        .unwrap();
    let native = tex_to_hml(tex).unwrap();
    let rows: Vec<_> = native.split(" # ").collect();
    assert_eq!(rows.len(), 5, "{native}");
    for row in &rows[..3] {
        assert!(
            row.contains(r#"{rm {"abc123 " bold {"abc123"} " " {it {"abc123"} rm}} it}"#),
            "{row}"
        );
    }
    assert!(
        rows[3]
            .contains(r#"{rm {{bold {"abc123"}} " " bold {"abc123"} " " {it {"abc123"} rm}} it}"#)
    );
    assert!(rows[4].contains(r#"{it {"abc123 " bold {"abc123"} " " {rm {"abc123"} it}} it}"#));
    assert!(!native.contains("bf"));
    let exported = hml_to_tex(&native).unwrap();
    assert!(exported.contains(r"\boldsymbol{abc123}"), "{exported}");
    assert!(tex_to_hml(&exported).is_ok());
}
