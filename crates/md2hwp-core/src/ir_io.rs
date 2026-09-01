//! Strict UTF-8 JSON input and validated-only output for IR v0.1.

use std::collections::HashSet;
use std::fmt;

use jsonschema::{Draft, JSONSchema};
use serde::de::{DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};

use crate::ir::{Document, IR_VERSION, SCHEMA_NAME};
use crate::validate::{SemanticError, ValidatedDocument, ValidationLimits, validate};

const IR_SCHEMA: &str = include_str!("../../../schemas/ir-v0.1.schema.json");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum IrReadErrorCode {
    UnsupportedIrVersion,
    InvalidIrSchema,
    InvalidIrSemantics,
}

impl IrReadErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UnsupportedIrVersion => "unsupported_ir_version",
            Self::InvalidIrSchema => "invalid_ir_schema",
            Self::InvalidIrSemantics => "invalid_ir_semantics",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrReadError {
    pub code: IrReadErrorCode,
    pub path: String,
    pub message: String,
}

impl fmt::Display for IrReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at {}: {}",
            self.code.as_str(),
            self.path,
            self.message
        )
    }
}

impl std::error::Error for IrReadError {}

#[derive(Debug)]
pub struct IrWriteError(serde_json::Error);

impl fmt::Display for IrWriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "could not serialize validated IR: {}", self.0)
    }
}

impl std::error::Error for IrWriteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StrictJsonError {
    pub message: String,
    pub resource_limit: bool,
}

pub(crate) fn read_strict_json_value(
    bytes: &[u8],
    limits: &ValidationLimits,
    label: &str,
) -> Result<Value, StrictJsonError> {
    if bytes.len() > limits.max_json_bytes {
        return Err(StrictJsonError {
            message: format!(
                "JSON byte limit exceeded: {} > {}",
                bytes.len(),
                limits.max_json_bytes
            ),
            resource_limit: true,
        });
    }
    let text = std::str::from_utf8(bytes).map_err(|error| StrictJsonError {
        message: format!("{label} is not UTF-8: {error}"),
        resource_limit: false,
    })?;
    let mut deserializer = serde_json::Deserializer::from_str(text);
    // The project limit below replaces serde_json's fixed recursion limit.
    deserializer.disable_recursion_limit();
    let value = StrictValueSeed {
        depth: 0,
        max_depth: limits.max_json_nesting,
    }
    .deserialize(&mut deserializer)
    .map_err(|error| {
        let message = error.to_string();
        StrictJsonError {
            resource_limit: message.contains("JSON nesting limit exceeded"),
            message,
        }
    })?;
    deserializer.end().map_err(|error| StrictJsonError {
        message: error.to_string(),
        resource_limit: false,
    })?;
    Ok(value)
}

pub fn read_ir(bytes: &[u8], limits: &ValidationLimits) -> Result<ValidatedDocument, IrReadError> {
    let value = read_strict_json_value(bytes, limits, "IR").map_err(|error| {
        if error.resource_limit {
            semantic_error("", error.message)
        } else {
            schema_error("", error.message)
        }
    })?;

    inspect_envelope(&value)?;
    validate_schema(&value)?;
    let document: Document = serde_json::from_value(value).map_err(|error| {
        schema_error(
            "",
            format!("typed IR decoding failed after schema validation: {error}"),
        )
    })?;
    validate(document, limits).map_err(from_semantic_error)
}

pub fn write_ir(document: &ValidatedDocument) -> Result<Vec<u8>, IrWriteError> {
    let mut bytes = serde_json::to_vec_pretty(document.as_document()).map_err(IrWriteError)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn inspect_envelope(value: &Value) -> Result<(), IrReadError> {
    let object = value
        .as_object()
        .ok_or_else(|| schema_error("", "IR root must be an object"))?;
    match object.get("schema") {
        Some(Value::String(schema)) if schema == SCHEMA_NAME => {}
        Some(actual) => {
            return Err(schema_error(
                "/schema",
                format!("expected {SCHEMA_NAME:?}, got {actual}"),
            ));
        }
        None => return Err(schema_error("/schema", "missing schema member")),
    }
    match object.get("ir_version") {
        Some(Value::String(version)) if version == IR_VERSION => Ok(()),
        Some(Value::String(version)) => Err(IrReadError {
            code: IrReadErrorCode::UnsupportedIrVersion,
            path: "/ir_version".to_owned(),
            message: format!("unsupported IR version {version:?}; expected {IR_VERSION:?}"),
        }),
        Some(actual) => Err(schema_error(
            "/ir_version",
            format!("IR version must be a string, got {actual}"),
        )),
        None => Err(schema_error("/ir_version", "missing ir_version member")),
    }
}

fn validate_schema(value: &Value) -> Result<(), IrReadError> {
    let schema: Value = serde_json::from_str(IR_SCHEMA).map_err(|error| {
        schema_error("", format!("embedded IR schema is invalid JSON: {error}"))
    })?;
    let compiled = JSONSchema::options()
        .with_draft(Draft::Draft202012)
        .compile(&schema)
        .map_err(|error| {
            schema_error(
                "",
                format!("embedded IR schema could not be compiled: {error}"),
            )
        })?;
    if let Err(errors) = compiled.validate(value)
        && let Some(error) = errors.into_iter().next()
    {
        return Err(schema_error(
            &error.instance_path.to_string(),
            error.to_string(),
        ));
    }
    Ok(())
}

fn schema_error(path: &str, message: impl Into<String>) -> IrReadError {
    IrReadError {
        code: IrReadErrorCode::InvalidIrSchema,
        path: path.to_owned(),
        message: message.into(),
    }
}

fn semantic_error(path: &str, message: impl Into<String>) -> IrReadError {
    IrReadError {
        code: IrReadErrorCode::InvalidIrSemantics,
        path: path.to_owned(),
        message: message.into(),
    }
}

fn from_semantic_error(error: SemanticError) -> IrReadError {
    semantic_error(&error.path, error.message)
}

struct StrictValueSeed {
    depth: usize,
    max_depth: usize,
}

impl<'de> DeserializeSeed<'de> for StrictValueSeed {
    type Value = Value;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        if self.depth > self.max_depth {
            return Err(serde::de::Error::custom(format!(
                "JSON nesting limit exceeded: {} > {}",
                self.depth, self.max_depth
            )));
        }
        deserializer.deserialize_any(StrictValueVisitor {
            depth: self.depth,
            max_depth: self.max_depth,
        })
    }
}

struct StrictValueVisitor {
    depth: usize,
    max_depth: usize,
}

impl<'de> Visitor<'de> for StrictValueVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Value, E> {
        Ok(Value::Bool(value))
    }
    fn visit_i64<E>(self, value: i64) -> Result<Value, E> {
        Ok(Value::Number(value.into()))
    }
    fn visit_u64<E>(self, value: u64) -> Result<Value, E> {
        Ok(Value::Number(value.into()))
    }
    fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<Value, E> {
        Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| E::custom("non-finite JSON number"))
    }
    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Value, E> {
        Ok(Value::String(value.to_owned()))
    }
    fn visit_string<E>(self, value: String) -> Result<Value, E> {
        Ok(Value::String(value))
    }
    fn visit_none<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_unit<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::with_capacity(sequence.size_hint().unwrap_or(0));
        let seed_depth = self.depth + 1;
        while let Some(value) = sequence.next_element_seed(StrictValueSeed {
            depth: seed_depth,
            max_depth: self.max_depth,
        })? {
            values.push(value);
        }
        Ok(Value::Array(values))
    }

    fn visit_map<A>(self, mut object: A) -> Result<Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut values = Map::new();
        let mut keys = HashSet::new();
        let seed_depth = self.depth + 1;
        while let Some(key) = object.next_key::<String>()? {
            if !keys.insert(key.clone()) {
                return Err(serde::de::Error::custom(format!(
                    "duplicate object member {key:?}"
                )));
            }
            let value = object.next_value_seed(StrictValueSeed {
                depth: seed_depth,
                max_depth: self.max_depth,
            })?;
            values.insert(key, value);
        }
        Ok(Value::Object(values))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{Block, Inline};

    const EXAMPLE: &[u8] = include_bytes!("../../../examples/ir-v0.1.json");
    const COMMONMARK_EXAMPLE: &[u8] =
        include_bytes!("../../../examples/commonmark-v0.1.expected.ir.json");
    const NON_NFC_EXAMPLE: &[u8] =
        include_bytes!("../../../examples/ir-v0.1-rejected-non-nfc.json");

    #[test]
    fn reads_and_writes_normative_nfc_example() {
        let limits = ValidationLimits::default();
        let document = read_ir(EXAMPLE, &limits).expect("normative example must decode");
        let Block::Paragraph { inlines, .. } = &document.as_document().blocks[1] else {
            panic!("expected paragraph")
        };
        assert!(inlines.contains(&Inline::Text {
            value: "가".to_owned()
        }));
        assert!(!inlines.contains(&Inline::Text {
            value: "가".to_owned()
        }));
        let encoded = write_ir(&document).expect("validated IR must serialize");
        assert_eq!(read_ir(&encoded, &limits).unwrap(), document);
    }

    #[test]
    fn rejects_literal_and_escaped_non_nfc_text_as_invalid_ir_semantics() {
        let escaped = br#"{"schema":"md2hwp.ir","ir_version":"0.1","metadata":{},"blocks":[{"type":"paragraph","inlines":[{"type":"text","value":"\u1100\u1161"}]}]}"#;
        for input in [NON_NFC_EXAMPLE, escaped] {
            let error = read_ir(input, &ValidationLimits::default()).unwrap_err();
            assert_eq!(error.code, IrReadErrorCode::InvalidIrSemantics);
            assert_eq!(error.path, "/blocks/0/inlines/0/value");
            assert!(error.message.contains("Unicode NFC"));
        }
    }

    #[test]
    fn preserves_non_nfc_link_targets_and_image_paths() {
        let input = br#"{"schema":"md2hwp.ir","ir_version":"0.1","metadata":{},"blocks":[{"type":"paragraph","inlines":[{"type":"link","target":"https://example.test/\u1100\u1161","title":null,"inlines":[{"type":"text","value":"label"}]}]},{"type":"figure","image":{"path":"assets/\u1100\u1161.png","alt":[],"title":null},"caption":[{"type":"text","value":"caption"}],"source":null}]}"#;
        let limits = ValidationLimits::default();
        let document = read_ir(input, &limits).expect("NFD identifiers must remain valid");

        let Block::Paragraph { inlines } = &document.as_document().blocks[0] else {
            panic!("expected paragraph")
        };
        let Inline::Link { target, .. } = &inlines[0] else {
            panic!("expected link")
        };
        assert_eq!(target, "https://example.test/가");

        let Block::Figure { image, .. } = &document.as_document().blocks[1] else {
            panic!("expected figure")
        };
        assert_eq!(image.path, "assets/가.png");

        let encoded = write_ir(&document).expect("validated IR must serialize");
        assert_eq!(read_ir(&encoded, &limits).unwrap(), document);
    }

    #[test]
    fn reads_and_writes_commonmark_golden_example() {
        let limits = ValidationLimits::default();
        let document = read_ir(COMMONMARK_EXAMPLE, &limits)
            .expect("CommonMark golden example must satisfy the IR contract");
        assert!(matches!(
            document.as_document().blocks[2],
            Block::VerbatimBlock { .. }
        ));
        let encoded = write_ir(&document).expect("validated IR must serialize");
        assert_eq!(read_ir(&encoded, &limits).unwrap(), document);
    }

    #[test]
    fn rejects_duplicate_members() {
        let json = br#"{"schema":"md2hwp.ir","schema":"md2hwp.ir","ir_version":"0.1","metadata":{},"blocks":[]}"#;
        let error = read_ir(json, &ValidationLimits::default()).unwrap_err();
        assert_eq!(error.code, IrReadErrorCode::InvalidIrSchema);
        assert!(error.message.contains("duplicate object member"));
    }

    #[test]
    fn reports_unsupported_version_before_schema_validation() {
        let json =
            br#"{"schema":"md2hwp.ir","ir_version":"0.2","metadata":{},"blocks":[],"future":true}"#;
        let error = read_ir(json, &ValidationLimits::default()).unwrap_err();
        assert_eq!(error.code, IrReadErrorCode::UnsupportedIrVersion);
        assert_eq!(error.path, "/ir_version");
    }

    #[test]
    fn rejects_unknown_node_and_member_through_closed_schema() {
        for json in [
            include_bytes!("../../../examples/ir-v0.1-rejected-page-break.json").as_slice(),
            include_bytes!("../../../examples/ir-v0.1-rejected-soft-break.json").as_slice(),
            br#"{"schema":"md2hwp.ir","ir_version":"0.1","metadata":{},"blocks":[],"extra":1}"#
                .as_slice(),
            br#"{"schema":"md2hwp.ir","ir_version":"0.1","metadata":{},"blocks":[{"type":"paragraph","style":"body","inlines":[{"type":"text","value":"legacy"}]}]}"#
                .as_slice(),
            br#"{"schema":"md2hwp.ir","ir_version":"0.1","metadata":{},"blocks":[{"type":"styled_block","role":"block.box","lines":["legacy"]}]}"#
                .as_slice(),
        ] {
            assert_eq!(
                read_ir(json, &ValidationLimits::default())
                    .unwrap_err()
                    .code,
                IrReadErrorCode::InvalidIrSchema
            );
        }
    }

    #[test]
    fn rejects_non_utf8_before_json_parsing() {
        let error = read_ir(&[0xff], &ValidationLimits::default()).unwrap_err();
        assert_eq!(error.code, IrReadErrorCode::InvalidIrSchema);
        assert!(error.message.contains("not UTF-8"));
    }

    #[test]
    fn enforces_json_byte_and_nesting_limits() {
        let limits = ValidationLimits {
            max_json_bytes: 10,
            ..ValidationLimits::default()
        };
        assert_eq!(
            read_ir(EXAMPLE, &limits).unwrap_err().code,
            IrReadErrorCode::InvalidIrSemantics
        );

        let limits = ValidationLimits {
            max_json_bytes: usize::MAX,
            max_json_nesting: 0,
            ..ValidationLimits::default()
        };
        assert_eq!(
            read_ir(
                br#"{"schema":"md2hwp.ir","ir_version":"0.1","metadata":{},"blocks":[]}"#,
                &limits
            )
            .unwrap_err()
            .code,
            IrReadErrorCode::InvalidIrSemantics
        );
    }
}
