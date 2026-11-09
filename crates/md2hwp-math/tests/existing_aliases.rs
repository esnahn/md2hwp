// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::{hml_to_tex, tex_to_hml};

#[test]
fn explicit_constructor_aliases_use_the_existing_grammar() {
    // Equation specification p16, 2.5: preserve the printed lower-case right.
    assert!(hml_to_tex("lim_N->inf 1 over N sum_n=1^N\nLEFT(SUM_k=1^n 1 over 2^k right)").is_ok());
    for (alias, canonical) in [
        ("a OVER b", "a over b"),
        ("a ATOP b", "a atop b"),
        ("left (x right )", "LEFT (x RIGHT )"),
        ("SQRT {x}", "sqrt {x}"),
        ("ROOT {3} OF {x}", "root {3} of {x}"),
        ("x SUP 2 SUB i", "x sup 2 sub i"),
        ("SUM FROM i TO n", "sum from i to n"),
        ("REL <-> {a}{b}", "REL lrarrow {a}{b}"),
    ] {
        assert_eq!(
            hml_to_tex(alias).unwrap(),
            hml_to_tex(canonical).unwrap(),
            "{alias}"
        );
    }
    for name in [
        "int", "sum", "union", "inter", "prod", "coprod", "oint", "dint", "tint", "odint", "otint",
    ] {
        assert_eq!(
            hml_to_tex(&name.to_ascii_uppercase()).unwrap(),
            hml_to_tex(name).unwrap()
        );
    }
    for name in [
        "matrix", "pmatrix", "bmatrix", "dmatrix", "cases", "eqalign",
    ] {
        assert_eq!(
            hml_to_tex(&format!("{} {{a&b#c&d}}", name.to_ascii_uppercase())).unwrap(),
            hml_to_tex(&format!("{name} {{a&b#c&d}}")).unwrap()
        );
    }
    for name in [
        "hat", "vec", "dyad", "acute", "grave", "dot", "ddot", "under", "bar", "check", "arch",
        "tilde",
    ] {
        assert_eq!(
            hml_to_tex(&format!("{} {{x}}", name.to_ascii_uppercase())).unwrap(),
            hml_to_tex(&format!("{name} {{x}}")).unwrap()
        );
    }
}

#[test]
fn existing_forward_scalar_mappings_are_readable_in_reverse() {
    for command in [
        "Gamma", "Delta", "Theta", "Lambda", "Xi", "Pi", "Sigma", "Upsilon", "Phi", "Psi", "Omega",
        "leq", "neq", "approx", "equiv", "sim", "in", "subset", "supset", "subseteq", "supseteq",
        "emptyset", "partial", "forall", "ldots", "vdots", "ddots",
    ] {
        let tex = format!("\\{command}");
        let native = tex_to_hml(&tex).unwrap();
        let canonical = if matches!(command, "Upsilon" | "Omega") {
            tex
        } else if command.starts_with(char::is_uppercase) {
            format!("\\mathit{{{tex}}}")
        } else {
            tex
        };
        assert_eq!(hml_to_tex(&native).unwrap(), canonical, "{native}");
    }
    for (native, tex) in [
        ("PLUSMINUS", r"\pm"),
        ("MINUSPLUS", r"\mp"),
        ("DIV", r"\div"),
        ("CAP", r"\cap"),
        ("OSLASH", r"\oslash"),
        ("larrow", r"\leftarrow"),
        ("cdots", r"\cdots"),
    ] {
        assert_eq!(hml_to_tex(native).unwrap(), tex);
    }
}

#[test]
fn aliases_preserve_literal_case_offsets_and_closed_grammar() {
    for literal in [
        "SUM", "SUMx", "HATx", "Gamma", "APPROX", "OVER", "left", "right", "COPROD",
    ] {
        let native = tex_to_hml(literal).unwrap();
        assert!(native.contains('"'), "{literal}: {native}");
        assert_eq!(hml_to_tex(&format!("\"{literal}\"")).unwrap(), literal);
    }
    for script in [
        "AlPhA",
        "Sin",
        "SIN",
        "EXP",
        "BUILDREL {a}{b}{c}",
        "CHOOSE {a}{b}",
        "a BINOM b",
        "col {a}",
        "lcol {a}",
        "rcol {a}",
        "SMALL x",
        "not SUM",
        "not INT",
        "not SQRT x",
        "not MATRIX {x}",
    ] {
        assert!(hml_to_tex(script).is_err(), "accepted {script}");
    }
    assert!(hml_to_tex("한 SQRT").unwrap_err().0.contains("byte 8"));
    assert_eq!(
        hml_to_tex("Lim lim Exp exp Pr").unwrap(),
        r"\operatorname*{Lim} \lim \operatorname{Exp} \exp \Pr"
    );
}

#[test]
fn verified_native_scalars_and_inverse_primitives_preserve_their_meaning() {
    for (alias, canonical) in [
        ("sqcap", "SQCAP"),
        ("VEE", "LOR"),
        ("LLL", "<<<"),
        ("VERT", "||"),
    ] {
        assert_eq!(hml_to_tex(alias).unwrap(), hml_to_tex(canonical).unwrap());
    }
    for (tex, native) in [
        (r"\lor", "LOR"),
        (r"\lll", "<<<"),
        (r"\vert", "vert"),
        (r"\Vert", "VERT"),
    ] {
        assert_eq!(tex_to_hml(tex).unwrap(), native);
    }
    assert_eq!(hml_to_tex("vert").unwrap(), r"\vert");
    assert_eq!(hml_to_tex("Rightarrow").unwrap(), "Rightarrow");
}
