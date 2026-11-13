//! Manifest JSONL parsing with checked row numbering.

pub(crate) fn parse_manifest(jsonl_text: &str) -> (Vec<(usize, serde_json::Value)>, Vec<String>) {
    let mut rows = Vec::new();
    let mut errors = Vec::new();

    for (idx, line) in jsonl_text.lines().enumerate() {
        let line_number = match idx.checked_add(1) {
            Some(n) => n,
            None => {
                errors.push("manifest row numbering overflow".to_string());
                break;
            }
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let data: serde_json::Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                errors.push(format!(
                    "manifest row {}: invalid JSONL row: {}",
                    line_number, e
                ));
                continue;
            }
        };
        if !data.is_object() {
            errors.push(format!(
                "manifest row {}: row is not an object",
                line_number
            ));
        } else {
            rows.push((line_number, data));
        }
    }

    if rows.is_empty() {
        errors.push("manifest has no evidence rows".to_string());
    }

    (rows, errors)
}
