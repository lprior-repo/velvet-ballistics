//! Tests for the durability module.
//!
//! Extracted from durability.rs to comply with the 300-line file limit rule.

#[cfg(test)]
mod tests {
    use vb_core::ids::{ActionId, SlotIdx, StepIdx};
    use vb_core::workflow::{CompiledNode, CompiledNodeKind};

    use crate::verify::durability::{
        DurabilityCheck, DurabilityLevel, DurabilityPanel, DurabilityReport,
        DurabilityResourceMetric, DurabilityVerifyCheck, ReplayRisk, ResourceBudgetBounds,
        build_durability_report, check_durability_level, compute_resource_usage,
    };

    // Re-exported color constants from screen.rs
    use crate::verify::durability::{NEON_CYAN, NEON_GREEN, NEON_ORANGE, NEON_RED};

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
        assert!(
            timeout.passed,
            "Do in RepeatStart body should have timeout coverage"
        );
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
        assert!(
            !recon.passed,
            "two RetryCheck targets should fail reconciliation"
        );
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
        assert_eq!(
            first, 1,
            "index 1 should be failed (completion_before_mutation)"
        );
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
        assert!(
            panel.passed(),
            "all checks should pass: {:?}",
            panel.checks()
        );
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

        let completion = checks
            .iter()
            .find(|c| c.label == "completion_before_mutation");
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
        assert!(
            !journal.passed,
            "Do in ForEachStart should still fail journal check"
        );

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
            recon.detail.contains("10")
                && recon.detail.contains("11")
                && recon.detail.contains("12"),
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
        assert_eq!(
            idx, 1,
            "only completion_before_mutation at index 1 should fail"
        );
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
        assert_eq!(
            failed.len(),
            4,
            "all four checks should be in the failed list"
        );
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

    // =========================================================================
    // BLACK HAT findings (BH-D01 through BH-D05)
    // =========================================================================

    /// BH-D01 [HIGH]: LowRisk is unreachable from real workflows because
    /// journal_before_dispatch and completion_before_mutation are structurally
    /// coupled.
    ///
    /// Both checks test the exact same condition (presence of on_error handler
    /// on Do nodes). In a real workflow from `from_workflow`, they always fail
    /// together or pass together. When they fail, fail_count is at least 2,
    /// which triggers the `fail_count > 1 => HighRisk` path, bypassing
    /// LowRisk entirely.
    #[test]
    fn bhd01_low_risk_unreachable_from_real_workflow() {
        let nodes = vec![
            make_do_node(0, 1, 0),
            make_node(
                1,
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            ),
        ];
        let panel = DurabilityPanel::from_workflow(&nodes);

        let journal_failed = panel
            .checks()
            .iter()
            .any(|c| c.label == "journal_before_dispatch" && !c.passed);
        let completion_failed = panel
            .checks()
            .iter()
            .any(|c| c.label == "completion_before_mutation" && !c.passed);

        assert!(
            journal_failed,
            "journal should fail for unprotected Do node"
        );
        assert!(
            completion_failed,
            "completion should fail for unprotected Do node"
        );

        let failed = panel.failed_checks();
        assert!(
            failed.len() >= 2,
            "at least 2 checks should fail due to structural coupling"
        );
        assert_eq!(
            panel.replay_risk_level(),
            ReplayRisk::HighRisk,
            "BLACK HAT [HIGH]: fail_count >= 2 from coupled checks produces HighRisk, \
             making LowRisk unreachable from real workflows"
        );

        let no_do_nodes = vec![
            make_node(0, CompiledNodeKind::Nop),
            make_node(
                1,
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            ),
        ];
        let safe_panel = DurabilityPanel::from_workflow(&no_do_nodes);
        assert_eq!(
            safe_panel.replay_risk_level(),
            ReplayRisk::Safe,
            "workflow without Do nodes is always Safe"
        );
    }

    /// BH-D02 [MEDIUM]: RetryCheck matches by StepIdx (node.id), not array
    /// index.
    #[test]
    fn bhd02_retry_check_matches_by_step_idx_not_array_position() {
        let nodes = vec![
            make_node(
                0,
                CompiledNodeKind::RetryCheck {
                    policy_slot: SlotIdx::new(0),
                    body: StepIdx::new(5),
                    exhausted: StepIdx::new(10),
                },
            ),
            make_do_node(1, 10, 0),
            make_do_node(5, 20, 1),
            make_node(
                10,
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
        let Some(recon) = recon else { return };

        assert!(
            !recon.passed,
            "reconciliation should fail for Do node at step idx 5"
        );
        assert!(
            recon.detail.contains("step(s) 5"),
            "should flag step 5 only, got: {:?}",
            recon.detail
        );
        // Step 1 should NOT appear in the step list. The detail is
        // "1 Do node(s) ... step(s) 5" so "1" appears as count but "5"
        // is the only step id in the step list. Verify the step list
        // does not contain "1" by checking "step(s) 1" is absent.
        assert!(
            !recon.detail.contains("step(s) 1"),
            "should NOT flag step 1 in step list, got: {:?}",
            recon.detail
        );
    }

    /// BH-D03 [MEDIUM]: RetryCheck `exhausted` target is not retry-exposed.
    #[test]
    fn bhd03_retry_check_exhausted_target_not_flagged() {
        let nodes = vec![
            make_node(
                0,
                CompiledNodeKind::RetryCheck {
                    policy_slot: SlotIdx::new(0),
                    body: StepIdx::new(1),
                    exhausted: StepIdx::new(2),
                },
            ),
            make_do_node(1, 10, 0),
            make_do_node(2, 20, 1),
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
        let Some(recon) = recon else { return };

        assert!(!recon.passed);
        assert!(
            recon.detail.contains("1 Do node(s)"),
            "got: {:?}",
            recon.detail
        );
        assert!(
            recon.detail.contains("step(s) 1"),
            "got: {:?}",
            recon.detail
        );
    }

    /// BH-D04 [LOW]: Many unprotected Do nodes all appear in detail.
    #[test]
    fn bhd04_many_unprotected_do_nodes_all_listed() {
        let mut nodes = Vec::new();
        for i in 0u16..10 {
            nodes.push(make_do_node(i, u16::from(i), 0));
        }
        nodes.push(make_node(
            10,
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        ));

        let panel = DurabilityPanel::from_workflow(&nodes);
        let journal = panel
            .checks()
            .iter()
            .find(|c| c.label == "journal_before_dispatch");
        let Some(journal) = journal else { return };

        assert!(!journal.passed);
        assert!(
            journal.detail.contains("10 Do node(s)"),
            "got: {:?}",
            journal.detail
        );
        for i in 0u16..10 {
            assert!(
                journal.detail.contains(&i.to_string()),
                "step {} missing, got: {:?}",
                i,
                journal.detail
            );
        }
    }

    /// BH-D05 [LOW]: ReplayRisk escalation ordering.
    #[test]
    fn bhd05_risk_levels_escalate_monotonically() {
        let safe = DurabilityPanel { checks: vec![] };
        assert_eq!(safe.replay_risk_level(), ReplayRisk::Safe);

        let low = DurabilityPanel {
            checks: vec![DurabilityCheck {
                label: String::from("journal_before_dispatch"),
                passed: false,
                detail: String::from("fail"),
            }],
        };
        assert_eq!(low.replay_risk_level(), ReplayRisk::LowRisk);

        let high = DurabilityPanel {
            checks: vec![DurabilityCheck {
                label: String::from("reconciliation_risk"),
                passed: false,
                detail: String::from("fail"),
            }],
        };
        assert_eq!(high.replay_risk_level(), ReplayRisk::HighRisk);

        let unsafe_panel = DurabilityPanel {
            checks: vec![
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
    }

    // =========================================================================
    // BLACKHAT security-focused tests
    // =========================================================================

    /// BLACKHAT_durability_step_id_vs_array_index [MEDIUM]:
    /// collect_do_node_indices collects array indices, but
    /// check_reconciliation_risk compares node.id (StepIdx) against
    /// retry_targets (which contain StepIdx values from RetryCheck.body).
    /// These are different: array index vs step ID. When step IDs are
    /// non-sequential or out of order, the mapping is incorrect.
    #[test]
    fn blackhat_durability_step_id_not_array_index() {
        let nodes = vec![
            // Array index 0: RetryCheck targeting step 5
            make_node(
                0,
                CompiledNodeKind::RetryCheck {
                    policy_slot: SlotIdx::new(0),
                    body: StepIdx::new(5),
                    exhausted: StepIdx::new(10),
                },
            ),
            // Array index 1: Do node with step ID 1 (NOT targeted)
            make_do_node(1, 10, 0),
            // Array index 2: Do node with step ID 5 (targeted by RetryCheck)
            make_do_node(5, 20, 1),
            make_node(
                10,
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
        let Some(recon) = recon else { return };
        // Only the Do node with step ID 5 should be flagged (not step ID 1).
        assert!(!recon.passed, "step 5 should be flagged");
        assert!(
            recon.detail.contains("step(s) 5"),
            "got: {:?}",
            recon.detail
        );
        assert!(
            !recon.detail.contains("step(s) 1"),
            "step 1 should NOT be flagged, got: {:?}",
            recon.detail
        );
    }

    /// BLACKHAT_durability_journal_completion_identical [MEDIUM]:
    /// check_journal_before_dispatch and check_completion_before_mutation
    /// perform the exact same check (on_error.is_none() on Do nodes).
    /// This means they always produce the same pass/fail result, making
    /// LowRisk unreachable from real workflows (fail_count is always 0 or >=2).
    #[test]
    fn blackhat_durability_journal_and_completion_always_agree() {
        // Case 1: No on_error handlers -> both fail.
        let unsafe_nodes = vec![
            make_do_node(0, 1, 0),
            make_node(
                1,
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            ),
        ];
        let panel = DurabilityPanel::from_workflow(&unsafe_nodes);
        let journal = panel
            .checks()
            .iter()
            .find(|c| c.label == "journal_before_dispatch");
        let completion = panel
            .checks()
            .iter()
            .find(|c| c.label == "completion_before_mutation");
        let Some(j) = journal else { return };
        let Some(c) = completion else { return };
        assert_eq!(
            j.passed, c.passed,
            "journal and completion should always agree (case 1)"
        );

        // Case 2: All on_error handlers -> both pass.
        let safe_nodes = vec![
            make_do_node_with_error_handler(0, 1, 0, 5),
            make_node(5, CompiledNodeKind::Nop),
        ];
        let safe_panel = DurabilityPanel::from_workflow(&safe_nodes);
        let j2 = safe_panel
            .checks()
            .iter()
            .find(|c| c.label == "journal_before_dispatch");
        let c2 = safe_panel
            .checks()
            .iter()
            .find(|c| c.label == "completion_before_mutation");
        let Some(j2) = j2 else { return };
        let Some(c2) = c2 else { return };
        assert_eq!(
            j2.passed, c2.passed,
            "journal and completion should always agree (case 2)"
        );
    }

    /// BLACKHAT_durability_timeout_coverage_no_on_error_in_error_handler_body [LOW]:
    /// A Do node that is the body of an ErrorHandler but has no on_error handler
    /// should still pass timeout_coverage (ErrorHandler provides the coverage).
    /// This test verifies the ErrorHandler body check works.
    #[test]
    fn blackhat_durability_error_handler_body_passes_timeout() {
        let nodes = vec![
            make_node(
                0,
                CompiledNodeKind::ErrorHandler {
                    body: StepIdx::new(1),
                    handler: StepIdx::new(2),
                    error_slot: None,
                },
            ),
            make_do_node(1, 10, 0), // No on_error, but inside ErrorHandler body
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
        let Some(t) = timeout else { return };
        assert!(
            t.passed,
            "Do inside ErrorHandler body should pass timeout coverage"
        );
    }

    /// BLACKHAT_durability_from_workflow_distinguishes_empty_from_no_do [LOW]:
    /// Both an empty workflow and a workflow with no Do nodes produce passing
    /// panels, but with different detail messages. This test verifies the
    /// distinction is maintained.
    #[test]
    fn blackhat_durability_empty_vs_no_do_nodes_different_details() {
        let empty_panel = DurabilityPanel::from_workflow(&[]);
        let no_do_nodes = vec![
            make_node(0, CompiledNodeKind::Nop),
            make_node(
                1,
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            ),
        ];
        let no_do_panel = DurabilityPanel::from_workflow(&no_do_nodes);

        // Empty panel should reference "empty workflow".
        let empty_journal = empty_panel
            .checks()
            .iter()
            .find(|c| c.label == "journal_before_dispatch");
        let Some(ej) = empty_journal else { return };
        assert!(ej.detail.contains("empty workflow"));

        // No-Do panel should reference "no Do nodes".
        let no_do_journal = no_do_panel
            .checks()
            .iter()
            .find(|c| c.label == "journal_before_dispatch");
        let Some(nj) = no_do_journal else { return };
        assert!(nj.detail.contains("no Do nodes"));
    }

    /// BLACKHAT_durability_retry_check_target_not_a_do_node [LOW]:
    /// A RetryCheck targeting a non-Do node (e.g., Nop) should not flag
    /// reconciliation_risk because only Do nodes are checked.
    #[test]
    fn blackhat_durability_retry_check_on_nop_no_risk() {
        let nodes = vec![
            make_node(
                0,
                CompiledNodeKind::RetryCheck {
                    policy_slot: SlotIdx::new(0),
                    body: StepIdx::new(1), // targets Nop, not Do
                    exhausted: StepIdx::new(2),
                },
            ),
            make_node(1, CompiledNodeKind::Nop),
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
        let Some(r) = recon else { return };
        assert!(
            r.passed,
            "RetryCheck targeting Nop should not flag reconciliation risk"
        );
    }

    // =====================================================================
    // Phase 2B: DurabilityLevel tests (tests 36-42)
    // =====================================================================

    /// Test 36: DurabilityLevel::label() returns correct strings.
    fn test_durability_level_labels() -> Result<(), String> {
        if DurabilityLevel::BestEffort.label() != "BestEffort" {
            return Err("BestEffort label mismatch".into());
        }
        if DurabilityLevel::Journaled.label() != "Journaled" {
            return Err("Journaled label mismatch".into());
        }
        if DurabilityLevel::Strict.label() != "Strict" {
            return Err("Strict label mismatch".into());
        }
        Ok(())
    }

    #[test]
    fn durability_level_labels() {
        assert!(test_durability_level_labels().is_ok());
    }

    /// Test 37: DurabilityLevel::color() returns valid hex color constants.
    fn test_durability_level_colors() -> Result<(), String> {
        if DurabilityLevel::BestEffort.color() != NEON_ORANGE {
            return Err(format!(
                "BestEffort color should be NEON_ORANGE, got {}",
                DurabilityLevel::BestEffort.color()
            ));
        }
        if DurabilityLevel::Journaled.color() != NEON_CYAN {
            return Err(format!(
                "Journaled color should be NEON_CYAN, got {}",
                DurabilityLevel::Journaled.color()
            ));
        }
        if DurabilityLevel::Strict.color() != NEON_GREEN {
            return Err(format!(
                "Strict color should be NEON_GREEN, got {}",
                DurabilityLevel::Strict.color()
            ));
        }
        Ok(())
    }

    #[test]
    fn durability_level_colors() {
        assert!(test_durability_level_colors().is_ok());
    }

    /// Test 38: DurabilityLevel::rank() returns ordered values.
    fn test_durability_level_rank_ordering() -> Result<(), String> {
        let be = DurabilityLevel::BestEffort.rank();
        let jo = DurabilityLevel::Journaled.rank();
        let st = DurabilityLevel::Strict.rank();
        if be >= jo {
            return Err(format!(
                "BestEffort rank ({}) should be < Journaled rank ({})",
                be, jo
            ));
        }
        if jo >= st {
            return Err(format!(
                "Journaled rank ({}) should be < Strict rank ({})",
                jo, st
            ));
        }
        Ok(())
    }

    #[test]
    fn durability_level_rank_ordering() {
        assert!(test_durability_level_rank_ordering().is_ok());
    }

    /// Test 39: DurabilityLevel derive traits (Debug, Clone, Copy, PartialEq, Eq).
    fn test_durability_level_derives() -> Result<(), String> {
        let a = DurabilityLevel::Journaled;
        let b = a; // Copy
        if a != b {
            return Err("Copy trait broken".into());
        }
        let c = a.clone();
        if a != c {
            return Err("Clone trait broken".into());
        }
        let debug_str = format!("{:?}", a);
        if !debug_str.contains("Journaled") {
            return Err(format!(
                "Debug should contain 'Journaled', got: {}",
                debug_str
            ));
        }
        if DurabilityLevel::BestEffort == DurabilityLevel::Strict {
            return Err("BestEffort should not equal Strict".into());
        }
        Ok(())
    }

    #[test]
    fn durability_level_derives() {
        assert!(test_durability_level_derives().is_ok());
    }

    /// Test 40: DurabilityLevel::default() is BestEffort.
    fn test_durability_level_default() -> Result<(), String> {
        let default = DurabilityLevel::default();
        if default != DurabilityLevel::BestEffort {
            return Err(format!("Default should be BestEffort, got {:?}", default));
        }
        Ok(())
    }

    #[test]
    fn durability_level_default() {
        assert!(test_durability_level_default().is_ok());
    }

    /// Test 41: DurabilityLevel pairwise inequality.
    fn test_durability_level_pairwise_inequality() -> Result<(), String> {
        let variants = [
            DurabilityLevel::BestEffort,
            DurabilityLevel::Journaled,
            DurabilityLevel::Strict,
        ];
        let mut i = 0;
        while i < variants.len() {
            let mut j = i.checked_add(1).unwrap_or(variants.len());
            while j < variants.len() {
                if variants[i] == variants[j] {
                    return Err(format!("Variants at {} and {} should differ", i, j));
                }
                j = j.checked_add(1).unwrap_or(variants.len());
            }
            i = i.checked_add(1).unwrap_or(variants.len());
        }
        Ok(())
    }

    #[test]
    fn durability_level_pairwise_inequality() {
        assert!(test_durability_level_pairwise_inequality().is_ok());
    }

    /// Test 42: DurabilityLevel all labels are non-empty and distinct.
    fn test_durability_level_labels_nonempty_distinct() -> Result<(), String> {
        let labels = [
            DurabilityLevel::BestEffort.label(),
            DurabilityLevel::Journaled.label(),
            DurabilityLevel::Strict.label(),
        ];
        for (i, label) in labels.iter().enumerate() {
            if label.is_empty() {
                return Err(format!("Label at index {} is empty", i));
            }
        }
        let mut i = 0;
        while i < labels.len() {
            let mut j = i.checked_add(1).unwrap_or(labels.len());
            while j < labels.len() {
                if labels[i] == labels[j] {
                    return Err(format!(
                        "Labels at {} and {} are identical: {}",
                        i, j, labels[i]
                    ));
                }
                j = j.checked_add(1).unwrap_or(labels.len());
            }
            i = i.checked_add(1).unwrap_or(labels.len());
        }
        Ok(())
    }

    #[test]
    fn durability_level_labels_nonempty_distinct() {
        assert!(test_durability_level_labels_nonempty_distinct().is_ok());
    }

    // =====================================================================
    // Phase 2B: DurabilityVerifyCheck tests (tests 43-48)
    // =====================================================================

    /// Test 43: DurabilityVerifyCheck::new() creates correct fields.
    fn test_verify_check_new() -> Result<(), String> {
        let check = DurabilityVerifyCheck::new(DurabilityLevel::Strict, true, "all good");
        if check.level != DurabilityLevel::Strict {
            return Err("level should be Strict".into());
        }
        if !check.passed {
            return Err("passed should be true".into());
        }
        if check.detail != "all good" {
            return Err(format!("detail mismatch: {}", check.detail));
        }
        Ok(())
    }

    #[test]
    fn verify_check_new() {
        assert!(test_verify_check_new().is_ok());
    }

    /// Test 44: DurabilityVerifyCheck with failed status.
    fn test_verify_check_failed() -> Result<(), String> {
        let check =
            DurabilityVerifyCheck::new(DurabilityLevel::BestEffort, false, "missing handler");
        if check.passed {
            return Err("passed should be false".into());
        }
        if check.level != DurabilityLevel::BestEffort {
            return Err("level should be BestEffort".into());
        }
        Ok(())
    }

    #[test]
    fn verify_check_failed() {
        assert!(test_verify_check_failed().is_ok());
    }

    /// Test 45: DurabilityVerifyCheck clone round-trip.
    fn test_verify_check_clone() -> Result<(), String> {
        let check = DurabilityVerifyCheck::new(DurabilityLevel::Journaled, true, "ok");
        let cloned = check.clone();
        if cloned.level != check.level {
            return Err("level mismatch after clone".into());
        }
        if cloned.passed != check.passed {
            return Err("passed mismatch after clone".into());
        }
        if cloned.detail != check.detail {
            return Err("detail mismatch after clone".into());
        }
        Ok(())
    }

    #[test]
    fn verify_check_clone() {
        assert!(test_verify_check_clone().is_ok());
    }

    /// Test 46: DurabilityVerifyCheck Debug format includes all fields.
    fn test_verify_check_debug() -> Result<(), String> {
        let check = DurabilityVerifyCheck::new(DurabilityLevel::Strict, true, "detail text");
        let debug = format!("{:?}", check);
        if !debug.contains("Strict") {
            return Err(format!("Debug should contain level, got: {}", debug));
        }
        if !debug.contains("detail text") {
            return Err(format!("Debug should contain detail, got: {}", debug));
        }
        Ok(())
    }

    #[test]
    fn verify_check_debug() {
        assert!(test_verify_check_debug().is_ok());
    }

    /// Test 47: DurabilityVerifyCheck with empty detail.
    fn test_verify_check_empty_detail() -> Result<(), String> {
        let check = DurabilityVerifyCheck::new(DurabilityLevel::Journaled, true, "");
        if !check.detail.is_empty() {
            return Err("detail should be empty".into());
        }
        if !check.passed {
            return Err("passed should be true".into());
        }
        Ok(())
    }

    #[test]
    fn verify_check_empty_detail() {
        assert!(test_verify_check_empty_detail().is_ok());
    }

    /// Test 48: DurabilityVerifyCheck with all durability levels.
    fn test_verify_check_all_levels() -> Result<(), String> {
        let levels = [
            DurabilityLevel::BestEffort,
            DurabilityLevel::Journaled,
            DurabilityLevel::Strict,
        ];
        for (i, &level) in levels.iter().enumerate() {
            let check = DurabilityVerifyCheck::new(level, true, "ok");
            if check.level != level {
                return Err(format!("Level mismatch at index {}", i));
            }
        }
        Ok(())
    }

    #[test]
    fn verify_check_all_levels() {
        assert!(test_verify_check_all_levels().is_ok());
    }

    // =====================================================================
    // Phase 2B: ResourceBudgetBounds tests (tests 49-54)
    // =====================================================================

    /// Test 49: ResourceBudgetBounds::new() creates correct fields.
    fn test_resource_bounds_new() -> Result<(), String> {
        let bounds = ResourceBudgetBounds::new(256, 1000, 5000);
        if bounds.max_memory_mb != 256 {
            return Err(format!(
                "max_memory_mb should be 256, got {}",
                bounds.max_memory_mb
            ));
        }
        if bounds.max_cpu_ms != 1000 {
            return Err(format!(
                "max_cpu_ms should be 1000, got {}",
                bounds.max_cpu_ms
            ));
        }
        if bounds.max_wall_ms != 5000 {
            return Err(format!(
                "max_wall_ms should be 5000, got {}",
                bounds.max_wall_ms
            ));
        }
        Ok(())
    }

    #[test]
    fn resource_bounds_new() {
        assert!(test_resource_bounds_new().is_ok());
    }

    /// Test 50: ResourceBudgetBounds::defaults() returns expected values.
    fn test_resource_bounds_defaults() -> Result<(), String> {
        let defaults = ResourceBudgetBounds::defaults();
        if defaults.max_memory_mb != 512 {
            return Err(format!(
                "default max_memory_mb should be 512, got {}",
                defaults.max_memory_mb
            ));
        }
        if defaults.max_cpu_ms != 5000 {
            return Err(format!(
                "default max_cpu_ms should be 5000, got {}",
                defaults.max_cpu_ms
            ));
        }
        if defaults.max_wall_ms != 10_000 {
            return Err(format!(
                "default max_wall_ms should be 10000, got {}",
                defaults.max_wall_ms
            ));
        }
        Ok(())
    }

    #[test]
    fn resource_bounds_defaults() {
        assert!(test_resource_bounds_defaults().is_ok());
    }

    /// Test 51: ResourceBudgetBounds::default() matches defaults().
    fn test_resource_bounds_default_trait() -> Result<(), String> {
        let from_trait = ResourceBudgetBounds::default();
        let from_method = ResourceBudgetBounds::defaults();
        if from_trait != from_method {
            return Err("Default trait should match defaults() method".into());
        }
        Ok(())
    }

    #[test]
    fn resource_bounds_default_trait() {
        assert!(test_resource_bounds_default_trait().is_ok());
    }

    /// Test 52: ResourceBudgetBounds equality works.
    fn test_resource_bounds_equality() -> Result<(), String> {
        let a = ResourceBudgetBounds::new(100, 200, 300);
        let b = ResourceBudgetBounds::new(100, 200, 300);
        let c = ResourceBudgetBounds::new(999, 200, 300);
        if a != b {
            return Err("Identical bounds should be equal".into());
        }
        if a == c {
            return Err("Different bounds should not be equal".into());
        }
        Ok(())
    }

    #[test]
    fn resource_bounds_equality() {
        assert!(test_resource_bounds_equality().is_ok());
    }

    /// Test 53: ResourceBudgetBounds clone round-trip.
    fn test_resource_bounds_clone() -> Result<(), String> {
        let bounds = ResourceBudgetBounds::new(64, 500, 2000);
        let cloned = bounds.clone();
        if bounds != cloned {
            return Err("Clone should produce equal bounds".into());
        }
        Ok(())
    }

    #[test]
    fn resource_bounds_clone() {
        assert!(test_resource_bounds_clone().is_ok());
    }

    /// Test 54: ResourceBudgetBounds with zero limits.
    fn test_resource_bounds_zero_limits() -> Result<(), String> {
        let bounds = ResourceBudgetBounds::new(0, 0, 0);
        if bounds.max_memory_mb != 0 {
            return Err("max_memory_mb should be 0".into());
        }
        if bounds.max_cpu_ms != 0 {
            return Err("max_cpu_ms should be 0".into());
        }
        if bounds.max_wall_ms != 0 {
            return Err("max_wall_ms should be 0".into());
        }
        Ok(())
    }

    #[test]
    fn resource_bounds_zero_limits() {
        assert!(test_resource_bounds_zero_limits().is_ok());
    }

    // =====================================================================
    // Phase 2B: DurabilityResourceMetric tests (tests 55-64)
    // =====================================================================

    /// Test 55: DurabilityResourceMetric::new() creates correct fields.
    fn test_resource_metric_new() -> Result<(), String> {
        let metric = DurabilityResourceMetric::new("memory", 128, 512);
        if metric.name != "memory" {
            return Err(format!("name should be 'memory', got '{}'", metric.name));
        }
        if metric.used != 128 {
            return Err(format!("used should be 128, got {}", metric.used));
        }
        if metric.limit != 512 {
            return Err(format!("limit should be 512, got {}", metric.limit));
        }
        Ok(())
    }

    #[test]
    fn resource_metric_new() {
        assert!(test_resource_metric_new().is_ok());
    }

    /// Test 56: DurabilityResourceMetric::within_bounds() true when under limit.
    fn test_resource_metric_within_bounds_true() -> Result<(), String> {
        let metric = DurabilityResourceMetric::new("cpu_time", 50, 100);
        if !metric.within_bounds() {
            return Err("Should be within bounds when used < limit".into());
        }
        Ok(())
    }

    #[test]
    fn resource_metric_within_bounds_true() {
        assert!(test_resource_metric_within_bounds_true().is_ok());
    }

    /// Test 57: DurabilityResourceMetric::within_bounds() true when at limit.
    fn test_resource_metric_within_bounds_at_limit() -> Result<(), String> {
        let metric = DurabilityResourceMetric::new("memory", 100, 100);
        if !metric.within_bounds() {
            return Err("Should be within bounds when used == limit".into());
        }
        Ok(())
    }

    #[test]
    fn resource_metric_within_bounds_at_limit() {
        assert!(test_resource_metric_within_bounds_at_limit().is_ok());
    }

    /// Test 58: DurabilityResourceMetric::within_bounds() false when over limit.
    fn test_resource_metric_within_bounds_over() -> Result<(), String> {
        let metric = DurabilityResourceMetric::new("wall_time", 200, 100);
        if metric.within_bounds() {
            return Err("Should NOT be within bounds when used > limit".into());
        }
        Ok(())
    }

    #[test]
    fn resource_metric_within_bounds_over() {
        assert!(test_resource_metric_within_bounds_over().is_ok());
    }

    /// Test 59: DurabilityResourceMetric::utilization() basic calculation.
    fn test_resource_metric_utilization() -> Result<(), String> {
        let metric = DurabilityResourceMetric::new("cpu_time", 50, 100);
        let util = metric.utilization();
        // 50 / 100 = 0.5
        let diff = (util - 0.5).abs();
        if diff > 0.01 {
            return Err(format!("utilization should be ~0.5, got {}", util));
        }
        Ok(())
    }

    #[test]
    fn resource_metric_utilization() {
        assert!(test_resource_metric_utilization().is_ok());
    }

    /// Test 60: DurabilityResourceMetric::utilization() returns 0 for zero limit.
    fn test_resource_metric_utilization_zero_limit() -> Result<(), String> {
        let metric = DurabilityResourceMetric::new("memory", 50, 0);
        let util = metric.utilization();
        if util != 0.0 {
            return Err(format!(
                "utilization with zero limit should be 0.0, got {}",
                util
            ));
        }
        Ok(())
    }

    #[test]
    fn resource_metric_utilization_zero_limit() {
        assert!(test_resource_metric_utilization_zero_limit().is_ok());
    }

    /// Test 61: DurabilityResourceMetric::status_color() returns correct colors.
    fn test_resource_metric_status_color() -> Result<(), String> {
        // Within bounds, low utilization -> green
        let green_metric = DurabilityResourceMetric::new("memory", 10, 100);
        if green_metric.status_color() != NEON_GREEN {
            return Err(format!(
                "Low utilization should be green, got {}",
                green_metric.status_color()
            ));
        }
        // Over limit -> red
        let red_metric = DurabilityResourceMetric::new("memory", 200, 100);
        if red_metric.status_color() != NEON_RED {
            return Err(format!(
                "Over limit should be red, got {}",
                red_metric.status_color()
            ));
        }
        // High utilization (>90%) -> orange
        let orange_metric = DurabilityResourceMetric::new("cpu_time", 95, 100);
        if orange_metric.status_color() != NEON_ORANGE {
            return Err(format!(
                "High utilization should be orange, got {}",
                orange_metric.status_color()
            ));
        }
        Ok(())
    }

    #[test]
    fn resource_metric_status_color() {
        assert!(test_resource_metric_status_color().is_ok());
    }

    /// Test 62: DurabilityResourceMetric clone round-trip.
    fn test_resource_metric_clone() -> Result<(), String> {
        let metric = DurabilityResourceMetric::new("wall_time", 300, 1000);
        let cloned = metric.clone();
        if cloned.name != metric.name {
            return Err("name mismatch after clone".into());
        }
        if cloned.used != metric.used {
            return Err("used mismatch after clone".into());
        }
        if cloned.limit != metric.limit {
            return Err("limit mismatch after clone".into());
        }
        Ok(())
    }

    #[test]
    fn resource_metric_clone() {
        assert!(test_resource_metric_clone().is_ok());
    }

    /// Test 63: DurabilityResourceMetric with zero used and nonzero limit.
    fn test_resource_metric_zero_used() -> Result<(), String> {
        let metric = DurabilityResourceMetric::new("memory", 0, 512);
        if !metric.within_bounds() {
            return Err("Zero used should be within bounds".into());
        }
        let util = metric.utilization();
        if util != 0.0 {
            return Err(format!(
                "Zero used should have 0.0 utilization, got {}",
                util
            ));
        }
        if metric.status_color() != NEON_GREEN {
            return Err("Zero used should be green".into());
        }
        Ok(())
    }

    #[test]
    fn resource_metric_zero_used() {
        assert!(test_resource_metric_zero_used().is_ok());
    }

    /// Test 64: DurabilityResourceMetric Debug output contains fields.
    fn test_resource_metric_debug() -> Result<(), String> {
        let metric = DurabilityResourceMetric::new("test_metric", 42, 100);
        let debug = format!("{:?}", metric);
        if !debug.contains("test_metric") {
            return Err(format!("Debug should contain name, got: {}", debug));
        }
        Ok(())
    }

    #[test]
    fn resource_metric_debug() {
        assert!(test_resource_metric_debug().is_ok());
    }

    // =====================================================================
    // Phase 2B: DurabilityReport tests (tests 65-76)
    // =====================================================================

    /// Test 65: DurabilityReport::new() creates correct fields.
    fn test_durability_report_new() -> Result<(), String> {
        let checks = vec![
            DurabilityVerifyCheck::new(DurabilityLevel::Strict, true, "journal ok"),
            DurabilityVerifyCheck::new(DurabilityLevel::Strict, true, "completion ok"),
        ];
        let metrics = vec![DurabilityResourceMetric::new("memory", 100, 512)];
        let report = DurabilityReport::new(DurabilityLevel::Strict, checks, metrics, 1_000_000);
        if report.level != DurabilityLevel::Strict {
            return Err("level should be Strict".into());
        }
        if report.checks.len() != 2 {
            return Err(format!(
                "checks should have 2 items, got {}",
                report.checks.len()
            ));
        }
        if report.resource_metrics.len() != 1 {
            return Err(format!(
                "metrics should have 1 item, got {}",
                report.resource_metrics.len()
            ));
        }
        if report.timestamp_micros != 1_000_000 {
            return Err(format!(
                "timestamp should be 1000000, got {}",
                report.timestamp_micros
            ));
        }
        Ok(())
    }

    #[test]
    fn durability_report_new() {
        assert!(test_durability_report_new().is_ok());
    }

    /// Test 66: DurabilityReport::empty() returns default empty report.
    fn test_durability_report_empty() -> Result<(), String> {
        let report = DurabilityReport::empty();
        if report.level != DurabilityLevel::BestEffort {
            return Err("empty report level should be BestEffort (default)".into());
        }
        if !report.checks.is_empty() {
            return Err("empty report should have no checks".into());
        }
        if !report.resource_metrics.is_empty() {
            return Err("empty report should have no metrics".into());
        }
        if report.timestamp_micros != 0 {
            return Err("empty report timestamp should be 0".into());
        }
        Ok(())
    }

    #[test]
    fn durability_report_empty() {
        assert!(test_durability_report_empty().is_ok());
    }

    /// Test 67: DurabilityReport::default() matches empty().
    fn test_durability_report_default() -> Result<(), String> {
        let from_default = DurabilityReport::default();
        let from_empty = DurabilityReport::empty();
        if from_default.level != from_empty.level {
            return Err("Default and empty level mismatch".into());
        }
        if from_default.checks.len() != from_empty.checks.len() {
            return Err("Default and empty checks count mismatch".into());
        }
        if from_default.timestamp_micros != from_empty.timestamp_micros {
            return Err("Default and empty timestamp mismatch".into());
        }
        Ok(())
    }

    #[test]
    fn durability_report_default() {
        assert!(test_durability_report_default().is_ok());
    }

    /// Test 68: DurabilityReport::all_passed() true when all checks pass.
    fn test_durability_report_all_passed() -> Result<(), String> {
        let checks = vec![
            DurabilityVerifyCheck::new(DurabilityLevel::Strict, true, "ok"),
            DurabilityVerifyCheck::new(DurabilityLevel::Strict, true, "ok"),
        ];
        let report = DurabilityReport::new(DurabilityLevel::Strict, checks, Vec::new(), 0);
        if !report.all_passed() {
            return Err("all_passed should be true when all checks pass".into());
        }
        Ok(())
    }

    #[test]
    fn durability_report_all_passed() {
        assert!(test_durability_report_all_passed().is_ok());
    }

    /// Test 69: DurabilityReport::all_passed() false when some checks fail.
    fn test_durability_report_not_all_passed() -> Result<(), String> {
        let checks = vec![
            DurabilityVerifyCheck::new(DurabilityLevel::BestEffort, true, "ok"),
            DurabilityVerifyCheck::new(DurabilityLevel::BestEffort, false, "fail"),
        ];
        let report = DurabilityReport::new(DurabilityLevel::BestEffort, checks, Vec::new(), 0);
        if report.all_passed() {
            return Err("all_passed should be false when a check fails".into());
        }
        Ok(())
    }

    #[test]
    fn durability_report_not_all_passed() {
        assert!(test_durability_report_not_all_passed().is_ok());
    }

    /// Test 70: DurabilityReport::pass_count() returns correct count.
    fn test_durability_report_pass_count() -> Result<(), String> {
        let checks = vec![
            DurabilityVerifyCheck::new(DurabilityLevel::Journaled, true, "ok"),
            DurabilityVerifyCheck::new(DurabilityLevel::Journaled, false, "fail"),
            DurabilityVerifyCheck::new(DurabilityLevel::Journaled, true, "ok"),
        ];
        let report = DurabilityReport::new(DurabilityLevel::Journaled, checks, Vec::new(), 0);
        if report.pass_count() != 2 {
            return Err(format!(
                "pass_count should be 2, got {}",
                report.pass_count()
            ));
        }
        Ok(())
    }

    #[test]
    fn durability_report_pass_count() {
        assert!(test_durability_report_pass_count().is_ok());
    }

    /// Test 71: DurabilityReport::fail_count() returns correct value.
    fn test_durability_report_fail_count() -> Result<(), String> {
        let passing = vec![DurabilityVerifyCheck::new(
            DurabilityLevel::Strict,
            true,
            "ok",
        )];
        let report_ok = DurabilityReport::new(DurabilityLevel::Strict, passing, Vec::new(), 0);
        if report_ok.fail_count() != 0 {
            return Err("fail_count should be 0 when all pass".into());
        }

        let failing = vec![
            DurabilityVerifyCheck::new(DurabilityLevel::BestEffort, true, "ok"),
            DurabilityVerifyCheck::new(DurabilityLevel::BestEffort, false, "fail"),
        ];
        let report_fail =
            DurabilityReport::new(DurabilityLevel::BestEffort, failing, Vec::new(), 0);
        if report_fail.fail_count() != 1 {
            return Err("fail_count should be 1 when one fails".into());
        }
        Ok(())
    }

    #[test]
    fn durability_report_fail_count() {
        assert!(test_durability_report_fail_count().is_ok());
    }

    /// Test 72: DurabilityReport::resources_within_bounds().
    fn test_durability_report_resources_within_bounds() -> Result<(), String> {
        let within = vec![
            DurabilityResourceMetric::new("memory", 100, 512),
            DurabilityResourceMetric::new("cpu_time", 50, 100),
        ];
        let report_ok = DurabilityReport::new(DurabilityLevel::Strict, Vec::new(), within, 0);
        if !report_ok.resources_within_bounds() {
            return Err("Should be within bounds".into());
        }

        let over = vec![
            DurabilityResourceMetric::new("memory", 100, 512),
            DurabilityResourceMetric::new("cpu_time", 200, 100),
        ];
        let report_over = DurabilityReport::new(DurabilityLevel::BestEffort, Vec::new(), over, 0);
        if report_over.resources_within_bounds() {
            return Err("Should NOT be within bounds when one metric is over".into());
        }
        Ok(())
    }

    #[test]
    fn durability_report_resources_within_bounds() {
        assert!(test_durability_report_resources_within_bounds().is_ok());
    }

    /// Test 73: DurabilityReport::summary() format.
    fn test_durability_report_summary() -> Result<(), String> {
        let checks = vec![
            DurabilityVerifyCheck::new(DurabilityLevel::Strict, true, "ok"),
            DurabilityVerifyCheck::new(DurabilityLevel::Strict, false, "fail"),
            DurabilityVerifyCheck::new(DurabilityLevel::Strict, true, "ok"),
        ];
        let report = DurabilityReport::new(DurabilityLevel::Strict, checks, Vec::new(), 0);
        let summary = report.summary();
        if summary != "2/3 checks passed" {
            return Err(format!(
                "Summary should be '2/3 checks passed', got '{}'",
                summary
            ));
        }
        Ok(())
    }

    #[test]
    fn durability_report_summary() {
        assert!(test_durability_report_summary().is_ok());
    }

    /// Test 74: DurabilityReport empty summary.
    fn test_durability_report_empty_summary() -> Result<(), String> {
        let report = DurabilityReport::empty();
        let summary = report.summary();
        if summary != "0/0 checks passed" {
            return Err(format!(
                "Empty summary should be '0/0 checks passed', got '{}'",
                summary
            ));
        }
        Ok(())
    }

    #[test]
    fn durability_report_empty_summary() {
        assert!(test_durability_report_empty_summary().is_ok());
    }

    /// Test 75: DurabilityReport clone round-trip.
    fn test_durability_report_clone() -> Result<(), String> {
        let checks = vec![DurabilityVerifyCheck::new(
            DurabilityLevel::Journaled,
            true,
            "test",
        )];
        let metrics = vec![DurabilityResourceMetric::new("memory", 50, 100)];
        let report = DurabilityReport::new(DurabilityLevel::Journaled, checks, metrics, 42);
        let cloned = report.clone();
        if cloned.level != report.level {
            return Err("level mismatch after clone".into());
        }
        if cloned.checks.len() != report.checks.len() {
            return Err("checks count mismatch after clone".into());
        }
        if cloned.resource_metrics.len() != report.resource_metrics.len() {
            return Err("metrics count mismatch after clone".into());
        }
        if cloned.timestamp_micros != report.timestamp_micros {
            return Err("timestamp mismatch after clone".into());
        }
        Ok(())
    }

    #[test]
    fn durability_report_clone() {
        assert!(test_durability_report_clone().is_ok());
    }

    /// Test 76: DurabilityReport with no resource metrics returns within bounds.
    fn test_durability_report_no_metrics_within_bounds() -> Result<(), String> {
        let report = DurabilityReport::new(DurabilityLevel::Strict, Vec::new(), Vec::new(), 0);
        // Empty metrics => all metrics within bounds (vacuously true)
        if !report.resources_within_bounds() {
            return Err("Empty metrics should be vacuously within bounds".into());
        }
        Ok(())
    }

    #[test]
    fn durability_report_no_metrics_within_bounds() {
        assert!(test_durability_report_no_metrics_within_bounds().is_ok());
    }

    // =====================================================================
    // Phase 2B: check_durability_level() tests (tests 77-80)
    // =====================================================================

    /// Test 77: check_durability_level returns Strict for fully safe workflow.
    fn test_check_durability_level_strict() -> Result<(), String> {
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
        let level = check_durability_level(&panel);
        if level != DurabilityLevel::Strict {
            return Err(format!("Safe workflow should be Strict, got {:?}", level));
        }
        Ok(())
    }

    #[test]
    fn check_durability_level_strict() {
        assert!(test_check_durability_level_strict().is_ok());
    }

    /// Test 78: check_durability_level returns BestEffort for unprotected Do.
    fn test_check_durability_level_best_effort() -> Result<(), String> {
        let nodes = vec![
            make_do_node(0, 1, 0),
            make_node(
                1,
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            ),
        ];
        let panel = DurabilityPanel::from_workflow(&nodes);
        let level = check_durability_level(&panel);
        if level != DurabilityLevel::BestEffort {
            return Err(format!(
                "Unprotected Do should be BestEffort, got {:?}",
                level
            ));
        }
        Ok(())
    }

    #[test]
    fn check_durability_level_best_effort() {
        assert!(test_check_durability_level_best_effort().is_ok());
    }

    /// Test 79: check_durability_level returns Journaled when on_error present
    /// but reconciliation risk exists.
    fn test_check_durability_level_journaled() -> Result<(), String> {
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
        let level = check_durability_level(&panel);
        if level != DurabilityLevel::Journaled {
            return Err(format!(
                "Journaled Do under retry should be Journaled, got {:?}",
                level
            ));
        }
        Ok(())
    }

    #[test]
    fn check_durability_level_journaled() {
        assert!(test_check_durability_level_journaled().is_ok());
    }

    /// Test 80: check_durability_level for empty workflow is Strict.
    fn test_check_durability_level_empty_workflow() -> Result<(), String> {
        let panel = DurabilityPanel::from_workflow(&[]);
        let level = check_durability_level(&panel);
        // Empty workflow passes all checks -> Strict
        if level != DurabilityLevel::Strict {
            return Err(format!("Empty workflow should be Strict, got {:?}", level));
        }
        Ok(())
    }

    #[test]
    fn check_durability_level_empty_workflow() {
        assert!(test_check_durability_level_empty_workflow().is_ok());
    }

    // =====================================================================
    // Phase 2B: compute_resource_usage() tests (tests 81-85)
    // =====================================================================

    /// Test 81: compute_resource_usage returns three metrics.
    fn test_compute_resource_usage_count() -> Result<(), String> {
        let bounds = ResourceBudgetBounds::defaults();
        let metrics = compute_resource_usage(100, 200, 300, &bounds);
        if metrics.len() != 3 {
            return Err(format!("Should return 3 metrics, got {}", metrics.len()));
        }
        Ok(())
    }

    #[test]
    fn compute_resource_usage_count() {
        assert!(test_compute_resource_usage_count().is_ok());
    }

    /// Test 82: compute_resource_usage metrics have correct names.
    fn test_compute_resource_usage_names() -> Result<(), String> {
        let bounds = ResourceBudgetBounds::defaults();
        let metrics = compute_resource_usage(100, 200, 300, &bounds);
        let names: Vec<&str> = metrics.iter().map(|m| m.name.as_str()).collect();
        if !names.contains(&"memory") {
            return Err("Missing 'memory' metric".into());
        }
        if !names.contains(&"cpu_time") {
            return Err("Missing 'cpu_time' metric".into());
        }
        if !names.contains(&"wall_time") {
            return Err("Missing 'wall_time' metric".into());
        }
        Ok(())
    }

    #[test]
    fn compute_resource_usage_names() {
        assert!(test_compute_resource_usage_names().is_ok());
    }

    /// Test 83: compute_resource_usage metrics have correct used values.
    fn test_compute_resource_usage_values() -> Result<(), String> {
        let bounds = ResourceBudgetBounds::new(512, 5000, 10_000);
        let metrics = compute_resource_usage(256, 1000, 5000, &bounds);
        let mem = metrics.iter().find(|m| m.name == "memory");
        let Some(mem) = mem else {
            return Err("memory metric missing".into());
        };
        if mem.used != 256 {
            return Err(format!("memory used should be 256, got {}", mem.used));
        }
        if mem.limit != 512 {
            return Err(format!("memory limit should be 512, got {}", mem.limit));
        }
        let cpu = metrics.iter().find(|m| m.name == "cpu_time");
        let Some(cpu) = cpu else {
            return Err("cpu_time metric missing".into());
        };
        if cpu.used != 1000 {
            return Err(format!("cpu_time used should be 1000, got {}", cpu.used));
        }
        let wall = metrics.iter().find(|m| m.name == "wall_time");
        let Some(wall) = wall else {
            return Err("wall_time metric missing".into());
        };
        if wall.used != 5000 {
            return Err(format!("wall_time used should be 5000, got {}", wall.used));
        }
        Ok(())
    }

    #[test]
    fn compute_resource_usage_values() {
        assert!(test_compute_resource_usage_values().is_ok());
    }

    /// Test 84: compute_resource_usage all within bounds.
    fn test_compute_resource_usage_all_within() -> Result<(), String> {
        let bounds = ResourceBudgetBounds::new(1000, 5000, 10_000);
        let metrics = compute_resource_usage(100, 500, 2000, &bounds);
        for metric in &metrics {
            if !metric.within_bounds() {
                return Err(format!(
                    "Metric '{}' should be within bounds (used={}, limit={})",
                    metric.name, metric.used, metric.limit
                ));
            }
        }
        Ok(())
    }

    #[test]
    fn compute_resource_usage_all_within() {
        assert!(test_compute_resource_usage_all_within().is_ok());
    }

    /// Test 85: compute_resource_usage one over limit.
    fn test_compute_resource_usage_one_over() -> Result<(), String> {
        let bounds = ResourceBudgetBounds::new(100, 5000, 10_000);
        let metrics = compute_resource_usage(200, 500, 2000, &bounds);
        let mem = metrics.iter().find(|m| m.name == "memory");
        let Some(mem) = mem else {
            return Err("memory metric missing".into());
        };
        if mem.within_bounds() {
            return Err("Memory should be over limit".into());
        }
        // CPU and wall should still be within bounds.
        let cpu = metrics.iter().find(|m| m.name == "cpu_time");
        let Some(cpu) = cpu else {
            return Err("cpu_time metric missing".into());
        };
        if !cpu.within_bounds() {
            return Err("CPU should be within bounds".into());
        }
        Ok(())
    }

    #[test]
    fn compute_resource_usage_one_over() {
        assert!(test_compute_resource_usage_one_over().is_ok());
    }

    // =====================================================================
    // Phase 2B: build_durability_report() integration tests (tests 86-90)
    // =====================================================================

    /// Test 86: build_durability_report creates a complete report from a safe workflow.
    fn test_build_durability_report_safe() -> Result<(), String> {
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
        let bounds = ResourceBudgetBounds::defaults();
        let report = build_durability_report(&panel, 100, 500, 2000, &bounds, 99_999);

        if report.level != DurabilityLevel::Strict {
            return Err(format!("Expected Strict, got {:?}", report.level));
        }
        if !report.all_passed() {
            return Err("All checks should pass for safe workflow".into());
        }
        if !report.resources_within_bounds() {
            return Err("Resources should be within bounds".into());
        }
        if report.timestamp_micros != 99_999 {
            return Err(format!(
                "Timestamp should be 99999, got {}",
                report.timestamp_micros
            ));
        }
        Ok(())
    }

    #[test]
    fn build_durability_report_safe() {
        assert!(test_build_durability_report_safe().is_ok());
    }

    /// Test 87: build_durability_report with failing workflow.
    fn test_build_durability_report_failing() -> Result<(), String> {
        let nodes = vec![
            make_do_node(0, 1, 0),
            make_node(
                1,
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            ),
        ];
        let panel = DurabilityPanel::from_workflow(&nodes);
        let bounds = ResourceBudgetBounds::defaults();
        let report = build_durability_report(&panel, 100, 500, 2000, &bounds, 0);

        if report.level != DurabilityLevel::BestEffort {
            return Err(format!("Expected BestEffort, got {:?}", report.level));
        }
        if report.all_passed() {
            return Err("Checks should not all pass for unprotected workflow".into());
        }
        Ok(())
    }

    #[test]
    fn build_durability_report_failing() {
        assert!(test_build_durability_report_failing().is_ok());
    }

    /// Test 88: build_durability_report carries all checks from panel.
    fn test_build_durability_report_carries_checks() -> Result<(), String> {
        let panel = DurabilityPanel::from_workflow(&[]);
        let bounds = ResourceBudgetBounds::defaults();
        let report = build_durability_report(&panel, 0, 0, 0, &bounds, 0);

        if report.checks.len() != 4 {
            return Err(format!(
                "Empty workflow should produce 4 checks, got {}",
                report.checks.len()
            ));
        }
        Ok(())
    }

    #[test]
    fn build_durability_report_carries_checks() {
        assert!(test_build_durability_report_carries_checks().is_ok());
    }

    /// Test 89: build_durability_report with over-budget resources.
    fn test_build_durability_report_over_budget() -> Result<(), String> {
        let nodes = vec![
            make_do_node_with_error_handler(0, 1, 0, 5),
            make_node(5, CompiledNodeKind::Nop),
        ];
        let panel = DurabilityPanel::from_workflow(&nodes);
        let bounds = ResourceBudgetBounds::new(100, 100, 100);
        let report = build_durability_report(&panel, 200, 50, 75, &bounds, 0);

        if report.resources_within_bounds() {
            return Err("Resources should NOT be within bounds (memory over)".into());
        }
        // Only memory is over; cpu and wall are fine.
        let over_metrics: Vec<&DurabilityResourceMetric> = report
            .resource_metrics
            .iter()
            .filter(|m| !m.within_bounds())
            .collect();
        if over_metrics.len() != 1 {
            return Err(format!(
                "Expected exactly 1 over-limit metric, got {}",
                over_metrics.len()
            ));
        }
        let Some(over) = over_metrics.first() else {
            return Err("No over-limit metric found".into());
        };
        if over.name != "memory" {
            return Err(format!(
                "Over-limit metric should be memory, got {}",
                over.name
            ));
        }
        Ok(())
    }

    #[test]
    fn build_durability_report_over_budget() {
        assert!(test_build_durability_report_over_budget().is_ok());
    }

    /// Test 90: build_durability_report with journaled level workflow.
    fn test_build_durability_report_journaled() -> Result<(), String> {
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
        let bounds = ResourceBudgetBounds::defaults();
        let report = build_durability_report(&panel, 50, 100, 500, &bounds, 1_234);

        if report.level != DurabilityLevel::Journaled {
            return Err(format!("Expected Journaled, got {:?}", report.level));
        }
        // Each check should carry the Journaled level.
        for (i, check) in report.checks.iter().enumerate() {
            if check.level != DurabilityLevel::Journaled {
                return Err(format!(
                    "Check {} level should be Journaled, got {:?}",
                    i, check.level
                ));
            }
        }
        Ok(())
    }

    #[test]
    fn build_durability_report_journaled() {
        assert!(test_build_durability_report_journaled().is_ok());
    }
}
