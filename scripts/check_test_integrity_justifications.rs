use super::check_test_integrity_diff::{is_hex40, resolve_base_blob, resolve_current_blob};
use super::{Finding, JustificationRecord};
use std::collections::BTreeSet;
use std::path::Path;

// ---------------------------------------------------------------------------
// Justifications (hash-bound, exact)
// ---------------------------------------------------------------------------

fn is_code_level_skip_detail(detail: &str) -> bool {
    let d = detail.trim();
    d.starts_with("#[ignore")
        || d.starts_with("#[cfg_attr")
        || d == "return"
        || d == "return;"
        || d == "skip"
        || d == "skip;"
        || d.starts_with("skip(")
}

pub(crate) fn parse_justifications(text: &str) -> Result<Vec<JustificationRecord>, String> {
    let mut records = Vec::new();
    let mut seen_identities: BTreeSet<(String, String, String, String)> = BTreeSet::new();
    for (line_no, raw_line) in text.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let columns: Vec<&str> = line.split('\t').collect();
        let [
            base_commit,
            old_path,
            old_blob,
            new_path,
            new_blob,
            finding_kind,
            finding_detail,
            replacement_paths_col,
            review_artifact_path,
            review_artifact_blob,
            bead_id,
        ] = match columns.as_slice() {
            [a, b, c, d, e, f, g, h, i, j, k] => [a, b, c, d, e, f, g, h, i, j, k],
            _ => {
                return Err(format!(
                    "justification record {}: expected 11 tab-separated columns, got {}",
                    line_no.saturating_add(1),
                    columns.len()
                ));
            }
        };
        let base_commit = base_commit.trim().to_owned();
        let old_path = old_path.trim().to_owned();
        let old_blob = old_blob.trim().to_owned();
        let new_path = new_path.trim().to_owned();
        let new_blob = new_blob.trim().to_owned();
        let finding_kind = finding_kind.trim().to_owned();
        let finding_detail = finding_detail.trim().to_owned();
        let replacement_paths_col = replacement_paths_col.trim();
        let review_artifact_path = review_artifact_path.trim().to_owned();
        let review_artifact_blob = review_artifact_blob.trim().to_owned();
        let bead_id = bead_id.trim().to_owned();

        if base_commit.is_empty()
            || old_path.is_empty()
            || finding_kind.is_empty()
            || finding_detail.is_empty()
            || review_artifact_path.is_empty()
            || bead_id.is_empty()
        {
            return Err(format!(
                "justification record {}: empty required field",
                line_no.saturating_add(1)
            ));
        }
        if old_blob != "ABSENT" && !is_hex40(&old_blob) {
            return Err(format!(
                "justification record {}: old_blob must be a 40-hex id or ABSENT",
                line_no.saturating_add(1)
            ));
        }
        if new_blob != "ABSENT" && !is_hex40(&new_blob) {
            return Err(format!(
                "justification record {}: new_blob must be a 40-hex id or ABSENT",
                line_no.saturating_add(1)
            ));
        }
        if !is_hex40(&review_artifact_blob) {
            return Err(format!(
                "justification record {}: review_artifact_blob must be a 40-hex id",
                line_no.saturating_add(1)
            ));
        }

        let replacement_paths = if replacement_paths_col.is_empty() || replacement_paths_col == "-"
        {
            Vec::new()
        } else {
            let mut paths = Vec::new();
            for pair in replacement_paths_col.split(';') {
                let pair = pair.trim();
                if pair.is_empty() {
                    continue;
                }
                let (path, blob) = pair.rsplit_once(':').ok_or_else(|| {
                    format!(
                        "justification record {}: replacement entry {pair:?} lacks ':' blob separator",
                        line_no.saturating_add(1)
                    )
                })?;
                let path = path.trim();
                let blob = blob.trim();
                if path.is_empty() || !is_hex40(blob) {
                    return Err(format!(
                        "justification record {}: replacement entry {pair:?} is malformed",
                        line_no.saturating_add(1)
                    ));
                }
                paths.push((path.to_owned(), blob.to_owned()));
            }
            paths
        };

        if finding_kind == "NonUtf8TestFile" {
            return Err(format!(
                "justification record {}: NonUtf8TestFile findings cannot be justified",
                line_no.saturating_add(1)
            ));
        }
        if finding_kind == "IgnoredOrSkippedTest"
            && is_code_level_skip_detail(&finding_detail)
            && replacement_paths.is_empty()
        {
            return Err(format!(
                "justification record {}: code-level IgnoredOrSkippedTest requires replacement coverage binding",
                line_no.saturating_add(1)
            ));
        }

        let identity = (
            base_commit.clone(),
            old_path.clone(),
            finding_kind.clone(),
            finding_detail.clone(),
        );
        if !seen_identities.insert(identity) {
            return Err(format!(
                "justification record {}: duplicate record for {finding_kind}|{old_path}",
                line_no.saturating_add(1)
            ));
        }

        records.push(JustificationRecord {
            base_commit,
            old_path,
            old_blob,
            new_path,
            new_blob,
            finding_kind,
            finding_detail,
            replacement_paths,
            review_artifact_path,
            review_artifact_blob,
            bead_id,
        });
    }
    Ok(records)
}

impl JustificationRecord {
    pub(crate) fn matches_identity(&self, finding: &Finding) -> bool {
        self.finding_kind == finding.kind
            && self.finding_detail == finding.detail
            && self.old_path == finding.path
    }

    pub(crate) fn hashes_valid(&self, root: &Path, base: &str) -> Result<bool, String> {
        let actual_base = resolve_base_blob(root, base, &self.old_path)?;
        if self.old_blob == "ABSENT" {
            if actual_base.is_some() {
                return Ok(false);
            }
        } else if actual_base.as_deref() != Some(self.old_blob.as_str()) {
            return Ok(false);
        }

        // The record must bind the ACTUAL current state of the finding's
        // location (old_path). An unrelated new_path or ABSENT while the file
        // still exists must not waive the finding.
        let actual_current_old = resolve_current_blob(root, &self.old_path)?;
        if actual_current_old.is_some() {
            // Original still exists: new_path must be the same path and its
            // exact current hash must match.
            if self.new_path != self.old_path || self.new_blob == "ABSENT" {
                return Ok(false);
            }
            if actual_current_old.as_deref() != Some(self.new_blob.as_str()) {
                return Ok(false);
            }
        } else {
            // Original is actually absent: only a real relocation/replacement
            // (different path, exact blob) or an explicit ABSENT is allowed.
            if self.new_path == "ABSENT" {
                if self.new_blob != "ABSENT" {
                    return Ok(false);
                }
            } else if self.new_path == self.old_path || self.new_blob == "ABSENT" {
                return Ok(false);
            } else {
                let actual_new = resolve_current_blob(root, &self.new_path)?;
                if actual_new.as_deref() != Some(self.new_blob.as_str()) {
                    return Ok(false);
                }
            }
        }

        let actual_review = resolve_current_blob(root, &self.review_artifact_path)?;
        if actual_review.as_deref() != Some(self.review_artifact_blob.as_str()) {
            return Ok(false);
        }

        for (path, blob) in &self.replacement_paths {
            let actual = resolve_current_blob(root, path)?;
            if actual.as_deref() != Some(blob.as_str()) {
                return Ok(false);
            }
        }
        Ok(true)
    }
}
