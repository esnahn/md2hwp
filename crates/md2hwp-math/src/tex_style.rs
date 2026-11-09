// SPDX-License-Identifier: AGPL-3.0-only
//! Logical TeX styles, independent of native SCALE and font sizes.
use std::{iter::Peekable, str::Chars};

fn control(chars: &mut Peekable<Chars<'_>>) -> String {
    let mut name = String::new();
    if chars.peek().is_some_and(char::is_ascii_alphabetic) {
        while chars.peek().is_some_and(char::is_ascii_alphabetic) {
            name.push(chars.next().expect("peeked character"));
        }
    } else if let Some(ch) = chars.next() {
        name.push(ch);
    }
    name
}

fn delimiter(chars: &mut Peekable<Chars<'_>>) {
    while chars.peek().is_some_and(|c| c.is_whitespace()) {
        chars.next();
    }
    if chars.next() == Some('\\') {
        control(chars);
    }
}

fn has_optional(name: &str) -> bool {
    matches!(
        name,
        "sqrt"
            | "cfrac"
            | "color"
            | "textcolor"
            | "xrightarrow"
            | "xleftarrow"
            | "xRightarrow"
            | "xLeftarrow"
            | "xleftrightarrow"
            | "xLeftrightarrow"
            | "xrightleftharpoons"
            | "xhookrightarrow"
            | "xhookleftarrow"
            | "xtwoheadrightarrow"
            | "xtwoheadleftarrow"
            | "xrightleftarrows"
            | "xrightequilibrium"
            | "xleftequilibrium"
    )
}

fn optional(chars: &mut Peekable<Chars<'_>>, depth: usize) -> bool {
    // The real parser reports excessive nesting; lookahead must also remain bounded.
    if depth > 96 {
        return false;
    }
    while chars.peek().is_some_and(|c| c.is_whitespace()) {
        chars.next();
    }
    if chars.peek() != Some(&'[') {
        return true;
    }
    chars.next();
    let (mut braces, mut fences, mut environments) = (0usize, 0usize, 0usize);
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            let name = control(chars);
            match name.as_str() {
                "begin" => environments += 1,
                "end" => environments = environments.saturating_sub(1),
                "left" | "right" | "middle" => {
                    if name == "left" {
                        fences += 1;
                    }
                    if name == "right" {
                        fences = fences.saturating_sub(1);
                    }
                    delimiter(chars);
                }
                _ if has_optional(&name) && !optional(chars, depth + 1) => return false,
                _ => {}
            }
        } else if ch == ']' && braces == 0 && fences == 0 && environments == 0 {
            return true;
        } else if ch == '{' {
            braces += 1;
        } else if ch == '}' {
            braces = braces.saturating_sub(1);
        }
    }
    false
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MathStyle {
    Display,
    Text,
    Script,
    ScriptScript,
}

impl MathStyle {
    pub(crate) const ALL: [Self; 4] = [Self::Display, Self::Text, Self::Script, Self::ScriptScript];

    pub(crate) fn script(self) -> Self {
        match self {
            Self::Display | Self::Text => Self::Script,
            Self::Script | Self::ScriptScript => Self::ScriptScript,
        }
    }

    pub(crate) fn fraction(self) -> Self {
        match self {
            Self::Display => Self::Text,
            Self::Text => Self::Script,
            Self::Script | Self::ScriptScript => Self::ScriptScript,
        }
    }
}

// Look ahead within one current math list, never through a nested group,
// left/right pair, environment, row or cell. Infix fractions determine the
// numerator's style before the streaming parser has reached the infix command.
pub(crate) fn has_infix_fraction(source: &str, close: Option<char>, cells: bool) -> bool {
    let mut chars = source.chars().peekable();
    let (mut braces, mut fences, mut environments) = (0usize, 0usize, 0usize);
    while let Some(ch) = chars.next() {
        let top = braces == 0 && fences == 0 && environments == 0;
        if ch == '\\' {
            let mut name = String::new();
            if chars.peek().is_some_and(char::is_ascii_alphabetic) {
                while chars.peek().is_some_and(char::is_ascii_alphabetic) {
                    name.push(chars.next().expect("peeked character"));
                }
            } else if let Some(escaped) = chars.next() {
                if top && escaped == '\\' && (cells || close != Some('}')) {
                    return false;
                }
                continue;
            }
            match name.as_str() {
                "over" | "atop" | "brace" | "brack" | "above" if top => return true,
                "end" if top => return false,
                "cr" if top && cells => return false,
                "newline" if top && (cells || close != Some('}')) => return false,
                "begin" => environments += 1,
                "end" => environments = environments.saturating_sub(1),
                "right" if top => return false,
                "left" | "right" | "middle" => {
                    if name == "left" {
                        fences += 1;
                    }
                    if name == "right" {
                        fences = fences.saturating_sub(1);
                    }
                    while chars.peek().is_some_and(|c| c.is_whitespace()) {
                        chars.next();
                    }
                    // Delimiter braces are glyphs, not argument groups.
                    if chars.next() == Some('\\') {
                        if chars.peek().is_some_and(char::is_ascii_alphabetic) {
                            while chars.peek().is_some_and(char::is_ascii_alphabetic) {
                                chars.next();
                            }
                        } else {
                            chars.next();
                        }
                    }
                }
                _ if has_optional(&name) && !optional(&mut chars, 0) => return false,
                _ => {}
            }
        } else if top && (Some(ch) == close || ch == '}' || (cells && ch == '&')) {
            return false;
        } else if ch == '{' {
            braces += 1;
        } else if ch == '}' {
            braces = braces.saturating_sub(1);
        }
    }
    false
}
