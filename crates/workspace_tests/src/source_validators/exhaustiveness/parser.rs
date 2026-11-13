//! Error exhaustiveness checker: compare fuzz oracle coverage against
//! production error enum variant definitions.
//!
//! Uses regex 1.12.3 (approved dependency) for comment stripping, enum
//! body extraction, and variant mention detection. No unsafe/panic/unwrap.

use regex::Regex;
use std::ops::ControlFlow;

/// Strip C-style line and block comments in two passes:
/// first block comments, then line comments (original Python order).
pub(crate) fn strip_comments(text: &str) -> Result<String, String> {
    let block_re =
        Regex::new(r"(?s)/\*.*?\*/").map_err(|e| format!("block comment regex: {}", e))?;
    let line_re = Regex::new(r"(?m)//[^\n]*").map_err(|e| format!("line comment regex: {}", e))?;

    let after_block = block_re.replace_all(text, "");
    Ok(line_re.replace_all(&after_block, "").to_string())
}

/// Find the position of `enum Name {` and extract the body.
/// Returns the body text with the opening and closing braces removed.
pub(crate) fn extract_enum_body(text: &str, enum_name: &str) -> Result<Option<String>, String> {
    let escaped = regex::escape(enum_name);
    let pattern = format!(r"\benum\s+{}\s*\{{", escaped);
    let re = Regex::new(&pattern).map_err(|e| format!("enum regex: {}", e))?;

    let capture = re.find(text);
    match capture {
        None => Ok(None),
        Some(cap) => {
            let after = cap
                .end()
                .checked_sub(1)
                .ok_or_else(|| "enum marker has no opening brace".to_string())?;
            let suffix = text
                .get(after..)
                .ok_or_else(|| "enum body past end of file".to_string())?;
            let body = extract_matching_brace_body(suffix)?;
            Ok(Some(body))
        }
    }
}

/// Find the function body by name.
/// Returns the body text (excluding the outer braces).
pub(crate) fn extract_function_body(
    text: &str,
    function_name: &str,
) -> Result<Option<String>, String> {
    let escaped = regex::escape(function_name);
    let pattern = format!(r"\bfn\s+{}\s*(?:<[^>]*>)?\(", escaped);
    let re = Regex::new(&pattern).map_err(|e| format!("function regex: {}", e))?;

    let capture = re.find(text);
    match capture {
        None => Ok(None),
        Some(cap) => {
            let after = cap.end();
            let suffix = text
                .get(after..)
                .ok_or_else(|| "function body past end of file".to_string())?;
            let body = extract_matching_brace_body(suffix)?;
            Ok(Some(body))
        }
    }
}

/// Extract the contents of the first balanced brace pair.
fn extract_matching_brace_body(text: &str) -> Result<String, String> {
    let open = text
        .find('{')
        .ok_or_else(|| "No opening brace found".to_string())?;
    let start = open
        .checked_add(1)
        .ok_or_else(|| "Opening brace offset overflow".to_string())?;
    let body = text
        .get(start..)
        .ok_or_else(|| "Opening brace past end of string".to_string())?;
    match body.char_indices().try_fold(1_u32, brace_step) {
        ControlFlow::Break(end) => body
            .get(..end?)
            .map(str::to_owned)
            .ok_or_else(|| "body range past end of string".to_string()),
        ControlFlow::Continue(_) => Err("No matching closing brace found".to_string()),
    }
}

fn brace_step(depth: u32, (offset, ch): (usize, char)) -> ControlFlow<Result<usize, String>, u32> {
    match ch {
        '{' => match depth.checked_add(1) {
            Some(next) => ControlFlow::Continue(next),
            None => ControlFlow::Break(Err("Brace depth overflow".to_string())),
        },
        '}' => match depth.checked_sub(1) {
            Some(0) => ControlFlow::Break(Ok(offset)),
            Some(next) => ControlFlow::Continue(next),
            None => ControlFlow::Break(Err("Brace depth underflow".to_string())),
        },
        _ => ControlFlow::Continue(depth),
    }
}

/// Extract enum variant names from enum body text.
/// Processes line by line, skipping attribute lines, matching the Python reference.
pub(crate) fn extract_enum_variants(body: &str) -> Result<Vec<String>, String> {
    let variant_re = Regex::new(r"^([A-Z][A-Za-z0-9_]*)\b(?:\s*[({,]|$)")
        .map_err(|e| format!("variant regex: {}", e))?;
    let mut variants = Vec::new();

    for line in body.lines() {
        let stripped = line.trim();
        if stripped.is_empty() || stripped.starts_with("#") {
            continue;
        }
        if let Some(cap) = variant_re.captures(stripped)
            && let Some(variant_cap) = cap.get(1)
        {
            let name = variant_cap.as_str().to_owned();
            if !variants.contains(&name) {
                variants.push(name);
            }
        }
    }

    Ok(variants)
}

/// Find all mentions of Type::Variant within function body text.
pub(crate) fn find_mentioned_variants(body: &str, type_name: &str) -> Result<Vec<String>, String> {
    let escaped = regex::escape(type_name);
    let pattern = format!(r"\b{}::([A-Z][A-Za-z0-9_]*)\b", escaped);
    let re = Regex::new(&pattern).map_err(|e| format!("mention regex: {}", e))?;
    let mut mentioned = Vec::new();

    for cap in re.captures_iter(body) {
        if let Some(variant_cap) = cap.get(1) {
            let variant = variant_cap.as_str().to_owned();
            if !mentioned.contains(&variant) {
                mentioned.push(variant);
            }
        }
    }

    Ok(mentioned)
}

/// Check for graceful wildcard arms in the function body.
/// Uses the original Python regex patterns.
pub(crate) fn has_graceful_wildcard(body: &str) -> Result<bool, String> {
    let patterns = [
        r"_\s*=>\s*\{\s*\}",
        r"_\s*=>\s*Ok\s*\(\s*\(\s*\)\s*\)",
        r"_\s*=>\s*\(\s*\)",
    ];

    for pattern in &patterns {
        let re = Regex::new(pattern).map_err(|e| format!("wildcard regex: {}", e))?;
        if re.is_match(body) {
            return Ok(true);
        }
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    include!("parser_tests.rs");
}
