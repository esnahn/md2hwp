// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::{hml_to_tex, tex_to_hml};

#[test]
fn mkern_accepts_only_bare_mu_dimensions() {
    assert_eq!(
        tex_to_hml(r"\overrightarrow{a}\mkern2.25mu b").unwrap(),
        "{vec {a}} ` b"
    );
    for input in [
        r"a\mkern-2.25mu b",
        r"a\mkern2.25pt b",
        r"a\mkern{x} b",
        r"a\mkern2.25mu2",
    ] {
        assert!(
            tex_to_hml(input).is_err(),
            "unsupported dimension accepted: {input}"
        );
    }
}

#[test]
fn structure_and_token_boundaries_are_explicit() {
    assert_eq!(
        tex_to_hml(r"\frac{xy}{z}+\sqrt[n]{a}").unwrap(),
        "{{xy} over {z}} + {root {n} of {a}}"
    );
    assert_eq!(tex_to_hml(r"x^12+y_{12}").unwrap(), "x ^{1} 2 + y _{12}");
    assert_eq!(
        tex_to_hml(r"\vec{a}\cdot\vec{b}").unwrap(),
        "{vec {a}} CDOT {vec {b}}"
    );
    assert_eq!(tex_to_hml(r"\quad\,\;\qquad").unwrap(), "~~ ` ``` ~~~~");
    assert_eq!(
        tex_to_hml(r"\text{한글 공백} + \mathrm{ABC}").unwrap(),
        "{rm \"한글 공백\" it} + {rm {\"ABC\"} it}"
    );
    assert_eq!(
        tex_to_hml(r"\textcolor[RGB]{12,34,56}{x^2}").unwrap(),
        "{color {12, 34, 56} {x ^{2}}}"
    );
}

#[test]
fn matrices_groups_and_paired_delimiters_compose() {
    assert_eq!(
        tex_to_hml(r"\begin{aligned}[x]a\\b\end{aligned}").unwrap(),
        "{eqalign {[ x ] a # b}}"
    );
    assert_eq!(
        tex_to_hml(r"[a,b]\rightarrow[c,d]").unwrap(),
        "[ a , b ] -> [ c , d ]"
    );
    assert_eq!(
        tex_to_hml(r"\left(\frac{a}{b}\right)^2").unwrap(),
        "{LEFT ( {{a} over {b}} RIGHT )} ^{2}"
    );
    assert_eq!(
        tex_to_hml(r"\begin{pmatrix}a&b\\c&d\end{pmatrix}").unwrap(),
        "{pmatrix {a & b # c & d}}"
    );
    assert_eq!(
        tex_to_hml(r"\begin{matrix}{a+b}&\sqrt{x}\\0&1\end{matrix}").unwrap(),
        "{matrix {{a + b} & {sqrt {x}} # 0 & 1}}"
    );
    assert_eq!(
        tex_to_hml(r"\left\{a\right\}").unwrap(),
        "{LEFT { a RIGHT }}"
    );
    assert_eq!(
        tex_to_hml(r"\left\lbrace a\right\rbrace").unwrap(),
        "{LEFT { a RIGHT }}"
    );
}

#[test]
fn malformed_and_stateful_tex_fail_explicitly() {
    for input in [
        "",
        "{x",
        "x}",
        "x^",
        "x__2",
        "x^2^3",
        r"\frac{x}",
        r"\unknown{x}",
        r"\def\x{1}\x",
        r"\newcommand{\x}{1}",
        r"\require{color}",
        r"\left(x",
        r"\left\rbrace a \right\lbrace",
        r"\left\{a\right]",
        r"\left[a\right\}",
        r"\left(a\right\}",
        r"x\right)",
        r"\begin{matrix}a\end{cases}",
        r"\begin{aligned}[t]a\\b\end{aligned}",
        r"\begin{aligned} [b] a\\b\end{aligned}",
        r"\begin{aligned}[c]a\\b\end{aligned}",
        r"\textcolor[RGB]{256,0,0}{x}",
        r"\textcolor{unknown}{x}",
        r"\text{\alpha}",
        r"\text{if $x > 1$}",
        r"\text{ for $|q| < 1$.}",
        r"\operatorname{f$x$}",
        r"\mbox{$x$}",
        r"\sqrt[]{}&",
        "x\0y",
    ] {
        assert!(tex_to_hml(input).is_err(), "unexpected success: {input:?}");
    }
    let e = tex_to_hml("α+\\unknown").unwrap_err();
    assert_eq!(e.offset, 3);
    assert!(e.to_string().contains("byte 3"));
}

#[test]
fn resource_limits_cover_groups_and_unbraced_command_chains() {
    let grouped = format!("{}x{}", "{".repeat(120), "}".repeat(120));
    assert!(tex_to_hml(&grouped).is_err());
    let chained = format!("{}x", r"\sqrt".repeat(120));
    assert!(tex_to_hml(&chained).is_err());
    assert!(tex_to_hml(&"x".repeat(262_145)).is_err());
    assert!(tex_to_hml(&"x+".repeat(20_000)).is_err());
}

#[test]
fn ascii_standard_symbols_have_stable_native_roundtrips() {
    for (native, tex, unicode) in [
        ("OHM", r"\Omega", "Ω"),
        ("varupsilon", r"\Upsilon", "ϒ"),
        ("DEG", r"{}^{\circ}", "°"),
        ("LAPLACE", r"\mathcal{L}", "ℒ"),
        ("CENTIGRADE", r"{}^{\circ}\mathrm{C}", "℃"),
        ("FAHRENHEIT", r"{}^{\circ}\mathrm{F}", "℉"),
        ("ANGSTROM", r"\text{\AA}", "Å"),
        ("att", r"\text{\textreferencemark}", "※"),
        ("hund", r"\text{\textperthousand}", "‰"),
        ("thou", r"\text{\textpertenthousand}", "‱"),
        ("IDENTICAL", "::", "∷"),
    ] {
        assert_eq!(hml_to_tex(native).unwrap(), tex, "{native}");
        assert_eq!(tex_to_hml(tex).unwrap(), native, "{tex}");
        assert_eq!(tex_to_hml(unicode).unwrap(), native, "{unicode}");
        let second = hml_to_tex(&tex_to_hml(tex).unwrap()).unwrap();
        assert_eq!(second, tex);
        let literal = hml_to_tex(&format!("\"{native}\"")).unwrap();
        assert_ne!(literal, tex, "literal name became a symbol: {native}");
    }
    assert_eq!(tex_to_hml(r"{}^\circ \mathrm { C }").unwrap(), "CENTIGRADE");
    assert_eq!(
        tex_to_hml(r"{}^{\circ} \mathrm{ F }").unwrap(),
        "FAHRENHEIT"
    );
    assert_eq!(tex_to_hml(r"{}^b a").unwrap(), "a lsup {b}");
    assert_eq!(tex_to_hml(r"{}^b").unwrap(), "{} ^{b}");
    assert_eq!(tex_to_hml(r"{}^\circ C").unwrap(), "DEG C");
    assert_eq!(
        tex_to_hml(r"{}^\circ\mathrm{C}^2").unwrap(),
        "CENTIGRADE ^{2}"
    );
    assert_ne!(tex_to_hml(r"{}^\circ \mathrm{CF}").unwrap(), "CENTIGRADE");
}

#[test]
fn upright_omega_and_explicit_greek_font_context_remain_distinct() {
    assert_eq!(tex_to_hml(r"\Omega").unwrap(), "OHM");
    assert_eq!(tex_to_hml(r"\mathit{\Omega}").unwrap(), "{Omega}");
    for native in ["OHM", "Omega", "Upsilon", "varupsilon"] {
        let tex = hml_to_tex(native).unwrap();
        let canonical = hml_to_tex(&tex_to_hml(&tex).unwrap()).unwrap();
        // Optional native outer grouping is presentation-neutral.
        assert_eq!(canonical, tex);
    }
    let native = tex_to_hml(r"\mathit{\Omega\mathrm{\Omega}\Omega}\Omega").unwrap();
    assert_eq!(native.matches("OHM").count(), 2, "{native}");
    assert_eq!(native.matches("Omega").count(), 2, "{native}");
    let native = tex_to_hml(r"\mathrm{\Omega\mathit{\Omega}\Omega}").unwrap();
    assert_eq!(native.matches("OHM").count(), 2, "{native}");
    assert_eq!(native.matches("Omega").count(), 1, "{native}");
    assert_eq!(
        tex_to_hml(r"\Gamma\mathit{\Gamma}").unwrap(),
        "Gamma {Gamma}"
    );
}

#[test]
fn scalar_alphabet_and_text_symbol_support_stays_bounded() {
    for input in [
        r"\mathcal{\L}",
        r"\text{\AAfoo}",
        r"\text{\input{file}}",
        r"\operatorname{\AA}",
        r#"\text{\"}"#,
    ] {
        assert!(tex_to_hml(input).is_err(), "accepted: {input}");
    }
    assert_eq!(tex_to_hml(r"\text{ \AA }").unwrap(), "ANGSTROM");
    assert!(tex_to_hml(&r"\mathcal{L}".repeat(16_385)).is_err());
    assert!(tex_to_hml(&r"\text{\AA}".repeat(32_769)).is_err());
    let nested = format!("{}\\Omega{}", r"\mathit{".repeat(100), "}".repeat(100));
    assert!(tex_to_hml(&nested).is_err());
}

#[test]
fn mathbb_keeps_native_capitals_and_accepts_scoped_content() {
    assert_eq!(tex_to_hml(r"\mathbb{A}").unwrap(), "VecA");
    assert_eq!(tex_to_hml(r"\mathbb A").unwrap(), "VecA");
    assert_eq!(
        tex_to_hml("\\mathbb{A B\tC\nD}").unwrap(),
        "{VecA VecB VecC VecD}"
    );
    let alphabet = ('A'..='Z')
        .map(|letter| format!("Vec{letter}"))
        .collect::<Vec<_>>()
        .join(" ");
    assert_eq!(
        tex_to_hml(r"\mathbb{ABCDEFGHIJKLMNOPQRSTUVWXYZ}").unwrap(),
        format!("{{{alphabet}}}")
    );
    assert_eq!(tex_to_hml(r"\mathbb{AB}_i").unwrap(), "{VecA VecB} _{i}");
    let fraction = hml_to_tex(&tex_to_hml(r"\frac{\mathbb{AB}}{\mathbb C}").unwrap()).unwrap();
    assert_eq!(
        fraction
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>(),
        r"\frac{\mathbb{A}\mathbb{B}}{\mathbb{C}}"
    );

    for input in [
        r"\mathbb{a}",
        r"\mathbb{A1}",
        r"\mathbb{\alpha}",
        r"\mathbb{{A}}",
        r"\mathbb a",
        r"\mathbb 1",
        r"\mathbb\alpha",
    ] {
        assert!(hml_to_tex(&tex_to_hml(input).unwrap()).is_ok(), "{input}");
    }
    for input in [r"\mathbb{}", r"\mathbb{ }"] {
        assert!(tex_to_hml(input).is_err(), "{input}");
    }
    let too_many_letters = format!(r"\mathbb{{{}}}", "A".repeat(32_768));
    assert!(
        tex_to_hml(&too_many_letters)
            .unwrap_err()
            .message
            .contains("atom limit")
    );
}

#[test]
fn identical_uses_adjacent_plain_colons_with_explicit_script_scope() {
    assert_eq!(hml_to_tex("IDENTICAL").unwrap(), "::");
    assert_eq!(tex_to_hml("::").unwrap(), "IDENTICAL");
    assert_eq!(tex_to_hml(r"\dblcolon").unwrap(), "IDENTICAL");
    assert_eq!(tex_to_hml(":").unwrap(), ":");
    assert_eq!(tex_to_hml(r"\colon").unwrap(), ":");
    assert_eq!(tex_to_hml(": :").unwrap(), ": :");
    assert_eq!(tex_to_hml(":::").unwrap(), "IDENTICAL :");
    assert_eq!(tex_to_hml(r"\frac::").unwrap(), "{{:} over {:}}");
    assert_eq!(tex_to_hml(r"x^::").unwrap(), "x ^{:} :");
    assert_eq!(hml_to_tex("IDENTICAL^2").unwrap(), "{::}^{2}");
    assert_eq!(hml_to_tex("not IDENTICAL").unwrap(), r"\not{::}");
    assert_eq!(tex_to_hml(r"\not{::}").unwrap(), "not IDENTICAL");
    assert_eq!(tex_to_hml(r"\not::").unwrap(), "not : :");
    assert!(tex_to_hml(r"\not{: :}").is_err());
    assert!(tex_to_hml(r"\not{:::}").is_err());
    assert!(!tex_to_hml(r"\text{::}").unwrap().contains("IDENTICAL"));
    assert!(
        !tex_to_hml(r"\operatorname{::}")
            .unwrap()
            .contains("IDENTICAL")
    );
    assert!(tex_to_hml(&"::".repeat(16_385)).is_err());
    for native in [
        "IDENTICAL_i",
        "IDENTICAL over x",
        "x over IDENTICAL",
        "not IDENTICAL^2",
    ] {
        let tex = hml_to_tex(native).unwrap();
        assert_eq!(
            hml_to_tex(&tex_to_hml(&tex).unwrap()).unwrap(),
            tex,
            "{native}"
        );
    }
}

#[test]
fn leading_scripts_use_an_empty_base_without_swallowing_operators() {
    assert_eq!(tex_to_hml("^3+").unwrap(), "{} ^{3} +");
    assert_eq!(tex_to_hml(r"{}^{3}+").unwrap(), "{} ^{3} +");
    assert_eq!(tex_to_hml(r"{}^b a").unwrap(), "a lsup {b}");
    assert!(
        tex_to_hml(r"^3^4+")
            .unwrap_err()
            .message
            .contains("duplicate script")
    );
    assert!(tex_to_hml("^").is_err());
    let fixtures: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/katex/cases.json")).unwrap();
    let tex = fixtures["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "katex-102")
        .unwrap()["tex"]
        .as_str()
        .unwrap();
    let native = tex_to_hml(tex).unwrap();
    assert!(native.contains("{} ^{3}"), "{native}");
    assert!(hml_to_tex(&native).is_ok());
}
