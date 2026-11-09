// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::hml_to_tex;

#[test]
fn autocomplete_catalog_spellings_retain_native_alias_semantics() {
    for (catalog, native, tex) in [
        ("box {a}", "BOX {a}", r"\boxed{a}"),
        (
            "color {1,2,3} {a}",
            "COLOR {1,2,3} {a}",
            r"\textcolor[RGB]{1,2,3}{a}",
        ),
        ("a lsub {b}", "a LSUB {b}", r"{}_{b}a"),
        ("a lsup {b}", "a LSUP {b}", r"{}^{b}a"),
        (
            "underover {a}_{b}^{c}",
            "UNDEROVER {a}_{b}^{c}",
            r"\mathop{a}\limits_{b}^{c}",
        ),
        (
            "rel rarrow {a} {b}",
            "REL rarrow {a} {b}",
            r"\xrightarrow[b]{a}",
        ),
        ("CDOT", "cdot", r"\cdot"),
        ("EXIST", "EXISTS", r"\exists"),
        ("nabla", "NABLA", r"\nabla"),
        ("notin", "NOTIN", r"\notin"),
        ("RARROW", "RARROW", r"\Rightarrow"),
        ("LARROW", "LARROW", r"\Leftarrow"),
        ("LRARROW", "LRARROW", r"\Leftrightarrow"),
    ] {
        assert_eq!(hml_to_tex(catalog).unwrap(), tex, "catalog: {catalog}");
        assert_eq!(hml_to_tex(native).unwrap(), tex, "native: {native}");
    }
}

#[test]
fn aliases_do_not_reinterpret_quoted_text_or_fold_unlisted_case() {
    for word in [
        "box",
        "color",
        "lsub",
        "lsup",
        "underover",
        "rel",
        "CDOT",
        "EXIST",
        "nabla",
        "notin",
        "RARROW",
        "LRARROW",
        "ALPHA",
        "Downarrow",
        "centigrade",
    ] {
        assert_eq!(
            hml_to_tex(&format!("a \"{word}\"")).unwrap(),
            format!("a {word}")
        );
    }
    assert_eq!(hml_to_tex("alpha").unwrap(), "\\alpha");
    assert_eq!(hml_to_tex("ALPHA").unwrap(), hml_to_tex("Alpha").unwrap());
    assert!(hml_to_tex("AlPhA").is_err());
    assert!(hml_to_tex("Box {a}").is_err());
}

#[test]
fn verified_case_aliases_are_exact_and_preserve_distinct_native_cases() {
    let aliases = [
        ("ALPHA", "Alpha"),
        ("BETA", "Beta"),
        ("GAMMA", "Gamma"),
        ("DELTA", "Delta"),
        ("EPSILON", "Epsilon"),
        ("ZETA", "Zeta"),
        ("ETA", "Eta"),
        ("THETA", "Theta"),
        ("IOTA", "Iota"),
        ("KAPPA", "Kappa"),
        ("LAMBDA", "Lambda"),
        ("MU", "Mu"),
        ("NU", "Nu"),
        ("XI", "Xi"),
        ("PI", "Pi"),
        ("RHO", "Rho"),
        ("SIGMA", "Sigma"),
        ("TAU", "Tau"),
        ("UPSILON", "Upsilon"),
        ("PHI", "Phi"),
        ("CHI", "Chi"),
        ("PSI", "Psi"),
        ("OMEGA", "Omega"),
        ("OMICRON", "Omicron"),
        ("Downarrow", "DOWNARROW"),
        ("Larrow", "LARROW"),
        ("Lrarrow", "LRARROW"),
        ("Rarrow", "RARROW"),
        ("Udarrow", "UDARROW"),
        ("Uparrow", "UPARROW"),
        ("Vert", "VERT"),
        ("centigrade", "CENTIGRADE"),
        ("fahrenheit", "FAHRENHEIT"),
        ("laplace", "LAPLACE"),
    ];
    for (alias, canonical) in aliases {
        assert_eq!(
            hml_to_tex(alias).unwrap(),
            hml_to_tex(canonical).unwrap(),
            "alias {alias}"
        );
        assert_eq!(
            hml_to_tex(&format!("{alias} _{{i}} + x")).unwrap(),
            hml_to_tex(&format!("{canonical} _{{i}} + x")).unwrap(),
            "contextual alias {alias}"
        );
        assert_eq!(
            hml_to_tex(&format!("\"{alias}\"")).unwrap(),
            alias,
            "quoted alias {alias} remains literal"
        );
    }

    assert_eq!(hml_to_tex("omicron").unwrap(), "o");
    assert_eq!(hml_to_tex("Omicron").unwrap(), "O");
    assert_eq!(hml_to_tex("OMICRON").unwrap(), "O");
    assert_ne!(
        hml_to_tex("omicron").unwrap(),
        hml_to_tex("Omicron").unwrap()
    );

    assert_eq!(hml_to_tex("alpha").unwrap(), r"\alpha");
    assert_eq!(hml_to_tex("Alpha").unwrap(), "A");
    assert_ne!(hml_to_tex("alpha").unwrap(), hml_to_tex("Alpha").unwrap());
    assert_eq!(hml_to_tex("lim").unwrap(), r"\lim");
    assert_eq!(hml_to_tex("Lim").unwrap(), r"\operatorname*{Lim}");
    assert_eq!(hml_to_tex("vert").unwrap(), r"\vert");
    assert_eq!(hml_to_tex("VERT").unwrap(), r"\Vert");
    assert_ne!(hml_to_tex("vert").unwrap(), hml_to_tex("VERT").unwrap());

    for mixed_case in ["AlPhA", "DownArrOw", "OmIcRoN"] {
        assert!(hml_to_tex(mixed_case).is_err(), "accepted {mixed_case}");
        assert_eq!(
            hml_to_tex(&format!("\"{mixed_case}\"")).unwrap(),
            mixed_case
        );
    }
}

#[test]
fn double_arrows_use_catalog_commands_instead_of_literal_relations() {
    use md2hwp_math::tex_to_hml;
    assert_eq!(tex_to_hml(r"\Rightarrow").unwrap(), "RARROW");
    assert_eq!(tex_to_hml(r"\Leftarrow").unwrap(), "LARROW");
    assert_eq!(hml_to_tex("<=").unwrap(), "\\leq");
    assert_eq!(tex_to_hml(r"\Leftrightarrow").unwrap(), "LRARROW");
    assert!(!hml_to_tex("=>").unwrap().contains(r"\Rightarrow"));
    assert!(!hml_to_tex("<=>").unwrap().contains(r"\Leftrightarrow"));
}
