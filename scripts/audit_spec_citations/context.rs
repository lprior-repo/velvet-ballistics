use std::collections::BTreeSet;

pub fn get_context(lines: &[String], line_num: usize) -> Result<String, String> {
    let i = line_num
        .checked_sub(1)
        .ok_or_else(|| format!("line {} outside range 1-{}", line_num, lines.len()))?;
    if i >= lines.len() {
        return Err(format!("line {} outside range 1-{}", line_num, lines.len()));
    }
    let start = i.saturating_sub(3);
    let end = lines.len().min(i.saturating_add(6));
    let window = lines
        .get(start..end)
        .ok_or_else(|| format!("context window [{}..{}] invalid", start, end))?;
    Ok(window.join(" "))
}

pub fn get_span(flines: &[String], a: u64, b: u64) -> Result<String, String> {
    let start = u64_to_usize(a)?;
    let end = u64_to_usize(b)?;
    let start_idx = start
        .checked_sub(1)
        .ok_or_else(|| "range start must be >= 1".to_string())?;
    if start_idx >= end {
        return Err("range start must be < end".to_string());
    }
    if end > flines.len() {
        return Err(format!(
            "range end {} exceeds file length {}",
            end,
            flines.len()
        ));
    }
    let span = flines
        .get(start_idx..end)
        .ok_or_else(|| format!("span range [{}..{}] invalid", start_idx, end))?;
    Ok(span.join("\n"))
}

fn u64_to_usize(val: u64) -> Result<usize, String> {
    usize::try_from(val).map_err(|_| "range value too large".to_string())
}

fn extract_identifiers(text: &str) -> Vec<String> {
    // Split at non-identifier chars, then find first letter/underscore start
    text.split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
        .filter(|token| !token.is_empty())
        .filter_map(|token| {
            let start = token
                .char_indices()
                .find(|&(_, ch)| ch.is_ascii_alphabetic() || ch == '_')
                .map(|(i, _)| i);
            start.and_then(|i| token.get(i..)).map(|s| s.to_string())
        })
        .collect()
}

fn expand_context(ctx_ids: Vec<String>, stop: &[&str]) -> BTreeSet<String> {
    // Filter base tokens: len > 2, not in STOP
    let base: BTreeSet<String> = ctx_ids
        .into_iter()
        .filter(|id| id.len() > 2 && !stop.contains(&id.as_str()))
        .collect();

    // Add Mirror suffix BEFORE underscore-component expansion
    let mirrored: BTreeSet<String> = base
        .iter()
        .filter_map(|id| id.strip_prefix("Mirror"))
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    let expanded: BTreeSet<String> = base.union(&mirrored).cloned().collect();

    // Underscore-component expansion
    let components: BTreeSet<String> = expanded
        .iter()
        .flat_map(|id| id.split('_'))
        .filter(|part| part.len() > 2 && !stop.contains(part))
        .map(|part| part.to_string())
        .collect();
    let expanded: BTreeSet<String> = expanded.union(&components).cloned().collect();

    // Lowercase expansion
    let lower: BTreeSet<String> = expanded.iter().map(|id| id.to_lowercase()).collect();
    expanded.iter().chain(lower.iter()).cloned().collect()
}

fn span_components(span_ids: &[String], stop: &[&str]) -> BTreeSet<String> {
    // Retain ALL full span tokens (original behavior)
    let full: BTreeSet<String> = span_ids.iter().cloned().collect();
    // Add underscore parts only if len > 2 and not in STOP
    let parts: BTreeSet<String> = span_ids
        .iter()
        .flat_map(|t| t.split('_'))
        .filter(|part| part.len() > 2 && !stop.contains(part))
        .map(|part| part.to_string())
        .collect();
    full.union(&parts).cloned().collect()
}

pub fn match_identifiers(span: &str, ctx: &str, stop: &[&str]) -> bool {
    let ctx_ids = extract_identifiers(ctx);
    let span_ids = extract_identifiers(span);
    let ctx_set = expand_context(ctx_ids, stop);
    let span_comps = span_components(&span_ids, stop);
    span_comps
        .iter()
        .any(|c| ctx_set.contains(c) || ctx_set.contains(&c.to_lowercase()))
}

#[cfg(test)]
#[path = "context_tests.rs"]
mod tests;
