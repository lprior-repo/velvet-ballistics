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

    arr.iter().map(parse_issue).collect()
}

fn parse_issue(raw: &serde_json::Value) -> Result<Issue, String> {
    let obj = raw
        .as_object()
        .ok_or_else(|| "children JSON contains a non-object issue".to_string())?;
    let bead_id = get_string_field(obj, "id")
        .ok_or_else(|| "child issue is missing id or status".to_string())?;
    let status = get_string_field(obj, "status")
        .ok_or_else(|| "child issue is missing id or status".to_string())?;
    Ok(Issue {
        bead_id,
        status,
        close_reason: get_string_field(obj, "close_reason").map_or(String::new(), |v| v),
        notes: get_string_field(obj, "notes").map_or(String::new(), |v| v),
    })
}

fn get_string_field(
    obj: &serde_json::Map<String, serde_json::Value>,
    field: &str,
) -> Option<String> {
    obj.get(field)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}
