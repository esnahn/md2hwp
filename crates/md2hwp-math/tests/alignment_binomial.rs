// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::{hml_to_tex, tex_to_hml};

#[test]
fn native_alignment_and_binomial_constructors_keep_their_roles() {
    for (native, tex) in [
        ("LPILE {a#bbb}", r"\begin{array}{l}a\\bbb\end{array}"),
        ("rpile {aaa#b}", r"\begin{array}{r}aaa\\b\end{array}"),
        ("n CHOOSE k", r"\binom{n}{k}"),
        ("BINOM {n+1} {k-1}", r"\binom{n + 1}{k - 1}"),
        ("{a+b} choose {c+d}", r"\binom{a + b}{c + d}"),
        (
            "matrix{lcol{a#bbb} col{ccc#d} rcol{ee#f}}",
            r"\begin{array}{lcr}a&ccc&ee\\bbb&d&f\end{array}",
        ),
        (
            "PMATRIX{COL{a#b} RCOL{c#d}}",
            r"\left(\begin{array}{cr}a&c\\b&d\end{array}\right)",
        ),
        (
            "bmatrix{lcol{a#b}}",
            r"\left[\begin{array}{l}a\\b\end{array}\right]",
        ),
        (
            "dmatrix{rcol{a#b}}",
            r"\left|\begin{array}{r}a\\b\end{array}\right|",
        ),
        (
            "lpile{rm a#it b}",
            r"\begin{array}{l}\mathrm{a}\\b\end{array}",
        ),
    ] {
        assert_eq!(hml_to_tex(native).unwrap(), tex, "{native}");
        let restored = tex_to_hml(tex).unwrap();
        let canonical = if tex.contains(r"\begin{array}{lcr}") {
            r"\begin{matrix}a&ccc&ee\\bbb&d&f\end{matrix}"
        } else if tex.contains(r"\begin{array}{cr}") {
            r"\left(\begin{matrix}a&c\\b&d\end{matrix}\right)"
        } else {
            tex
        };
        assert_eq!(hml_to_tex(&restored).unwrap(), canonical, "{restored}");
    }
}

#[test]
fn tex_arrays_keep_shared_native_rows_and_literals() {
    assert_eq!(
        tex_to_hml(r"\begin{array}{lcr}a&b&c\\d&e&f\end{array}").unwrap(),
        "{matrix {a & b & c # d & e & f}}"
    );
    assert_eq!(
        tex_to_hml(r"\begin{array}{l}a\\b\end{array}").unwrap(),
        "{lpile {a # b}}"
    );
    assert_eq!(
        tex_to_hml(r"\begin{array}{r}a\\b\end{array}").unwrap(),
        "{rpile {a # b}}"
    );
    assert_eq!(
        tex_to_hml(r"\begin{array}{c}a\\b\end{array}").unwrap(),
        "{matrix {a # b}}"
    );
    assert_eq!(
        tex_to_hml(r"\begin{array}{|rl:c||}a&b&c\\d&e&f\end{array}").unwrap(),
        "{matrix {a & b & c # d & e & f}}"
    );
    for word in ["lpile", "rpile", "col", "lcol", "rcol", "choose", "binom"] {
        let native = tex_to_hml(word).unwrap();
        assert!(native.contains('"'), "{native}");
        assert_eq!(hml_to_tex(&native).unwrap(), word);
    }
    let native = tex_to_hml(r"\begin{array}{lr}\mathrm{abc}&d\\e&\mathbf{f}\end{array}").unwrap();
    let tex = hml_to_tex(&native).unwrap();
    assert!(tex.contains(r"\mathrm{abc}"), "{tex}");
    assert!(tex.contains(r"\mathbf{f}"), "{tex}");
}

#[test]
fn malformed_alignment_binomials_and_array_programs_fail_explicitly() {
    for native in [
        "lpile{}",
        "rpile{a&b}",
        "lpile{a#}",
        "choose a b",
        "binom a",
        "binom {} a",
        "a choose {}",
        "col{a#b}",
        "lcol{a}",
        "rcol{a}",
        "matrix{col{a#b} col{c}}",
        "matrix{col{a&b}}",
        "matrix{col{a} & b}",
        "matrix{a & col{b}}",
        "Lpile{a#b}",
        "Binom a b",
        "Choose a b",
        "matrix{Col{a#b}}",
        "scale 70 choose a",
        "lpile{scale 70#a}",
    ] {
        assert!(hml_to_tex(native).is_err(), "accepted {native}");
    }
    for tex in [
        r"\begin{array}{}a\end{array}",
        r"\begin{array}{p{2cm}}a\end{array}",
        r"\begin{array}{*{2}{c}}a&b\end{array}",
        r"\begin{array}{l@{}r}a&b\end{array}",
        r"\begin{array}{c}\end{array}",
        r"\begin{array}{c}a&b\end{array}",
        r"\begin{array}{c}a\end{matrix}",
    ] {
        assert!(tex_to_hml(tex).is_err(), "accepted {tex}");
    }
    let native = format!("{}a{}", "lpile{".repeat(120), "}".repeat(120));
    assert!(hml_to_tex(&native).is_err());
    let tex = format!(
        "{}a{}",
        r"\begin{array}{l}".repeat(120),
        r"\end{array}".repeat(120)
    );
    assert!(tex_to_hml(&tex).is_err());
}
