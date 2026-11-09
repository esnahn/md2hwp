// SPDX-License-Identifier: AGPL-3.0-only
//! A deliberately bounded, direct TeX math parser for Hancom equation SCRIPT.
//! It accepts math expressions, not TeX documents or macro programs.
use crate::is_hangul_letter;
use crate::math_alphabet::MathAlphabet;
use crate::tex_style::{MathStyle, has_infix_fraction};
use std::fmt;

// Private provenance delimiters never occur in accepted source input.
const LITERAL_START: char = '\x01';
const LITERAL_END: char = '\x02';
const MAX_BYTES: usize = 262_144;
const MAX_DEPTH: usize = 96;
const MAX_ATOMS: usize = 32_768;

fn breve_scalar(body: &str) -> Option<char> {
    let mut literal = body.trim();
    while literal.starts_with('{') && literal.ends_with('}') {
        let mut depth = 0;
        let entire_group = literal.char_indices().all(|(offset, ch)| {
            match ch {
                '{' => depth += 1,
                '}' => depth -= 1,
                _ => {}
            }
            depth > 0 || offset + ch.len_utf8() == literal.len()
        });
        if !entire_group {
            break;
        }
        literal = literal[1..literal.len() - 1].trim();
    }
    if let Some(value) = literal
        .strip_prefix(LITERAL_START)
        .and_then(|s| s.strip_suffix(LITERAL_END))
    {
        literal = value;
    } else if let Some(value) = literal.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
        literal = value;
    }
    let mut chars = literal.chars();
    if let Some(first) = chars.next()
        && chars.next().is_none()
    {
        return Some(first);
    }
    match literal {
        "alpha" => Some('α'),
        "iota" => Some('ι'),
        "upsilon" => Some('υ'),
        "Alpha" => Some('Α'),
        "Iota" => Some('Ι'),
        "Upsilon" => Some('Υ'),
        _ => None,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Font {
    roman: bool,
    bold: bool,
}
impl Font {
    const ITALIC: Self = Self {
        roman: false,
        bold: false,
    };
    const ROMAN: Self = Self {
        roman: true,
        bold: false,
    };
    fn command(self) -> &'static str {
        // Both shape switches clear native bold. Restore shape first, then
        // weight: a bare bold cannot restore a shape changed by a nested scope.
        match (self.roman, self.bold) {
            (false, false) => "it",
            (true, false) => "rm",
            (false, true) => "it bold",
            (true, true) => "rm bold",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TexError {
    /// UTF-8 byte offset in the original input.
    pub offset: usize,
    pub message: String,
}
impl fmt::Display for TexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TeX at byte {}: {}", self.offset, self.message)
    }
}
impl std::error::Error for TexError {}

/// Convert a supported TeX math expression directly to Hancom SCRIPT.
pub fn tex_to_hml(tex: &str) -> Result<String, TexError> {
    tex_to_hml_with_display(tex, false)
}

/// Convert with the initial TeX display style selected by the caller.
pub fn tex_to_hml_with_display(tex: &str, display: bool) -> Result<String, TexError> {
    if let Some(offset) = tex.find([LITERAL_START, LITERAL_END]) {
        return Err(TexError {
            offset,
            message: "reserved internal character in input".into(),
        });
    }
    if tex.len() > MAX_BYTES {
        return Err(TexError {
            offset: 0,
            message: "input byte limit exceeded".into(),
        });
    }
    let mut p = Parser {
        source: tex,
        pos: 0,
        depth: 0,
        atoms: 0,
        font: Font::ITALIC,
        alphabet: MathAlphabet::Normal,
        boldsymbol: false,
        explicit_greek_italic: false,
        scale: 100,
        style: if display {
            MathStyle::Display
        } else {
            MathStyle::Text
        },
        monospace: false,
        text_italic: false,
        colored_right_tail: None,
        delimiter_depth: 0,
    };
    let out = p.sequence(None, false)?;
    if p.pos != tex.len() {
        return p.fail("unexpected right delimiter");
    }
    if out.trim().is_empty() {
        return p.fail("empty math expression");
    }
    lower_literal_words(&out)
}

fn close_bold_segment(out: &mut Vec<String>, start: &mut Option<usize>) {
    if let Some(start) = start.take() {
        let body = out.drain(start..).collect::<Vec<_>>().join(" ");
        if !body.is_empty() {
            out.push(format!("bold {{{body}}}"));
        }
    }
}

fn lower_literal_words(script: &str) -> Result<String, TexError> {
    struct Literal<'a> {
        start: usize,
        end: usize,
        text: &'a str,
        quote: bool,
    }
    fn finish_run(literals: &mut [Literal<'_>], run: &mut Vec<usize>, word: &mut String) {
        if crate::hml_to_tex::literal_needs_quotes(word) {
            for &index in run.iter() {
                literals[index].quote = true;
            }
        }
        run.clear();
        word.clear();
    }
    let invalid = || TexError {
        offset: 0,
        message: "invalid internal literal provenance".into(),
    };
    let mut literals = Vec::new();
    let mut run = Vec::new();
    let mut word = String::new();
    let mut script_groups = Vec::new();
    let mut pending_script = false;
    let mut pos = 0;
    while pos < script.len() {
        let c = script[pos..].chars().next().expect("in bounds");
        if c == LITERAL_START {
            let start = pos;
            pos += 1;
            let length = script[pos..].find(LITERAL_END).ok_or_else(invalid)?;
            let text = &script[pos..pos + length];
            if text.is_empty() || !text.bytes().all(|c| c.is_ascii_alphabetic()) {
                return Err(invalid());
            }
            pos += length + 1;
            // Preserve the native reader's closed constructor grammar too:
            // an ordinary word followed by a group would otherwise look like
            // an unknown constructor on a supported SCRIPT roundtrip.
            let before_group = text.len() > 2 && script[pos..].trim_start().starts_with('{');
            let quote =
                text.len() > 9 || before_group || crate::hml_to_tex::literal_needs_quotes(text);
            if quote {
                finish_run(&mut literals, &mut run, &mut word);
            }
            literals.push(Literal {
                start,
                end: pos,
                text,
                quote,
            });
            if !quote {
                run.push(literals.len() - 1);
                word.push_str(text);
            }
            continue;
        }
        match c {
            LITERAL_END => return Err(invalid()),
            '{' => {
                script_groups.push(pending_script);
                pending_script = false;
            }
            '}' => {
                if script_groups.pop() == Some(true) {
                    finish_run(&mut literals, &mut run, &mut word);
                    if let Some(opaque) = script_groups.last_mut() {
                        *opaque = true;
                    }
                }
            }
            '^' | '_' => {
                finish_run(&mut literals, &mut run, &mut word);
                pending_script = true;
            }
            '"' => {
                finish_run(&mut literals, &mut run, &mut word);
                if let Some(opaque) = script_groups.last_mut() {
                    *opaque = true;
                }
                // Explicit text/operator names have their own quoted semantics.
                pos += 1;
                let length = script[pos..].find('"').ok_or_else(invalid)?;
                pos += length + 1;
                continue;
            }
            c if c.is_whitespace() => {}
            _ => {
                finish_run(&mut literals, &mut run, &mut word);
                // Only letter-only groups are transparent. A fraction,
                // constructor, numeric operand or relation ends its own run
                // before following sibling letters are examined.
                if let Some(opaque) = script_groups.last_mut() {
                    *opaque = true;
                }
            }
        }
        pos += c.len_utf8();
    }
    finish_run(&mut literals, &mut run, &mut word);
    let mut out = String::with_capacity(script.len());
    let mut pos = 0;
    for literal in literals {
        out.push_str(&script[pos..literal.start]);
        if literal.quote {
            out.push('"');
        }
        out.push_str(literal.text);
        if literal.quote {
            out.push('"');
        }
        pos = literal.end;
    }
    out.push_str(&script[pos..]);
    if out.contains([LITERAL_START, LITERAL_END]) {
        return Err(invalid());
    }
    Ok(out)
}

// Annotation operands retain their TeX structure until the complete base is known.
// In particular, emitted SCRIPT text cannot distinguish an arrow from quoted text.
enum AnnotationOperand {
    Plain(String),
    Arrow(&'static str, String),
    Group(Vec<AnnotationOperand>),
    Annotation(Box<Annotation>),
}
enum AboveAnnotation {
    Text(String),
    Arc,
}
struct Annotation {
    base: AnnotationOperand,
    above: Option<AboveAnnotation>,
    below: Option<String>,
}
impl AnnotationOperand {
    fn ungroup(self) -> Self {
        match self {
            Self::Group(mut parts) if parts.len() == 1 => parts.remove(0).ungroup(),
            other => other,
        }
    }
    fn lower(self) -> String {
        match self {
            Self::Plain(text) | Self::Arrow(_, text) => text,
            Self::Group(parts) => format!(
                "{{{}}}",
                parts
                    .into_iter()
                    .map(Self::lower)
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            Self::Annotation(annotation) => annotation.lower(),
        }
    }
}
impl Annotation {
    fn lower(self) -> String {
        let Self { base, above, below } = self;
        // Native relation/accent syntax is chosen only after generic annotation
        // parsing; it never recognizes names in generated text or drops content.
        if let Some(AboveAnnotation::Arc) = above {
            return format!("{{arch {{{}}}}}", base.lower());
        }
        let above = above.map(|label| match label {
            AboveAnnotation::Text(text) => text,
            AboveAnnotation::Arc => unreachable!(),
        });
        if let AnnotationOperand::Arrow(arrow, _) = &base {
            return format!(
                "{{rel {arrow} {{{}}} {{{}}}}}",
                above.as_deref().unwrap_or(""),
                below.as_deref().unwrap_or("")
            );
        }
        let body = base.lower();
        let mut result = native_large_operator(&body)
            .map(str::to_owned)
            .unwrap_or_else(|| format!("underover {{{body}}}"));
        if let Some(below) = below {
            result.push_str(&format!("_{{{below}}}"));
        }
        if let Some(above) = above {
            result.push_str(&format!("^{{{above}}}"));
        }
        format!("{{{result}}}")
    }
}

fn native_large_operator(body: &str) -> Option<&'static str> {
    Some(match body {
        "SQCAP" => "BIGSQCAP",
        "OMINUS" => "BIGOMINUS",
        "ODIV" => "BIGODIV",
        "LOR" => "BIGVEE",
        "WEDGE" => "BIGWEDGE",
        _ => return None,
    })
}

struct Parser<'a> {
    source: &'a str,
    pos: usize,
    depth: usize,
    atoms: usize,
    font: Font,
    alphabet: MathAlphabet,
    // Math-version bold affects explicit math alphabets and operator names,
    // independently of a nested mathbf/mathit alphabet selection.
    boldsymbol: bool,
    // Default TeX uppercase Greek is upright, unlike the default Latin alphabet.
    explicit_greek_italic: bool,
    scale: u16,
    style: MathStyle,
    monospace: bool,
    // Current text shape. Native font switches are not brace-scoped, so nested
    // text commands restore the previous shape explicitly.
    text_italic: bool,
    colored_right_tail: Option<([u8; 3], String)>,
    delimiter_depth: usize,
}
impl Parser<'_> {
    fn fail<T>(&self, message: &str) -> Result<T, TexError> {
        Err(TexError {
            offset: self.pos,
            message: message.into(),
        })
    }
    fn peek(&self) -> Option<char> {
        self.source[self.pos..].chars().next()
    }
    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }
    fn whitespace(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.bump();
        }
    }
    fn starts(&self, s: &str) -> bool {
        self.source[self.pos..].starts_with(s)
    }
    fn at_command(&self, name: &str) -> bool {
        self.source[self.pos..]
            .strip_prefix('\\')
            .and_then(|s| s.strip_prefix(name))
            .is_some_and(|tail| !tail.starts_with(|c: char| c.is_ascii_alphabetic()))
    }
    fn sequence(&mut self, close: Option<char>, cells: bool) -> Result<String, TexError> {
        let previous_scale = self.scale;
        let previous_style = self.style;
        let previous_font = self.font;
        let previous_monospace = self.monospace;
        let previous_greek = self.explicit_greek_italic;
        let previous_alphabet = self.alphabet;
        let mut segment_style = self.style;
        let mut infix_style = has_infix_fraction(&self.source[self.pos..], close, cells)
            .then(|| segment_style.fraction());
        if let Some(style) = infix_style {
            self.style = style;
        }
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return self.fail("nesting limit exceeded");
        }
        let mut out: Vec<String> = Vec::new();
        let mut bold_start = None;
        let mut lines = Vec::new();
        let mut atop_numerator = None;
        let mut infix_command = "atop";
        let mut hangul_end = None;
        let mut ascii_literal_end = None;
        let mut active_color = None;
        let mut active_color_start = 0;
        let mut double_quotes = 0usize;
        let mut trailing_tex_glue = false;
        loop {
            self.whitespace();
            let Some(c) = self.peek() else {
                if close.is_some() {
                    return self.fail("unclosed group");
                }
                break;
            };
            if close == Some(c) {
                self.bump();
                break;
            }
            if cells
                && (c == '&'
                    || self.starts("\\\\")
                    || self.at_command("cr")
                    || self.at_command("newline")
                    || self.at_command("end"))
            {
                break;
            }
            if !cells && (self.starts("\\\\") || self.at_command("newline")) {
                self.count_annotation_atom()?;
                self.pos += if self.at_command("newline") { 8 } else { 2 };
                if close == Some('}') {
                    self.whitespace();
                    if matches!(self.peek(), Some('[' | '*')) {
                        return self.fail(
                            "grouped line break with optional spacing or star is unsupported",
                        );
                    }
                    // LaTeX's ordinary newline expands to unskip plus
                    // zero-natural-width glue and penalties. A sub-mlist is
                    // packed horizontally, so those penalties cannot split it.
                    if trailing_tex_glue
                        && out
                            .last()
                            .is_some_and(|part| part.chars().all(|c| matches!(c, '~' | '`')))
                    {
                        out.pop();
                    }
                    trailing_tex_glue = false;
                    hangul_end = None;
                    ascii_literal_end = None;
                    continue;
                }
                self.discard_row_spacing()?;
                close_bold_segment(&mut out, &mut bold_start);
                let line = self.finish_infix_fraction(
                    infix_command,
                    atop_numerator.take(),
                    std::mem::take(&mut out).join(" "),
                )?;
                if line.trim().is_empty() {
                    return self.fail("empty equation line");
                }
                lines.push(line);
                if self.font.bold {
                    bold_start = Some(0);
                }
                segment_style = if infix_style.is_some() {
                    segment_style
                } else {
                    self.style
                };
                infix_style = has_infix_fraction(&self.source[self.pos..], close, cells)
                    .then(|| segment_style.fraction());
                self.style = infix_style.unwrap_or(segment_style);
                hangul_end = None;
                ascii_literal_end = None;
                continue;
            }
            if self.at_command("right") {
                self.colored_right_tail = if let Some(rgb) = active_color {
                    if atop_numerator.is_some() || !lines.is_empty() {
                        return self.fail("colored right delimiter with an infix fraction or outer line break is unsupported");
                    }
                    Some((rgb, out.split_off(active_color_start).join(" ")))
                } else {
                    None
                };
                break;
            }
            if self.at_command("color") {
                self.count_annotation_atom()?;
                self.pos += "\\color".len();
                active_color = Some(self.color_spec()?);
                active_color_start = out.len();
                hangul_end = None;
                ascii_literal_end = None;
                continue;
            }
            if let Some(name) = ["rm", "it", "bf", "tt", "sf"]
                .into_iter()
                .find(|name| self.at_command(name))
            {
                self.count_annotation_atom()?;
                self.pos += name.len() + 1;
                close_bold_segment(&mut out, &mut bold_start);
                self.font = match name {
                    "it" => Font::ITALIC,
                    "bf" => Font {
                        bold: true,
                        ..Font::ROMAN
                    },
                    _ => Font::ROMAN,
                };
                self.monospace = name == "tt";
                self.explicit_greek_italic = name == "it";
                self.alphabet = MathAlphabet::Normal;
                out.push(if self.font.roman { "rm" } else { "it" }.into());
                if self.font.bold {
                    bold_start = Some(out.len());
                }
                hangul_end = None;
                ascii_literal_end = None;
                continue;
            }
            if c == '}' {
                return self.fail("unexpected closing delimiter");
            }
            if c == '^' || c == '_' {
                let atom = self.script_suffix("{}".into())?;
                out.push(atom);
                hangul_end = None;
                ascii_literal_end = None;
                continue;
            }
            if let Some(command) = self.infix_fraction_command() {
                infix_command =
                    self.consume_infix_fraction(command, &out, atop_numerator.is_some())?;
                self.style = segment_style.fraction();
                close_bold_segment(&mut out, &mut bold_start);
                atop_numerator = Some(std::mem::take(&mut out).join(" "));
                if self.font.bold {
                    bold_start = Some(0);
                }
                hangul_end = None;
                ascii_literal_end = None;
                continue;
            }
            let start = self.pos;
            let mut atom = if let Some(quote) = self.paired_quote(&mut double_quotes)? {
                self.script_suffix(quote)?
            } else {
                self.scripted_atom()?
            };
            if atom.is_empty() {
                continue;
            }
            let size_declaration = atom
                .strip_prefix("scale ")
                .is_some_and(|size| size.parse::<u16>().is_ok());
            if let Some(rgb) = active_color.filter(|_| !size_declaration) {
                out.push(native_color(rgb, std::mem::take(&mut atom)));
                hangul_end = None;
                ascii_literal_end = None;
                continue;
            }
            let end = self.native_hangul_end(start, &atom);
            let literal_end = self.native_ascii_literal_end(start, &atom);
            if end.is_some() && hangul_end == Some(start) {
                out.last_mut()
                    .expect("preceding Hangul atom")
                    .push_str(&atom);
            } else if literal_end.is_some() && self.only_whitespace_since(ascii_literal_end, start)
            {
                let previous = out.last_mut().expect("preceding ASCII literal");
                previous.pop(); // Join source literals, never across a script.
                previous.push_str(&atom[1..]);
            } else {
                out.push(atom);
            }
            hangul_end = end;
            ascii_literal_end = literal_end;
            trailing_tex_glue = out
                .last()
                .is_some_and(|part| part.chars().all(|c| matches!(c, '~' | '`')))
                && ["quad", "qquad", "hspace", ",", ":", ";", " "]
                    .iter()
                    .any(|name| {
                        self.source[start..]
                            .strip_prefix('\\')
                            .and_then(|s| s.strip_prefix(name))
                            .is_some_and(|tail| {
                                !name.ends_with(|c: char| c.is_ascii_alphabetic())
                                    || !tail.starts_with(|c: char| c.is_ascii_alphabetic())
                            })
                    });
        }
        close_bold_segment(&mut out, &mut bold_start);
        self.depth -= 1;
        self.style = previous_style;
        if self.font != previous_font {
            out.push(previous_font.command().into());
        }
        self.font = previous_font;
        self.monospace = previous_monospace;
        self.explicit_greek_italic = previous_greek;
        self.alphabet = previous_alphabet;
        if self.scale != previous_scale {
            if out.is_empty() && atop_numerator.is_none() && !lines.is_empty() {
                lines
                    .last_mut()
                    .expect("previous line")
                    .push_str(&format!(" scale {previous_scale}"));
            } else {
                out.push(format!("scale {previous_scale}"));
            }
            self.scale = previous_scale;
        }
        let last = self.finish_infix_fraction(infix_command, atop_numerator, out.join(" "))?;
        if lines.is_empty() {
            return Ok(last);
        }
        if !last.trim().is_empty() {
            lines.push(last);
        }
        Ok(format!("{{pile {{{}}}}}", lines.join(" # ")))
    }
    fn infix_fraction_command(&self) -> Option<&'static str> {
        ["atop", "over", "brace", "brack", "above"]
            .into_iter()
            .find(|name| self.at_command(name))
    }
    fn consume_infix_fraction<T>(
        &mut self,
        command: &'static str,
        numerator: &[T],
        duplicate: bool,
    ) -> Result<&'static str, TexError> {
        if duplicate {
            return self.fail(&format!(
                "duplicate infix fraction \\{command} in the same math group"
            ));
        }
        if numerator.is_empty() {
            return self.fail(&format!("empty \\{command} numerator"));
        }
        self.count_annotation_atom()?;
        self.pos += command.len() + 1;
        if command == "above" {
            self.whitespace();
            let thickness = if self.peek() == Some('{') {
                self.raw_group()?
            } else {
                let start = self.pos;
                if matches!(self.peek(), Some('+' | '-')) {
                    self.bump();
                }
                while self.peek().is_some_and(|c| c.is_ascii_digit() || c == '.') {
                    self.bump();
                }
                self.whitespace();
                for _ in 0..2 {
                    if self.peek().is_some_and(|c| c.is_ascii_alphabetic()) {
                        self.bump();
                    }
                }
                self.source[start..self.pos].to_owned()
            };
            if thickness.trim().is_empty() {
                return self.fail("above requires a fraction thickness");
            }
            return Ok(if self.fraction_has_rule(&thickness)? {
                "over"
            } else {
                "atop"
            });
        }
        Ok(command)
    }
    fn finish_infix_fraction(
        &self,
        command: &str,
        numerator: Option<String>,
        denominator: String,
    ) -> Result<String, TexError> {
        match numerator {
            Some(numerator) => {
                if denominator.trim().is_empty() {
                    return self.fail(&format!("empty \\{command} denominator"));
                }
                Ok(match command {
                    "brace" => {
                        format!("{{LEFT {{ {{{{{numerator}}} atop {{{denominator}}}}} RIGHT }}}}")
                    }
                    "brack" => {
                        format!("{{LEFT [ {{{{{numerator}}} atop {{{denominator}}}}} RIGHT ]}}")
                    }
                    _ => format!("{{{{{numerator}}} {command} {{{denominator}}}}}"),
                })
            }
            None => Ok(denominator),
        }
    }
    fn native_hangul_end(&self, start: usize, atom: &str) -> Option<usize> {
        // Coalesce only literal, adjacent, unmodified Hangul atoms. Parsing
        // still consumes TeX arguments one character at a time, and a script
        // prevents merging its base into the preceding native word.
        (!atom.is_empty()
            && atom.chars().all(is_hangul_letter)
            && self.source[start..].starts_with(atom))
        .then_some(start + atom.len())
    }
    fn native_ascii_literal_end(&self, start: usize, atom: &str) -> Option<usize> {
        let c = self.source[start..].chars().next()?;
        (c.is_ascii_alphabetic() && atom == format!("{LITERAL_START}{c}{LITERAL_END}"))
            .then_some(start + 1)
    }
    fn only_whitespace_since(&self, end: Option<usize>, start: usize) -> bool {
        // Ordinary TeX whitespace has no visible width. Join only source
        // letters separated by that whitespace, never emitted commands,
        // groups, scripts or explicit spacing. Quoting also keeps long words
        // intact under the native nine-character word rule.
        end.is_some_and(|end| {
            end <= start && self.source[end..start].chars().all(char::is_whitespace)
        })
    }
    fn scripted_atom(&mut self) -> Result<String, TexError> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return self.fail("nesting limit exceeded");
        }
        let atom = self.atom()?;
        let result = self.script_suffix(atom);
        self.depth -= 1;
        result
    }
    fn paired_quote(&mut self, open: &mut usize) -> Result<Option<String>, TexError> {
        let opening = self.starts("``");
        if !opening && !(*open > 0 && self.starts("''")) {
            return Ok(None);
        }
        self.count_annotation_atom()?;
        self.count_annotation_atom()?;
        self.pos += 2;
        if opening {
            *open += 1;
        } else {
            *open -= 1;
        }
        Ok(Some(if opening { "\"“\"" } else { "\"”\"" }.into()))
    }
    fn script_suffix(&mut self, mut atom: String) -> Result<String, TexError> {
        let mut scripts = Vec::new();
        loop {
            self.whitespace();
            let Some(kind @ ('^' | '_')) = self.peek() else {
                break;
            };
            if scripts.iter().any(|(k, _)| *k == kind) {
                return self.fail("duplicate script");
            }
            self.bump();
            let arg = self.styled_argument(self.style.script())?;
            if arg.is_empty() {
                return self.fail("empty script argument");
            }
            scripts.push((kind, arg));
        }
        if atom == "{}" && scripts.len() == 1 && scripts[0] == ('^', "CIRC".into()) {
            // The standard scalar degree construction is not a left script.
            // Consume only an exact explicit upright C/F unit, never adjacent text.
            let tail = &self.source[self.pos..];
            if let Some(rest) = tail.strip_prefix(r"\mathrm")
                && let Some(body) = rest.trim_start().strip_prefix('{')
                && let Some(end) = body.find('}')
            {
                let unit = body[..end].trim();
                if matches!(unit, "C" | "F") {
                    self.count_annotation_atom()?;
                    self.count_annotation_atom()?;
                    self.pos += tail.len() - body.len() + end + 1;
                    return self.script_suffix(if unit == "C" {
                        "CENTIGRADE".into()
                    } else {
                        "FAHRENHEIT".into()
                    });
                }
            }
            return Ok("DEG".into());
        }
        if atom == "{}" && !scripts.is_empty() {
            // TeX's empty-base idiom is the inverse of native LSUP/LSUB.
            // Consume the actual following base with its independent scripts.
            if self
                .peek()
                .is_none_or(|c| matches!(c, '}' | ']' | '&' | '+' | '-' | '=' | ',' | ';' | ':'))
                || self.at_command("end")
                || self.at_command("right")
            {
                for (kind, arg) in scripts {
                    atom.push_str(&format!(" {kind}{{{arg}}}"));
                }
                return Ok(atom);
            }
            atom = self.scripted_atom()?;
            for (kind, arg) in scripts {
                atom.push_str(&format!(
                    " {} {{{arg}}}",
                    if kind == '^' { "lsup" } else { "lsub" }
                ));
            }
        } else {
            for (kind, arg) in scripts {
                atom.push_str(&format!(" {kind}{{{arg}}}"));
            }
        }
        Ok(atom)
    }
    fn argument(&mut self) -> Result<String, TexError> {
        let previous_style = self.style;
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return self.fail("nesting limit exceeded");
        }
        let result = self.argument_inner();
        self.depth -= 1;
        self.style = previous_style;
        result
    }
    fn styled_argument(&mut self, style: MathStyle) -> Result<String, TexError> {
        let previous = self.style;
        self.style = style;
        let result = self.argument();
        self.style = previous;
        result
    }
    fn styled_sequence(
        &mut self,
        style: MathStyle,
        close: Option<char>,
        cells: bool,
    ) -> Result<String, TexError> {
        let previous = self.style;
        self.style = style;
        let result = self.sequence(close, cells);
        self.style = previous;
        result
    }
    fn fraction_arguments(&mut self, style: MathStyle) -> Result<(String, String), TexError> {
        let child = style.fraction();
        Ok((self.styled_argument(child)?, self.styled_argument(child)?))
    }
    fn argument_inner(&mut self) -> Result<String, TexError> {
        self.whitespace();
        if self.peek() == Some('{') {
            self.bump();
            return self.sequence(Some('}'), false);
        }
        if self
            .peek()
            .is_none_or(|c| matches!(c, '}' | ']' | '^' | '_' | '&'))
        {
            return self.fail("missing argument");
        }
        // An unbraced TeX argument consumes one character, including digits
        // and each colon of an otherwise recognized adjacent colon pair.
        if self.peek() == Some(':') {
            self.count_annotation_atom()?;
            self.bump();
            return Ok(":".into());
        }
        if self.peek().is_some_and(|c| c.is_ascii_digit()) {
            return Ok(self.bump().unwrap().to_string());
        }
        self.atom()
    }
    fn discard_numbering_group(&mut self) -> Result<(), TexError> {
        self.whitespace();
        if self.bump() != Some('{') {
            return self.fail("expected numbering argument");
        }
        let mut nesting = 1;
        while let Some(c) = self.bump() {
            match c {
                '\\' => {
                    self.bump();
                }
                '{' => {
                    nesting += 1;
                    if self.depth + nesting > MAX_DEPTH {
                        return self.fail("nesting limit exceeded");
                    }
                }
                '}' => {
                    nesting -= 1;
                    if nesting == 0 {
                        return Ok(());
                    }
                }
                _ => {}
            }
        }
        self.fail("unclosed numbering argument")
    }
    fn raw_group(&mut self) -> Result<String, TexError> {
        self.whitespace();
        if self.bump() != Some('{') {
            return self.fail("expected braced argument");
        }
        let mut out = String::new();
        loop {
            match self.bump() {
                Some('}') => return Ok(out),
                Some('{') => return self.fail("nested literal group is unsupported"),
                Some(c) if c.is_control() => return self.fail("control character in literal"),
                Some(c) => out.push(c),
                None => return self.fail("unclosed literal group"),
            }
        }
    }
    fn blackboard(&mut self) -> Result<String, TexError> {
        self.whitespace();
        let tail = &self.source[self.pos..];
        let simple = if let Some(group) = tail.strip_prefix('{') {
            group.split_once('}').is_some_and(|(text, _)| {
                text.chars().any(|c| c.is_ascii_uppercase())
                    && text
                        .chars()
                        .all(|c| c.is_ascii_uppercase() || c.is_whitespace())
            })
        } else {
            self.peek().is_some_and(|c| c.is_ascii_uppercase())
        };
        if !simple {
            return self.alphabet_argument(MathAlphabet::Blackboard, "mathbb");
        }
        let (raw, braced) = if self.peek() == Some('{') {
            self.bump();
            let mut raw = String::new();
            loop {
                match self.bump() {
                    Some('}') => break,
                    Some('{') => return self.fail("nested \\mathbb groups are unsupported"),
                    Some(c) if c.is_whitespace() => raw.push(c),
                    Some(c) if c.is_control() => {
                        return self.fail("control character in \\mathbb argument");
                    }
                    Some(c) => raw.push(c),
                    None => return self.fail("unclosed \\mathbb argument"),
                }
            }
            (raw, true)
        } else {
            match self.peek() {
                Some(c) if c.is_ascii_uppercase() => {
                    self.bump();
                    (c.to_string(), false)
                }
                Some(_) => return self.fail("\\mathbb requires uppercase ASCII letters"),
                None => return self.fail("\\mathbb requires an argument"),
            }
        };
        let mut symbols = Vec::new();
        for c in raw.chars() {
            if c.is_whitespace() {
                continue;
            }
            if !c.is_ascii_uppercase() {
                return self.fail("\\mathbb accepts only uppercase ASCII letters and whitespace");
            }
            self.count_annotation_atom()?;
            symbols.push(format!("Vec{c}"));
        }
        if symbols.is_empty() {
            return self.fail("\\mathbb requires at least one uppercase ASCII letter");
        }
        let output = symbols.join(" ");
        Ok(if braced && symbols.len() > 1 {
            format!("{{{output}}}")
        } else {
            output
        })
    }
    fn command_name(&mut self) -> String {
        let start = self.pos;
        while self.peek().is_some_and(|c| c.is_ascii_alphabetic()) {
            self.bump();
        }
        if self.pos == start {
            self.bump();
        }
        self.source[start..self.pos].into()
    }
    fn atom(&mut self) -> Result<String, TexError> {
        self.atoms += 1;
        if self.atoms > MAX_ATOMS {
            return self.fail("atom limit exceeded");
        }
        let start = self.pos;
        match self.bump() {
            Some('{') => Ok(format!("{{{}}}", self.sequence(Some('}'), false)?)),
            Some('\\') => {
                let name = self.command_name();
                self.command(&name, start)
            }
            Some(c) if c.is_ascii_digit() => {
                while self.peek().is_some_and(|x| x.is_ascii_digit() || x == '.') {
                    self.bump();
                }
                let value = &self.source[start..self.pos];
                Ok(if self.monospace {
                    format!("\"{}\"", monospace_text(value))
                } else {
                    value.into()
                })
            }
            // Retain source-letter provenance until contextual native recognition
            // and operand-length checks can decide whether quotes are necessary.
            Some(c) if c.is_ascii_alphabetic() => Ok(
                if self.alphabet == MathAlphabet::Blackboard && c.is_ascii_uppercase() {
                    format!("Vec{c}")
                } else if self.alphabet == MathAlphabet::Calligraphic && c == 'L' {
                    "LAPLACE".into()
                } else if let Some(glyph) = self.alphabet.glyph(c).filter(|&glyph| glyph != c) {
                    format!("\"{glyph}\"")
                } else if self.monospace {
                    format!("\"{}\"", monospace_text(&c.to_string()))
                } else {
                    format!("{LITERAL_START}{c}{LITERAL_END}")
                },
            ),
            Some(':') if self.peek() == Some(':') => {
                self.count_annotation_atom()?;
                self.bump();
                Ok("IDENTICAL".into())
            }
            Some('*') => Ok("\"*\"".into()),
            // In math mode TeX maps each backtick to a left single quote;
            // native backticks are spacing operators, even in quoted SCRIPT.
            Some('`') => Ok("\"‘\"".into()),
            Some(c) if "+-=<>(),.!?/:;|'[]".contains(c) => Ok(c.to_string()),
            Some('~') => Ok("~".into()),
            Some(c) if is_hangul_letter(c) => Ok(c.to_string()),
            Some(c) if !c.is_ascii() && !c.is_control() => {
                if let Some(native) = unicode_symbol(c) {
                    return Ok(native.into());
                }
                // Other Unicode literals retain the existing quoted spelling.
                // Native Hangul needs no quotation or command escaping.
                Ok(format!("\"{c}\""))
            }
            Some(_) => self.fail("unsupported literal character (escape TeX special characters)"),
            None => self.fail("missing argument"),
        }
    }
    fn delimiter(&mut self) -> Result<String, TexError> {
        self.whitespace();
        if self.peek() == Some('\\') {
            self.bump();
            return match self.command_name().as_str() {
                "{" | "lbrace" => Ok("{".into()),
                "}" | "rbrace" => Ok("}".into()),
                "langle" => Ok("<".into()),
                "rangle" => Ok(">".into()),
                "vert" | "lvert" | "rvert" => Ok("|".into()),
                "Vert" | "lVert" | "rVert" => Ok("∥".into()),
                "lceil" => Ok("⌈".into()),
                "rceil" => Ok("⌉".into()),
                "lfloor" => Ok("⌊".into()),
                "rfloor" => Ok("⌋".into()),
                name @ ("uparrow" | "downarrow" | "updownarrow" | "Uparrow" | "Downarrow"
                | "Updownarrow") => Ok(symbol(name).unwrap().into()),
                _ => self.fail("unsupported delimiter"),
            };
        }
        match self.bump() {
            Some(c) if "()[]|/.<>".contains(c) => Ok(c.to_string()),
            _ => self.fail("missing or unsupported delimiter"),
        }
    }
    fn fraction_delimiter(&mut self) -> Result<String, TexError> {
        self.whitespace();
        if self.peek() != Some('{') {
            return self.delimiter();
        }
        self.bump();
        self.whitespace();
        if self.peek() == Some('}') {
            self.bump();
            return Ok(".".into());
        }
        let delimiter = self.delimiter()?;
        self.whitespace();
        if self.bump() != Some('}') {
            return self.fail("genfrac delimiter must contain one delimiter");
        }
        Ok(delimiter)
    }
    fn fraction_literal_argument(&mut self) -> Result<String, TexError> {
        self.whitespace();
        if self.peek() == Some('{') {
            self.raw_group()
        } else {
            match self.bump() {
                Some(c) if c.is_ascii_digit() => Ok(c.to_string()),
                _ => self.fail("genfrac requires a literal argument"),
            }
        }
    }
    fn fraction_has_rule(&self, thickness: &str) -> Result<bool, TexError> {
        let thickness = thickness.trim();
        let ruled = if thickness.is_empty() {
            true
        } else {
            let split = thickness
                .find(|c: char| !c.is_ascii_digit() && !matches!(c, '+' | '-' | '.'))
                .unwrap_or(thickness.len());
            let (number, unit) = thickness.split_at(split);
            let value = number.parse::<f64>().ok().filter(|n| n.is_finite());
            if value.is_none()
                || !matches!(
                    unit.trim(),
                    "pt" | "mm"
                        | "cm"
                        | "in"
                        | "bp"
                        | "pc"
                        | "dd"
                        | "cc"
                        | "sp"
                        | "em"
                        | "ex"
                        | "mu"
                )
            {
                return self.fail("fraction thickness requires a finite decimal TeX dimension");
            }
            value.unwrap() > 0.0
        };
        Ok(ruled)
    }
    fn generalized_fraction(&mut self) -> Result<String, TexError> {
        let left = self.fraction_delimiter()?;
        let right = self.fraction_delimiter()?;
        let thickness = self.fraction_literal_argument()?;
        let ruled = self.fraction_has_rule(&thickness)?;
        let style = self.fraction_literal_argument()?;
        let fraction_style = match style.trim() {
            "" => self.style,
            "0" => MathStyle::Display,
            "1" => MathStyle::Text,
            "2" => MathStyle::Script,
            "3" => MathStyle::ScriptScript,
            _ => return self.fail("genfrac style must be empty or 0, 1, 2, 3"),
        };
        let scale = match style.trim() {
            "" => None,
            "0" | "1" => Some(100),
            "2" => Some(70),
            "3" => Some(50),
            _ => return self.fail("genfrac style must be empty or 0, 1, 2, 3"),
        };
        let previous_scale = self.scale;
        let scale = scale.filter(|&value| value != 100 || previous_scale != 100);
        if let Some(scale) = scale {
            self.scale = scale;
        }
        let (a, b) = self.fraction_arguments(fraction_style)?;
        self.scale = previous_scale;
        let operator = if ruled { "over" } else { "atop" };
        let mut fraction = format!("{{{{{a}}} {operator} {{{b}}}}}");
        if left != "." || right != "." {
            fraction = format!("{{LEFT {left} {fraction} RIGHT {right}}}");
        }
        if let Some(scale) = scale {
            fraction = format!("{{scale {scale} {fraction} scale {previous_scale}}}");
        }
        Ok(fraction)
    }
    fn declare_scale(&mut self, scale: u16) -> String {
        let omit = scale == 100 && self.scale == 100;
        self.scale = scale;
        if omit {
            String::new()
        } else {
            format!("scale {scale}")
        }
    }
    fn small_style_operator(&mut self) -> Result<Option<&'static str>, TexError> {
        let start = self.pos;
        self.whitespace();
        for (command, native) in [
            ("sum", "SMALLSUM"),
            ("prod", "SMALLPROD"),
            ("coprod", "SMALLCOPROD"),
        ] {
            if self.at_command(command) {
                self.pos += command.len() + 1;
                self.whitespace();
                if matches!(self.peek(), None | Some('}')) {
                    self.count_annotation_atom()?;
                    return Ok(Some(native));
                }
                break;
            }
        }
        self.pos = start;
        Ok(None)
    }
    fn command(&mut self, name: &str, start: usize) -> Result<String, TexError> {
        match name {
            // KaTeX's bmod combines 1mu and the 4mu binary spacing in
            // display/text, and uses 5mu directly in script styles. Native
            // SCRIPT does not supply that mathbin spacing automatically.
            "bmod" => Ok("`` mod ``".into()),
            "mod" => {
                let leading = if self.style == MathStyle::Display {
                    "~~"
                } else {
                    "~`"
                };
                Ok(format!("{leading} mod ```"))
            }
            "pmod" | "pod" => {
                let argument = self.argument()?;
                if argument.trim().is_empty() {
                    return self.fail("modulo argument must not be empty");
                }
                let leading = if self.style == MathStyle::Display {
                    "~~"
                } else {
                    "~"
                };
                let prefix = if name == "pmod" { "mod ``` " } else { "" };
                Ok(format!("{leading} {{( {prefix}{argument} )}}"))
            }
            "mathchoice" => {
                let selected = self.style;
                let mut output = String::new();
                for style in MathStyle::ALL {
                    self.whitespace();
                    if self.peek() != Some('{') {
                        return self.fail("mathchoice requires four braced branches");
                    }
                    let branch = self.styled_argument(style)?;
                    if style == selected {
                        output = branch;
                    }
                }
                Ok(format!("{{{output}}}"))
            }
            "nonumber" | "notag" | "hline" | "hdashline" | "nobreak" | "allowbreak" => {
                Ok(String::new())
            }
            "tag" | "label" => {
                self.whitespace();
                if name == "tag" && self.peek() == Some('*') {
                    self.bump();
                }
                self.discard_numbering_group()?;
                Ok(String::new())
            }
            "scriptstyle" | "scriptscriptstyle" | "textstyle" | "displaystyle" => {
                self.style = match name {
                    "displaystyle" => MathStyle::Display,
                    "textstyle" => MathStyle::Text,
                    "scriptstyle" => MathStyle::Script,
                    _ => MathStyle::ScriptScript,
                };
                if name == "textstyle"
                    && let Some(native) = self.small_style_operator()?
                {
                    if self.scale != 100 {
                        self.scale = 100;
                        return Ok(format!("scale 100 {native}"));
                    }
                    return Ok(native.into());
                }
                let scale = match name {
                    "scriptstyle" => 70,
                    "scriptscriptstyle" => 50,
                    _ => 100,
                };
                Ok(self.declare_scale(scale))
            }
            "tiny" | "scriptsize" | "footnotesize" | "small" | "normalsize" | "large" | "Large"
            | "LARGE" | "huge" | "Huge" => {
                let scale = font_size_scale(name).expect("matched size command");
                Ok(self.declare_scale(scale))
            }
            "not" => {
                let operand = self.negation_argument()?;
                Ok(format!("not {operand}"))
            }
            "cancel" | "bcancel" | "xcancel" => {
                let body = self.argument()?;
                self.cancellation_body(&body)
            }
            "sout" => {
                let body = self.text_group()?;
                self.cancellation_body(&body)
            }
            "genfrac" => self.generalized_fraction(),
            "frac" | "dfrac" | "tfrac" => {
                let style = match name {
                    "dfrac" => MathStyle::Display,
                    "tfrac" => MathStyle::Text,
                    _ => self.style,
                };
                let (a, b) = self.fraction_arguments(style)?;
                Ok(format!("{{{{{a}}} over {{{b}}}}}"))
            }
            "cfrac" => {
                self.whitespace();
                if self.peek() == Some('[') {
                    self.bump();
                    self.whitespace();
                    // Accept the standard presentation option without applying
                    // it: native fractions retain their own centered layout.
                    if matches!(self.peek(), Some('l' | 'r')) {
                        self.bump();
                        self.whitespace();
                    }
                    if self.bump() != Some(']') {
                        return self
                            .fail("invalid cfrac alignment option; expected [l], [r], or []");
                    }
                }
                let (a, b) = self.fraction_arguments(MathStyle::Display)?;
                Ok(format!("{{{{{a}}} over {{{b}}}}}"))
            }
            "binom" | "dbinom" | "tbinom" => {
                let style = match name {
                    "dbinom" => MathStyle::Display,
                    "tbinom" => MathStyle::Text,
                    _ => self.style,
                };
                let (a, b) = self.fraction_arguments(style)?;
                Ok(format!("{{binom {{{a}}} {{{b}}}}}"))
            }
            "sqrt" => {
                self.whitespace();
                let index = if self.peek() == Some('[') {
                    self.bump();
                    Some(self.styled_sequence(MathStyle::ScriptScript, Some(']'), false)?)
                } else {
                    None
                };
                let body = self.argument()?;
                Ok(match index {
                    Some(n) => format!("{{root {{{n}}} of {{{body}}}}}"),
                    None => format!("{{sqrt {{{body}}}}}"),
                })
            }
            "vec" | "overrightarrow" | "overleftrightarrow" | "acute" | "grave" | "check"
            | "boxed" | "hat" | "bar" | "tilde" | "dot" | "ddot" | "overline" | "underline"
            | "overgroup" | "widehat" | "widetilde" | "widecheck" => {
                let body = self.argument()?;
                let h = match name {
                    "overline" => "bar",
                    "overrightarrow" => "vec",
                    "overleftrightarrow" => "dyad",
                    "boxed" => "box",
                    "overgroup" => "arch",
                    "widehat" => "hat",
                    "widetilde" => "tilde",
                    "widecheck" => "check",
                    x => x,
                };
                Ok(format!("{{{h} {{{body}}}}}"))
            }
            "overleftarrow" | "Overrightarrow" | "overleftharpoon" | "overrightharpoon"
            | "overlinesegment" => {
                let body = self.argument()?;
                let mark = match name {
                    "overleftarrow" => "larrow",
                    "Overrightarrow" => "RARROW",
                    "overleftharpoon" => "\"↼\"",
                    "overrightharpoon" => "\"⇀\"",
                    _ => "\"─\"",
                };
                Ok(format!("{{underover {{{body}}}^{{{mark}}}}}"))
            }
            "breve" => {
                let body = self.argument()?;
                Ok(
                    match breve_scalar(&body).and_then(crate::breve_unicode::compose_breve) {
                        Some(composed) => format!("\"{composed}\""),
                        None => format!("{{{body} \"˘\"}}"),
                    },
                )
            }
            "underleftarrow" | "underrightarrow" | "underleftrightarrow" => {
                let body = self.argument()?;
                let arrow = match name {
                    "underleftarrow" => "larrow",
                    "underrightarrow" => "rarrow",
                    _ => "lrarrow",
                };
                Ok(format!("{{underover {{{body}}}_{{{arrow}}}}}"))
            }
            "undergroup" | "utilde" => {
                let body = self.argument()?;
                let mark = match name {
                    "undergroup" => "\"⌣\"",
                    _ => "SIM",
                };
                Ok(format!("{{underover {{{body}}}_{{{mark}}}}}"))
            }
            "underlinesegment" => {
                let body = self.argument()?;
                Ok(format!("{{underover {{{body}}}_{{\"─\"}}}}"))
            }
            "underbar" => {
                let body = self.text_group()?;
                Ok(format!("{{under {body}}}"))
            }
            "phase" => {
                let body = self.argument()?;
                Ok(if body.is_empty() {
                    "ANGLE".into()
                } else {
                    format!("ANGLE {{{body}}}")
                })
            }
            "mathllap" | "mathrlap" | "mathclap" | "llap" | "rlap" | "clap" => {
                // User policy: discard overlap/zero-width layout, not content.
                Ok(format!("{{{}}}", self.argument()?))
            }
            "smash" => {
                self.whitespace();
                if self.peek() == Some('[') {
                    self.bump();
                    loop {
                        self.whitespace();
                        if self.peek() == Some(']') {
                            self.bump();
                            break;
                        }
                        self.count_annotation_atom()?;
                        if !matches!(self.bump(), Some('t' | 'b')) {
                            return self.fail("smash option accepts only t and b");
                        }
                    }
                }
                Ok(format!("{{{}}}", self.argument()?))
            }
            "mathbin" => {
                let body = self.argument()?;
                // Explicit binary class: TeX uses medium (4mu) spacing in
                // text/display styles and none in script styles. Quantize
                // through the established 2.25mu native quarter-space model.
                let gap = if matches!(self.style, MathStyle::Display | MathStyle::Text) {
                    "``"
                } else {
                    ""
                };
                Ok(format!("{gap} {{{body}}} {gap}"))
            }
            "mathrel" | "mathord" | "mathopen" | "mathclose" | "mathpunct" | "mathinner" => {
                Ok(format!("{{{}}}", self.argument()?))
            }
            "mathrm" | "mathbf" | "mathit" | "mathnormal" | "mathsf" | "mathsfit" => {
                let target = Font {
                    roman: matches!(name, "mathrm" | "mathbf" | "mathsf"),
                    bold: name == "mathbf" || self.boldsymbol,
                };
                let previous = self.explicit_greek_italic;
                self.explicit_greek_italic = matches!(name, "mathit" | "mathsfit");
                let previous_alphabet = self.alphabet;
                let previous_monospace = self.monospace;
                self.alphabet = MathAlphabet::Normal;
                self.monospace = false;
                let result = self.font_argument(target);
                self.explicit_greek_italic = previous;
                self.alphabet = previous_alphabet;
                self.monospace = previous_monospace;
                result
            }
            "boldsymbol" | "pmb" => {
                let previous = self.boldsymbol;
                self.boldsymbol = true;
                let result = self.font_argument(Font {
                    bold: true,
                    ..self.font
                });
                self.boldsymbol = previous;
                result
            }
            "mathcal" => self.alphabet_argument(MathAlphabet::Calligraphic, "mathcal"),
            "mathfrak" => self.alphabet_argument(MathAlphabet::Fraktur, "mathfrak"),
            "mathscr" => self.alphabet_argument(MathAlphabet::Script, "mathscr"),
            "mathtt" => {
                let previous = self.monospace;
                let previous_alphabet = self.alphabet;
                self.alphabet = MathAlphabet::Normal;
                self.monospace = true;
                let result = self.font_argument(Font::ROMAN);
                self.monospace = previous;
                self.alphabet = previous_alphabet;
                result
            }
            "mathbb" => self.blackboard(),
            "Omega" => Ok(if self.explicit_greek_italic {
                "Omega"
            } else {
                "OHM"
            }
            .into()),
            "Upsilon" => Ok("varupsilon".into()),
            "degree" => Ok("DEG".into()),
            "maltese" => Ok("\"✠\"".into()),
            "pounds" => Ok("\"£\"".into()),
            "minuso" => Ok("\"⦵\"".into()),
            "mathop" => self.operator(),
            "overset" | "stackrel" | "underset" => Ok(self.annotation(name != "underset")?.lower()),
            "xrightarrow" | "xleftarrow" | "xRightarrow" | "xLeftarrow" | "xleftrightarrow"
            | "xLeftrightarrow" | "xrightleftharpoons" | "xhookrightarrow" | "xhookleftarrow"
            | "xtwoheadrightarrow" | "xtwoheadleftarrow" | "xrightleftarrows"
            | "xrightequilibrium" | "xleftequilibrium" => {
                let previous_style = self.style;
                self.style = self.style.script();
                let below = self.optional_argument()?;
                let above = self.argument()?;
                self.style = previous_style;
                let arrow = match name {
                    "xrightarrow" => "rarrow",
                    "xleftarrow" => "larrow",
                    "xRightarrow" => "RARROW",
                    "xLeftarrow" => "LARROW",
                    "xleftrightarrow" => "lrarrow",
                    "xLeftrightarrow" => "LRARROW",
                    "xrightleftharpoons" => "EXARROW",
                    "xhookrightarrow" => "hookright",
                    "xhookleftarrow" => "hookleft",
                    "xtwoheadrightarrow" => "↠",
                    "xtwoheadleftarrow" => "↞",
                    "xrightleftarrows" | "xrightequilibrium" | "xleftequilibrium" => "EXARROW",
                    _ => unreachable!(),
                };
                Ok(Annotation {
                    base: AnnotationOperand::Arrow(arrow, arrow.into()),
                    above: Some(AboveAnnotation::Text(above)),
                    below,
                }
                .lower())
            }
            "overbrace" | "underbrace" => {
                let body = self.argument()?;
                self.whitespace();
                let expected = if name == "overbrace" { '^' } else { '_' };
                let label = if self.peek() == Some(expected) {
                    self.bump();
                    self.styled_argument(self.style.script())?
                } else {
                    String::new()
                };
                self.whitespace();
                if self.peek() == Some(expected) {
                    return self.fail("duplicate brace annotation");
                }
                Ok(if name == "overbrace" {
                    format!("{{OVERBRACE {{{body}}} {{{label}}}}}")
                } else {
                    format!("{{UNDERBRACE {{{label}}} {{{body}}}}}")
                })
            }
            "substack" => {
                self.whitespace();
                if self.bump() != Some('{') {
                    return self.fail("substack requires a group");
                }
                let previous_scale = self.scale;
                self.scale = 70;
                let mut rows = Vec::new();
                loop {
                    let row = self.styled_sequence(MathStyle::Script, Some('}'), true)?;
                    if row.trim().is_empty() {
                        return self.fail("empty substack row");
                    }
                    self.count_annotation_atom()?;
                    rows.push(row);
                    if self.starts("\\\\") {
                        self.pos += 2;
                    } else if self.at_command("cr") {
                        self.pos += 3;
                    } else if self.at_command("newline") {
                        self.pos += 8;
                    } else if self.source[..self.pos].ends_with('}') {
                        break;
                    } else {
                        return self.fail("invalid substack separator");
                    }
                    self.whitespace();
                    self.discard_row_spacing()?;
                }
                self.scale = previous_scale;
                Ok(format!(
                    "{{scale 70 {{pile {{{}}}}} scale {previous_scale}}}",
                    rows.join(" # ")
                ))
            }
            "operatorname" => {
                self.whitespace();
                let starred = self.peek() == Some('*');
                if starred {
                    self.bump();
                }
                let mut limits = starred && self.style == MathStyle::Display;
                self.whitespace();
                let raw = if self.peek() == Some('{') {
                    self.raw_group()?
                } else {
                    match self.bump() {
                        Some('\\') => {
                            let start = self.pos;
                            while self.peek().is_some_and(|c| c.is_ascii_alphabetic()) {
                                self.bump();
                            }
                            if self.pos == start {
                                return self.fail("missing operator name command");
                            }
                            format!("\\{}", &self.source[start..self.pos])
                        }
                        Some(c)
                            if !c.is_control()
                                && !matches!(c, '}' | '{' | '^' | '_' | '&' | ']' | '$') =>
                        {
                            c.to_string()
                        }
                        _ => return self.fail("missing operator name"),
                    }
                };
                let raw = self.operator_text(&raw)?;
                if raw.trim().is_empty() {
                    self.whitespace();
                    if matches!(self.peek(), Some('^' | '_')) {
                        return self.fail("empty operator name cannot have scripts");
                    }
                    return Ok(String::new());
                }
                let name = self.literal_text(raw, true)?;
                self.whitespace();
                if self.at_command("limits") {
                    self.count_annotation_atom()?;
                    self.pos += "\\limits".len();
                    limits = true;
                } else if self.at_command("nolimits") {
                    self.count_annotation_atom()?;
                    self.pos += "\\nolimits".len();
                    limits = false;
                }
                if starred && !matches!(self.peek(), Some('_' | '^')) {
                    limits = true;
                }
                if limits {
                    Ok(native_limit_operator(format!("underover {name}")))
                } else {
                    Ok(native_plain_operator(name))
                }
            }
            "text" | "textrm" | "textsf" | "mbox" => self.text_group(),
            "textbf" => {
                // Native bold text uses the same roman bold scope as \mathbf.
                self.whitespace();
                if self.bump() != Some('{') {
                    return self.fail("expected braced text argument");
                }
                let previous = std::mem::replace(&mut self.text_italic, false);
                let body = self.text_contents();
                self.text_italic = previous;
                Ok(format!("{{rm bold {{{}}} {}}}", body?, self.font.command()))
            }
            "textit" => {
                self.whitespace();
                if self.bump() != Some('{') {
                    return self.fail("expected braced text argument");
                }
                let previous = std::mem::replace(&mut self.text_italic, true);
                let body = self.text_contents();
                self.text_italic = previous;
                Ok(format!("{{it {{{}}} {}}}", body?, self.font.command()))
            }
            "texttt" => {
                let previous = self.monospace;
                self.monospace = true;
                let result = self.text_group();
                self.monospace = previous;
                result
            }
            "big" | "bigl" | "bigr" | "bigm" | "Big" | "Bigl" | "Bigr" | "Bigm" | "bigg"
            | "biggl" | "biggr" | "biggm" | "Bigg" | "Biggl" | "Biggr" | "Biggm" => {
                let scale = if name.starts_with("Bigg") {
                    300
                } else if name.starts_with("bigg") {
                    240
                } else if name.starts_with("Big") {
                    180
                } else {
                    120
                };
                let delimiter = self.delimiter()?;
                let delimiter = match delimiter.as_str() {
                    "." => return Ok(String::new()),
                    "{" | "}" | "(" | ")" | "[" | "]" | "<" | ">" | "|" | "/" | "∥" | "⌈" | "⌉"
                    | "⌊" | "⌋" => format!("\"{delimiter}\""),
                    _ => delimiter,
                };
                Ok(format!(
                    "{{scale {scale} {delimiter} scale {}}}",
                    self.scale
                ))
            }
            "middle" => {
                if self.delimiter_depth == 0 {
                    return self.fail("middle requires an enclosing left/right pair");
                }
                let delimiter = self.delimiter()?;
                Ok(match delimiter.as_str() {
                    "." => String::new(),
                    "{" | "}" => format!("\"{delimiter}\""),
                    _ => delimiter,
                })
            }
            "left" => {
                let left = self.delimiter()?;
                self.delimiter_depth += 1;
                let body = self.sequence(None, false);
                self.delimiter_depth -= 1;
                let body = body?;
                let colored_tail = self.colored_right_tail.take();
                if !self.at_command("right") {
                    return self.fail("missing matching right delimiter");
                }
                self.pos += "\\right".len();
                let right = self.delimiter()?;
                if left == "}"
                    || right == "{"
                    || (((left == "{") != (right == "}"))
                        && !(left == "." && right == "}")
                        && !(left == "{" && right == "."))
                {
                    return self.fail("reversed or unpaired curly delimiters are unsupported");
                }
                let ending = if let Some((rgb, tail)) = colored_tail {
                    native_color(rgb, format!("{tail} RIGHT {right}"))
                } else {
                    format!("RIGHT {right}")
                };
                Ok(format!("{{LEFT {left} {body} {ending}}}"))
            }
            "Set" | "Braket" => {
                let body = self.argument()?;
                let (left, right) = if name == "Set" {
                    ("{", "}")
                } else {
                    ("<", ">")
                };
                Ok(format!("{{LEFT {left} {body} RIGHT {right}}}"))
            }
            "begin" => {
                let env = self.raw_group()?;
                self.environment(&env)
            }
            "textcolor" => self.color(),
            "blue" | "red" | "green" | "orange" | "pink" | "purple" | "gray" | "black"
            | "white" | "magenta" | "cyan" | "yellow" => {
                let rgb = crate::katex_color_commands::command_color(name)
                    .map(Ok)
                    .unwrap_or_else(|| crate::tex_colors::parse_color("", name))
                    .map_err(|message| TexError {
                        offset: start,
                        message,
                    })?;
                let body = self.argument()?;
                Ok(native_color(rgb, body))
            }
            "colorbox" | "fcolorbox" => {
                self.color_spec()?;
                if name == "fcolorbox" {
                    self.color_spec()?;
                }
                let body = self.argument()?;
                let body = format!("{{rm {{{body}}} {}}}", self.font.command());
                Ok(format!("{{box {{{body}}}}}"))
            }
            "mkern" => self.math_kern(),
            "kern" => self.kern(),
            "hspace" => {
                self.whitespace();
                if self.peek() == Some('*') {
                    self.bump();
                }
                self.kern()
            }
            "!" => Ok(String::new()),
            "," => Ok("`".into()),
            ":" => Ok("``".into()),
            ";" => Ok("```".into()),
            "quad" => Ok("~~".into()),
            "qquad" => Ok("~~~~".into()),
            " " | "enspace" => Ok("~".into()),
            "{" | "}" | "%" | "$" | "#" | "&" | "_" => Ok(format!("\"{name}\"")),
            _ => {
                if let Some(rgb) = crate::katex_color_commands::command_color(name) {
                    let body = self.argument()?;
                    return Ok(native_color(rgb, body));
                }
                if let Some(h) = symbol(name) {
                    let integral =
                        matches!(name, "int" | "oint" | "iint" | "iiint" | "oiint" | "oiiint");
                    let display_limits = matches!(
                        name,
                        "sum"
                            | "prod"
                            | "coprod"
                            | "bigcap"
                            | "bigcup"
                            | "bigsqcap"
                            | "bigsqcup"
                            | "bigoplus"
                            | "bigotimes"
                            | "bigodot"
                            | "bigvee"
                            | "bigwedge"
                            | "biguplus"
                            | "lim"
                            | "limsup"
                            | "liminf"
                            | "max"
                            | "min"
                            | "sup"
                            | "inf"
                            | "det"
                            | "gcd"
                            | "Pr"
                            | "intop"
                    );
                    if integral || display_limits {
                        self.whitespace();
                        let explicit = if self.at_command("nolimits") {
                            self.count_annotation_atom()?;
                            self.pos += "\\nolimits".len();
                            Some(false)
                        } else if self.at_command("limits") {
                            self.count_annotation_atom()?;
                            self.pos += "\\limits".len();
                            Some(true)
                        } else {
                            None
                        };
                        let below =
                            explicit.unwrap_or(display_limits && self.style == MathStyle::Display);
                        let scripted = matches!(self.peek(), Some('_' | '^'));
                        if below {
                            return Ok(
                                if integral || matches!(name, "intop" | "limsup" | "liminf") {
                                    format!("underover {{{h}}}")
                                } else {
                                    h.into()
                                },
                            );
                        }
                        if (scripted && !integral) || explicit == Some(false) {
                            return Ok(format!("{{{h}}}"));
                        }
                    }
                    return Ok(h.into());
                }
                Err(TexError {
                    offset: start,
                    message: format!("unsupported TeX command \\{name}"),
                })
            }
        }
    }
    fn cancellation_body(&self, body: &str) -> Result<String, TexError> {
        // NOT owns a glyph, never a native brace or a quoted text run. Walk
        // through generated scope/font wrappers, keeping them around the slash.
        let mut pos = 0;
        loop {
            while body[pos..].starts_with(char::is_whitespace) {
                pos += body[pos..].chars().next().unwrap().len_utf8();
            }
            if body[pos..].starts_with('{') {
                pos += 1;
                continue;
            }
            let end = body[pos..]
                .find(|c: char| c.is_whitespace() || matches!(c, '{' | '}'))
                .map_or(body.len(), |n| pos + n);
            let token = &body[pos..end];
            if matches!(token, "rm" | "it" | "bold" | "boldrm" | "rmbold") {
                pos = end;
                continue;
            }
            if token == "scale" {
                pos = end;
                while body[pos..].starts_with(char::is_whitespace) {
                    pos += 1;
                }
                while body[pos..].starts_with(|c: char| c.is_ascii_digit()) {
                    pos += 1;
                }
                continue;
            }
            break;
        }
        let Some(first) = body[pos..].chars().next() else {
            return self.fail("cancellation requires nonempty content");
        };
        if first == LITERAL_START || first == '"' {
            let close = if first == LITERAL_START {
                LITERAL_END
            } else {
                '"'
            };
            let start = pos + first.len_utf8();
            let end = body[start..]
                .find(close)
                .map(|n| start + n)
                .ok_or_else(|| TexError {
                    offset: self.pos,
                    message: "invalid generated cancellation literal".into(),
                })?;
            let Some(glyph) = body[start..end].chars().next() else {
                return self.fail("cancellation requires nonempty content");
            };
            if !crate::hml_to_tex::negatable_scalar(&glyph.to_string()) {
                return self.fail("cancellation requires a supported first glyph");
            }
            let tail = &body[start + glyph.len_utf8()..end];
            let tail = if tail.is_empty() {
                String::new()
            } else {
                format!(" {first}{tail}{close}")
            };
            return Ok(format!(
                "{{{}not {glyph}{tail}{}}}",
                &body[..pos],
                &body[end + close.len_utf8()..]
            ));
        }
        let end = body[pos..]
            .find(|c: char| c.is_whitespace() || matches!(c, '{' | '}' | '_' | '^'))
            .map_or(body.len(), |n| pos + n);
        let token = &body[pos..end];
        if let Some(glyph) = crate::hml_to_tex::negated_integral_glyph(token) {
            return Ok(format!("{{{}not {glyph}{}}}", &body[..pos], &body[end..]));
        }
        let ordinary = !token.is_empty()
            && token.bytes().all(|c| c.is_ascii_alphanumeric())
            && !crate::hml_to_tex::literal_needs_quotes(token);
        if !crate::hml_to_tex::negatable_scalar(token) && !ordinary {
            return self.fail("cancellation requires a supported first glyph");
        }
        Ok(format!("{{{}not {}}}", &body[..pos], &body[pos..]))
    }
    fn negation_argument(&mut self) -> Result<String, TexError> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return self.fail("nesting limit exceeded");
        }
        self.whitespace();
        let grouped = self.peek() == Some('{');
        if grouped {
            self.bump();
            self.whitespace();
            if let Some(end) = self.source[self.pos..].find('}') {
                let word = &self.source[self.pos..self.pos + end];
                if word.len() > 1
                    && word.bytes().all(|c| c.is_ascii_alphabetic())
                    && !crate::hml_to_tex::literal_needs_quotes(word)
                {
                    let word = word.to_owned();
                    for _ in word.chars() {
                        self.count_annotation_atom()?;
                    }
                    self.pos += end + 1;
                    self.depth -= 1;
                    return Ok(word);
                }
            }
        }
        // An unbraced TeX macro argument consumes one token. Only a grouped
        // colon pair denotes the complete IDENTICAL glyph under NOT.
        let result = if !grouped && self.peek() == Some(':') {
            self.count_annotation_atom()?;
            self.bump();
            Ok(":".into())
        } else {
            self.negation_symbol()
        };
        self.depth -= 1;
        let operand = result?;
        if grouped {
            self.whitespace();
            if self.bump() != Some('}') {
                return self.fail("not requires a single character or supported scalar symbol");
            }
        }
        Ok(operand)
    }

    fn negation_symbol(&mut self) -> Result<String, TexError> {
        let start = self.pos;
        if self.peek() == Some('{') {
            // Parse the empty-base degree idiom through the normal bounded
            // parser, then require one complete native glyph. Generic groups,
            // scripts on the glyph, and multiatom contents remain unsupported.
            let native = self.scripted_atom()?;
            if matches!(native.as_str(), "DEG" | "CENTIGRADE" | "FAHRENHEIT") {
                return Ok(native);
            }
            return self.fail("not requires a single character or supported scalar symbol");
        }
        if self.peek() == Some('\\') {
            self.bump();
            let name = self.command_name();
            if matches!(
                name.as_str(),
                "mathcal" | "text" | "textrm" | "mbox" | "Omega" | "Upsilon"
            ) {
                self.count_annotation_atom()?;
                let native = self.command(&name, start)?;
                if matches!(
                    native.as_str(),
                    "LAPLACE" | "ANGSTROM" | "att" | "hund" | "thou" | "OHM" | "varupsilon"
                ) {
                    return Ok(native);
                }
                return self.fail("not requires a single character or supported scalar symbol");
            }
            let native = match name.as_str() {
                "le" | "leq" => Some("<="),
                _ => symbol(&name),
            }
            .map(|value| crate::hml_to_tex::negated_integral_glyph(value).unwrap_or(value));
            if let Some(native) = native.filter(|value| crate::hml_to_tex::negatable_scalar(value))
            {
                self.count_annotation_atom()?;
                return Ok(native.into());
            }
            return Err(TexError {
                offset: start,
                message: "not requires a single character or supported scalar symbol".into(),
            });
        }
        if let Some(glyph) = self
            .peek()
            .filter(|c| matches!(c, '∫' | '∬' | '∭' | '∮' | '∯' | '∰'))
        {
            self.count_annotation_atom()?;
            self.bump();
            return Ok(glyph.to_string());
        }
        if let Some(native) = self
            .peek()
            .and_then(unicode_symbol)
            .map(|value| crate::hml_to_tex::negated_integral_glyph(value).unwrap_or(value))
            .filter(|value| crate::hml_to_tex::negatable_scalar(value))
        {
            self.count_annotation_atom()?;
            self.bump();
            return Ok(native.into());
        }
        if self.peek().is_some_and(|c| {
            c.is_ascii_alphanumeric() || is_hangul_letter(c) || "+-=<>.,!?/:;|'".contains(c)
        }) {
            // A TeX unbraced operand is one character, even in a digit run.
            if self.peek().is_some_and(|c| c.is_ascii_alphanumeric()) {
                self.count_annotation_atom()?;
                // NOT consumes this one character directly, before the native
                // parser recognizes following letters as a separate word.
                return Ok(self.bump().expect("character checked").to_string());
            }
            return self.atom();
        }
        self.fail("not requires a single character or supported scalar symbol")
    }

    fn kern(&mut self) -> Result<String, TexError> {
        self.whitespace();
        let raw = if self.peek() == Some('{') {
            self.raw_group()?
        } else {
            let start = self.pos;
            if matches!(self.peek(), Some('+' | '-')) {
                self.bump();
            }
            while self.peek().is_some_and(|c| c.is_ascii_digit() || c == '.') {
                self.bump();
            }
            self.whitespace();
            for _ in 0..2 {
                if self.peek().is_some_and(|c| c.is_ascii_alphabetic()) {
                    self.bump();
                }
            }
            self.source[start..self.pos].to_owned()
        };
        let raw = raw.trim();
        let split = raw
            .find(|c: char| !c.is_ascii_digit() && !matches!(c, '+' | '-' | '.'))
            .unwrap_or(raw.len());
        let (number, unit) = raw.split_at(split);
        let value = number.parse::<f64>().ok().filter(|n| n.is_finite());
        // Approximate TeX lengths at the prototype's nominal 10pt baseline;
        // ex uses half an em, independently of the active native font metrics.
        let mu_per_unit = match unit.trim() {
            "em" => 18.0,
            "ex" => 9.0,
            "pt" => 1.8,
            "pc" => 21.6,
            "in" => 72.27 * 1.8,
            "bp" => 72.27 / 72.0 * 1.8,
            "cm" => 72.27 / 2.54 * 1.8,
            "mm" => 72.27 / 25.4 * 1.8,
            "dd" => 1238.0 / 1157.0 * 1.8,
            "cc" => 12.0 * 1238.0 / 1157.0 * 1.8,
            "sp" => 1.8 / 65536.0,
            _ => {
                return self.fail(
                    "kern requires a decimal TeX dimension (em/ex/pt/pc/in/bp/cm/mm/dd/cc/sp)",
                );
            }
        };
        let Some(value) = value else {
            return self.fail("kern requires a finite decimal dimension");
        };
        let mu = value * mu_per_unit;
        if !mu.is_finite() || mu.abs() > 72.0 {
            return self.fail("kern dimension must be within -72..72mu after conversion");
        }
        // Native SCRIPT has positive spaces only. Negative kern is omitted.
        let quarters = (mu.max(0.0) / 2.25).round() as usize;
        Ok(format!(
            "{}{}",
            "~".repeat(quarters / 4),
            "`".repeat(quarters % 4)
        ))
    }
    fn math_kern(&mut self) -> Result<String, TexError> {
        self.whitespace();
        if self.peek() == Some('+') {
            self.bump();
        }
        let start = self.pos;
        let mut digits = 0;
        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.bump();
            digits += 1;
        }
        if self.peek() == Some('.') {
            self.bump();
            while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                self.bump();
                digits += 1;
            }
        }
        if digits == 0 {
            return self.fail("mkern requires a nonnegative decimal dimension in mu (0..72)");
        }
        let value = self.source[start..self.pos]
            .parse::<f64>()
            .map_err(|_| TexError {
                offset: start,
                message: "invalid mkern decimal dimension".into(),
            })?;
        if !value.is_finite() || !(0.0..=72.0).contains(&value) {
            return self.fail("mkern dimension must be in 0..72mu");
        }
        self.whitespace();
        if !self.starts("mu") {
            return self.fail("mkern requires the mu unit");
        }
        self.pos += 2;
        // A letter after the unit starts the next math atom (for example,
        // \mkern18mub). A digit or decimal point would extend the dimension.
        if self.peek().is_some_and(|c| c.is_ascii_digit() || c == '.') {
            return self.fail("invalid suffix after mkern mu unit");
        }
        // Approximate positive TeX lengths with native quarter-spaces. These
        // are conversion units, not measured font-independent Hancom widths:
        // one native space is estimated as 9mu, its quarter as 2.25mu.
        // Round to the nearest quarter (ties upward); no spacing means no tag.
        let quarters = (value / 2.25).round() as usize;
        Ok(format!(
            "{}{}",
            "~".repeat(quarters / 4),
            "`".repeat(quarters % 4)
        ))
    }
    fn optional_argument(&mut self) -> Result<Option<String>, TexError> {
        self.whitespace();
        if self.peek() != Some('[') {
            return Ok(None);
        }
        self.bump();
        self.sequence(Some(']'), false).map(Some)
    }
    fn discard_row_spacing(&mut self) -> Result<(), TexError> {
        self.whitespace();
        if self.peek() != Some('[') {
            return Ok(());
        }
        self.bump();
        let mut groups = 0;
        while let Some(c) = self.bump() {
            match c {
                '\\' => {
                    self.bump();
                }
                '{' => {
                    groups += 1;
                    if self.depth + groups > MAX_DEPTH {
                        return self.fail("nesting limit exceeded");
                    }
                }
                '}' if groups > 0 => groups -= 1,
                '}' => return self.fail("unmatched group in row spacing"),
                ']' if groups == 0 => return Ok(()),
                _ => {}
            }
        }
        self.fail("unclosed row spacing option")
    }
    fn operator_text(&self, raw: &str) -> Result<String, TexError> {
        let mut out = String::new();
        let mut chars = raw.chars().peekable();
        while let Some(c) = chars.next() {
            if c != '\\' {
                out.push(c);
                continue;
            }
            let mut name = String::new();
            while chars.peek().is_some_and(|c| c.is_ascii_alphabetic()) {
                name.push(chars.next().expect("peeked"));
            }
            let scalar = match name.as_str() {
                "alpha" => 'α',
                "beta" => 'β',
                "gamma" => 'γ',
                "delta" => 'δ',
                "epsilon" => 'ε',
                "zeta" => 'ζ',
                "eta" => 'η',
                "theta" => 'θ',
                "iota" => 'ι',
                "kappa" => 'κ',
                "lambda" => 'λ',
                "mu" => 'μ',
                "nu" => 'ν',
                "xi" => 'ξ',
                "pi" => 'π',
                "rho" => 'ρ',
                "sigma" => 'σ',
                "tau" => 'τ',
                "upsilon" => 'υ',
                "phi" => 'φ',
                "chi" => 'χ',
                "psi" => 'ψ',
                "omega" => 'ω',
                "Gamma" => 'Γ',
                "Delta" => 'Δ',
                "Theta" => 'Θ',
                "Lambda" => 'Λ',
                "Xi" => 'Ξ',
                "Pi" => 'Π',
                "Sigma" => 'Σ',
                "Upsilon" => 'Υ',
                "Phi" => 'Φ',
                "Psi" => 'Ψ',
                "Omega" => 'Ω',
                "varepsilon" => 'ϵ',
                "vartheta" => 'ϑ',
                "varpi" => 'ϖ',
                "varsigma" => 'ς',
                "varphi" => 'ϕ',
                _ => match symbol(&name) {
                    Some(native) if native.chars().count() == 1 => {
                        native.chars().next().expect("one scalar")
                    }
                    _ => {
                        return self.fail(&format!(
                            "unsupported scalar command in operator name: \\{name}"
                        ));
                    }
                },
            };
            out.push(scalar);
        }
        Ok(out)
    }
    fn literal_text(&self, raw: String, math_operator: bool) -> Result<String, TexError> {
        if !math_operator {
            // Only complete scalar text-symbol wrappers are lowered. General
            // text commands/macros remain unsupported and cannot execute here.
            let scalar = match raw.trim() {
                r"\AA" => Some("ANGSTROM"),
                r"\textreferencemark" => Some("att"),
                r"\textperthousand" => Some("hund"),
                r"\textpertenthousand" => Some("thou"),
                _ => None,
            };
            if let Some(scalar) = scalar {
                return Ok(scalar.into());
            }
        }
        if raw.contains('$') {
            return self.fail("nested math inside text is unsupported");
        }
        if raw.contains(['"', '\\']) {
            return self.fail("quotes and TeX commands inside text are unsupported");
        }
        let raw = if math_operator {
            raw
        } else {
            text_punctuation(&raw, self.monospace)
        };
        let target = Font {
            bold: math_operator && self.boldsymbol,
            ..Font::ROMAN
        };
        if self.font == target {
            Ok(format!("{{\"{raw}\"}}"))
        } else {
            Ok(format!(
                "{{{} \"{raw}\" {}}}",
                target.command(),
                self.font.command()
            ))
        }
    }
    fn text_group(&mut self) -> Result<String, TexError> {
        self.whitespace();
        if self.peek() == Some('{')
            && let Some(end) = self.source[self.pos + 1..].find('}')
        {
            let raw = &self.source[self.pos + 1..self.pos + 1 + end];
            if !raw.contains(['{', '\\'])
                || matches!(
                    raw.trim(),
                    r"\AA" | r"\textreferencemark" | r"\textperthousand" | r"\textpertenthousand"
                )
            {
                let raw = self.raw_group()?;
                let raw = if self.monospace {
                    monospace_text(&raw)
                } else {
                    raw
                };
                return self.literal_text(raw, false);
            }
        }
        if self.bump() != Some('{') {
            return self.fail("expected braced text argument");
        }
        let body = self.text_contents()?;
        if self.font == Font::ROMAN {
            Ok(format!("{{{body}}}"))
        } else {
            Ok(format!("{{rm {{{body}}} {}}}", self.font.command()))
        }
    }
    /// Reads a braced text command nested inside text. `italic` and `monospace`
    /// replace the current shape/family for the group (None keeps it); the
    /// previous native shape is restored after the group when it changed.
    fn nested_text(
        &mut self,
        italic: Option<bool>,
        monospace: Option<bool>,
    ) -> Result<String, TexError> {
        self.whitespace();
        if self.bump() != Some('{') {
            return self.fail("expected braced text argument");
        }
        let previous_italic = self.text_italic;
        let previous_monospace = self.monospace;
        let italic = italic.unwrap_or(previous_italic);
        self.text_italic = italic;
        self.monospace = monospace.unwrap_or(previous_monospace);
        let body = self.text_contents();
        self.text_italic = previous_italic;
        self.monospace = previous_monospace;
        let body = body?;
        let shape = |italic: bool| if italic { "it" } else { "rm" };
        Ok(if italic == previous_italic {
            format!("{{{body}}}")
        } else {
            format!(
                "{{{} {{{body}}} {}}}",
                shape(italic),
                shape(previous_italic)
            )
        })
    }
    fn text_contents(&mut self) -> Result<String, TexError> {
        let previous_monospace = self.monospace;
        let previous_italic = self.text_italic;
        let previous_scale = self.scale;
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return self.fail("nesting limit exceeded");
        }
        let mut parts = Vec::new();
        let mut bold_start = None;
        let mut plain = String::new();
        loop {
            match self.peek() {
                Some('}' | '{' | '\\') => {
                    if !plain.is_empty() {
                        let value = std::mem::take(&mut plain);
                        let value = text_punctuation(&value, self.monospace);
                        parts.push(format!(
                            "\"{}\"",
                            if self.monospace {
                                monospace_text(&value)
                            } else {
                                value
                            }
                        ));
                    }
                    match self.bump().unwrap() {
                        '}' => break,
                        '{' => parts.push(format!("{{{}}}", self.text_contents()?)),
                        _ => {
                            let command = self.text_command()?;
                            if matches!(command.as_str(), "bf" | "rm" | "it") {
                                if let Some(start) = bold_start.take() {
                                    let body = parts.drain(start..).collect::<Vec<_>>().join(" ");
                                    parts.push(format!("bold {{{body}}}"));
                                }
                                if command == "bf" {
                                    bold_start = Some(parts.len());
                                } else {
                                    parts.push(command);
                                }
                            } else {
                                parts.push(command);
                            }
                        }
                    }
                }
                Some('$' | '"') => {
                    return self
                        .fail("math delimiters and literal quotes inside text are unsupported");
                }
                Some(c) if c.is_control() => return self.fail("control character in text"),
                Some(c) => {
                    self.count_annotation_atom()?;
                    self.bump();
                    plain.push(c);
                }
                None => return self.fail("unclosed text group"),
            }
        }
        if let Some(start) = bold_start {
            let body = parts.drain(start..).collect::<Vec<_>>().join(" ");
            parts.push(format!("bold {{{body}}}"));
        }
        self.depth -= 1;
        self.monospace = previous_monospace;
        self.text_italic = previous_italic;
        if self.scale != previous_scale {
            parts.push(format!("scale {previous_scale}"));
            self.scale = previous_scale;
        }
        Ok(parts.join(" "))
    }
    fn text_command(&mut self) -> Result<String, TexError> {
        self.count_annotation_atom()?;
        let start = self.pos;
        if self.peek().is_some_and(|c| c.is_ascii_alphabetic()) {
            while self.peek().is_some_and(|c| c.is_ascii_alphabetic()) {
                self.bump();
            }
            self.whitespace();
        } else if self.bump().is_none() {
            return self.fail("missing text command");
        }
        let name = self.source[start..self.pos].trim();
        if let Some(scale) = font_size_scale(name) {
            return Ok(self.declare_scale(scale));
        }
        let native = match name {
            "sout" | "cancel" | "bcancel" | "xcancel" => {
                let body = self.text_group()?;
                return self.cancellation_body(&body);
            }
            // A control space is visible text spacing, not an unknown macro.
            "" => return Ok("~".into()),
            "it" | "rm" | "bf" | "sf" => {
                self.monospace = false;
                if name != "bf" {
                    self.text_italic = name == "it";
                }
                return Ok(if name == "sf" { "rm" } else { name }.into());
            }
            "tt" => {
                self.monospace = true;
                self.text_italic = false;
                return Ok("rm".into());
            }
            // Nested text commands. textsf uses roman, following the existing
            // sans-serif policy; like KaTeX it resets an enclosing italic shape.
            "textit" => return self.nested_text(Some(true), None),
            "textrm" | "textsf" => return self.nested_text(Some(false), Some(false)),
            "text" | "mbox" => return self.nested_text(None, None),
            "textquoteleft" => return Ok("\"‘\"".into()),
            "textquoteright" => return Ok("\"’\"".into()),
            "textquotedblleft" => return Ok("\"“\"".into()),
            "textquotedblright" => return Ok("\"”\"".into()),
            "textendash" => return Ok("\"–\"".into()),
            "textemdash" => return Ok("\"—\"".into()),
            "i" => return Ok("\"ı\"".into()),
            "j" => return Ok("\"ȷ\"".into()),
            "AA" => return Ok("ANGSTROM".into()),
            "textreferencemark" => return Ok("att".into()),
            "textperthousand" => return Ok("hund".into()),
            "textpertenthousand" => return Ok("thou".into()),
            "maltese" => return Ok("\"✠\"".into()),
            "degree" => return Ok("\"°\"".into()),
            "pounds" => return Ok("\"£\"".into()),
            "textdollar" => return Ok("\"$\"".into()),
            "underline" => {
                let body = self.text_group()?;
                return Ok(format!("under {{{body}}}"));
            }
            // Nested bold text uses the same native scope as a text-local \bf.
            "textbf" => {
                self.whitespace();
                if self.bump() != Some('{') {
                    return self.fail("expected braced text argument");
                }
                let body = self.text_contents()?;
                return Ok(format!("bold {{{body}}}"));
            }
            "'" | "acute" => "acute",
            "`" | "grave" => "grave",
            "." | "dot" => "dot",
            "\"" | "ddot" => "ddot",
            "^" | "hat" => "hat",
            "~" | "tilde" => "tilde",
            "v" | "check" => "check",
            "=" | "bar" => "bar",
            "H" => "doubleacute",
            "r" => "ring",
            _ => return self.fail(&format!("unsupported text command \\{name}")),
        };
        self.whitespace();
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return self.fail("nesting limit exceeded");
        }
        let body = match self.bump() {
            Some('{') => self.text_contents()?,
            Some('\\') => self.text_command()?,
            Some(c) if !c.is_control() && !matches!(c, '}' | '$' | '"') => format!("\"{c}\""),
            _ => return self.fail("missing text accent argument"),
        };
        self.depth -= 1;
        Ok(match native {
            // Keep these unavailable accents as adjacent literal marks.
            "doubleacute" => format!("{body} \"˝\""),
            "ring" => format!("{body} \"˚\""),
            _ => format!("{native} {{{body}}}"),
        })
    }
    fn alphabet_argument(
        &mut self,
        alphabet: MathAlphabet,
        name: &str,
    ) -> Result<String, TexError> {
        let previous_alphabet = self.alphabet;
        let previous_font = self.font;
        let previous_monospace = self.monospace;
        let previous_greek = self.explicit_greek_italic;
        self.alphabet = alphabet;
        self.font = Font::ITALIC;
        self.monospace = false;
        self.explicit_greek_italic = false;
        let result = self.argument();
        self.alphabet = previous_alphabet;
        self.font = previous_font;
        self.monospace = previous_monospace;
        self.explicit_greek_italic = previous_greek;
        let body = result?;
        if body.trim().is_empty() {
            return self.fail(&format!("\\{name} requires nonempty content"));
        }
        if previous_font == Font::ITALIC {
            if body == "LAPLACE" {
                Ok(body)
            } else {
                Ok(format!("{{{body}}}"))
            }
        } else {
            Ok(format!("{{it {{{body}}} {}}}", previous_font.command()))
        }
    }
    fn font_argument(&mut self, target: Font) -> Result<String, TexError> {
        let previous = self.font;
        self.font = target;
        let body = self.argument();
        self.font = previous;
        let body = body?;
        if target == previous {
            Ok(format!("{{{body}}}"))
        } else {
            Ok(format!(
                "{{{} {{{body}}} {}}}",
                target.command(),
                previous.command()
            ))
        }
    }
    fn operator(&mut self) -> Result<String, TexError> {
        self.whitespace();
        let grouped = self.peek() == Some('{');
        let body = self.argument()?;
        if body.is_empty() {
            return self.fail("empty mathop body");
        }
        self.whitespace();
        let limits = if self.at_command("limits") {
            self.count_annotation_atom()?;
            self.pos += "\\limits".len();
            true
        } else {
            if self.at_command("nolimits") {
                self.count_annotation_atom()?;
                self.pos += "\\nolimits".len();
            }
            false
        };
        if !grouped || !limits {
            // User policy: without both a grouped body and explicit limits,
            // preserve the contents as ordinary symbols, not a large operator.
            return Ok(format!("{{{body}}}"));
        }
        // A scalar symbol promoted to an operator has a native big-operator
        // counterpart. An explicitly sized composite keeps its full body.
        let body = native_large_operator(&body)
            .map(str::to_owned)
            .unwrap_or_else(|| format!("underover {{{body}}}"));
        Ok(body)
    }
    fn annotation(&mut self, upper: bool) -> Result<AnnotationOperand, TexError> {
        let (above, below) = if upper {
            let label = if self.frown_argument()? {
                AboveAnnotation::Arc
            } else {
                AboveAnnotation::Text(self.styled_argument(self.style.script())?)
            };
            (Some(label), None)
        } else {
            (None, Some(self.styled_argument(self.style.script())?))
        };
        let base = self.annotation_argument()?.ungroup();
        // Opposite axes can share one native operator. Repeated axes must stay
        // nested: overwriting an existing label would change the expression.
        let annotation = match base {
            AnnotationOperand::Annotation(mut inner)
                if (above.is_none() || inner.above.is_none())
                    && (below.is_none() || inner.below.is_none()) =>
            {
                if above.is_some() {
                    inner.above = above;
                }
                if below.is_some() {
                    inner.below = below;
                }
                inner
            }
            base => Box::new(Annotation { base, above, below }),
        };
        if matches!(annotation.above, Some(AboveAnnotation::Arc)) && annotation.below.is_some() {
            return self.fail("arc with a lower annotation is unsupported");
        }
        Ok(AnnotationOperand::Annotation(annotation))
    }
    fn annotation_argument(&mut self) -> Result<AnnotationOperand, TexError> {
        let previous_style = self.style;
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return self.fail("nesting limit exceeded");
        }
        self.whitespace();
        let result = if self.peek() == Some('{') {
            self.bump();
            self.annotation_group().map(|mut parts| {
                if parts.len() == 1 {
                    parts.remove(0)
                } else {
                    AnnotationOperand::Plain(
                        parts
                            .into_iter()
                            .map(AnnotationOperand::lower)
                            .collect::<Vec<_>>()
                            .join(" "),
                    )
                }
            })
        } else if self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.count_annotation_atom()?;
            Ok(AnnotationOperand::Plain(self.bump().unwrap().to_string()))
        } else if self
            .peek()
            .is_none_or(|c| matches!(c, '}' | ']' | '^' | '_' | '&'))
        {
            self.fail("missing argument")
        } else {
            self.annotation_atom()
        };
        self.depth -= 1;
        self.style = previous_style;
        result
    }
    fn annotation_group(&mut self) -> Result<Vec<AnnotationOperand>, TexError> {
        let previous_scale = self.scale;
        let previous_style = self.style;
        if has_infix_fraction(&self.source[self.pos..], Some('}'), false) {
            self.style = previous_style.fraction();
        }
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return self.fail("nesting limit exceeded");
        }
        let mut parts: Vec<AnnotationOperand> = Vec::new();
        let mut atop_numerator = None;
        let mut infix_command = "atop";
        let mut hangul_end = None;
        let mut ascii_literal_end = None;
        let mut double_quotes = 0usize;
        loop {
            self.whitespace();
            if self.peek() == Some('}') {
                self.bump();
                break;
            }
            if self.peek().is_none() {
                return self.fail("unclosed group");
            }
            if self.peek().is_some_and(|c| matches!(c, '^' | '_')) {
                parts.push(AnnotationOperand::Plain(self.script_suffix("{}".into())?));
                hangul_end = None;
                ascii_literal_end = None;
                continue;
            }
            if let Some(command) = self.infix_fraction_command() {
                infix_command =
                    self.consume_infix_fraction(command, &parts, atop_numerator.is_some())?;
                self.style = previous_style.fraction();
                atop_numerator = Some(
                    std::mem::take(&mut parts)
                        .into_iter()
                        .map(AnnotationOperand::lower)
                        .collect::<Vec<_>>()
                        .join(" "),
                );
                hangul_end = None;
                ascii_literal_end = None;
                continue;
            }
            let start = self.pos;
            let atom = if let Some(quote) = self.paired_quote(&mut double_quotes)? {
                AnnotationOperand::Plain(quote)
            } else {
                self.annotation_atom()?
            };
            self.whitespace();
            // Scripts attach to the whole parsed atom. They prevent flattening
            // into its inner annotation or arrow, preserving their own base.
            let atom = if self.peek().is_some_and(|c| matches!(c, '^' | '_')) {
                AnnotationOperand::Plain(self.script_suffix(atom.lower())?)
            } else {
                atom
            };
            let end = match &atom {
                AnnotationOperand::Plain(text) => self.native_hangul_end(start, text),
                _ => None,
            };
            let literal_end = match &atom {
                AnnotationOperand::Plain(text) => self.native_ascii_literal_end(start, text),
                _ => None,
            };
            if end.is_some() && hangul_end == Some(start) {
                let Some(AnnotationOperand::Plain(previous)) = parts.last_mut() else {
                    unreachable!("preceding Hangul atom")
                };
                let AnnotationOperand::Plain(text) = atom else {
                    unreachable!("literal Hangul atom")
                };
                previous.push_str(&text);
            } else if literal_end.is_some() && self.only_whitespace_since(ascii_literal_end, start)
            {
                let Some(AnnotationOperand::Plain(previous)) = parts.last_mut() else {
                    unreachable!("preceding ASCII literal")
                };
                let AnnotationOperand::Plain(text) = atom else {
                    unreachable!("literal ASCII atom")
                };
                previous.pop();
                previous.push_str(&text[1..]);
            } else {
                parts.push(atom);
            }
            hangul_end = end;
            ascii_literal_end = literal_end;
        }
        self.depth -= 1;
        if self.scale != previous_scale {
            parts.push(AnnotationOperand::Plain(format!("scale {previous_scale}")));
            self.scale = previous_scale;
        }
        self.style = previous_style;
        if atop_numerator.is_some() {
            let denominator = parts
                .into_iter()
                .map(AnnotationOperand::lower)
                .collect::<Vec<_>>()
                .join(" ");
            Ok(vec![AnnotationOperand::Plain(self.finish_infix_fraction(
                infix_command,
                atop_numerator,
                denominator,
            )?)])
        } else {
            Ok(parts)
        }
    }
    fn annotation_atom(&mut self) -> Result<AnnotationOperand, TexError> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return self.fail("nesting limit exceeded");
        }
        let result = if self.peek() == Some('{') {
            self.count_annotation_atom()?;
            self.bump();
            self.annotation_group().map(AnnotationOperand::Group)
        } else if self.at_command("overset")
            || self.at_command("stackrel")
            || self.at_command("underset")
        {
            self.count_annotation_atom()?;
            let stackrel = self.at_command("stackrel");
            let upper = self.at_command("overset") || stackrel;
            self.pos += if stackrel {
                "\\stackrel".len()
            } else if upper {
                "\\overset".len()
            } else {
                "\\underset".len()
            };
            self.annotation(upper)
        } else {
            let start = self.pos;
            let text = self.atom()?;
            // Identify only a single source command, never quoted text, plain
            // letters, a composite builtin, or SCRIPT's <= comparison token.
            let arrow = self.source[start..self.pos]
                .strip_prefix('\\')
                .and_then(annotation_arrow);
            Ok(match arrow {
                Some(arrow) => AnnotationOperand::Arrow(arrow, text),
                None => AnnotationOperand::Plain(text),
            })
        };
        self.depth -= 1;
        result
    }
    fn count_annotation_atom(&mut self) -> Result<(), TexError> {
        self.atoms += 1;
        if self.atoms > MAX_ATOMS {
            self.fail("atom limit exceeded")
        } else {
            Ok(())
        }
    }
    fn frown_argument(&mut self) -> Result<bool, TexError> {
        // Recognize the TeX accent structurally, before any SCRIPT is emitted.
        // Only redundant groups may surround it; other uses go through the
        // regular parser, which rejects the unsupported standalone command.
        let start = self.pos;
        self.whitespace();
        let mut groups = 0;
        while self.peek() == Some('{') {
            self.bump();
            groups += 1;
            if self.depth + groups > MAX_DEPTH {
                return self.fail("nesting limit exceeded");
            }
            self.whitespace();
        }
        if !self.at_command("frown") {
            self.pos = start;
            return Ok(false);
        }
        self.pos += "\\frown".len();
        for _ in 0..groups {
            self.whitespace();
            if self.peek() != Some('}') {
                self.pos = start;
                return Ok(false);
            }
            self.bump();
        }
        self.atoms += groups.max(1);
        if self.atoms > MAX_ATOMS {
            return self.fail("atom limit exceeded");
        }
        Ok(true)
    }
    fn environment(&mut self, env: &str) -> Result<String, TexError> {
        self.whitespace();
        if matches!(env, "matrix*" | "pmatrix*") && self.peek() == Some('[') {
            self.bump();
            self.whitespace();
            if !matches!(self.bump(), Some('l' | 'c' | 'r')) {
                return self.fail("starred matrix alignment must be l, c, or r");
            }
            self.whitespace();
            if self.bump() != Some(']') {
                return self.fail("starred matrix alignment must be l, c, or r");
            }
            self.whitespace();
        }
        if env == "aligned"
            && ["[t]", "[b]", "[c]"]
                .iter()
                .any(|position| self.starts(position))
        {
            return self.fail("optional aligned vertical positioning is unsupported");
        }
        let h = match env {
            "array" => return self.array_environment(),
            "smallmatrix" => "matrix",
            "subarray" => match self.raw_group()?.trim() {
                "c" => "pile",
                "l" => "lpile",
                _ => return self.fail("subarray requires one c or l alignment column"),
            },
            "matrix" | "matrix*" => "matrix",
            "rcases" => "matrix",
            "pmatrix" | "pmatrix*" => "pmatrix",
            "bmatrix" => "bmatrix",
            "vmatrix" => "dmatrix",
            "cases" => "cases",
            "aligned" | "align" | "align*" | "alignedat" | "alignat" | "alignat*" | "split" => {
                "eqalign"
            }
            "gathered" | "gather" | "gather*" | "equation" | "equation*" => "pile",
            _ => return self.fail(&format!("unsupported environment {env}")),
        };
        if matches!(env, "alignedat" | "alignat" | "alignat*") {
            let count = self.raw_group()?;
            if !count.bytes().all(|c| c.is_ascii_digit())
                || count
                    .parse::<usize>()
                    .ok()
                    .filter(|n| *n > 0 && *n <= MAX_ATOMS / 2)
                    .is_none()
            {
                return self.fail("alignedat requires a positive bounded column-pair count");
            }
        }
        let previous_scale = self.scale;
        let small = matches!(env, "smallmatrix" | "subarray");
        let previous_style = self.style;
        let cell_style = if small {
            MathStyle::Script
        } else if matches!(h, "eqalign" | "pile") && !matches!(env, "subarray") {
            MathStyle::Display
        } else {
            MathStyle::Text
        };
        if small {
            self.scale = 70;
        }
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return self.fail("nesting limit exceeded");
        }
        let mut rows = Vec::new();
        let mut row = Vec::new();
        let mut columns = 1;
        let mut expected_columns = None;
        let pad_rows = h == "eqalign"
            || matches!(
                env,
                "matrix"
                    | "matrix*"
                    | "pmatrix"
                    | "pmatrix*"
                    | "bmatrix"
                    | "vmatrix"
                    | "smallmatrix"
            );
        loop {
            row.push(self.styled_sequence(cell_style, None, true)?);
            self.whitespace();
            if self.peek() == Some('&') {
                if matches!(
                    env,
                    "gathered" | "gather" | "gather*" | "equation" | "equation*" | "subarray"
                ) {
                    return self.fail(&format!("{env} does not accept alignment columns"));
                }
                self.bump();
                columns += 1;
                continue;
            }
            if self.starts("\\\\") || self.at_command("cr") || self.at_command("newline") {
                self.pos += if self.at_command("newline") {
                    8
                } else if self.at_command("cr") {
                    3
                } else {
                    2
                };
                self.whitespace();
                self.discard_row_spacing()?;
                if !pad_rows && expected_columns.is_some_and(|n| n != columns) {
                    return self.fail("nonrectangular environment");
                }
                expected_columns = Some(columns);
                columns = 1;
                rows.push(std::mem::take(&mut row));
                continue;
            }
            if self.at_command("end") {
                self.pos += 4;
                if self.raw_group()? != env {
                    return self.fail("mismatched environment end");
                }
                // A final row separator terminates the preceding row.
                if row.iter().any(|cell| !cell.trim().is_empty()) || rows.is_empty() {
                    if !pad_rows && expected_columns.is_some_and(|n| n != columns) {
                        return self.fail("nonrectangular environment");
                    }
                    rows.push(row);
                }
                break;
            }
            return self.fail("missing environment end or invalid alignment separator");
        }
        self.depth -= 1;
        if pad_rows {
            let width = rows.iter().map(Vec::len).max().unwrap_or(0);
            if width.checked_mul(rows.len()).is_none_or(|n| n > MAX_ATOMS) {
                return self.fail("alignment cell limit exceeded");
            }
            for row in &mut rows {
                row.resize(width, String::new());
            }
        }
        let body = rows
            .into_iter()
            .map(|row| row.join(" & "))
            .collect::<Vec<_>>()
            .join(" # ");
        self.scale = previous_scale;
        self.style = previous_style;
        if env == "rcases" {
            Ok(format!("{{LEFT . matrix {{{body}}} RIGHT }}}}"))
        } else if small {
            Ok(format!(
                "{{scale 70 {{{h} {{{body}}}}} scale {previous_scale}}}"
            ))
        } else {
            Ok(format!("{{{h} {{{body}}}}}"))
        }
    }
    // Accept l/c/r columns and ignore vertical rule markers. Column insertion,
    // width and repetition programs remain unsupported.
    fn array_environment(&mut self) -> Result<String, TexError> {
        let preamble = self.raw_group()?;
        let align: Vec<char> = preamble
            .chars()
            .filter(|c| !c.is_whitespace() && !matches!(c, '|' | ':'))
            .collect();
        if align.is_empty() || align.iter().any(|c| !matches!(c, 'l' | 'c' | 'r')) {
            return self
                .fail("array requires l/c/r columns; only | and : rule markers may be ignored");
        }
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return self.fail("nesting limit exceeded");
        }
        let mut rows = Vec::new();
        let mut row = Vec::new();
        loop {
            row.push(self.styled_sequence(MathStyle::Text, None, true)?);
            self.whitespace();
            if self.peek() == Some('&') {
                self.bump();
                if row.len() >= align.len() {
                    return self.fail("nonrectangular array");
                }
                continue;
            }
            // TeX leaves omitted trailing alignment entries empty.
            row.resize(align.len(), String::new());
            let empty = row.iter().all(|cell| cell.trim().is_empty());
            if empty && (!self.at_command("end") || rows.is_empty()) {
                return self.fail("empty array row");
            }
            if !empty {
                rows.push(std::mem::take(&mut row));
            }
            if self.starts("\\\\") || self.at_command("cr") || self.at_command("newline") {
                self.pos += if self.at_command("newline") {
                    8
                } else if self.at_command("cr") {
                    3
                } else {
                    2
                };
                self.whitespace();
                self.discard_row_spacing()?;
                continue;
            }
            if self.at_command("end") {
                self.pos += 4;
                if self.raw_group()? != "array" {
                    return self.fail("mismatched environment end");
                }
                break;
            }
            return self.fail("missing environment end or invalid alignment separator");
        }
        self.depth -= 1;
        if align.len() == 1 && align[0] != 'c' {
            let h = if align[0] == 'l' { "lpile" } else { "rpile" };
            let body = rows
                .into_iter()
                .map(|row| row.into_iter().next().expect("one column"))
                .collect::<Vec<_>>()
                .join(" # ");
            return Ok(format!("{{{h} {{{body}}}}}"));
        }
        // Native column piles determine row heights independently. Author
        // complete rows to keep cells on a shared baseline, ignoring the
        // multi-column array's individual horizontal alignment declarations.
        let body = rows
            .into_iter()
            .map(|row| row.join(" & "))
            .collect::<Vec<_>>()
            .join(" # ");
        Ok(format!("{{matrix {{{body}}}}}"))
    }
    fn color(&mut self) -> Result<String, TexError> {
        let rgb = self.color_spec()?;
        let body = self.argument()?;
        Ok(native_color(rgb, body))
    }
    fn color_spec(&mut self) -> Result<[u8; 3], TexError> {
        self.whitespace();
        let model = if self.peek() == Some('[') {
            self.bump();
            let start = self.pos;
            while self.peek().is_some_and(|c| c != ']') {
                self.bump();
            }
            if self.peek().is_none() {
                return self.fail("unclosed color model");
            }
            let model = self.source[start..self.pos].to_owned();
            self.bump();
            model
        } else {
            String::new()
        };
        let value = self.raw_group()?;
        crate::tex_colors::parse_color(&model, &value).map_err(|message| TexError {
            offset: self.pos,
            message,
        })
    }
}

fn native_color(rgb: [u8; 3], body: String) -> String {
    format!(
        "{{color {{{}, {}, {}}} {{{body}}}}}",
        rgb[0], rgb[1], rgb[2]
    )
}

// KaTeX Options sizeMultipliers and sizing command indices, 10pt baseline.
fn font_size_scale(command: &str) -> Option<u16> {
    Some(match command {
        "tiny" => 50,
        "scriptsize" => 70,
        "footnotesize" => 80,
        "small" => 90,
        "normalsize" => 100,
        "large" => 120,
        "Large" => 144,
        "LARGE" => 173,
        "huge" => 207,
        "Huge" => 249,
        _ => return None,
    })
}

fn monospace_text(value: &str) -> String {
    value
        .chars()
        .map(|c| match c {
            'A'..='Z' => char::from_u32(0x1d670 + c as u32 - 'A' as u32).unwrap(),
            'a'..='z' => char::from_u32(0x1d68a + c as u32 - 'a' as u32).unwrap(),
            '0'..='9' => char::from_u32(0x1d7f6 + c as u32 - '0' as u32).unwrap(),
            _ => c,
        })
        .collect()
}

// Quote pairs become one typographic quote in every text font. Typewriter
// dashes remain separate. Ligatures stop at group/font-command boundaries.
fn text_punctuation(value: &str, monospace: bool) -> String {
    let mut result = String::with_capacity(value.len());
    let mut rest = value;
    while !rest.is_empty() {
        let ligature = [("---", '—'), ("--", '–'), ("``", '“'), ("''", '”')]
            .into_iter()
            .find(|(prefix, _)| {
                rest.starts_with(prefix) && (!monospace || !prefix.starts_with('-'))
            });
        if let Some((prefix, glyph)) = ligature {
            result.push(glyph);
            rest = &rest[prefix.len()..];
        } else {
            let ch = rest.chars().next().expect("nonempty text");
            result.push(match ch {
                '`' => '‘',
                '\'' => '’',
                _ => ch,
            });
            rest = &rest[ch.len_utf8()..];
        }
    }
    result
}

fn annotation_arrow(command: &str) -> Option<&'static str> {
    Some(match command {
        "Longleftrightarrow" | "Leftrightarrow" => "LRARROW",
        "longleftrightarrow" | "leftrightarrow" => "lrarrow",
        "Longrightarrow" | "Rightarrow" => "RARROW",
        "Longleftarrow" | "Leftarrow" => "LARROW",
        "to" | "rightarrow" | "longrightarrow" => "rarrow",
        "leftarrow" | "longleftarrow" => "larrow",
        "rightleftarrows" => "EXARROW",
        "hookrightarrow" => "hookright",
        "hookleftarrow" => "hookleft",
        "twoheadrightarrow" => "↠",
        "twoheadleftarrow" => "↞",
        _ => return None,
    })
}

fn native_plain_operator(generated: String) -> String {
    // Normalize only the complete default-font, non-limit spelling of the
    // known native keywords. Other casing, literal text, starred operators
    // and styled scopes retain their independently generated restoration.
    let name = generated
        .strip_prefix("{rm \"")
        .and_then(|text| text.strip_suffix("\" it}"));
    if let Some(name @ ("cosec" | "Exp" | "arc" | "mod" | "if" | "for" | "and")) = name {
        name.into()
    } else {
        generated
    }
}

fn native_limit_operator(generated: String) -> String {
    // First lower every starred operator name through the same upright-name
    // rule. Recognize the native Lim spelling only for this complete form.
    // This preserves Lim -> operatorname*{Lim} -> Lim while other names remain
    // generic. Do not rewrite literal text or discard a different font restore
    // (e.g. `bold`) from an enclosing style scope.
    if generated == r#"underover {rm "Lim" it}"# {
        "Lim".into()
    } else {
        generated
    }
}

fn symbol(name: &str) -> Option<&str> {
    Some(match name {
        "fallingdotseq" => "image",
        "risingdotseq" => "REIMAGE",
        "Bumpeq" => "ISO",
        "dotplus" => "DSUM",
        "veebar" => "XOR",
        "dashv" => "DASHV",
        "backslash" => "RSLANT",
        "diagup" => "LSLANT",
        "diagdown" => "RSLANT",
        "square" | "Box" => "□",
        "vartheta" | "varpi" | "varsigma" | "varphi" | "varepsilon" => name,
        "aleph" => "ALEPH",
        "hbar" => "HBAR",
        "imath" => "IMATH",
        "jmath" => "JMATH",
        "ell" => "ELL",
        "wp" => "WP",
        "Im" => "IMAG",
        "sqcup" => "SQCUP",
        "oplus" => "OPLUS",
        "otimes" => "OTIMES",
        "odot" => "ODOT",
        "ni" | "owns" => "NI",
        "sqsubset" => "SQSUBSET",
        "sqsupset" => "SQSUPSET",
        "sqsubseteq" => "SQSUBSETEQ",
        "sqsupseteq" => "SQSUPSETEQ",
        "prec" => "PREC",
        "succ" => "SUCC",
        "uplus" => "UPLUS",
        "circ" => "CIRC",
        "ast" => "AST",
        "star" | "bigstar" => "STAR",
        "bigcirc" => "BIGCIRC",
        "therefore" => "THEREFORE",
        "because" => "BECAUSE",
        "dblcolon" => "IDENTICAL",
        "colon" => ":",
        "doteq" => "DOTEQ",
        "simeq" => "SIMEQ",
        "cong" => "CONG",
        "asymp" => "ASYMP",
        "diamond" => "DIAMOND",
        "prime" => "prime",
        "lnot" | "neg" => "LNOT",
        "propto" => "PROPTO",
        "dagger" => "DAGGER",
        "ddagger" => "DDAGGER",
        "uparrow" => "uparrow",
        "downarrow" => "downarrow",
        "Uparrow" => "UPARROW",
        "Downarrow" => "DOWNARROW",
        "updownarrow" => "udarrow",
        "Updownarrow" => "UDARROW",
        "nwarrow" => "nwarrow",
        "searrow" => "searrow",
        "nearrow" => "nearrow",
        "swarrow" => "swarrow",
        "hookleftarrow" => "hookleft",
        "hookrightarrow" => "hookright",
        "twoheadrightarrow" => "↠",
        "twoheadleftarrow" => "↞",
        "mapsto" => "mapsto",
        "mapsfrom" => "\"↤\"",
        "triangle" => "TRIANGLE",
        "triangledown" => "TRIANGLED",
        "angle" => "ANGLE",
        "rightangle" => "RTANGLE",
        "viewdata" => "well",
        "house" => "base",
        "measuredangle" => "MSANGLE",
        "sphericalangle" => "SANGLE",
        "vdash" => "VDASH",
        "bot" => "BOT",
        "top" => "TOP",
        "models" => "MODELS",
        "alpha" | "beta" | "gamma" | "delta" | "epsilon" | "zeta" | "eta" | "theta" | "iota"
        | "kappa" | "lambda" | "mu" | "nu" | "xi" | "pi" | "rho" | "sigma" | "tau" | "upsilon"
        | "phi" | "chi" | "psi" | "omega" | "Gamma" | "Delta" | "Theta" | "Lambda" | "Xi"
        | "Pi" | "Sigma" | "Upsilon" | "Phi" | "Psi" | "Omega" | "sin" | "cos" | "tan" | "cot"
        | "sec" | "csc" | "sinh" | "cosh" | "tanh" | "log" | "ln" | "exp" | "lim" | "max"
        | "min" | "det" | "gcd" | "coth" | "lg" | "arcsin" | "arccos" | "arctan" | "deg"
        | "arg" | "dim" | "hom" | "ker" | "Pr" | "sum" | "prod" | "int" | "oint" => name,
        "intop" => "int",
        "limsup" => r#"{rm "lim sup" it}"#,
        "liminf" => r#"{rm "lim inf" it}"#,
        "sup" => r#"{rm "sup" it}"#,
        "inf" => r#"{rm "inf" it}"#,
        "infty" => "INFTY",
        "times" => "TIMES",
        "div" => "DIVIDE",
        "pm" => "+-",
        "mp" => "-+",
        "cdot" => "CDOT",
        "sqcap" => "SQCAP",
        "ominus" => "OMINUS",
        "oslash" => "ODIV",
        "bullet" => "BULLET",
        "cdots" => "CDOTS",
        "ldots" | "dots" => "LDOTS",
        "vdots" => "VDOTS",
        "ddots" => "DDOTS",
        "iddots" => "SDOTS",
        "mho" => "MHO",
        "triangleleft" => "TRIANGLEL",
        "triangleright" => "TRIANGLER",
        "nni" => "NOWNS",
        "nsubset" => "NSUBSET",
        "nsubseteq" => "NSUBSETEQ",
        "nsupset" => "NSUPSET",
        "nsupseteq" => "NSUPSETEQ",
        "cap" => "SMALLINTER",
        "cup" => "CUP",
        "bigcap" => "inter",
        "bigcup" => "union",
        "coprod" => "coprod",
        "bigsqcup" => "BIGSQCUP",
        "bigoplus" => "BIGOPLUS",
        "bigotimes" => "BIGOTIMES",
        "bigodot" => "BIGODOT",
        "biguplus" => "BIGUPLUS",
        "bigvee" => "BIGVEE",
        "bigwedge" => "BIGWEDGE",
        "iint" => "dint",
        "iiint" => "tint",
        "oiint" => "odint",
        "oiiint" => "otint",
        "Longleftrightarrow" => "LRARROW",
        "longleftrightarrow" => "lrarrow",
        "Longrightarrow" => "RARROW",
        "Longleftarrow" => "LARROW",
        "rightleftarrows" => "EXARROW",
        "lceil" => "LCEIL",
        "rceil" => "RCEIL",
        "lfloor" => "LFLOOR",
        "rfloor" => "RFLOOR",
        "le" | "leq" => "LEQ",
        "ge" | "geq" => "GEQ",
        "ne" | "neq" => "NEQ",
        "approx" => "APPROX",
        "equiv" => "EQUIV",
        "sim" => "SIM",
        "wedge" => "WEDGE",
        "lor" => "LOR",
        "lll" => "<<<",
        "vert" | "lvert" | "rvert" => "vert",
        "Vert" | "lVert" | "rVert" => "VERT",
        "ll" => "<<",
        "gg" => ">>",
        "ggg" => "GGG",
        "to" | "rightarrow" | "longrightarrow" => "->",
        "leftarrow" | "longleftarrow" => "<-",
        "leftrightarrow" => "<->",
        "Rightarrow" => "RARROW",
        "Leftarrow" => "LARROW",
        "Leftrightarrow" => "LRARROW",
        "in" => "IN",
        "notin" => "notin",
        "subset" => "SUBSET",
        "supset" => "SUPSET",
        "subseteq" => "SUBSETEQ",
        "supseteq" => "SUPSETEQ",
        "emptyset" => "EMPTYSET",
        "partial" => "PARTIAL",
        "nabla" => "nabla",
        "forall" => "FORALL",
        "exists" => "EXIST",
        _ => return None,
    })
}

// Known Unicode scalar aliases resolve to their native keyword identities.
// Do not reinterpret ordinary Latin letters or explicit TeX font declarations.
fn unicode_symbol(value: char) -> Option<&'static str> {
    Some(match value {
        '↤' => "\"↤\"",
        '↦' => "mapsto",
        '↠' => "↠",
        '↞' => "↞",
        '⇄' => "⇄",
        '🣒' | '🣓' => "EXARROW",
        'Α' => "Alpha",
        'Β' => "Beta",
        'Ε' => "Epsilon",
        'Ζ' => "Zeta",
        'Η' => "Eta",
        'Ι' => "Iota",
        'Κ' => "Kappa",
        'Μ' => "Mu",
        'Ν' => "Nu",
        'Ο' => "Omicron",
        'Ρ' => "Rho",
        'Τ' => "Tau",
        'Υ' => "Upsilon",
        'Χ' => "Chi",
        'ϒ' => "varupsilon",
        'Å' => "ANGSTROM",
        'Ω' => "OHM",
        '∷' => "IDENTICAL",
        '⊾' | '∟' => "RTANGLE",
        '°' => "DEG",
        'ℒ' => "LAPLACE",
        '℃' => "CENTIGRADE",
        '℉' => "FAHRENHEIT",
        '※' => "att",
        '‰' => "hund",
        '‱' => "thou",
        '⌗' => "well",
        '⌂' => "base",
        '⌬' => "benzene",
        '℧' => "MHO",
        '◁' => "TRIANGLEL",
        '▷' => "TRIANGLER",
        '⋰' => "SDOTS",
        '⋯' => "CDOTS",
        '…' => "LDOTS",
        '⋮' => "VDOTS",
        '⋱' => "DDOTS",
        '⋙' => "GGG",
        '⋁' => "BIGVEE",
        '⋀' => "BIGWEDGE",
        '∪' => "CUP",
        '∇' => "nabla",
        '⊢' => "VDASH",
        '⊣' => "DASHV",
        '⊤' => "TOP",
        '⊥' => "BOT",
        '∞' => "INFTY",
        '∧' => "WEDGE",
        '¬' => "LNOT",
        '∋' => "NI",
        '∉' => "notin",
        '∌' => "NOWNS",
        '⊄' => "NSUBSET",
        '⊈' => "NSUBSETEQ",
        '⊅' => "NSUPSET",
        '⊉' => "NSUPSETEQ",
        '∥' => "VERT",
        '∂' => "PARTIAL",
        _ => return None,
    })
}
