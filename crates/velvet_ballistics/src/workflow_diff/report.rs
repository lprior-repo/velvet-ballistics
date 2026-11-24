#![forbid(unsafe_code)]
//! Report assembly and rendering for `diff <workflow.yaml>`.

use serde_json::{Value, json};

use super::compare::Change;
use super::text::LineDiff;
use crate::cli_envelope::Kind;

/// Everything `diff <workflow.yaml>` learned about the two workflows.
pub(super) struct DiffReport {
    pub(super) workflow: String,
    pub(super) baseline: Value,
    pub(super) old_digest: String,
    pub(super) new_digest: String,
    pub(super) old_steps: u16,
    pub(super) new_steps: u16,
    /// Compiled content (ignoring digest, source identity and source symbols) matches.
    pub(super) content_equal: bool,
    /// `None` when the baseline has no retained YAML source (DB artifacts).
    pub(super) text: Option<LineDiff>,
    pub(super) changes: Vec<Change>,
}

impl DiffReport {
    pub(super) fn has_semantic_changes(&self) -> bool {
        !self.changes.is_empty()
    }

    fn text_value(&self) -> Value {
        match &self.text {
            None => json!({
                "available": false,
                "reason": "baseline is a compiled artifact without retained YAML source",
            }),
            Some(diff) => json!({
                "available": true,
                "coarse": diff.coarse,
                "added": diff.count(true),
                "removed": diff.count(false),
                "changes": diff.changes.iter().map(|change| json!({
                    "op": if change.added { "+" } else { "-" },
                    "line": change.line,
                    "text": change.text,
                })).collect::<Vec<_>>(),
            }),
        }
    }

    /// Structured form used by `--emit yaml` and `--emit postcard`.
    pub(super) fn to_value(&self) -> Value {
        let body = json!({
            "workflow": self.workflow,
            "baseline": self.baseline,
            "semantic_changes": self.has_semantic_changes(),
            "digest": {
                "old": self.old_digest,
                "new": self.new_digest,
                "equal": self.old_digest == self.new_digest,
                "content_equal": self.content_equal,
            },
            "text": self.text_value(),
            "semantic": {
                "old_step_count": self.old_steps,
                "new_step_count": self.new_steps,
                "changes": self.changes.iter().map(|change| json!({
                    "category": change.category,
                    "message": change.message,
                })).collect::<Vec<_>>(),
            },
        });
        crate::cli_envelope::serialize_with_version(&body, Kind::WorkflowDiff)
    }

    /// Human-readable lines for `--emit text`.
    pub(super) fn render_text(&self) -> Vec<String> {
        let verdict = match (self.old_digest == self.new_digest, self.content_equal) {
            (true, _) => "identical",
            (false, true) => "differ; compiled content identical, source spelling only",
            (false, false) => "differ",
        };
        let mut lines = vec![
            format!("workflow diff: {}", self.workflow),
            format!(
                "digest: old={} new={} ({})",
                self.old_digest, self.new_digest, verdict
            ),
        ];
        lines.extend(self.text_lines());
        lines.extend(self.semantic_lines());
        lines
    }

    fn text_lines(&self) -> Vec<String> {
        let Some(diff) = &self.text else {
            return vec!["text changes: unavailable (baseline has no YAML source)".into()];
        };
        let mut lines = vec![format!(
            "text changes: +{} -{}{}",
            diff.count(true),
            diff.count(false),
            if diff.coarse {
                " (unaligned: file too large for line alignment)"
            } else {
                ""
            }
        )];
        lines.extend(diff.changes.iter().map(|change| {
            let sign = if change.added { '+' } else { '-' };
            format!("  {sign} {}: {}", change.line, change.text)
        }));
        lines
    }

    fn semantic_lines(&self) -> Vec<String> {
        if self.changes.is_empty() {
            return vec![format!("no semantic changes (steps: {})", self.new_steps)];
        }
        let mut lines = vec![format!(
            "semantic changes: {} (steps {} -> {})",
            self.changes.len(),
            self.old_steps,
            self.new_steps
        )];
        lines.extend(
            self.changes
                .iter()
                .map(|change| format!("  [{}] {}", change.category, change.message)),
        );
        lines
    }
}
