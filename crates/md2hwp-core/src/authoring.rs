// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Euisoon Ahn
//! Strict manuscript checks in addition to the converter's permissive parsing.

use std::collections::BTreeMap;

use regex::Regex;
use serde_json::Value;

use crate::PandocDocument;
use crate::ir::Document;

#[derive(Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub location: String,
    pub code: &'static str,
    pub message: String,
    pub warning: bool,
}

fn diagnostic(
    location: impl Into<String>,
    code: &'static str,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic {
        location: location.into(),
        code,
        message: message.into(),
        warning: false,
    }
}

fn regex(pattern: &str) -> Regex {
    Regex::new(pattern).expect("built-in authoring pattern")
}

/// The source checks that conversion enforces as well as validate. Pandoc reads
/// an HTML comment line inside a table without an error but changes the table:
/// a comment line between rows ends it, and a commented-out row stays as cells.
/// Every other source check stays validate-only.
pub fn check_conversion_source(source: &str) -> Vec<Diagnostic> {
    check_source(source)
        .into_iter()
        .filter(|d| !d.warning && d.code == "table_comment")
        .collect()
}

/// Detect syntax that may disappear or become literal text during parsing.
/// This is intentionally a source supplement, not an alternative Markdown parser.
pub fn check_source(source: &str) -> Vec<Diagnostic> {
    let mut result = Vec::new();
    let lines: Vec<_> = source.lines().collect();
    let mut fence: Option<(char, usize, usize)> = None;
    let mut yaml = lines.first().is_some_and(|s| s.trim() == "---");
    let mut yaml_closed = !yaml;
    let mut definitions = BTreeMap::new();
    let mut metadata_keys = BTreeMap::new();
    let mut references = Vec::new();
    let note = regex(r"\[\^([^\]\r\n]+)\]");
    let definition = regex(r"^ {0,3}\[\^([^\]]+)\]:");
    let bullets = regex(r"^\s*[+*]\s+");
    let number = regex(r"^\s*\d+[.)]\s+");
    let heading = regex(r"^ {0,3}#{1,6}\s+(?:\d+(?:\.\d+)*[.)]?\s+|제\s*\d+\s*[장절])");
    let metadata_key = regex(r"^([^\s:#][^:]*):\s*(.*)$");
    // An HTML comment at the start of a line, list item or footnote definition
    // is an HTML block. It runs to the first line containing '-->', even
    // across blank lines, and is skipped like the comments Pandoc drops.
    let comment_block =
        regex(r"^ {0,3}(?:(?:[-+*]|\d{1,9}[.)])[ \t]+|\[\^[^\]\r\n]+\]:[ \t]*)*<!--");
    // Lines that end a paragraph, and with it an unclosed inline comment.
    let interrupts = regex(
        r"^ {0,3}(?:#{1,6}(?:[ \t]|$)|[-+*][ \t]|1[.)][ \t]|\[\^[^\]\r\n]+\]:|>|```|~~~|<!--)",
    );
    // Start line and the indentation that keeps the comment's container open.
    let mut comment: Option<(usize, usize)> = None;
    let mut inline_comment = false;
    let mut table_comment: Option<usize> = None;
    let mut table_columns = None;
    let mut previous_row: Option<(usize, usize)> = None;
    for (index, line) in lines.iter().enumerate() {
        let line_number = index + 1;
        let location = format!("line {line_number}");
        let trimmed = line.trim();
        if yaml {
            if index > 0 && matches!(trimmed, "---" | "...") {
                yaml = false;
                yaml_closed = true;
            } else if let Some(capture) = metadata_key.captures(line) {
                let key = capture[1].trim().to_owned();
                if metadata_keys.insert(key.clone(), line_number).is_some() {
                    result.push(diagnostic(
                        &location,
                        "duplicate_metadata",
                        format!("duplicate metadata key '{key}'"),
                    ));
                }
                if capture[2].starts_with(['|', '>']) {
                    result.push(diagnostic(
                        &location,
                        "multiline_metadata",
                        "metadata values must be single-line strings",
                    ));
                }
            }
            if let Some(value) = trimmed.strip_prefix("date:") {
                let value = value.trim();
                if !value.starts_with(['\'', '"']) {
                    result.push(diagnostic(&location, "quoted_date", "quote the date value"));
                }
            }
            continue;
        }
        let fence_candidate = line.trim_start();
        let indent = line.len() - fence_candidate.len();
        if let Some((marker, count, _)) = fence {
            if indent <= 3
                && fence_candidate.starts_with(marker)
                && fence_candidate.chars().take_while(|c| *c == marker).count() >= count
                && fence_candidate.trim_matches(marker).trim().is_empty()
            {
                fence = None;
            }
            continue;
        }
        if let Some((start, indent)) = comment {
            if indent > 0 && !trimmed.is_empty() && line_indent(line) < indent {
                // A less indented line closes the list item or footnote
                // first, so Pandoc keeps the unclosed comment as raw HTML.
                result.push(unclosed_comment(start));
                comment = None;
            } else {
                if line.contains("-->") {
                    comment = None;
                }
                continue;
            }
        }
        let continues_comment = inline_comment;
        let mut comment_start = comment_block
            .find(line)
            .filter(|_| !continues_comment)
            .map(|found| found.end() - "<!--".len());
        if comment_start.is_some() && table_columns.is_some() && unescaped(line).contains('|') {
            // Inside a table a line with '|' is a row, not an HTML block.
            result.push(diagnostic(
                &location,
                "table_comment",
                "a line with '|' inside a table is a table row, so an HTML comment there stays as cell text; delete the row instead of commenting it out",
            ));
            comment_start = None;
        }
        if let Some(start) = comment_start {
            if !line[start..].contains("-->") {
                comment = Some((line_number, container_indent(&line[..start])));
            }
            if table_columns.is_some() {
                table_comment = Some(line_number);
            }
        } else {
            if let Some(comment_line) = table_comment.take()
                && unescaped(line).contains('|')
                && !lines.get(index + 1).is_some_and(|next| {
                    unescaped(next).contains('|') && is_separator(&pipe_cells(next))
                })
            {
                result.push(diagnostic(
                    format!("line {comment_line}"),
                    "table_comment",
                    "an HTML comment line between table rows ends the table, so the rows after it become paragraph text; put the comment before or after the table",
                ));
            }
            if !continues_comment {
                if indent <= 3 && fence_candidate.starts_with(['`', '~']) {
                    let marker = fence_candidate.chars().next().unwrap();
                    let count = fence_candidate.chars().take_while(|c| *c == marker).count();
                    if count >= 3 {
                        fence = Some((marker, count, line_number));
                        if !fence_candidate[count..].trim().is_empty() {
                            result.push(diagnostic(
                                &location,
                                "box_attributes",
                                "box fences cannot have a language or attributes",
                            ));
                        }
                        table_columns = None;
                        previous_row = None;
                        continue;
                    }
                }
                // Indented verbatim content is checked as a CodeBlock by Pandoc, not prose.
                // Footnote continuation paragraphs remain covered by AST/IR validation.
                if line.starts_with("    ") || line.starts_with('\t') {
                    continue;
                }
            }
        }
        // Comment text is not checked. A comment line keeps only its list
        // marker or footnote label; a continued inline comment hides the
        // line start.
        let text = match comment_start {
            Some(start) => line[..start].to_owned(),
            None => {
                strip_inline_comments(line, &lines[index + 1..], &mut inline_comment, &interrupts)
            }
        };
        let visible = unescaped(&text);

        if !continues_comment {
            if bullets.is_match(line) && !trimmed.chars().all(|c| c == '*' || c.is_whitespace()) {
                result.push(diagnostic(
                    &location,
                    "bullet_marker",
                    "use '-' for bullet lists",
                ));
            }
            if number.is_match(line)
                && trimmed
                    .split(['.', ')'])
                    .next()
                    .is_some_and(|s| s.bytes().all(|b| b == b'0'))
            {
                result.push(diagnostic(
                    &location,
                    "list_start",
                    "ordered lists require a positive decimal start",
                ));
            }
            if heading.is_match(line) {
                let mut issue = diagnostic(
                    &location,
                    "manual_heading_number",
                    "possible duplicate heading number; use generated numbering",
                );
                issue.warning = true;
                result.push(issue);
            }
        }
        if visible.contains("^[") {
            result.push(diagnostic(
                &location,
                "inline_note",
                "use reference-style footnotes, not ^[...]",
            ));
        }
        let mut definition_end = 0;
        if !continues_comment && let Some(capture) = definition.captures(&visible) {
            definition_end = capture.get(0).map_or(0, |m| m.end());
            let name = capture[1].to_owned();
            if definitions.insert(name.clone(), line_number).is_some() {
                result.push(diagnostic(
                    &location,
                    "duplicate_footnote",
                    format!("duplicate footnote definition '{name}'"),
                ));
            }
        }
        for capture in note.captures_iter(&visible[definition_end..]) {
            references.push((capture[1].to_owned(), line_number));
        }
        if comment_start.is_some() || continues_comment {
            // A comment line ends a table; a paragraph line cannot be a row.
            previous_row = None;
            table_columns = None;
            continue;
        }
        // Pipe-row counts are checked before Pandoc can pad/truncate rows.
        // GFM splits cells before reading inline HTML, so comments count too.
        if unescaped(line).contains('|') {
            let cells = pipe_cells(line);
            let separator = is_separator(&cells);
            if separator {
                if let Some((header_count, header_line)) = previous_row {
                    if header_count != cells.len() {
                        result.push(diagnostic(
                            format!("line {header_line}"),
                            "table_columns",
                            "header and separator column counts differ",
                        ));
                    }
                    table_columns = Some(cells.len());
                }
            } else if let Some(expected) = table_columns
                && cells.len() != expected
            {
                result.push(diagnostic(
                    &location,
                    "table_columns",
                    format!("expected {expected} cells, found {}", cells.len()),
                ));
            }
            previous_row = Some((cells.len(), line_number));
        } else {
            if table_columns.is_some() && !trimmed.is_empty() {
                result.push(diagnostic(&location, "table_boundary", "separate a table from following prose with a blank line; check for missing cell separators"));
            }
            previous_row = None;
            table_columns = None;
        }
    }
    if let Some((_, _, line)) = fence {
        result.push(diagnostic(
            format!("line {line}"),
            "unclosed_box",
            "close the box fence explicitly",
        ));
    }
    if let Some((line, _)) = comment {
        result.push(unclosed_comment(line));
    }
    if !yaml_closed {
        result.push(diagnostic(
            "line 1",
            "unclosed_metadata",
            "close YAML front matter",
        ));
    }
    for (name, line) in references {
        if !definitions.contains_key(&name) {
            result.push(diagnostic(
                format!("line {line}"),
                "undefined_footnote",
                format!("undefined footnote '{name}'"),
            ));
        }
    }
    result
}

fn unescaped(text: &str) -> String {
    let mut result = String::new();
    let mut chars = text.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(next) = chars.next() {
                if next.is_ascii_punctuation() {
                    result.push(' ');
                } else {
                    result.push(ch);
                    result.push(next);
                }
            }
        } else {
            result.push(ch);
        }
    }
    result
}

fn unclosed_comment(line: usize) -> Diagnostic {
    diagnostic(
        format!("line {line}"),
        "unclosed_comment",
        "close the HTML comment with '-->'; inside a list item or footnote, indent its lines like the item's content",
    )
}

/// Leading indentation in columns, with tabs to the next multiple of four.
fn line_indent(line: &str) -> usize {
    let mut column = 0;
    for ch in line.chars() {
        match ch {
            ' ' => column += 1,
            '\t' => column += 4 - column % 4,
            _ => break,
        }
    }
    column
}

/// Indentation that continues a comment opened after `prefix`: 0 at the top
/// level, the content column after a list marker, and four columns past a
/// footnote label.
fn container_indent(prefix: &str) -> usize {
    let mut indent = 0;
    let mut column = line_indent(prefix);
    let mut rest = prefix.trim_start();
    while !rest.is_empty() {
        let label = rest.starts_with("[^");
        let marker = if label {
            rest.find("]:").map_or(rest.len(), |end| end + 2)
        } else {
            rest.find([' ', '\t']).unwrap_or(rest.len())
        };
        let content = rest[marker..].trim_start();
        let spaces = line_indent(&rest[marker..]);
        indent = if label {
            column + 4
        } else {
            column + marker + spaces
        };
        column += marker + spaces;
        rest = content;
    }
    indent
}

/// Position of the first `<!--` that is not escaped with a backslash.
fn comment_open(text: &str) -> Option<usize> {
    let mut from = 0;
    while let Some(found) = text[from..].find("<!--") {
        let start = from + found;
        let backslashes = text[..start]
            .bytes()
            .rev()
            .take_while(|b| *b == b'\\')
            .count();
        if backslashes % 2 == 0 {
            return Some(start);
        }
        from = start + 1;
    }
    None
}

/// Removes inline HTML comments from one prose line. `open` carries a comment
/// continued from an earlier line. CommonMark keeps an unclosed `<!--` as
/// literal text, so a comment opened here continues only when a later line of
/// the same paragraph closes it; otherwise the text stays and the AST check
/// reports the literal `<!--`.
fn strip_inline_comments(
    line: &str,
    following: &[&str],
    open: &mut bool,
    interrupts: &Regex,
) -> String {
    let mut text = String::new();
    let mut rest = line;
    if *open {
        let Some(end) = rest.find("-->") else {
            return text;
        };
        rest = &rest[end + "-->".len()..];
        *open = false;
    }
    while let Some(start) = comment_open(rest) {
        text.push_str(&rest[..start]);
        rest = &rest[start..];
        if let Some(length) = crate::html_comment::comment_len(rest) {
            rest = &rest[length..];
            continue;
        }
        *open = following
            .iter()
            .take_while(|line| !line.trim().is_empty() && !interrupts.is_match(line))
            .any(|line| line.contains("-->"));
        if *open {
            return text;
        }
        break;
    }
    text.push_str(rest);
    text
}

fn is_separator(cells: &[String]) -> bool {
    cells.iter().all(|c| {
        let c = c.trim().trim_matches(':');
        !c.is_empty() && c.chars().all(|ch| ch == '-')
    })
}

fn pipe_cells(line: &str) -> Vec<String> {
    let visible = unescaped(line);
    let visible = visible.trim();
    let visible = visible.strip_prefix('|').unwrap_or(visible);
    let visible = visible.strip_suffix('|').unwrap_or(visible);
    visible.split('|').map(str::to_owned).collect()
}

/// Inspect distinctions lost during normalization (soft paragraph boundaries
/// and footnotes in number-reference labels).
pub fn check_pandoc(document: &PandocDocument) -> Vec<Diagnostic> {
    // Inspect the same comment-free AST that normalization reads.
    let mut document = document.clone();
    crate::html_comment::strip(&mut document);
    let mut result = Vec::new();
    for (key, value) in &document.metadata {
        if contains_constructor(value, "SoftBreak") {
            result.push(diagnostic(
                format!("/meta/{key}"),
                "multiline_metadata",
                "metadata values must be single-line strings",
            ));
        }
    }
    for (index, block) in document.blocks.iter().enumerate() {
        visit_pandoc(block, &format!("/blocks/{index}"), &mut result);
    }
    result
}

fn contains_constructor(value: &Value, constructor: &str) -> bool {
    match value {
        Value::Object(map) => {
            map.get("t").and_then(Value::as_str) == Some(constructor)
                || map.values().any(|v| contains_constructor(v, constructor))
        }
        Value::Array(array) => array.iter().any(|v| contains_constructor(v, constructor)),
        _ => false,
    }
}

fn visit_pandoc(value: &Value, path: &str, result: &mut Vec<Diagnostic>) {
    match value {
        Value::Object(map) => {
            if map.get("t").and_then(Value::as_str) == Some("CodeBlock") {
                return;
            }
            if map.get("t").and_then(Value::as_str) == Some("Link")
                && value["c"][2][0]
                    .as_str()
                    .is_some_and(|target| target.starts_with('#'))
                && contains_constructor(&value["c"][1], "Note")
            {
                result.push(diagnostic(
                    path,
                    "reference_label_note",
                    "put footnotes after number references, not inside their labels",
                ));
            }
            if matches!(map.get("t").and_then(Value::as_str), Some("Para" | "Plain"))
                && let Some(inlines) = value["c"].as_array()
            {
                for (index, pair) in inlines.windows(2).enumerate() {
                    if pair[0]["t"].as_str() == Some("SoftBreak")
                        && let Some(content) = pair[1]["c"].as_str()
                        && reserved_label(content).is_some()
                    {
                        result.push(diagnostic(
                            format!("{path}/c/{}", index + 1),
                            "paragraph_boundary",
                            "put a blank line before each object note/caption paragraph",
                        ));
                    }
                }
            }
            for (key, child) in map {
                visit_pandoc(child, &format!("{path}/{key}"), result);
            }
        }
        Value::Array(array) => {
            for (index, child) in array.iter().enumerate() {
                visit_pandoc(child, &format!("{path}/{index}"), result);
            }
        }
        _ => {}
    }
}

/// Recognize reserved-looking labels without reserving arbitrary prose words.
fn reserved_label(text: &str) -> Option<(&str, bool)> {
    let (prefix, _) = text.split_once(':')?;
    let candidate = prefix.trim();
    let base = candidate
        .trim_end_matches(|c: char| c == '.' || c.is_numeric())
        .trim_end();
    if crate::source_prefix::is_supported(candidate) || crate::source_prefix::is_supported(base) {
        return Some((candidate, false));
    }
    if matches!(candidate, "표" | "Table" | "table" | "") {
        return Some((candidate, true));
    }
    if candidate == "제목" {
        return Some((candidate, false));
    }
    None
}

/// Normalization consumes valid object notes/captions. Any remaining reserved
/// paragraph is misplaced or malformed, including in lists and footnote bodies.
pub fn check_document(document: &Document) -> Vec<Diagnostic> {
    let mut result = Vec::new();
    let value = serde_json::to_value(document).expect("IR serialization");
    visit(&value, "", &mut result);
    result
}

fn text(inlines: &Value) -> String {
    let mut result = String::new();
    if let Some(inlines) = inlines.as_array() {
        for inline in inlines {
            match inline["type"].as_str() {
                Some("text") => result.push_str(inline["value"].as_str().unwrap_or_default()),
                Some("space" | "line_break") => result.push(' '),
                Some("strong" | "emph" | "strikeout" | "superscript" | "subscript" | "link") => {
                    result.push_str(&text(&inline["inlines"]))
                }
                _ => {}
            }
        }
    }
    result
}

fn visit(value: &Value, path: &str, result: &mut Vec<Diagnostic>) {
    match value {
        Value::Object(map) => {
            if map.get("type").and_then(Value::as_str) == Some("verbatim_block") {
                // Body literals are excluded; attached notes still require checks.
                if let Some(source) = map.get("source") {
                    visit(source, &format!("{path}/source"), result);
                }
                return;
            }
            if map.get("type").and_then(Value::as_str) == Some("paragraph") {
                let content = text(&value["inlines"]);
                if let Some((label, caption)) = reserved_label(&content) {
                    result.push(diagnostic(path, if caption { "orphan_caption" } else { "orphan_object_note" },
                        format!("reserved paragraph '{label}:' was not attached to an eligible object; check placement, label and ': ' syntax")));
                }
            }
            if let Some("text") = map.get("type").and_then(Value::as_str) {
                let content = value["value"].as_str().unwrap_or_default();
                if content.contains("{{md2hwp:") {
                    result.push(diagnostic(
                        path,
                        "template_tag",
                        "template tags do not belong in manuscripts",
                    ));
                }
                if content.contains("[@") {
                    result.push(diagnostic(
                        path,
                        "citation",
                        "[@...] citations are unsupported; use internal number-reference links",
                    ));
                }
                if content.contains("^[") {
                    result.push(diagnostic(
                        path,
                        "inline_note",
                        "inline notes are unsupported",
                    ));
                }
                if content.contains("[^") {
                    result.push(diagnostic(path, "unresolved_footnote_syntax", "footnote-like text remained literal; check definition and syntax, or put the example in a box"));
                }
                if content.contains("{#") {
                    result.push(diagnostic(path, "unattached_id", "ID syntax remained literal; attach IDs only to headings, figures, or the standalone line immediately before a table"));
                }
                if content.contains("<!--") {
                    result.push(diagnostic(path, "unclosed_comment", "'<!--' remained literal text; close an HTML comment with '-->' before the next blank line or block, or start it on a line of its own"));
                }
            }
            for (key, child) in map {
                visit(child, &format!("{path}/{key}"), result);
            }
        }
        Value::Array(array) => {
            for (index, child) in array.iter().enumerate() {
                visit(child, &format!("{path}/{index}"), result);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ValidationLimits, load_builtin_rules, normalize_pandoc, read_pandoc_json};

    #[test]
    fn source_checks_parser_fallbacks_and_preserves_box_literals() {
        let issues = check_source(
            "```\n주 : literal\n[^missing]\n[@literal]\n```\n\n[^a]: first\n\n[^a]: second\n\n[^missing]\n\n* bullet\n\n0. zero\n\n```\n",
        );
        assert_eq!(
            issues.iter().map(|d| d.code).collect::<Vec<_>>(),
            vec![
                "duplicate_footnote",
                "bullet_marker",
                "list_start",
                "unclosed_box",
                "undefined_footnote"
            ]
        );
    }

    #[test]
    fn checks_rows_before_padding_and_honors_escaped_pipes() {
        assert!(check_source("| A | B |\n| --- | --- |\n| a\\|b | c |\n").is_empty());
        assert_eq!(
            check_source("| A | B |\n| --- | --- |\n| a |\n")[0].code,
            "table_columns"
        );
    }

    #[test]
    fn escaped_note_is_not_a_reference() {
        assert!(check_source(r"\[^literal] and \^[literal]").is_empty());
    }

    fn codes(source: &str) -> Vec<(&'static str, String)> {
        check_source(source)
            .into_iter()
            .map(|d| (d.code, d.location))
            .collect()
    }

    #[test]
    fn source_checks_skip_html_comments() {
        for source in [
            // Block comments run across blank lines, fences and indented lines.
            "<!-- 예시\n* 별표\n\n```text\n[^a]: 하나\n\n    [^a]: 둘\n[^missing] ^[인라인]\n| A | B |\n| --- | --- |\n| a |\n-->\n",
            "  <!-- 한 줄 --> <!-- 두 번째 -->\n",
            // Inline comments, also continued on the next paragraph line.
            "본문 <!-- [^missing] ^[x] --> 계속 <!-- 여러\n[^missing] | ^[y] --> 끝\n",
            // After a list marker or footnote label the comment is a block too.
            "- <!-- 목록 안 예시\n\n  [^missing]\n  -->\n",
            "본문[^a]\n\n[^a]: <!-- 각주 예시\n\n    [^missing]\n    -->\n",
            "1. <!-- 목록 예시\n   [^missing]\n   -->\n",
            // A row after a comment that starts a new table is fine.
            "| a | b |\n| --- | --- |\n| 1 | 2 |\n<!-- c -->\n| x | y |\n| --- | --- |\n",
        ] {
            assert_eq!(codes(source), vec![], "{source:?}");
        }
    }

    #[test]
    fn source_checks_report_comment_mistakes() {
        assert_eq!(
            codes("본문\n\n<!-- 닫지 않은 주석\n\n내용\n"),
            vec![("unclosed_comment", "line 3".to_owned())]
        );
        // A less indented line closes the footnote before '-->'.
        assert_eq!(
            codes("본문[^a]\n\n[^a]: <!-- 각주 예시\n\n[^missing]\n-->\n"),
            vec![
                ("unclosed_comment", "line 3".to_owned()),
                ("undefined_footnote", "line 5".to_owned())
            ]
        );
        assert_eq!(
            codes("- <!-- 목록 예시\n [^missing]\n-->\n"),
            vec![
                ("unclosed_comment", "line 1".to_owned()),
                ("undefined_footnote", "line 2".to_owned())
            ]
        );
        // Without '|' a comment line ends the table; with '|' it is a row.
        assert_eq!(
            codes("| a | b |\n| --- | --- |\n| 1 | 2 |\n<!-- 메모\n| 3 | 4 |\n-->\n| 5 | 6 |\n"),
            vec![("table_comment", "line 4".to_owned())]
        );
        assert_eq!(
            codes("| a | b |\n| --- | --- |\n| 1 | 2 |\n<!-- | 3 | 4 | -->\n| 5 | 6 |\n"),
            vec![
                ("table_comment", "line 4".to_owned()),
                ("table_columns", "line 4".to_owned())
            ]
        );
        // GFM splits cells before it reads inline HTML.
        assert_eq!(
            codes("| a | b |\n| --- | --- |\n| 1 <!-- x | y --> | 2 |\n"),
            vec![("table_columns", "line 3".to_owned())]
        );
        // Conversion rejects only the table comments, not the other checks.
        let conversion = |source| {
            check_conversion_source(source)
                .into_iter()
                .map(|d| (d.code, d.location))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            conversion("| a | b |\n| --- | --- |\n| 1 | 2 |\n<!-- 메모 -->\n| 3 | 4 |\n"),
            vec![("table_comment", "line 4".to_owned())]
        );
        assert_eq!(
            conversion("| a | b |\n| --- | --- |\n| 1 | 2 |\n<!-- | 3 | 4 | -->\n| 5 | 6 |\n"),
            vec![("table_comment", "line 4".to_owned())]
        );
        for source in [
            "| a | b |\n| --- | --- |\n| 1 <!-- 메모 --> | 2 |\n",
            "<!-- 메모 -->\n\n| a | b |\n| --- | --- |\n| 1 | 2 |\n\n<!-- 메모 -->\n",
            "| a | b |\n| --- | --- |\n| 1 | 2 | 3 |\n\n본문 <!-- 열림\n\n[^missing]\n",
        ] {
            assert!(conversion(source).is_empty(), "{source}");
        }
        // A comment left open is literal text; following lines are checked,
        // and a list item ends the paragraph before '-->'.
        assert_eq!(
            codes("본문 <!-- 열림\n\n[^missing]\n"),
            vec![("undefined_footnote", "line 3".to_owned())]
        );
        assert_eq!(
            codes("본문 <!-- 열림\n- [^missing]\n-->\n"),
            vec![("undefined_footnote", "line 2".to_owned())]
        );
        assert_eq!(
            codes(r"\<!-- [^missing] -->"),
            vec![("undefined_footnote", "line 1".to_owned())]
        );
        // A list marker before a comment is still checked.
        assert_eq!(
            codes("* <!-- 주석 -->\n"),
            vec![("bullet_marker", "line 1".to_owned())]
        );
    }

    fn normalized(blocks: Value) -> Document {
        let bytes = serde_json::to_vec(
            &serde_json::json!({"pandoc-api-version": [1,23,1,2], "meta": {}, "blocks": blocks}),
        )
        .unwrap();
        let limits = ValidationLimits::default();
        normalize_pandoc(
            read_pandoc_json(&bytes, &limits).unwrap(),
            &load_builtin_rules().unwrap(),
            "commonmark",
            &limits,
        )
        .unwrap()
        .into_document()
    }

    #[test]
    fn attached_notes_are_allowed_but_orphans_and_malformed_labels_fail() {
        let doc = normalized(serde_json::json!([
            {"t":"CodeBlock","c":[["",[],[]],"주: literal"]},
            {"t":"Para","c":[{"t":"Str","c":"주:"},{"t":"Space"},{"t":"Str","c":"attached"}]},
            {"t":"Para","c":[{"t":"Str","c":"body"}]},
            {"t":"Para","c":[{"t":"Str","c":"주:"},{"t":"Space"},{"t":"Str","c":"orphan"}]},
            {"t":"Para","c":[{"t":"Str","c":"source1:"},{"t":"Space"},{"t":"Str","c":"bad"}]},
            {"t":"Para","c":[{"t":"Str","c":"표:"},{"t":"Space"},{"t":"Str","c":"orphan"}]}
        ]));
        let issues = check_document(&doc);
        assert_eq!(issues.len(), 3);
        assert_eq!(issues[2].code, "orphan_caption");
    }

    #[test]
    fn prose_words_are_not_reserved() {
        assert!(reserved_label("주의 사항을 확인: 설명").is_none());
        assert!(reserved_label("https://example.com").is_none());
    }

    #[test]
    fn catches_metadata_fallback_and_empty_edge_cells() {
        let issues = check_source(
            "---\ntitle: one\ntitle: two\ndate: 2026-10-11\nsubtitle: |\n  multiple lines\n---\n",
        );
        assert_eq!(
            issues.iter().map(|d| d.code).collect::<Vec<_>>(),
            vec!["duplicate_metadata", "quoted_date", "multiline_metadata"]
        );
        assert!(check_source("| A | B |\n| --- | --- |\n|| value |\n").is_empty());
        assert_eq!(
            check_source("A | B\n--- | ---\none\n")[0].code,
            "table_boundary"
        );
        assert_eq!(
            check_source("A | B\n--- | ---\none | two | three\n")[0].code,
            "table_columns"
        );
    }

    #[test]
    fn detects_merged_notes_and_footnotes_lost_in_reference_labels() {
        let value = serde_json::json!({"pandoc-api-version":[1,23,1,2], "meta":{}, "blocks":[
            {"t":"Para","c":[{"t":"Str","c":"body"},{"t":"SoftBreak"},{"t":"Str","c":"주:"},{"t":"Space"},{"t":"Str","c":"note"}]},
            {"t":"Para","c":[{"t":"Link","c":[["",[],[]],[{"t":"Note","c":[{"t":"Para","c":[{"t":"Str","c":"body"}]}]}],["#target",""]]}]}
        ]});
        let document = read_pandoc_json(
            &serde_json::to_vec(&value).unwrap(),
            &ValidationLimits::default(),
        )
        .unwrap();
        let issues = check_pandoc(&document);
        assert_eq!(
            issues.iter().map(|d| d.code).collect::<Vec<_>>(),
            vec!["paragraph_boundary", "reference_label_note"]
        );
    }
}
