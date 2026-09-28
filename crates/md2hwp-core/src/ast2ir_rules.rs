//! Loading and closed typed decoding of the product-owned AST-to-IR ruleset.

use std::collections::BTreeMap;
use std::fmt;

use jsonschema::{Draft, JSONSchema};
use serde::Deserialize;
use serde_json::Value;

use crate::ir::{IR_VERSION, SCHEMA_NAME};
use crate::ir_io::read_strict_json_value;
use crate::validate::ValidationLimits;

const RULES_JSON: &str = include_str!("../../../rules/ast2ir/ir-v0.2.json");
const RULES_SCHEMA: &str = include_str!("../../../schemas/ast2ir-rules.schema.json");

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ast2IrRules {
    pub schema: String,
    pub target_ir: TargetIr,
    pub unlisted_constructor: RejectPolicy,
    pub document: DocumentRules,
    pub blocks: BTreeMap<String, BlockRule>,
    pub inlines: BTreeMap<String, InlineRule>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetIr {
    pub schema: String,
    pub version: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RejectPolicy {
    Reject,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentRules {
    pub metadata: RequireEmpty,
    pub object_sources: ObjectSources,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectSources {
    pub handler: String,
    pub prefix: String,
    pub separator: String,
    pub empty: String,
    pub owners: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StandaloneFigure {
    pub handler: String,
    pub attributes: RequireEmpty,
    pub caption: String,
    pub context: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequireEmpty {
    RequireEmpty,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(tag = "handler", rename_all = "snake_case", deny_unknown_fields)]
pub enum BlockRule {
    Paragraph {
        figure: StandaloneFigure,
    },
    Heading {
        minimum_level: u8,
        maximum_level: u8,
        attributes: RequireEmpty,
    },
    ListItemParagraph {
        context: ListItemOnly,
    },
    VerbatimBlock {
        attributes: RequireEmpty,
    },
    BulletList,
    OrderedList {
        minimum_start: u64,
        number_styles: Vec<NumberStyle>,
        delimiters: Vec<NumberDelimiter>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ListItemOnly {
    ListItemOnly,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub enum NumberStyle {
    DefaultStyle,
    Decimal,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub enum NumberDelimiter {
    DefaultDelim,
    Period,
    OneParen,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(tag = "handler", rename_all = "snake_case", deny_unknown_fields)]
pub enum InlineRule {
    Text,
    Space,
    LineBreak,
    Strong,
    Emph,
    Reject,
    Link {
        attributes: RequireEmpty,
        target: RequireNonempty,
        absent_title: NullPolicy,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequireNonempty {
    RequireNonempty,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NullPolicy {
    Null,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RulesError {
    pub path: String,
    pub message: String,
}

impl fmt::Display for RulesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid AST-to-IR rules at {}: {}",
            self.path, self.message
        )
    }
}

impl std::error::Error for RulesError {}

pub fn load_builtin_rules() -> Result<Ast2IrRules, RulesError> {
    let value = read_strict_json_value(
        RULES_JSON.as_bytes(),
        &ValidationLimits::default(),
        "AST-to-IR rules",
    )
    .map_err(|error| rules_error("", error.message))?;
    let schema: Value = serde_json::from_str(RULES_SCHEMA).map_err(|error| {
        rules_error(
            "",
            format!("embedded rules schema is invalid JSON: {error}"),
        )
    })?;
    let compiled = JSONSchema::options()
        .with_draft(Draft::Draft202012)
        .compile(&schema)
        .map_err(|error| {
            rules_error(
                "",
                format!("embedded rules schema could not be compiled: {error}"),
            )
        })?;
    if let Err(errors) = compiled.validate(&value)
        && let Some(error) = errors.into_iter().next()
    {
        return Err(rules_error(
            &error.instance_path.to_string(),
            error.to_string(),
        ));
    }
    let rules: Ast2IrRules = serde_json::from_value(value)
        .map_err(|error| rules_error("", format!("typed rules decoding failed: {error}")))?;
    if rules.schema != "md2hwp.ast2ir-rules" {
        return Err(rules_error("", "unsupported rules envelope"));
    }
    if rules.target_ir.schema != SCHEMA_NAME || rules.target_ir.version != IR_VERSION {
        return Err(rules_error(
            "/target_ir",
            "rules target does not match IR 0.2",
        ));
    }
    if let Some(BlockRule::Heading {
        minimum_level,
        maximum_level,
        ..
    }) = rules.blocks.get("Header")
        && minimum_level > maximum_level
    {
        return Err(rules_error(
            "/blocks/Header",
            "minimum_level exceeds maximum_level",
        ));
    }
    Ok(rules)
}

fn rules_error(path: &str, message: impl Into<String>) -> RulesError {
    RulesError {
        path: path.to_owned(),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_the_exact_builtin_ruleset() {
        let rules = load_builtin_rules().unwrap();
        assert_eq!(rules.target_ir.version, IR_VERSION);
        let envelope: Value = serde_json::from_str(RULES_JSON).unwrap();
        assert!(envelope.get("rules_version").is_none());
        assert_eq!(rules.blocks.len(), 6);
        assert_eq!(rules.inlines.len(), 7);
        assert!(matches!(
            rules.inlines.get("SoftBreak"),
            Some(InlineRule::Space)
        ));
        assert!(matches!(
            rules.blocks.get("CodeBlock"),
            Some(BlockRule::VerbatimBlock { .. })
        ));
    }
}
