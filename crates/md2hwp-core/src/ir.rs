//! Closed, backend-neutral IR types with version-specific validation.

use serde::{Deserialize, Serialize};

pub const SCHEMA_NAME: &str = "md2hwp.ir";
pub const IR_VERSION: &str = "0.3";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Document {
    pub schema: String,
    pub ir_version: String,
    pub metadata: Metadata,
    pub blocks: Vec<Block>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Metadata(pub std::collections::BTreeMap<String, MetadataValue>);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MetadataValue {
    Text(String),
    Authors(Vec<String>),
}

impl Metadata {
    pub fn text(&self, key: &str) -> Option<&str> {
        match self.0.get(key) {
            Some(MetadataValue::Text(text)) => Some(text),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Block {
    Paragraph {
        inlines: Vec<Inline>,
    },
    Heading {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        level: u8,
        inlines: Vec<Inline>,
    },
    VerbatimBlock {
        lines: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        source: Option<Vec<Inline>>,
    },
    List {
        kind: ListKind,
        #[serde(skip_serializing_if = "Option::is_none")]
        start: Option<u64>,
        tight: bool,
        items: Vec<ListItem>,
    },
    Table {
        columns: Vec<TableAlignment>,
        header: Vec<Vec<Inline>>,
        rows: Vec<Vec<Vec<Inline>>>,
        caption: Option<Vec<Inline>>,
        source: Option<Vec<Inline>>,
    },
    Figure {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        image: ImageRef,
        caption: Vec<Inline>,
        source: Option<Vec<Inline>>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TableAlignment {
    Default,
    Left,
    Center,
    Right,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ListKind {
    Bullet,
    Ordered,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListItem {
    pub blocks: Vec<ListItemBlock>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ListItemBlock {
    Paragraph {
        inlines: Vec<Inline>,
    },
    List {
        kind: ListKind,
        #[serde(skip_serializing_if = "Option::is_none")]
        start: Option<u64>,
        tight: bool,
        items: Vec<ListItem>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImageRef {
    pub path: String,
    pub alt: Vec<Inline>,
    pub title: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum FootnoteBlock {
    Paragraph { inlines: Vec<Inline> },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Inline {
    CrossReference {
        kind: CrossReferenceKind,
        target: String,
    },
    Footnote {
        blocks: Vec<FootnoteBlock>,
    },
    Text {
        value: String,
    },
    Space,
    LineBreak,
    Strong {
        inlines: Vec<Inline>,
    },
    Emph {
        inlines: Vec<Inline>,
    },
    Link {
        target: String,
        title: Option<String>,
        inlines: Vec<Inline>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrossReferenceKind {
    HeadingNumber,
    FigureNumber,
}

pub(crate) fn is_target_id(value: &str) -> bool {
    !value.is_empty()
        && !value
            .chars()
            .any(|ch| ch.is_whitespace() || ch.is_control() || ch == '#')
}
