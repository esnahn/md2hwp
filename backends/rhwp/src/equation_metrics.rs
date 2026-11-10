//! Hancom-like equation geometry without Hancom.
//!
//! Hancom keeps a stored equation box as given and lays the surrounding line
//! out from it, so generated equations need Hancom's own width, height and
//! baseline. Hancom 2020 lays equations out on a 96-dpi pixel grid
//! (1 px = 75 HWPUNIT). Line height uses h = round(base/75) px; glyph advances
//! depend on h and the 48-dpi size s = round-half-down(base/150). Glyph widths
//! come from tables measured with Hancom (`data/hancom-equation-glyphs.json`);
//! structure rules were fitted to `data/hancom-equation-geometry.json` and
//! checked against out-of-sample toolbar equations (`data/hancom-equation-toolbar.json`).
//! Glyphs missing from the table get a typical-advance estimate and the result
//! is reported as approximate; no script is rejected. SCRIPT grammar is not
//! validated here: the rhwp parser reads whatever it can.

use std::collections::HashMap;
use std::sync::OnceLock;

use rhwp::renderer::equation::ast::{EqNode, MatrixStyle, SpaceKind};
use rhwp::renderer::equation::symbols::{DecoKind, FontStyleKind};
use rhwp::renderer::equation::{parser::EqParser, tokenizer::tokenize};

const GLYPHS_JSON: &str = include_str!("../data/hancom-equation-glyphs.json");
const SCRIPTS_JSON: &str = include_str!("../data/hancom-equation-scripts.json");
const PX: f64 = 75.0;

/// Stored geometry for one equation control.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EquationGeometry {
    pub width: u32,
    pub height: u32,
    /// Baseline position as a percentage of the height (HWP EQEDIT baseline).
    pub baseline: i16,
    /// False when any part used the fallback estimate.
    pub fitted: bool,
}

pub(crate) fn measure(script: &str, base_unit: u32) -> EquationGeometry {
    let ast = EqParser::new(tokens(script)).parse();
    let mut layout = Layout { fitted: true };
    let b = layout.node(&ast, f64::from(base_unit.max(1)), Style::Italic);
    let height = b.height().max(1.0);
    EquationGeometry {
        width: b.width.round().max(1.0) as u32,
        height: height.round() as u32,
        baseline: (b.ascent / height * 100.0).round().clamp(0.0, 100.0) as i16,
        fitted: layout.fitted,
    }
}

/// Tokenizes, keeping brace commands as plain text so the row layout can
/// bind their body and label (the parser reads them as decorations).
fn tokens(script: &str) -> Vec<rhwp::renderer::equation::tokenizer::Token> {
    use rhwp::renderer::equation::tokenizer::TokenType;
    let mut tokens = tokenize(script);
    // rhwp reads `LEFT <` as an empty delimiter followed by a relation.
    for i in 1..tokens.len() {
        let after_delimiter = tokens[i - 1].ty == TokenType::Command
            && (tokens[i - 1].value.eq_ignore_ascii_case("left")
                || tokens[i - 1].value.eq_ignore_ascii_case("right"));
        if after_delimiter
            && tokens[i].ty == TokenType::Symbol
            && (tokens[i].value == "<" || tokens[i].value == ">")
        {
            tokens[i].value = if tokens[i].value == "<" {
                "LANGLE"
            } else {
                "RANGLE"
            }
            .into();
            tokens[i].ty = TokenType::Command;
        }
    }
    for t in &mut tokens {
        if t.ty != TokenType::Command {
            continue;
        }
        if t.value == OVERBRACE || t.value == UNDERBRACE {
            t.ty = TokenType::Text;
        } else if t.value.eq_ignore_ascii_case("inter") || t.value.eq_ignore_ascii_case("union") {
            // Hancom draws INTER/UNION as big operators; rhwp reads small ∩/∪.
            t.value = format!(
                "{BIG_MARK}{}",
                if t.value.eq_ignore_ascii_case("inter") {
                    "∩"
                } else {
                    "∪"
                }
            );
            t.ty = TokenType::Text;
        }
    }
    tokens
}

const OVERBRACE: &str = "OVERBRACE";
const UNDERBRACE: &str = "UNDERBRACE";
const BIG_MARK: char = '\u{E000}';

// Fitted constants: fractions of the base unit or of the line height h*75.
const TEXT_ASCENT: f64 = 0.855; // text baseline within its line
const EXPLICIT_STYLE_HEIGHT: f64 = 1.0; // rm/it/spaces use the exact base unit
const SCRIPT_SCALE: f64 = 0.65; // sub/superscript and limit size
const BINARY_SPACE: f64 = 0.287; // total around + - × ≈ (of line)
const RELATION_SPACE: f64 = 0.431; // total around = < > ≤ ≥ ≠ :
const COMMA_SPACE: f64 = 0.072; // after a comma
const OPEN_SPACE: f64 = 0.07; // each plain ( or [ (of base)
const FRACTION_GAP: f64 = 0.3; // between numerator and denominator (of line)
const FRACTION_EXTRA: f64 = 0.5; // added width (of base)
const AXIS: f64 = 0.36; // baseline below a centered fraction rule (of base)
const SQRT_EXTRA: f64 = 0.8; // radical sign width (of line)
const SQRT_TOP: f64 = 0.15; // space above the body (of line)
const ROOT_INDEX_FREE: f64 = 0.45; // index width hidden by the sign (of line)
const NORMAL_SPACE: f64 = 0.5;
const THIN_SPACE: f64 = 0.125;
const BIG_OP_WIDTH: f64 = 1.3; // ∑ ∏ (of line)
const BIG_CAP_WIDTH: f64 = 1.17; // INTER/UNION
const BIG_OTHER_WIDTH: f64 = 1.8; // BIGVEE, BIGOPLUS, BIGSQCAP, ...
const BIG_OP_HEIGHT: f64 = 1.35;
const BIG_OP_ASCENT: f64 = 0.76;
const INTEGRAL_WIDTH: f64 = 1.12;
const INTEGRAL_DOUBLE: f64 = 0.44; // added for ∬ ∯ (of line)
const INTEGRAL_TRIPLE: f64 = 0.845; // added for ∭ ∰
const INTEGRAL_HEIGHT: f64 = 1.887; // of the size rounded to INTEGRAL_QUANTUM
const INTEGRAL_QUANTUM: f64 = 37.5; // half a pixel
const INTEGRAL_SUB_TUCK: f64 = 0.4; // lone lower limit width hidden under the sign (of line)
const BIG_OP_QUANTUM: f64 = 75.0 / 1.8; // ∑ heights step by 0.75 px
const INTEGRAL_ASCENT: f64 = 0.66;
const LIMIT_GAP: f64 = 0.05; // between operator and its limit (of base)
const LIM_HEIGHT: f64 = 1.2; // lim line (of line)
const LIM_ASCENT: f64 = 0.79;
const SUP_DEPTH_LIFT: f64 = 1.3; // extra raise per unit of tall superscript depth
const INTEGRAL_LIMIT: f64 = 0.34; // integral limit extension beyond a script line (of line)
const DECORATION_TOP: f64 = 0.2; // space for an accent (of line)
const DECORATION_WIDE: f64 = 0.096; // extra width of bar/vec/overline (of line)
const OVERBRACE_HEIGHT: f64 = 0.22; // brace above the body (of line)
const UNDERBRACE_HEIGHT: f64 = 0.31; // brace below the body (of line)
const BRACE_WIDTH: f64 = 1.5; // brace width beyond body/label (of line)
const BRACE_WIDTH_FIXED: f64 = 1162.0; // plus this many HWPUNIT
const BOX_PAD_X: f64 = 0.105; // box width padding (of line)
const BOX_PAD_Y: f64 = 0.1; // box padding above and below (of line)
const ANGLE_WIDTH: f64 = 4.0 / 3.0; // LEFT < / RIGHT > (of the plain delimiter)
const ROW_GAP: f64 = 0.15; // matrix/pile/cases rows (of line)
const COLUMN_GAP: f64 = 0.2; // matrix columns and & tabs (of base)
const ATOP_GAP: f64 = 0.1; // atop and eqalign rows (of line)
const EQALIGN_GAP: f64 = 0.35; // between eqalign columns (of line)

#[derive(Clone, Copy, Debug, Default)]
struct Box_ {
    width: f64,
    ascent: f64,
    descent: f64,
}

impl Box_ {
    fn height(&self) -> f64 {
        self.ascent + self.descent
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Style {
    Italic,
    Roman,
    Bold,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
enum Key {
    Letter(Style, char),
    Upright(String),
}

struct Glyphs {
    /// Widths at base = h*75 for h 4..48.
    grid: HashMap<Key, Vec<u32>>,
    /// Widths at odd h 5..47 with the larger 48-dpi size (base = h*75+25).
    odd_upper: HashMap<Key, Vec<u32>>,
}

fn key_for(script: &str) -> Option<Key> {
    match EqParser::new(tokenize(script)).parse() {
        EqNode::Text(t) if t.chars().count() == 1 => {
            Some(Key::Letter(Style::Italic, t.chars().next()?))
        }
        EqNode::FontStyle { style, body } => match (*body, style) {
            (EqNode::Text(t), FontStyleKind::Roman) if t.chars().count() == 1 => {
                Some(Key::Letter(Style::Roman, t.chars().next()?))
            }
            (EqNode::Text(t), FontStyleKind::Bold) if t.chars().count() == 1 => {
                Some(Key::Letter(Style::Bold, t.chars().next()?))
            }
            _ => None,
        },
        EqNode::Number(t) | EqNode::Symbol(t) | EqNode::MathSymbol(t) => Some(Key::Upright(t)),
        _ => None,
    }
}

/// Height a one-line superscript adds above, and a subscript below, a text
/// line of this size, measured with Hancom in 25-unit steps. Pixel rounding
/// makes it irregular, so it is a table rather than a fraction of the size.
fn script_extents(base: f64) -> (f64, f64) {
    static TABLE: OnceLock<(Vec<f64>, Vec<f64>, Vec<f64>)> = OnceLock::new();
    let (bases, above, below) = TABLE.get_or_init(|| {
        let value: serde_json::Value =
            serde_json::from_str(SCRIPTS_JSON).expect("embedded script table is valid JSON");
        let read = |name: &str| -> Vec<f64> {
            value[name]
                .as_array()
                .expect("script table column")
                .iter()
                .map(|v| v.as_f64().expect("script table value"))
                .collect()
        };
        (
            read("base_units"),
            read("superscript_top"),
            read("subscript_bottom"),
        )
    });
    // Nearest measured size; outside the table, scale the end value.
    let index = bases
        .iter()
        .enumerate()
        .min_by(|a, b| (a.1 - base).abs().total_cmp(&(b.1 - base).abs()))
        .map_or(0, |(i, _)| i);
    let ratio = if (bases[0]..=bases[bases.len() - 1]).contains(&base) {
        1.0
    } else {
        base / bases[index]
    };
    (above[index] * ratio, below[index] * ratio)
}

fn glyphs() -> &'static Glyphs {
    static TABLE: OnceLock<Glyphs> = OnceLock::new();
    TABLE.get_or_init(|| {
        let value: serde_json::Value =
            serde_json::from_str(GLYPHS_JSON).expect("embedded equation glyph table is valid JSON");
        let read = |name: &str| {
            let mut map = HashMap::new();
            for (script, row) in value[name].as_object().expect("glyph object") {
                let widths: Vec<u32> = row
                    .as_array()
                    .expect("glyph widths")
                    .iter()
                    .map(|v| v.as_u64().expect("glyph width") as u32)
                    .collect();
                // Key each measured script by the token rhwp's parser produces.
                if let Some(key) = key_for(script) {
                    map.entry(key).or_insert(widths);
                }
            }
            map
        };
        Glyphs {
            grid: read("glyphs"),
            odd_upper: read("glyphs_odd_upper"),
        }
    })
}

/// Hancom's line height in pixels.
fn line_px(base: f64) -> f64 {
    (base / PX).round().max(1.0)
}

/// Script size: Hancom scales the line height and the 48-dpi size separately,
/// so pick a base unit that yields both rounded sizes.
fn script_base(base: f64) -> f64 {
    let exact = base * SCRIPT_SCALE;
    let h = line_px(exact);
    let s = (size48(base) * SCRIPT_SCALE).ceil().max(1.0);
    let low = ((h - 0.5) * PX).max((s - 0.5) * 150.0 + 1.0);
    let high = ((h + 0.5) * PX - 1.0).min((s + 0.5) * 150.0);
    if low <= high {
        exact.clamp(low, high)
    } else {
        exact
    }
}

/// The size rounded to a multiple of `quantum` HWPUNIT (big operator glyphs).
fn quantize(base: f64, quantum: f64) -> f64 {
    (base / quantum).round().max(1.0) * quantum
}

/// 48-dpi size used with the line height for glyph advances; .5 rounds down.
fn size48(base: f64) -> f64 {
    (base / 150.0 - 0.5).ceil().max(1.0)
}

impl Glyphs {
    fn width(&self, key: &Key, base: f64) -> Option<f64> {
        let h = line_px(base) as i64;
        let s = size48(base) as i64;
        let px = if h % 2 == 1 && s * 2 == h + 1 && (5..=47).contains(&h) {
            f64::from(*self.odd_upper.get(key)?.get(((h - 5) / 2) as usize)?)
        } else {
            let row = self.grid.get(key)?;
            if (4..=48).contains(&h) {
                f64::from(row[(h - 4) as usize])
            } else if h < 4 {
                f64::from(row[0]) * h as f64 / 4.0
            } else {
                f64::from(row[44]) * h as f64 / 48.0
            }
        };
        Some(px * PX)
    }
}

/// Spacing class of an operator symbol, as a fraction of the line height.
fn operator_space(symbol: &str) -> f64 {
    match symbol {
        "+" | "-" | "×" | "≈" | "−" => BINARY_SPACE,
        "=" | "<" | ">" | "≤" | "≥" | "≠" | ":" => RELATION_SPACE,
        _ => 0.0,
    }
}

/// Hancom accepts uppercase spellings (INFTY, LEQ) that rhwp reads as words.
fn symbol_alias(text: &str) -> Option<String> {
    use rhwp::renderer::equation::symbols::lookup_symbol;
    if text.chars().count() < 2 || !text.chars().all(|c| c.is_ascii_alphabetic()) {
        return None;
    }
    lookup_symbol(text)
        .or_else(|| lookup_symbol(&text.to_lowercase()))
        .or_else(|| match text.to_ascii_uppercase().as_str() {
            "INFTY" | "INF" => Some("∞"),
            _ => None,
        })
        .map(str::to_string)
}

fn is_integral(symbol: &str) -> bool {
    matches!(symbol, "∫" | "∬" | "∭" | "∮" | "∯" | "∰")
}

fn is_atom(node: &EqNode) -> bool {
    matches!(
        node,
        EqNode::Text(_)
            | EqNode::Number(_)
            | EqNode::Function(_)
            | EqNode::MathSymbol(_)
            | EqNode::Quoted(_)
    ) || matches!(
        node,
        EqNode::Superscript { .. } | EqNode::Subscript { .. } | EqNode::SubSup { .. }
    )
}

struct Layout {
    fitted: bool,
}

impl Layout {
    fn line(base: f64) -> f64 {
        line_px(base) * PX
    }

    fn text_box(width: f64, base: f64) -> Box_ {
        let height = Self::line(base);
        Box_ {
            width,
            ascent: height * TEXT_ASCENT,
            descent: height * (1.0 - TEXT_ASCENT),
        }
    }

    fn glyph(&mut self, key: Key, ch: &str, base: f64) -> f64 {
        // rhwp reads some punctuation as text; try the upright table too.
        let alternate = match &key {
            Key::Letter(_, c) => Some(Key::Upright(c.to_string())),
            Key::Upright(s) => s
                .chars()
                .next()
                .filter(|_| s.chars().count() == 1)
                .map(|c| Key::Letter(Style::Italic, c)),
        };
        let found = glyphs()
            .width(&key, base)
            .or_else(|| alternate.and_then(|k| glyphs().width(&k, base)));
        match found {
            Some(width) => width,
            None => {
                // Unmeasured glyph: CJK takes a full em, others a typical advance.
                self.fitted = false;
                #[cfg(test)]
                if std::env::var_os("MD2HWP_EQ_MISS").is_some() {
                    println!("MISS {ch}");
                }
                ch.chars()
                    .map(|c| if c >= '\u{1100}' { base } else { 0.55 * base })
                    .sum()
            }
        }
    }

    fn letters(&mut self, text: &str, style: Style, base: f64) -> f64 {
        let mut width = 0.0;
        for ch in text.chars() {
            let key = if ch.is_ascii_digit() || ch == '.' || !ch.is_alphanumeric() {
                Key::Upright(ch.to_string())
            } else {
                Key::Letter(style, ch)
            };
            width += self.glyph(key, &ch.to_string(), base);
        }
        width
    }

    fn node(&mut self, node: &EqNode, base: f64, style: Style) -> Box_ {
        match node {
            EqNode::Row(children) => self.row(children, base, style),
            EqNode::Text(t) if t.starts_with(BIG_MARK) => {
                self.big_operator(&t[BIG_MARK.len_utf8()..], None, None, base, style)
            }
            EqNode::Text(t) => match symbol_alias(t) {
                Some(symbol) => self.node(&EqNode::MathSymbol(symbol), base, style),
                None => Self::text_box(self.letters(t, style, base), base),
            },
            EqNode::Rel { arrow, over, under } => {
                // Unknown extensible arrows take an ordinary arrow's width.
                let width = match glyphs().width(&Key::Upright(arrow.clone()), base) {
                    Some(w) => w,
                    None => self.glyph(Key::Upright("→".into()), "→", base),
                };
                let mut out = Self::text_box(width, base);
                let small = script_base(base);
                for (part, above) in [(Some(&**over), true), (under.as_deref(), false)] {
                    if let Some(part) = part {
                        let s = self.node(part, small, style);
                        if above {
                            out.ascent += s.height();
                        } else {
                            out.descent += s.height();
                        }
                        out.width = out.width.max(s.width);
                    }
                }
                out
            }
            EqNode::Number(t) | EqNode::Function(t) | EqNode::Quoted(t) => {
                Self::text_box(self.letters(t, Style::Roman, base), base)
            }
            EqNode::Symbol(s) | EqNode::MathSymbol(s) => {
                if is_integral(s) {
                    return self.big_operator(s, None, None, base, style);
                }
                Self::text_box(self.glyph(Key::Upright(s.clone()), s, base), base)
            }
            EqNode::FontStyle { style: s, body } => {
                let next = match s {
                    FontStyleKind::Roman => Style::Roman,
                    FontStyleKind::Bold => Style::Bold,
                    FontStyleKind::Italic => Style::Italic,
                    _ => {
                        self.fitted = false;
                        style
                    }
                };
                let mut b = self.node(body, base, next);
                if next != Style::Bold {
                    b.ascent = b.ascent.max(EXPLICIT_STYLE_HEIGHT * base * TEXT_ASCENT);
                    b.descent = b
                        .descent
                        .max(EXPLICIT_STYLE_HEIGHT * base * (1.0 - TEXT_ASCENT));
                }
                b
            }
            EqNode::Color { body, .. } => self.node(body, base, style),
            EqNode::Space(kind) => {
                let width = match kind {
                    SpaceKind::Normal => NORMAL_SPACE * base,
                    SpaceKind::Thin => THIN_SPACE * base,
                    SpaceKind::Tab => COLUMN_GAP * base,
                };
                Box_ {
                    width,
                    ascent: base * TEXT_ASCENT,
                    descent: base * (1.0 - TEXT_ASCENT),
                }
            }
            EqNode::Superscript { base: b, sup } => self.scripts(b, None, Some(sup), base, style),
            EqNode::Subscript { base: b, sub } => self.scripts(b, Some(sub), None, base, style),
            EqNode::SubSup { base: b, sub, sup } => {
                self.scripts(b, Some(sub), Some(sup), base, style)
            }
            EqNode::Fraction { numer, denom } => self.fraction(numer, denom, base, style),
            EqNode::Atop { top, bottom } => {
                let top = self.node(top, base, style);
                let bottom = self.node(bottom, base, style);
                let height = top.height() + bottom.height() + ATOP_GAP * Self::line(base);
                let ascent = height / 2.0 + AXIS * base;
                Box_ {
                    width: top.width.max(bottom.width) + FRACTION_EXTRA * base,
                    ascent,
                    descent: height - ascent,
                }
            }
            EqNode::Sqrt { index, body } => {
                let inner = self.node(body, base, style);
                let line = Self::line(base);
                let mut width = inner.width + SQRT_EXTRA * line;
                if let Some(index) = index {
                    let i = self.node(index, script_base(base), style);
                    width += (i.width - ROOT_INDEX_FREE * line).max(0.0);
                }
                Box_ {
                    width,
                    ascent: inner.ascent + SQRT_TOP * line,
                    descent: inner.descent,
                }
            }
            EqNode::Paren { left, right, body } => {
                // binom/choose: a two-row pile inside parentheses.
                let inner = if let EqNode::Atop { top, bottom } = &**body {
                    self.grid(
                        &[vec![(**top).clone()], vec![(**bottom).clone()]],
                        base,
                        style,
                    )
                } else {
                    self.node(body, base, style)
                };
                let w = self.delimiter(left, base) + self.delimiter(right, base);
                Box_ {
                    width: inner.width + w,
                    ..inner
                }
            }
            EqNode::BigOp { symbol, sub, sup } => {
                self.big_operator(symbol, sub.as_deref(), sup.as_deref(), base, style)
            }
            EqNode::Limit { is_upper, sub } => self.limit(*is_upper, sub.as_deref(), base, style),
            EqNode::Decoration { kind, body } => {
                let mut b = self.node(body, base, style);
                let line = Self::line(base);
                match kind {
                    DecoKind::Under | DecoKind::Underline => b.descent += DECORATION_TOP * line,
                    _ => b.ascent += DECORATION_TOP * line,
                }
                if matches!(
                    kind,
                    DecoKind::Bar
                        | DecoKind::Vec
                        | DecoKind::Overline
                        | DecoKind::Underline
                        | DecoKind::Dyad
                ) {
                    b.width += DECORATION_WIDE * line;
                }
                b
            }
            EqNode::Matrix {
                rows,
                style: matrix,
            } => {
                let mut b = self.grid(rows, base, style);
                b.width += match matrix {
                    MatrixStyle::Plain => 0.0,
                    MatrixStyle::Paren => self.delimiter("(", base) + self.delimiter(")", base),
                    MatrixStyle::Bracket | MatrixStyle::Vert => 2.0 * size48(base) * PX,
                };
                b
            }
            EqNode::Pile { rows, .. } => {
                let rows: Vec<Vec<EqNode>> = rows.iter().map(|r| vec![r.clone()]).collect();
                self.grid(&rows, base, style)
            }
            EqNode::EqAlign { rows } => {
                // Left parts align right, right parts left; the first row owns the baseline.
                let line = Self::line(base);
                let (mut left, mut right, mut height, mut ascent) = (0.0_f64, 0.0_f64, 0.0, 0.0);
                for (i, (l, r)) in rows.iter().enumerate() {
                    let l = self.node(l, base, style);
                    let r = self.node(r, base, style);
                    left = left.max(l.width);
                    right = right.max(r.width);
                    let row_ascent = l.ascent.max(r.ascent);
                    if i == 0 {
                        ascent = row_ascent;
                    } else {
                        height += ATOP_GAP * line;
                    }
                    height += row_ascent + l.descent.max(r.descent);
                }
                Box_ {
                    width: left + right + EQALIGN_GAP * line,
                    ascent,
                    descent: height - ascent,
                }
            }
            EqNode::Cases { rows } => {
                let rows: Vec<Vec<EqNode>> = rows.iter().map(|r| vec![r.clone()]).collect();
                let mut b = self.grid(&rows, base, style);
                b.width += size48(base) * PX;
                b
            }
            EqNode::Empty | EqNode::Newline => Box_::default(),
        }
    }

    /// `underover` stacks scripts above and below an ordinary base.
    fn underover(
        &mut self,
        n: &EqNode,
        sub: Option<&EqNode>,
        sup: Option<&EqNode>,
        base: f64,
        style: Style,
    ) -> Box_ {
        let core = self.node(n, base, style);
        self.limits(core, sub, sup, base, style)
    }

    /// lim/Lim are roman letters at 1.2 times the size, with stacked limits.
    fn limit(&mut self, upper: bool, sub: Option<&EqNode>, base: f64, style: Style) -> Box_ {
        let name = if upper { "Lim" } else { "lim" };
        let height = Self::line(base * LIM_HEIGHT);
        let core = Box_ {
            width: self.letters(name, Style::Roman, base * LIM_HEIGHT),
            ascent: height * LIM_ASCENT,
            descent: height * (1.0 - LIM_ASCENT),
        };
        self.limits(core, sub, None, base, style)
    }

    fn delimiter(&mut self, d: &str, base: f64) -> f64 {
        match d {
            "(" | ")" => self.glyph(Key::Upright(d.to_string()), d, base),
            "" | "." => 0.0,
            "⟨" | "⟩" => ANGLE_WIDTH * size48(base) * PX,
            _ => size48(base) * PX,
        }
    }

    fn row(&mut self, children: &[EqNode], base: f64, style: Style) -> Box_ {
        let mut out = Box_::default();
        let line = Self::line(base);
        // Hancom `rm`/`it` switch the shape of the rest of the group; rhwp
        // applies them to one term only, so carry the switch here.
        let mut current = style;
        let mut skip = 0;
        for (i, child) in children.iter().enumerate() {
            if skip > 0 {
                skip -= 1;
                continue;
            }
            match child {
                EqNode::FontStyle { style: s, body } if matches!(**body, EqNode::Empty) => {
                    current = match s {
                        FontStyleKind::Roman => Style::Roman,
                        _ => Style::Italic,
                    };
                    continue;
                }
                // Hancom 2020 shows no SCALE size effect; skip `scale N`.
                EqNode::Text(t)
                    if t == "scale" && matches!(children.get(i + 1), Some(EqNode::Number(_))) =>
                {
                    skip = 1;
                    continue;
                }
                EqNode::Text(t)
                    if (t == OVERBRACE || t == UNDERBRACE) && children.len() > i + 2 =>
                {
                    let body = self.node(&children[i + 1], base, current);
                    let label = self.node(&children[i + 2], script_base(base), current);
                    let line = Self::line(base);
                    let mut b = body;
                    b.width = body.width.max(label.width) + BRACE_WIDTH * line + BRACE_WIDTH_FIXED;
                    if t == OVERBRACE {
                        b.ascent += OVERBRACE_HEIGHT * line + label.height();
                    } else {
                        b.descent += UNDERBRACE_HEIGHT * line + label.height();
                    }
                    out.width += b.width;
                    out.ascent = out.ascent.max(b.ascent);
                    out.descent = out.descent.max(b.descent);
                    skip = 2;
                    continue;
                }
                EqNode::Text(t) if t == "box" && children.len() > i + 1 => {
                    let mut b = self.node(&children[i + 1], base, current);
                    let line = Self::line(base);
                    b.width += BOX_PAD_X * line;
                    b.ascent += BOX_PAD_Y * line;
                    b.descent += BOX_PAD_Y * line;
                    out.width += b.width;
                    out.ascent = out.ascent.max(b.ascent);
                    out.descent = out.descent.max(b.descent);
                    skip = 1;
                    continue;
                }
                EqNode::Text(t) if t == "underover" => {
                    if let Some(next) = children.get(i + 1) {
                        let b = match next {
                            EqNode::Subscript { base: n, sub } => {
                                self.underover(n, Some(sub), None, base, current)
                            }
                            EqNode::Superscript { base: n, sup } => {
                                self.underover(n, None, Some(sup), base, current)
                            }
                            EqNode::SubSup { base: n, sub, sup } => {
                                self.underover(n, Some(sub), Some(sup), base, current)
                            }
                            other => self.node(other, base, current),
                        };
                        out.width += b.width;
                        out.ascent = out.ascent.max(b.ascent);
                        out.descent = out.descent.max(b.descent);
                        skip = 1;
                        continue;
                    }
                }
                _ => {}
            }
            let b = self.node(child, base, current);
            if let EqNode::FontStyle {
                style: FontStyleKind::Roman,
                ..
            } = child
            {
                current = Style::Roman;
            }
            let (mut before, mut after) = (0.0, 0.0);
            let symbol = match child {
                EqNode::Symbol(s) | EqNode::MathSymbol(s) => Some(s.clone()),
                EqNode::Text(t) => symbol_alias(t),
                EqNode::Rel { .. } => Some("=".to_string()),
                _ => None,
            };
            if let Some(s) = symbol.as_ref() {
                let space = operator_space(s) * line;
                if i > 0 {
                    before = space / 2.0;
                }
                if i + 1 < children.len() {
                    after = space / 2.0;
                }
                if s == "," && i + 1 < children.len() {
                    after += COMMA_SPACE * line;
                }
                if s == "(" || s == "[" {
                    before += OPEN_SPACE * base;
                    if i > 0 && is_atom(&children[i - 1]) {
                        before += OPEN_SPACE * base;
                    }
                }
            }
            out.width += before + b.width + after;
            out.ascent = out.ascent.max(b.ascent);
            out.descent = out.descent.max(b.descent);
        }
        out
    }

    fn scripts(
        &mut self,
        b: &EqNode,
        sub: Option<&EqNode>,
        sup: Option<&EqNode>,
        base: f64,
        style: Style,
    ) -> Box_ {
        // Integrals keep their limits at the right; ∑-like operators stack them.
        // md2hwp writes `{sum} _{..} ^{..}` and `{lim} _{..}`, which parse as
        // scripts on a bare operator.
        match b {
            EqNode::MathSymbol(s) | EqNode::Symbol(s) if is_integral(s) => {
                return self.big_operator(s, sub, sup, base, style);
            }
            _ => {}
        }
        // A grouped operator such as `{sum} _{a} ^{b}` or `{lim} _{a}` takes
        // ordinary side scripts in Hancom; only bare `sum _{a}` stacks limits.
        let nucleus = self.node(b, base, style);
        let small = script_base(base);
        let mut out = nucleus;
        let mut extra: f64 = 0.0;
        // A one-line script on a one-line nucleus extends the line by the
        // measured amount; taller nuclei and scripts shift it from there.
        let text_ascent = Self::line(base) * TEXT_ASCENT;
        let text_descent = Self::line(base) - text_ascent;
        let script_ascent = Self::line(small) * TEXT_ASCENT;
        let script_descent = Self::line(small) - script_ascent;
        let (above, below) = script_extents(base);
        if let Some(sup) = sup {
            let s = self.node(sup, small, style);
            let raise = text_ascent + above - script_ascent
                + (nucleus.ascent - text_ascent).max(0.0)
                + SUP_DEPTH_LIFT * (s.descent - script_descent).max(0.0);
            out.ascent = out.ascent.max(raise + s.ascent);
            extra = extra.max(s.width);
        }
        if let Some(sub) = sub {
            let s = self.node(sub, small, style);
            let drop =
                text_descent + below - script_descent + (nucleus.descent - text_descent).max(0.0);
            out.descent = out.descent.max(drop + s.descent);
            extra = extra.max(s.width);
        }
        out.width += extra;
        out
    }

    fn fraction(&mut self, n: &EqNode, d: &EqNode, base: f64, style: Style) -> Box_ {
        let num = self.node(n, base, style);
        let den = self.node(d, base, style);
        let gap = FRACTION_GAP * Self::line(base);
        let height = num.height() + den.height() + gap;
        let ascent = num.height() + gap / 2.0 + AXIS * base;
        Box_ {
            width: num.width.max(den.width) + FRACTION_EXTRA * base,
            ascent,
            descent: height - ascent,
        }
    }

    fn big_operator(
        &mut self,
        symbol: &str,
        sub: Option<&EqNode>,
        sup: Option<&EqNode>,
        base: f64,
        style: Style,
    ) -> Box_ {
        let line = Self::line(base);
        if is_integral(symbol) {
            let multiple = match symbol {
                "∬" | "∯" => INTEGRAL_DOUBLE,
                "∭" | "∰" => INTEGRAL_TRIPLE,
                _ => 0.0,
            };
            let core = Box_ {
                width: (INTEGRAL_WIDTH + multiple) * line,
                ascent: INTEGRAL_HEIGHT * quantize(base, INTEGRAL_QUANTUM) * INTEGRAL_ASCENT,
                descent: INTEGRAL_HEIGHT
                    * quantize(base, INTEGRAL_QUANTUM)
                    * (1.0 - INTEGRAL_ASCENT),
            };
            // Integral limits sit at the right, above and below the sign.
            let small = script_base(base);
            let mut out = core;
            let mut extra: f64 = 0.0;
            let script_line = Self::line(small);
            if let Some(sup) = sup {
                let s = self.node(sup, small, style);
                out.ascent += s.height() - script_line + INTEGRAL_LIMIT * line;
                extra = extra.max(s.width);
            }
            if let Some(sub) = sub {
                let s = self.node(sub, small, style);
                out.descent += s.height() - script_line + INTEGRAL_LIMIT * line;
                // A lone lower limit tucks under the slanted sign.
                let tuck = if sup.is_none() {
                    INTEGRAL_SUB_TUCK * line
                } else {
                    0.0
                };
                extra = extra.max(s.width - tuck);
            }
            out.width += extra;
            return out;
        }
        let width = match symbol {
            "∑" | "∏" | "∐" => BIG_OP_WIDTH,
            "∩" | "∪" => BIG_CAP_WIDTH,
            _ => BIG_OTHER_WIDTH,
        };
        let core = Box_ {
            width: width * line,
            ascent: BIG_OP_HEIGHT * quantize(base, BIG_OP_QUANTUM) * BIG_OP_ASCENT,
            descent: BIG_OP_HEIGHT * quantize(base, BIG_OP_QUANTUM) * (1.0 - BIG_OP_ASCENT),
        };
        self.limits(core, sub, sup, base, style)
    }

    fn limits(
        &mut self,
        core: Box_,
        sub: Option<&EqNode>,
        sup: Option<&EqNode>,
        base: f64,
        style: Style,
    ) -> Box_ {
        let small = script_base(base);
        let mut out = core;
        if let Some(sup) = sup {
            let s = self.node(sup, small, style);
            out.ascent += s.height() + LIMIT_GAP * base;
            out.width = out.width.max(s.width);
        }
        if let Some(sub) = sub {
            let s = self.node(sub, small, style);
            out.descent += s.height() + LIMIT_GAP * base;
            out.width = out.width.max(s.width);
        }
        out
    }

    fn grid(&mut self, rows: &[Vec<EqNode>], base: f64, style: Style) -> Box_ {
        let line = Self::line(base);
        let mut widths: Vec<f64> = Vec::new();
        let mut height = 0.0;
        for (r, row) in rows.iter().enumerate() {
            let mut row_box = Box_::default();
            for (c, cell) in row.iter().enumerate() {
                let b = self.node(cell, base, style);
                if widths.len() <= c {
                    widths.push(0.0);
                }
                widths[c] = widths[c].max(b.width);
                row_box.ascent = row_box.ascent.max(b.ascent);
                row_box.descent = row_box.descent.max(b.descent);
            }
            height += row_box.height() + if r > 0 { ROW_GAP * line } else { 0.0 };
        }
        let columns = widths.len().max(1) as f64;
        let width = widths.iter().sum::<f64>() + (columns - 1.0) * COLUMN_GAP * base;
        let ascent = height / 2.0 + AXIS * base;
        Box_ {
            width,
            ascent,
            descent: height - ascent,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GEOMETRY_JSON: &str = include_str!("../data/hancom-equation-geometry.json");

    fn cases_from(json: &str) -> Vec<(String, u32, u32, u32, i64)> {
        let value: serde_json::Value = serde_json::from_str(json).unwrap();
        value["cases"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| {
                (
                    c["script"].as_str().unwrap().to_string(),
                    c["base_unit"].as_u64().unwrap() as u32,
                    c["width"].as_u64().unwrap() as u32,
                    c["height"].as_u64().unwrap() as u32,
                    c["baseline"].as_i64().unwrap(),
                )
            })
            .collect()
    }

    #[test]
    #[ignore]
    fn print_probe_ast() {
        for s in [
            "{LEFT < a RIGHT >}",
            "{lim} _{a -> b} {c}",
            "{rel EXARROW {a} {b}}",
            "{OVERBRACE {a} {b}}",
            "{UNDERBRACE {a} {b}}",
            "tint _{a} ^{b} {c}",
            "otint _{a} ^{b} c",
            "{LEFT < a RIGHT >}",
            "{BIGVEE} _{a} ^{b}",
            "{inter} _{a} ^{b}",
            "box {a}",
        ] {
            println!(
                "AST {s} => {:?} TOK {:?}",
                EqParser::new(tokens(s)).parse(),
                tokens(s)
                    .iter()
                    .map(|t| (t.ty, t.value.clone()))
                    .collect::<Vec<_>>()
            );
        }
    }
    /// Writes md2hwp-converted KaTeX scripts for out-of-sample Hancom measurement.
    #[test]
    #[ignore]
    fn write_katex_scripts() {
        let Ok(out) = std::env::var("MD2HWP_EQ_SCRIPTS_OUT") else {
            return;
        };
        let source = if std::env::var("MD2HWP_EQ_SOURCE").as_deref() == Ok("toolbar") {
            include_str!("../../../crates/md2hwp-math/tests/fixtures/toolbar-equations.json")
        } else {
            include_str!("../../../tests/katex/cases.json")
        };
        let cases: serde_json::Value = serde_json::from_str(source).unwrap();
        let mut list = Vec::new();
        for case in cases["cases"].as_array().unwrap() {
            let display = case["options"]["displayMode"].as_bool().unwrap_or(false);
            let Ok(script) =
                md2hwp_math::tex_to_hml_with_display(case["tex"].as_str().unwrap(), display)
            else {
                continue;
            };
            for unit in [1000, 1400] {
                list.push(serde_json::json!({"script": script, "base_unit": unit}));
            }
        }
        std::fs::write(
            out,
            serde_json::to_string_pretty(&serde_json::json!({"cases": list})).unwrap(),
        )
        .unwrap();
    }

    /// Syntax md2hwp never writes and the rhwp parser reads differently from
    /// Hancom (unbraced operands, `from`/`to`, `choose`).
    const PARSER_MISMATCH: [&str; 4] = [
        "root 3 of x",
        "sum from {i=1} to n x",
        "x^2+1",
        "{n} choose {k}",
    ];

    struct Residuals {
        width: Vec<i64>,
        height: Vec<i64>,
        baseline: Vec<i64>,
    }

    fn residuals(json: &str) -> Residuals {
        let mut r = Residuals {
            width: Vec::new(),
            height: Vec::new(),
            baseline: Vec::new(),
        };
        for (script, unit, w, h, bl) in cases_from(json) {
            if PARSER_MISMATCH.contains(&script.as_str()) {
                continue;
            }
            let g = measure(&script, unit);
            r.width.push((i64::from(g.width) - i64::from(w)).abs());
            r.height.push((i64::from(g.height) - i64::from(h)).abs());
            r.baseline.push((i64::from(g.baseline) - bl).abs());
        }
        r
    }

    fn quantile(values: &[i64], q: f64) -> i64 {
        let mut v = values.to_vec();
        v.sort_unstable();
        v[((v.len() as f64 * q) as usize).min(v.len() - 1)]
    }

    /// Accuracy against Hancom 2020 measurements (HWPUNIT; 75 = one pixel).
    #[test]
    fn matches_fitted_hancom_measurements() {
        let r = residuals(GEOMETRY_JSON);
        assert!(r.width.len() > 1300);
        assert!(
            quantile(&r.width, 0.5) <= 10,
            "width median {}",
            quantile(&r.width, 0.5)
        );
        assert!(
            quantile(&r.width, 0.9) <= 100,
            "width p90 {}",
            quantile(&r.width, 0.9)
        );
        assert!(
            quantile(&r.height, 0.9) <= 75,
            "height p90 {}",
            quantile(&r.height, 0.9)
        );
        assert!(
            quantile(&r.baseline, 0.9) <= 2,
            "baseline p90 {}",
            quantile(&r.baseline, 0.9)
        );
    }

    /// Out-of-sample toolbar equations converted by md2hwp-math.
    #[test]
    fn matches_toolbar_hancom_measurements() {
        let r = residuals(include_str!("../data/hancom-equation-toolbar.json"));
        assert!(r.width.len() > 100);
        assert!(
            quantile(&r.width, 0.5) <= 40,
            "width median {}",
            quantile(&r.width, 0.5)
        );
        assert!(
            quantile(&r.width, 0.9) <= 150,
            "width p90 {}",
            quantile(&r.width, 0.9)
        );
        assert!(
            quantile(&r.height, 0.9) <= 300,
            "height p90 {}",
            quantile(&r.height, 0.9)
        );
        assert!(
            quantile(&r.baseline, 0.9) <= 10,
            "baseline p90 {}",
            quantile(&r.baseline, 0.9)
        );
    }

    /// Footnote-size equation saved by Hancom 2020 (an underestimate hid the
    /// following period); scripts at 8pt use the rounded-up script size.
    #[test]
    fn matches_small_script_sizes() {
        let g = measure("e ^{i pi} + 1 = 0", 800);
        assert!((i64::from(g.width) - 3458).abs() <= 75, "{g:?}");
        for (script, width) in [
            ("e ^{i pi}", 900),
            ("x ^{pi}", 825),
            ("e ^{i}", 525),
            ("e ^{pi}", 675),
        ] {
            assert_eq!(measure(script, 800).width, width, "{script}");
        }
    }

    /// Script and big-operator heights across sizes (500–2000 in 25–50 steps),
    /// where pixel rounding made fixed fractions drift by up to a pixel and
    /// moved line and page breaks in a large manuscript.
    #[test]
    fn matches_hancom_heights_across_sizes() {
        let r = residuals(include_str!("../data/hancom-equation-heights.json"));
        assert!(r.height.len() > 400);
        assert!(
            quantile(&r.height, 0.5) <= 10,
            "height median {}",
            quantile(&r.height, 0.5)
        );
        assert!(
            quantile(&r.height, 0.9) <= 40,
            "height p90 {}",
            quantile(&r.height, 0.9)
        );
        assert!(
            quantile(&r.width, 0.9) <= 100,
            "width p90 {}",
            quantile(&r.width, 0.9)
        );
        assert!(
            quantile(&r.baseline, 0.9) <= 2,
            "baseline p90 {}",
            quantile(&r.baseline, 0.9)
        );
    }

    /// Unknown or malformed scripts still get a positive estimate.
    #[test]
    fn estimates_without_rejecting() {
        for script in [
            "",
            "{",
            "}}}",
            "가나다",
            "🙂 over",
            "matrix {",
            "LEFT ( a",
            "unknowncommand x",
        ] {
            let g = measure(script, 1000);
            assert!(g.width >= 1 && g.height >= 1, "{script:?} -> {g:?}");
            assert!((0..=100).contains(&g.baseline));
        }
    }

    /// Prints native equation and Equation AUTONUM records of `MD2HWP_EQ_DUMP`.
    #[test]
    #[ignore]
    fn dump_equations() {
        use rhwp::model::control::Control;
        let Ok(path) = std::env::var("MD2HWP_EQ_DUMP") else {
            return;
        };
        let core = rhwp::DocumentCore::from_bytes(&std::fs::read(path).unwrap()).unwrap();
        fn walk(ps: &[rhwp::model::paragraph::Paragraph], depth: usize) {
            for p in ps {
                for c in &p.controls {
                    match c {
                        Control::Equation(e) => {
                            println!(
                                "EQ depth={depth} style={} text={:?} offsets={:?} shapes={:?}",
                                p.style_id, p.text, p.char_offsets, p.char_shapes
                            );
                            println!("   {:?}", e);
                        }
                        Control::AutoNumber(n) => println!("AUTONUM depth={depth} {:?}", n),
                        Control::Footnote(f) => walk(&f.paragraphs, depth + 1),
                        _ => {}
                    }
                }
            }
        }
        walk(&core.document().sections[0].paragraphs, 0);
    }

    /// Prints every residual; run with --ignored --nocapture while fitting.
    #[test]
    #[ignore]
    fn print_residuals() {
        let ground = std::env::var("MD2HWP_EQ_GROUND")
            .ok()
            .map(|p| std::fs::read_to_string(p).unwrap());
        for (script, unit, w, h, bl) in cases_from(ground.as_deref().unwrap_or(GEOMETRY_JSON)) {
            let g = measure(&script, unit);
            println!(
                "RES {:>5} {:>3} | {:<34} | w {:>6} h {:>6} bl {:>3}",
                unit,
                if g.fitted { "fit" } else { "est" },
                script,
                i64::from(g.width) - i64::from(w),
                i64::from(g.height) - i64::from(h),
                i64::from(g.baseline) - bl
            );
        }
    }
}
