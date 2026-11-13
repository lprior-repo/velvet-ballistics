//! Issue parsing from bd children JSON.

#[derive(Debug, Clone)]
pub(crate) struct Issue {
    pub(crate) bead_id: String,
    pub(crate) status: String,
    pub(crate) close_reason: String,
    pub(crate) notes: String,
}

pub(crate) fn parse_issues(json_text: &str) -> Result<Vec<Issue>, String> {
    let value: serde_json::Value =
        serde_json::from_str(json_text).map_err(|e| format!("children JSON is invalid: {}", e))?;

    let arr = value
        .as_array()
        .ok_or_else(|| "children JSON must be a list".to_string())?;

    let mut issues = Vec::with_capacity(arr.len());
    for raw in arr {
        let obj = raw
            .as_object()
            .ok_or_else(|| "children JSON contains a non-object issue".to_string())?;

        let bead_id = get_string_field(obj, "id");
        let status = get_string_field(obj, "status");
        if bead_id.is_none() || status.is_none() {
            return Err("child issue is missing id or status".to_string());
        }

        issues.push(Issue {
            bead_id: bead_id.unwrap_or_default(),
            status: status.unwrap_or_default(),
            close_reason: get_string_field(obj, "close_reason").unwrap_or_default(),
            notes: get_string_field(obj, "notes").unwrap_or_default(),
        });
    }
    Ok(issues)
}

fn get_string_field(
    obj: &serde_json::Map<String, serde_json::Value>,
    field: &str,
) -> Option<String> {
    obj.get(field)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}
