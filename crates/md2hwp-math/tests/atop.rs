// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::{hml_to_tex, tex_to_hml};

#[test]
fn native_stacks_emit_tex_primitive_and_keep_binomial_recognition() {
    for (script, tex) in [
        ("x atop y", r"{x \atop y}"),
        ("{a+b} atop {c+d}", r"{a+b \atop c+d}"),
        (
            "{a atop b} atop {c atop d}",
            r"{{a \atop b} \atop {c \atop d}}",
        ),
        ("LEFT ( n atop k RIGHT )", r"\binom{n}{k}"),
        ("( n atop k )", r"\binom{n}{k}"),
    ] {
        let actual = hml_to_tex(script).unwrap();
        let compact = |value: &str| {
            value
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect::<String>()
        };
        assert_eq!(compact(&actual), compact(tex), "{script}: {actual}");
        assert!(tex_to_hml(&actual).is_ok(), "{script}: {actual}");
    }
}

#[test]
fn atop_scopes_the_complete_current_math_list() {
    for (tex, script) in [
        (r"x\atop y", "{{x} atop {y}}"),
        (r"{a+b\atop c+d}", "{{{a + b} atop {c + d}}}"),
        (r"z+{a+b\atop c+d}+w", "z + {{{a + b} atop {c + d}}} + w"),
        (
            r"{{a\atop b}\atop {c\atop d}}",
            "{{{{{{a} atop {b}}}} atop {{{{c} atop {d}}}}}}",
        ),
        (
            r"\frac{a+b\atop c+d}{e}",
            "{{{{a + b} atop {c + d}}} over {e}}",
        ),
        (r"{a_1\atop b^2}_k", "{{{a _{1}} atop {b ^{2}}}} _{k}"),
        (
            r"\left[a+b\atop c+d\right]",
            "{LEFT [ {{a + b} atop {c + d}} RIGHT ]}",
        ),
        (
            r"\begin{matrix}a\atop b&c\\d&e\atop f\end{matrix}",
            "{matrix {{{a} atop {b}} & c # d & {{e} atop {f}}}}",
        ),
    ] {
        assert_eq!(tex_to_hml(tex).unwrap(), script, "{tex}");
    }
}

#[test]
fn annotation_stacks_remain_opaque_composites() {
    assert_eq!(
        tex_to_hml(r"\overset{x}{a\atop b}").unwrap(),
        "{underover {{{a} atop {b}}}^{x}}"
    );
    assert_eq!(
        tex_to_hml(r"\overset{a\atop b}{x}").unwrap(),
        "{underover {x}^{{{a} atop {b}}}}"
    );
    assert_eq!(
        tex_to_hml(r"\overset{x}{\to\atop\leftarrow}").unwrap(),
        "{underover {{{->} atop {<-}}}^{x}}"
    );
    assert_eq!(
        tex_to_hml(r"\overset{x}{\to}").unwrap(),
        "{rel rarrow {x} {}}"
    );
}

#[test]
fn binomial_commands_use_native_binom_and_keep_nested_stack_operands() {
    for command in ["binom", "dbinom", "tbinom"] {
        assert_eq!(
            tex_to_hml(&format!(r"\{command}{{n}}{{k}}_i")).unwrap(),
            "{binom {n} {k}} _{i}"
        );
        assert_eq!(
            tex_to_hml(&format!(r"\{command}{{a\atop b}}{{c}}")).unwrap(),
            "{binom {{{a} atop {b}}} {c}}"
        );
    }
}

#[test]
fn malformed_stacks_are_rejected_at_each_scope_boundary() {
    for tex in [
        r"\atop b",
        r"a\atop",
        r"{\atop b}",
        r"{a\atop}",
        r"a\atop b\atop c",
        r"{a\atop b\atop c}",
        r"\overset{x}{\atop b}",
        r"\overset{x}{a\atop}",
        r"\overset{x}{a\atop b\atop c}",
        r"\overset{\atop b}{x}",
        r"\overset{a\atop}{x}",
        r"\begin{matrix}a\atop&b\end{matrix}",
        r"\begin{matrix}a&\atop b\end{matrix}",
        r"\begin{matrix}a\atop\\b\end{matrix}",
        r"\left(a\atop\right)",
    ] {
        assert!(
            tex_to_hml(tex).unwrap_err().message.contains("\\atop"),
            "{tex}"
        );
    }
}

#[test]
fn stacks_retain_parser_resource_limits() {
    let tex = format!("{}a{}", r"{a\atop ".repeat(120), "}".repeat(120));
    assert!(
        tex_to_hml(&tex)
            .unwrap_err()
            .message
            .contains("nesting limit")
    );
    let tex = format!(r"{}\atop b", "a+".repeat(17_000));
    assert!(tex_to_hml(&tex).unwrap_err().message.contains("atom limit"));
    let tex = format!(r"a\atop {}", "b".repeat(262_144));
    assert!(tex_to_hml(&tex).unwrap_err().message.contains("byte limit"));
}
