// SPDX-License-Identifier: AGPL-3.0-only
use md2hwp_math::hml_to_tex;

#[test]
fn specification_compact_functions_export_as_named_functions() {
    assert_eq!(
        hml_to_tex("f(x)= logx+sinx").unwrap(),
        "f (x) = \\log x + \\sin x"
    );
    for name in [
        "sin", "cos", "coth", "log", "tan", "cot", "ln", "lg", "sec", "cosec", "max", "min", "csc",
        "arcsin", "lim", "Lim", "arccos", "arctan", "exp", "Exp", "arc", "sinh", "det", "gcd",
        "cosh", "tanh", "mod", "if", "for", "and", "hom", "ker", "deg", "arg", "dim", "Pr",
    ] {
        assert_eq!(
            hml_to_tex(&format!("{name}x")).unwrap(),
            hml_to_tex(&format!("{name} x")).unwrap(),
            "{name}"
        );
        let split = name
            .chars()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        assert_eq!(
            hml_to_tex(&format!("{split} x")).unwrap(),
            hml_to_tex(&format!("{name} x")).unwrap(),
            "{split}"
        );
    }
    assert_eq!(hml_to_tex("logsinx").unwrap(), "\\log \\sin x");
}

#[test]
fn native_word_examples_recognize_only_their_leading_functions() {
    // Native rendered evidence confirms these everyday words also use the
    // function prefix; their suffix remains italic and the word binds as one.
    for (word, tex) in [
        ("minimum", r"\min imum"),
        ("format", r"\operatorname{for} mat"),
        ("Prune", r"\Pr une"),
    ] {
        assert_eq!(hml_to_tex(word).unwrap(), tex);
        assert_eq!(
            hml_to_tex(&format!("{word} over y")).unwrap(),
            format!(r"\frac{{{tex}}}{{y}}")
        );
    }
    assert_eq!(hml_to_tex("sinx{a+b}").unwrap(), "\\sin x {a + b}");
}

#[test]
fn longest_function_prefix_wins_before_its_shorter_name() {
    for name in [
        "arcsin", "arccos", "arctan", "sinh", "cosh", "tanh", "coth", "cosec",
    ] {
        assert_eq!(
            hml_to_tex(&format!("{name}x")).unwrap(),
            format!("{} x", hml_to_tex(name).unwrap())
        );
    }
}

#[test]
fn compact_words_keep_fraction_operands_but_script_the_last_letter() {
    assert_eq!(hml_to_tex("sinx over y").unwrap(), "\\frac{\\sin x}{y}");
    assert_eq!(hml_to_tex("{sinx} over {y}").unwrap(), "\\frac{\\sin x}{y}");
    assert_eq!(hml_to_tex("sin x over y").unwrap(), "\\sin \\frac{x}{y}");
    assert_eq!(hml_to_tex("a over sinx").unwrap(), "\\frac{a}{\\sin x}");
    assert_eq!(hml_to_tex("sinx^2").unwrap(), "\\sin x^{2}");
    assert_eq!(hml_to_tex("sin x^2").unwrap(), "\\sin x^{2}");
    assert_eq!(hml_to_tex("abc over xyz").unwrap(), "\\frac{abc}{xyz}");
    assert_eq!(hml_to_tex("분자 over 분모").unwrap(), "\\frac{분자}{분모}");
}

#[test]
fn literal_suffixes_quotes_and_nonprefix_identifiers_are_not_reinterpreted() {
    assert_eq!(hml_to_tex("xsinx").unwrap(), "xsinx");
    assert_eq!(hml_to_tex("logxsinx").unwrap(), "\\log xsinx");
    assert_eq!(
        hml_to_tex("si n, c os, a n d").unwrap(),
        "\\sin , \\cos , \\operatorname{and}"
    );
    assert_eq!(hml_to_tex(r#""logx" "sinx""#).unwrap(), "logx sinx");
    assert_eq!(hml_to_tex("logit logrm").unwrap(), "\\log it \\log rm");
    assert_eq!(
        hml_to_tex("rm logx it logx").unwrap(),
        "\\log \\mathrm{x} \\log x"
    );
    assert_eq!(
        hml_to_tex("rm bold logx it y").unwrap(),
        "\\log \\mathbf{x} y"
    );
    assert_eq!(
        hml_to_tex("Equation font test").unwrap(),
        "Equation font test"
    );
    assert_eq!(hml_to_tex("Sinx").unwrap(), "Sinx");
    for input in ["Forall", "MYSTERY{a}", "UNSUPPORTED{a}", "Sin", "Sqrt"] {
        assert!(hml_to_tex(input).is_err(), "accepted {input}");
    }
}

#[test]
fn split_native_functions_follow_rendered_adjacency() {
    for input in ["si n", "s i n", "{si}n", "s{i}n", "{s i}n", "s{i n}"] {
        assert_eq!(hml_to_tex(input).unwrap(), r"\sin", "{input}");
    }
    for input in ["si nx", "si n x", "s i nx", "s i n x"] {
        assert_eq!(hml_to_tex(input).unwrap(), r"\sin x", "{input}");
    }
    assert_eq!(hml_to_tex("s i n h x").unwrap(), "\\sinh x");
    assert_eq!(hml_to_tex("x si n x").unwrap(), "x si n x");
    assert_eq!(hml_to_tex("x sin x").unwrap(), "x \\sin x");
    assert_eq!(hml_to_tex("x+si n").unwrap(), "x + \\sin");
    assert_eq!(hml_to_tex("si~n").unwrap(), "si \\  n");
    assert_eq!(hml_to_tex(r#"si "n""#).unwrap(), "si n");
    assert_eq!(hml_to_tex(r#""sin""#).unwrap(), "sin");
    assert_eq!(hml_to_tex("s it i n").unwrap(), "s i n");
    assert_eq!(hml_to_tex("s rm i n").unwrap(), "s \\mathrm{i} \\mathrm{n}");
    for input in ["s inx", "s inh x"] {
        assert!(hml_to_tex(input).is_err(), "accepted {input}");
    }
}

#[test]
fn split_functions_respect_native_fraction_and_script_boundaries() {
    assert_eq!(hml_to_tex("si n over y").unwrap(), "si \\frac{n}{y}");
    assert_eq!(hml_to_tex("{si}n over y").unwrap(), "{si} \\frac{n}{y}");
    assert_eq!(hml_to_tex("s{i}n over y").unwrap(), "s i \\frac{n}{y}");
    assert_eq!(hml_to_tex("s i n over y").unwrap(), "s i \\frac{n}{y}");
    assert_eq!(hml_to_tex("si nx over y").unwrap(), "si \\frac{nx}{y}");
    assert_eq!(hml_to_tex("si n x over y").unwrap(), "\\sin \\frac{x}{y}");
    assert_eq!(hml_to_tex("{si n} over y").unwrap(), "\\frac{\\sin}{y}");
    for input in ["si n^2", "s i n^2"] {
        assert_eq!(hml_to_tex(input).unwrap(), r"\sin^{2}", "{input}");
    }
    assert_eq!(hml_to_tex("si n x^2").unwrap(), "\\sin x^{2}");
    assert_eq!(hml_to_tex("si nx^2").unwrap(), "\\sin x^{2}");
    assert_eq!(hml_to_tex("s i^2 n").unwrap(), "s i^{2} n");
}

#[test]
fn long_function_runs_remain_bounded_and_unknown_tails_are_not_rescanned() {
    let run = format!("{}x", "sin".repeat(10_000));
    let out = hml_to_tex(&run).unwrap();
    assert_eq!(out.matches(r"\sin").count(), 10_000);
    assert!(out.ends_with(" x"));
    let plain = "x".repeat(100_000);
    assert_eq!(hml_to_tex(&plain).unwrap(), plain);
    assert!(hml_to_tex(&format!("{}x", "sin ".repeat(32_768))).is_err());
    assert!(hml_to_tex(&format!("{}x", "sin".repeat(90_000))).is_err());
}
