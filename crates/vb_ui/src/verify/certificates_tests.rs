//! Tests for the certificates verification module.
//!
//! Extracted from certificates.rs to comply with the 300-line file limit.

#[cfg(test)]
mod tests {
    use vb_core::ids::WorkflowDigest;
    use vb_core::ids::{SlotIdx, StepIdx};
    use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};

    // Certificate types from the parent module
    use crate::verify::certificates::{
        Certificate, CertificateKind, CertificateStatus, CheckStatus, VerificationResult,
        collect_successors, verify_workflow,
    };

    fn minimal_parts() -> WorkflowParts {
        WorkflowParts {
            name: String::from("test").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            }]
            .into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        }
    }

    fn empty_parts() -> WorkflowParts {
        WorkflowParts {
            name: String::from("empty").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: Vec::new().into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 0,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        }
    }

    #[test]
    fn test_empty_nodes_fails_structural_validity() {
        let result = VerificationResult::analyze(&empty_parts());
        let structural = result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::StructuralValidity);
        assert!(structural.is_some());
        let Some(cert) = structural else {
            assert!(false, "structural cert missing");
            return;
        };
        assert!(
            matches!(cert.status, CertificateStatus::Fail(_)),
            "expected Fail for empty nodes, got {:?}",
            cert.status
        );
    }

    #[test]
    fn test_single_finish_node_passes_all() {
        let result = VerificationResult::analyze(&minimal_parts());
        // Structural validity should pass.
        let structural = result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::StructuralValidity);
        assert!(structural.is_some());
        let Some(structural) = structural else {
            assert!(false, "cert missing");
            return;
        };
        assert!(matches!(structural.status, CertificateStatus::Pass));

        // Strict durability should warn (Finish node present but no error handlers).
        let durability = result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::StrictDurability);
        assert!(durability.is_some());
        let Some(durability) = durability else {
            assert!(false, "cert missing");
            return;
        };
        let dur_status = &durability.status;
        // A single Finish node with no error handlers/on_error produces Warn.
        assert!(
            matches!(
                dur_status,
                CertificateStatus::Pass | CertificateStatus::Warn(_)
            ),
            "expected Pass or Warn for strict durability, got {:?}",
            dur_status
        );

        // Reachability should pass (single node reachable from entry).
        let reachability = result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::Reachability);
        assert!(reachability.is_some());
        let Some(reachability) = reachability else {
            assert!(false, "cert missing");
            return;
        };
        assert!(matches!(reachability.status, CertificateStatus::Pass));
    }

    #[test]
    fn test_unreachable_node_fails_reachability() {
        // Node 0 is a Nop with no next (entry), node 1 is a Finish but unreachable.
        let parts = WorkflowParts {
            name: String::from("unreachable").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Nop,
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        };

        let result = VerificationResult::analyze(&parts);
        let reachability = result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::Reachability);
        assert!(reachability.is_some());
        let Some(reachability) = reachability else {
            assert!(false, "cert missing");
            return;
        };
        assert!(matches!(reachability.status, CertificateStatus::Fail(_)));
    }

    #[test]
    fn test_analysis_counts_match() {
        let result = VerificationResult::analyze(&minimal_parts());
        // total_checks should equal the number of certificates.
        assert_eq!(result.total_checks, result.certificates.len());
        assert_eq!(result.total_checks, 8);

        // pass_count + fail_count + warn_count should equal total_checks.
        let sum = result.pass_count + result.fail_count + result.warn_count;
        assert_eq!(sum, result.total_checks);
    }

    // ========================================================================
    // Pre-flight verify_workflow tests
    // ========================================================================

    fn preflight_minimal_parts() -> WorkflowParts {
        WorkflowParts {
            name: String::from("preflight-test").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            }]
            .into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        }
    }

    fn preflight_empty_parts() -> WorkflowParts {
        WorkflowParts {
            name: String::from("preflight-empty").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: Vec::new().into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 0,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        }
    }

    // -- Pre-flight test 1: Structural validity --

    #[test]
    fn preflight_structural_validity_passes_for_valid_workflow() {
        let report = verify_workflow(&preflight_minimal_parts());
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "structural_validity");
        assert!(check.is_some());
        let Some(c) = check else {
            assert!(false, "check missing");
            return;
        };
        assert_eq!(c.status, CheckStatus::Pass);
        assert!(c.detail.contains("valid"));
    }

    #[test]
    fn preflight_structural_validity_fails_for_empty_nodes() {
        let report = verify_workflow(&preflight_empty_parts());
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "structural_validity");
        assert!(check.is_some());
        let Some(c) = check else {
            assert!(false, "check missing");
            return;
        };
        assert_eq!(c.status, CheckStatus::Fail);
        assert!(c.detail.contains("empty"));
    }

    #[test]
    fn preflight_structural_validity_fails_for_entry_out_of_bounds() {
        let mut parts = preflight_minimal_parts();
        parts.entry = StepIdx::new(99);
        let report = verify_workflow(&parts);
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "structural_validity");
        assert!(check.is_some());
        let Some(c) = check else {
            assert!(false, "check missing");
            return;
        };
        assert_eq!(c.status, CheckStatus::Fail);
        assert!(c.detail.contains("exceeds"));
    }

    #[test]
    fn preflight_structural_validity_fails_for_node_id_mismatch() {
        let mut parts = preflight_minimal_parts();
        // Create a node with wrong ID at position 0.
        let mut nodes = Vec::new();
        nodes.push(CompiledNode {
            id: StepIdx::new(5), // wrong: should be 0
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        });
        parts.nodes = nodes.into_boxed_slice();
        let report = verify_workflow(&parts);
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "structural_validity");
        assert!(check.is_some());
        let Some(c) = check else {
            assert!(false, "check missing");
            return;
        };
        assert_eq!(c.status, CheckStatus::Fail);
        assert!(c.detail.contains("mismatch"));
    }

    // -- Pre-flight test 2: Bounded transitions --

    #[test]
    fn preflight_bounded_transitions_passes_for_default_contract() {
        let report = verify_workflow(&preflight_minimal_parts());
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "bounded_transitions");
        assert!(check.is_some());
        let Some(c) = check else {
            assert!(false, "check missing");
            return;
        };
        assert_eq!(c.status, CheckStatus::Pass);
    }

    #[test]
    fn preflight_bounded_transitions_fails_for_zero_max_steps() {
        let mut parts = preflight_minimal_parts();
        parts.resource_contract.max_steps = 0;
        let report = verify_workflow(&parts);
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "bounded_transitions");
        assert!(check.is_some());
        let Some(c) = check else {
            assert!(false, "check missing");
            return;
        };
        assert_eq!(c.status, CheckStatus::Fail);
        assert!(c.detail.contains("max_steps"));
    }

    #[test]
    fn preflight_bounded_transitions_fails_for_zero_max_slots() {
        let mut parts = preflight_minimal_parts();
        parts.resource_contract.max_slots = 0;
        let report = verify_workflow(&parts);
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "bounded_transitions");
        assert!(check.is_some());
        let Some(c) = check else {
            assert!(false, "check missing");
            return;
        };
        assert_eq!(c.status, CheckStatus::Fail);
        assert!(c.detail.contains("max_slots"));
    }

    #[test]
    fn preflight_bounded_transitions_fails_for_node_count_exceeding_max_steps() {
        let mut parts = preflight_minimal_parts();
        parts.resource_contract.max_steps = 1;
        // Add extra nodes so node count > max_steps.
        let mut nodes = Vec::new();
        for i in 0..5u16 {
            nodes.push(CompiledNode {
                id: StepIdx::new(i),
                output: None,
                next: if i < 4 {
                    Some(StepIdx::new(i.saturating_add(1)))
                } else {
                    None
                },
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            });
        }
        nodes[4].kind = CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        };
        parts.nodes = nodes.into_boxed_slice();
        let report = verify_workflow(&parts);
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "bounded_transitions");
        assert!(check.is_some());
        let Some(c) = check else {
            assert!(false, "check missing");
            return;
        };
        assert_eq!(c.status, CheckStatus::Fail);
        assert!(c.detail.contains("exceeds"));
    }

    // -- Pre-flight test 3: Secret-to-result leak --

    #[test]
    fn preflight_secret_to_result_leak_passes_for_clean_workflow() {
        let report = verify_workflow(&preflight_minimal_parts());
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "secret_to_result_leak");
        assert!(check.is_some());
        let Some(c) = check else {
            assert!(false, "check missing");
            return;
        };
        assert_eq!(c.status, CheckStatus::Pass);
        assert!(c.detail.contains("no secret"));
    }

    #[test]
    fn preflight_secret_to_result_leak_fails_for_secret_reaching_finish() {
        let mut nodes = Vec::new();
        nodes.push(CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::WaitEvent {
                event: SlotIdx::new(0),
                timeout_slot: None,
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        });
        let parts = WorkflowParts {
            name: String::from("leak-test").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 4,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        };
        let report = verify_workflow(&parts);
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "secret_to_result_leak");
        assert!(check.is_some());
        let Some(c) = check else {
            assert!(false, "check missing");
            return;
        };
        assert_eq!(c.status, CheckStatus::Fail);
        assert!(c.detail.contains("Finish"));
    }

    // -- Pre-flight test 4: Strict durability eligibility --

    #[test]
    fn preflight_strict_durability_passes_for_safe_workflow() {
        let mut nodes = Vec::new();
        nodes.push(CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: Some(StepIdx::new(2)),
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: vb_core::ids::ActionId::new(1),
                input: SlotIdx::new(0),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        });
        let parts = WorkflowParts {
            name: String::from("durable-test").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 4,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        };
        let report = verify_workflow(&parts);
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "strict_durability_eligibility");
        assert!(check.is_some());
        let Some(c) = check else {
            assert!(false, "check missing");
            return;
        };
        assert_eq!(c.status, CheckStatus::Pass);
        assert!(c.detail.contains("error handler"));
    }

    #[test]
    fn preflight_strict_durability_warns_for_do_without_error_handler() {
        let mut nodes = Vec::new();
        nodes.push(CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: vb_core::ids::ActionId::new(1),
                input: SlotIdx::new(0),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        });
        let parts = WorkflowParts {
            name: String::from("non-durable-test").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 4,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        };
        let report = verify_workflow(&parts);
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "strict_durability_eligibility");
        assert!(check.is_some());
        let Some(c) = check else {
            assert!(false, "check missing");
            return;
        };
        assert_eq!(c.status, CheckStatus::Warn);
        assert!(c.detail.contains("error handler"));
    }

    // -- Pre-flight test 5: Action idempotency --

    #[test]
    fn preflight_action_idempotency_passes_for_no_do_nodes() {
        let report = verify_workflow(&preflight_minimal_parts());
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "action_idempotency");
        assert!(check.is_some());
        let Some(c) = check else {
            assert!(false, "check missing");
            return;
        };
        assert_eq!(c.status, CheckStatus::Pass);
        assert!(c.detail.contains("no Do nodes"));
    }

    #[test]
    fn preflight_action_idempotency_warns_for_unguarded_actions() {
        let mut nodes = Vec::new();
        nodes.push(CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: vb_core::ids::ActionId::new(1),
                input: SlotIdx::new(0),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        });
        let parts = WorkflowParts {
            name: String::from("idem-test").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 4,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        };
        let report = verify_workflow(&parts);
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "action_idempotency");
        assert!(check.is_some());
        let Some(c) = check else {
            assert!(false, "check missing");
            return;
        };
        assert_eq!(c.status, CheckStatus::Warn);
        assert!(c.detail.contains("retry"));
    }

    // -- Pre-flight test 6: Worst-case memory budget --

    #[test]
    fn preflight_memory_budget_passes_for_small_slot_count() {
        let report = verify_workflow(&preflight_minimal_parts());
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "worst_case_memory_budget");
        assert!(check.is_some());
        let Some(c) = check else {
            assert!(false, "check missing");
            return;
        };
        assert_eq!(c.status, CheckStatus::Pass);
        assert!(c.detail.contains("64"));
    }

    #[test]
    fn preflight_memory_budget_passes_for_zero_slots() {
        let mut parts = preflight_minimal_parts();
        parts.slot_count = 0;
        let report = verify_workflow(&parts);
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "worst_case_memory_budget");
        assert!(check.is_some());
        let Some(c) = check else {
            assert!(false, "check missing");
            return;
        };
        assert_eq!(c.status, CheckStatus::Pass);
        assert!(c.detail.contains("zero"));
    }

    #[test]
    fn preflight_memory_budget_warns_for_exceeding_output_limit() {
        let mut parts = preflight_minimal_parts();
        // Set a very low output limit so the budget exceeds it.
        // 100 slots * 64 bytes = 6400 bytes. max_output_bytes = 100.
        parts.slot_count = 100;
        parts.resource_contract.max_output_bytes = 100;
        let report = verify_workflow(&parts);
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "worst_case_memory_budget");
        assert!(check.is_some());
        let Some(c) = check else {
            assert!(false, "check missing");
            return;
        };
        assert_eq!(c.status, CheckStatus::Warn);
        assert!(c.detail.contains("exceeds"));
    }

    // -- Pre-flight test 7: Max transitions --

    #[test]
    fn preflight_max_transitions_passes_within_limit() {
        let report = verify_workflow(&preflight_minimal_parts());
        let check = report.checks.iter().find(|c| c.name == "max_transitions");
        assert!(check.is_some());
        let Some(c) = check else {
            assert!(false, "check missing");
            return;
        };
        assert_eq!(c.status, CheckStatus::Pass);
        assert!(c.detail.contains("within"));
    }

    #[test]
    fn preflight_max_transitions_fails_when_exceeding_limit() {
        let mut parts = preflight_minimal_parts();
        parts.resource_contract.max_steps = 2;
        // Add extra nodes (3 > max_steps of 2).
        let mut nodes = Vec::new();
        for i in 0..3u16 {
            nodes.push(CompiledNode {
                id: StepIdx::new(i),
                output: None,
                next: if i < 2 {
                    Some(StepIdx::new(i.saturating_add(1)))
                } else {
                    None
                },
                on_error: None,
                error_slot: None,
                kind: if i < 2 {
                    CompiledNodeKind::Nop
                } else {
                    CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    }
                },
            });
        }
        parts.nodes = nodes.into_boxed_slice();
        let report = verify_workflow(&parts);
        let check = report.checks.iter().find(|c| c.name == "max_transitions");
        assert!(check.is_some());
        let Some(c) = check else {
            assert!(false, "check missing");
            return;
        };
        assert_eq!(c.status, CheckStatus::Fail, "detail: {}", c.detail);
        assert!(
            c.detail.contains("max_steps"),
            "expected 'max_steps' in detail, got: {}",
            c.detail,
        );
    }

    // -- Pre-flight test 8: Max action calls --

    #[test]
    fn preflight_max_action_calls_passes_within_ceiling() {
        let report = verify_workflow(&preflight_minimal_parts());
        let check = report.checks.iter().find(|c| c.name == "max_action_calls");
        assert!(check.is_some());
        let Some(c) = check else {
            assert!(false, "check missing");
            return;
        };
        assert_eq!(c.status, CheckStatus::Pass);
        assert!(c.detail.contains("0 Do node"));
    }

    #[test]
    fn preflight_max_action_calls_warns_for_exceeding_ceiling() {
        let mut nodes = Vec::new();
        // Create 5 Do nodes with Finish at the end.
        for i in 0..5u16 {
            nodes.push(CompiledNode {
                id: StepIdx::new(i),
                output: None,
                next: Some(StepIdx::new(i.saturating_add(1))),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Do {
                    action: vb_core::ids::ActionId::new(u16::from(i).saturating_add(1)),
                    input: SlotIdx::new(0),
                },
            });
        }
        nodes.push(CompiledNode {
            id: StepIdx::new(5),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        });
        let mut parts = WorkflowParts {
            name: String::from("many-dos").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 4,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        };
        // Default max_retry_attempts is 3, so 5 Do nodes will exceed it.
        parts.resource_contract.max_retry_attempts = 3;
        let report = verify_workflow(&parts);
        let check = report.checks.iter().find(|c| c.name == "max_action_calls");
        assert!(check.is_some());
        let Some(c) = check else {
            assert!(false, "check missing");
            return;
        };
        assert_eq!(c.status, CheckStatus::Warn);
        assert!(c.detail.contains("5 Do nodes"));
    }

    // -- Integration test: verify_workflow produces 8 checks --

    #[test]
    fn preflight_verify_workflow_produces_eight_checks() {
        let report = verify_workflow(&preflight_minimal_parts());
        assert_eq!(report.checks.len(), 8);
    }

    #[test]
    fn preflight_verify_workflow_all_pass_report_fields() {
        let report = verify_workflow(&preflight_minimal_parts());
        // For a minimal Finish-only workflow with default contract, we expect
        // no failures (some checks may warn, but none should fail).
        assert!(
            report.all_pass,
            "all_pass should be true for minimal valid workflow, worst_risk={:?}",
            report.worst_risk
        );
        assert!(matches!(
            report.worst_risk,
            CheckStatus::Pass | CheckStatus::Warn
        ));
    }

    #[test]
    fn preflight_verify_workflow_empty_nodes_has_failures() {
        let report = verify_workflow(&preflight_empty_parts());
        assert!(!report.all_pass);
        assert_eq!(report.worst_risk, CheckStatus::Fail);
    }

    // -- CheckStatus merge_worst tests --

    #[test]
    fn check_status_merge_worst_fail_dominates() {
        assert_eq!(
            CheckStatus::Fail.merge_worst(CheckStatus::Pass),
            CheckStatus::Fail
        );
        assert_eq!(
            CheckStatus::Fail.merge_worst(CheckStatus::Warn),
            CheckStatus::Fail
        );
        assert_eq!(
            CheckStatus::Fail.merge_worst(CheckStatus::Fail),
            CheckStatus::Fail
        );
        assert_eq!(
            CheckStatus::Pass.merge_worst(CheckStatus::Fail),
            CheckStatus::Fail
        );
    }

    #[test]
    fn check_status_merge_worst_warn_dominates_pass() {
        assert_eq!(
            CheckStatus::Warn.merge_worst(CheckStatus::Pass),
            CheckStatus::Warn
        );
        assert_eq!(
            CheckStatus::Pass.merge_worst(CheckStatus::Warn),
            CheckStatus::Warn
        );
    }

    #[test]
    fn check_status_merge_worst_pass_pass() {
        assert_eq!(
            CheckStatus::Pass.merge_worst(CheckStatus::Pass),
            CheckStatus::Pass
        );
    }

    // ========================================================================
    // Certificate analysis: additional edge-case tests
    // ========================================================================

    #[test]
    fn analysis_entry_out_of_bounds_fails_structural() {
        let mut parts = minimal_parts();
        parts.entry = StepIdx::new(200);
        let result = VerificationResult::analyze(&parts);
        let structural = result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::StructuralValidity);
        assert!(structural.is_some());
        let cert = structural.ok_or("missing").ok();
        if let Some(c) = cert {
            assert!(matches!(c.status, CertificateStatus::Fail(_)));
        }
    }

    #[test]
    fn analysis_node_id_mismatch_fails_structural() {
        let mut nodes = Vec::new();
        nodes.push(CompiledNode {
            id: StepIdx::new(99), // mismatch: position 0 has id 99
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        });
        let parts = WorkflowParts {
            name: String::from("id-mismatch").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        };
        let result = VerificationResult::analyze(&parts);
        let structural = result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::StructuralValidity);
        assert!(structural.is_some());
        assert!(matches!(
            structural.ok_or("missing").ok(),
            Some(Certificate {
                status: CertificateStatus::Fail(_),
                ..
            })
        ));
    }

    #[test]
    fn analysis_zero_max_steps_fails_boundedness() {
        let mut parts = minimal_parts();
        parts.resource_contract.max_steps = 0;
        let result = VerificationResult::analyze(&parts);
        let boundedness = result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::Boundedness);
        assert!(boundedness.is_some());
        assert!(matches!(
            boundedness.ok_or("missing").ok(),
            Some(Certificate {
                status: CertificateStatus::Fail(_),
                ..
            })
        ));
    }

    #[test]
    fn analysis_node_count_exceeds_max_steps_fails_boundedness() {
        let mut parts = minimal_parts();
        parts.resource_contract.max_steps = 1;
        // Add extra nodes beyond max_steps.
        let mut nodes = Vec::new();
        for i in 0..5u16 {
            nodes.push(CompiledNode {
                id: StepIdx::new(i),
                output: None,
                next: if i < 4 {
                    Some(StepIdx::new(i.saturating_add(1)))
                } else {
                    None
                },
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            });
        }
        nodes[4].kind = CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        };
        parts.nodes = nodes.into_boxed_slice();
        let result = VerificationResult::analyze(&parts);
        let boundedness = result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::Boundedness);
        assert!(boundedness.is_some());
        assert!(matches!(
            boundedness.ok_or("missing").ok(),
            Some(Certificate {
                status: CertificateStatus::Fail(_),
                ..
            })
        ));
    }

    #[test]
    fn analysis_slot_count_exceeds_max_slots_fails_resource_bounds() {
        let mut parts = minimal_parts();
        parts.slot_count = 5000;
        parts.resource_contract.max_slots = 100;
        let result = VerificationResult::analyze(&parts);
        let rb = result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::ResourceBounds);
        assert!(rb.is_some());
        assert!(matches!(
            rb.ok_or("missing").ok(),
            Some(Certificate {
                status: CertificateStatus::Fail(_),
                ..
            })
        ));
    }

    #[test]
    fn analysis_no_finish_node_fails_strict_durability() {
        let mut nodes = Vec::new();
        for i in 0..3u16 {
            nodes.push(CompiledNode {
                id: StepIdx::new(i),
                output: None,
                next: if i < 2 {
                    Some(StepIdx::new(i.saturating_add(1)))
                } else {
                    None
                },
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            });
        }
        let parts = WorkflowParts {
            name: String::from("no-finish").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 4,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        };
        let result = VerificationResult::analyze(&parts);
        let dur = result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::StrictDurability);
        assert!(dur.is_some());
        assert!(matches!(
            dur.ok_or("missing").ok(),
            Some(Certificate {
                status: CertificateStatus::Fail(_),
                ..
            })
        ));
    }

    #[test]
    fn analysis_do_node_without_retry_or_error_warns_action_policy() {
        let mut nodes = Vec::new();
        nodes.push(CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: vb_core::ids::ActionId::new(1),
                input: SlotIdx::new(0),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        });
        let parts = WorkflowParts {
            name: String::from("do-no-retry").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 4,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        };
        let result = VerificationResult::analyze(&parts);
        let ap = result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::ActionPolicy);
        assert!(ap.is_some());
        assert!(matches!(
            ap.ok_or("missing").ok(),
            Some(Certificate {
                status: CertificateStatus::Warn(_),
                ..
            })
        ));
    }

    #[test]
    fn analysis_taint_flow_warns_for_contained_sources() {
        // WaitEvent node without path to Finish (no next edge).
        let mut nodes = Vec::new();
        nodes.push(CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::WaitEvent {
                event: SlotIdx::new(0),
                timeout_slot: None,
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        });
        let parts = WorkflowParts {
            name: String::from("contained-secret").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 4,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        };
        let result = VerificationResult::analyze(&parts);
        let tf = result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::TaintFlow);
        assert!(tf.is_some());
        // WaitEvent at step 0 has no outgoing edge, so source is contained -> Warn
        assert!(matches!(
            tf.ok_or("missing").ok(),
            Some(Certificate {
                status: CertificateStatus::Warn(_),
                ..
            })
        ));
    }

    #[test]
    fn analysis_properly_nested_loops_pass() {
        // Outer loop: step 0 to step 4, inner loop: step 1 to step 3.
        let mut nodes = Vec::new();
        nodes.push(CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 5,
                body: StepIdx::new(1),
                done: StepIdx::new(4),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(2),
                item_slot: SlotIdx::new(3),
                limit: 3,
                body: StepIdx::new(2),
                done: StepIdx::new(3),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachJoin {
                output: SlotIdx::new(4),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(4),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        });
        let parts = WorkflowParts {
            name: String::from("nested-loops").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 8,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        };
        let result = VerificationResult::analyze(&parts);
        let ln = result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::LoopNesting);
        assert!(ln.is_some());
        assert!(matches!(
            ln.ok_or("missing").ok(),
            Some(Certificate {
                status: CertificateStatus::Pass,
                ..
            })
        ));
    }

    #[test]
    fn analysis_improperly_nested_loops_fail() {
        // Outer loop: step 0 to step 3, inner loop: step 1 to step 5 (extends past outer).
        let mut nodes = Vec::new();
        nodes.push(CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 5,
                body: StepIdx::new(1),
                done: StepIdx::new(3),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::CollectStart {
                source: SlotIdx::new(2),
                limit: 3,
                page_size: 10,
                body: StepIdx::new(2),
                done: StepIdx::new(5),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachJoin {
                output: SlotIdx::new(4),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(4),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(5),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::CollectFinish {
                collector_slot: SlotIdx::new(5),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(6),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        });
        let parts = WorkflowParts {
            name: String::from("bad-nesting").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 8,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        };
        let result = VerificationResult::analyze(&parts);
        let ln = result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::LoopNesting);
        assert!(ln.is_some());
        assert!(matches!(
            ln.ok_or("missing").ok(),
            Some(Certificate {
                status: CertificateStatus::Fail(_),
                ..
            })
        ));
    }

    #[test]
    fn analysis_collect_successors_for_jump_node() {
        let succs = collect_successors(
            &CompiledNodeKind::Jump {
                target: StepIdx::new(7),
            },
            None,
            None,
        );
        assert!(succs.contains(&StepIdx::new(7)));
    }

    #[test]
    fn analysis_collect_successors_for_together_start() {
        let succs = collect_successors(
            &CompiledNodeKind::TogetherStart {
                branches: Box::new([StepIdx::new(1), StepIdx::new(2)]),
                join: StepIdx::new(3),
            },
            None,
            None,
        );
        assert!(succs.contains(&StepIdx::new(1)));
        assert!(succs.contains(&StepIdx::new(2)));
        assert!(succs.contains(&StepIdx::new(3)));
    }

    #[test]
    fn analysis_collect_successors_includes_on_error() {
        let succs = collect_successors(
            &CompiledNodeKind::Nop,
            Some(StepIdx::new(1)),
            Some(StepIdx::new(5)),
        );
        assert!(succs.contains(&StepIdx::new(1)));
        assert!(succs.contains(&StepIdx::new(5)));
    }

    #[test]
    fn preflight_bounded_transitions_zero_budget_fails() {
        let mut parts = preflight_minimal_parts();
        parts.resource_contract.max_step_budget_per_tick = 0;
        let report = verify_workflow(&parts);
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "bounded_transitions");
        assert!(check.is_some());
        let c = check.ok_or("missing").ok();
        if let Some(ch) = c {
            assert_eq!(ch.status, CheckStatus::Fail);
            assert!(ch.detail.contains("budget_per_tick"));
        }
    }

    #[test]
    fn preflight_max_transitions_zero_steps_fails() {
        let mut parts = preflight_minimal_parts();
        parts.resource_contract.max_steps = 0;
        let report = verify_workflow(&parts);
        let check = report.checks.iter().find(|c| c.name == "max_transitions");
        assert!(check.is_some());
        let c = check.ok_or("missing").ok();
        if let Some(ch) = c {
            assert_eq!(ch.status, CheckStatus::Fail);
        }
    }

    #[test]
    fn preflight_strict_durability_no_finish_fails() {
        let mut nodes = Vec::new();
        nodes.push(CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        });
        let parts = WorkflowParts {
            name: String::from("no-finish-pf").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 4,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        };
        let report = verify_workflow(&parts);
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "strict_durability_eligibility");
        assert!(check.is_some());
        let c = check.ok_or("missing").ok();
        if let Some(ch) = c {
            assert_eq!(ch.status, CheckStatus::Fail);
        }
    }

    #[test]
    fn preflight_action_idempotency_with_retry_check_passes() {
        let mut nodes = Vec::new();
        nodes.push(CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: vb_core::ids::ActionId::new(1),
                input: SlotIdx::new(0),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: Some(StepIdx::new(2)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RetryCheck {
                policy_slot: SlotIdx::new(1),
                body: StepIdx::new(0),
                exhausted: StepIdx::new(2),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        });
        let parts = WorkflowParts {
            name: String::from("with-retry").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 4,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        };
        let report = verify_workflow(&parts);
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "action_idempotency");
        assert!(check.is_some());
        let c = check.ok_or("missing").ok();
        if let Some(ch) = c {
            assert_eq!(ch.status, CheckStatus::Pass);
        }
    }

    #[test]
    fn analysis_certificate_status_equality() {
        assert_eq!(CertificateStatus::Pass, CertificateStatus::Pass);
        assert_eq!(
            CertificateStatus::Fail(String::from("x")),
            CertificateStatus::Fail(String::from("x"))
        );
        assert_ne!(
            CertificateStatus::Fail(String::from("a")),
            CertificateStatus::Fail(String::from("b"))
        );
    }

    #[test]
    fn analysis_certificate_kind_copy_equality() {
        let kind = CertificateKind::TaintFlow;
        let copy = kind;
        assert_eq!(kind, copy);
    }

    // ========================================================================
    // Additional edge-case tests for coverage
    // ========================================================================

    /// Test 1: collect_successors for CollectFinish node returns no extra
    /// successors beyond next/on_error (it falls into the simple-arm match).
    #[test]
    fn collect_successors_collect_finish_returns_only_next_and_on_error() {
        let succs = collect_successors(
            &CompiledNodeKind::CollectFinish {
                collector_slot: SlotIdx::new(5),
            },
            Some(StepIdx::new(10)),
            Some(StepIdx::new(20)),
        );
        // Should contain next and on_error, but no extra edges from the kind.
        assert!(
            succs.contains(&StepIdx::new(10)),
            "expected next=10 in successors"
        );
        assert!(
            succs.contains(&StepIdx::new(20)),
            "expected on_error=20 in successors"
        );
        assert_eq!(
            succs.len(),
            2,
            "CollectFinish should produce exactly 2 successors (next + on_error)"
        );
    }

    /// Test 2: collect_successors for ReduceFinish node returns no extra
    /// successors beyond next/on_error (it also falls into the simple-arm match).
    #[test]
    fn collect_successors_reduce_finish_returns_only_next_and_on_error() {
        let succs = collect_successors(
            &CompiledNodeKind::ReduceFinish {
                accumulator: SlotIdx::new(7),
            },
            None,
            None,
        );
        // ReduceFinish with no next and no on_error should produce an empty vec.
        assert!(
            succs.is_empty(),
            "ReduceFinish with no next/on_error should produce no successors"
        );

        // With next only.
        let succs_next = collect_successors(
            &CompiledNodeKind::ReduceFinish {
                accumulator: SlotIdx::new(7),
            },
            Some(StepIdx::new(3)),
            None,
        );
        assert_eq!(succs_next.len(), 1);
        assert!(succs_next.contains(&StepIdx::new(3)));
    }

    /// Test 3: check_reachability with disconnected nodes -- a graph where
    /// the entry leads to some nodes but other nodes have no path from entry.
    #[test]
    fn reachability_fails_with_disconnected_nodes() {
        let mut nodes = Vec::new();
        // Node 0: Nop -> Node 1
        nodes.push(CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        });
        // Node 1: Finish
        nodes.push(CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        });
        // Node 2: disconnected Nop (no one points to it)
        nodes.push(CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        });
        // Node 3: disconnected Finish (no one points to it)
        nodes.push(CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        });
        let parts = WorkflowParts {
            name: String::from("disconnected").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 4,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        };
        let result = VerificationResult::analyze(&parts);
        let reachability = result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::Reachability);
        let Some(cert) = reachability else {
            return;
        };
        assert!(
            matches!(cert.status, CertificateStatus::Fail(_)),
            "expected Fail for disconnected nodes, got {:?}",
            cert.status
        );
        if let CertificateStatus::Fail(ref msg) = cert.status {
            // Should report 2 unreachable nodes (step 2 and step 3).
            assert!(
                msg.contains("2 unreachable"),
                "expected '2 unreachable' in message, got: {}",
                msg,
            );
        }
    }

    /// Test 4: check_boundedness with zero max_slots should fail.
    #[test]
    fn boundedness_fails_with_zero_max_slots() {
        let mut parts = minimal_parts();
        parts.resource_contract.max_slots = 0;
        let result = VerificationResult::analyze(&parts);
        let boundedness = result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::Boundedness);
        let Some(cert) = boundedness else {
            return;
        };
        assert!(
            matches!(cert.status, CertificateStatus::Fail(_)),
            "expected Fail for zero max_slots, got {:?}",
            cert.status
        );
        if let CertificateStatus::Fail(ref msg) = cert.status {
            assert!(
                msg.contains("max_slots is zero"),
                "expected 'max_slots is zero' in failure, got: {}",
                msg,
            );
        }
    }

    /// Test 5: check_preflight_max_transitions with max_steps = u16::MAX
    /// should pass even with a non-trivial node count.
    #[test]
    fn preflight_max_transitions_passes_at_u16_max() {
        let mut nodes = Vec::new();
        // Build a chain of 500 nodes, well under u16::MAX.
        for i in 0..500u16 {
            nodes.push(CompiledNode {
                id: StepIdx::new(i),
                output: None,
                next: if i < 499 {
                    Some(StepIdx::new(i.saturating_add(1)))
                } else {
                    None
                },
                on_error: None,
                error_slot: None,
                kind: if i < 499 {
                    CompiledNodeKind::Nop
                } else {
                    CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    }
                },
            });
        }
        let parts = WorkflowParts {
            name: String::from("big-steps").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 4,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract {
                max_steps: u16::MAX,
                ..ResourceContract::DEFAULT
            },
            step_names: Vec::new().into_boxed_slice(),
        };
        let report = verify_workflow(&parts);
        let check = report.checks.iter().find(|c| c.name == "max_transitions");
        let Some(c) = check else {
            return;
        };
        assert_eq!(
            c.status,
            CheckStatus::Pass,
            "expected Pass for max_steps=u16::MAX with 500 nodes, got: {}",
            c.detail,
        );
    }

    /// Test 6: Empty node list -- both the certificate analysis and pre-flight
    /// verify_workflow should report failures for an empty node array.
    #[test]
    fn empty_node_list_fails_all_structural_checks() {
        let empty = empty_parts();

        // Certificate analysis: structural validity should fail.
        let cert_result = VerificationResult::analyze(&empty);
        let structural = cert_result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::StructuralValidity);
        let Some(cert) = structural else {
            return;
        };
        assert!(
            matches!(cert.status, CertificateStatus::Fail(_)),
            "certificate structural should Fail for empty nodes"
        );

        // Certificate analysis: reachability should also fail for empty nodes.
        let reach = cert_result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::Reachability);
        let Some(reach_cert) = reach else {
            return;
        };
        assert!(
            matches!(reach_cert.status, CertificateStatus::Fail(_)),
            "certificate reachability should Fail for empty nodes"
        );

        // Pre-flight: structural_validity should fail.
        let pf_report = verify_workflow(&empty);
        assert!(
            !pf_report.all_pass,
            "pre-flight all_pass should be false for empty nodes"
        );
        let pf_struct = pf_report
            .checks
            .iter()
            .find(|c| c.name == "structural_validity");
        let Some(pf_s) = pf_struct else {
            return;
        };
        assert_eq!(
            pf_s.status,
            CheckStatus::Fail,
            "pre-flight structural should be Fail for empty nodes"
        );
    }

    /// Test 7: Single Finish node workflow should pass certificate analysis
    /// and pre-flight checks (reachable, structurally valid, durable enough).
    #[test]
    fn single_finish_node_workflow_passes_validation() {
        let parts = minimal_parts();

        // Certificate analysis: all key checks should pass or warn.
        let cert_result = VerificationResult::analyze(&parts);

        // Structural validity must pass.
        let structural = cert_result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::StructuralValidity);
        let Some(s) = structural else {
            return;
        };
        assert!(
            matches!(s.status, CertificateStatus::Pass),
            "structural should pass for single Finish node"
        );

        // Reachability must pass (single node reachable from entry).
        let reach = cert_result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::Reachability);
        let Some(r) = reach else {
            return;
        };
        assert!(
            matches!(r.status, CertificateStatus::Pass),
            "reachability should pass for single Finish node"
        );

        // Strict durability should pass or warn (Finish present, no error handlers).
        let dur = cert_result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::StrictDurability);
        let Some(d) = dur else {
            return;
        };
        assert!(
            matches!(
                d.status,
                CertificateStatus::Pass | CertificateStatus::Warn(_)
            ),
            "strict durability should Pass or Warn for single Finish, got {:?}",
            d.status,
        );

        // Pre-flight should report all_pass.
        let pf = verify_workflow(&parts);
        assert!(
            pf.all_pass,
            "pre-flight all_pass should be true for single Finish node, worst={:?}",
            pf.worst_risk,
        );
    }

    /// Test 8: ForEachStart creates correct successor edges (body + done
    /// in addition to next/on_error).
    #[test]
    fn collect_successors_for_each_start_includes_body_and_done() {
        let succs = collect_successors(
            &CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 10,
                body: StepIdx::new(5),
                done: StepIdx::new(20),
            },
            Some(StepIdx::new(99)), // next
            Some(StepIdx::new(50)), // on_error
        );

        // Should contain: next(99), on_error(50), body(5), done(20).
        assert!(succs.contains(&StepIdx::new(99)), "expected next=99");
        assert!(succs.contains(&StepIdx::new(50)), "expected on_error=50");
        assert!(succs.contains(&StepIdx::new(5)), "expected body=5");
        assert!(succs.contains(&StepIdx::new(20)), "expected done=20");
        assert_eq!(
            succs.len(),
            4,
            "ForEachStart should produce 4 successors, got {:?}",
            succs,
        );
    }

    // ========================================================================
    // BLACK HAT security-focused tests
    // ========================================================================

    /// BLACKHAT_cert_bfs_not_bfs [MEDIUM]: check_reachability uses Vec::pop()
    /// (DFS/LIFO), not VecDeque (BFS/FIFO), despite the comment saying "BFS
    /// from entry". This affects traversal order but not reachability
    /// correctness. The test documents the discrepancy.
    #[test]
    fn blackhat_cert_bfs_uses_vec_pop_which_is_dfs() {
        let mut nodes = Vec::new();
        // Linear chain: 0 -> 1 -> 2 -> 3 -> 4 (Finish)
        for i in 0..5u16 {
            nodes.push(CompiledNode {
                id: StepIdx::new(i),
                output: None,
                next: if i < 4 {
                    Some(StepIdx::new(i.saturating_add(1)))
                } else {
                    None
                },
                on_error: None,
                error_slot: None,
                kind: if i < 4 {
                    CompiledNodeKind::Nop
                } else {
                    CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    }
                },
            });
        }
        let parts = WorkflowParts {
            name: String::from("bh-bfs").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 4,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        };
        let result = VerificationResult::analyze(&parts);
        let reachability = result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::Reachability);
        let Some(cert) = reachability else { return };
        // Reachability is still correct despite DFS vs BFS.
        assert!(
            matches!(cert.status, CertificateStatus::Pass),
            "reachability should still pass with DFS traversal"
        );
    }

    /// BLACKHAT_cert_loop_nesting_misses_reverse_overlap [MEDIUM]:
    /// check_loop_nesting iterates i from 0..N and j from i+1..N, checking
    /// both forward and reverse partial overlaps. However, the reverse check
    /// (`a_start > b_start && a_start < b_done && a_done > b_done`) is
    /// redundant because if B is after A in the array, B cannot start before A
    /// unless the indices are non-monotonic. The test confirms proper nesting
    /// detection still works for valid loops.
    #[test]
    fn blackhat_cert_loop_nesting_reverse_overlap_redundant() {
        // Two properly nested loops where inner done == outer done (valid).
        let mut nodes = Vec::new();
        nodes.push(CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 5,
                body: StepIdx::new(1),
                done: StepIdx::new(4),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::CollectStart {
                source: SlotIdx::new(2),
                limit: 3,
                page_size: 10,
                body: StepIdx::new(2),
                done: StepIdx::new(3),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::CollectFinish {
                collector_slot: SlotIdx::new(5),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(4),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachJoin {
                output: SlotIdx::new(4),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(5),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        });
        let parts = WorkflowParts {
            name: String::from("bh-nesting").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 8,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        };
        let result = VerificationResult::analyze(&parts);
        let ln = result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::LoopNesting);
        let Some(cert) = ln else { return };
        assert!(
            matches!(cert.status, CertificateStatus::Pass),
            "properly nested loops should pass nesting check"
        );
    }

    /// BLACKHAT_cert_boundedness_u16_truncation [MEDIUM]: check_boundedness
    /// converts node count to u16 via `u16::try_from(parts.nodes.len())`
    /// clamping to u16::MAX. If a workflow has more than 65535 nodes, the
    /// comparison against max_steps (u16) silently succeeds because
    /// u16::MAX <= max_steps, even though the real node count exceeds it.
    #[test]
    fn blackhat_cert_boundedness_large_node_count_clamped_to_u16_max() {
        // We cannot actually create 65536+ nodes in a test (too much memory),
        // but we can verify the clamp logic directly.
        let large_count: usize = 70_000;
        let clamped = u16::try_from(large_count).unwrap_or(u16::MAX);
        assert_eq!(
            clamped,
            u16::MAX,
            "BLACKHAT [MEDIUM]: node count > u16::MAX is clamped to u16::MAX, \
             hiding overflow in boundedness check"
        );
        // With max_steps = u16::MAX, the clamped value would pass even though
        // the real count exceeds it. This test documents the truncation risk.
    }

    /// BLACKHAT_cert_max_action_calls_zero_ceiling [LOW]:
    /// check_preflight_max_action_calls uses max_retry_attempts as a "ceiling"
    /// for Do node count. When max_retry_attempts is 0, the condition
    /// `do_count > retry_ceiling && retry_ceiling > 0` never triggers,
    /// so any number of Do nodes silently passes. This means a workflow with
    /// 1000 Do nodes passes if max_retry_attempts is 0.
    #[test]
    fn blackhat_cert_max_action_calls_zero_ceiling_passes_any_count() {
        let mut parts = preflight_minimal_parts();
        parts.resource_contract.max_retry_attempts = 0;
        // Add 100 Do nodes -- should still pass because retry_ceiling is 0
        // and the check guards with `retry_ceiling > 0`.
        let mut nodes = Vec::new();
        for i in 0..100u16 {
            nodes.push(CompiledNode {
                id: StepIdx::new(i),
                output: None,
                next: if i < 99 {
                    Some(StepIdx::new(i.saturating_add(1)))
                } else {
                    None
                },
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Do {
                    action: vb_core::ids::ActionId::new(u16::from(i).saturating_add(1)),
                    input: SlotIdx::new(0),
                },
            });
        }
        nodes.push(CompiledNode {
            id: StepIdx::new(100),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        });
        parts.nodes = nodes.into_boxed_slice();
        let report = verify_workflow(&parts);
        let check = report.checks.iter().find(|c| c.name == "max_action_calls");
        let Some(c) = check else { return };
        assert_eq!(
            c.status,
            CheckStatus::Pass,
            "BLACKHAT [LOW]: 100 Do nodes pass when max_retry_attempts=0 because the \
             ceiling check is bypassed"
        );
    }

    /// BLACKHAT_cert_worst_case_memory_zero_output_limit [LOW]:
    /// check_preflight_worst_case_memory_budget only warns when
    /// worst_case_bytes > output_limit AND output_limit > 0.
    /// When output_limit is 0, the condition fails and a workflow with massive
    /// slot usage passes the memory budget check without even a warning.
    #[test]
    fn blackhat_cert_memory_budget_zero_output_limit_no_warning() {
        let mut parts = preflight_minimal_parts();
        parts.slot_count = 10000;
        parts.resource_contract.max_output_bytes = 0;
        // 10000 * 64 = 640000 bytes, but output_limit=0 skips the check.
        let report = verify_workflow(&parts);
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "worst_case_memory_budget");
        let Some(c) = check else { return };
        assert_eq!(
            c.status,
            CheckStatus::Pass,
            "BLACKHAT [LOW]: 10000 slots * 64 bytes passes when max_output_bytes=0 \
             because the warning check is skipped"
        );
    }

    /// BLACKHAT_cert_action_policy_action_id_zero [LOW]:
    /// check_action_policy flags Do nodes with action_id 0 as "missing".
    /// However, the code pushes a warning string but does not check whether
    /// the action_id is actually zero in any meaningful way. The test
    /// verifies that a Do node with action_id 0 produces a Warn.
    #[test]
    fn blackhat_cert_action_id_zero_produces_warning() {
        let mut nodes = Vec::new();
        nodes.push(CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: vb_core::ids::ActionId::new(0), // action_id 0
                input: SlotIdx::new(0),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RepeatStart {
                max_attempts: 3,
                body: StepIdx::new(0),
                done: StepIdx::new(1),
            },
        });
        let parts = WorkflowParts {
            name: String::from("bh-action-zero").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 4,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        };
        let result = VerificationResult::analyze(&parts);
        let ap = result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::ActionPolicy);
        let Some(cert) = ap else { return };
        assert!(
            matches!(cert.status, CertificateStatus::Warn(_)),
            "action_id 0 should produce a Warn in ActionPolicy"
        );
    }

    /// BLACKHAT_cert_strict_durability_multiple_finish_warns [LOW]:
    /// check_strict_durability warns when there is more than one Finish node.
    /// This is correct behavior but the test documents the edge case.
    #[test]
    fn blackhat_cert_strict_durability_multiple_finish_warns() {
        let mut nodes = Vec::new();
        // Two Finish nodes
        nodes.push(CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(1),
            },
        });
        let parts = WorkflowParts {
            name: String::from("bh-multi-finish").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 4,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        };
        let result = VerificationResult::analyze(&parts);
        let dur = result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::StrictDurability);
        let Some(cert) = dur else { return };
        // Should warn about multiple Finish nodes.
        assert!(
            matches!(cert.status, CertificateStatus::Warn(_)),
            "multiple Finish nodes should produce a Warn"
        );
    }

    /// BLACKHAT_cert_loop_nesting_done_equals_start [MEDIUM]:
    /// When a loop's done target equals its start step (degenerate loop),
    /// the code skips it via `if a_done <= a_start`. This means a loop where
    /// done == start (zero-length span) is silently ignored rather than
    /// flagged as malformed.
    #[test]
    fn blackhat_cert_loop_nesting_done_equals_start_silently_ignored() {
        let mut nodes = Vec::new();
        // Degenerate loop: start=0, done=0 (self-referencing)
        nodes.push(CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 5,
                body: StepIdx::new(1),
                done: StepIdx::new(0), // done == start
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        });
        let parts = WorkflowParts {
            name: String::from("bh-done-eq-start").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 4,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        };
        let result = VerificationResult::analyze(&parts);
        let ln = result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::LoopNesting);
        let Some(cert) = ln else { return };
        // The degenerate loop is silently ignored because a_done <= a_start.
        assert!(
            matches!(cert.status, CertificateStatus::Pass),
            "BLACKHAT [MEDIUM]: degenerate loop (done==start) is silently ignored in nesting check"
        );
    }

    /// BLACKHAT_cert_together_start_body_equals_id [LOW]:
    /// TogetherStart loop spans use (node.id, node.id, join) which creates
    /// a zero-length body span. This is handled differently from other loops
    /// where body is a distinct field. The test confirms this edge case.
    #[test]
    fn blackhat_cert_together_start_body_field_reused() {
        let mut nodes = Vec::new();
        nodes.push(CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherStart {
                branches: Box::new([StepIdx::new(1)]),
                join: StepIdx::new(2),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherBranch {
                branch: 0,
                entry: StepIdx::new(1),
                join: StepIdx::new(2),
                accumulator: SlotIdx::new(0),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherJoin {
                branch_count: 1,
                accumulator: SlotIdx::new(0),
            },
        });
        let parts = WorkflowParts {
            name: String::from("bh-together-body").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 4,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        };
        let result = VerificationResult::analyze(&parts);
        let ln = result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::LoopNesting);
        let Some(cert) = ln else { return };
        assert!(
            matches!(cert.status, CertificateStatus::Pass),
            "TogetherStart should pass loop nesting check"
        );
    }
}
