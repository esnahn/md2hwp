use md2hwp_core::ir::{
    Block, CrossReferenceKind, Document, FootnoteBlock, IR_VERSION, Inline, Metadata, SCHEMA_NAME,
    TableAlignment,
};
use md2hwp_core::{
    ValidationLimits, load_builtin_rules, normalize_pandoc, read_ir, read_pandoc_json, validate,
    write_ir,
};
use serde_json::{Value, json};

const FIXTURE: &[u8] =
    include_bytes!("../../../tests/fixtures/pandoc-json/commonmark-tables-v0.4.json");

fn normalize(blocks: Value) -> Result<md2hwp_core::ValidatedDocument, md2hwp_core::NormalizeError> {
    let limits = ValidationLimits::default();
    let bytes =
        serde_json::to_vec(&json!({"pandoc-api-version":[1,23,1,2],"meta":{},"blocks":blocks}))
            .unwrap();
    normalize_pandoc(
        read_pandoc_json(&bytes, &limits).unwrap(),
        &load_builtin_rules().unwrap(),
        "commonmark+pipe_tables",
        &limits,
    )
}

fn fixture_table() -> Value {
    let value: Value = serde_json::from_slice(FIXTURE).unwrap();
    value["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["t"] == "Table")
        .unwrap()
        .clone()
}

fn text(value: &str) -> Inline {
    Inline::Text {
        value: value.into(),
    }
}
fn caption(prefix: &str) -> Value {
    json!({"t":"Para","c":[{"t":"Str","c":prefix},{"t":"Space"},{"t":"Strong","c":[{"t":"Str","c":"제목"}]}]})
}
fn source() -> Value {
    json!({"t":"Para","c":[{"t":"Str","c":"출처:"},{"t":"Space"},{"t":"Emph","c":[{"t":"Str","c":"작성자"}]}]})
}
fn simple_table() -> Value {
    let mut table = fixture_table();
    table["c"][4][0][3] = json!([]);
    table
}
fn document(block: Block) -> Document {
    Document {
        schema: SCHEMA_NAME.into(),
        ir_version: IR_VERSION.into(),
        metadata: Metadata::default(),
        blocks: vec![block],
    }
}
fn direct_table() -> Block {
    Block::Table {
        columns: vec![TableAlignment::Default, TableAlignment::Center],
        header: vec![vec![text("항목")], vec![]],
        rows: vec![vec![vec![], vec![text("값")]]],
        caption: None,
        source: None,
    }
}

#[test]
fn actual_commonmark_tables_have_caption_source_alignment_empty_cells_and_references() {
    let limits = ValidationLimits::default();
    let ir = normalize_pandoc(
        read_pandoc_json(FIXTURE, &limits).unwrap(),
        &load_builtin_rules().unwrap(),
        "commonmark+pipe_tables",
        &limits,
    )
    .unwrap();
    let blocks = &ir.as_document().blocks;
    assert_eq!(blocks.len(), 5);
    let Block::Table {
        columns,
        header,
        rows,
        caption,
        source,
    } = &blocks[1]
    else {
        panic!("table");
    };
    assert_eq!(
        columns,
        &[
            TableAlignment::Left,
            TableAlignment::Center,
            TableAlignment::Right
        ]
    );
    assert_eq!(header.len(), 3);
    assert_eq!(rows.len(), 2);
    assert!(rows[1][1].is_empty());
    assert!(matches!(
        caption.as_ref().unwrap()[0],
        Inline::Strong { .. }
    ));
    assert!(matches!(
        source.as_ref().unwrap()[0].inlines[2],
        Inline::Emph { .. }
    ));
    assert!(rows[0][2].iter().any(|i| matches!(i,Inline::CrossReference { kind:CrossReferenceKind::HeadingNumber,target } if target == "chapter")));
    let Inline::Footnote { blocks: notes } = rows[1][2].last().unwrap() else {
        panic!("note");
    };
    let FootnoteBlock::Paragraph { inlines } = &notes[0];
    assert!(
        inlines
            .iter()
            .any(|i| matches!(i, Inline::CrossReference { .. }))
    );
    assert!(matches!(
        blocks[2],
        Block::Table {
            caption: Some(_),
            source: None,
            ..
        }
    ));
    assert!(
        matches!(blocks[4],Block::Table { caption:None,source:None,ref rows,.. } if rows.is_empty())
    );
    assert_eq!(read_ir(&write_ir(&ir).unwrap(), &limits).unwrap(), ir);
}

#[test]
fn supported_caption_prefixes_attach_above_or_below_without_losing_format() {
    for prefix in ["Table:", "table:", ":", "표:"] {
        for above in [true, false] {
            let table = simple_table();
            let blocks = if above {
                json!([caption(prefix), table, source()])
            } else {
                json!([table, caption(prefix), source()])
            };
            let ir = normalize(blocks).unwrap().into_document();
            assert_eq!(ir.blocks.len(), 1);
            let Block::Table {
                caption: Some(c),
                source: Some(s),
                ..
            } = &ir.blocks[0]
            else {
                panic!("caption/source");
            };
            assert_eq!(
                c,
                &[Inline::Strong {
                    inlines: vec![text("제목")]
                }]
            );
            assert_eq!(
                &s[0].inlines,
                &[Inline::Emph {
                    inlines: vec![text("작성자")]
                }]
            );
        }
    }
    let ir = normalize(json!([caption("표:")])).unwrap();
    assert!(matches!(
        ir.as_document().blocks[0],
        Block::Paragraph { .. }
    ));
}

#[test]
fn caption_ownership_rejects_both_sides_ambiguity_and_native_duplicates() {
    let table = simple_table();
    for blocks in [
        json!([caption("표:"), table.clone(), caption(":")]),
        json!([table.clone(), caption("표:"), table.clone()]),
    ] {
        let error = normalize(blocks).unwrap_err();
        assert!(error.message.contains("captions both") || error.message.contains("ambiguous"));
    }
    let mut native = table;
    native["c"][1][1] = json!([{"t":"Plain","c":[{"t":"Str","c":"제목"}]}]);
    let ir = normalize(json!([native.clone()])).unwrap();
    assert!(matches!(
        ir.as_document().blocks[0],
        Block::Table {
            caption: Some(_),
            ..
        }
    ));
    assert!(
        normalize(json!([caption("표:"), native]))
            .unwrap_err()
            .message
            .contains("native caption")
    );
}

#[test]
fn empty_or_unsupported_caption_and_source_content_is_not_silently_consumed() {
    for prefix in ["Table:", "table:", ":", "표:", "출처:"] {
        let empty = json!({"t":"Para","c":[{"t":"Str","c":prefix}]});
        assert!(normalize(json!([simple_table(), empty])).is_err());
        let code = json!({"t":"Para","c":[{"t":"Str","c":prefix},{"t":"Space"},{"t":"Code","c":[["",[],[]],"x"]}]});
        assert_eq!(
            normalize(json!([simple_table(), code]))
                .unwrap_err()
                .constructor
                .as_deref(),
            Some("Code")
        );
    }
}

#[test]
fn closed_pandoc_table_structure_rejects_unrepresented_features() {
    let cases = [
        ("/c/0/0", json!("table-id")),
        ("/c/0/1", json!(["class"])),
        ("/c/1/0", json!([])),
        (
            "/c/1/1",
            json!([{ "t":"Plain","c":[] },{ "t":"Plain","c":[] }]),
        ),
        ("/c/2/0/1", json!({"t":"ColWidth","c":0.2})),
        ("/c/3/1", json!([])),
        ("/c/4", json!([])),
        ("/c/4/0/1", json!(1)),
        ("/c/4/0/2", json!([[]])),
        ("/c/5/1", json!([[]])),
        ("/c/3/1/0/1/0/0/0", json!("cell-id")),
        ("/c/3/1/0/1/0/1", json!({"t":"AlignCenter"})),
        ("/c/3/1/0/1/0/2", json!(2)),
        ("/c/3/1/0/1/0/3", json!(2)),
        ("/c/3/1/0/1/0/4", json!([{ "t":"BulletList","c":[] }])),
        (
            "/c/3/1/0/1/0/4",
            json!([{ "t":"Plain","c":[] },{ "t":"Plain","c":[] }]),
        ),
    ];
    for (pointer, value) in cases {
        let mut table = simple_table();
        *table.pointer_mut(pointer).unwrap() = value;
        assert!(normalize(json!([table])).is_err(), "accepted {pointer}");
    }
    for pointer in ["/c/3/1", "/c/4"] {
        let mut table = simple_table();
        let values = table.pointer_mut(pointer).unwrap().as_array_mut().unwrap();
        values.push(values[0].clone());
        assert!(
            normalize(json!([table])).is_err(),
            "accepted multiple {pointer}"
        );
    }
    let mut table = simple_table();
    table["c"][3][1][0][1].as_array_mut().unwrap().pop();
    assert!(
        normalize(json!([table]))
            .unwrap_err()
            .message
            .contains("cell count")
    );
}

#[test]
fn table_cells_accept_rich_text_breaks_and_notes_but_caption_and_source_reject_notes() {
    let note = json!({"t":"Note","c":[{"t":"Para","c":[{"t":"Str","c":"주석"}]}]});
    let inlines =
        json!([{"t":"Strong","c":[{"t":"Str","c":"굵게"}]},{"t":"LineBreak"},note.clone()]);
    let mut table = simple_table();
    table["c"][3][1][0][1][0][4] = json!([{"t":"Para","c":inlines}]);
    normalize(json!([table.clone()])).unwrap();
    for prefix in ["Table:", "표:", "출처:"] {
        let paragraph = json!({"t":"Para","c":[{"t":"Str","c":prefix},{"t":"Space"},note]});
        assert!(
            normalize(json!([table.clone(), paragraph]))
                .unwrap_err()
                .message
                .contains("footnotes are allowed only")
        );
    }
}

#[test]
fn references_inside_table_headers_cells_and_notes_resolve_forward_to_actual_targets() {
    let mut table = simple_table();
    let link = json!({"t":"Link","c":[["",[],[]],[],["#figure",""]]});
    table["c"][3][1][0][1][0][4] = json!([{"t":"Plain","c":[link]}]);
    let image = json!({"t":"Para","c":[{"t":"Image","c":[["figure",[],[]],[{"t":"Str","c":"그림"}],["image.png",""]]}]});
    let ir = normalize(json!([table.clone(), image.clone()]))
        .unwrap()
        .into_document();
    let Block::Table { header, .. } = &ir.blocks[0] else {
        panic!("table");
    };
    assert_eq!(
        header[0],
        vec![Inline::CrossReference {
            kind: CrossReferenceKind::FigureNumber,
            target: "figure".into()
        }]
    );
    for field in ["Table:", "표:", "출처:"] {
        let paragraph = json!({"t":"Para","c":[{"t":"Str","c":field},{"t":"Space"},link.clone()]});
        normalize(json!([table.clone(), paragraph, image.clone()])).unwrap();
    }
}

#[test]
fn direct_ir_tables_are_closed_rectangular_and_context_validated() {
    let valid = validate(document(direct_table()), &ValidationLimits::default()).unwrap();
    assert_eq!(
        read_ir(&write_ir(&valid).unwrap(), &ValidationLimits::default()).unwrap(),
        valid
    );
    for pointer in ["/columns", "/header", "/rows/0"] {
        let mut value = serde_json::to_value(direct_table()).unwrap();
        value
            .pointer_mut(pointer)
            .unwrap()
            .as_array_mut()
            .unwrap()
            .pop();
        let bytes = serde_json::to_vec(
            &json!({"schema":SCHEMA_NAME,"ir_version":IR_VERSION,"metadata":{},"blocks":[value]}),
        )
        .unwrap();
        assert!(read_ir(&bytes, &ValidationLimits::default()).is_err());
    }
    for (member, value) in [
        ("id", json!("t")),
        ("extra", json!(1)),
        ("caption", json!([])),
        ("source", json!([])),
        ("columns", json!(["unknown"])),
    ] {
        let mut block = serde_json::to_value(direct_table()).unwrap();
        block[member] = value;
        let bytes = serde_json::to_vec(
            &json!({"schema":SCHEMA_NAME,"ir_version":IR_VERSION,"metadata":{},"blocks":[block]}),
        )
        .unwrap();
        assert!(
            read_ir(&bytes, &ValidationLimits::default()).is_err(),
            "accepted {member}"
        );
    }
}

#[test]
fn table_rows_cells_notes_and_caption_source_share_cumulative_limits() {
    let mut table = direct_table();
    let Block::Table {
        caption,
        source,
        rows,
        ..
    } = &mut table
    else {
        unreachable!()
    };
    *caption = Some(vec![text("제목")]);
    *source = Some(vec![md2hwp_core::ir::SourceParagraph {
        prefix: "출처".into(),
        inlines: vec![text("출처")],
    }]);
    rows[0][0] = vec![Inline::Footnote {
        blocks: vec![FootnoteBlock::Paragraph {
            inlines: vec![text("주석")],
        }],
    }];
    for (limits, expected) in [
        (
            ValidationLimits {
                max_blocks: 7,
                ..ValidationLimits::default()
            },
            "blocks limit",
        ),
        (
            ValidationLimits {
                max_inlines: 5,
                ..ValidationLimits::default()
            },
            "inlines limit",
        ),
        (
            ValidationLimits {
                max_text_bytes: 14,
                ..ValidationLimits::default()
            },
            "text bytes limit",
        ),
    ] {
        assert!(
            validate(document(table.clone()), &limits)
                .unwrap_err()
                .message
                .contains(expected)
        );
    }
}

#[test]
fn multiple_object_notes_preserve_labels_order_and_stop_at_body() {
    let note = |label: &str| json!({"t":"Para","c":[{"t":"Str","c":format!("{label}:")},{"t":"Space"},{"t":"Strong","c":[{"t":"Str","c":"설명"}]}]});
    let ir = normalize(json!([simple_table(), note("주.3"), note("Source."), note("trans."),
        {"t":"Para","c":[{"t":"Str","c":"일반"},{"t":"Space"},{"t":"Str","c":"문장:"},{"t":"Space"},{"t":"Str","c":"본문"}]}, note("출처")])).unwrap();
    let Block::Table {
        source: Some(source),
        ..
    } = &ir.as_document().blocks[0]
    else {
        panic!("source")
    };
    assert_eq!(
        source.iter().map(|s| s.prefix.as_str()).collect::<Vec<_>>(),
        ["주.3", "Source.", "trans."]
    );
    assert!(matches!(source[0].inlines[0], Inline::Strong { .. }));
    assert_eq!(ir.as_document().blocks.len(), 3);
    let bytes = write_ir(&ir).unwrap();
    assert_eq!(read_ir(&bytes, &ValidationLimits::default()).unwrap(), ir);
    let limits = ValidationLimits {
        max_blocks: 3,
        ..ValidationLimits::default()
    };
    assert!(read_ir(&bytes, &limits).is_err());
    for nodes in [
        json!([{"t":"Str","c":"출처"},{"t":"Space"},{"t":"Str","c":":"},{"t":"Space"},{"t":"Str","c":"내용"}]),
        json!([{"t":"Str","c":"note1.:"},{"t":"Space"},{"t":"Str","c":"내용"}]),
        json!([{"t":"Str","c":"결과:"},{"t":"Space"},{"t":"Str","c":"내용"}]),
    ] {
        let ir = normalize(json!([simple_table(), {"t":"Para","c":nodes}])).unwrap();
        assert!(matches!(
            ir.as_document().blocks[0],
            Block::Table { source: None, .. }
        ));
        assert_eq!(ir.as_document().blocks.len(), 2);
    }
}

#[test]
fn table_caption_and_all_source_paragraphs_accept_forward_number_references() {
    let r = json!({"t":"Link","c":[["",[],[]],[],["#figure",""]]});
    let image = json!({"t":"Para","c":[{"t":"Image","c":[["figure",[],[]],[{"t":"Str","c":"그림"}],["image.png",""]]}]});
    let ir = normalize(json!([
        {"t":"Para","c":[{"t":"Str","c":"Table:"},{"t":"Space"},r.clone()]}, simple_table(),
        {"t":"Para","c":[{"t":"Str","c":"출처:"},{"t":"Space"},r.clone()]},
        {"t":"Para","c":[{"t":"Str","c":"주:"},{"t":"Space"},{"t":"Emph","c":[r]}]}, image]))
    .unwrap();
    let Block::Table {
        caption: Some(caption),
        source: Some(notes),
        ..
    } = &ir.as_document().blocks[0]
    else {
        panic!("table")
    };
    assert!(matches!(caption[0], Inline::CrossReference { .. }));
    assert_eq!(notes.len(), 2);
    assert_eq!(
        read_ir(&write_ir(&ir).unwrap(), &ValidationLimits::default()).unwrap(),
        ir
    );
}
