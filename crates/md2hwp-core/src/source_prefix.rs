//! Recognized object-note labels, retaining original spelling in IR.
pub(crate) fn is_supported(prefix: &str) -> bool {
    static RULES: std::sync::OnceLock<crate::ast2ir_rules::Ast2IrRules> =
        std::sync::OnceLock::new();
    let rules =
        RULES.get_or_init(|| crate::ast2ir_rules::load_builtin_rules().expect("embedded rules"));
    let lower = prefix.to_ascii_lowercase();
    let without_period = lower.strip_suffix('.').unwrap_or(&lower);
    if rules
        .document
        .object_sources
        .prefixes
        .iter()
        .any(|p| p == without_period)
    {
        return true;
    }
    for base in &rules.document.object_sources.numbered_prefixes {
        if let Some(rest) = lower.strip_prefix(base) {
            let digits = rest.strip_prefix('.').unwrap_or(rest);
            if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn labels_and_punctuation_are_closed() {
        let rules = crate::load_builtin_rules().unwrap();
        for p in &rules.document.object_sources.prefixes {
            for label in [p.clone(), format!("{p}."), p.to_ascii_uppercase()] {
                assert!(is_supported(&label), "{label}");
            }
        }
        for valid in ["주1", "주.3", "주석12", "note.1", "NOTE2", "trans."] {
            assert!(is_supported(valid), "{valid}");
        }
        for invalid in [
            "", "주 ", "주..1", "note1.", "source1", "note.١", "src..", "주의1", "결과", "※",
            "rem", "source\n",
        ] {
            assert!(!is_supported(invalid), "{invalid}");
        }
    }
}
