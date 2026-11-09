// SPDX-License-Identifier: AGPL-3.0-only
//! Bounded native Hancom SCRIPT parsing.
// 본 제품은 한글과컴퓨터의 글 문서 파일(.hwp) 공개 문서를 참고하여 개발하였습니다.
// Grammar reference: Hancom Equation revision 1.2, PDF pages 10–16.
// Native toolbar constructor meanings also follow the user's approved 76 pairs.
use crate::is_hangul_letter;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortError(pub String);
impl fmt::Display for PortError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for PortError {}
const MAX_BYTES: usize = 262_144;
const MAX_TOKENS: usize = 32_768;
const MAX_DEPTH: usize = 96;
const MAX_OUTPUT: usize = 4 * 1024 * 1024;
// Native command names must not become italic identifier letters. Exact
// supported forms dispatch before this list; it also reserves their unsupported
// spellings. Ordinary identifiers (ab, mn, Equation, etc.) remain valid.
const UNSUPPORTED_WORDS: &[&str] = &[
    "not",
    "bigg",
    "small",
    "rel",
    "buildrel",
    "coth",
    "lg",
    "cosec",
    "max",
    "min",
    "arcsin",
    "arccos",
    "arctan",
    "arc",
    "det",
    "gcd",
    "mod",
    "if",
    "for",
    "and",
    "hom",
    "ker",
    "deg",
    "arg",
    "dim",
    "Pr",
    "neq",
    "notin",
    "vartheta",
    "varpi",
    "varsigma",
    "varphi",
    "varepsilon",
    // Reserve the documented vocabulary (Equation revision 1.2, PDF pp12–15),
    // including unsupported case variants. Supported exact spellings dispatch
    // before this guard; other spellings must not render as identifier letters.
    "omicron",
    "varupsilon",
    "aleph",
    "hbar",
    "imath",
    "jmath",
    "ohm",
    "ell",
    "liter",
    "wp",
    "imag",
    "angstrom",
    "cap",
    "sqcap",
    "sqcup",
    "oplus",
    "ominus",
    "otimes",
    "odot",
    "oslash",
    "vee",
    "subset",
    "supset",
    "subseteq",
    "supseteq",
    "in",
    "owns",
    "sqsubset",
    "sqsupset",
    "sqsubseteq",
    "sqsupseteq",
    "leq",
    "prec",
    "succ",
    "uplus",
    "plusminus",
    "minusplus",
    "div",
    "circ",
    "ast",
    "star",
    "bigcirc",
    "emptyset",
    "therefore",
    "because",
    "identical",
    "exist",
    "doteq",
    "image",
    "reimage",
    "sim",
    "approx",
    "simeq",
    "cong",
    "equiv",
    "asymp",
    "iso",
    "diamond",
    "dsum",
    "forall",
    "prime",
    "partial",
    "lnot",
    "propto",
    "xor",
    "triangled",
    "dagger",
    "ddagger",
    "larrow",
    "uparrow",
    "downarrow",
    "udarrow",
    "nwarrow",
    "searrow",
    "nearrow",
    "swarrow",
    "hookleft",
    "hookright",
    "mapsto",
    "vert",
    "cdots",
    "ldots",
    "vdots",
    "ddots",
    "triangle",
    "angle",
    "msangle",
    "sangle",
    "rtangle",
    "vdash",
    "hleft",
    "bot",
    "top",
    "models",
    "laplace",
    "centigrade",
    "fahrenheit",
    "lslant",
    "rslant",
    "att",
    "hund",
    "thou",
    "well",
    "base",
    "benzene",
    "smallsum",
    "smallprod",
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Word,
    Number,
    Symbol,
    Quoted,
}
struct Token {
    text: String,
    offset: usize,
    end: usize,
    kind: Kind,
}
// Only exact, explicitly verified native spellings alias. Meaningful case
// distinctions remain intact, and quoted text never passes through this map.
fn command_alias(word: &str) -> Option<&'static str> {
    Some(match word {
        // Preserve the intended bold Roman meaning for both combined forms.
        // Native Hancom 2020 renders boldrm as ordinary Roman.
        "rmbold" => "boldrm",
        "ASSERT" => "VDASH",
        "CUP" => "SMALLUNION",
        "del" => "nabla",
        "dline" => "VERT",
        "DOTSAXIS" => "CDOTS",
        "DOTSDIAG" => "DDOTS",
        "DOTSLOW" => "LDOTS",
        "DOTSVERT" => "VDOTS",
        "GGG" => ">>>",
        "INFINITY" | "INFTY" => "INF",
        "LAND" => "WEDGE",
        "NEG" => "LNOT",
        "NI" => "OWNS",
        "overline" => "bar",
        "PREP" => "BOT",
        "underline" => "under",
        "OVER" => "over",
        "ATOP" => "atop",
        "SQRT" => "sqrt",
        "ROOT" => "root",
        "OF" => "of",
        "INT" => "int",
        "SUM" => "sum",
        "UNION" => "union",
        "INTER" => "inter",
        "PROD" => "prod",
        "COPROD" => "coprod",
        "left" => "LEFT",
        "right" => "RIGHT",
        "OINT" => "oint",
        "DINT" => "dint",
        "TINT" => "tint",
        "ODINT" => "odint",
        "OTINT" => "otint",
        "MATRIX" => "matrix",
        "PMATRIX" => "pmatrix",
        "BMATRIX" => "bmatrix",
        "DMATRIX" => "dmatrix",
        "CASES" => "cases",
        "PILE" => "pile",
        "LPILE" => "lpile",
        "RPILE" => "rpile",
        "COL" => "col",
        "LCOL" => "lcol",
        "RCOL" => "rcol",
        "CHOOSE" => "choose",
        "BINOM" => "binom",
        "EQALIGN" => "eqalign",
        "SUP" => "sup",
        "SUB" => "sub",
        "FROM" => "from",
        "TO" => "to",
        "VEC" => "vec",
        "Vec" => "vec",
        "DYAD" => "dyad",
        "ACUTE" => "acute",
        "GRAVE" => "grave",
        "DOT" => "dot",
        "DDOT" => "ddot",
        "UNDER" => "under",
        "BAR" => "bar",
        "HAT" => "hat",
        "CHECK" => "check",
        "ARCH" => "arch",
        "TILDE" => "tilde",
        "ALPHA" => "Alpha",
        "BETA" => "Beta",
        "GAMMA" => "Gamma",
        "DELTA" => "Delta",
        "EPSILON" => "Epsilon",
        "ZETA" => "Zeta",
        "ETA" => "Eta",
        "THETA" => "Theta",
        "IOTA" => "Iota",
        "KAPPA" => "Kappa",
        "LAMBDA" => "Lambda",
        "MU" => "Mu",
        "NU" => "Nu",
        "XI" => "Xi",
        "PI" => "Pi",
        "RHO" => "Rho",
        "SIGMA" => "Sigma",
        "TAU" => "Tau",
        "UPSILON" => "Upsilon",
        "PHI" => "Phi",
        "CHI" => "Chi",
        "PSI" => "Psi",
        "OMEGA" => "Omega",
        "OMICRON" => "Omicron",
        "Downarrow" => "DOWNARROW",
        "Larrow" => "LARROW",
        "Lrarrow" => "LRARROW",
        "Rarrow" => "RARROW",
        "Udarrow" => "UDARROW",
        "Uparrow" => "UPARROW",
        "Vert" => "VERT",
        "centigrade" => "CENTIGRADE",
        "fahrenheit" => "FAHRENHEIT",
        "laplace" => "LAPLACE",
        _ => return None,
    })
}
// Exact names from the specification's basic-function/reserved-word list.
// Share this vocabulary between whole-word dispatch and compact-word prefixes.
const FUNCTION_NAMES: &[&str] = &[
    "sin", "cos", "coth", "log", "tan", "cot", "ln", "lg", "sec", "cosec", "max", "min", "csc",
    "arcsin", "lim", "Lim", "arccos", "arctan", "exp", "Exp", "arc", "sinh", "det", "gcd", "cosh",
    "tanh", "mod", "if", "for", "and", "hom", "ker", "deg", "arg", "dim", "Pr",
];
fn function_tex(name: &str) -> Option<String> {
    if !FUNCTION_NAMES.contains(&name) {
        return None;
    }
    Some(match name {
        // Preserve native names without a matching standalone core TeX command.
        "cosec" | "Exp" | "arc" | "mod" | "if" | "for" | "and" => {
            format!("\\operatorname{{{name}}}")
        }
        // Preserve the existing native Lim canonicalization separately from lim.
        "Lim" => "\\operatorname*{Lim}".into(),
        _ => format!("\\{name}"),
    })
}
fn function_prefix(word: &str) -> Option<&'static str> {
    // Examine only fixed-size known prefixes, never scan and rewind the run.
    // Longest match keeps sinh/cosec/arcsin ahead of sin/cos/arc.
    FUNCTION_NAMES
        .iter()
        .copied()
        .filter(|name| word.starts_with(name))
        .max_by_key(|name| name.len())
}
// Additional exact spellings from Hancom 2020's equation autocomplete catalog.
// These are guards for literal output, not additional supported import grammar.
const ADDITIONAL_NATIVE_WORDS: &[&str] = &[
    "ASSERT",
    "BIGVEE",
    "BIGWEDGE",
    "boldrm",
    "CUP",
    "DASHV",
    "del",
    "dirac",
    "dline",
    "DOTSAXIS",
    "DOTSDIAG",
    "DOTSLOW",
    "DOTSVERT",
    "exrel",
    "GGG",
    "HCROSS",
    "HDOWN",
    "HRIGHT",
    "HUP",
    "INFINITY",
    "INFTY",
    "ladder",
    "LAND",
    "ldirac",
    "ldrel",
    "LLL",
    "longdiv",
    "lrdrel",
    "lrel",
    "MASSERT",
    "MHO",
    "NEG",
    "NI",
    "NIN",
    "NOWNS",
    "NSUBSET",
    "NSUBSETEQ",
    "NSUPSET",
    "NSUPSETEQ",
    "overline",
    "PHOR",
    "PREP",
    "rdirac",
    "rdrel",
    "ROUND",
    "rrel",
    "SDOTS",
    "sladder",
    "SMALLCOPROD",
    "string",
    "TRIANGLEL",
    "TRIANGLER",
];
fn is_native_word_spelling(word: &str) -> bool {
    command_alias(word).is_some()
        || function_tex(word).is_some()
        || scalar_symbol(word).is_some()
        || UNSUPPORTED_WORDS
            .iter()
            .any(|name| name.eq_ignore_ascii_case(word))
        || ADDITIONAL_NATIVE_WORDS
            .iter()
            .any(|name| name.eq_ignore_ascii_case(word))
        || unsupported_spelling(word)
        || matches!(
            word,
            "OVER" | "ATOP" | "CHOOSE" | "SUB" | "SUP" | "FROM" | "TO" | "OF" | "LEFT" | "RIGHT"
        )
}
// Native recognition also spans whitespace and transparent groups. TeX source
// letters use this vocabulary only to decide whether SCRIPT needs quoting;
// this does not expand the supported native-to-TeX grammar.
pub(crate) fn literal_needs_quotes(word: &str) -> bool {
    (word.len() > 1 && word.bytes().all(|c| c.is_ascii_uppercase()))
        || function_prefix(word).is_some()
        || ["asin", "acos", "atan", "in"].iter().any(|name| word.starts_with(name))
        // Native command recognition consumes case-sensitive prefixes, e.g.
        // alphax, sumx, intx, sqrtx and rmx. Checking fixed-size prefixes keeps
        // this bounded even for a very long source identifier.
        || (1..=word.len().min(32)).any(|end| {
            word.get(..end).is_some_and(|prefix| {
                scalar_symbol(prefix).is_some() || command_alias(prefix).is_some()
            })
        })
        || ["rm", "it", "bold", "boldrm", "scale", "sqrt", "root", "of",
            "over", "atop", "sub", "sup", "from", "to", "matrix", "pmatrix",
            "bmatrix", "dmatrix", "cases", "pile", "lpile", "rpile", "col", "lcol", "rcol", "choose", "binom", "eqalign", "vec", "dyad",
            "acute", "grave", "dot", "ddot", "under", "bar", "hat", "check",
            "arch", "tilde", "box", "BOX", "underover", "UNDEROVER", "rel", "REL", "buildrel", "BUILDREL",
            "color", "COLOR", "LEFT", "RIGHT", "bigg", "BIGG", "lsup", "lsub", "LSUP", "LSUB",
            "OVERBRACE", "UNDERBRACE", "odint", "otint", "rarrow", "lrarrow",
            "EXARROW", "inf"]
            .iter().any(|name| word.starts_with(name))
        || scalar_symbol(word).is_some()
        || unsupported_spelling(word)
        || UNSUPPORTED_WORDS.iter().any(|name| word.starts_with(name) || name.eq_ignore_ascii_case(word))
        || ADDITIONAL_NATIVE_WORDS.iter().any(|name| word.starts_with(name) || name.eq_ignore_ascii_case(word))
        || (word.len() >= 4 && word.starts_with("Vec") && word.as_bytes()[3].is_ascii_uppercase())
}
fn error(offset: usize, message: impl AsRef<str>) -> PortError {
    PortError(format!("SCRIPT at byte {offset}: {}", message.as_ref()))
}
fn lex(input: &str) -> Result<Vec<Token>, PortError> {
    let mut tokens = Vec::new();
    let mut pos = 0;
    while pos < input.len() {
        let rest = &input[pos..];
        let c = rest.chars().next().expect("in bounds");
        if c.is_control() && !matches!(c, '\n' | '\r' | '\t') {
            return Err(error(pos, "control character in equation"));
        }
        if c.is_whitespace() {
            pos += c.len_utf8();
            continue;
        }
        let start = pos;
        let (text, kind) = if c == '"' {
            pos += 1;
            let body = pos;
            while pos < input.len() && !input[pos..].starts_with('"') {
                let ch = input[pos..].chars().next().expect("in bounds");
                if ch.is_control() {
                    return Err(error(pos, "control character in quoted text"));
                }
                pos += ch.len_utf8();
            }
            if pos == input.len() {
                return Err(error(start, "unclosed quoted text"));
            }
            let text = input[body..pos].to_owned();
            pos += 1;
            (text, Kind::Quoted)
        } else if c.is_ascii_alphabetic() {
            while pos < input.len() && input.as_bytes()[pos].is_ascii_alphabetic() {
                pos += 1;
            }
            // Hancom accepts compact vector notation. A blackboard-vector
            // scalar has precedence over the three-letter vector prefix, so
            // VecAB is VecA followed by B. Split each recognized prefix at
            // its original byte boundary; the remaining ASCII word is then
            // scanned by the same rules.
            let word_end = pos;
            let mut part_start = start;
            while part_start < word_end {
                let rest = &input[part_start..word_end];
                if rest.len() >= 4
                    && rest.starts_with("Vec")
                    && rest.as_bytes()[3].is_ascii_uppercase()
                {
                    let symbol_end = part_start + 4;
                    let symbol = &input[part_start..symbol_end];
                    tokens.push(Token {
                        text: symbol.into(),
                        offset: part_start,
                        end: symbol_end,
                        kind: Kind::Word,
                    });
                    if tokens.len() > MAX_TOKENS {
                        return Err(error(part_start, "token limit exceeded"));
                    }
                    part_start = symbol_end;

                    // Hancom inserts an unknown all-caps remainder as
                    // individual variables: VecABC is VecA followed by B and
                    // C. Recognized commands and symbols retain normal token
                    // handling, so VecAALPHA keeps ALPHA intact.
                    let tail = &input[part_start..word_end];
                    if tail.bytes().all(|byte| byte.is_ascii_uppercase())
                        && !is_native_word_spelling(tail)
                    {
                        while part_start < word_end {
                            let letter_end = part_start + 1;
                            tokens.push(Token {
                                text: input[part_start..letter_end].into(),
                                offset: part_start,
                                end: letter_end,
                                kind: Kind::Word,
                            });
                            if tokens.len() > MAX_TOKENS {
                                return Err(error(part_start, "token limit exceeded"));
                            }
                            part_start = letter_end;
                        }
                    }
                    continue;
                }
                let prefix = if rest.len() > 3
                    && (rest.starts_with("vec")
                        || rest.starts_with("Vec")
                        || rest.starts_with("VEC"))
                {
                    Some(3)
                } else {
                    None
                };
                let part_end = part_start + prefix.unwrap_or(rest.len());
                let source = &input[part_start..part_end];
                let text = command_alias(source).unwrap_or(source).to_owned();
                tokens.push(Token {
                    text,
                    offset: part_start,
                    end: part_end,
                    kind: Kind::Word,
                });
                if tokens.len() > MAX_TOKENS {
                    return Err(error(part_start, "token limit exceeded"));
                }
                part_start = part_end;
            }
            continue;
        } else if is_hangul_letter(c) {
            // A native word is one OVER operand. ASCII commands, digits and
            // punctuation terminate the run rather than being swallowed by it.
            while pos < input.len() {
                let next = input[pos..].chars().next().expect("in bounds");
                if !is_hangul_letter(next) {
                    break;
                }
                pos += next.len_utf8();
            }
            (input[start..pos].to_owned(), Kind::Word)
        } else if c.is_ascii_digit() {
            pos += 1;
            while pos < input.len() && input.as_bytes()[pos].is_ascii_digit() {
                pos += 1;
            }
            if pos < input.len()
                && input.as_bytes()[pos] == b'.'
                && input
                    .as_bytes()
                    .get(pos + 1)
                    .is_some_and(u8::is_ascii_digit)
            {
                pos += 1;
                while pos < input.len() && input.as_bytes()[pos].is_ascii_digit() {
                    pos += 1;
                }
            }
            (input[start..pos].to_owned(), Kind::Number)
        } else if rest.starts_with("&amp;") {
            pos += 5;
            ("&".into(), Kind::Symbol)
        } else if let Some(op) = [
            "<->", "<<<", ">>>", "->", "<-", "+-", "-+", "!=", "==", "<=", ">=", "<<", ">>", "||",
            "~~",
        ]
        .into_iter()
        .find(|op| rest.starts_with(op))
        {
            pos += op.len();
            (op.into(), Kind::Symbol)
        } else {
            pos += c.len_utf8();
            (c.to_string(), Kind::Symbol)
        };
        let text = if kind == Kind::Word {
            match command_alias(&text) {
                Some(alias) => alias.to_owned(),
                None => text,
            }
        } else {
            text
        };
        tokens.push(Token {
            text,
            offset: start,
            end: pos,
            kind,
        });
        if tokens.len() > MAX_TOKENS {
            return Err(error(start, "token limit exceeded"));
        }
    }
    Ok(tokens)
}

enum NodeDetails {
    SideOperator,
    Atop(String, String),
    UprightName(String),
    Letters(LetterRun),
}

#[derive(Clone)]
struct LetterRun {
    text: String,
    literal: String,
    font: FontState,
    font_generation: usize,
    // A right script terminates horizontal recognition, but its base still
    // participates in the preceding run. Track its base start in raw letters;
    // ungrouped ordinary words use the final letter, explicit groups use zero.
    script: Option<(usize, String)>,
}

struct Node {
    tex: String,
    group: Option<String>,
    atomic: bool,
    // Ungrouped native letter words put right scripts on their last visible
    // letter (or final named function), while OVER still consumes the word.
    right_script_base: Option<(String, usize)>,
    // Keep infrequent structural metadata off the bounded recursive stack.
    details: Option<Box<NodeDetails>>,
}
impl Node {
    fn atom(tex: impl Into<String>) -> Self {
        Self {
            tex: tex.into(),
            group: None,
            atomic: true,
            right_script_base: None,
            details: None,
        }
    }
    fn ordinary(tex: String) -> Self {
        let atomic = tex.chars().count() == 1;
        Self {
            tex,
            atomic,
            right_script_base: None,
            group: None,
            details: None,
        }
    }
    fn argument(&self) -> &str {
        self.group.as_deref().unwrap_or(&self.tex)
    }
    fn bound(&self) -> String {
        if self.atomic {
            self.tex.clone()
        } else {
            format!("{{{}}}", self.tex)
        }
    }
}
struct Expr {
    nodes: Vec<Node>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum FontState {
    Italic,
    Roman,
    BoldItalic,
    BoldRoman,
}
impl FontState {
    fn with_bold(self) -> Self {
        // Weight is applied to the current shape for one primary operand.
        match self {
            Self::Italic | Self::BoldItalic => Self::BoldItalic,
            Self::Roman | Self::BoldRoman => Self::BoldRoman,
        }
    }
    fn is_bold(self) -> bool {
        matches!(self, Self::BoldItalic | Self::BoldRoman)
    }
    fn command(self) -> &'static str {
        match self {
            Self::Italic => "mathit",
            Self::Roman => "mathrm",
            Self::BoldItalic => "boldsymbol",
            Self::BoldRoman => "mathbf",
        }
    }
}
impl Expr {
    fn tex(&self) -> String {
        self.nodes
            .iter()
            .map(|n| n.tex.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Convert the supported native Hancom equation SCRIPT grammar to TeX.
/// Unknown constructors and malformed or excessive input return explicit errors.
pub fn hml_to_tex(input: &str) -> Result<String, PortError> {
    if input.len() > MAX_BYTES {
        return Err(error(0, "input byte limit exceeded"));
    }
    let tokens = lex(input)?;
    let mut parser = Parser {
        tokens,
        pos: 0,
        depth: 0,
        source_len: input.len(),
        font: FontState::Italic,
        font_generation: 0,
        pending_bold: false,
        active_bold: false,
        constructor_operand: false,
        pending_style: None,
        expression_depth: 0,
        delimiter_body_depth: None,
        colored_right: None,
    };
    let rows = parser.rows(None)?;
    if parser.pos != parser.tokens.len() {
        return parser.fail("unexpected trailing token");
    }
    let text = if rows.len() == 1 && rows[0].len() == 1 {
        let text = rows[0][0].tex();
        if text.is_empty() {
            return parser.fail("empty math expression");
        }
        text
    } else {
        parser.environment_tex("aligned", rows)?
    };
    if text.len() > MAX_OUTPUT {
        return parser.fail("output byte limit exceeded");
    }
    Ok(text)
}
struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    depth: usize,
    source_len: usize,
    // Shape declarations persist across groups; bold is scoped to a primary.
    font: FontState,
    pending_bold: bool,
    active_bold: bool,
    font_generation: usize,
    // Uppercase variable words are valid operands of known constructors.
    // This context also follows explicit grouping inside such an operand.
    constructor_operand: bool,
    pending_style: Option<&'static str>,
    expression_depth: usize,
    delimiter_body_depth: Option<usize>,
    colored_right: Option<&'static str>,
}
impl Parser {
    fn offset(&self) -> usize {
        self.tokens
            .get(self.pos)
            .map_or(self.source_len, |t| t.offset)
    }
    fn fail<T>(&self, message: impl AsRef<str>) -> Result<T, PortError> {
        Err(error(self.offset(), message))
    }
    fn peek(&self) -> Option<&str> {
        self.tokens.get(self.pos).map(|t| t.text.as_str())
    }
    fn eat(&mut self, text: &str) -> bool {
        if self.peek() == Some(text) && !self.quoted() {
            self.pos += 1;
            true
        } else {
            false
        }
    }
    fn quoted(&self) -> bool {
        self.tokens
            .get(self.pos)
            .is_some_and(|t| t.kind == Kind::Quoted)
    }
    fn require(&mut self, text: &str) -> Result<(), PortError> {
        if self.eat(text) {
            Ok(())
        } else {
            self.fail(format!("expected {text}"))
        }
    }
    fn enter(&mut self) -> Result<(), PortError> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            self.fail("nesting limit exceeded")
        } else {
            Ok(())
        }
    }
    fn effective_font(&self) -> FontState {
        if self.active_bold {
            self.font.with_bold()
        } else {
            self.font
        }
    }
    fn declarations(
        &mut self,
        allow_trailing_scale: bool,
        allow_bold: bool,
    ) -> Result<(), PortError> {
        let mut scaled = false;
        while let Some(token) = self.tokens.get(self.pos) {
            if token.kind != Kind::Word {
                break;
            }
            match token.text.as_str() {
                "rm" => {
                    self.font = FontState::Roman;
                    self.pending_bold = false;
                }
                "it" => {
                    self.font = FontState::Italic;
                    self.pending_bold = false;
                }
                "bold" if allow_bold => self.pending_bold = true,
                "boldrm" if allow_bold => {
                    self.font = FontState::Roman;
                    self.pending_bold = true;
                }
                "scale" => {
                    // Preserve a bounded SCALE as the nearest TeX math style.
                    // Declaration runs remain iterative and preserve font state.
                    self.pos += 1;
                    let size = self.tokens.get(self.pos).ok_or_else(|| {
                        error(self.offset(), "scale requires a positive numeric size")
                    })?;
                    if size.kind != Kind::Number
                        || !size
                            .text
                            .parse::<f64>()
                            .is_ok_and(|n| n.is_finite() && n > 0.0)
                    {
                        return self.fail("scale requires a positive numeric size");
                    }
                    let end = size.offset + size.text.len();
                    let scale = size.text.parse::<f64>().expect("validated scale");
                    self.pending_style = Some(if scale < 60.0 {
                        "scriptscriptstyle"
                    } else if scale < 85.0 {
                        "scriptstyle"
                    } else {
                        "textstyle"
                    });
                    self.pos += 1;
                    if self.tokens.get(self.pos).is_some_and(|next| {
                        next.offset == end && (next.text == "." || next.kind == Kind::Number)
                    }) {
                        return self.fail("malformed scale size");
                    }
                    scaled = true;
                    continue;
                }
                _ => break,
            }
            self.font_generation += 1;
            self.pos += 1;
        }
        // Require a body for leading SCALE declarations. A trailing restore
        // after existing math is allowed at the end of its scope.
        if scaled
            && !allow_trailing_scale
            && (self.peek().is_none()
                || (!self.quoted()
                    && matches!(
                        self.peek(),
                        Some(
                            "}" | ")"
                                | "]"
                                | "RIGHT"
                                | "&"
                                | "#"
                                | "_"
                                | "^"
                                | "sub"
                                | "sup"
                                | "LSUP"
                                | "LSUB"
                                | "lsup"
                                | "lsub"
                                | "from"
                                | "to"
                                | "of"
                                | "over"
                                | "atop"
                                | "choose"
                        )
                    )))
        {
            return self.fail("scale requires following math");
        }
        Ok(())
    }
    fn ordinary(&self, text: String) -> Node {
        Self::ordinary_with_font(text, self.effective_font())
    }
    fn styled_node(node: Node, style: Option<&str>) -> Node {
        let Some(style) = style else {
            return node;
        };
        let body = format!("\\{style} {}", node.argument());
        let mut result = Node::atom(format!("{{{body}}}"));
        result.group = Some(body);
        result
    }
    fn ordinary_with_font(text: String, font: FontState) -> Node {
        if (font == FontState::Roman && text.chars().any(char::is_alphabetic))
            || (font.is_bold() && text.chars().any(char::is_alphanumeric))
        {
            Node::atom(format!("\\{}{{{text}}}", font.command()))
        } else {
            Node::ordinary(text)
        }
    }
    fn letter_script_base(text: &str, font: FontState) -> String {
        let last = text.len() - 1; // Called only for nonempty ASCII words.
        if font == FontState::Italic || last == 0 {
            return Self::ordinary_with_font(text.into(), font).tex;
        }
        format!(
            "{} {}",
            Self::ordinary_with_font(text[..last].into(), font).tex,
            Self::ordinary_with_font(text[last..].into(), font).tex
        )
    }
    fn expression(&mut self, stops: &[&str]) -> Result<Expr, PortError> {
        self.expression_depth += 1;
        let result = self.expression_inner(stops);
        self.expression_depth -= 1;
        result
    }
    fn expression_inner(&mut self, stops: &[&str]) -> Result<Expr, PortError> {
        let mut nodes = Vec::new();
        let mut size = 0;
        loop {
            if self.colored_right.is_some() {
                break;
            }
            self.declarations(!nodes.is_empty(), true)?;
            if let Some(style) = self.pending_style.take() {
                nodes.push(Node::atom(format!("\\{style}")));
            }
            let Some(token) = self.peek() else { break };
            if !self.quoted() && stops.contains(&token) {
                break;
            }
            let units = self.unit()?;
            size += units.iter().map(|node| node.tex.len() + 1).sum::<usize>();
            if size > MAX_OUTPUT {
                return self.fail("output byte limit exceeded");
            }
            nodes.extend(units);
        }
        Ok(Expr {
            nodes: self.function_runs(nodes)?,
        })
    }
    fn argument(&mut self) -> Result<Node, PortError> {
        self.declarations(false, true)?;
        let style = self.pending_style.take();
        if self.peek().is_none() {
            return self.fail("missing argument");
        }
        self.enter()?;
        let previous_bold = self.active_bold;
        let pending_bold = std::mem::take(&mut self.pending_bold);
        self.active_bold |= pending_bold;
        if pending_bold && !previous_bold {
            self.font_generation += 1;
        }
        let result = self.primary();
        if pending_bold && !previous_bold {
            self.font_generation += 1;
        }
        self.active_bold = previous_bold;
        self.depth -= 1;
        result.map(|node| Self::styled_node(node, style))
    }
    fn braced_argument(&mut self) -> Result<Node, PortError> {
        if self.peek() != Some("{") || self.quoted() {
            return self.fail("expected braced argument");
        }
        self.argument()
    }
    fn constructor_argument(&mut self) -> Result<Node, PortError> {
        let previous = self.constructor_operand;
        self.constructor_operand = true;
        let result = self.argument();
        self.constructor_operand = previous;
        result
    }
    fn group(&mut self) -> Result<Node, PortError> {
        let expression = self.expression(&["}"])?;
        self.require("}")?;
        if self.colored_right.is_some() {
            return Ok(Node::ordinary(expression.tex()));
        }
        if expression.nodes.len() == 1 && expression.nodes[0].atomic {
            let mut node = expression.nodes.into_iter().next().expect("one node");
            node.right_script_base = None;
            if matches!(
                node.tex.as_str(),
                r"\sum"
                    | r"\prod"
                    | r"\coprod"
                    | r"\bigcap"
                    | r"\bigcup"
                    | r"\bigsqcap"
                    | r"\bigsqcup"
                    | r"\bigoplus"
                    | r"\bigotimes"
                    | r"\bigodot"
                    | r"\bigvee"
                    | r"\bigwedge"
                    | r"\biguplus"
                    | r"\int"
                    | r"\oint"
                    | r"\iint"
                    | r"\iiint"
                    | r"\oiint"
                    | r"\oiiint"
                    | r"\lim"
                    | r"\limsup"
                    | r"\liminf"
                    | r"\max"
                    | r"\min"
                    | r"\sup"
                    | r"\inf"
            ) {
                node.details = Some(Box::new(NodeDetails::SideOperator));
            }
            return Ok(node);
        }
        let letters = if expression.nodes.len() == 1 {
            match expression.nodes[0].details.as_deref() {
                Some(NodeDetails::Letters(run)) => {
                    let mut run = run.clone();
                    run.literal = format!("{{{}}}", run.literal);
                    Some(run)
                }
                _ => None,
            }
        } else {
            None
        };
        let body = expression.tex();
        Ok(Node {
            tex: format!("{{{body}}}"),
            group: Some(body),
            atomic: true,
            right_script_base: None,
            details: letters.map(|run| Box::new(NodeDetails::Letters(run))),
        })
    }
    fn unit(&mut self) -> Result<Vec<Node>, PortError> {
        self.enter()?;
        let nodes = self.lexical_nodes(true)?;
        if self.quoted() || !matches!(self.peek(), Some("over" | "atop" | "choose")) {
            self.depth -= 1;
            return Ok(nodes);
        }
        let mut node = self.combine_operand(nodes)?;
        // Native OVER operands are contiguous lexical runs, including their
        // arithmetic operators and postfix scripts. Whitespace and explicit
        // groups delimit those runs; a grouped operand is its own island.
        while !self.quoted() && matches!(self.peek(), Some("over" | "atop" | "choose")) {
            let choose = self.eat("choose");
            let atop = !choose && self.eat("atop");
            if !atop && !choose {
                self.require("over")?;
            }
            let right = self.lexical_operand(true)?;
            let a = node.argument().to_owned();
            let b = right.argument().to_owned();
            if choose && (a.is_empty() || b.is_empty()) {
                return self.fail("empty binomial operand");
            }
            node = if choose {
                Node::atom(format!("\\binom{{{a}}}{{{b}}}"))
            } else if atop {
                let mut out = Node::atom(format!("{{{a} \\atop {b}}}"));
                out.details = Some(Box::new(NodeDetails::Atop(a, b)));
                out
            } else {
                Node::atom(format!("\\frac{{{a}}}{{{b}}}"))
            };
            node = self.postfix(node)?;
        }
        self.depth -= 1;
        Ok(vec![node])
    }
    fn lexical_operand(&mut self, scripts: bool) -> Result<Node, PortError> {
        let nodes = self.lexical_nodes(scripts)?;
        self.combine_operand(nodes)
    }
    fn lexical_nodes(&mut self, scripts: bool) -> Result<Vec<Node>, PortError> {
        self.declarations(false, true)?;
        let style = self.pending_style.take();
        let grouped = !self.quoted() && matches!(self.peek(), Some("{" | "(" | "[" | "LEFT"));
        let first = self.argument()?;
        let first = if scripts { self.postfix(first)? } else { first };
        if grouped {
            return Ok(vec![Self::styled_node(first, style)]);
        }
        let mut nodes = vec![first];
        let mut size = nodes[0].tex.len();
        while let Some(next) = self.tokens.get(self.pos) {
            let previous = &self.tokens[self.pos - 1];
            if next.offset != previous.end
                || (!scripts
                    && next.kind != Kind::Quoted
                    && matches!(
                        next.text.as_str(),
                        "`" | "~"
                            | "~~"
                            | "sqrt"
                            | "root"
                            | "binom"
                            | "LEFT"
                            | "bigg"
                            | "BIGG"
                            | "matrix"
                            | "pmatrix"
                            | "bmatrix"
                            | "dmatrix"
                            | "cases"
                            | "pile"
                            | "lpile"
                            | "rpile"
                            | "eqalign"
                            | "REL"
                            | "rel"
                            | "BUILDREL"
                            | "buildrel"
                            | "not"
                            | "NOT"
                            | "COLOR"
                            | "color"
                            | "vec"
                            | "dyad"
                            | "acute"
                            | "grave"
                            | "dot"
                            | "ddot"
                            | "under"
                            | "bar"
                            | "hat"
                            | "check"
                            | "arch"
                            | "tilde"
                            | "BOX"
                            | "box"
                            | "OVERBRACE"
                            | "UNDERBRACE"
                            | "UNDEROVER"
                            | "underover"
                    ))
                || (next.kind != Kind::Quoted
                    && matches!(
                        next.text.as_str(),
                        "{" | "("
                            | "["
                            | "LEFT"
                            | "}"
                            | ")"
                            | "]"
                            | "RIGHT"
                            | "&"
                            | "#"
                            | "_"
                            | "^"
                            | "sub"
                            | "sup"
                            | "LSUP"
                            | "LSUB"
                            | "lsup"
                            | "lsub"
                            | "from"
                            | "to"
                            | "of"
                            | "over"
                            | "atop"
                            | "choose"
                            | "col"
                            | "lcol"
                            | "rcol"
                            | "rm"
                            | "it"
                            | "bold"
                            | "boldrm"
                            | "scale"
                    ))
            {
                break;
            }
            let node = self.argument()?;
            let node = if scripts { self.postfix(node)? } else { node };
            size += node.tex.len() + 1;
            if size > MAX_OUTPUT {
                return self.fail("output byte limit exceeded");
            }
            nodes.push(node);
        }
        if style.is_some() {
            Ok(vec![Self::styled_node(self.combine_operand(nodes)?, style)])
        } else {
            Ok(nodes)
        }
    }
    fn combine_operand(&self, mut nodes: Vec<Node>) -> Result<Node, PortError> {
        if nodes.len() == 1 {
            return Ok(nodes.pop().expect("one operand"));
        }
        let nodes = self.function_runs(nodes)?;
        Ok(Node::ordinary(Expr { nodes }.tex()))
    }
    fn postfix(&mut self, mut node: Node) -> Result<Node, PortError> {
        let mut sub = None;
        let mut sup = None;
        let mut left_sub = None;
        let mut left_sup = None;
        loop {
            self.declarations(true, false)?;
            if self
                .tokens
                .get(self.pos)
                .is_some_and(|t| t.kind == Kind::Quoted)
            {
                break;
            }
            let kind = match self.peek() {
                Some("_" | "sub") => 0,
                Some("^" | "sup") => 1,
                Some("LSUB" | "lsub") => 2,
                Some("LSUP" | "lsup") => 3,
                Some("from") => 4,
                _ => break,
            };
            self.pos += 1;
            if kind == 4 {
                if sub.is_some() || sup.is_some() {
                    return self.fail("duplicate from/to or right scripts");
                }
                let lower = self.expression(&["to", "}", "#", "&", "RIGHT"])?;
                if lower.nodes.is_empty() {
                    return self.fail("empty lower limit");
                }
                self.require("to")?;
                let upper = self.argument()?;
                sub = Some(lower.tex());
                sup = Some(upper.argument().to_owned());
                continue;
            }
            let arg = self.lexical_operand(false)?.argument().to_owned();
            if arg.is_empty() {
                return self.fail("empty script argument");
            }
            let slot = match kind {
                0 => &mut sub,
                1 => &mut sup,
                2 => &mut left_sub,
                _ => &mut left_sup,
            };
            if slot.replace(arg).is_some() {
                return self.fail("duplicate script");
            }
        }
        if sub.is_some() || sup.is_some() || left_sub.is_some() || left_sup.is_some() {
            if (sub.is_some() || sup.is_some())
                && matches!(node.details.as_deref(), Some(NodeDetails::SideOperator))
            {
                node.tex = format!("{{{}}}", node.tex);
            }
            let letters = if left_sub.is_none() && left_sup.is_none() {
                match node.details.as_deref() {
                    Some(NodeDetails::Letters(run)) if run.script.is_none() => Some(run.clone()),
                    _ => None,
                }
            } else {
                None
            };
            if (sub.is_some() || sup.is_some())
                && (node.tex.starts_with(r"\operatorname*{")
                    || matches!(
                        node.tex.as_str(),
                        r"\sum"
                            | r"\prod"
                            | r"\coprod"
                            | r"\bigcap"
                            | r"\bigcup"
                            | r"\bigsqcup"
                            | r"\bigoplus"
                            | r"\bigotimes"
                            | r"\bigodot"
                            | r"\bigvee"
                            | r"\bigwedge"
                            | r"\biguplus"
                            | r"\lim"
                            | r"\max"
                            | r"\min"
                            | r"\sup"
                            | r"\inf"
                            | r"\det"
                            | r"\gcd"
                            | r"\Pr"
                    ))
            {
                node.tex.push_str(r"\limits");
            }
            let mut tex = String::new();
            let right_script_base = if left_sub.is_none() && left_sup.is_none() {
                node.right_script_base.take()
            } else {
                None
            };
            if left_sub.is_some() || left_sup.is_some() {
                tex.push_str("{}");
                if let Some(s) = left_sub {
                    tex.push_str(&format!("_{{{s}}}"));
                }
                if let Some(s) = left_sup {
                    tex.push_str(&format!("^{{{s}}}"));
                }
            }
            tex.push_str(
                &right_script_base
                    .as_ref()
                    .map_or_else(|| node.bound(), |(base, _)| base.clone()),
            );
            let suffix_start = tex.len();
            if let Some(s) = sub {
                tex.push_str(&format!("_{{{s}}}"));
            }
            if let Some(s) = sup {
                tex.push_str(&format!("^{{{s}}}"));
            }
            let suffix = tex[suffix_start..].to_owned();
            node = Node::atom(tex);
            // A following explicit native group must retain this complete
            // scripted expression as its base (e.g. {mc^2}^3).
            node.atomic = false;
            if let Some(mut run) = letters {
                run.literal = node.tex.clone();
                run.script = Some((right_script_base.map_or(0, |(_, start)| start), suffix));
                node.details = Some(Box::new(NodeDetails::Letters(run)));
            }
        }
        Ok(node)
    }
    fn rows(&mut self, end: Option<&str>) -> Result<Vec<Vec<Expr>>, PortError> {
        let mut rows = Vec::new();
        let mut cells = Vec::new();
        loop {
            let mut stops = vec!["&", "#"];
            if let Some(end) = end {
                stops.push(end);
            }
            cells.push(self.expression(&stops)?);
            if self.eat("&") {
                continue;
            }
            rows.push(std::mem::take(&mut cells));
            if self.eat("#") {
                if self.peek().is_none() || self.peek() == end {
                    return self.fail("empty trailing row");
                }
                continue;
            }
            if let Some(end) = end {
                self.require(end)?;
            }
            return Ok(rows);
        }
    }
    fn environment_tex(&self, env: &str, rows: Vec<Vec<Expr>>) -> Result<String, PortError> {
        if rows.is_empty() {
            return self.fail("empty environment");
        }
        let mut expected = None;
        let mut rendered = Vec::new();
        for row in rows {
            if matches!(env, "gathered" | "lpile" | "rpile") && row.len() != 1 {
                return self.fail("pile does not accept alignment columns");
            }
            let cells: Vec<String> = if env == "cases" && row.len() > 2 {
                // Native stops may split an expression internally. The final
                // column is its condition; preceding cells make one expression.
                let last = row.len() - 1;
                vec![
                    row[..last]
                        .iter()
                        .map(Expr::tex)
                        .filter(|s| !s.is_empty())
                        .collect::<Vec<_>>()
                        .join(" "),
                    row[last].tex(),
                ]
            } else {
                row.iter().map(Expr::tex).collect()
            };
            if env != "aligned" {
                if expected.is_some_and(|count| count != cells.len()) {
                    return self.fail("nonrectangular environment");
                }
                expected = Some(cells.len());
            }
            if cells.iter().all(|s| s.is_empty()) {
                return self.fail("empty environment row");
            }
            rendered.push(cells.join("&"));
        }
        let body = rendered.join(r"\\");
        let tex = match env {
            "lpile" => format!("\\begin{{array}}{{l}}{body}\\end{{array}}"),
            "rpile" => format!("\\begin{{array}}{{r}}{body}\\end{{array}}"),
            _ => format!("\\begin{{{env}}}{body}\\end{{{env}}}"),
        };
        if tex.len() > MAX_OUTPUT {
            return self.fail("output byte limit exceeded");
        }
        Ok(tex)
    }
    fn environment(&mut self, name: &str) -> Result<Node, PortError> {
        self.require("{")?;
        let env = match name {
            "dmatrix" => "vmatrix",
            "pile" => "gathered",
            "eqalign" => "aligned",
            other => other,
        };
        if matches!(name, "matrix" | "pmatrix" | "bmatrix" | "dmatrix")
            && !self.quoted()
            && matches!(self.peek(), Some("col" | "lcol" | "rcol"))
        {
            return self.column_environment(name);
        }
        let rows = self.rows(Some("}"))?;
        Ok(Node::atom(self.environment_tex(env, rows)?))
    }
    fn column_environment(&mut self, name: &str) -> Result<Node, PortError> {
        let mut align = String::new();
        let mut columns = Vec::new();
        while !self.quoted() && matches!(self.peek(), Some("col" | "lcol" | "rcol")) {
            align.push(match self.peek() {
                Some("lcol") => 'l',
                Some("rcol") => 'r',
                _ => 'c',
            });
            self.pos += 1;
            self.require("{")?;
            let rows = self.rows(Some("}"))?;
            if rows.iter().any(|row| row.len() != 1) {
                return self.fail("column does not accept alignment columns");
            }
            if columns
                .first()
                .is_some_and(|column: &Vec<String>| column.len() != rows.len())
            {
                return self.fail("nonrectangular column environment");
            }
            columns.push(rows.into_iter().map(|row| row[0].tex()).collect::<Vec<_>>());
        }
        self.require("}")?;
        let mut rows = Vec::new();
        for i in 0..columns[0].len() {
            let cells: Vec<_> = columns.iter().map(|column| column[i].clone()).collect();
            if cells.iter().all(String::is_empty) {
                return self.fail("empty environment row");
            }
            rows.push(cells.join("&"));
        }
        let body = rows.join(r"\\");
        let array = format!("\\begin{{array}}{{{align}}}{body}\\end{{array}}");
        let tex = match name {
            "pmatrix" => format!("\\left({array}\\right)"),
            "bmatrix" => format!("\\left[{array}\\right]"),
            "dmatrix" => format!("\\left|{array}\\right|"),
            _ => array,
        };
        if tex.len() > MAX_OUTPUT {
            return self.fail("output byte limit exceeded");
        }
        Ok(Node::atom(tex))
    }
    fn delimiter(&mut self) -> Result<&'static str, PortError> {
        if self.quoted() {
            return self.fail("unsupported quoted delimiter");
        }
        let token = self
            .peek()
            .ok_or_else(|| error(self.offset(), "missing delimiter"))?;
        let result = match token {
            "(" => "(",
            ")" => ")",
            "[" => "[",
            "]" => "]",
            "{" => r"\{",
            "}" => r"\}",
            "<" => r"\langle",
            ">" => r"\rangle",
            "|" => "|",
            "||" | "∥" | "PVER" => r"\lVert",
            "." => ".",
            "⌈" | "LCEIL" => r"\lceil",
            "⌉" | "RCEIL" => r"\rceil",
            "⌊" | "LFLOOR" => r"\lfloor",
            "⌋" | "RFLOOR" => r"\rfloor",
            _ => return self.fail("unsupported delimiter"),
        };
        self.pos += 1;
        Ok(result)
    }
    fn enlarged_delimiter(&mut self) -> Result<Node, PortError> {
        // Deliberate presentation normalization: native BIGG sizes from its
        // preceding context, while the retained TeX spelling has fixed size.
        // Raw braces are native groups rather than supported BIGG delimiters.
        if matches!(self.peek(), Some("." | "{" | "}")) && !self.quoted() {
            return self.fail("bigg requires a supported visible delimiter");
        }
        let delimiter = if self.peek() == Some("/") && !self.quoted() {
            self.pos += 1;
            "/"
        } else {
            self.delimiter()?
        };
        Ok(Node::atom(format!("\\Bigg{delimiter}")))
    }
    fn delimited(&mut self) -> Result<Node, PortError> {
        let left = self.delimiter()?;
        let previous_depth = self.delimiter_body_depth.replace(self.expression_depth + 1);
        let result = self.expression(&["RIGHT"]);
        self.delimiter_body_depth = previous_depth;
        let body = result?;
        let colored_right = self.colored_right.take();
        let mut right = if let Some(right) = colored_right {
            right
        } else {
            self.require("RIGHT")?;
            self.delimiter()?
        };
        if right == r"\lVert" {
            right = r"\rVert";
        }
        if colored_right.is_none()
            && left == "("
            && right == ")"
            && body.nodes.len() == 1
            && let Some(NodeDetails::Atop(a, b)) = body.nodes[0].details.as_deref()
        {
            return Ok(Node::atom(format!("\\binom{{{a}}}{{{b}}}")));
        }
        let separator = if left.ends_with(|c: char| c.is_ascii_alphabetic()) {
            " "
        } else {
            ""
        };
        let result = format!("\\left{left}{separator}{}\\right{right}", body.tex());
        Ok(Node::atom(if colored_right.is_some() {
            format!("{{{result}}}")
        } else {
            result
        }))
    }
    fn relation(&mut self, upper_only: bool) -> Result<Node, PortError> {
        let arrow = self
            .peek()
            .ok_or_else(|| error(self.offset(), "missing REL arrow"))?
            .to_owned();
        let glyph = match arrow.as_str() {
            "LRARROW" => r"\Longleftrightarrow",
            "lrarrow" | "<->" => r"\longleftrightarrow",
            "RARROW" => r"\Longrightarrow",
            "LARROW" => r"\Longleftarrow",
            "rarrow" => r"\xrightarrow",
            "larrow" => r"\xleftarrow",
            "hookright" => r"\xhookrightarrow",
            "hookleft" => r"\xhookleftarrow",
            "↠" => r"\xtwoheadrightarrow",
            "↞" => r"\xtwoheadleftarrow",
            "⇄" => r"\xrightleftarrows",
            "🣒" => r"\xrightequilibrium",
            "🣓" => r"\xleftequilibrium",
            "EXARROW" => r"\rightleftarrows",
            _ => return self.fail("unsupported REL arrow"),
        };
        self.pos += 1;
        let above = self.braced_argument()?.argument().to_owned();
        let below = if upper_only {
            String::new()
        } else {
            self.braced_argument()?.argument().to_owned()
        };
        Ok(Node::atom(
            if matches!(
                arrow.as_str(),
                "rarrow" | "larrow" | "hookright" | "hookleft" | "↠" | "↞" | "⇄" | "🣒" | "🣓"
            ) {
                // Optional TeX arguments stop at ]. A native label can contain
                // square brackets, so bind that label inside a TeX group.
                if below.contains(']') {
                    format!("{glyph}[{{{below}}}]{{{above}}}")
                } else {
                    format!("{glyph}[{below}]{{{above}}}")
                }
            } else if upper_only {
                format!("\\overset{{{above}}}{{{glyph}}}")
            } else {
                format!("\\overset{{{above}}}{{\\underset{{{below}}}{{{glyph}}}}}")
            },
        ))
    }
    fn color(&mut self) -> Result<Node, PortError> {
        self.require("{")?;
        let mut rgb = Vec::new();
        for i in 0..3 {
            let value = self
                .peek()
                .and_then(|s| s.parse::<u8>().ok())
                .ok_or_else(|| error(self.offset(), "COLOR requires three RGB bytes"))?;
            self.pos += 1;
            rgb.push(value.to_string());
            if i < 2 {
                self.require(",")?;
            }
        }
        self.require("}")?;
        if self
            .delimiter_body_depth
            .is_some_and(|depth| self.expression_depth >= depth)
            && self.color_contains_direct_right()
        {
            self.require("{")?;
            let body = self.expression(&["RIGHT"])?;
            self.require("RIGHT")?;
            let right = self.delimiter()?;
            self.require("}")?;
            self.colored_right = Some(right);
            return Ok(Node::atom(format!(
                "\\color[RGB]{{{}}}{}",
                rgb.join(","),
                body.tex()
            )));
        }
        let body = self.braced_argument()?.argument().to_owned();
        Ok(Node::atom(format!(
            "\\textcolor[RGB]{{{}}}{{{body}}}",
            rgb.join(",")
        )))
    }
    fn color_contains_direct_right(&self) -> bool {
        if self.peek() != Some("{") || self.quoted() {
            return false;
        }
        let mut braces = 0usize;
        let mut delimiters = 0usize;
        for token in &self.tokens[self.pos..] {
            if token.kind == Kind::Quoted {
                continue;
            }
            match token.text.as_str() {
                "{" => braces += 1,
                "}" => {
                    braces -= 1;
                    if braces == 0 {
                        return false;
                    }
                }
                "LEFT" if braces == 1 => delimiters += 1,
                "RIGHT" if braces == 1 && delimiters > 0 => delimiters -= 1,
                "RIGHT" if braces == 1 => return true,
                _ => {}
            }
        }
        false
    }
    fn compact_function_word(&self, word: &str) -> Result<Option<Node>, PortError> {
        if !word.is_ascii() || function_prefix(word).is_none() {
            return Ok(None);
        }
        let mut tail = word;
        let mut parts = Vec::new();
        let mut size = 0;
        while let Some(name) = function_prefix(tail) {
            let part = function_tex(name).expect("recognized function");
            size += part.len() + 1;
            if size > MAX_OUTPUT {
                return self.fail("output byte limit exceeded");
            }
            parts.push(part);
            tail = &tail[name.len()..];
        }
        if !tail.is_empty() {
            // The first non-function suffix remains one ordinary native word.
            // Do not reinterpret its interior, commands or font declarations.
            let part = self.ordinary(tail.into()).tex;
            size += part.len();
            if size > MAX_OUTPUT {
                return self.fail("output byte limit exceeded");
            }
            parts.push(part);
        }
        // OVER consumes the complete compact word; a right script belongs to
        // its last suffix letter, or its last named function when no tail exists.
        let mut node = Node::ordinary(parts.join(" "));
        if !tail.is_empty() {
            parts.pop();
            parts.push(Self::letter_script_base(tail, self.effective_font()));
        }
        node.right_script_base = Some((parts.join(" "), word.len() - 1));
        Ok(Some(node))
    }
    fn function_runs(&self, nodes: Vec<Node>) -> Result<Vec<Node>, PortError> {
        let mut out = Vec::new();
        let mut nodes = nodes.into_iter().peekable();
        while let Some(first) = nodes.next() {
            let Some(NodeDetails::Letters(first_run)) = first.details.as_deref() else {
                out.push(first);
                continue;
            };
            let mut run = first_run.clone();
            let mut originals = vec![run.literal.clone()];
            while run.script.is_none() {
                let Some(NodeDetails::Letters(next)) =
                    nodes.peek().and_then(|node| node.details.as_deref())
                else {
                    break;
                };
                if next.font != run.font || next.font_generation != run.font_generation {
                    break;
                }
                if let Some((start, suffix)) = &next.script {
                    run.script = Some((run.text.len() + start, suffix.clone()));
                }
                run.text.push_str(&next.text);
                originals.push(next.literal.clone());
                nodes.next();
            }
            let mut tail = run.text.as_str();
            let mut consumed = 0;
            let mut before = Vec::new();
            let mut scripted = Vec::new();
            while let Some(name) = function_prefix(tail) {
                consumed += name.len();
                let part = function_tex(name).expect("recognized function");
                if run
                    .script
                    .as_ref()
                    .is_some_and(|(start, _)| consumed > *start)
                {
                    scripted.push(part);
                } else {
                    before.push(part);
                }
                tail = &tail[name.len()..];
            }
            run.literal = originals.join(" ");
            let mut tex = if consumed == 0 {
                run.literal.clone()
            } else {
                if let Some((start, suffix)) = &run.script {
                    // Names may cross the final operand boundary. Keep a
                    // complete named function together, while a separate x^2
                    // following the name retains its own script binding.
                    if consumed < *start {
                        before.push(
                            Self::ordinary_with_font(run.text[consumed..*start].into(), run.font)
                                .tex,
                        );
                    }
                    let tail_start = consumed.max(*start);
                    if tail_start < run.text.len() {
                        scripted.push(
                            Self::ordinary_with_font(run.text[tail_start..].into(), run.font).tex,
                        );
                    }
                    let base = scripted.join(" ");
                    let base = if scripted.len() == 1 {
                        base
                    } else {
                        format!("{{{base}}}")
                    };
                    before.push(format!("{base}{suffix}"));
                } else if !tail.is_empty() {
                    before.push(Self::ordinary_with_font(tail.into(), run.font).tex);
                }
                before.join(" ")
            };
            if tex.len() > MAX_OUTPUT {
                return self.fail("output byte limit exceeded");
            }
            // Retain raw letters through transparent groups: an outer leading
            // identifier can prevent recognition of a split function name.
            let mut node = if originals.len() == 1 && consumed == 0 && first.tex == tex {
                first
            } else {
                Node::ordinary(std::mem::take(&mut tex))
            };
            node.details = Some(Box::new(NodeDetails::Letters(run)));
            out.push(node);
        }
        Ok(out)
    }
    // Ordinary brackets are visible characters and need not have partners.
    // Group properly nested pairs to retain parenthesized ATOP conversion.
    fn has_ordinary_delimiter_end(&self, opening: &str) -> bool {
        let mut stack = vec![opening];
        let mut braces = 0usize;
        let mut native_pairs = 0usize;
        let mut native_delimiter = false;
        for token in &self.tokens[self.pos..] {
            if token.kind == Kind::Quoted {
                continue;
            }
            if native_delimiter {
                native_delimiter = false;
                continue;
            }
            match token.text.as_str() {
                "{" => braces += 1,
                "}" if braces > 0 => braces -= 1,
                "LEFT" if braces == 0 => {
                    native_pairs += 1;
                    native_delimiter = true;
                }
                "RIGHT" if braces == 0 && native_pairs > 0 => {
                    native_pairs -= 1;
                    native_delimiter = true;
                }
                "}" | "RIGHT" | "&" | "#" if braces == 0 && native_pairs == 0 => return false,
                _ if braces > 0 => {}
                _ if native_pairs > 0 => {}
                "(" | "[" => {
                    stack.push(token.text.as_str());
                    if stack.len() > MAX_DEPTH {
                        return false;
                    }
                }
                ")" | "]" => {
                    let expected = if token.text == ")" { "(" } else { "[" };
                    if stack.pop() != Some(expected) {
                        return false;
                    }
                    if stack.is_empty() {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    fn primary(&mut self) -> Result<Node, PortError> {
        let token = self
            .tokens
            .get(self.pos)
            .ok_or_else(|| error(self.offset(), "missing argument"))?;
        let text = token.text.clone();
        let kind = token.kind;
        self.pos += 1;
        if kind == Kind::Quoted {
            let mut glyphs = text.chars();
            if let (Some(glyph), None) = (glyphs.next(), glyphs.next())
                && let Some((command, base)) = crate::math_alphabet::tex_glyph(glyph)
            {
                return Ok(Node::atom(format!("\\{command}{{{base}}}")));
            }
            if text == "↤" {
                return Ok(Node::atom(r"\mapsfrom"));
            }
            if matches!(text.as_str(), "↠" | "↞") {
                return Ok(Node::atom(if text == "↠" {
                    r"\twoheadrightarrow"
                } else {
                    r"\twoheadleftarrow"
                }));
            }
            if text.chars().any(|ch| monospace_ascii(ch).is_some()) {
                let mut output = String::new();
                let mut run = String::new();
                let mut fixed = None;
                for ch in text.chars() {
                    let mapped = monospace_ascii(ch);
                    // Whitespace and punctuation can share the fixed-width run;
                    // ordinary letters and other Unicode keep their own font.
                    let is_fixed =
                        mapped.is_some() || ch.is_ascii_whitespace() || ch.is_ascii_punctuation();
                    if fixed.is_some_and(|previous| previous != is_fixed) {
                        let command = if fixed == Some(true) {
                            "mathtt"
                        } else {
                            self.effective_font().command()
                        };
                        output.push_str(&format!("\\{command}{{{}}}", escape_math_text(&run)));
                        run.clear();
                    }
                    fixed = Some(is_fixed);
                    run.push(mapped.unwrap_or(ch));
                }
                let command = if fixed == Some(true) {
                    "mathtt"
                } else {
                    self.effective_font().command()
                };
                output.push_str(&format!("\\{command}{{{}}}", escape_math_text(&run)));
                return Ok(Node::atom(output));
            }
            // A single quoted mathematical script letter is an intrinsic
            // alphabet glyph, not an ordinary quoted text run. Keep this
            // distinct from native keywords such as LAPLACE and from mixed
            // quoted text, whose literal semantics must remain intact.
            const SCRIPT_LETTERS: &str = "𝒜ℬ𝒞𝒟ℰℱ𝒢ℋℐ𝒥𝒦ℒℳ𝒩𝒪𝒫𝒬ℛ𝒮𝒯𝒰𝒱𝒲𝒳𝒴𝒵𝒶𝒷𝒸𝒹ℯ𝒻ℊ𝒽𝒾𝒿𝓀𝓁𝓂𝓃ℴ𝓅𝓆𝓇𝓈𝓉𝓊𝓋𝓌𝓍𝓎𝓏";
            let mut chars = text.chars();
            if let (Some(ch), None) = (chars.next(), chars.next())
                && let Some(index) = SCRIPT_LETTERS.chars().position(|value| value == ch)
            {
                let letter = char::from(if index < 26 {
                    b'A' + index as u8
                } else {
                    b'a' + (index - 26) as u8
                });
                return Ok(Node::atom(format!("\\mathscr{{{letter}}}")));
            }
            // A math alphabet has no visible effect on whitespace alone.
            // Keep explicit quoted spaces without a redundant italic wrapper.
            if self.effective_font() == FontState::Italic
                && !text.is_empty()
                && text.chars().all(char::is_whitespace)
            {
                return Ok(Node::ordinary(escape_math_text(&text)));
            }
            // Native quotes suppress keyword recognition; TeX's ordinary
            // alphabet already has the default italic shape. Keep word-wide
            // script binding without introducing a redundant math alphabet.
            if self.effective_font() == FontState::Italic
                && !text.is_empty()
                && text.bytes().all(|c| c.is_ascii_alphabetic())
            {
                return Ok(Node::ordinary(text));
            }
            let command = self.effective_font().command();
            let mut node = Node::atom(format!("\\{command}{{{}}}", escape_math_text(&text)));
            if self.effective_font() == FontState::Roman {
                node.details = Some(Box::new(NodeDetails::UprightName(text)));
            }
            return Ok(node);
        }
        if kind == Kind::Number {
            return Ok(self.ordinary(text));
        }
        match text.as_str() {
            "{" => self.group(),
            "(" | "[" => {
                let end = if text == "(" { ")" } else { "]" };
                if !self.has_ordinary_delimiter_end(&text) {
                    return Ok(Node::atom(text));
                }
                let body = self.expression(&[end])?;
                self.require(end)?;
                if text == "("
                    && body.nodes.len() == 1
                    && let Some(NodeDetails::Atop(a, b)) = body.nodes[0].details.as_deref()
                {
                    return Ok(Node::atom(format!("\\binom{{{a}}}{{{b}}}")));
                }
                Ok(Node::atom(format!("{text}{}{end}", body.tex())))
            }
            "LEFT" => self.delimited(),
            "bigg" | "BIGG" => self.enlarged_delimiter(),
            "matrix" | "pmatrix" | "bmatrix" | "dmatrix" | "cases" | "pile" | "lpile" | "rpile"
            | "eqalign" => self.environment(&text),
            "binom" => {
                let a = self.constructor_argument()?.argument().to_owned();
                let b = self.constructor_argument()?.argument().to_owned();
                if a.is_empty() || b.is_empty() {
                    return self.fail("empty binomial operand");
                }
                Ok(Node::atom(format!("\\binom{{{a}}}{{{b}}}")))
            }
            "REL" | "rel" => self.relation(false),
            "BUILDREL" | "buildrel" => self.relation(true),
            "not" | "NOT" => self.negation(),
            "COLOR" | "color" => self.color(),
            "sqrt" => {
                let body = self.constructor_argument()?.argument().to_owned();
                Ok(Node::atom(format!("\\sqrt{{{body}}}")))
            }
            "root" => {
                let index = self.constructor_argument()?.argument().to_owned();
                self.require("of")?;
                let body = self.constructor_argument()?.argument().to_owned();
                if index.contains(']') {
                    Ok(Node::atom(format!("\\sqrt[{{{index}}}]{{{body}}}")))
                } else {
                    Ok(Node::atom(format!("\\sqrt[{index}]{{{body}}}")))
                }
            }
            "vec" | "dyad" | "acute" | "grave" | "dot" | "ddot" | "under" | "bar" | "hat"
            | "check" | "arch" | "tilde" | "BOX" | "box" => {
                let command = match text.as_str() {
                    "vec" => "vec",
                    "dyad" => "overleftrightarrow",
                    "under" => "underline",
                    "arch" => "overset{\\frown}",
                    "BOX" | "box" => "boxed",
                    other => other,
                };
                let body = self.constructor_argument()?;
                let body = if text == "vec" {
                    self.postfix(body)?
                } else {
                    body
                };
                let body = body.argument().to_owned();
                Ok(Node::atom(format!("\\{command}{{{body}}}")))
            }
            "OVERBRACE" | "UNDERBRACE" => {
                let first = self.constructor_argument()?.argument().to_owned();
                let second = self.constructor_argument()?.argument().to_owned();
                Ok(Node::atom(if text == "UNDERBRACE" {
                    format!("\\underbrace{{{second}}}_{{{first}}}")
                } else {
                    format!("\\overbrace{{{first}}}^{{{second}}}")
                }))
            }
            "UNDEROVER" | "underover" => {
                let argument = self.argument()?;
                if let Some(NodeDetails::UprightName(name)) = argument.details.as_deref() {
                    return Ok(Node::atom(format!(
                        "\\operatorname*{{{}}}",
                        escape_text(name)
                    )));
                }
                let body = argument.argument().to_owned();
                Ok(Node::atom(match body.as_str() {
                    r"\lor" => r"\bigvee".into(),
                    r"\wedge" => r"\bigwedge".into(),
                    _ => format!("\\mathop{{{body}}}\\limits"),
                }))
            }
            "BIGSQCAP" => Ok(Node::atom(r"\mathop{\sqcap}\limits")),
            "BIGOMINUS" => Ok(Node::atom(r"\mathop{\ominus}\limits")),
            "BIGODIV" => Ok(Node::atom(r"\mathop{\oslash}\limits")),
            "odint" => Ok(Node::atom(r"\oiint")),
            "otint" => Ok(Node::atom(r"\oiiint")),
            "rarrow" | "->" => Ok(Node::atom(r"\to")),
            "lrarrow" => Ok(Node::atom(r"\leftrightarrow")),
            "↠" => Ok(Node::atom(r"\twoheadrightarrow")),
            "↞" => Ok(Node::atom(r"\twoheadleftarrow")),
            "EXARROW" => Ok(Node::atom(r"\rightleftarrows")),
            "WEDGE" => Ok(Node::atom(r"\wedge")),
            "SMALLUNION" => Ok(Node::atom(r"\cup")),
            "inf" => Ok(Node::atom(r"\infty")),
            "DIVIDE" => Ok(Node::atom(r"\div")),
            "cdot" | "CDOT" => Ok(Node::atom(r"\cdot")),
            "+-" => Ok(Node::atom(r"\pm")),
            "<-" => Ok(Node::atom(r"\leftarrow")),
            "<=" => Ok(Node::atom(r"\leq")),
            ">=" => Ok(Node::atom(r"\geq")),
            "||" => Ok(Node::atom(r"\Vert")),
            "LCEIL" => Ok(Node::atom(r"\lceil")),
            "RCEIL" => Ok(Node::atom(r"\rceil")),
            "LFLOOR" => Ok(Node::atom(r"\lfloor")),
            "RFLOOR" => Ok(Node::atom(r"\rfloor")),
            "`" => Ok(Node::atom(r"\,")),
            "~" => Ok(Node::atom(r"\ ")),
            "~~" => Ok(Node::atom(r"\quad")),
            ")" | "]" => Ok(Node::atom(text)),
            "}" | "RIGHT" | "&" | "#" | "_" | "^" | "sub" | "sup" | "LSUP" | "LSUB" | "lsup"
            | "lsub" | "from" | "to" | "of" | "over" | "atop" | "choose" | "col" | "lcol"
            | "rcol" => self.fail(format!("unexpected structural token {text}")),
            "\\" => self.fail("TeX commands are not native SCRIPT input"),
            _ => {
                if let Some(value) = function_tex(&text) {
                    return Ok(Node::atom(value));
                }
                if let Some(value) = scalar_symbol(&text) {
                    let mut node = Node::atom(value);
                    // These native glyphs expand into a TeX superscript or a
                    // degree/unit pair, or two colon atoms. A native postfix
                    // owns the whole glyph.
                    node.atomic = !matches!(
                        text.as_str(),
                        "DEG" | "CENTIGRADE" | "FAHRENHEIT" | "IDENTICAL"
                    );
                    return Ok(node);
                }
                if kind == Kind::Word {
                    if UNSUPPORTED_WORDS
                        .iter()
                        .any(|word| word.eq_ignore_ascii_case(&text))
                        || unsupported_spelling(&text)
                        || (self.constructor_operand
                            && ADDITIONAL_NATIVE_WORDS
                                .iter()
                                .any(|word| word.eq_ignore_ascii_case(&text)))
                    {
                        return self.fail(format!("unsupported SCRIPT command or spelling {text}"));
                    }
                    if let Some(node) = self.compact_function_word(&text)? {
                        return Ok(node);
                    }
                    if text.starts_with("in") {
                        return self.fail(format!("unsupported SCRIPT command prefix in ({text})"));
                    }
                    // Capitalized ordinary words (e.g. Equation) still mean
                    // italic letters. Reserve unknown all-caps/constructor
                    // shapes and known command spellings rather than guessing.
                    if self.effective_font() == FontState::Italic
                        && text.is_ascii()
                        && text.len() > 1
                        && ((!self.constructor_operand && text.chars().all(char::is_uppercase))
                            || (self.peek() == Some("{")
                                && text.len() > 2
                                && !(self.delimiter_body_depth.is_some()
                                    && self.tokens.get(self.pos + 1).is_some_and(|token| {
                                        token.kind != Kind::Quoted
                                            && matches!(token.text.as_str(), "color" | "COLOR")
                                    }))))
                    {
                        return self.fail(format!("unsupported SCRIPT constructor {text}"));
                    }
                    let mut node = self.ordinary(text.clone());
                    if text.is_ascii() {
                        node.right_script_base = Some((
                            Self::letter_script_base(&text, self.effective_font()),
                            text.len() - 1,
                        ));
                        node.details = Some(Box::new(NodeDetails::Letters(LetterRun {
                            text,
                            literal: node.tex.clone(),
                            font: self.effective_font(),
                            font_generation: self.font_generation,
                            script: None,
                        })));
                    }
                    return Ok(node);
                }
                if text
                    .chars()
                    .all(|c| c.is_alphabetic() || "+-=<>.,!?/:;|'".contains(c))
                {
                    return Ok(self.ordinary(text));
                }
                self.fail(format!("unsupported SCRIPT token {text}"))
            }
        }
    }

    fn negation(&mut self) -> Result<Node, PortError> {
        self.enter()?;
        let result = self.negation_operand();
        self.depth -= 1;
        let (operand, ordinary) = result?;
        let separator = if operand.starts_with(char::is_alphabetic) {
            " "
        } else {
            ""
        };
        let tex = format!("\\not{separator}{operand}");
        let font_applies = ordinary
            && ((self.effective_font() == FontState::Roman
                && operand.chars().any(char::is_alphabetic))
                || (self.effective_font().is_bold() && operand.chars().any(char::is_alphanumeric)));
        Ok(Node::atom(if font_applies {
            format!("\\{}{{{tex}}}", self.effective_font().command())
        } else {
            tex
        }))
    }

    fn negation_operand(&mut self) -> Result<(String, bool), PortError> {
        let Some(token) = self.tokens.get(self.pos) else {
            return self.fail("not requires one character or supported scalar symbol");
        };
        let mut text = token.text.clone();
        if text.len() > 1
            && ((token.kind == Kind::Word
                && text.bytes().all(|c| c.is_ascii_alphabetic())
                && !literal_needs_quotes(&text))
                || token.kind == Kind::Number)
        {
            // Native NOT consumes one visible character, even when the lexer
            // collected an ordinary word. Keep the remaining word and its
            // original source offset for subsequent scripts and diagnostics.
            self.tokens[self.pos].text = text[1..].into();
            self.tokens[self.pos].offset += 1;
            text.truncate(1);
            return Ok((text, true));
        }
        if token.kind == Kind::Quoted || !negatable_scalar(&text) {
            return self.fail("not requires one character or supported scalar symbol");
        }
        if matches!(text.as_str(), "IN" | "LEQ") {
            self.pos += 1;
            return Ok((if text == "IN" { r"\in" } else { r"\leq" }.into(), false));
        }
        let integral = match text.as_str() {
            "∫" => Some(r"\int"),
            "∬" => Some(r"\iint"),
            "∭" => Some(r"\iiint"),
            "∮" => Some(r"\oint"),
            "∯" => Some(r"\oiint"),
            "∰" => Some(r"\oiiint"),
            _ => None,
        };
        if let Some(command) = integral {
            self.pos += 1;
            return Ok((command.into(), false));
        }
        if text.chars().count() == 1 && text.chars().all(char::is_alphanumeric) {
            self.pos += 1;
            return Ok((text, true));
        }
        let operand = self.primary()?;
        let composite = matches!(
            text.as_str(),
            "DEG"
                | "CENTIGRADE"
                | "FAHRENHEIT"
                | "LAPLACE"
                | "ANGSTROM"
                | "att"
                | "hund"
                | "thou"
                | "IDENTICAL"
        );
        Ok((
            if composite {
                format!("{{{}}}", operand.tex)
            } else {
                operand.tex
            },
            false,
        ))
    }
}

/// Unicode character used for a native integral keyword under NOT.
pub(crate) fn negated_integral_glyph(native: &str) -> Option<&'static str> {
    Some(match command_alias(native).unwrap_or(native) {
        "int" => "∫",
        "dint" => "∬",
        "tint" => "∭",
        "oint" => "∮",
        "odint" => "∯",
        "otint" => "∰",
        _ => return None,
    })
}

// NOT is a slash over one native character/symbol, not a cancellation box.
// Constructors and function names must not be accepted merely because their
// result is a Node. Ordinary words are split before consulting this predicate.
pub(crate) fn negatable_scalar(text: &str) -> bool {
    let text = command_alias(text).unwrap_or(text);
    // Hancom draws no NOT slash over integral keywords (`not oint` shows only a
    // stray dot), but it does over the Unicode integral characters.
    if matches!(text, "∫" | "∬" | "∭" | "∮" | "∯" | "∰") {
        return true;
    }
    (text.chars().count() == 1
        && text.chars().all(|c| {
            c.is_ascii_alphanumeric() || is_hangul_letter(c) || "+-=<>.,!?/:;|'".contains(c)
        }))
        || (scalar_symbol(text).is_some_and(|value| {
            !matches!(
                value.as_str(),
                r"\sum"
                    | r"\prod"
                    | r"\coprod"
                    | r"\int"
                    | r"\oint"
                    | r"\lim"
                    | r"\bigcap"
                    | r"\bigcup"
                    | r"\iint"
                    | r"\iiint"
                    | r"\oiint"
                    | r"\oiiint"
            )
        }) && !matches!(
            text,
            "sum"
                | "prod"
                | "coprod"
                | "int"
                | "oint"
                | "lim"
                | "inter"
                | "union"
                | "dint"
                | "tint"
        ))
        || matches!(
            text,
            "IN" | "LEQ"
                | "<="
                | ">="
                | "+-"
                | "<-"
                | "->"
                | "<->"
                | "cdot"
                | "CDOT"
                | "WEDGE"
                | "SMALLUNION"
                | "DIVIDE"
                | "inf"
                | "rarrow"
                | "lrarrow"
                | "EXARROW"
        )
}

// Independently authored scalar vocabulary: native names are specified in
// Hancom Equation revision 1.2 pp10/12–14, and the TeX names agree with this
// project's original tex_to_hml::symbol match. The large-set operators and
// multiple integrals additionally use the explicit approved toolbar pairs.
// This is deliberately limited vocabulary, not an external conversion table.
fn scalar_symbol(name: &str) -> Option<String> {
    if name.len() == 4 && name.starts_with("Vec") && name.as_bytes()[3].is_ascii_uppercase() {
        let letter = name.as_bytes()[3] as char;
        return Some(format!(r"\mathbb{{{letter}}}"));
    }
    // Use Latin math letters for the matching uppercase Greek shapes so this
    // output works in standard LaTeX math mode without Unicode Greek fonts.
    // Native uppercase Greek remains slanted even inside an rm run.
    let latin_shape = match name {
        "Alpha" => "A",
        "Beta" => "B",
        "Epsilon" => "E",
        "Zeta" => "Z",
        "Eta" => "H",
        "Iota" => "I",
        "Kappa" => "K",
        "Mu" => "M",
        "Nu" => "N",
        "Omicron" => "O",
        "Rho" => "P",
        "Tau" => "T",
        "Chi" => "X",
        "Upsilon" => "Y",
        _ => "",
    };
    if !latin_shape.is_empty() {
        return Some(latin_shape.into());
    }
    let uppercase_greek = match name {
        "Gamma" => r"\Gamma",
        "Delta" => r"\Delta",
        "Theta" => r"\Theta",
        "Lambda" => r"\Lambda",
        "Xi" => r"\Xi",
        "Pi" => r"\Pi",
        "Sigma" => r"\Sigma",
        "Phi" => r"\Phi",
        "Psi" => r"\Psi",
        "Omega" => r"\Omega",
        _ => "",
    };
    if !uppercase_greek.is_empty() {
        return Some(format!("\\mathit{{{uppercase_greek}}}"));
    }
    let same_name = matches!(
        name,
        "alpha"
            | "beta"
            | "gamma"
            | "delta"
            | "epsilon"
            | "zeta"
            | "eta"
            | "theta"
            | "iota"
            | "kappa"
            | "lambda"
            | "mu"
            | "nu"
            | "xi"
            | "pi"
            | "rho"
            | "sigma"
            | "tau"
            | "upsilon"
            | "phi"
            | "chi"
            | "psi"
            | "omega"
            | "vartheta"
            | "varpi"
            | "varsigma"
            | "varphi"
            | "varepsilon"
            | "sum"
            | "prod"
            | "coprod"
            | "int"
            | "oint"
            | "lim"
    );
    if same_name {
        return Some(format!("\\{name}"));
    }
    Some(
        match name {
            "omicron" => "o",
            "BIGVEE" => r"\bigvee",
            "BIGWEDGE" => r"\bigwedge",
            "DASHV" | "MASSERT" => r"\dashv",
            // These native glyph variants share a TeX semantic counterpart;
            // their original font metrics are not retained by normalization.
            "HCROSS" => "+",
            "HDOWN" => r"\top",
            "HRIGHT" => r"\vdash",
            "HUP" => r"\bot",
            "MHO" => r"\mho",
            "NIN" => r"\notin",
            "NOWNS" => r"\not\ni",
            "NSUBSET" => r"\not\subset",
            "NSUBSETEQ" => r"\not\subseteq",
            "NSUPSET" => r"\not\supset",
            "NSUPSETEQ" => r"\not\supseteq",
            "PHOR" => "=",
            "PVER" => r"\Vert",
            "ROUND" => r"\partial",
            // Unicode retains the ascending direction without a nonstandard
            // iddots extension. SMALL* affects this operator alone.
            "SDOTS" => "⋰",
            "SMALLPROD" => r"{\textstyle\prod}",
            "SMALLSUM" => r"{\textstyle\sum}",
            // User-authorized nominal symbol: Hancom 2020 renders this name
            // literally, but its TeX mathematical counterpart is unambiguous.
            "SMALLCOPROD" => r"{\textstyle\coprod}",
            "TRIANGLEL" => r"\triangleleft",
            "TRIANGLER" => r"\triangleright",
            "varupsilon" => r"\Upsilon",
            "image" => r"\fallingdotseq",
            "REIMAGE" => r"\risingdotseq",
            "ISO" => r"\Bumpeq",
            "DSUM" => r"\dotplus",
            "XOR" => r"\veebar",
            "DEG" => r"{}^{\circ}",
            "HLEFT" => r"\dashv",
            "LAPLACE" => r"\mathcal{L}",
            "CENTIGRADE" => r"{}^{\circ}\mathrm{C}",
            "FAHRENHEIT" => r"{}^{\circ}\mathrm{F}",
            "LSLANT" => r"\diagup",
            "RSLANT" => r"\diagdown",
            "att" => r"\text{\textreferencemark}",
            "hund" => r"\text{\textperthousand}",
            "thou" => r"\text{\textpertenthousand}",
            "well" => "⌗",
            "base" => "⌂",
            "benzene" => "⌬",
            "□" => r"\square",
            "ALEPH" => r"\aleph",
            "HBAR" => r"\hbar",
            "IMATH" => r"\imath",
            "JMATH" => r"\jmath",
            "OHM" => r"\Omega",
            "ELL" | "LITER" => r"\ell",
            "WP" => r"\wp",
            "IMAG" => r"\Im",
            "ANGSTROM" => r"\text{\AA}",
            "SQCUP" => r"\sqcup",
            "OPLUS" => r"\oplus",
            "OTIMES" => r"\otimes",
            "ODOT" => r"\odot",
            "OWNS" => r"\ni",
            "SQSUBSET" => r"\sqsubset",
            "SQSUPSET" => r"\sqsupset",
            "SQSUBSETEQ" => r"\sqsubseteq",
            "SQSUPSETEQ" => r"\sqsupseteq",
            "PREC" => r"\prec",
            "SUCC" => r"\succ",
            "UPLUS" => r"\uplus",
            "CIRC" => r"\circ",
            "AST" => r"\ast",
            "STAR" => r"\bigstar",
            "BIGCIRC" => r"\bigcirc",
            "THEREFORE" => r"\therefore",
            "BECAUSE" => r"\because",
            "IDENTICAL" => "::",
            "DOTEQ" => r"\doteq",
            "SIMEQ" => r"\simeq",
            "CONG" => r"\cong",
            "ASYMP" => r"\asymp",
            "DIAMOND" => r"\diamond",
            "prime" => r"\prime",
            "LNOT" => r"\lnot",
            "PROPTO" => r"\propto",
            "DAGGER" => r"\dagger",
            "DDAGGER" => r"\ddagger",
            "uparrow" => r"\uparrow",
            "downarrow" => r"\downarrow",
            "UPARROW" => r"\Uparrow",
            "DOWNARROW" => r"\Downarrow",
            "udarrow" => r"\updownarrow",
            "UDARROW" => r"\Updownarrow",
            "nwarrow" => r"\nwarrow",
            "searrow" => r"\searrow",
            "nearrow" => r"\nearrow",
            "swarrow" => r"\swarrow",
            "hookleft" => r"\hookleftarrow",
            "hookright" => r"\hookrightarrow",
            "mapsto" => r"\mapsto",
            "↤" => r"\mapsfrom",
            "TRIANGLE" => r"\triangle",
            "TRIANGLED" => r"\triangledown",
            "ANGLE" => r"\angle",
            "MSANGLE" => r"\measuredangle",
            "SANGLE" => r"\sphericalangle",
            "RTANGLE" => "∟",
            "VDASH" => r"\vdash",
            "BOT" => r"\bot",
            "TOP" => r"\top",
            "MODELS" => r"\models",
            "TIMES" | "times" => r"\times",
            "PLUSMINUS" => r"\pm",
            "MINUSPLUS" => r"\mp",
            "DIV" | "div" => r"\div",
            "CAP" | "cap" => r"\cap",
            "OSLASH" | "oslash" => r"\oslash",
            "larrow" => r"\leftarrow",
            "rarrow" => r"\to",
            "lrarrow" => r"\leftrightarrow",
            "inf" => r"\infty",
            "cdots" => r"\cdots",
            "VEE" => r"\lor",
            "LLL" => r"\lll",
            "vert" => r"\vert",
            "VERT" => r"\Vert",
            "LCEIL" => r"\lceil",
            "RCEIL" => r"\rceil",
            "LFLOOR" => r"\lfloor",
            "RFLOOR" => r"\rfloor",
            "WEDGE" => r"\wedge",
            "SMALLUNION" => r"\cup",
            "DIVIDE" => r"\div",
            "CDOT" => r"\cdot",
            "EXARROW" => r"\rightleftarrows",
            "odint" => r"\oiint",
            "otint" => r"\oiiint",
            "->" => r"\to",
            "<-" => r"\leftarrow",
            "+-" => r"\pm",
            "||" => r"\Vert",
            "|" => r"\vert",
            "LDOTS" | "ldots" => r"\ldots",
            "VDOTS" | "vdots" => r"\vdots",
            "DDOTS" | "ddots" => r"\ddots",
            "LEQ" | "leq" => r"\leq",
            "NEQ" | "neq" => r"\neq",
            "APPROX" | "approx" => r"\approx",
            "EQUIV" | "equiv" => r"\equiv",
            "SIM" | "sim" => r"\sim",
            "IN" | "in" => r"\in",
            "SUBSET" | "subset" => r"\subset",
            "SUPSET" | "supset" => r"\supset",
            "SUBSETEQ" | "subseteq" => r"\subseteq",
            "SUPSETEQ" | "supseteq" => r"\supseteq",
            "EMPTYSET" | "emptyset" => r"\emptyset",
            "PARTIAL" | "partial" => r"\partial",
            "FORALL" | "forall" => r"\forall",
            "±" => r"\pm",
            "-+" => r"\mp",
            "÷" => r"\div",
            "BULLET" => r"\bullet",
            "CDOTS" => r"\cdots",
            "SQCAP" | "sqcap" => r"\sqcap",
            "OMINUS" => r"\ominus",
            "ODIV" => r"\oslash",
            "EXIST" | "EXISTS" => r"\exists",
            "nabla" | "NABLA" => r"\nabla",
            "notin" | "NOTIN" => r"\notin",
            "RARROW" => r"\Rightarrow",
            "LARROW" => r"\Leftarrow",
            "LRARROW" => r"\Leftrightarrow",
            "GEQ" => r"\geq",
            "INF" => r"\infty",
            "SMALLINTER" => r"\cap",
            "inter" => r"\bigcap",
            "union" => r"\bigcup",
            "dint" => r"\iint",
            "tint" => r"\iiint",
            "LOR" => r"\lor",
            "BIGSQCUP" => r"\bigsqcup",
            "BIGOPLUS" => r"\bigoplus",
            "BIGOTIMES" => r"\bigotimes",
            "BIGODOT" => r"\bigodot",
            "BIGUPLUS" => r"\biguplus",
            "<->" => r"\leftrightarrow",
            "!=" => r"\neq",
            "==" => r"\equiv",
            "<<" => r"\ll",
            ">>" => r"\gg",
            "<<<" => r"\lll",
            ">>>" => r"\ggg",
            _ => return None,
        }
        .into(),
    )
}

fn unsupported_spelling(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    scalar_symbol(&upper).is_some()
        // A wrong-case Greek/operator name must not become ordinary letters
        // merely because mixed-case identifiers are otherwise accepted.
        || scalar_symbol(&name.to_ascii_lowercase()).is_some()
        || matches!(
            upper.as_str(),
            "SCALE"
                | "RM"
                | "IT"
                | "BOLD"
                | "SQRT"
                | "ROOT"
                | "OF"
                | "MATRIX"
                | "PMATRIX"
                | "BMATRIX"
                | "DMATRIX"
                | "CASES"
                | "PILE"
                | "LPILE"
                | "RPILE"
                | "COL"
                | "LCOL"
                | "RCOL"
                | "CHOOSE"
                | "BINOM"
                | "EQALIGN"
                | "VEC"
                | "DYAD"
                | "ACUTE"
                | "GRAVE"
                | "DOT"
                | "DDOT"
                | "UNDER"
                | "BAR"
                | "HAT"
                | "CHECK"
                | "ARCH"
                | "TILDE"
                | "SIN"
                | "COS"
                | "TAN"
                | "COT"
                | "SEC"
                | "CSC"
                | "LOG"
                | "LN"
                | "EXP"
                | "SINH"
                | "COSH"
                | "TANH"
                | "LSUP"
                | "LSUB"
                | "UNDEROVER"
                | "REL"
                | "BUILDREL"
                | "COLOR"
                | "BOX"
                | "OVERBRACE"
                | "UNDERBRACE"
                | "BIGSQCAP"
                | "BIGOMINUS"
                | "BIGODIV"
                | "LEFT"
                | "RIGHT"
                | "WEDGE"
                | "SMALLUNION"
                | "DIVIDE"
                | "LCEIL"
                | "RCEIL"
                | "LFLOOR"
                | "RFLOOR"
        )
}
fn escape_text(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        match c {
            '\\' => out.push_str(r"\textbackslash{}"),
            '{' | '}' | '%' | '$' | '#' | '&' | '_' => {
                out.push('\\');
                out.push(c);
            }
            '^' => out.push_str(r"\textasciicircum{}"),
            '~' => out.push_str(r"\textasciitilde{}"),
            _ => out.push(c),
        }
    }
    out
}

// Quotation prevents native command recognition, but does not select Roman.
// Preserve its literal characters inside the active TeX math font.
fn escape_math_text(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        match c {
            ' ' => out.push_str(r"\ "),
            '\\' => out.push_str(r"\backslash{}"),
            '{' | '}' | '%' | '$' | '#' | '&' | '_' => {
                out.push('\\');
                out.push(c);
            }
            '^' => out.push_str(r"\text{\textasciicircum{}}"),
            '~' => out.push_str(r"\text{\textasciitilde{}}"),
            '\'' => out.push_str(r"\text{'}"),
            _ => out.push(c),
        }
    }
    out
}

fn monospace_ascii(ch: char) -> Option<char> {
    let value = ch as u32;
    let ascii = match value {
        0x1d670..=0x1d689 => b'A' as u32 + value - 0x1d670,
        0x1d68a..=0x1d6a3 => b'a' as u32 + value - 0x1d68a,
        0x1d7f6..=0x1d7ff => b'0' as u32 + value - 0x1d7f6,
        _ => return None,
    };
    char::from_u32(ascii)
}
