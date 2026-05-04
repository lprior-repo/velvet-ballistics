//! Durability/replay panel for verifying workflow replay safety.
//!
//! Examines Do nodes in a compiled workflow and checks whether their actions
//! are safe to replay after a crash or restart. Durability is inferred from
//! structural context surrounding each Do node: error handler presence,
//! retry-check coverage, and timeout/RepeatStart wrapping.

use vb_core::ids::StepIdx;
use vb_core::workflow::{CompiledNode, CompiledNodeKind};

/// Overall replay risk classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayRisk {
    /// All checks passed; workflow is safe to replay.
    Safe,
    /// Minor issues found; replay is likely safe but not guaranteed.
    LowRisk,
    /// Significant concerns; replay may produce incorrect results.
    HighRisk,
    /// Critical issues; replay is not safe.
    Unsafe,
}

/// Result of a single durability check.
#[derive(Debug, Clone)]
pub struct DurabilityCheck {
    /// Human-readable label identifying this check.
    pub label: String,
    /// Whether the check passed.
    pub passed: bool,
    /// Human-readable detail explaining the result.
    pub detail: String,
}

/// Panel of durability checks for replay safety analysis.
#[derive(Debug, Clone)]
pub struct DurabilityPanel {
    checks: Vec<DurabilityCheck>,
}

impl DurabilityPanel {
    /// Creates an empty durability panel with no checks.
    #[must_use]
    pub fn new() -> Self {
        Self {
            checks: Vec::new(),
        }
    }

    /// Builds a durability panel by examining Do nodes in a workflow.
    ///
    /// Checks performed:
    /// - **journal_before_dispatch**: Every Do node should have an `on_error`
    ///   handler so that journaling occurs before the action is dispatched.
    /// - **completion_before_mutation**: Same structural guarantee -- Do nodes
    ///   with `on_error` handlers ensure completion is recorded before the
    ///   workflow mutates further state.
    /// - **reconciliation_risk**: Any Do node that is reachable from a
    ///   `RetryCheck` node has an idempotency/reconciliation concern.
    /// - **timeout_coverage**: Each Do node should be wrapped in an
    ///   `ErrorHandler` or `RepeatStart` for timeout coverage, or have an
    ///   `on_error` handler.
    #[must_use]
    pub fn from_workflow(nodes: &[CompiledNode]) -> Self {
        let mut checks = Vec::new();
        if nodes.is_empty() {
            checks.push(DurabilityCheck {
                label: String::from("journal_before_dispatch"),
                passed: true,
                detail: String::from("no Do nodes in empty workflow"),
            });
            checks.push(DurabilityCheck {
                label: String::from("completion_before_mutation"),
                passed: true,
                detail: String::from("no Do nodes in empty workflow"),
            });
            checks.push(DurabilityCheck {
                label: String::from("reconciliation_risk"),
                passed: true,
                detail: String::from("no retry-exposed Do nodes"),
            });
            checks.push(DurabilityCheck {
                label: String::from("timeout_coverage"),
                passed: true,
                detail: String::from("no Do nodes in empty workflow"),
            });
            return Self { checks };
        }

        let do_indices = collect_do_node_indices(nodes);
        let retry_targets = collect_retry_check_targets(nodes);

        // --- journal_before_dispatch ---
        let journal = check_journal_before_dispatch(nodes, &do_indices);
        checks.push(journal);

        // --- completion_before_mutation ---
        let completion = check_completion_before_mutation(nodes, &do_indices);
        checks.push(completion);

        // --- reconciliation_risk ---
        let reconciliation = check_reconciliation_risk(nodes, &do_indices, &retry_targets);
        checks.push(reconciliation);

        // --- timeout_coverage ---
        let timeout = check_timeout_coverage(nodes, &do_indices);
        checks.push(timeout);

        Self { checks }
    }

    /// Returns all durability checks.
    #[must_use]
    pub fn checks(&self) -> &[DurabilityCheck] {
        &self.checks
    }

    /// Returns true if every durability check passed.
    #[must_use]
    pub fn passed(&self) -> bool {
        self.checks.iter().all(|c| c.passed)
    }

    /// Returns indices of all failed checks.
    #[must_use]
    pub fn failed_checks(&self) -> Vec<usize> {
        let mut result = Vec::new();
        let mut i = 0;
        while i < self.checks.len() {
            if let Some(check) = self.checks.get(i)
                && !check.passed
            {
                result.push(i);
            }
            i = match i.checked_add(1) {
                Some(n) => n,
                None => break,
            };
        }
        result
    }

    /// Returns the overall replay risk level based on failed checks.
    #[must_use]
    pub fn replay_risk_level(&self) -> ReplayRisk {
        if self.checks.is_empty() {
            return ReplayRisk::Safe;
        }
        let fail_count = self.failed_checks().len();
        if fail_count == 0 {
            return ReplayRisk::Safe;
        }
        let has_timeout_failure = self
            .checks
            .iter()
            .any(|c| !c.passed && c.label == "timeout_coverage");
        let has_reconciliation_failure = self
            .checks
            .iter()
            .any(|c| !c.passed && c.label == "reconciliation_risk");
        // Reconciliation risk with retry-exposed Do nodes is the most
        // dangerous because replay can produce duplicate side effects.
        if has_reconciliation_failure && has_timeout_failure {
            return ReplayRisk::Unsafe;
        }
        if has_reconciliation_failure {
            return ReplayRisk::HighRisk;
        }
        if fail_count > 1 {
            return ReplayRisk::HighRisk;
        }
        ReplayRisk::LowRisk
    }
}

/// Collects the array indices of all Do nodes in the workflow.
fn collect_do_node_indices(nodes: &[CompiledNode]) -> Vec<usize> {
    let mut indices = Vec::new();
    let mut i = 0;
    while i < nodes.len() {
        if let Some(node) = nodes.get(i)
            && matches!(node.kind, CompiledNodeKind::Do { .. })
        {
            indices.push(i);
        }
        i = match i.checked_add(1) {
            Some(n) => n,
            None => break,
        };
    }
    indices
}

/// Collects all StepIdx targets reachable from RetryCheck nodes.
///
/// A RetryCheck node has a `body` field that points to the step to retry.
/// Any Do node whose step index appears as a RetryCheck body target is
/// considered retry-exposed.
fn collect_retry_check_targets(nodes: &[CompiledNode]) -> Vec<StepIdx> {
    let mut targets = Vec::new();
    let mut i = 0;
    while i < nodes.len() {
        if let Some(node) = nodes.get(i)
            && let CompiledNodeKind::RetryCheck { body, .. } = node.kind
        {
            targets.push(body);
        }
        i = match i.checked_add(1) {
            Some(n) => n,
            None => break,
        };
    }
    targets
}

/// Checks that all Do nodes have an on_error handler (journal_before_dispatch).
fn check_journal_before_dispatch(
    nodes: &[CompiledNode],
    do_indices: &[usize],
) -> DurabilityCheck {
    let mut missing = Vec::new();
    for &idx in do_indices {
        if let Some(node) = nodes.get(idx)
            && node.on_error.is_none()
        {
            missing.push(node.id.get());
        }
    }
    if missing.is_empty() {
        DurabilityCheck {
            label: String::from("journal_before_dispatch"),
            passed: true,
            detail: if do_indices.is_empty() {
                String::from("no Do nodes found")
            } else {
                format!("all {} Do nodes have on_error handlers", do_indices.len())
            },
        }
    } else {
        DurabilityCheck {
            label: String::from("journal_before_dispatch"),
            passed: false,
            detail: format!(
                "{} Do node(s) without on_error handler: step(s) {}",
                missing.len(),
                format_u16_slice(&missing)
            ),
        }
    }
}

/// Checks that all Do nodes have an on_error handler (completion_before_mutation).
///
/// This is the same structural check as journal_before_dispatch but framed
/// differently: the on_error handler ensures that the completion of a Do
/// action is recorded before the workflow can mutate further state.
fn check_completion_before_mutation(
    nodes: &[CompiledNode],
    do_indices: &[usize],
) -> DurabilityCheck {
    let mut missing = Vec::new();
    for &idx in do_indices {
        if let Some(node) = nodes.get(idx)
            && node.on_error.is_none()
        {
            missing.push(node.id.get());
        }
    }
    if missing.is_empty() {
        DurabilityCheck {
            label: String::from("completion_before_mutation"),
            passed: true,
            detail: if do_indices.is_empty() {
                String::from("no Do nodes found")
            } else {
                format!("all {} Do nodes ensure completion before mutation", do_indices.len())
            },
        }
    } else {
        DurabilityCheck {
            label: String::from("completion_before_mutation"),
            passed: false,
            detail: format!(
                "{} Do node(s) without completion guard: step(s) {}",
                missing.len(),
                format_u16_slice(&missing)
            ),
        }
    }
}

/// Checks whether any Do node is reachable from a RetryCheck and thus has
/// an idempotency/reconciliation concern on replay.
fn check_reconciliation_risk(
    nodes: &[CompiledNode],
    do_indices: &[usize],
    retry_targets: &[StepIdx],
) -> DurabilityCheck {
    if retry_targets.is_empty() {
        return DurabilityCheck {
            label: String::from("reconciliation_risk"),
            passed: true,
            detail: String::from("no retry-exposed Do nodes"),
        };
    }
    let mut at_risk = Vec::new();
    for &idx in do_indices {
        if let Some(node) = nodes.get(idx)
            && retry_targets.contains(&node.id)
        {
            at_risk.push(node.id.get());
        }
    }
    if at_risk.is_empty() {
        DurabilityCheck {
            label: String::from("reconciliation_risk"),
            passed: true,
            detail: String::from("no Do nodes under retry paths"),
        }
    } else {
        DurabilityCheck {
            label: String::from("reconciliation_risk"),
            passed: false,
            detail: format!(
                "{} Do node(s) under RetryCheck without idempotency guarantee: step(s) {}",
                at_risk.len(),
                format_u16_slice(&at_risk)
            ),
        }
    }
}

/// Checks whether each Do node has timeout coverage.
///
/// Timeout coverage is provided by:
/// - An `on_error` handler on the Do node itself, OR
/// - Being the `body` target of a `RepeatStart` or `RepeatAttempt` node, OR
/// - Being within an `ErrorHandler` node's body.
fn check_timeout_coverage(
    nodes: &[CompiledNode],
    do_indices: &[usize],
) -> DurabilityCheck {
    let error_handler_bodies = collect_error_handler_bodies(nodes);
    let repeat_bodies = collect_repeat_bodies(nodes);

    let mut uncovered = Vec::new();
    for &idx in do_indices {
        if let Some(node) = nodes.get(idx) {
            if node.on_error.is_some() {
                continue;
            }
            if error_handler_bodies.contains(&node.id) || repeat_bodies.contains(&node.id) {
                continue;
            }
            uncovered.push(node.id.get());
        }
    }
    if uncovered.is_empty() {
        DurabilityCheck {
            label: String::from("timeout_coverage"),
            passed: true,
            detail: if do_indices.is_empty() {
                String::from("no Do nodes found")
            } else {
                format!("all {} Do nodes have timeout coverage", do_indices.len())
            },
        }
    } else {
        DurabilityCheck {
            label: String::from("timeout_coverage"),
            passed: false,
            detail: format!(
                "{} Do node(s) without timeout coverage: step(s) {}",
                uncovered.len(),
                format_u16_slice(&uncovered)
            ),
        }
    }
}

/// Collects all step indices that are the `body` of an ErrorHandler node.
fn collect_error_handler_bodies(nodes: &[CompiledNode]) -> Vec<StepIdx> {
    let mut bodies = Vec::new();
    let mut i = 0;
    while i < nodes.len() {
        if let Some(node) = nodes.get(i)
            && let CompiledNodeKind::ErrorHandler { body, .. } = node.kind
        {
            bodies.push(body);
        }
        i = match i.checked_add(1) {
            Some(n) => n,
            None => break,
        };
    }
    bodies
}

/// Collects all step indices that are the body of RepeatStart or RepeatAttempt nodes.
fn collect_repeat_bodies(nodes: &[CompiledNode]) -> Vec<StepIdx> {
    let mut bodies = Vec::new();
    let mut i = 0;
    while i < nodes.len() {
        if let Some(node) = nodes.get(i) {
            match &node.kind {
                CompiledNodeKind::RepeatStart { body, .. } => {
                    bodies.push(*body);
                }
                CompiledNodeKind::RepeatAttempt { body, .. } => {
                    bodies.push(*body);
                }
                _ => {}
            }
        }
        i = match i.checked_add(1) {
            Some(n) => n,
            None => break,
        };
    }
    bodies
}

/// Formats a slice of u16 values as a comma-separated string.
fn format_u16_slice(values: &[u16]) -> String {
    let mut parts = Vec::new();
    let mut i = 0;
    while i < values.len() {
        if let Some(&v) = values.get(i) {
            parts.push(v.to_string());
        }
        i = match i.checked_add(1) {
            Some(n) => n,
            None => break,
        };
    }
    parts.join(", ")
}

impl Default for DurabilityPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::ids::{ActionId, SlotIdx};

    /// Helper to make a minimal CompiledNode with a given kind and no optional fields.
    fn make_node(id: u16, kind: CompiledNodeKind) -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(id),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind,
        }
    }

    /// Helper to make a Do node.
    fn make_do_node(id: u16, action: u16, input: u16) -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(id),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(action),
                input: SlotIdx::new(input),
            },
        }
    }

    /// Helper to make a Do node with an on_error handler.
    fn make_do_node_with_error_handler(
        id: u16,
        action: u16,
        input: u16,
        handler: u16,
    ) -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(id),
            output: None,
            next: None,
            on_error: Some(StepIdx::new(handler)),
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(action),
                input: SlotIdx::new(input),
            },
        }
    }

    // =========================================================================
    // Test 1: All-safe workflow -- every Do node has on_error handler,
    // no RetryCheck, and full timeout coverage.
    // =========================================================================
    #[test]
    fn all_safe_workflow() {
        let nodes = vec![
            make_do_node_with_error_handler(0, 1, 0, 5),
            make_do_node_with_error_handler(1, 2, 1, 5),
            make_node(
                2,
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            ),
            make_node(5, CompiledNodeKind::Nop),
        ];
        let panel = DurabilityPanel::from_workflow(&nodes);
        assert!(panel.passed(), "all checks should pass for safe workflow");
        assert_eq!(panel.checks().len(), 4);
        assert_eq!(panel.replay_risk_level(), ReplayRisk::Safe);
        assert!(panel.failed_checks().is_empty());
    }

    // =========================================================================
    // Test 2: One Do node without on_error (non-durable).
    // =========================================================================
    #[test]
    fn one_non_durable_do_node() {
        let nodes = vec![
            make_do_node(0, 1, 0), // No on_error handler
            make_node(
                1,
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            ),
        ];
        let panel = DurabilityPanel::from_workflow(&nodes);
        assert!(!panel.passed());
        let failed = panel.failed_checks();
        // journal_before_dispatch and completion_before_mutation should both fail.
        assert!(
            failed.len() >= 2,
            "should have at least 2 failures for missing on_error"
        );
        let labels: Vec<&str> = failed
            .iter()
            .filter_map(|&i| panel.checks().get(i).map(|c| c.label.as_str()))
            .collect();
        assert!(labels.contains(&"journal_before_dispatch"));
        assert!(labels.contains(&"completion_before_mutation"));
    }

    // =========================================================================
    // Test 3: Retry without idempotency -- Do node under RetryCheck.
    // =========================================================================
    #[test]
    fn retry_without_idempotency() {
        let nodes = vec![
            make_node(
                0,
                CompiledNodeKind::RetryCheck {
                    policy_slot: SlotIdx::new(0),
                    body: StepIdx::new(1),
                    exhausted: StepIdx::new(3),
                },
            ),
            make_do_node(1, 10, 0), // This Do node is the retry body
            make_node(2, CompiledNodeKind::Nop),
            make_node(
                3,
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            ),
        ];
        let panel = DurabilityPanel::from_workflow(&nodes);
        assert!(!panel.passed());
        let recon = panel
            .checks()
            .iter()
            .find(|c| c.label == "reconciliation_risk");
        assert!(recon.is_some());
        let Some(recon) = recon else {
            return;
        };
        assert!(!recon.passed, "reconciliation_risk should fail");
        assert!(recon.detail.contains("step(s) 1"));
    }

    // =========================================================================
    // Test 4: Missing timeout coverage.
    // =========================================================================
    #[test]
    fn missing_timeout_coverage() {
        let nodes = vec![
            make_do_node(0, 1, 0), // No on_error, no wrapping RepeatStart/ErrorHandler
            make_node(
                1,
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            ),
        ];
        let panel = DurabilityPanel::from_workflow(&nodes);
        let timeout = panel
            .checks()
            .iter()
            .find(|c| c.label == "timeout_coverage");
        assert!(timeout.is_some());
        let Some(timeout) = timeout else {
            return;
        };
        assert!(!timeout.passed, "timeout_coverage should fail");
    }

    // =========================================================================
    // Test 5: Empty workflow.
    // =========================================================================
    #[test]
    fn empty_workflow() {
        let panel = DurabilityPanel::from_workflow(&[]);
        assert!(panel.passed());
        assert_eq!(panel.checks().len(), 4);
        assert_eq!(panel.replay_risk_level(), ReplayRisk::Safe);
        assert!(panel.failed_checks().is_empty());
    }

    // =========================================================================
    // Test 6: Empty panel via new().
    // =========================================================================
    #[test]
    fn new_panel_is_empty() {
        let panel = DurabilityPanel::new();
        assert!(panel.passed());
        assert!(panel.checks().is_empty());
        assert_eq!(panel.replay_risk_level(), ReplayRisk::Safe);
        assert!(panel.failed_checks().is_empty());
    }

    // =========================================================================
    // Test 7: default() matches new().
    // =========================================================================
    #[test]
    fn default_matches_new() {
        let new_panel = DurabilityPanel::new();
        let default_panel = DurabilityPanel::default();
        assert_eq!(new_panel.checks().len(), default_panel.checks().len());
        assert_eq!(new_panel.passed(), default_panel.passed());
    }

    // =========================================================================
    // Test 8: Workflow with only Nop and Finish (no Do nodes).
    // =========================================================================
    #[test]
    fn workflow_without_do_nodes() {
        let nodes = vec![
            make_node(0, CompiledNodeKind::Nop),
            make_node(
                1,
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            ),
        ];
        let panel = DurabilityPanel::from_workflow(&nodes);
        assert!(panel.passed());
        assert_eq!(panel.replay_risk_level(), ReplayRisk::Safe);
        // Should report "no Do nodes found" for the relevant checks.
        let journal = panel
            .checks()
            .iter()
            .find(|c| c.label == "journal_before_dispatch");
        assert!(journal.is_some());
        let Some(journal) = journal else {
            return;
        };
        assert!(journal.passed);
        assert!(journal.detail.contains("no Do nodes"));
    }

    // =========================================================================
    // Test 9: Replay risk levels -- Safe.
    // =========================================================================
    #[test]
    fn replay_risk_safe() {
        let nodes = vec![
            make_do_node_with_error_handler(0, 1, 0, 5),
            make_node(
                1,
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            ),
            make_node(5, CompiledNodeKind::Nop),
        ];
        let panel = DurabilityPanel::from_workflow(&nodes);
        assert_eq!(panel.replay_risk_level(), ReplayRisk::Safe);
    }

    // =========================================================================
    // Test 10: Replay risk levels -- LowRisk (one timeout failure).
    // =========================================================================
    #[test]
    fn replay_risk_low_risk() {
        let panel = DurabilityPanel {
            checks: vec![DurabilityCheck {
                label: String::from("timeout_coverage"),
                passed: false,
                detail: String::from("missing timeout"),
            }],
        };
        assert_eq!(panel.replay_risk_level(), ReplayRisk::LowRisk);
    }

    // =========================================================================
    // Test 11: Replay risk levels -- HighRisk (reconciliation failure only).
    // =========================================================================
    #[test]
    fn replay_risk_high_risk() {
        let panel = DurabilityPanel {
            checks: vec![
                DurabilityCheck {
                    label: String::from("reconciliation_risk"),
                    passed: false,
                    detail: String::from("retry without idempotency"),
                },
                DurabilityCheck {
                    label: String::from("timeout_coverage"),
                    passed: true,
                    detail: String::from("ok"),
                },
            ],
        };
        assert_eq!(panel.replay_risk_level(), ReplayRisk::HighRisk);
    }

    // =========================================================================
    // Test 12: Replay risk levels -- Unsafe (both reconciliation and timeout).
    // =========================================================================
    #[test]
    fn replay_risk_unsafe() {
        let panel = DurabilityPanel {
            checks: vec![
                DurabilityCheck {
                    label: String::from("reconciliation_risk"),
                    passed: false,
                    detail: String::from("retry without idempotency"),
                },
                DurabilityCheck {
                    label: String::from("timeout_coverage"),
                    passed: false,
                    detail: String::from("missing timeout"),
                },
            ],
        };
        assert_eq!(panel.replay_risk_level(), ReplayRisk::Unsafe);
    }

    // =========================================================================
    // Test 13: Replay risk levels -- HighRisk via multiple non-reconciliation failures.
    // =========================================================================
    #[test]
    fn replay_risk_high_risk_multiple_failures() {
        let panel = DurabilityPanel {
            checks: vec![
                DurabilityCheck {
                    label: String::from("journal_before_dispatch"),
                    passed: false,
                    detail: String::from("missing handler"),
                },
                DurabilityCheck {
                    label: String::from("completion_before_mutation"),
                    passed: false,
                    detail: String::from("missing handler"),
                },
            ],
        };
        assert_eq!(panel.replay_risk_level(), ReplayRisk::HighRisk);
    }

    // =========================================================================
    // Test 14: Do node in RepeatStart body has timeout coverage.
    // =========================================================================
    #[test]
    fn do_node_in_repeat_start_has_timeout_coverage() {
        let nodes = vec![
            make_node(
                0,
                CompiledNodeKind::RepeatStart {
                    max_attempts: 3,
                    body: StepIdx::new(1),
                    done: StepIdx::new(2),
                },
            ),
            make_do_node(1, 10, 0), // In RepeatStart body -> has timeout coverage
            make_node(
                2,
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            ),
        ];
        let panel = DurabilityPanel::from_workflow(&nodes);
        let timeout = panel
            .checks()
            .iter()
            .find(|c| c.label == "timeout_coverage");
        assert!(timeout.is_some());
        let Some(timeout) = timeout else {
            return;
        };
        assert!(timeout.passed, "Do in RepeatStart body should have timeout coverage");
    }

    // =========================================================================
    // Test 15: Do node in ErrorHandler body has timeout coverage.
    // =========================================================================
    #[test]
    fn do_node_in_error_handler_has_timeout_coverage() {
        let nodes = vec![
            make_node(
                0,
                CompiledNodeKind::ErrorHandler {
                    body: StepIdx::new(1),
                    handler: StepIdx::new(2),
                    error_slot: None,
                },
            ),
            make_do_node(1, 10, 0), // In ErrorHandler body -> has timeout coverage
            make_node(2, CompiledNodeKind::Nop),
            make_node(
                3,
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            ),
        ];
        let panel = DurabilityPanel::from_workflow(&nodes);
        let timeout = panel
            .checks()
            .iter()
            .find(|c| c.label == "timeout_coverage");
        assert!(timeout.is_some());
        let Some(timeout) = timeout else {
            return;
        };
        assert!(
            timeout.passed,
            "Do in ErrorHandler body should have timeout coverage"
        );
    }

    // =========================================================================
    // Test 16: Multiple Do nodes, some safe some not.
    // =========================================================================
    #[test]
    fn mixed_do_nodes_partial_failure() {
        let nodes = vec![
            make_do_node_with_error_handler(0, 1, 0, 10), // Safe
            make_do_node(1, 2, 1),                        // Unsafe: no on_error
            make_do_node_with_error_handler(2, 3, 2, 10), // Safe
            make_node(
                3,
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            ),
            make_node(10, CompiledNodeKind::Nop),
        ];
        let panel = DurabilityPanel::from_workflow(&nodes);
        assert!(!panel.passed());
        // Should report step 1 as the failing Do node.
        let journal = panel
            .checks()
            .iter()
            .find(|c| c.label == "journal_before_dispatch");
        assert!(journal.is_some());
        let Some(journal) = journal else {
            return;
        };
        assert!(!journal.passed);
        assert!(journal.detail.contains("1 Do node(s)"));
        assert!(journal.detail.contains("step(s) 1"));
    }

    // =========================================================================
    // Test 17: Do node in RepeatAttempt body has timeout coverage.
    // =========================================================================
    #[test]
    fn do_node_in_repeat_attempt_has_timeout_coverage() {
        let nodes = vec![
            make_node(
                0,
                CompiledNodeKind::RepeatAttempt {
                    attempt_slot: SlotIdx::new(0),
                    body: StepIdx::new(1),
                    done: StepIdx::new(2),
                },
            ),
            make_do_node(1, 10, 0),
            make_node(
                2,
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            ),
        ];
        let panel = DurabilityPanel::from_workflow(&nodes);
        let timeout = panel
            .checks()
            .iter()
            .find(|c| c.label == "timeout_coverage");
        assert!(timeout.is_some());
        let Some(timeout) = timeout else {
            return;
        };
        assert!(
            timeout.passed,
            "Do in RepeatAttempt body should have timeout coverage"
        );
    }

    // =========================================================================
    // Test 18: RetryCheck targeting a Do node that has on_error still fails
    // reconciliation_risk (idempotency concern remains).
    // =========================================================================
    #[test]
    fn retry_target_do_with_on_error_still_flags_reconciliation() {
        let nodes = vec![
            make_node(
                0,
                CompiledNodeKind::RetryCheck {
                    policy_slot: SlotIdx::new(0),
                    body: StepIdx::new(1),
                    exhausted: StepIdx::new(3),
                },
            ),
            make_do_node_with_error_handler(1, 10, 0, 5),
            make_node(5, CompiledNodeKind::Nop),
            make_node(
                3,
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            ),
        ];
        let panel = DurabilityPanel::from_workflow(&nodes);
        let recon = panel
            .checks()
            .iter()
            .find(|c| c.label == "reconciliation_risk");
        assert!(recon.is_some());
        let Some(recon) = recon else {
            return;
        };
        assert!(
            !recon.passed,
            "Do under RetryCheck should flag reconciliation risk even with on_error"
        );
    }

    // =========================================================================
    // Test 19: checks() returns slice with correct length.
    // =========================================================================
    #[test]
    fn checks_slice_length() {
        let panel = DurabilityPanel::from_workflow(&[]);
        assert_eq!(panel.checks().len(), 4);
    }

    // =========================================================================
    // Test 20: ReplayRisk derive traits.
    // =========================================================================
    #[test]
    fn replay_risk_copy_and_equality() {
        let a = ReplayRisk::Safe;
        let b = a; // Copy
        assert_eq!(a, b);
        assert_ne!(ReplayRisk::Safe, ReplayRisk::Unsafe);
        let _debug = format!("{:?}", ReplayRisk::HighRisk);
    }

    // =========================================================================
    // Test 21: Multiple RetryCheck nodes each tracked independently.
    //
    // Two RetryCheck nodes pointing at two different Do nodes. Both Do nodes
    // should appear in the reconciliation_risk detail string.
    // =========================================================================
    #[test]
    fn multiple_retry_checks_each_tracked() {
        let nodes = vec![
            // RetryCheck #0 -> retries Do at step 2
            make_node(
                0,
                CompiledNodeKind::RetryCheck {
                    policy_slot: SlotIdx::new(0),
                    body: StepIdx::new(2),
                    exhausted: StepIdx::new(6),
                },
            ),
            // RetryCheck #1 -> retries Do at step 3
            make_node(
                1,
                CompiledNodeKind::RetryCheck {
                    policy_slot: SlotIdx::new(1),
                    body: StepIdx::new(3),
                    exhausted: StepIdx::new(6),
                },
            ),
            make_do_node(2, 10, 0), // retry-exposed Do #1
            make_do_node(3, 11, 1), // retry-exposed Do #2
            make_node(4, CompiledNodeKind::Nop),
            make_node(5, CompiledNodeKind::Nop),
            make_node(
                6,
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            ),
        ];
        let panel = DurabilityPanel::from_workflow(&nodes);
        let recon = panel
            .checks()
            .iter()
            .find(|c| c.label == "reconciliation_risk");
        assert!(recon.is_some());
        let Some(recon) = recon else {
            return;
        };
        assert!(!recon.passed, "two RetryCheck targets should fail reconciliation");
        assert!(recon.detail.contains("2 Do node(s)"));
        assert!(
            recon.detail.contains("step(s) 2") && recon.detail.contains("3"),
            "detail should reference both Do node step ids: {:?}",
            recon.detail
        );
    }

    // =========================================================================
    // Test 22: Replay risk level LowRisk with only journal_before_dispatch failure.
    //
    // A single non-reconciliation, non-timeout failure should classify as LowRisk.
    // =========================================================================
    #[test]
    fn replay_risk_low_from_journal_failure_only() {
        // Construct a panel where only journal_before_dispatch fails.
        let panel = DurabilityPanel {
            checks: vec![
                DurabilityCheck {
                    label: String::from("journal_before_dispatch"),
                    passed: false,
                    detail: String::from("1 Do node(s) without on_error handler: step(s) 0"),
                },
                DurabilityCheck {
                    label: String::from("completion_before_mutation"),
                    passed: true,
                    detail: String::from("all 1 Do nodes ensure completion before mutation"),
                },
                DurabilityCheck {
                    label: String::from("reconciliation_risk"),
                    passed: true,
                    detail: String::from("no retry-exposed Do nodes"),
                },
                DurabilityCheck {
                    label: String::from("timeout_coverage"),
                    passed: true,
                    detail: String::from("all 1 Do nodes have timeout coverage"),
                },
            ],
        };
        assert!(!panel.passed());
        assert_eq!(panel.replay_risk_level(), ReplayRisk::LowRisk);
    }

    // =========================================================================
    // Test 23: failed_checks returns the correct indices.
    //
    // Build a panel with a known pattern of passes and failures and verify that
    // failed_checks() returns exactly the failing indices.
    // =========================================================================
    #[test]
    fn failed_checks_returns_correct_indices() {
        // Index 0: pass, Index 1: fail, Index 2: fail, Index 3: pass
        let panel = DurabilityPanel {
            checks: vec![
                DurabilityCheck {
                    label: String::from("journal_before_dispatch"),
                    passed: true,
                    detail: String::from("ok"),
                },
                DurabilityCheck {
                    label: String::from("completion_before_mutation"),
                    passed: false,
                    detail: String::from("missing"),
                },
                DurabilityCheck {
                    label: String::from("reconciliation_risk"),
                    passed: false,
                    detail: String::from("retry concern"),
                },
                DurabilityCheck {
                    label: String::from("timeout_coverage"),
                    passed: true,
                    detail: String::from("ok"),
                },
            ],
        };
        let failed = panel.failed_checks();
        assert_eq!(failed.len(), 2, "expected exactly 2 failed checks");
        let Some(&first) = failed.get(0) else {
            return;
        };
        let Some(&second) = failed.get(1) else {
            return;
        };
        assert_eq!(first, 1, "index 1 should be failed (completion_before_mutation)");
        assert_eq!(second, 2, "index 2 should be failed (reconciliation_risk)");
        // Verify the labels at those indices match what we expect.
        let Some(check_1) = panel.checks().get(first) else {
            return;
        };
        assert_eq!(check_1.label, "completion_before_mutation");
        let Some(check_2) = panel.checks().get(second) else {
            return;
        };
        assert_eq!(check_2.label, "reconciliation_risk");
    }

    // =========================================================================
    // Test 24: Do node with on_error AND inside ErrorHandler body is not
    // double-counted for timeout coverage.
    //
    // A Do node that has its own on_error handler AND is also referenced as
    // the body of an ErrorHandler node should still pass timeout_coverage
    // exactly once -- it should not appear in the uncovered list.
    // =========================================================================
    #[test]
    fn do_node_with_on_error_and_error_handler_body_not_double_counted() {
        let nodes = vec![
            // ErrorHandler wraps the Do at step 1
            make_node(
                0,
                CompiledNodeKind::ErrorHandler {
                    body: StepIdx::new(1),
                    handler: StepIdx::new(2),
                    error_slot: None,
                },
            ),
            // Do node at step 1 also has its own on_error pointing to step 3
            make_do_node_with_error_handler(1, 10, 0, 3),
            make_node(2, CompiledNodeKind::Nop),
            make_node(3, CompiledNodeKind::Nop),
            make_node(
                4,
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            ),
        ];
        let panel = DurabilityPanel::from_workflow(&nodes);
        // The Do node has on_error, so journal and completion should pass.
        assert!(panel.passed(), "all checks should pass: {:?}", panel.checks());
        // Verify timeout_coverage specifically passes.
        let timeout = panel
            .checks()
            .iter()
            .find(|c| c.label == "timeout_coverage");
        assert!(timeout.is_some());
        let Some(timeout) = timeout else {
            return;
        };
        assert!(
            timeout.passed,
            "Do with on_error inside ErrorHandler body should pass timeout coverage"
        );
        assert_eq!(panel.replay_risk_level(), ReplayRisk::Safe);
    }

    // =========================================================================
    // Test 25: Empty workflow from_workflow returns clean panel with all
    // checks passing and specific detail messages.
    // =========================================================================
    #[test]
    fn empty_workflow_returns_clean_panel_with_details() {
        let panel = DurabilityPanel::from_workflow(&[]);
        assert!(panel.passed());
        assert!(panel.failed_checks().is_empty());
        assert_eq!(panel.replay_risk_level(), ReplayRisk::Safe);
        // Verify each check has the expected "empty workflow" detail message.
        let checks = panel.checks();
        assert_eq!(checks.len(), 4);

        let journal = checks.iter().find(|c| c.label == "journal_before_dispatch");
        assert!(journal.is_some());
        let Some(journal) = journal else { return };
        assert!(journal.passed);
        assert!(journal.detail.contains("empty workflow"));

        let completion = checks.iter().find(|c| c.label == "completion_before_mutation");
        assert!(completion.is_some());
        let Some(completion) = completion else { return };
        assert!(completion.passed);
        assert!(completion.detail.contains("empty workflow"));

        let recon = checks.iter().find(|c| c.label == "reconciliation_risk");
        assert!(recon.is_some());
        let Some(recon) = recon else { return };
        assert!(recon.passed);
        assert!(recon.detail.contains("no retry-exposed Do nodes"));

        let timeout = checks.iter().find(|c| c.label == "timeout_coverage");
        assert!(timeout.is_some());
        let Some(timeout) = timeout else { return };
        assert!(timeout.passed);
        assert!(timeout.detail.contains("empty workflow"));
    }

    // =========================================================================
    // Test 26: All four risk levels exercised from real workflows.
    //
    // Safe:   fully safe workflow with on_error handlers.
    // LowRisk:  one non-reconciliation, non-timeout failure (journal_only).
    // HighRisk: reconciliation_risk failure alone.
    // Unsafe:   both reconciliation_risk AND timeout_coverage failures.
    // =========================================================================
    #[test]
    fn all_risk_levels_from_real_workflows() {
        // --- Safe: Do node with on_error, no RetryCheck ---
        let safe_nodes = vec![
            make_do_node_with_error_handler(0, 1, 0, 2),
            make_node(
                1,
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            ),
            make_node(2, CompiledNodeKind::Nop),
        ];
        let safe_panel = DurabilityPanel::from_workflow(&safe_nodes);
        assert_eq!(
            safe_panel.replay_risk_level(),
            ReplayRisk::Safe,
            "safe workflow should be Safe"
        );

        // --- LowRisk: exactly one non-reconciliation, non-timeout failure.
        // journal_before_dispatch fails and completion_before_mutation also fails
        // (2 failures, both non-reconciliation non-timeout => HighRisk), so we
        // cannot achieve LowRisk from a real workflow because those two checks are
        // structurally coupled (both check for on_error). Construct manually:
        let low_panel = DurabilityPanel {
            checks: vec![
                DurabilityCheck {
                    label: String::from("journal_before_dispatch"),
                    passed: false,
                    detail: String::from("single failure"),
                },
                DurabilityCheck {
                    label: String::from("completion_before_mutation"),
                    passed: true,
                    detail: String::from("ok"),
                },
                DurabilityCheck {
                    label: String::from("reconciliation_risk"),
                    passed: true,
                    detail: String::from("ok"),
                },
                DurabilityCheck {
                    label: String::from("timeout_coverage"),
                    passed: true,
                    detail: String::from("ok"),
                },
            ],
        };
        assert_eq!(
            low_panel.replay_risk_level(),
            ReplayRisk::LowRisk,
            "single non-reconciliation, non-timeout failure should be LowRisk"
        );

        // --- HighRisk: RetryCheck targeting a Do without idempotency ---
        let high_risk_nodes = vec![
            make_node(
                0,
                CompiledNodeKind::RetryCheck {
                    policy_slot: SlotIdx::new(0),
                    body: StepIdx::new(1),
                    exhausted: StepIdx::new(3),
                },
            ),
            make_do_node_with_error_handler(1, 10, 0, 5), // has on_error so journal/completion/timeout pass
            make_node(5, CompiledNodeKind::Nop),
            make_node(
                3,
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            ),
        ];
        let high_panel = DurabilityPanel::from_workflow(&high_risk_nodes);
        assert_eq!(
            high_panel.replay_risk_level(),
            ReplayRisk::HighRisk,
            "reconciliation failure alone should be HighRisk"
        );

        // --- Unsafe: RetryCheck targeting Do + no timeout coverage ---
        let unsafe_nodes = vec![
            make_node(
                0,
                CompiledNodeKind::RetryCheck {
                    policy_slot: SlotIdx::new(0),
                    body: StepIdx::new(1),
                    exhausted: StepIdx::new(3),
                },
            ),
            make_do_node(1, 10, 0), // No on_error, no ErrorHandler/RepeatStart wrap
            make_node(2, CompiledNodeKind::Nop),
            make_node(
                3,
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            ),
        ];
        let unsafe_panel = DurabilityPanel::from_workflow(&unsafe_nodes);
        assert_eq!(
            unsafe_panel.replay_risk_level(),
            ReplayRisk::Unsafe,
            "reconciliation + timeout failure should be Unsafe"
        );
    }

    // =========================================================================
    // Test 27: RetryCheck targeting a non-Do node (Nop) does not flag
    // reconciliation_risk, even though the target exists.
    // =========================================================================
    #[test]
    fn retry_check_targeting_non_do_node_no_reconciliation_risk() {
        let nodes = vec![
            make_node(
                0,
                CompiledNodeKind::RetryCheck {
                    policy_slot: SlotIdx::new(0),
                    body: StepIdx::new(1), // targets a Nop, not a Do
                    exhausted: StepIdx::new(2),
                },
            ),
            make_node(1, CompiledNodeKind::Nop), // not a Do node
            make_node(
                2,
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            ),
        ];
        let panel = DurabilityPanel::from_workflow(&nodes);
        let recon = panel
            .checks()
            .iter()
            .find(|c| c.label == "reconciliation_risk");
        assert!(recon.is_some());
        let Some(recon) = recon else {
            return;
        };
        assert!(
            recon.passed,
            "RetryCheck targeting a non-Do node should not flag reconciliation risk"
        );
        assert!(recon.detail.contains("no Do nodes under retry paths"));
    }

    // =========================================================================
    // Test 28: Do node inside ForEachStart body still requires its own
    // on_error for journal_before_dispatch and completion_before_mutation.
    // ForEachStart is not a recognized timeout wrapper, so the Do node
    // should fail timeout_coverage unless it has on_error.
    // =========================================================================
    #[test]
    fn do_node_in_foreach_start_still_needs_own_error_handling() {
        let nodes = vec![
            make_node(
                0,
                CompiledNodeKind::ForEachStart {
                    input: SlotIdx::new(0),
                    item_slot: SlotIdx::new(1),
                    limit: 10,
                    body: StepIdx::new(1),
                    done: StepIdx::new(2),
                },
            ),
            make_do_node(1, 10, 0), // No on_error, ForEachStart is not a timeout wrapper
            make_node(
                2,
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            ),
        ];
        let panel = DurabilityPanel::from_workflow(&nodes);
        // journal_before_dispatch and completion_before_mutation should fail.
        let journal = panel
            .checks()
            .iter()
            .find(|c| c.label == "journal_before_dispatch");
        assert!(journal.is_some());
        let Some(journal) = journal else { return };
        assert!(!journal.passed, "Do in ForEachStart should still fail journal check");

        // timeout_coverage should also fail since ForEachStart is not recognized.
        let timeout = panel
            .checks()
            .iter()
            .find(|c| c.label == "timeout_coverage");
        assert!(timeout.is_some());
        let Some(timeout) = timeout else { return };
        assert!(
            !timeout.passed,
            "Do in ForEachStart without on_error should fail timeout coverage"
        );
    }

    // =========================================================================
    // Test 29: Three RetryCheck nodes each targeting a different Do node.
    //
    // Verifies that collect_retry_check_targets correctly accumulates all
    // targets and the reconciliation_risk check reports all three Do nodes.
    // =========================================================================
    #[test]
    fn three_retry_checks_each_tracked_independently() {
        let nodes = vec![
            // RetryCheck targeting Do at step 10
            make_node(
                0,
                CompiledNodeKind::RetryCheck {
                    policy_slot: SlotIdx::new(0),
                    body: StepIdx::new(10),
                    exhausted: StepIdx::new(20),
                },
            ),
            // RetryCheck targeting Do at step 11
            make_node(
                1,
                CompiledNodeKind::RetryCheck {
                    policy_slot: SlotIdx::new(1),
                    body: StepIdx::new(11),
                    exhausted: StepIdx::new(20),
                },
            ),
            // RetryCheck targeting Do at step 12
            make_node(
                2,
                CompiledNodeKind::RetryCheck {
                    policy_slot: SlotIdx::new(2),
                    body: StepIdx::new(12),
                    exhausted: StepIdx::new(20),
                },
            ),
            make_do_node_with_error_handler(10, 100, 0, 30),
            make_do_node_with_error_handler(11, 101, 1, 30),
            make_do_node_with_error_handler(12, 102, 2, 30),
            make_node(30, CompiledNodeKind::Nop),
            make_node(
                20,
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            ),
        ];
        let panel = DurabilityPanel::from_workflow(&nodes);

        // All Do nodes have on_error, so journal/completion/timeout pass.
        // But all three are retry-exposed, so reconciliation should fail.
        let recon = panel
            .checks()
            .iter()
            .find(|c| c.label == "reconciliation_risk");
        assert!(recon.is_some());
        let Some(recon) = recon else { return };
        assert!(
            !recon.passed,
            "three RetryCheck targets should fail reconciliation"
        );
        assert!(
            recon.detail.contains("3 Do node(s)"),
            "should report 3 retry-exposed Do nodes: {:?}",
            recon.detail
        );
        // All three step ids should appear in the detail.
        assert!(
            recon.detail.contains("10") && recon.detail.contains("11") && recon.detail.contains("12"),
            "detail should reference all three Do node step ids: {:?}",
            recon.detail
        );
        // Only reconciliation fails, so risk should be HighRisk.
        assert_eq!(panel.replay_risk_level(), ReplayRisk::HighRisk);
    }

    // =========================================================================
    // Test 30: Replay risk LowRisk achieved from a real workflow where only
    // timeout_coverage fails.
    //
    // A Do node without on_error, inside an ErrorHandler body, but with no
    // RetryCheck nodes. timeout_coverage passes (ErrorHandler body), so
    // journal_before_dispatch and completion_before_mutation both fail (2
    // failures, non-reconciliation non-timeout => fail_count > 1 => HighRisk).
    //
    // Since journal and completion are structurally coupled (both check
    // on_error), the only way to get LowRisk from a real workflow is to have
    // exactly 1 of them fail while the other passes. This is not possible
    // from from_workflow alone, so we verify LowRisk via a single
    // completion_before_mutation failure using manual construction.
    // =========================================================================
    #[test]
    fn replay_risk_low_risk_from_single_completion_failure() {
        let panel = DurabilityPanel {
            checks: vec![
                DurabilityCheck {
                    label: String::from("journal_before_dispatch"),
                    passed: true,
                    detail: String::from("all 2 Do nodes have on_error handlers"),
                },
                DurabilityCheck {
                    label: String::from("completion_before_mutation"),
                    passed: false,
                    detail: String::from("1 Do node(s) without completion guard: step(s) 5"),
                },
                DurabilityCheck {
                    label: String::from("reconciliation_risk"),
                    passed: true,
                    detail: String::from("no retry-exposed Do nodes"),
                },
                DurabilityCheck {
                    label: String::from("timeout_coverage"),
                    passed: true,
                    detail: String::from("all 2 Do nodes have timeout coverage"),
                },
            ],
        };
        assert!(!panel.passed());
        assert_eq!(
            panel.replay_risk_level(),
            ReplayRisk::LowRisk,
            "single non-reconciliation, non-timeout failure should be LowRisk"
        );
        let failed = panel.failed_checks();
        assert_eq!(failed.len(), 1);
        let Some(&idx) = failed.first() else { return };
        assert_eq!(idx, 1, "only completion_before_mutation at index 1 should fail");
    }

    // =========================================================================
    // Test 31: failed_checks returns empty vec when all checks pass.
    //
    // Complements test 23 which verifies indices for a mixed pass/fail panel.
    // =========================================================================
    #[test]
    fn failed_checks_empty_when_all_pass() {
        let panel = DurabilityPanel {
            checks: vec![
                DurabilityCheck {
                    label: String::from("journal_before_dispatch"),
                    passed: true,
                    detail: String::from("ok"),
                },
                DurabilityCheck {
                    label: String::from("completion_before_mutation"),
                    passed: true,
                    detail: String::from("ok"),
                },
                DurabilityCheck {
                    label: String::from("reconciliation_risk"),
                    passed: true,
                    detail: String::from("ok"),
                },
                DurabilityCheck {
                    label: String::from("timeout_coverage"),
                    passed: true,
                    detail: String::from("ok"),
                },
            ],
        };
        assert!(panel.passed());
        let failed = panel.failed_checks();
        assert!(
            failed.is_empty(),
            "all-passing panel should have no failed check indices"
        );
    }

    // =========================================================================
    // Test 32: failed_checks returns all indices when every check fails.
    //
    // Complements test 23 and test 31 by covering the all-fail edge case.
    // =========================================================================
    #[test]
    fn failed_checks_all_indices_when_all_fail() {
        let panel = DurabilityPanel {
            checks: vec![
                DurabilityCheck {
                    label: String::from("journal_before_dispatch"),
                    passed: false,
                    detail: String::from("fail"),
                },
                DurabilityCheck {
                    label: String::from("completion_before_mutation"),
                    passed: false,
                    detail: String::from("fail"),
                },
                DurabilityCheck {
                    label: String::from("reconciliation_risk"),
                    passed: false,
                    detail: String::from("fail"),
                },
                DurabilityCheck {
                    label: String::from("timeout_coverage"),
                    passed: false,
                    detail: String::from("fail"),
                },
            ],
        };
        assert!(!panel.passed());
        let failed = panel.failed_checks();
        assert_eq!(failed.len(), 4, "all four checks should be in the failed list");
        // Verify each index.
        let Some(&f0) = failed.get(0) else { return };
        let Some(&f1) = failed.get(1) else { return };
        let Some(&f2) = failed.get(2) else { return };
        let Some(&f3) = failed.get(3) else { return };
        assert_eq!(f0, 0);
        assert_eq!(f1, 1);
        assert_eq!(f2, 2);
        assert_eq!(f3, 3);
        // Both reconciliation and timeout fail, so risk is Unsafe.
        assert_eq!(panel.replay_risk_level(), ReplayRisk::Unsafe);
    }

    // =========================================================================
    // Test 33: Do node with on_error AND inside RepeatStart body is not
    // double-counted for timeout coverage.
    //
    // Complements test 24 which uses ErrorHandler wrapping. Here the Do node
    // has its own on_error AND is the body of a RepeatStart. It should pass
    // timeout_coverage without being counted as uncovered.
    // =========================================================================
    #[test]
    fn do_node_with_on_error_and_repeat_start_body_not_double_counted() {
        let nodes = vec![
            make_node(
                0,
                CompiledNodeKind::RepeatStart {
                    max_attempts: 3,
                    body: StepIdx::new(1),
                    done: StepIdx::new(2),
                },
            ),
            // Do node at step 1 is the RepeatStart body AND has on_error.
            make_do_node_with_error_handler(1, 10, 0, 5),
            make_node(
                2,
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            ),
            make_node(5, CompiledNodeKind::Nop),
        ];
        let panel = DurabilityPanel::from_workflow(&nodes);
        assert!(
            panel.passed(),
            "Do with on_error inside RepeatStart body should pass all checks: {:?}",
            panel.checks()
        );
        let timeout = panel
            .checks()
            .iter()
            .find(|c| c.label == "timeout_coverage");
        assert!(timeout.is_some());
        let Some(timeout) = timeout else { return };
        assert!(
            timeout.passed,
            "Do with on_error inside RepeatStart should pass timeout coverage"
        );
        assert_eq!(panel.replay_risk_level(), ReplayRisk::Safe);
    }

    // =========================================================================
    // Test 34: DurabilityPanel::new() and from_workflow(&[]) produce
    // consistent behavior for empty inputs.
    //
    // new() returns a panel with zero checks. from_workflow(&[]) returns a
    // panel with 4 checks that all pass. Both should report Safe and passed.
    // =========================================================================
    #[test]
    fn new_and_from_workflow_empty_both_safe() {
        let new_panel = DurabilityPanel::new();
        let empty_panel = DurabilityPanel::from_workflow(&[]);

        // Both report passed and Safe.
        assert!(new_panel.passed());
        assert!(empty_panel.passed());
        assert_eq!(new_panel.replay_risk_level(), ReplayRisk::Safe);
        assert_eq!(empty_panel.replay_risk_level(), ReplayRisk::Safe);

        // new() has zero checks; from_workflow(&[]) has 4 checks.
        assert!(new_panel.checks().is_empty());
        assert_eq!(empty_panel.checks().len(), 4);

        // Both have empty failed_checks.
        assert!(new_panel.failed_checks().is_empty());
        assert!(empty_panel.failed_checks().is_empty());
    }

    // =========================================================================
    // Test 35: Every ReplayRisk variant is reachable and distinguishable.
    //
    // Exercises Safe, LowRisk, HighRisk, and Unsafe from manually constructed
    // panels to ensure the classification logic is correct for each variant.
    // Uses different failure combinations than test 26 to increase coverage.
    // =========================================================================
    #[test]
    fn all_risk_variants_distinguishable() {
        // Safe: no failures.
        let safe = DurabilityPanel {
            checks: vec![
                DurabilityCheck {
                    label: String::from("journal_before_dispatch"),
                    passed: true,
                    detail: String::from("ok"),
                },
                DurabilityCheck {
                    label: String::from("completion_before_mutation"),
                    passed: true,
                    detail: String::from("ok"),
                },
                DurabilityCheck {
                    label: String::from("reconciliation_risk"),
                    passed: true,
                    detail: String::from("ok"),
                },
                DurabilityCheck {
                    label: String::from("timeout_coverage"),
                    passed: true,
                    detail: String::from("ok"),
                },
            ],
        };
        assert_eq!(safe.replay_risk_level(), ReplayRisk::Safe);

        // LowRisk: single failure that is neither reconciliation nor timeout.
        let low = DurabilityPanel {
            checks: vec![
                DurabilityCheck {
                    label: String::from("journal_before_dispatch"),
                    passed: false,
                    detail: String::from("fail"),
                },
                DurabilityCheck {
                    label: String::from("completion_before_mutation"),
                    passed: true,
                    detail: String::from("ok"),
                },
                DurabilityCheck {
                    label: String::from("reconciliation_risk"),
                    passed: true,
                    detail: String::from("ok"),
                },
                DurabilityCheck {
                    label: String::from("timeout_coverage"),
                    passed: true,
                    detail: String::from("ok"),
                },
            ],
        };
        assert_eq!(low.replay_risk_level(), ReplayRisk::LowRisk);
        assert_ne!(low.replay_risk_level(), ReplayRisk::Safe);
        assert_ne!(low.replay_risk_level(), ReplayRisk::HighRisk);
        assert_ne!(low.replay_risk_level(), ReplayRisk::Unsafe);

        // HighRisk: reconciliation failure without timeout failure.
        let high = DurabilityPanel {
            checks: vec![
                DurabilityCheck {
                    label: String::from("journal_before_dispatch"),
                    passed: true,
                    detail: String::from("ok"),
                },
                DurabilityCheck {
                    label: String::from("completion_before_mutation"),
                    passed: true,
                    detail: String::from("ok"),
                },
                DurabilityCheck {
                    label: String::from("reconciliation_risk"),
                    passed: false,
                    detail: String::from("fail"),
                },
                DurabilityCheck {
                    label: String::from("timeout_coverage"),
                    passed: true,
                    detail: String::from("ok"),
                },
            ],
        };
        assert_eq!(high.replay_risk_level(), ReplayRisk::HighRisk);

        // Unsafe: both reconciliation_risk AND timeout_coverage fail.
        let unsafe_panel = DurabilityPanel {
            checks: vec![
                DurabilityCheck {
                    label: String::from("journal_before_dispatch"),
                    passed: true,
                    detail: String::from("ok"),
                },
                DurabilityCheck {
                    label: String::from("completion_before_mutation"),
                    passed: true,
                    detail: String::from("ok"),
                },
                DurabilityCheck {
                    label: String::from("reconciliation_risk"),
                    passed: false,
                    detail: String::from("fail"),
                },
                DurabilityCheck {
                    label: String::from("timeout_coverage"),
                    passed: false,
                    detail: String::from("fail"),
                },
            ],
        };
        assert_eq!(unsafe_panel.replay_risk_level(), ReplayRisk::Unsafe);

        // Verify all variants are pairwise distinct.
        let variants = [
            ReplayRisk::Safe,
            ReplayRisk::LowRisk,
            ReplayRisk::HighRisk,
            ReplayRisk::Unsafe,
        ];
        let mut i = 0;
        while i < variants.len() {
            let mut j = i.checked_add(1).unwrap_or_else(|| variants.len());
            while j < variants.len() {
                assert_ne!(
                    variants[i], variants[j],
                    "ReplayRisk variants at indices {} and {} should differ",
                    i, j
                );
                j = match j.checked_add(1) {
                    Some(n) => n,
                    None => break,
                };
            }
            i = match i.checked_add(1) {
                Some(n) => n,
                None => break,
            };
        }
    }
}
