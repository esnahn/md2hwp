// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::{hml_to_tex, tex_to_hml};

#[test]
fn specification_scalar_symbols_have_standard_tex_and_stable_roundtrips() {
    for (native, tex) in [
        ("vartheta", r"\vartheta"),
        ("varpi", r"\varpi"),
        ("varsigma", r"\varsigma"),
        ("varphi", r"\varphi"),
        ("varepsilon", r"\varepsilon"),
        ("ALEPH", r"\aleph"),
        ("HBAR", r"\hbar"),
        ("IMATH", r"\imath"),
        ("JMATH", r"\jmath"),
        ("OHM", r"\Omega"),
        ("ELL", r"\ell"),
        ("LITER", r"\ell"),
        ("WP", r"\wp"),
        ("IMAG", r"\Im"),
        ("SQCUP", r"\sqcup"),
        ("OPLUS", r"\oplus"),
        ("OTIMES", r"\otimes"),
        ("ODOT", r"\odot"),
        ("OWNS", r"\ni"),
        ("SQSUBSET", r"\sqsubset"),
        ("SQSUPSET", r"\sqsupset"),
        ("SQSUBSETEQ", r"\sqsubseteq"),
        ("SQSUPSETEQ", r"\sqsupseteq"),
        ("PREC", r"\prec"),
        ("SUCC", r"\succ"),
        ("UPLUS", r"\uplus"),
        ("CIRC", r"\circ"),
        ("AST", r"\ast"),
        ("STAR", r"\bigstar"),
        ("BIGCIRC", r"\bigcirc"),
        ("THEREFORE", r"\therefore"),
        ("BECAUSE", r"\because"),
        ("IDENTICAL", "::"),
        ("DOTEQ", r"\doteq"),
        ("SIMEQ", r"\simeq"),
        ("CONG", r"\cong"),
        ("ASYMP", r"\asymp"),
        ("DIAMOND", r"\diamond"),
        ("prime", r"\prime"),
        ("LNOT", r"\lnot"),
        ("PROPTO", r"\propto"),
        ("DAGGER", r"\dagger"),
        ("DDAGGER", r"\ddagger"),
        ("uparrow", r"\uparrow"),
        ("downarrow", r"\downarrow"),
        ("UPARROW", r"\Uparrow"),
        ("DOWNARROW", r"\Downarrow"),
        ("udarrow", r"\updownarrow"),
        ("UDARROW", r"\Updownarrow"),
        ("nwarrow", r"\nwarrow"),
        ("searrow", r"\searrow"),
        ("nearrow", r"\nearrow"),
        ("swarrow", r"\swarrow"),
        ("hookleft", r"\hookleftarrow"),
        ("hookright", r"\hookrightarrow"),
        ("mapsto", r"\mapsto"),
        ("TRIANGLE", r"\triangle"),
        ("TRIANGLED", r"\triangledown"),
        ("ANGLE", r"\angle"),
        ("MSANGLE", r"\measuredangle"),
        ("SANGLE", r"\sphericalangle"),
        ("RTANGLE", "∟"),
        ("VDASH", r"\vdash"),
        ("BOT", r"\bot"),
        ("TOP", r"\top"),
        ("MODELS", r"\models"),
    ] {
        assert_eq!(hml_to_tex(native).unwrap(), tex, "{native}");
        let reconstructed = tex_to_hml(tex).unwrap();
        assert_eq!(
            hml_to_tex(&reconstructed).unwrap(),
            tex,
            "{native}: {reconstructed}"
        );
        assert_eq!(hml_to_tex(&format!("\"{native}\"")).unwrap(), native);
        let literal = tex_to_hml(native).unwrap();
        assert!(literal.contains('"'), "literal {native}: {literal}");
        assert_eq!(hml_to_tex(&literal).unwrap(), native);
    }
}

#[test]
fn symbol_spellings_remain_exact_and_arrows_keep_their_cases() {
    for invalid in ["Aleph", "Hbar", "Vartheta", "Sqsubset", "UpArrow", "Mapsto"] {
        assert!(hml_to_tex(invalid).is_err(), "accepted {invalid}");
    }
    assert_ne!(
        hml_to_tex("uparrow").unwrap(),
        hml_to_tex("UPARROW").unwrap()
    );
    assert_ne!(
        hml_to_tex("udarrow").unwrap(),
        hml_to_tex("UDARROW").unwrap()
    );
}

#[test]
fn raw_square_in_the_printed_fraction_example_roundtrips() {
    assert_eq!(hml_to_tex("□").unwrap(), r"\square");
    assert_eq!(tex_to_hml(r"\square").unwrap(), "□");
    assert_eq!(tex_to_hml(r"\Box").unwrap(), "□");
    let source = "10a^3 over b^2 times ~□ ~÷ b^3 over 2a =( 2a^2 over b )^3";
    let tex = hml_to_tex(source).unwrap();
    assert!(tex.contains(r"\square"));
    let native = tex_to_hml(&tex).unwrap();
    assert_eq!(hml_to_tex(&native).unwrap(), tex);
}

#[test]
fn native_slants_keep_their_long_diagonal_shapes_and_names() {
    assert_eq!(hml_to_tex("LSLANT").unwrap(), r"\diagup");
    assert_eq!(hml_to_tex("RSLANT").unwrap(), r"\diagdown");
    assert_eq!(tex_to_hml(r"\diagup").unwrap(), "LSLANT");
    assert_eq!(tex_to_hml(r"\diagdown").unwrap(), "RSLANT");
    assert_eq!(tex_to_hml(r"\backslash").unwrap(), "RSLANT");
    assert_ne!(hml_to_tex("LSLANT").unwrap(), hml_to_tex("/").unwrap());
}

#[test]
fn every_printed_symbol_from_alpha_onward_converts_and_roundtrips() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/specification-equations.json")).unwrap();
    let mut checked = 0;
    for record in fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .skip_while(|record| record["category"] != "greek-uppercase")
        .filter(|record| record["source_kind"] == "symbol")
    {
        let script = record["hancom_script"].as_str().unwrap();
        let tex =
            hml_to_tex(script).unwrap_or_else(|error| panic!("{} {script}: {error}", record["id"]));
        let native = tex_to_hml(&tex)
            .unwrap_or_else(|error| panic!("{} {script} -> {tex}: {error}", record["id"]));
        assert_eq!(
            hml_to_tex(&native).unwrap(),
            tex,
            "{} {script}: {native}",
            record["id"]
        );
        checked += 1;
    }
    assert!(
        checked >= 140,
        "only {checked} printed symbol cases checked"
    );
}

#[test]
fn uncommon_scalars_keep_native_identity_without_changing_latin_fonts() {
    for (native, tex) in [
        ("varupsilon", r"\Upsilon"),
        ("ANGSTROM", r"\text{\AA}"),
        ("OHM", r"\Omega"),
        ("IDENTICAL", "::"),
        ("RTANGLE", "∟"),
        ("DEG", r"{}^{\circ}"),
        ("LAPLACE", r"\mathcal{L}"),
        ("CENTIGRADE", r"{}^{\circ}\mathrm{C}"),
        ("FAHRENHEIT", r"{}^{\circ}\mathrm{F}"),
        ("att", r"\text{\textreferencemark}"),
        ("hund", r"\text{\textperthousand}"),
        ("thou", r"\text{\textpertenthousand}"),
        ("well", "⌗"),
        ("base", "⌂"),
        ("benzene", "⌬"),
    ] {
        assert_eq!(hml_to_tex(native).unwrap(), tex, "{native}");
        assert_eq!(tex_to_hml(tex).unwrap(), native, "{tex}");
    }
    assert!(!tex_to_hml(r"\mathrm{A}").unwrap().contains("Alpha"));
    assert_eq!(tex_to_hml(r"\Omega").unwrap(), "OHM");
    assert_ne!(hml_to_tex("OHM").unwrap(), hml_to_tex("Omega").unwrap());
    assert_eq!(tex_to_hml(r"\star").unwrap(), "STAR");
    assert!(tex_to_hml("Alpha").unwrap().contains('"'));
}

#[test]
fn uppercase_greek_uses_ascii_tex_and_stays_slanted_in_native_font_contexts() {
    for (name, body) in [
        ("Alpha", "A"),
        ("Beta", "B"),
        ("Gamma", r"\Gamma"),
        ("Delta", r"\Delta"),
        ("Epsilon", "E"),
        ("Zeta", "Z"),
        ("Eta", "H"),
        ("Theta", r"\Theta"),
        ("Iota", "I"),
        ("Kappa", "K"),
        ("Lambda", r"\Lambda"),
        ("Mu", "M"),
        ("Nu", "N"),
        ("Xi", r"\Xi"),
        ("Omicron", "O"),
        ("Pi", r"\Pi"),
        ("Rho", "P"),
        ("Sigma", r"\Sigma"),
        ("Tau", "T"),
        ("Upsilon", "Y"),
        ("Phi", r"\Phi"),
        ("Chi", "X"),
        ("Psi", r"\Psi"),
        ("Omega", r"\Omega"),
    ] {
        let tex = if body.len() == 1 {
            body.to_owned()
        } else {
            format!("\\mathit{{{body}}}")
        };
        assert!(tex.is_ascii(), "{name}: {tex}");
        for native in [name.to_owned(), format!("rm {name}"), format!("it {name}")] {
            assert_eq!(hml_to_tex(&native).unwrap(), tex, "{native}");
        }
        let native = tex_to_hml(&tex).unwrap();
        if body.len() == 1 {
            assert_eq!(native, body, "{name} canonicalizes to a Latin letter");
        }
        assert_eq!(hml_to_tex(&native).unwrap(), tex, "{name}: {native}");
    }
    assert_eq!(hml_to_tex("OHM").unwrap(), r"\Omega");
    assert_eq!(hml_to_tex("varupsilon").unwrap(), r"\Upsilon");
    assert_eq!(hml_to_tex("omega").unwrap(), r"\omega");
    assert_eq!(tex_to_hml(r"\Upsilon").unwrap(), "varupsilon");
    assert_eq!(hml_to_tex(&tex_to_hml("Y").unwrap()).unwrap(), "Y");
}

#[test]
fn rightangle_outputs_unicode_and_keeps_command_and_unicode_inputs() {
    assert_eq!(hml_to_tex("RTANGLE").unwrap(), "∟");
    assert_eq!(tex_to_hml(r"\rightangle").unwrap(), "RTANGLE");
    assert_eq!(tex_to_hml("⊾").unwrap(), "RTANGLE");
    assert_eq!(tex_to_hml("∟").unwrap(), "RTANGLE");
    for native in [
        "RTANGLE^2",
        "RTANGLE_i",
        "not RTANGLE",
        "not RTANGLE^2",
        "RTANGLE over x",
    ] {
        let tex = hml_to_tex(native).unwrap();
        assert_eq!(
            hml_to_tex(&tex_to_hml(&tex).unwrap()).unwrap(),
            tex,
            "{native}"
        );
    }
    assert!(tex_to_hml(r"\rightanglex").is_err());
    assert!(
        !tex_to_hml(r"\text{rightangle}")
            .unwrap()
            .contains("RTANGLE")
    );
}

#[test]
fn viewdata_and_house_output_unicode_and_keep_command_aliases() {
    for (native, tex, unicode) in [
        ("well", r"\viewdata", "⌗"),
        ("base", r"\house", "⌂"),
        ("benzene", "⌬", "⌬"),
    ] {
        assert_eq!(hml_to_tex(native).unwrap(), unicode);
        assert_eq!(tex_to_hml(tex).unwrap(), native);
        assert_eq!(tex_to_hml(unicode).unwrap(), native);
        for script in [
            format!("{native}^2"),
            format!("{native}_i"),
            format!("not {native}"),
            format!("not {native}^2"),
            format!("{native} over x"),
        ] {
            if native == "benzene" && script.starts_with("not ") {
                continue;
            }
            let tex = hml_to_tex(&script).unwrap();
            assert_eq!(
                hml_to_tex(&tex_to_hml(&tex).unwrap()).unwrap(),
                tex,
                "{script}"
            );
        }
    }
    for tex in [r"\viewdatax", r"\housex"] {
        assert!(tex_to_hml(tex).is_err());
    }
    for tex in [r"\text{viewdata}", r"\text{house}"] {
        let native = tex_to_hml(tex).unwrap();
        assert!(!native.contains("well"));
        assert!(!native.contains("base"));
    }
}
