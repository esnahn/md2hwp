// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::tex_to_hml;

#[test]
fn mathop_without_group_or_explicit_limits_preserves_ordinary_symbols() {
    for (tex, native) in [
        (r"\mathop+", "{+}"),
        (r"\mathop:", "{:}"),
        (r"\mathop{\delta}", "{delta}"),
        (r"\mathop{f}\nolimits_i", "{f} _{i}"),
        (r"\mathop{f}_i", "{f} _{i}"),
        (r"\mathop+\limits_i", "{+} _{i}"),
        (r"\mathop{\sum}_i", "{sum} _{i}"),
    ] {
        assert_eq!(tex_to_hml(tex).unwrap(), native, "{tex}");
    }
    assert!(
        tex_to_hml(r"\mathop{a}\limits_b")
            .unwrap()
            .contains("underover")
    );
    for tex in [r"\mathop", r"\mathop{}", r"\mathop^a", r"\mathop\unknown"] {
        assert!(tex_to_hml(tex).is_err(), "{tex}");
    }
}

#[test]
fn starred_matrices_ignore_valid_column_alignment() {
    for env in ["matrix", "pmatrix"] {
        let expected = tex_to_hml(&format!(r"\begin{{{env}}}a&b\\c\end{{{env}}}")).unwrap();
        for align in ["[l]", "[c]", "[r]", ""] {
            assert_eq!(
                tex_to_hml(&format!(r"\begin{{{env}*}}{align}a&b\\c\end{{{env}*}}")).unwrap(),
                expected
            );
        }
    }
    for tex in [
        r"\begin{matrix*}[x]a\end{matrix*}",
        r"\begin{matrix*}[l]a\end{matrix}",
    ] {
        assert!(tex_to_hml(tex).is_err(), "{tex}");
    }
}
