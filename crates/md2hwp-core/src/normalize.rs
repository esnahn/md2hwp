//! Closed Pandoc AST-to-IR v0.1 normalization handlers.

use std::fmt;

use serde_json::Value;

use crate::ast2ir_rules::{Ast2IrRules, BlockRule, InlineRule, NumberDelimiter, NumberStyle};
use crate::ir::{
    Block, Document, IR_VERSION, Inline, ListItem, ListItemBlock, ListKind, Metadata, SCHEMA_NAME,
};
use crate::pandoc_input::PandocDocument;
use crate::validate::{ValidatedDocument, ValidationLimits, validate};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizeError {
    pub code: &'static str,
    pub path: String,
    pub constructor: Option<String>,
    pub reader: String,
    pub message: String,
}

impl fmt::Display for NormalizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at {}: {}", self.code, self.path, self.message)
    }
}

impl std::error::Error for NormalizeError {}

pub fn normalize_pandoc(
    document: PandocDocument,
    rules: &Ast2IrRules,
    reader: &str,
    limits: &ValidationLimits,
) -> Result<ValidatedDocument, NormalizeError> {
    let mut normalizer = Normalizer { rules, reader };
    if !document.metadata.is_empty() {
        return Err(normalizer.error(
            "invalid_pandoc_structure",
            "/meta",
            None,
            "Pandoc metadata must be empty for IR v0.1",
        ));
    }
    let mut blocks = Vec::with_capacity(document.blocks.len());
    for (index, block) in document.blocks.iter().enumerate() {
        blocks.push(normalizer.block(block, &format!("/blocks/{index}"))?);
    }
    let ir = Document {
        schema: SCHEMA_NAME.to_owned(),
        ir_version: IR_VERSION.to_owned(),
        metadata: Metadata::default(),
        blocks,
    };
    validate(ir, limits)
        .map_err(|error| normalizer.error("invalid_ir_semantics", &error.path, None, error.message))
}

struct Normalizer<'a> {
    rules: &'a Ast2IrRules,
    reader: &'a str,
}

impl Normalizer<'_> {
    fn block(&mut self, node: &Value, path: &str) -> Result<Block, NormalizeError> {
        let (constructor, content) = self.node(node, path)?;
        let rule = self
            .rules
            .blocks
            .get(constructor)
            .ok_or_else(|| self.unsupported(path, constructor))?;
        match (constructor, rule) {
            ("Para", BlockRule::Paragraph) => Ok(Block::Paragraph {
                inlines: self.inlines(
                    self.array(content, &format!("{path}/c"))?,
                    &format!("{path}/c"),
                )?,
            }),
            (
                "Header",
                BlockRule::Heading {
                    minimum_level,
                    maximum_level,
                    ..
                },
            ) => {
                let values = self.fixed_array(content, 3, &format!("{path}/c"))?;
                let level = values[0].as_u64().ok_or_else(|| {
                    self.invalid(
                        &format!("{path}/c/0"),
                        constructor,
                        "heading level must be an integer",
                    )
                })?;
                if level < u64::from(*minimum_level) || level > u64::from(*maximum_level) {
                    return Err(self.invalid(
                        &format!("{path}/c/0"),
                        constructor,
                        "heading level is outside the configured range",
                    ));
                }
                self.require_empty_attr(&values[1], &format!("{path}/c/1"), constructor)?;
                Ok(Block::Heading {
                    level: u8::try_from(level).expect("configured heading levels fit in u8"),
                    inlines: self.inlines(
                        self.array(Some(&values[2]), &format!("{path}/c/2"))?,
                        &format!("{path}/c/2"),
                    )?,
                })
            }
            ("CodeBlock", BlockRule::VerbatimBlock { .. }) => {
                let values = self.fixed_array(content, 2, &format!("{path}/c"))?;
                self.require_empty_attr(&values[0], &format!("{path}/c/0"), constructor)?;
                let text = values[1].as_str().ok_or_else(|| {
                    self.invalid(
                        &format!("{path}/c/1"),
                        constructor,
                        "code block content must be a string",
                    )
                })?;
                Ok(Block::VerbatimBlock {
                    lines: text.split('\n').map(str::to_owned).collect(),
                })
            }
            ("BulletList", BlockRule::BulletList) => {
                let items = self.array(content, &format!("{path}/c"))?;
                let tight = self.list_tightness(items, &format!("{path}/c"), constructor)?;
                Ok(Block::List {
                    kind: ListKind::Bullet,
                    start: None,
                    tight,
                    items: self.list_items(items, tight, &format!("{path}/c"))?,
                })
            }
            (
                "OrderedList",
                BlockRule::OrderedList {
                    minimum_start,
                    number_styles,
                    delimiters,
                },
            ) => {
                let values = self.fixed_array(content, 2, &format!("{path}/c"))?;
                let attributes = self.fixed_array(Some(&values[0]), 3, &format!("{path}/c/0"))?;
                let start = attributes[0].as_u64().ok_or_else(|| {
                    self.invalid(
                        &format!("{path}/c/0/0"),
                        constructor,
                        "ordered-list start must be an integer",
                    )
                })?;
                if start < *minimum_start {
                    return Err(self.invalid(
                        &format!("{path}/c/0/0"),
                        constructor,
                        "ordered-list start is below the configured minimum",
                    ));
                }
                let style = self.constructor_only(&attributes[1], &format!("{path}/c/0/1"))?;
                let style_supported = match style {
                    "DefaultStyle" => number_styles.contains(&NumberStyle::DefaultStyle),
                    "Decimal" => number_styles.contains(&NumberStyle::Decimal),
                    _ => false,
                };
                if !style_supported {
                    return Err(self.invalid(
                        &format!("{path}/c/0/1"),
                        style,
                        "ordered-list number style is not supported",
                    ));
                }
                let delimiter = self.constructor_only(&attributes[2], &format!("{path}/c/0/2"))?;
                let delimiter_supported = match delimiter {
                    "DefaultDelim" => delimiters.contains(&NumberDelimiter::DefaultDelim),
                    "Period" => delimiters.contains(&NumberDelimiter::Period),
                    "OneParen" => delimiters.contains(&NumberDelimiter::OneParen),
                    _ => false,
                };
                if !delimiter_supported {
                    return Err(self.invalid(
                        &format!("{path}/c/0/2"),
                        delimiter,
                        "ordered-list delimiter is not supported",
                    ));
                }
                let items = self.array(Some(&values[1]), &format!("{path}/c/1"))?;
                let tight = self.list_tightness(items, &format!("{path}/c/1"), constructor)?;
                Ok(Block::List {
                    kind: ListKind::Ordered,
                    start: Some(start),
                    tight,
                    items: self.list_items(items, tight, &format!("{path}/c/1"))?,
                })
            }
            _ => Err(self.invalid(
                path,
                constructor,
                "constructor and configured handler do not match",
            )),
        }
    }

    fn list_items(
        &mut self,
        items: &[Value],
        tight: bool,
        path: &str,
    ) -> Result<Vec<ListItem>, NormalizeError> {
        let mut normalized = Vec::with_capacity(items.len());
        for (item_index, item) in items.iter().enumerate() {
            let item_path = format!("{path}/{item_index}");
            let blocks = item.as_array().ok_or_else(|| {
                self.error(
                    "invalid_pandoc_structure",
                    &item_path,
                    None,
                    "list item must be an array",
                )
            })?;
            let mut item_blocks = Vec::with_capacity(blocks.len());
            for (block_index, block) in blocks.iter().enumerate() {
                item_blocks.push(self.list_item_block(
                    block,
                    tight,
                    &format!("{item_path}/{block_index}"),
                )?);
            }
            normalized.push(ListItem {
                blocks: item_blocks,
            });
        }
        Ok(normalized)
    }

    fn list_item_block(
        &mut self,
        node: &Value,
        tight: bool,
        path: &str,
    ) -> Result<ListItemBlock, NormalizeError> {
        let (constructor, content) = self.node(node, path)?;
        match constructor {
            "Plain" if tight => {
                let Some(BlockRule::ListItemParagraph { .. }) = self.rules.blocks.get("Plain")
                else {
                    return Err(self.unsupported(path, constructor));
                };
                Ok(ListItemBlock::Paragraph {
                    inlines: self.inlines(
                        self.array(content, &format!("{path}/c"))?,
                        &format!("{path}/c"),
                    )?,
                })
            }
            "Para" if !tight => {
                let Some(BlockRule::Paragraph) = self.rules.blocks.get("Para") else {
                    return Err(self.unsupported(path, constructor));
                };
                Ok(ListItemBlock::Paragraph {
                    inlines: self.inlines(
                        self.array(content, &format!("{path}/c"))?,
                        &format!("{path}/c"),
                    )?,
                })
            }
            "BulletList" | "OrderedList" => match self.block(node, path)? {
                Block::List {
                    kind,
                    start,
                    tight,
                    items,
                } => Ok(ListItemBlock::List {
                    kind,
                    start,
                    tight,
                    items,
                }),
                _ => unreachable!("list constructors normalize to list blocks"),
            },
            "Plain" | "Para" => Err(self.invalid(
                path,
                constructor,
                "list paragraph constructor does not match list tightness",
            )),
            _ if !self.rules.blocks.contains_key(constructor) => {
                Err(self.unsupported(path, constructor))
            }
            _ => Err(self.invalid(
                path,
                constructor,
                "block is not representable inside an IR v0.1 list item",
            )),
        }
    }

    fn list_tightness(
        &self,
        items: &[Value],
        path: &str,
        constructor: &str,
    ) -> Result<bool, NormalizeError> {
        if items.is_empty() {
            return Err(self.invalid(path, constructor, "list must contain at least one item"));
        }
        let mut saw_plain = false;
        let mut saw_para = false;
        for (item_index, item) in items.iter().enumerate() {
            let blocks = item.as_array().ok_or_else(|| {
                self.invalid(
                    &format!("{path}/{item_index}"),
                    constructor,
                    "list item must be an array",
                )
            })?;
            if blocks.is_empty() {
                return Err(self.invalid(
                    &format!("{path}/{item_index}"),
                    constructor,
                    "list item must not be empty",
                ));
            }
            for (block_index, block) in blocks.iter().enumerate() {
                let (kind, _) = self.node(block, &format!("{path}/{item_index}/{block_index}"))?;
                saw_plain |= kind == "Plain";
                saw_para |= kind == "Para";
            }
        }
        match (saw_plain, saw_para) {
            (true, false) => Ok(true),
            (false, true) => Ok(false),
            (true, true) => Err(self.invalid(
                path,
                constructor,
                "mixed Plain and Para list items are ambiguous",
            )),
            (false, false) => {
                Err(self.invalid(path, constructor, "list items require paragraph anchors"))
            }
        }
    }

    fn inlines(&mut self, nodes: &[Value], path: &str) -> Result<Vec<Inline>, NormalizeError> {
        let mut inlines = Vec::with_capacity(nodes.len());
        for (index, node) in nodes.iter().enumerate() {
            inlines.push(self.inline(node, &format!("{path}/{index}"))?);
        }
        Ok(inlines)
    }

    fn inline(&mut self, node: &Value, path: &str) -> Result<Inline, NormalizeError> {
        let (constructor, content) = self.node(node, path)?;
        let rule = self
            .rules
            .inlines
            .get(constructor)
            .ok_or_else(|| self.unsupported(path, constructor))?;
        match (constructor, rule) {
            ("Str", InlineRule::Text) => {
                let value = content.and_then(Value::as_str).ok_or_else(|| {
                    self.invalid(
                        &format!("{path}/c"),
                        constructor,
                        "Str content must be a string",
                    )
                })?;
                Ok(Inline::Text {
                    value: value.to_owned(),
                })
            }
            ("Space", InlineRule::Space) | ("SoftBreak", InlineRule::Space) => Ok(Inline::Space),
            ("LineBreak", InlineRule::LineBreak) => Ok(Inline::LineBreak),
            ("Strong", InlineRule::Strong) => Ok(Inline::Strong {
                inlines: self.inlines(
                    self.array(content, &format!("{path}/c"))?,
                    &format!("{path}/c"),
                )?,
            }),
            ("Emph", InlineRule::Emph) => Ok(Inline::Emph {
                inlines: self.inlines(
                    self.array(content, &format!("{path}/c"))?,
                    &format!("{path}/c"),
                )?,
            }),
            ("Link", InlineRule::Link { .. }) => {
                let values = self.fixed_array(content, 3, &format!("{path}/c"))?;
                self.require_empty_attr(&values[0], &format!("{path}/c/0"), constructor)?;
                let target = self.fixed_array(Some(&values[2]), 2, &format!("{path}/c/2"))?;
                let target_value = target[0].as_str().ok_or_else(|| {
                    self.invalid(
                        &format!("{path}/c/2/0"),
                        constructor,
                        "link target must be a string",
                    )
                })?;
                let title = target[1].as_str().ok_or_else(|| {
                    self.invalid(
                        &format!("{path}/c/2/1"),
                        constructor,
                        "link title must be a string",
                    )
                })?;
                Ok(Inline::Link {
                    target: target_value.to_owned(),
                    title: (!title.is_empty()).then(|| title.to_owned()),
                    inlines: self.inlines(
                        self.array(Some(&values[1]), &format!("{path}/c/1"))?,
                        &format!("{path}/c/1"),
                    )?,
                })
            }
            (_, InlineRule::Reject) => Err(self.unsupported(path, constructor)),
            _ => Err(self.invalid(
                path,
                constructor,
                "constructor and configured handler do not match",
            )),
        }
    }

    fn node<'a>(
        &self,
        value: &'a Value,
        path: &str,
    ) -> Result<(&'a str, Option<&'a Value>), NormalizeError> {
        let object = value.as_object().ok_or_else(|| {
            self.error(
                "invalid_pandoc_structure",
                path,
                None,
                "Pandoc node must be an object",
            )
        })?;
        if object.keys().any(|key| key != "t" && key != "c") {
            return Err(self.error(
                "invalid_pandoc_structure",
                path,
                None,
                "Pandoc node contains unknown members",
            ));
        }
        let constructor = object.get("t").and_then(Value::as_str).ok_or_else(|| {
            self.error(
                "invalid_pandoc_structure",
                &format!("{path}/t"),
                None,
                "Pandoc node constructor must be a string",
            )
        })?;
        Ok((constructor, object.get("c")))
    }

    fn constructor_only<'a>(
        &self,
        value: &'a Value,
        path: &str,
    ) -> Result<&'a str, NormalizeError> {
        let (constructor, content) = self.node(value, path)?;
        if content.is_some() {
            return Err(self.invalid(path, constructor, "constructor must not have content"));
        }
        Ok(constructor)
    }

    fn array<'a>(
        &self,
        value: Option<&'a Value>,
        path: &str,
    ) -> Result<&'a [Value], NormalizeError> {
        value
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .ok_or_else(|| self.error("invalid_pandoc_structure", path, None, "expected an array"))
    }

    fn fixed_array<'a>(
        &self,
        value: Option<&'a Value>,
        length: usize,
        path: &str,
    ) -> Result<&'a [Value], NormalizeError> {
        let values = self.array(value, path)?;
        if values.len() != length {
            return Err(self.error(
                "invalid_pandoc_structure",
                path,
                None,
                format!("expected {length} array elements, got {}", values.len()),
            ));
        }
        Ok(values)
    }

    fn require_empty_attr(
        &self,
        value: &Value,
        path: &str,
        constructor: &str,
    ) -> Result<(), NormalizeError> {
        let values = self.fixed_array(Some(value), 3, path)?;
        let empty = values[0].as_str() == Some("")
            && values[1].as_array().is_some_and(Vec::is_empty)
            && values[2].as_array().is_some_and(Vec::is_empty);
        if empty {
            Ok(())
        } else {
            Err(self.invalid(
                path,
                constructor,
                "attributes must be exactly [\"\", [], []]",
            ))
        }
    }

    fn unsupported(&self, path: &str, constructor: &str) -> NormalizeError {
        self.error(
            "unsupported_pandoc_node",
            path,
            Some(constructor),
            format!("{constructor} is not supported by IR {IR_VERSION}"),
        )
    }

    fn invalid(&self, path: &str, constructor: &str, message: impl Into<String>) -> NormalizeError {
        self.error("invalid_pandoc_structure", path, Some(constructor), message)
    }

    fn error(
        &self,
        code: &'static str,
        path: &str,
        constructor: Option<&str>,
        message: impl Into<String>,
    ) -> NormalizeError {
        NormalizeError {
            code,
            path: path.to_owned(),
            constructor: constructor.map(str::to_owned),
            reader: self.reader.to_owned(),
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast2ir_rules::load_builtin_rules;
    use crate::ir_io::read_ir;
    use crate::pandoc_input::read_pandoc_json;

    const COMMONMARK_FIXTURE: &[u8] =
        include_bytes!("../../../tests/fixtures/pandoc-json/commonmark-v0.1.json");
    const EXPECTED_IR: &[u8] = include_bytes!("../../../examples/commonmark-v0.1.expected.ir.json");

    #[test]
    fn normalizes_the_commonmark_compatibility_fixture() {
        let limits = ValidationLimits::default();
        let pandoc = read_pandoc_json(COMMONMARK_FIXTURE, &limits).unwrap();
        let rules = load_builtin_rules().unwrap();
        let ir = normalize_pandoc(pandoc, &rules, "commonmark", &limits).unwrap();
        assert_eq!(ir, read_ir(EXPECTED_IR, &limits).unwrap());
        assert_eq!(ir.as_document().blocks.len(), 4);
        let Block::Paragraph { inlines, .. } = &ir.as_document().blocks[1] else {
            panic!("expected paragraph")
        };
        let composed_count = inlines
            .iter()
            .filter(|inline| {
                **inline
                    == Inline::Text {
                        value: "가".to_owned(),
                    }
            })
            .count();
        assert_eq!(composed_count, 2);
        assert!(!inlines.contains(&Inline::Text {
            value: "가".to_owned()
        }));
        let Block::VerbatimBlock { lines } = &ir.as_document().blocks[2] else {
            panic!("expected verbatim block")
        };
        assert_eq!(lines, &["박스의 첫째 줄", "", "빈 줄 다음 줄", ""]);
        let Block::List { tight, .. } = &ir.as_document().blocks[3] else {
            panic!("expected list")
        };
        assert!(!tight);
    }

    #[test]
    fn rejects_an_unlisted_constructor_with_its_path() {
        let limits = ValidationLimits::default();
        let pandoc = read_pandoc_json(
            br#"{"pandoc-api-version":[1,23,1,2],"meta":{},"blocks":[{"t":"BlockQuote","c":[]}]}"#,
            &limits,
        )
        .unwrap();
        let error = normalize_pandoc(
            pandoc,
            &load_builtin_rules().unwrap(),
            "commonmark",
            &limits,
        )
        .unwrap_err();
        assert_eq!(error.code, "unsupported_pandoc_node");
        assert_eq!(error.path, "/blocks/0");
        assert_eq!(error.constructor.as_deref(), Some("BlockQuote"));
    }
}
