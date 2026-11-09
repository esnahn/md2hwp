// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::{hml_to_tex, tex_to_hml};

#[test]
fn specification_bigg_normalizes_to_fixed_tex_size() {
    assert_eq!(
        hml_to_tex("{a+b} over {a-b} bigg / {x+y} over {x-y}").unwrap(),
        r"\frac{a + b}{a - b} \Bigg/ \frac{x + y}{x - y}"
    );
    for command in ["bigg", "BIGG"] {
        assert_eq!(hml_to_tex(&format!("{command} / x")).unwrap(), r"\Bigg/ x");
    }
    let tex = r"\frac{a+b}{a-b}\Bigg/\frac{x+y}{x-y}";
    let script = tex_to_hml(tex).unwrap();
    assert_eq!(
        script,
        "{{a + b} over {a - b}} {scale 300 \"/\" scale 100} {{x + y} over {x - y}}"
    );
    assert!(hml_to_tex(&script).is_ok());
}

#[test]
fn finite_visible_delimiters_roundtrip() {
    for (native, tex) in [
        ("/", "/"),
        ("(", "("),
        (")", ")"),
        ("[", "["),
        ("]", "]"),
        ("<", r"\langle"),
        (">", r"\rangle"),
        ("|", "|"),
        ("∥", r"\lVert"),
        ("⌈", r"\lceil"),
        ("⌉", r"\rceil"),
        ("⌊", r"\lfloor"),
        ("⌋", r"\rfloor"),
    ] {
        let tex = format!(r"\Bigg{tex}");
        assert_eq!(hml_to_tex(&format!("bigg {native}")).unwrap(), tex);
        let script = tex_to_hml(&tex).unwrap();
        assert_eq!(script, format!("{{scale 300 \"{native}\" scale 100}}"));
        assert!(hml_to_tex(&script).is_ok());
    }
    for (native, tex) in [
        ("||", r"\Bigg\lVert"),
        ("PVER", r"\Bigg\lVert"),
        ("LCEIL", r"\Bigg\lceil"),
        ("RFLOOR", r"\Bigg\rfloor"),
    ] {
        assert_eq!(hml_to_tex(&format!("bigg {native}")).unwrap(), tex);
    }
}

#[test]
fn scripts_groups_and_fraction_arguments_keep_delimiter_scope() {
    assert_eq!(hml_to_tex("bigg / ^2 x").unwrap(), r"\Bigg/^{2} x");
    assert_eq!(
        tex_to_hml(r"\Bigg/^2 x").unwrap(),
        "{scale 300 \"/\" scale 100} ^{2} x"
    );
    assert_eq!(hml_to_tex("{bigg /} ^2 x").unwrap(), r"\Bigg/^{2} x");
    assert_eq!(
        tex_to_hml(r"\frac{\Bigg/}{x}").unwrap(),
        "{{{scale 300 \"/\" scale 100}} over {x}}"
    );
    assert_eq!(hml_to_tex("{bigg /} over x").unwrap(), r"\frac{\Bigg/}{x}");
    assert_eq!(hml_to_tex("bigg / over x").unwrap(), r"\frac{\Bigg/}{x}");
    assert_eq!(
        hml_to_tex("LEFT ( bigg / x RIGHT )").unwrap(),
        r"\left(\Bigg/ x\right)"
    );
    assert_eq!(
        hml_to_tex("a bigg / b bigg | c").unwrap(),
        r"a \Bigg/ b \Bigg| c"
    );
    assert_eq!(
        hml_to_tex("matrix{bigg / & x}").unwrap(),
        r"\begin{matrix}\Bigg/&x\end{matrix}"
    );
}

#[test]
fn malformed_and_unretained_size_forms_reject() {
    for native in [
        "bigg",
        "BIGG",
        "bigg x",
        "bigg +",
        "bigg .",
        "bigg bigg /",
        "bigg \"/\"",
        "bigg / ^",
        "Bigg /",
        "BiGg /",
        "{bigg}",
        "bigg {",
        "bigg }",
        "bigg \\",
    ] {
        assert!(hml_to_tex(native).is_err(), "accepted {native}");
    }
    for tex in [
        r"\Bigg",
        r"\Bigg x",
        r"\Bigg+",
        r"\Bigg\unknown",
        r"\Bigg/^",
        r"\Bigg{",
        r"\middle/",
    ] {
        assert!(tex_to_hml(tex).is_err(), "accepted {tex}");
    }
}

#[test]
fn literal_command_letters_do_not_become_size_commands() {
    for literal in ["bigg", "BIGG", "Bigg", "biggx", "BIGGx"] {
        let tex = format!(r"\mathit{{{literal}}}");
        let script = tex_to_hml(&tex).unwrap();
        assert!(script.contains(&format!("\"{literal}\"")), "{script}");
        assert!(!hml_to_tex(&script).unwrap().contains(r"\Bigg"));
    }
    assert!(!hml_to_tex(r#""bigg" /"#).unwrap().contains(r"\Bigg"));
}

#[test]
fn enlarged_delimiters_retain_resource_limits() {
    assert!(
        hml_to_tex(&"bigg / ".repeat(17_000))
            .unwrap_err()
            .0
            .contains("token limit")
    );
    assert!(
        tex_to_hml(&r"\Bigg/ ".repeat(33_000))
            .unwrap_err()
            .message
            .contains("atom limit")
    );
    let nested = format!("{}bigg /{}", "{".repeat(120), "}".repeat(120));
    assert!(hml_to_tex(&nested).unwrap_err().0.contains("nesting limit"));
}
