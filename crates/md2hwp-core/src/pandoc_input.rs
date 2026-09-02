//! Pandoc JSON envelope decoding and API-version checks.

use std::fmt;

use serde_json::{Map, Value};

use crate::ir_io::read_strict_json_value;
use crate::validate::ValidationLimits;

pub const PANDOC_API_VERSION: [u64; 4] = [1, 23, 1, 2];

#[derive(Clone, Debug)]
pub struct PandocDocument {
    pub(crate) metadata: Map<String, Value>,
    pub(crate) blocks: Vec<Value>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PandocInputError {
    pub code: &'static str,
    pub path: String,
    pub message: String,
}

impl fmt::Display for PandocInputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at {}: {}", self.code, self.path, self.message)
    }
}

impl std::error::Error for PandocInputError {}

pub fn read_pandoc_json(
    bytes: &[u8],
    limits: &ValidationLimits,
) -> Result<PandocDocument, PandocInputError> {
    let value = read_strict_json_value(bytes, limits, "Pandoc JSON")
        .map_err(|error| input_error("invalid_pandoc_json", "", error.message))?;
    let root = value.as_object().ok_or_else(|| {
        input_error(
            "invalid_pandoc_json",
            "",
            "Pandoc JSON root must be an object",
        )
    })?;
    let version = root
        .get("pandoc-api-version")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            input_error(
                "unsupported_pandoc_api_version",
                "/pandoc-api-version",
                "missing or invalid Pandoc API version",
            )
        })?;
    let actual: Option<Vec<u64>> = version.iter().map(Value::as_u64).collect();
    if actual.as_deref() != Some(PANDOC_API_VERSION.as_slice()) {
        return Err(input_error(
            "unsupported_pandoc_api_version",
            "/pandoc-api-version",
            format!("expected {:?}, got {:?}", PANDOC_API_VERSION, actual),
        ));
    }
    let metadata = root
        .get("meta")
        .and_then(Value::as_object)
        .cloned()
        .ok_or_else(|| {
            input_error(
                "invalid_pandoc_json",
                "/meta",
                "Pandoc metadata must be an object",
            )
        })?;
    let blocks = root
        .get("blocks")
        .and_then(Value::as_array)
        .cloned()
        .ok_or_else(|| {
            input_error(
                "invalid_pandoc_json",
                "/blocks",
                "Pandoc blocks must be an array",
            )
        })?;
    if root.len() != 3 {
        return Err(input_error(
            "invalid_pandoc_json",
            "",
            "Pandoc JSON envelope contains unknown members",
        ));
    }
    Ok(PandocDocument { metadata, blocks })
}

fn input_error(code: &'static str, path: &str, message: impl Into<String>) -> PandocInputError {
    PandocInputError {
        code,
        path: path.to_owned(),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_an_unknown_api_version() {
        let error = read_pandoc_json(
            br#"{"pandoc-api-version":[1,99],"meta":{},"blocks":[]}"#,
            &ValidationLimits::default(),
        )
        .unwrap_err();
        assert_eq!(error.code, "unsupported_pandoc_api_version");
        assert_eq!(error.path, "/pandoc-api-version");
    }

    #[test]
    fn rejects_duplicate_pandoc_members() {
        let error = read_pandoc_json(
            br#"{"pandoc-api-version":[1,23,1,2],"meta":{},"meta":{},"blocks":[]}"#,
            &ValidationLimits::default(),
        )
        .unwrap_err();
        assert_eq!(error.code, "invalid_pandoc_json");
        assert!(error.message.contains("duplicate object member"));
    }
}
