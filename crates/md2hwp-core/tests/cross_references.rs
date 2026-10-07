use md2hwp_core::ir::{
    Block, CrossReferenceKind, Document, FootnoteBlock, IR_VERSION, ImageRef, Inline, Metadata,
    SCHEMA_NAME,
};
use md2hwp_core::{
    NormalizeError, ValidatedDocument, ValidationLimits, load_builtin_rules, normalize_pandoc,
    read_ir, read_pandoc_json, validate, write_ir,
};
use serde_json::{Value, json};

fn normalize_with_limits(
    blocks: Value,
    limits: &ValidationLimits,
) -> Result<ValidatedDocument, NormalizeError> {
    let bytes = serde_json::to_vec(
        &json!({"pandoc-api-version": [1,23,1,2], "meta": {}, "blocks": blocks}),
    )
    .unwrap();
    normalize_pandoc(
        read_pandoc_json(&bytes, limits).unwrap(),
        &load_builtin_rules().unwrap(),
        "commonmark+yaml_metadata_block+footnotes+attributes+implicit_figures",
        limits,
    )
}
fn normalize(blocks: Value) -> Result<ValidatedDocument, NormalizeError> {
    normalize_with_limits(blocks, &ValidationLimits::default())
}
fn image_node(id: &str) -> Value {
    json!({"t":"Image","c":[[id,[],[]],[{"t":"Str","c":"c"}],["a.png",""]]})
}
fn para_image(id: &str) -> Value {
    json!({"t":"Para","c":[image_node(id)]})
}
fn heading(id: &str) -> Value {
    json!({"t":"Header","c":[2,[id,[],[]],[{"t":"Str","c":"heading"}]]})
}
fn link(target: &str) -> Value {
    json!({"t":"Link","c":[["",[],[]],[{"t":"Str","c":"label"}],[target,""]]})
}

#[test]
fn heading_number_references_resolve_all_six_levels_and_roundtrip() {
    let targets: Vec<_> = (1..=6).map(|level| format!("수준-{level}")).collect();
    let links: Vec<_> = targets.iter().map(|id| link(&format!("#{id}"))).collect();
    let mut blocks = vec![json!({"t":"Para","c":links})];
    for (level, id) in (1..=6).zip(&targets) {
        blocks.push(json!({"t":"Header","c":[level,[id,[],[]],[{"t":"Str","c":"heading"}]]}));
    }
    let actual = normalize(Value::Array(blocks)).unwrap();
    let Block::Paragraph { inlines } = &actual.as_document().blocks[0] else {
        panic!("expected reference paragraph")
    };
    assert_eq!(
        inlines,
        &targets
            .iter()
            .map(|id| reference(CrossReferenceKind::HeadingNumber, id))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        read_ir(&write_ir(&actual).unwrap(), &ValidationLimits::default()).unwrap(),
        actual
    );
}
fn reference(kind: CrossReferenceKind, target: &str) -> Inline {
    Inline::CrossReference {
        kind,
        target: target.into(),
    }
}
fn figure(id: Option<&str>) -> Block {
    Block::Figure {
        id: id.map(str::to_owned),
        image: ImageRef {
            path: "a.png".into(),
            alt: vec![],
            title: None,
        },
        caption: vec![Inline::Text { value: "c".into() }],
        source: None,
    }
}
fn document(blocks: Vec<Block>) -> Document {
    Document {
        schema: SCHEMA_NAME.into(),
        ir_version: IR_VERSION.into(),
        metadata: Metadata::default(),
        blocks,
    }
}

#[test]
fn actual_pandoc_inline_and_reference_links_resolve_while_at_syntax_remains_literal() {
    let limits = ValidationLimits::default();
    let fixture =
        include_bytes!("../../../tests/fixtures/pandoc-json/cross-reference-links-v0.3.json");
    let raw: Value = serde_json::from_slice(fixture).unwrap();
    assert!(
        raw["blocks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|b| b["t"] == "Figure")
    );
    let actual = normalize_pandoc(
        read_pandoc_json(fixture, &limits).unwrap(),
        &load_builtin_rules().unwrap(),
        "commonmark+yaml_metadata_block+footnotes+attributes+implicit_figures",
        &limits,
    )
    .unwrap();
    let blocks = &actual.as_document().blocks;
    assert_eq!(
        actual.as_document().metadata.text("md2hwp-heading1-start"),
        Some("3")
    );
    let Block::Paragraph { inlines } = &blocks[1] else {
        panic!("forward links")
    };
    assert_eq!(inlines.iter().filter(|i| matches!(i, Inline::CrossReference { kind: CrossReferenceKind::FigureNumber, target } if target == "diagram")).count(), 2);
    assert!(inlines.iter().any(|i| matches!(i, Inline::CrossReference { kind: CrossReferenceKind::HeadingNumber, target } if target == "chapter")));
    assert!(
        blocks
            .iter()
            .any(|b| matches!(b, Block::Figure { id: None, .. }))
    );
    assert!(blocks.iter().any(|b| matches!(b, Block::Paragraph { inlines } if inlines.iter().any(|i| matches!(i, Inline::Text { value } if value == "[@fig:diagram]")))));
    assert!(blocks.iter().any(|b| matches!(b, Block::VerbatimBlock { lines, .. } if lines[0] == "[그림](#diagram) [@fig:diagram]")));
    assert_eq!(
        read_ir(&write_ir(&actual).unwrap(), &limits).unwrap(),
        actual
    );
}

#[test]
fn standalone_image_and_one_image_figure_normalize_to_same_ir() {
    let para = normalize(json!([para_image("target")])).unwrap();
    let native = normalize(json!([{"t":"Figure","c":[["target",[],[]],[null,[{"t":"Plain","c":[{"t":"Str","c":"c"}]}]],[{"t":"Plain","c":[image_node("")]}]]}])).unwrap();
    assert_eq!(para, native);
}

#[test]
fn kind_follows_actual_block_and_unicode_fragments_decode_strictly() {
    let result = normalize(json!([
        {"t":"Para","c":[link("#fig:heading"),link("#section"),link("#%EA%B7%B8%EB%A6%BC"),link("#100%25"),link("#a+b")]},
        heading("fig:heading"),para_image("section"),para_image("그림"),para_image("100%"),heading("a+b")
    ])).unwrap().into_document();
    let Block::Paragraph { inlines } = &result.blocks[0] else {
        panic!("paragraph")
    };
    assert_eq!(
        inlines,
        &vec![
            reference(CrossReferenceKind::HeadingNumber, "fig:heading"),
            reference(CrossReferenceKind::FigureNumber, "section"),
            reference(CrossReferenceKind::FigureNumber, "그림"),
            reference(CrossReferenceKind::FigureNumber, "100%"),
            reference(CrossReferenceKind::HeadingNumber, "a+b")
        ]
    );
    for target in [
        "#", "#bad%", "#bad%0", "#bad%zz", "#%ff", "#%c3%28", "#%23id", "#a%20b",
    ] {
        let error = normalize(json!([{"t":"Para","c":[link(target)]}])).unwrap_err();
        assert_eq!(error.code, "invalid_pandoc_structure", "{target}: {error}");
    }
}

#[test]
fn ordinary_links_and_literal_at_strings_keep_existing_labels() {
    let result = normalize(json!([{"t":"Para","c":[link("https://example.net/#missing"),{"t":"Str","c":"한글[@fig:missing]"},{"t":"Str","c":"[@tbl:table]"}]}])).unwrap().into_document();
    let Block::Paragraph { inlines } = &result.blocks[0] else {
        panic!("paragraph")
    };
    assert!(
        matches!(&inlines[0], Inline::Link { target, inlines, .. } if target == "https://example.net/#missing" && inlines == &vec![Inline::Text { value: "label".into() }])
    );
    assert_eq!(
        inlines[1],
        Inline::Text {
            value: "한글[@fig:missing]".into()
        }
    );
    assert_eq!(
        inlines[2],
        Inline::Text {
            value: "[@tbl:table]".into()
        }
    );
}

#[test]
fn source_labels_titles_and_unsupported_ast_are_validated_before_replacement() {
    for label in [
        json!([{"t":"Str","c":""}]),
        json!([{"t":"Str","c":"contains space"}]),
        json!([{"t":"Str","c":"가"}]),
        json!([link("https://example.net")]),
        json!([{"t":"Code","c":[["",[],[]],"raw"]}]),
        json!([{"t":"Math","c":[]}]),
    ] {
        let mut node = link("#target");
        node["c"][1] = label;
        assert!(normalize(json!([{"t":"Para","c":[node]},para_image("target")])).is_err());
    }
    for title in ["bad\n", "가"] {
        let mut node = link("#target");
        node["c"][2][1] = json!(title);
        assert!(normalize(json!([{"t":"Para","c":[node]},para_image("target")])).is_err());
    }
    let mut node = link("#target");
    node["c"][0] = json!(["label-id", [], []]);
    assert!(
        normalize(json!([{"t":"Para","c":[node]},para_image("target")]))
            .unwrap_err()
            .message
            .contains("attributes must be exactly")
    );
    let mut node = link("#target");
    node["c"][1] = json!([{"t":"Strong","c":[{"t":"Str","c":"label"}]}]);
    node["c"][2][1] = json!("valid title");
    let result = normalize(json!([{"t":"Para","c":[node]},para_image("target")]))
        .unwrap()
        .into_document();
    assert_eq!(
        result.blocks[0],
        Block::Paragraph {
            inlines: vec![reference(CrossReferenceKind::FigureNumber, "target")]
        }
    );
}

#[test]
fn discarded_labels_and_titles_still_use_cumulative_resource_limits() {
    let blocks = json!([{"t":"Para","c":[link("#f")]},para_image("f")]);
    let exact = ValidationLimits {
        max_text_bytes: 15,
        max_inlines: 4,
        ..ValidationLimits::default()
    };
    normalize_with_limits(blocks.clone(), &exact).unwrap();
    for limits in [
        ValidationLimits {
            max_text_bytes: 14,
            ..exact.clone()
        },
        ValidationLimits {
            max_inlines: 3,
            ..exact.clone()
        },
    ] {
        assert!(
            normalize_with_limits(blocks.clone(), &limits)
                .unwrap_err()
                .message
                .contains("limit exceeded")
        );
    }
    let mut titled = blocks;
    titled[0]["c"][0]["c"][2][1] = json!("title");
    assert!(
        normalize_with_limits(titled, &exact)
            .unwrap_err()
            .message
            .contains("text bytes limit exceeded")
    );
}

#[test]
fn duplicate_ids_missing_targets_and_wrong_direct_ir_kinds_fail() {
    let duplicate = normalize(json!([heading("same"), para_image("same")])).unwrap_err();
    assert_eq!(duplicate.path, "/blocks/1/id");
    assert!(duplicate.message.contains("duplicate target ID"));
    let missing = normalize(json!([{"t":"Para","c":[link("#missing")]}])).unwrap_err();
    assert_eq!(missing.code, "unresolved_cross_reference");
    assert!(missing.message.contains("table numbers are not supported"));
    let mismatch = validate(
        document(vec![
            Block::Paragraph {
                inlines: vec![reference(CrossReferenceKind::HeadingNumber, "f")],
            },
            figure(Some("f")),
        ]),
        &ValidationLimits::default(),
    )
    .unwrap_err();
    assert_eq!(mismatch.path, "/blocks/0/inlines/0/kind");
    assert_eq!(
        normalize(json!([{"t":"Table","c":[]}])).unwrap_err().code,
        "invalid_pandoc_structure"
    );
}

#[test]
fn refs_work_in_headings_lists_formatting_and_note_body_but_not_object_fields() {
    let r = link("#f");
    normalize(json!([
        {"t":"Header","c":[2,["section",[],[]],[{"t":"Strong","c":[r.clone()]}]]},
        {"t":"BulletList","c":[[{"t":"Plain","c":[{"t":"Emph","c":[r.clone()]}]}]]},
        {"t":"Para","c":[{"t":"Note","c":[{"t":"Para","c":[r.clone()]},{"t":"Para","c":[{"t":"Strong","c":[r.clone()]}]}]}]},para_image("f")
    ])).unwrap();
    let mut image = para_image("f");
    image["c"][0]["c"][1] = json!([r.clone()]);
    assert!(
        normalize(json!([image]))
            .unwrap_err()
            .message
            .contains("figure alt/caption")
    );
    for object in [
        para_image(""),
        json!({"t":"CodeBlock","c":[["",[],[]],"raw"]}),
    ] {
        assert!(normalize(json!([object,{"t":"Para","c":[{"t":"Str","c":"출처:"},{"t":"Space"},r.clone()]},heading("f")])).unwrap_err().message.contains("object sources"));
    }
    let mut target = figure(Some("f"));
    let Block::Figure { caption, .. } = &mut target else {
        unreachable!()
    };
    *caption = vec![reference(CrossReferenceKind::FigureNumber, "f")];
    assert_eq!(
        validate(document(vec![target]), &ValidationLimits::default())
            .unwrap_err()
            .path,
        "/blocks/0/caption/0"
    );
}

#[test]
fn id_attributes_and_figure_shapes_are_closed() {
    for id in ["bad id", "bad#id", "bad\n", "bad\u{0080}"] {
        assert!(normalize(json!([heading(id)])).is_err());
        assert!(normalize(json!([para_image(id)])).is_err());
    }
    let mut image = para_image("f");
    image["c"][0]["c"][0][1] = json!(["class"]);
    assert!(normalize(json!([image])).is_err());
    let valid = json!({"t":"Figure","c":[["f",[],[]],[null,[{"t":"Plain","c":[{"t":"Str","c":"caption"}]}]],[{"t":"Plain","c":[image_node("")]}]]});
    for body in [
        json!([]),
        json!([{ "t":"Plain","c":[image_node(""),image_node("")] }]),
        json!([{ "t":"CodeBlock","c":[["",[],[]],"raw"] }]),
    ] {
        let mut node = valid.clone();
        node["c"][2] = body;
        assert!(normalize(json!([node])).is_err());
    }
    for caption in [
        json!([]),
        json!([{ "t":"Plain","c":[] },{ "t":"Plain","c":[] }]),
        json!([{ "t":"Header","c":[] }]),
    ] {
        let mut node = valid.clone();
        node["c"][1][1] = caption;
        assert!(normalize(json!([node])).is_err());
    }
    let mut node = valid.clone();
    node["c"][1][0] = json!([{ "t":"Str","c":"short" }]);
    assert!(normalize(json!([node])).is_err());
    let mut node = valid;
    node["c"][2][0]["c"][0]["c"][0][0] = json!("different");
    assert!(
        normalize(json!([node]))
            .unwrap_err()
            .message
            .contains("conflicting IDs")
    );
}

#[test]
fn closed_ir_supports_only_current_reference_kinds_and_optional_nonnull_ids() {
    let valid = validate(
        document(vec![
            Block::Paragraph {
                inlines: vec![reference(CrossReferenceKind::FigureNumber, "그림")],
            },
            figure(Some("그림")),
        ]),
        &ValidationLimits::default(),
    )
    .unwrap();
    let encoded = write_ir(&valid).unwrap();
    let value: Value = serde_json::from_slice(&encoded).unwrap();
    read_ir(&encoded, &ValidationLimits::default()).unwrap();
    for kind in ["table_number", "page_number", ""] {
        let mut invalid = value.clone();
        invalid["blocks"][0]["inlines"][0]["kind"] = json!(kind);
        assert!(
            read_ir(
                &serde_json::to_vec(&invalid).unwrap(),
                &ValidationLimits::default()
            )
            .is_err()
        );
    }
    for id in [
        json!(null),
        json!(""),
        json!("bad id"),
        json!("bad#id"),
        json!("bad\u{0080}"),
    ] {
        let mut invalid = value.clone();
        invalid["blocks"][1]["id"] = id;
        assert!(
            read_ir(
                &serde_json::to_vec(&invalid).unwrap(),
                &ValidationLimits::default()
            )
            .is_err()
        );
    }
    let encoded =
        write_ir(&validate(document(vec![figure(None)]), &ValidationLimits::default()).unwrap())
            .unwrap();
    let value: Value = serde_json::from_slice(&encoded).unwrap();
    assert!(value["blocks"][0].get("id").is_none());
}

#[test]
fn note_references_ids_and_text_share_cumulative_ir_limits() {
    let input = document(vec![
        Block::Paragraph {
            inlines: vec![Inline::Footnote {
                blocks: vec![FootnoteBlock::Paragraph {
                    inlines: vec![reference(CrossReferenceKind::FigureNumber, "f")],
                }],
            }],
        },
        figure(Some("f")),
    ]);
    let exact = ValidationLimits {
        max_text_bytes: 8,
        max_inlines: 3,
        max_blocks: 3,
        ..ValidationLimits::default()
    };
    validate(input.clone(), &exact).unwrap();
    for limits in [
        ValidationLimits {
            max_text_bytes: 7,
            ..exact.clone()
        },
        ValidationLimits {
            max_inlines: 2,
            ..exact.clone()
        },
        ValidationLimits {
            max_blocks: 2,
            ..exact
        },
    ] {
        assert!(
            validate(input.clone(), &limits)
                .unwrap_err()
                .message
                .contains("limit exceeded")
        );
    }
}

#[test]
fn reference_label_notes_fail_explicitly_while_notes_after_refs_and_external_label_notes_survive() {
    let note = json!({"t":"Note","c":[{"t":"Para","c":[{"t":"Str","c":"note"}]}]});
    for label in [
        json!([{"t":"Str","c":"label"},note.clone()]),
        json!([{"t":"Strong","c":[note.clone()]}]),
    ] {
        let mut node = link("#target");
        node["c"][1] = label;
        let error = normalize(json!([{"t":"Para","c":[node]},para_image("target")])).unwrap_err();
        assert_eq!(error.code, "unsupported_cross_reference");
        assert!(error.message.contains("place the note after the link"));
    }
    let mut external = link("https://example.net");
    external["c"][1] = json!([{"t":"Str","c":"label"},note.clone()]);
    let actual = normalize(
        json!([{"t":"Para","c":[link("#target"),note.clone(),external]},para_image("target")]),
    )
    .unwrap()
    .into_document();
    let Block::Paragraph { inlines } = &actual.blocks[0] else {
        unreachable!()
    };
    assert!(matches!(inlines[0], Inline::CrossReference { .. }));
    assert!(matches!(inlines[1], Inline::Footnote { .. }));
    let Inline::Link { inlines, .. } = &inlines[2] else {
        unreachable!()
    };
    assert!(matches!(inlines[1], Inline::Footnote { .. }));
}

#[test]
fn empty_internal_reference_labels_use_actual_targets_without_hidden_label_text() {
    let limits = ValidationLimits::default();
    let fixture = include_bytes!(
        "../../../tests/fixtures/pandoc-json/cross-reference-empty-labels-v0.3.json"
    );
    let raw: Value = serde_json::from_slice(fixture).unwrap();
    assert_eq!(raw["blocks"][0]["c"][0]["t"], "Link");
    assert_eq!(raw["blocks"][0]["c"][0]["c"][1], json!([]));
    let actual = normalize_pandoc(
        read_pandoc_json(fixture, &limits).unwrap(),
        &load_builtin_rules().unwrap(),
        "commonmark+yaml_metadata_block+footnotes+attributes+implicit_figures",
        &limits,
    )
    .unwrap();
    let Block::Paragraph { inlines } = &actual.as_document().blocks[0] else {
        unreachable!()
    };
    assert_eq!(
        inlines,
        &vec![
            reference(CrossReferenceKind::HeadingNumber, "h"),
            Inline::Space,
            reference(CrossReferenceKind::FigureNumber, "f")
        ]
    );
    assert_eq!(
        read_ir(&write_ir(&actual).unwrap(), &limits).unwrap(),
        actual
    );
    for target in ["#missing", "#"] {
        let mut node = link(target);
        node["c"][1] = json!([]);
        let error = normalize(json!([{"t":"Para","c":[node]}])).unwrap_err();
        assert!(!error.message.contains("link label must not be empty"));
    }
}

#[test]
fn empty_labels_remain_invalid_for_external_source_links_and_public_ir_links() {
    for target in [
        "https://example.net",
        "relative.md",
        "https://example.net/#h",
    ] {
        let mut node = link(target);
        node["c"][1] = json!([]);
        assert!(
            normalize(json!([{"t":"Para","c":[node]}]))
                .unwrap_err()
                .message
                .contains("link label must not be empty")
        );
    }
    let source = document(vec![
        Block::Heading {
            id: Some("h".into()),
            level: 2,
            inlines: vec![Inline::Text {
                value: "heading".into(),
            }],
        },
        Block::Paragraph {
            inlines: vec![Inline::Link {
                target: "#h".into(),
                title: None,
                inlines: vec![],
            }],
        },
    ]);
    assert!(
        validate(source.clone(), &ValidationLimits::default())
            .unwrap_err()
            .message
            .contains("link label must not be empty")
    );
    let error = read_ir(
        &serde_json::to_vec(&source).unwrap(),
        &ValidationLimits::default(),
    )
    .unwrap_err();
    assert_eq!(error.code, md2hwp_core::IrReadErrorCode::InvalidIrSchema);
}

#[test]
fn empty_reference_labels_still_count_source_targets_ids_inlines_and_blocks() {
    let mut h = link("#h");
    h["c"][1] = json!([]);
    let mut f = link("#f");
    f["c"][1] = json!([]);
    let blocks = json!([{"t":"Para","c":[h,f]},heading("h"),para_image("f")]);
    let exact = ValidationLimits {
        max_text_bytes: 20,
        max_inlines: 5,
        max_blocks: 3,
        ..ValidationLimits::default()
    };
    normalize_with_limits(blocks.clone(), &exact).unwrap();
    for limits in [
        ValidationLimits {
            max_text_bytes: 19,
            ..exact.clone()
        },
        ValidationLimits {
            max_inlines: 4,
            ..exact.clone()
        },
        ValidationLimits {
            max_blocks: 2,
            ..exact.clone()
        },
    ] {
        assert!(
            normalize_with_limits(blocks.clone(), &limits)
                .unwrap_err()
                .message
                .contains("limit exceeded")
        );
    }
    let mut titled = blocks;
    titled[0]["c"][0]["c"][2][1] = json!("title");
    assert!(
        normalize_with_limits(titled, &exact)
            .unwrap_err()
            .message
            .contains("text bytes limit exceeded")
    );
}
