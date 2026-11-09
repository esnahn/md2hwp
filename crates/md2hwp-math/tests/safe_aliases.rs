// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::tex_to_hml;

#[test]
fn safe_aliases_match_existing_conversions() {
    for (alias, canonical) in [
        (r"a\!b", r"a\kern{-1pt}b"),
        (r"a\newline b", r"a\\b"),
        (r"a\newline b\over c", r"a\\b\over c"),
        (
            r"\mathchoice{D}{T}{S}{Q}\newline a\over b",
            r"\mathchoice{D}{T}{S}{Q}\\a\over b",
        ),
        (
            r"\begin{matrix}a\newline b\end{matrix}",
            r"\begin{matrix}a\\b\end{matrix}",
        ),
        (
            r"\begin{array}{c}a\newline b\end{array}",
            r"\begin{array}{c}a\\b\end{array}",
        ),
        (r"\stackrel{x}{y}", r"\overset{x}{y}"),
        (
            r"\underset{x}{\stackrel{y}{\longrightarrow}}",
            r"\underset{x}{\overset{y}{\rightarrow}}",
        ),
        (r"\longleftarrow", r"\leftarrow"),
    ] {
        assert_eq!(
            tex_to_hml(alias).unwrap(),
            tex_to_hml(canonical).unwrap(),
            "{alias}"
        );
    }
}
