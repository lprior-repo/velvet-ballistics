use super::check_test_integrity_justifications::parse_justifications;
use super::{Finding, JUSTIFICATIONS_REL, JustificationRecord};
use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::Path;

pub(crate) fn load_justifications(root: &Path) -> Result<Vec<JustificationRecord>, String> {
    let tsv_path = root.join(JUSTIFICATIONS_REL);
    let text = match fs::read_to_string(&tsv_path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(format!("cannot read {}: {error}", tsv_path.display()));
        }
    };
    parse_justifications(&text)
}

pub(crate) fn apply_justifications(
    root: &Path,
    resolved_base: &str,
    findings: &[Finding],
) -> Result<(Vec<(Finding, String)>, Vec<Finding>), String> {
    let records = load_justifications(root)?;
    if records.is_empty() {
        return Ok((Vec::new(), findings.to_vec()));
    }

    let mut justified: Vec<(Finding, String)> = Vec::new();
    let mut remaining: Vec<Finding> = Vec::new();
    let mut matched_indices: BTreeSet<usize> = BTreeSet::new();

    for finding in findings {
        let mut matched_record: Option<&JustificationRecord> = None;
        for (index, record) in records.iter().enumerate() {
            // Inactive records (a different full base commit) can neither waive
            // nor error; only records for the resolved base are applied.
            if record.base_commit != resolved_base {
                continue;
            }
            if !record.matches_identity(finding) {
                continue;
            }
            matched_indices.insert(index);
            if record.hashes_valid(root, resolved_base)? {
                matched_record = Some(record);
            }
            break;
        }
        match matched_record {
            Some(record) => justified.push((finding.clone(), record.bead_id.clone())),
            None => remaining.push(finding.clone()),
        }
    }

    // Only ACTIVE records (matching the resolved base) that reference no
    // finding are stale errors. Inactive other-base records are ignored.
    let unmatched: Vec<&JustificationRecord> = records
        .iter()
        .enumerate()
        .filter(|(index, _)| !matched_indices.contains(index))
        .filter(|(_, record)| record.base_commit == resolved_base)
        .map(|(_, record)| record)
        .collect();
    if !unmatched.is_empty() {
        let names = unmatched
            .iter()
            .map(|record| {
                format!(
                    "{}|{}|{}",
                    record.finding_kind, record.old_path, record.finding_detail
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        return Err(format!(
            "stale justification records reference no matching finding: {names}"
        ));
    }

    Ok((justified, remaining))
}
