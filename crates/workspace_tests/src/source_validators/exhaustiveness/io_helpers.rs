// File IO helpers for the error exhaustiveness checker.

use std::fs;
use std::path::Path;

use super::exhaustiveness_parser::{
    extract_enum_body, extract_enum_variants, extract_function_body, find_mentioned_variants,
    has_graceful_wildcard, strip_comments,
};

pub(crate) fn read_and_parse_enum(path: &Path, enum_name: &str) -> Result<Vec<String>, String> {
    let content =
        fs::read_to_string(path).map_err(|e| format!("Read file {}: {}", path.display(), e))?;
    let stripped = strip_comments(&content)?;

    let body = extract_enum_body(&stripped, enum_name)?
        .ok_or_else(|| format!("enum {} not found in {}", enum_name, path.display()))?;

    let variants = extract_enum_variants(&body)?;
    if variants.is_empty() {
        return Err(format!(
            "no variants parsed for {} in {}",
            enum_name,
            path.display()
        ));
    }
    Ok(variants)
}

pub(crate) fn read_and_parse_oracle(
    path: &Path,
    type_name: &str,
    function_name: &str,
) -> Result<Vec<String>, String> {
    let content =
        fs::read_to_string(path).map_err(|e| format!("Read file {}: {}", path.display(), e))?;
    let stripped = strip_comments(&content)?;

    let body = extract_function_body(&stripped, function_name)?
        .ok_or_else(|| format!("function {} not found in {}", function_name, path.display()))?;

    let mentioned = find_mentioned_variants(&body, type_name)?;
    Ok(mentioned)
}

pub(crate) fn read_and_check_wildcard(path: &Path, function_name: &str) -> Result<bool, String> {
    let content =
        fs::read_to_string(path).map_err(|e| format!("Read file {}: {}", path.display(), e))?;
    let stripped = strip_comments(&content)?;

    let body = extract_function_body(&stripped, function_name)?
        .ok_or_else(|| format!("function {} not found in {}", function_name, path.display()))?;

    has_graceful_wildcard(&body)
        .map_err(|e| format!("wildcard check failed in {}: {}", path.display(), e))
}
