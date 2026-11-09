// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::tex_to_hml;

#[test]
fn underbar_uses_the_native_under_decoration() {
    assert_eq!(
        tex_to_hml(r"\underbar{a}").unwrap(),
        "{under {rm \"a\" it}}"
    );
    let native = tex_to_hml(r"\underbar{a+b}").unwrap();
    assert_eq!(
        md2hwp_math::hml_to_tex(&native).unwrap(),
        r"\underline{\mathrm{a+b}}"
    );
}

#[test]
fn directly_observed_scalar_symbols_have_explicit_inverse_names() {
    for (tex, native) in [
        (r"\mho", "MHO"),
        (r"\triangleleft", "TRIANGLEL"),
        (r"\triangleright", "TRIANGLER"),
        (r"\iddots", "SDOTS"),
        (r"\nni", "NOWNS"),
        (r"\nsubset", "NSUBSET"),
        (r"\nsubseteq", "NSUBSETEQ"),
        (r"\nsupset", "NSUPSET"),
        (r"\nsupseteq", "NSUPSETEQ"),
        ("℧", "MHO"),
        ("◁", "TRIANGLEL"),
        ("▷", "TRIANGLER"),
        ("⋰", "SDOTS"),
        ("⊢", "VDASH"),
        ("⊣", "DASHV"),
        ("∌", "NOWNS"),
        ("∉", "notin"),
        ("⊄", "NSUBSET"),
        ("⊈", "NSUBSETEQ"),
        ("⊅", "NSUPSET"),
        ("⊉", "NSUPSETEQ"),
        ("∥", "VERT"),
        ("∂", "PARTIAL"),
    ] {
        assert_eq!(tex_to_hml(tex).unwrap(), native, "{tex}");
    }
}

#[test]
fn small_operators_are_explicitly_scoped_and_compose_with_scripts() {
    assert_eq!(
        tex_to_hml(r"{\textstyle\sum}_{i=1}^{n} a_i").unwrap(),
        "{SMALLSUM} _{i = 1} ^{n} a _{i}"
    );
    assert_eq!(tex_to_hml(r"{\textstyle\prod} x").unwrap(), "{SMALLPROD} x");
    assert_eq!(
        tex_to_hml(r"{\textstyle\coprod}_{i=1}^{n} x").unwrap(),
        "{SMALLCOPROD} _{i = 1} ^{n} x"
    );
    for unsupported in [r"\textstyle\sumthing", r"\textstyle\coproduction"] {
        assert!(tex_to_hml(unsupported).is_err(), "{unsupported}");
    }
}

#[test]
fn familiar_symbol_aliases_keep_the_existing_native_canonical_names() {
    for (tex, native) in [
        (r"\dashv", "DASHV"),
        (r"\vdash", "VDASH"),
        (r"\notin", "notin"),
        (r"\not\ni", "not NI"),
        (r"\not\subseteq", "not SUBSETEQ"),
        (r"\Vert", "VERT"),
        (r"\partial", "PARTIAL"),
    ] {
        assert_eq!(tex_to_hml(tex).unwrap(), native, "{tex}");
    }
}
