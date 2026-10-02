//! Document metadata and deterministic calendar-date normalization.
use crate::ir::{Metadata, MetadataValue};
use chrono::{Datelike, NaiveDate};
use regex::Regex;
use serde_json::{Map, Value};
use std::sync::LazyLock;

type Error = (String, String);

pub fn supported_key(key: &str) -> bool {
    matches!(key, "title" | "subtitle" | "author" | "date" | "publisher")
        || key
            .strip_prefix("md2hwp-")
            .is_some_and(|name| !name.is_empty())
}

pub fn normalize(source: &Map<String, Value>) -> Result<Metadata, Error> {
    let mut result = Metadata::default();
    for (key, value) in source {
        let path = format!("/meta/{key}");
        if !supported_key(key) {
            return Err((path, format!("Unsupported metadata key {key}")));
        }
        let value = if key == "author" && value.get("t").and_then(Value::as_str) == Some("MetaList")
        {
            let list = value
                .get("c")
                .and_then(Value::as_array)
                .ok_or_else(|| (path.clone(), "Expected author list".into()))?;
            if list.is_empty() {
                return Err((path, "Author list must not be empty".into()));
            }
            MetadataValue::Authors(
                list.iter()
                    .enumerate()
                    .map(|(i, v)| plain_meta(v, &format!("{path}/{i}")))
                    .collect::<Result<_, _>>()?,
            )
        } else {
            MetadataValue::Text(plain_meta(value, &path)?)
        };
        result.0.insert(key.clone(), value);
    }
    if let Some(date) = result.text("date").and_then(normalize_date) {
        result
            .0
            .insert("date-meta".into(), MetadataValue::Text(date));
    }
    validate(&result)?;
    Ok(result)
}

fn plain_meta(value: &Value, path: &str) -> Result<String, Error> {
    let text = match value.get("t").and_then(Value::as_str) {
        Some("MetaString") => value.get("c").and_then(Value::as_str).map(str::to_owned),
        Some("MetaInlines") => Some(plain_inlines(value.get("c").unwrap_or(&Value::Null), path)?),
        _ => None,
    }
    .ok_or_else(|| (path.into(), "Metadata must be single-paragraph text".into()))?;
    if text.chars().any(char::is_control) {
        return Err((path.into(), "Metadata contains a control character".into()));
    }
    Ok(text)
}

fn plain_inlines(value: &Value, path: &str) -> Result<String, Error> {
    let items = value
        .as_array()
        .ok_or_else(|| (path.into(), "Expected inline array".into()))?;
    let mut result = String::new();
    for (i, item) in items.iter().enumerate() {
        let child_path = format!("{path}/{i}");
        let content = item.get("c").unwrap_or(&Value::Null);
        match item.get("t").and_then(Value::as_str) {
            Some("Str") => result.push_str(
                content
                    .as_str()
                    .ok_or_else(|| (child_path.clone(), "Expected Str text".into()))?,
            ),
            Some("Space" | "SoftBreak") => result.push(' '),
            Some("Strong" | "Emph") => result.push_str(&plain_inlines(content, &child_path)?),
            Some("Link") => {
                let link = content
                    .as_array()
                    .filter(|a| a.len() == 3)
                    .ok_or_else(|| (child_path.clone(), "Invalid Link".into()))?;
                result.push_str(&plain_inlines(&link[1], &child_path)?);
            }
            other => return Err((child_path, format!("Unsupported metadata inline {other:?}"))),
        }
    }
    Ok(result)
}

pub fn validate(metadata: &Metadata) -> Result<(), Error> {
    for (key, value) in &metadata.0 {
        let path = format!("/metadata/{key}");
        if !supported_key(key) && key != "date-meta" {
            return Err((path, format!("Unsupported metadata key {key}")));
        }
        if key.chars().any(char::is_control) {
            return Err((path, "Metadata key contains a control character".into()));
        }
        let values: Vec<&str> = match value {
            MetadataValue::Text(text) => vec![text],
            MetadataValue::Authors(values)
                if key == "author"
                    && !values.is_empty()
                    && values.iter().all(|name| !name.trim().is_empty()) =>
            {
                values.iter().map(String::as_str).collect()
            }
            _ => return Err((path, "Only author supports a nonempty string list".into())),
        };
        if values.iter().any(|s| s.chars().any(char::is_control)) {
            return Err((path, "Metadata value contains a control character".into()));
        }
    }
    heading1_start(metadata)?;
    let expected = metadata.text("date").and_then(normalize_date);
    if metadata.text("date-meta") != expected.as_deref() {
        return Err((
            "/metadata/date-meta".into(),
            "date-meta must equal the normalized date (or be absent for an unrecognized date)"
                .into(),
        ));
    }
    Ok(())
}

/// Reserved manuscript setting; other md2hwp-* values remain literal metadata.
pub fn heading1_start(metadata: &Metadata) -> Result<u32, Error> {
    let Some(value) = metadata.text("md2hwp-heading1-start") else {
        return Ok(1);
    };
    value
        .parse::<u32>()
        .ok()
        .filter(|n| (1..=i32::MAX as u32).contains(n) && value.bytes().all(|c| c.is_ascii_digit()))
        .ok_or_else(|| {
            (
                "/metadata/md2hwp-heading1-start".into(),
                "Expected a positive integer from 1 to 2147483647".into(),
            )
        })
}

static DATE_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    [
        r"^([0-9]{4})([0-9]{2})?([0-9]{2})?$",
        r"^([0-9]{4})년(?:\s*([0-9]{1,2})월(?:\s*([0-9]{1,2})일)?)?$",
        r"^([0-9]{4})\.\s*([0-9]{1,2})\.\s*([0-9]{1,2})\.?$",
        r"^([0-9]{4})/([0-9]{1,2})/([0-9]{1,2})$",
        r"^([0-9]{4})-([0-9]{1,2})-([0-9]{1,2})$",
        r"^([0-9]{1,2})/([0-9]{1,2})/([0-9]{4}|[0-9]{2})$",
        r"(?i)^([0-9]{1,2})\s+([a-z]+)\.?\s+([0-9]{4})$",
        r"(?i)^([a-z]+)\.?\s+([0-9]{1,2}),\s*([0-9]{4})$",
    ]
    .iter()
    .map(|pattern| Regex::new(pattern).expect("builtin date pattern"))
    .collect()
});

/// Recognize only the declared absolute-date formats; never consult today's date.
pub fn normalize_date(text: &str) -> Option<String> {
    for (index, pattern) in DATE_PATTERNS.iter().enumerate() {
        let Some(c) = pattern.captures(text.trim()) else {
            continue;
        };
        let number = |i| {
            c.get(i)
                .map(|s| s.as_str().parse::<u32>().ok())
                .unwrap_or(Some(1))
        };
        let (year, month, day) = match index {
            0..=4 => (number(1)?, number(2)?, number(3)?),
            5 => {
                let mut year = number(3)?;
                if c[3].len() == 2 {
                    year += if year <= 68 { 2000 } else { 1900 };
                }
                (year, number(1)?, number(2)?)
            }
            6 => (number(3)?, english_month(&c[2])?, number(1)?),
            7 => (number(3)?, english_month(&c[1])?, number(2)?),
            _ => unreachable!(),
        };
        if !(1..=9999).contains(&year) {
            return None;
        }
        let date = NaiveDate::from_ymd_opt(year as i32, month, day)?;
        return Some(format!(
            "{:04}-{:02}-{:02}",
            date.year(),
            date.month(),
            date.day()
        ));
    }
    None
}

fn english_month(text: &str) -> Option<u32> {
    match text.to_ascii_lowercase().as_str() {
        "jan" | "january" => Some(1),
        "feb" | "february" => Some(2),
        "mar" | "march" => Some(3),
        "apr" | "april" => Some(4),
        "may" => Some(5),
        "jun" | "june" => Some(6),
        "jul" | "july" => Some(7),
        "aug" | "august" => Some(8),
        "sep" | "september" => Some(9),
        "oct" | "october" => Some(10),
        "nov" | "november" => Some(11),
        "dec" | "december" => Some(12),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn supports_pandoc_and_extended_dates() {
        for (input, expected) in [
            ("04/02/2018", "2018-04-02"),
            ("4/2/18", "2018-04-02"),
            ("2018-04-02", "2018-04-02"),
            ("2018-4-2", "2018-04-02"),
            ("02 Apr 2018", "2018-04-02"),
            ("2 April 2018", "2018-04-02"),
            ("Apr. 02, 2018", "2018-04-02"),
            ("April 2, 2018", "2018-04-02"),
            ("Apr 2, 2018", "2018-04-02"),
            ("2018", "2018-01-01"),
            ("201804", "2018-04-01"),
            ("20180402", "2018-04-02"),
            ("2026년", "2026-01-01"),
            ("2026년 10월", "2026-10-01"),
            ("2026년 10월 25일", "2026-10-25"),
            ("2026년 1월 2일", "2026-01-02"),
            ("2026. 10. 25", "2026-10-25"),
            ("2026. 10. 25.", "2026-10-25"),
            ("2026.10.25", "2026-10-25"),
            ("2026.10.25.", "2026-10-25"),
            ("2026.1.2", "2026-01-02"),
            ("2026/10/25", "2026-10-25"),
            ("2026/1/2", "2026-01-02"),
            ("04/02/68", "2068-04-02"),
            ("04/02/69", "1969-04-02"),
            ("2000-2-29", "2000-02-29"),
        ] {
            assert_eq!(normalize_date(input).as_deref(), Some(expected), "{input}");
        }
        for input in [
            "미정",
            "2026-02-29",
            "1900-2-29",
            "2026/13/1",
            "2026/1/0",
            "0000",
            "201842",
            "2026.10",
            "tomorrow",
            "2026년 25일",
            "2018-04-02junk",
        ] {
            assert_eq!(normalize_date(input), None, "{input}");
        }
    }
    #[test]
    fn metadata_ir_round_trip_and_rejects_forged_dates() {
        let limits = crate::validate::ValidationLimits::default();
        let input = include_bytes!("../../../examples/metadata/metadata.ir.json");
        let valid = crate::read_ir(input, &limits).unwrap();
        assert_eq!(
            valid,
            crate::read_ir(&crate::write_ir(&valid).unwrap(), &limits).unwrap()
        );
        for value in [json!("2026-01-03"), json!("2026-02-29"), json!(null)] {
            let mut forged: Value = serde_json::from_slice(input).unwrap();
            forged["metadata"]["date-meta"] = value;
            assert!(crate::read_ir(&serde_json::to_vec(&forged).unwrap(), &limits).is_err());
        }
        let mut missing: Value = serde_json::from_slice(input).unwrap();
        missing["metadata"]
            .as_object_mut()
            .unwrap()
            .remove("date-meta");
        assert!(crate::read_ir(&serde_json::to_vec(&missing).unwrap(), &limits).is_err());
        let mut limits = limits;
        limits.max_text_bytes = 1;
        assert!(crate::read_ir(input, &limits).is_err());
    }

    #[test]
    fn metadata_keeps_display_text_and_enforces_derived_date() {
        let source = json!({"date":{"t":"MetaInlines","c":[{"t":"Str","c":"2026년"},{"t":"Space"},{"t":"Str","c":"10월"}]},
            "md2hwp-report-number":{"t":"MetaString","c":"기본 2026-01"},
            "author":{"t":"MetaList","c":[{"t":"MetaString","c":"홍길동"},{"t":"MetaString","c":"김연구"}]}});
        let mut metadata = normalize(source.as_object().unwrap()).unwrap();
        assert_eq!(metadata.text("date"), Some("2026년 10월"));
        assert_eq!(metadata.text("date-meta"), Some("2026-10-01"));
        metadata
            .0
            .insert("date-meta".into(), MetadataValue::Text("2026-10-02".into()));
        assert!(validate(&metadata).is_err());
        for bad in [
            json!({"lang":{"t":"MetaString","c":"ko-KR"}}),
            json!({"date-meta":{"t":"MetaString","c":"2026-01-01"}}),
            json!({"title":{"t":"MetaBool","c":true}}),
        ] {
            assert!(normalize(bad.as_object().unwrap()).is_err());
        }
        let unknown = normalize(
            json!({"date":{"t":"MetaString","c":"미정"}})
                .as_object()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(unknown.text("date"), Some("미정"));
        assert_eq!(unknown.text("date-meta"), None);
    }
}

#[cfg(test)]
mod heading1_tests {
    use super::*;
    #[test]
    fn heading1_count_must_not_overflow() {
        use crate::ir::{Block, Document, IR_VERSION, SCHEMA_NAME};
        let mut meta = Metadata::default();
        meta.0.insert(
            "md2hwp-heading1-start".into(),
            MetadataValue::Text("2147483647".into()),
        );
        let heading = Block::Heading {
            level: 1,
            inlines: vec![crate::ir::Inline::Text {
                value: "title".into(),
            }],
        };
        let document = Document {
            schema: SCHEMA_NAME.into(),
            ir_version: IR_VERSION.into(),
            metadata: meta,
            blocks: vec![heading.clone(), heading],
        };
        let error =
            crate::validate::validate(document, &crate::validate::ValidationLimits::default())
                .unwrap_err();
        assert_eq!(error.path, "/metadata/md2hwp-heading1-start");
    }
    #[test]
    fn heading1_start_is_a_validated_reserved_setting() {
        assert_eq!(heading1_start(&Metadata::default()).unwrap(), 1);
        for value in ["3", "003", "2147483647"] {
            let mut meta = Metadata::default();
            meta.0.insert(
                "md2hwp-heading1-start".into(),
                MetadataValue::Text(value.into()),
            );
            assert!(validate(&meta).is_ok());
            assert_eq!(
                heading1_start(&meta).unwrap(),
                value.parse::<u32>().unwrap()
            );
        }
        for value in ["", "0", "-1", "+3", "1.5", " 3", "3 ", "2147483648", "３"] {
            let mut meta = Metadata::default();
            meta.0.insert(
                "md2hwp-heading1-start".into(),
                MetadataValue::Text(value.into()),
            );
            assert!(validate(&meta).is_err(), "{value}");
        }
    }
}
