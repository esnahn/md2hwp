// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Euisoon Ahn
//! HTML comments are author notes, not manuscript content.
//!
//! Comments are removed from the Pandoc AST before normalization and strict
//! AST checks, so a manuscript behaves as if they were absent: captions and
//! object notes attach across them. Raw HTML holding anything besides
//! comments and whitespace is left in place and rejected as before.

use serde_json::Value;

use crate::pandoc_input::PandocDocument;

/// Byte length of the HTML comment at the start of `text`, if it starts with
/// a complete one. CommonMark 0.31 comments are `<!-->`, `<!--->`, or `<!--`
/// followed by text up to the first `-->`.
pub(crate) fn comment_len(text: &str) -> Option<usize> {
    let body = text.strip_prefix("<!--")?;
    if body.starts_with('>') {
        Some(5)
    } else if body.starts_with("->") {
        Some(6)
    } else {
        body.find("-->").map(|end| 4 + end + 3)
    }
}

/// Whether `text` holds at least one HTML comment and otherwise only ASCII
/// whitespace.
pub(crate) fn is_comments_only(text: &str) -> bool {
    fn skip_space(text: &str) -> &str {
        text.trim_start_matches(|c: char| c.is_ascii_whitespace())
    }
    let mut rest = skip_space(text);
    if rest.is_empty() {
        return false;
    }
    while !rest.is_empty() {
        let Some(length) = comment_len(rest) else {
            return false;
        };
        rest = skip_space(&rest[length..]);
    }
    true
}

/// Removes comment-only raw HTML from the blocks and metadata, together with
/// paragraphs that held nothing else. Paths in later errors and diagnostics
/// refer to this comment-free AST.
pub(crate) fn strip(document: &mut PandocDocument) {
    for value in document.metadata.values_mut() {
        strip_value(value);
    }
    strip_list(&mut document.blocks);
}

fn kind(node: &Value) -> Option<&str> {
    node.get("t").and_then(Value::as_str)
}

fn is_space(node: &Value) -> bool {
    matches!(kind(node), Some("Space" | "SoftBreak"))
}

fn is_comment(node: &Value) -> bool {
    matches!(kind(node), Some("RawBlock" | "RawInline"))
        && node
            .get("c")
            .and_then(Value::as_array)
            .is_some_and(|content| match content.as_slice() {
                [format, text] => {
                    format.as_str() == Some("html") && text.as_str().is_some_and(is_comments_only)
                }
                _ => false,
            })
}

/// Strips comments inside `value` and returns whether `value` itself must be
/// removed from the list holding it: it is a comment, or a paragraph that held
/// only comments and whitespace.
fn strip_value(value: &mut Value) -> bool {
    if is_comment(value) {
        return true;
    }
    match value {
        Value::Array(items) => {
            strip_list(items);
            false
        }
        Value::Object(node) => {
            let mut emptied = false;
            for child in node.values_mut() {
                if let Value::Array(items) = child {
                    emptied |= strip_list(items) && items.is_empty();
                } else {
                    strip_value(child);
                }
            }
            emptied
                && matches!(
                    node.get("t").and_then(Value::as_str),
                    Some("Para" | "Plain")
                )
        }
        _ => false,
    }
}

/// Strips comments from one list of nodes and returns whether it removed any.
/// Where an inline comment is removed, whitespace is not doubled, not left at
/// either end and not left before a hard line break, and adjacent `Str` text
/// is joined, as if the comment had never been written.
fn strip_list(items: &mut Vec<Value>) -> bool {
    let mut removed = false;
    let mut seam = false;
    for mut item in std::mem::take(items) {
        if strip_value(&mut item) {
            removed = true;
            seam |= kind(&item) == Some("RawInline");
            continue;
        }
        if seam {
            if is_space(&item) && items.last().is_none_or(is_space) {
                continue;
            }
            if kind(&item) == Some("LineBreak") {
                while items.last().is_some_and(is_space) {
                    items.pop();
                }
            }
            if let (Some("Str"), Some(Value::String(text))) = (kind(&item), item.get("c"))
                && let Some(previous) = items.last_mut().filter(|node| kind(node) == Some("Str"))
                && let Some(Value::String(joined)) = previous.get_mut("c")
            {
                joined.push_str(text);
                seam = false;
                continue;
            }
            seam = false;
        }
        items.push(item);
    }
    if seam {
        while items.last().is_some_and(is_space) {
            items.pop();
        }
    }
    removed
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn recognizes_comment_only_text() {
        for text in [
            "<!-- 주석 -->",
            "  <!-- 여러\n\n줄 -->\n",
            "<!-- a --> <!-- b -->\n",
            "<!---->",
            "<!-->",
            "<!--->",
            "<!-- -- -->",
        ] {
            assert!(is_comments_only(text), "{text:?}");
        }
        for text in [
            "",
            " \n",
            "<!-- 닫히지 않음\n\n내용\n\n",
            "<!-- c --> 뒤 텍스트\n",
            "앞 <!-- c -->",
            "<div><!-- c --></div>",
            "<!-- a <!-- b --> c -->",
            "<!-- c -->\u{3000}",
            "<sup>",
        ] {
            assert!(!is_comments_only(text), "{text:?}");
        }
    }

    fn strip_inlines(inlines: Value) -> Value {
        let mut value = json!([{"t":"Para","c":inlines}]);
        strip_value(&mut value);
        value
    }

    fn comment() -> Value {
        json!({"t":"RawInline","c":["html","<!-- 주석 -->"]})
    }

    #[test]
    fn removes_inline_comments_without_leaving_extra_whitespace() {
        let s = |text: &str| json!({"t":"Str","c":text});
        let space = json!({"t":"Space"});
        let soft = json!({"t":"SoftBreak"});
        let cases = [
            // 본문 <!-- 주석 --> 계속.
            (
                json!([s("본문"), space, comment(), space, s("계속.")]),
                json!([s("본문"), space, s("계속.")]),
            ),
            // 앞<!-- 주석 -->뒤
            (json!([s("앞"), comment(), s("뒤")]), json!([s("앞뒤")])),
            // 본문 <!-- 주석 -->
            (json!([s("본문"), space, comment()]), json!([s("본문")])),
            (
                json!([s("본문"), space, comment(), soft, s("다음")]),
                json!([s("본문"), space, s("다음")]),
            ),
            (
                json!([s("본문"), space, comment(), {"t":"LineBreak"}, s("다음")]),
                json!([s("본문"), {"t":"LineBreak"}, s("다음")]),
            ),
            (
                json!([s("a"), space, comment(), space, comment(), space, s("b")]),
                json!([s("a"), space, s("b")]),
            ),
        ];
        for (input, expected) in cases {
            assert_eq!(
                strip_inlines(input.clone()),
                json!([{"t":"Para","c":expected}]),
                "{input}"
            );
        }
        // A paragraph that held only comments disappears.
        assert_eq!(strip_inlines(json!([comment(), space])), json!([]));
    }

    #[test]
    fn keeps_raw_html_that_is_not_only_comments() {
        let blocks = json!([
            {"t":"RawBlock","c":["html","<!-- c --> 뒤 텍스트\n"]},
            {"t":"RawBlock","c":["latex","<!-- c -->"]},
            {"t":"Para","c":[{"t":"RawInline","c":["html","<sup>"]}]}
        ]);
        let mut value = blocks.clone();
        strip_value(&mut value);
        assert_eq!(value, blocks);
    }
}
