use std::fs;

use super::classify_occurrence;
use super::ordering::compare_finding;
use super::types::*;

pub(crate) fn scan_file(
    input: ScanInput,
    config: &ScanConfig,
) -> Result<Vec<NamingFinding>, NamingScanError> {
    let (path, text) = input_text(input)?;
    let mut findings = Vec::new();
    for (line_index, raw_line) in text.lines().enumerate() {
        let line_number = one_based_line(line_index)?;
        scan_line(&path, line_number, raw_line, config, &mut findings)?;
    }
    findings.sort_by(compare_finding);
    Ok(findings)
}

fn input_text(input: ScanInput) -> Result<(RepoPath, String), NamingScanError> {
    match input {
        ScanInput::Text { path, contents } => Ok((path, contents)),
        ScanInput::Bytes { path, bytes } => String::from_utf8(bytes)
            .map(|contents| (path.clone(), contents))
            .map_err(|_source| NamingScanError::InputReadFailed {
                path,
                source: "input is not supported UTF-8 text".to_owned(),
            }),
        ScanInput::File {
            path,
            absolute_path,
        } => fs::read_to_string(&absolute_path)
            .map(|contents| (path.clone(), contents))
            .map_err(|source| NamingScanError::InputReadFailed {
                path,
                source: source.to_string(),
            }),
    }
}

fn scan_line(
    path: &RepoPath,
    line: LineNumber,
    text: &str,
    config: &ScanConfig,
    findings: &mut Vec<NamingFinding>,
) -> Result<(), NamingScanError> {
    for pattern in &config.scan_patterns {
        scan_pattern(path, line, text, pattern, config, findings)?;
    }
    Ok(())
}

fn scan_pattern(
    path: &RepoPath,
    line: LineNumber,
    text: &str,
    pattern: &str,
    config: &ScanConfig,
    findings: &mut Vec<NamingFinding>,
) -> Result<(), NamingScanError> {
    let mut search_start = 0usize;
    while let Some(relative) = find_from(text, pattern, search_start) {
        let offset = search_start.checked_add(relative).ok_or_else(count_error)?;
        handle_match(path, line, text, pattern, offset, config, findings)?;
        search_start = next_search_start(offset, pattern)?;
    }
    Ok(())
}

fn handle_match(
    path: &RepoPath,
    line: LineNumber,
    text: &str,
    token: &str,
    offset: usize,
    config: &ScanConfig,
    findings: &mut Vec<NamingFinding>,
) -> Result<(), NamingScanError> {
    let end = offset.checked_add(token.len()).ok_or_else(count_error)?;
    if occurrence_is_allowed_in_line(text, offset, end, config)? {
        return Ok(());
    }
    push_invalid_occurrence(path, line, offset, token, config, findings)
}

fn push_invalid_occurrence(
    path: &RepoPath,
    line: LineNumber,
    offset: usize,
    token: &str,
    config: &ScanConfig,
    findings: &mut Vec<NamingFinding>,
) -> Result<(), NamingScanError> {
    let column = one_based(offset)?;
    let occurrence = classify_occurrence(path.clone(), line, column, token, config)?;
    if matches!(occurrence, OccurrenceClass::InvalidLegacy { .. }) {
        findings.push(finding_for_token(path.clone(), line, column, token));
    }
    Ok(())
}

fn next_search_start(offset: usize, pattern: &str) -> Result<usize, NamingScanError> {
    let step = pattern.len().max(1usize);
    offset.checked_add(step).ok_or_else(count_error)
}

fn occurrence_is_allowed_in_line(
    line: &str,
    occurrence_start: usize,
    occurrence_end: usize,
    config: &ScanConfig,
) -> Result<bool, NamingScanError> {
    match &config.allowlist_policy {
        AllowlistPolicy::Exact(rules) => rules.iter().try_fold(false, |allowed, rule| {
            if allowed {
                Ok(true)
            } else {
                allowed_rule_covers_occurrence(line, occurrence_start, occurrence_end, rule)
            }
        }),
    }
}

fn allowed_rule_covers_occurrence(
    line: &str,
    occurrence_start: usize,
    occurrence_end: usize,
    rule: &LegacyAllowRule,
) -> Result<bool, NamingScanError> {
    let Some(allowed_text) = allowed_rule_text(rule) else {
        return Ok(false);
    };
    let mut search_start = 0usize;
    while let Some(relative) = find_from(line, &allowed_text, search_start) {
        let allowed_start = search_start.checked_add(relative).ok_or_else(count_error)?;
        let allowed_end = allowed_start
            .checked_add(allowed_text.len())
            .ok_or_else(count_error)?;
        if allowed_start <= occurrence_start
            && occurrence_end <= allowed_end
            && has_allowlist_boundaries(line, allowed_start, allowed_end)
        {
            return Ok(true);
        }
        search_start = next_search_start(allowed_start, &allowed_text)?;
    }
    Ok(false)
}

fn allowed_rule_text(rule: &LegacyAllowRule) -> Option<String> {
    match rule {
        LegacyAllowRule::RepositoryPath { path } => Some(path.clone()),
        LegacyAllowRule::MasterFilename { filename } => Some(filename.clone()),
        LegacyAllowRule::MigrationReference {
            label,
            artifact,
            legacy_text,
        } => Some(format!("{label} {artifact} {legacy_text}")),
        LegacyAllowRule::Wildcard { .. }
        | LegacyAllowRule::PrefixOnly { .. }
        | LegacyAllowRule::Substring { .. } => None,
    }
}

fn has_allowlist_boundaries(line: &str, start: usize, end: usize) -> bool {
    boundary_before(line, start) && boundary_after(line, end)
}

fn boundary_before(line: &str, start: usize) -> bool {
    line.get(..start)
        .and_then(|prefix| prefix.chars().next_back())
        .is_none_or(is_allowlist_boundary)
}

fn boundary_after(line: &str, end: usize) -> bool {
    line.get(end..)
        .and_then(|suffix| suffix.chars().next())
        .is_none_or(is_allowlist_boundary)
}

fn is_allowlist_boundary(ch: char) -> bool {
    !(ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '/' | ':'))
}

fn find_from(text: &str, pattern: &str, start: usize) -> Option<usize> {
    text.get(start..).and_then(|tail| tail.find(pattern))
}

fn finding_for_token(
    path: RepoPath,
    line: LineNumber,
    column: ColumnNumber,
    token: &str,
) -> NamingFinding {
    let (spelling_class, remediation) = token_class_and_remediation(token);
    NamingFinding {
        path,
        line,
        column,
        spelling_class,
        remediation: remediation.to_owned(),
    }
}

fn token_class_and_remediation(token: &str) -> (SpellingClass, &'static str) {
    if token == LEGACY_CRATE {
        return (
            SpellingClass::LegacyCrateModuleSpelling,
            CANONICAL_UNDERSCORE,
        );
    }
    if token == LEGACY_LANGUAGE_VERSION {
        return (
            SpellingClass::LegacyLanguageVersionSpelling,
            CANONICAL_LANGUAGE_VERSION,
        );
    }
    (SpellingClass::LegacyProjectSpelling, CANONICAL_HYPHEN)
}

fn one_based(zero_based: usize) -> Result<ColumnNumber, NamingScanError> {
    let base =
        u64::try_from(zero_based).map_err(|source| NamingScanError::InvalidConfiguration {
            reason: source.to_string(),
        })?;
    let value = base.checked_add(1).ok_or_else(count_error)?;
    Ok(ColumnNumber::new(value))
}

fn one_based_line(zero_based: usize) -> Result<LineNumber, NamingScanError> {
    let base =
        u64::try_from(zero_based).map_err(|source| NamingScanError::InvalidConfiguration {
            reason: source.to_string(),
        })?;
    let value = base.checked_add(1).ok_or_else(count_error)?;
    Ok(LineNumber::new(value))
}

pub(crate) fn count_error() -> NamingScanError {
    NamingScanError::InvalidConfiguration {
        reason: "scan count overflow".to_owned(),
    }
}
