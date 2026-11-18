//! HTML comments are author notes: conversion and strict checks behave as if
//! they were absent. The fixtures were produced by Pandoc 3.10.1 from
//! tests/fixtures/html-comments.md and its comment-free twin
//! tests/fixtures/html-comments-removed.md.

use std::collections::BTreeSet;

use md2hwp_core::authoring::{check_document, check_pandoc, check_source};
use md2hwp_core::ir::{Block, Inline};
use md2hwp_core::{
    NormalizeError, ValidatedDocument, ValidationLimits, load_builtin_rules, normalize_pandoc,
    read_pandoc_json, write_ir,
};
use serde_json::{Value, json};

const SOURCE: &str = include_str!("../../../tests/fixtures/html-comments.md");
const COMMENTED: &[u8] =
    include_bytes!("../../../tests/fixtures/pandoc-json/commonmark-comments-v0.6.json");
const REMOVED: &[u8] =
    include_bytes!("../../../tests/fixtures/pandoc-json/commonmark-comments-removed-v0.6.json");
const READER: &str = "commonmark+yaml_metadata_block+footnotes+attributes+implicit_figures+pipe_tables+tex_math_dollars+strikeout";

fn normalize_bytes(bytes: &[u8]) -> Result<ValidatedDocument, NormalizeError> {
    let limits = ValidationLimits::default();
    normalize_pandoc(
        read_pandoc_json(bytes, &limits).unwrap(),
        &load_builtin_rules().unwrap(),
        READER,
        &limits,
    )
}

fn normalize_blocks(blocks: Value) -> Result<ValidatedDocument, NormalizeError> {
    normalize_bytes(
        &serde_json::to_vec(&json!({"pandoc-api-version":[1,23,1,2],"meta":{},"blocks":blocks}))
            .unwrap(),
    )
}

fn raw_block(text: &str) -> Value {
    json!({"t":"RawBlock","c":["html",text]})
}

fn raw_inline(text: &str) -> Value {
    json!({"t":"RawInline","c":["html",text]})
}

fn str_node(text: &str) -> Value {
    json!({"t":"Str","c":text})
}

#[test]
fn comments_normalize_as_if_they_were_absent() {
    let commented = normalize_bytes(COMMENTED).unwrap();
    assert_eq!(commented, normalize_bytes(REMOVED).unwrap());

    // Captions and object notes attach across comment blocks.
    let blocks = &commented.as_document().blocks;
    assert_eq!(blocks.len(), 7);
    let Block::Table {
        id,
        caption: Some(_),
        source: Some(table_notes),
        header,
        ..
    } = &blocks[3]
    else {
        panic!("table with caption and notes: {:?}", blocks[3]);
    };
    assert_eq!(id.as_deref(), Some("regional"));
    assert_eq!(table_notes.len(), 2);
    assert_eq!(
        header[1],
        vec![Inline::Text {
            value: "수량".into()
        }]
    );
    assert!(matches!(&blocks[4], Block::Figure { source: Some(notes), .. } if notes.len() == 1));
    let Block::VerbatimBlock {
        lines,
        source: Some(_),
    } = &blocks[5]
    else {
        panic!("box with note: {:?}", blocks[5]);
    };
    // Comment syntax inside a box is box text, not a comment.
    assert_eq!(
        lines[0],
        "제목: 글상자 <!-- 글상자 안의 주석 표기는 글자로 남는다 -->"
    );
    let serialized = String::from_utf8(write_ir(&commented).unwrap()).unwrap();
    assert_eq!(serialized.matches("<!--").count(), 1);
    assert_eq!(
        commented.as_document().metadata.text("subtitle"),
        Some("부제")
    );
}

#[test]
fn strict_checks_ignore_comment_contents() {
    assert_eq!(check_source(SOURCE), vec![]);
    let limits = ValidationLimits::default();
    assert_eq!(
        check_pandoc(&read_pandoc_json(COMMENTED, &limits).unwrap()),
        vec![]
    );
    let ir = normalize_bytes(COMMENTED).unwrap();
    assert_eq!(check_document(ir.as_document()), vec![]);

    // The same example text outside a comment is diagnosed, so the empty
    // result above comes from skipping the comment.
    let start = SOURCE.find("<!-- 예시").unwrap();
    let example = &SOURCE[start..];
    let example = &example[example.find('\n').unwrap() + 1..example.find("\n-->").unwrap()];
    let codes: BTreeSet<_> = check_source(example).iter().map(|d| d.code).collect();
    for code in [
        "manual_heading_number",
        "bullet_marker",
        "list_start",
        "inline_note",
        "table_columns",
        "duplicate_footnote",
        "undefined_footnote",
        "box_attributes",
        "unclosed_box",
    ] {
        assert!(codes.contains(code), "{code} missing from {codes:?}");
    }
}

#[test]
fn comments_in_cells_lists_notes_and_paragraphs_disappear() {
    let cell =
        |inlines: Value| json!([["",[],[]],{"t":"AlignDefault"},1,1,[{"t":"Plain","c":inlines}]]);
    let table = json!({"t":"Table","c":[["",[],[]],[null,[]],
        [[{"t":"AlignDefault"},{"t":"ColWidthDefault"}],[{"t":"AlignDefault"},{"t":"ColWidthDefault"}]],
        [["",[],[]],[[["",[],[]],[cell(json!([str_node("가")])), cell(json!([raw_inline("<!-- 빈 칸 -->")]))]]]],
        [[["",[],[]],0,[],[[["",[],[]],[cell(json!([str_node("1")])), cell(json!([str_node("2")]))]]]]],
        [["",[],[]],[]]]});
    let document = normalize_blocks(json!([
        // A paragraph that holds only a comment is dropped.
        {"t":"Para","c":[raw_inline("<!-- 주석 -->")]},
        table,
        // - 항목\n  <!-- 블록 주석 -->
        {"t":"BulletList","c":[[{"t":"Plain","c":[str_node("항목")]}, raw_block("<!-- 블록 주석 -->\n")]]},
        // 본문[^1] with [^1]: 각주 <!-- c --> 내용 and an indented block comment.
        {"t":"Para","c":[str_node("본문"),{"t":"Note","c":[
            {"t":"Para","c":[str_node("각주"),{"t":"Space"},raw_inline("<!-- c -->"),{"t":"Space"},str_node("내용")]},
            raw_block("<!-- 각주 안 블록 -->\n")
        ]}]}
    ]))
    .unwrap()
    .into_document();
    assert_eq!(document.blocks.len(), 3);
    let Block::Table { header, .. } = &document.blocks[0] else {
        panic!("table")
    };
    assert_eq!(header[1], vec![]);
    let Block::Paragraph { inlines } = &document.blocks[2] else {
        panic!("paragraph")
    };
    let Inline::Footnote { blocks } = &inlines[1] else {
        panic!("footnote")
    };
    assert_eq!(blocks.len(), 1);
}

#[test]
fn raw_html_besides_comments_is_still_rejected_with_reasons() {
    let comment_reason = "must contain only comments";
    let cases = [
        // <!-- c --> 뒤 텍스트
        (
            json!([raw_block("<!-- c --> 뒤 텍스트\n")]),
            "/blocks/0",
            comment_reason,
        ),
        // An unclosed block comment runs to the end of the document.
        (
            json!([raw_block("<!-- 닫히지 않은 블록\n\n내용\n\n")]),
            "/blocks/0",
            comment_reason,
        ),
        // - <!-- c --> 항목
        (
            json!([{"t":"BulletList","c":[[raw_block("<!-- c --> 항목\n")]]}]),
            "/blocks/0/c/0/0",
            comment_reason,
        ),
        // 본문[^x] with [^x]: <!-- c --> 내용
        (
            json!([{"t":"Para","c":[str_node("본문"),{"t":"Note","c":[raw_block(" <!-- c --> 내용\n")]}]}]),
            "/blocks/0/c/1/c/0",
            comment_reason,
        ),
        (
            json!([raw_block("<div>\n")]),
            "/blocks/0",
            "only HTML comments are ignored",
        ),
        // {#t}\n<!-- c -->\n| table: the attribute attaches to the comment.
        (
            json!([{"t":"Div","c":[["t",[],[["wrapper","1"]]],[raw_block("<!-- c -->\n")]]}]),
            "/blocks/0",
            "move the comment above the attribute line",
        ),
        (
            json!([{"t":"Para","c":[str_node("a"),raw_inline("<b>"),str_node("b"),raw_inline("</b>")]}]),
            "/blocks/0/c/1",
            "HTML comments are ignored",
        ),
    ];
    for (blocks, path, message) in cases {
        let error = normalize_blocks(blocks.clone()).unwrap_err();
        assert_eq!(error.code, "unsupported_pandoc_node", "{blocks}");
        assert_eq!(error.path, path, "{blocks}");
        assert!(error.message.contains(message), "{error}");
    }
}

#[test]
fn literal_comment_openers_are_reported_by_strict_checks() {
    // 본문 <!-- 닫히지 않음\n\n다음: CommonMark keeps the unclosed opener as text.
    let document = normalize_blocks(json!([
        {"t":"Para","c":[str_node("본문"),{"t":"Space"},str_node("<!--"),{"t":"Space"},str_node("닫히지"),{"t":"Space"},str_node("않음")]},
        {"t":"Para","c":[str_node("다음")]}
    ]))
    .unwrap();
    let issues = check_document(document.as_document());
    assert_eq!(
        issues.iter().map(|d| d.code).collect::<Vec<_>>(),
        vec!["unclosed_comment"]
    );
}
