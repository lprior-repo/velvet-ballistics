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
}
