//! IR-local semantic and resource validation.

use std::collections::BTreeMap;
use std::fmt;

use icu_normalizer::ComposingNormalizerBorrowed;

use crate::ir::{
    Block, CrossReferenceKind, Document, FootnoteBlock, IR_VERSION, Inline, ListItem,
    ListItemBlock, ListKind, SCHEMA_NAME, is_target_id,
};

const NFC: ComposingNormalizerBorrowed<'static> = ComposingNormalizerBorrowed::new_nfc();

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationLimits {
    pub max_json_bytes: usize,
    pub max_json_nesting: usize,
    pub max_blocks: usize,
    pub max_list_items: usize,
    pub max_inlines: usize,
    pub max_text_bytes: usize,
}

impl Default for ValidationLimits {
    fn default() -> Self {
        Self {
            max_json_bytes: 16 * 1024 * 1024,
            max_json_nesting: 128,
            max_blocks: 100_000,
            max_list_items: 100_000,
            max_inlines: 1_000_000,
            max_text_bytes: 64 * 1024 * 1024,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticError {
    pub path: String,
    pub message: String,
}

impl fmt::Display for SemanticError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path, self.message)
    }
}

impl std::error::Error for SemanticError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidatedDocument(Document);

impl ValidatedDocument {
    pub fn as_document(&self) -> &Document {
        &self.0
    }

    pub fn into_document(self) -> Document {
        self.0
    }
}

pub fn validate(
    document: Document,
    limits: &ValidationLimits,
) -> Result<ValidatedDocument, SemanticError> {
    validate_semantics(&document, limits, ValidationMode::Ir)?;
    Ok(ValidatedDocument(document))
}

// Only Pandoc's original internal Link labels may be empty. This check returns
// no ValidatedDocument: normalization must resolve them and validate public IR.
pub(crate) fn validate_pandoc_source(
    document: &Document,
    limits: &ValidationLimits,
) -> Result<(), SemanticError> {
    validate_semantics(document, limits, ValidationMode::PandocSource)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ValidationMode {
    Ir,
    PandocSource,
}

fn validate_semantics(
    document: &Document,
    limits: &ValidationLimits,
    mode: ValidationMode,
) -> Result<(), SemanticError> {
    let mut state = State {
        limits,
        mode,
        blocks: 0,
        list_items: 0,
        inlines: 0,
        text_bytes: 0,
        targets: BTreeMap::new(),
        references: Vec::new(),
    };
    state.require(
        document.schema == SCHEMA_NAME,
        "/schema",
        "schema must be md2hwp.ir",
    )?;
    state.require(
        document.ir_version == IR_VERSION,
        "/ir_version",
        "IR version must be 0.4",
    )?;
    crate::metadata::validate(&document.metadata)
        .map_err(|(path, message)| SemanticError { path, message })?;
    for (key, value) in &document.metadata.0 {
        state.require_nfc(key, "/metadata", "metadata key")?;
        let values: Vec<&str> = match value {
            crate::ir::MetadataValue::Text(value) => vec![value.as_str()],
            crate::ir::MetadataValue::Authors(values) => {
                values.iter().map(String::as_str).collect()
            }
        };
        for value in values {
            state.require_nfc(value, &format!("/metadata/{key}"), "metadata value")?;
            state.add_limited("text bytes", &format!("/metadata/{key}"), value.len())?;
        }
    }
    let start = crate::metadata::heading1_start(&document.metadata)
        .map_err(|(path, message)| SemanticError { path, message })?;
    let headings = document
        .blocks
        .iter()
        .filter(|b| matches!(b, Block::Heading { level: 1, .. }))
        .count();
    if headings > 0 && u64::from(start) + headings as u64 - 1 > i32::MAX as u64 {
        return Err(SemanticError {
            path: "/metadata/md2hwp-heading1-start".into(),
            message: "Heading1 numbering exceeds 2147483647".into(),
        });
    }
    for (index, block) in document.blocks.iter().enumerate() {
        state.block(block, &format!("/blocks/{index}"))?;
    }
    for (kind, target, path) in &state.references {
        let (actual_kind, _) = state.targets.get(target).ok_or_else(|| SemanticError {
            path: format!("{path}/target"),
            message: format!("unknown cross-reference target '{target}'"),
        })?;
        state.require(
            kind == actual_kind,
            &format!("{path}/kind"),
            "cross-reference kind does not match the target block",
        )?;
    }
    Ok(())
}

struct State<'a> {
    limits: &'a ValidationLimits,
    mode: ValidationMode,
    blocks: usize,
    list_items: usize,
    inlines: usize,
    text_bytes: usize,
    targets: BTreeMap<String, (CrossReferenceKind, String)>,
    references: Vec<(CrossReferenceKind, String, String)>,
}

impl State<'_> {
    fn fail<T>(&self, path: &str, message: impl Into<String>) -> Result<T, SemanticError> {
        Err(SemanticError {
            path: path.to_owned(),
            message: message.into(),
        })
    }

    fn require(&self, condition: bool, path: &str, message: &str) -> Result<(), SemanticError> {
        if condition {
            Ok(())
        } else {
            self.fail(path, message)
        }
    }

    fn add_limited(
        &mut self,
        field: &'static str,
        path: &str,
        amount: usize,
    ) -> Result<(), SemanticError> {
        let (current, limit) = match field {
            "blocks" => (&mut self.blocks, self.limits.max_blocks),
            "list items" => (&mut self.list_items, self.limits.max_list_items),
            "inlines" => (&mut self.inlines, self.limits.max_inlines),
            "text bytes" => (&mut self.text_bytes, self.limits.max_text_bytes),
            _ => unreachable!(),
        };
        *current = current.checked_add(amount).ok_or_else(|| SemanticError {
            path: path.to_owned(),
            message: format!("{field} counter overflow"),
        })?;
        if *current > limit {
            return Err(SemanticError {
                path: path.to_owned(),
                message: format!("{field} limit exceeded: {current} > {limit}"),
            });
        }
        Ok(())
    }

    fn block(&mut self, block: &Block, path: &str) -> Result<(), SemanticError> {
        self.add_limited("blocks", path, 1)?;
        match block {
            Block::Paragraph { inlines } => {
                self.inline_array(inlines, &format!("{path}/inlines"), false, true, true)
            }
            Block::Heading { id, level, inlines } => {
                if let Some(id) = id {
                    self.register_id(id, CrossReferenceKind::HeadingNumber, &format!("{path}/id"))?;
                }
                self.require(
                    (1..=6).contains(level),
                    &format!("{path}/level"),
                    "heading level must be between 1 and 6",
                )?;
                self.inline_array(inlines, &format!("{path}/inlines"), false, true, true)
            }
            Block::VerbatimBlock { lines, source } => {
                self.require(
                    !lines.is_empty(),
                    &format!("{path}/lines"),
                    "verbatim block must contain at least one line",
                )?;
                for (index, line) in lines.iter().enumerate() {
                    let line_path = format!("{path}/lines/{index}");
                    self.require_nfc(line, &line_path, "verbatim-block line")?;
                    self.require(
                        !line.chars().any(is_forbidden_verbatim_control),
                        &line_path,
                        "verbatim-block line contains a forbidden control character",
                    )?;
                    self.add_limited("text bytes", &line_path, line.len())?;
                }
                if let Some(source) = source {
                    self.sources(source, &format!("{path}/source"))?;
                }
                Ok(())
            }
            Block::Table {
                columns,
                header,
                rows,
                caption,
                source,
            } => {
                self.require(
                    !columns.is_empty(),
                    &format!("{path}/columns"),
                    "table must contain at least one column",
                )?;
                self.table_row(header, columns.len(), &format!("{path}/header"))?;
                for (index, row) in rows.iter().enumerate() {
                    self.table_row(row, columns.len(), &format!("{path}/rows/{index}"))?;
                }
                if let Some(caption) = caption {
                    self.inline_array(caption, &format!("{path}/caption"), false, false, false)?;
                }
                if let Some(source) = source {
                    self.sources(source, &format!("{path}/source"))?;
                }
                Ok(())
            }
            Block::List {
                kind,
                start,
                tight,
                items,
            } => self.list(kind, *start, *tight, items, path),
            Block::Figure {
                id,
                image,
                caption,
                source,
            } => {
                if let Some(id) = id {
                    self.register_id(id, CrossReferenceKind::FigureNumber, &format!("{path}/id"))?;
                }
                self.image_path(&image.path, &format!("{path}/image/path"))?;
                if let Some(title) = &image.title {
                    self.nonempty_nfc_control_free(
                        title,
                        &format!("{path}/image/title"),
                        "image title",
                    )?;
                }
                self.inline_array(&image.alt, &format!("{path}/image/alt"), true, false, false)?;
                self.inline_array(caption, &format!("{path}/caption"), false, false, false)?;
                if let Some(source) = source {
                    self.sources(source, &format!("{path}/source"))?;
                }
                Ok(())
            }
        }
    }

    fn sources(
        &mut self,
        paragraphs: &[crate::ir::SourceParagraph],
        path: &str,
    ) -> Result<(), SemanticError> {
        self.require(
            !paragraphs.is_empty(),
            path,
            "source must contain at least one paragraph",
        )?;
        for (i, paragraph) in paragraphs.iter().enumerate() {
            let p = format!("{path}/{i}");
            self.add_limited("blocks", &p, 1)?;
            self.require(
                crate::source_prefix::is_supported(&paragraph.prefix),
                &format!("{p}/prefix"),
                "unrecognized source prefix",
            )?;
            self.require_nfc(&paragraph.prefix, &format!("{p}/prefix"), "source prefix")?;
            self.add_limited(
                "text bytes",
                &format!("{p}/prefix"),
                paragraph.prefix.len() + 2,
            )?;
            self.inline_array(
                &paragraph.inlines,
                &format!("{p}/inlines"),
                false,
                false,
                false,
            )?;
            fn visible(inlines: &[Inline]) -> bool {
                inlines.iter().any(|inline| match inline {
                    Inline::Text { value } => !value.trim().is_empty(),
                    Inline::Strong { inlines } | Inline::Emph { inlines } => visible(inlines),
                    // Empty internal-link labels are symbolic references until
                    // normalization resolves them; context validation rejects them.
                    Inline::Link {
                        target, inlines, ..
                    } => target.starts_with('#') || visible(inlines),
                    _ => false,
                })
            }
            self.require(
                visible(&paragraph.inlines),
                &format!("{p}/inlines"),
                "source paragraph content must not be empty",
            )?;
        }
        Ok(())
    }

    fn table_row(
        &mut self,
        row: &[Vec<Inline>],
        columns: usize,
        path: &str,
    ) -> Result<(), SemanticError> {
        self.add_limited("blocks", path, 1)?;
        self.require(
            row.len() == columns,
            path,
            "table row cell count must match columns",
        )?;
        for (index, cell) in row.iter().enumerate() {
            let cell_path = format!("{path}/{index}");
            self.add_limited("blocks", &cell_path, 1)?;
            self.inline_array(cell, &cell_path, true, true, true)?;
        }
        Ok(())
    }

    fn list(
        &mut self,
        kind: &ListKind,
        start: Option<u64>,
        tight: bool,
        items: &[ListItem],
        path: &str,
    ) -> Result<(), SemanticError> {
        match kind {
            ListKind::Bullet => self.require(
                start.is_none(),
                &format!("{path}/start"),
                "bullet list must omit start",
            )?,
            ListKind::Ordered => self.require(
                start.is_some_and(|value| value >= 1),
                &format!("{path}/start"),
                "ordered list start must be at least 1",
            )?,
        }
        self.require(
            !items.is_empty(),
            &format!("{path}/items"),
            "list must contain at least one item",
        )?;
        for (index, item) in items.iter().enumerate() {
            self.list_item(item, tight, &format!("{path}/items/{index}"))?;
        }
        Ok(())
    }

    fn list_item(&mut self, item: &ListItem, tight: bool, path: &str) -> Result<(), SemanticError> {
        self.add_limited("list items", path, 1)?;
        self.require(
            matches!(item.blocks.first(), Some(ListItemBlock::Paragraph { .. })),
            &format!("{path}/blocks"),
            "list item must start with a paragraph",
        )?;
        let paragraph_count = item
            .blocks
            .iter()
            .filter(|block| matches!(block, ListItemBlock::Paragraph { .. }))
            .count();
        if tight {
            self.require(
                paragraph_count == 1,
                &format!("{path}/blocks"),
                "tight list item must contain exactly one direct paragraph",
            )?;
        }
        for (index, block) in item.blocks.iter().enumerate() {
            let block_path = format!("{path}/blocks/{index}");
            self.add_limited("blocks", &block_path, 1)?;
            match block {
                ListItemBlock::Paragraph { inlines } => {
                    self.inline_array(
                        inlines,
                        &format!("{block_path}/inlines"),
                        false,
                        true,
                        true,
                    )?;
                }
                ListItemBlock::List {
                    kind,
                    start,
                    tight,
                    items,
                } => self.list(kind, *start, *tight, items, &block_path)?,
            }
        }
        Ok(())
    }

    fn inline_array(
        &mut self,
        inlines: &[Inline],
        path: &str,
        allow_empty: bool,
        allow_footnote: bool,
        allow_reference: bool,
    ) -> Result<(), SemanticError> {
        if !allow_empty {
            self.require(!inlines.is_empty(), path, "inline array must not be empty")?;
        }
        for (index, inline) in inlines.iter().enumerate() {
            self.inline(
                inline,
                &format!("{path}/{index}"),
                false,
                allow_footnote,
                allow_reference,
            )?;
        }
        Ok(())
    }

    fn inline(
        &mut self,
        inline: &Inline,
        path: &str,
        inside_link: bool,
        allow_footnote: bool,
        allow_reference: bool,
    ) -> Result<(), SemanticError> {
        self.add_limited("inlines", path, 1)?;
        match inline {
            Inline::CrossReference { kind, target } => {
                self.require(allow_reference, path,
                    "cross references are allowed only in document paragraphs, headings, list paragraphs, table cells and footnote bodies; figure alt/caption, table captions and object sources are unsupported")?;
                self.target_id(target, &format!("{path}/target"))?;
                self.references
                    .push((kind.clone(), target.clone(), path.to_owned()));
                Ok(())
            }
            Inline::Footnote { blocks } => {
                self.require(allow_footnote, path,
                    "footnotes are allowed only in document paragraphs, headings, list paragraphs and table cells; nested footnotes are not supported")?;
                self.require(
                    !blocks.is_empty(),
                    &format!("{path}/blocks"),
                    "footnote must contain at least one paragraph",
                )?;
                for (index, block) in blocks.iter().enumerate() {
                    let block_path = format!("{path}/blocks/{index}");
                    self.add_limited("blocks", &block_path, 1)?;
                    let FootnoteBlock::Paragraph { inlines } = block;
                    self.inline_array(
                        inlines,
                        &format!("{block_path}/inlines"),
                        false,
                        false,
                        true,
                    )?;
                }
                Ok(())
            }
            Inline::Text { value } => {
                self.require(
                    !value.is_empty(),
                    &format!("{path}/value"),
                    "text must not be empty",
                )?;
                self.require_nfc(value, &format!("{path}/value"), "text")?;
                self.require(
                    !value.chars().any(|ch| ch == ' ' || is_control(ch)),
                    &format!("{path}/value"),
                    "text contains a space or control character",
                )?;
                self.add_limited("text bytes", &format!("{path}/value"), value.len())
            }
            Inline::Space | Inline::LineBreak => Ok(()),
            Inline::Strong { inlines } | Inline::Emph { inlines } => {
                self.require(
                    !inlines.is_empty(),
                    &format!("{path}/inlines"),
                    "marked inline array must not be empty",
                )?;
                for (index, child) in inlines.iter().enumerate() {
                    self.inline(
                        child,
                        &format!("{path}/inlines/{index}"),
                        inside_link,
                        allow_footnote,
                        allow_reference,
                    )?;
                }
                Ok(())
            }
            Inline::Link {
                target,
                title,
                inlines,
            } => {
                self.require(!inside_link, path, "links must not be nested")?;
                self.nonempty_control_free(target, &format!("{path}/target"), "link target")?;
                if let Some(title) = title {
                    self.nonempty_nfc_control_free(title, &format!("{path}/title"), "link title")?;
                }
                self.require(
                    !inlines.is_empty()
                        || self.mode == ValidationMode::PandocSource && target.starts_with('#'),
                    &format!("{path}/inlines"),
                    "link label must not be empty",
                )?;
                for (index, child) in inlines.iter().enumerate() {
                    self.inline(
                        child,
                        &format!("{path}/inlines/{index}"),
                        true,
                        allow_footnote,
                        allow_reference,
                    )?;
                }
                Ok(())
            }
        }
    }

    fn target_id(&mut self, value: &str, path: &str) -> Result<(), SemanticError> {
        self.require(
            is_target_id(value),
            path,
            "ID must be nonempty and contain no whitespace, controls or '#'",
        )?;
        self.add_limited("text bytes", path, value.len())
    }

    fn register_id(
        &mut self,
        value: &str,
        kind: CrossReferenceKind,
        path: &str,
    ) -> Result<(), SemanticError> {
        self.target_id(value, path)?;
        if let Some((_, previous)) = self
            .targets
            .insert(value.to_owned(), (kind, path.to_owned()))
        {
            return self.fail(
                path,
                format!("duplicate target ID '{value}'; first declared at {previous}"),
            );
        }
        Ok(())
    }

    fn nonempty_control_free(
        &mut self,
        value: &str,
        path: &str,
        label: &str,
    ) -> Result<(), SemanticError> {
        self.require(
            !value.is_empty(),
            path,
            &format!("{label} must not be empty"),
        )?;
        self.require(
            !value.chars().any(is_control),
            path,
            &format!("{label} contains a control character"),
        )?;
        self.add_limited("text bytes", path, value.len())
    }

    fn nonempty_nfc_control_free(
        &mut self,
        value: &str,
        path: &str,
        label: &str,
    ) -> Result<(), SemanticError> {
        self.require_nfc(value, path, label)?;
        self.nonempty_control_free(value, path, label)
    }

    fn require_nfc(&self, value: &str, path: &str, label: &str) -> Result<(), SemanticError> {
        self.require(
            NFC.is_normalized(value),
            path,
            &format!("{label} must use Unicode NFC"),
        )
    }

    fn image_path(&mut self, value: &str, path: &str) -> Result<(), SemanticError> {
        self.require(!value.is_empty(), path, "image path must not be empty")?;
        let has_scheme = value.find(':').is_some_and(|colon| {
            let scheme = &value.as_bytes()[..colon];
            scheme.first().is_some_and(u8::is_ascii_alphabetic)
                && scheme[1..]
                    .iter()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'.' | b'-'))
        });
        self.require(
            !value.starts_with('/')
                && !value.contains('\\')
                && !has_scheme
                && !value.chars().any(is_control),
            path,
            "image path must be a canonical relative local path using / separators",
        )?;
        self.add_limited("text bytes", path, value.len())
    }
}

fn is_control(ch: char) -> bool {
    matches!(ch as u32, 0x00..=0x1f | 0x7f)
}

fn is_forbidden_verbatim_control(ch: char) -> bool {
    is_control(ch) && ch != '\t'
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{ImageRef, Metadata};

    fn document(blocks: Vec<Block>) -> Document {
        Document {
            schema: SCHEMA_NAME.to_owned(),
            ir_version: IR_VERSION.to_owned(),
            metadata: Metadata::default(),
            blocks,
        }
    }

    fn text(value: &str) -> Inline {
        Inline::Text {
            value: value.to_owned(),
        }
    }

    #[test]
    fn rejects_nested_links_with_an_rfc6901_path() {
        let nested = Inline::Link {
            target: "https://outer.example".to_owned(),
            title: None,
            inlines: vec![Inline::Strong {
                inlines: vec![Inline::Link {
                    target: "https://inner.example".to_owned(),
                    title: None,
                    inlines: vec![text("label")],
                }],
            }],
        };
        let error = validate(
            document(vec![Block::Paragraph {
                inlines: vec![nested],
            }]),
            &ValidationLimits::default(),
        )
        .unwrap_err();
        assert_eq!(error.path, "/blocks/0/inlines/0/inlines/0/inlines/0");
        assert!(error.message.contains("nested"));
    }

    #[test]
    fn rejects_extra_paragraphs_in_tight_list_items() {
        let item = ListItem {
            blocks: vec![
                ListItemBlock::Paragraph {
                    inlines: vec![text("first")],
                },
                ListItemBlock::Paragraph {
                    inlines: vec![text("second")],
                },
            ],
        };
        let error = validate(
            document(vec![Block::List {
                kind: ListKind::Bullet,
                start: None,
                tight: true,
                items: vec![item],
            }]),
            &ValidationLimits::default(),
        )
        .unwrap_err();
        assert_eq!(error.path, "/blocks/0/items/0/blocks");
        assert!(error.message.contains("exactly one"));
    }

    #[test]
    fn rejects_noncanonical_direct_ir_image_paths() {
        for path in [
            "C:/asset.png",
            "https://example/asset.png",
            "dir\\asset.png",
        ] {
            let error = validate(
                document(vec![Block::Figure {
                    id: None,
                    image: ImageRef {
                        path: path.to_owned(),
                        alt: vec![],
                        title: None,
                    },
                    caption: vec![text("caption")],
                    source: None,
                }]),
                &ValidationLimits::default(),
            )
            .unwrap_err();
            assert_eq!(error.path, "/blocks/0/image/path");
        }
    }

    #[test]
    fn rejects_non_nfc_human_readable_strings_at_exact_paths() {
        let cases = vec![
            (
                Block::Paragraph {
                    inlines: vec![text("가")],
                },
                "/blocks/0/inlines/0/value",
            ),
            (
                Block::VerbatimBlock {
                    lines: vec!["가".to_owned()],
                    source: None,
                },
                "/blocks/0/lines/0",
            ),
            (
                Block::Paragraph {
                    inlines: vec![Inline::Link {
                        target: "https://example.test".to_owned(),
                        title: Some("가".to_owned()),
                        inlines: vec![text("label")],
                    }],
                },
                "/blocks/0/inlines/0/title",
            ),
            (
                Block::Paragraph {
                    inlines: vec![Inline::Link {
                        target: "https://example.test".to_owned(),
                        title: None,
                        inlines: vec![text("가")],
                    }],
                },
                "/blocks/0/inlines/0/inlines/0/value",
            ),
            (
                Block::Figure {
                    id: None,
                    image: ImageRef {
                        path: "assets/image.png".to_owned(),
                        alt: vec![],
                        title: Some("가".to_owned()),
                    },
                    caption: vec![text("caption")],
                    source: None,
                },
                "/blocks/0/image/title",
            ),
            (
                Block::Figure {
                    id: None,
                    image: ImageRef {
                        path: "assets/image.png".to_owned(),
                        alt: vec![text("가")],
                        title: None,
                    },
                    caption: vec![text("caption")],
                    source: None,
                },
                "/blocks/0/image/alt/0/value",
            ),
            (
                Block::Figure {
                    id: None,
                    image: ImageRef {
                        path: "assets/image.png".to_owned(),
                        alt: vec![],
                        title: None,
                    },
                    caption: vec![text("가")],
                    source: None,
                },
                "/blocks/0/caption/0/value",
            ),
            (
                Block::Figure {
                    id: None,
                    image: ImageRef {
                        path: "assets/image.png".to_owned(),
                        alt: vec![],
                        title: None,
                    },
                    caption: vec![text("caption")],
                    source: Some(vec![crate::ir::SourceParagraph {
                        prefix: "출처".into(),
                        inlines: vec![text("가")],
                    }]),
                },
                "/blocks/0/source/0/inlines/0/value",
            ),
        ];

        for (block, expected_path) in cases {
            let error = validate(document(vec![block]), &ValidationLimits::default()).unwrap_err();
            assert_eq!(error.path, expected_path);
            assert!(error.message.contains("Unicode NFC"));
        }
    }

    #[test]
    fn preserves_non_nfc_link_targets_and_image_paths() {
        let target = "https://example.test/가";
        let image_path = "assets/가.png";
        let validated = validate(
            document(vec![
                Block::Paragraph {
                    inlines: vec![Inline::Link {
                        target: target.to_owned(),
                        title: None,
                        inlines: vec![text("label")],
                    }],
                },
                Block::Figure {
                    id: None,
                    image: ImageRef {
                        path: image_path.to_owned(),
                        alt: vec![],
                        title: None,
                    },
                    caption: vec![text("caption")],
                    source: None,
                },
            ]),
            &ValidationLimits::default(),
        )
        .unwrap()
        .into_document();

        let Block::Paragraph { inlines } = &validated.blocks[0] else {
            panic!("expected paragraph")
        };
        let Inline::Link {
            target: validated_target,
            ..
        } = &inlines[0]
        else {
            panic!("expected link")
        };
        assert_eq!(validated_target, target);

        let Block::Figure { image, .. } = &validated.blocks[1] else {
            panic!("expected figure")
        };
        assert_eq!(image.path, image_path);
    }

    #[test]
    fn enforces_cumulative_inline_and_text_limits() {
        let input = document(vec![Block::Paragraph {
            inlines: vec![text("가"), Inline::Space, text("나")],
        }]);
        let limits = ValidationLimits {
            max_inlines: 2,
            ..ValidationLimits::default()
        };
        assert!(
            validate(input.clone(), &limits)
                .unwrap_err()
                .message
                .contains("inlines limit")
        );

        let limits = ValidationLimits {
            max_inlines: usize::MAX,
            max_text_bytes: 5,
            ..ValidationLimits::default()
        };
        assert!(
            validate(input, &limits)
                .unwrap_err()
                .message
                .contains("text bytes limit")
        );
    }

    #[test]
    fn accepts_tabs_and_blank_lines_in_verbatim_blocks() {
        let input = document(vec![Block::VerbatimBlock {
            lines: vec!["첫째\t열".to_owned(), "".to_owned()],
            source: None,
        }]);
        validate(input, &ValidationLimits::default()).unwrap();
    }
    fn footnote(inlines: Vec<Inline>) -> Inline {
        Inline::Footnote {
            blocks: vec![FootnoteBlock::Paragraph { inlines }],
        }
    }

    #[test]
    fn footnotes_share_cumulative_block_inline_and_text_limits() {
        let input = document(vec![Block::Paragraph {
            inlines: vec![footnote(vec![text("abcdef")])],
        }]);
        for (limits, expected) in [
            (
                ValidationLimits {
                    max_blocks: 1,
                    ..ValidationLimits::default()
                },
                "blocks limit",
            ),
            (
                ValidationLimits {
                    max_inlines: 1,
                    ..ValidationLimits::default()
                },
                "inlines limit",
            ),
            (
                ValidationLimits {
                    max_text_bytes: 5,
                    ..ValidationLimits::default()
                },
                "text bytes limit",
            ),
        ] {
            assert!(
                validate(input.clone(), &limits)
                    .unwrap_err()
                    .message
                    .contains(expected)
            );
        }
    }

    #[test]
    fn validates_note_body_text_and_rejects_empty_or_nested_notes() {
        let error = validate(
            document(vec![Block::Paragraph {
                inlines: vec![footnote(vec![text("가")])],
            }]),
            &ValidationLimits::default(),
        )
        .unwrap_err();
        assert_eq!(error.path, "/blocks/0/inlines/0/blocks/0/inlines/0/value");
        assert!(error.message.contains("Unicode NFC"));
        for note in [
            Inline::Footnote { blocks: vec![] },
            footnote(vec![]),
            footnote(vec![Inline::Strong {
                inlines: vec![footnote(vec![text("nested")])],
            }]),
        ] {
            assert!(
                validate(
                    document(vec![Block::Paragraph {
                        inlines: vec![note]
                    }]),
                    &ValidationLimits::default()
                )
                .is_err()
            );
        }
    }

    #[test]
    fn rejects_direct_ir_notes_in_every_figure_and_source_field() {
        let figure = Block::Figure {
            id: None,
            image: ImageRef {
                path: "image.png".into(),
                alt: vec![],
                title: None,
            },
            caption: vec![text("caption")],
            source: None,
        };
        for field in ["alt", "caption", "source"] {
            let mut block = figure.clone();
            let Block::Figure {
                image,
                caption,
                source,
                ..
            } = &mut block
            else {
                unreachable!()
            };
            let value = vec![footnote(vec![text("note")])];
            match field {
                "alt" => image.alt = value,
                "caption" => *caption = value,
                "source" => {
                    *source = Some(vec![crate::ir::SourceParagraph {
                        prefix: "출처".into(),
                        inlines: value,
                    }])
                }
                _ => unreachable!(),
            }
            let error = validate(document(vec![block]), &ValidationLimits::default()).unwrap_err();
            assert!(error.message.contains("footnotes are allowed only"));
        }
    }
}
