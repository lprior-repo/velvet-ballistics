//! Manifest JSONL parsing with checked row numbering.

pub(crate) fn parse_manifest(jsonl_text: &str) -> (Vec<(usize, serde_json::Value)>, Vec<String>) {
    let parsed = jsonl_text.lines().enumerate().try_fold(
        (Vec::new(), Vec::new()),
        |(mut rows, mut errors), (idx, line)| {
            parse_row(line, idx, &mut rows, &mut errors);
            if errors.iter().any(|e| e.contains("row numbering overflow")) {
                std::ops::ControlFlow::Break((rows, errors))
            } else {
                std::ops::ControlFlow::Continue((rows, errors))
            }
        },
    );
    let (rows, mut errors) = match parsed {
        std::ops::ControlFlow::Break(state) | std::ops::ControlFlow::Continue(state) => state,
    };
    if rows.is_empty() {
        errors.push("manifest has no evidence rows".to_string());
    }

    (rows, errors)
}

fn parse_row(
    line: &str,
    idx: usize,
    rows: &mut Vec<(usize, serde_json::Value)>,
    errors: &mut Vec<String>,
) {
    let line_number = match idx.checked_add(1) {
        Some(n) => n,
        None => {
            errors.push("manifest row numbering overflow".to_string());
            return;
        }
    };
    let trimmed = line.trim_matches(crate::source_text::legacy_whitespace);
    if trimmed.is_empty() {
        return;
    }
    let parse_result = parse_row_json(trimmed);
    match parse_result {
        Ok(data) => handle_parsed_row(data, line_number, rows, errors),
        Err(e) => {
            errors.push(format!(
                "manifest row {}: invalid JSONL row: {}",
                line_number, e
            ));
        }
    }
}

fn handle_parsed_row(
    data: serde_json::Value,
    line_number: usize,
    rows: &mut Vec<(usize, serde_json::Value)>,
    errors: &mut Vec<String>,
) {
    if data.is_object() {
        rows.push((line_number, data));
    } else {
        errors.push(format!(
            "manifest row {}: row is not an object",
            line_number
        ));
    }
}

fn parse_row_json(text: &str) -> Result<serde_json::Value, String> {
    serde_json::from_str::<serde_json::Value>(text).map_err(|e| e.to_string())
}
