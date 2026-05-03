//! Action policy panel -- analyzes Do-node policy compliance for the Verification screen.
//!
//! For each Do node in the compiled workflow, this module classifies the action's
//! idempotency, timeout coverage, strict-mode eligibility, and flags any policy
//! issues such as missing timeouts, missing idempotency declarations, or unsafe
//! retry configurations.

use vb_core::action::{ActionContract, Idempotency, RetrySafety};
use vb_core::ids::ActionId;
use vb_core::workflow::{CompiledNodeKind, WorkflowParts};

/// Classification of an action's idempotency guarantee.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdempotencyClass {
    /// Pure deterministic computation with no side effects.
    DeterministicPure,
    /// External call that is idempotent when retried with the same key.
    AtLeastOnce,
    /// No contract found or idempotency cannot be determined.
    Unknown,
}

/// A policy compliance issue found during action analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyIssue {
    /// The action has no timeout configured (timeout_ms == 0).
    MissingTimeout,
    /// The action has no idempotency declaration or it is Unknown.
    MissingIdempotency,
    /// The action has RetrySafety::Unsafe, meaning retries can cause duplicate side effects.
    UnsafeRetry,
}

/// Per-action policy compliance report produced by the verifier.
#[derive(Debug, Clone)]
pub struct ActionPolicyReport {
    /// The ActionId from the Do node.
    pub action_id: u16,
    /// Idempotency classification derived from the action contract.
    pub idempotency_class: IdempotencyClass,
    /// Whether the action has a non-zero timeout.
    pub has_timeout: bool,
    /// The timeout in milliseconds, if configured (non-zero).
    pub timeout_ms: Option<u32>,
    /// Whether this action is eligible for strict-mode execution.
    pub strict_eligible: bool,
    /// Policy issues found during analysis.
    pub issues: Vec<PolicyIssue>,
}

/// Analyzes all Do nodes in the workflow and produces a policy compliance report
/// for each unique action invocation.
///
/// The `contracts` slice maps action contracts by their ActionId. Actions not
/// found in the contracts slice are classified as `Unknown` idempotency and
/// receive `MissingTimeout` and `MissingIdempotency` issues.
///
/// Strict-mode eligibility requires:
/// - DeterministicPure idempotency class
/// - A non-zero timeout
/// - RetrySafety::Safe (no unsafe retry)
/// - No policy issues
pub fn analyze_action_policies(
    parts: &WorkflowParts,
    contracts: &[ActionContract],
) -> Vec<ActionPolicyReport> {
    let mut reports: Vec<ActionPolicyReport> = Vec::new();
    let mut seen_actions: Vec<u16> = Vec::new();

    for node in parts.nodes.iter() {
        if let CompiledNodeKind::Do { action, .. } = node.kind {
            let action_raw = action.get();
            if seen_actions.contains(&action_raw) {
                continue;
            }
            seen_actions.push(action_raw);

            let report = build_report(action, contracts);
            reports.push(report);
        }
    }

    reports
}

/// Builds a single action policy report by looking up the contract and classifying.
fn build_report(action: ActionId, contracts: &[ActionContract]) -> ActionPolicyReport {
    let action_raw = action.get();
    let contract = find_contract(action, contracts);

    let mut issues: Vec<PolicyIssue> = Vec::new();

    let idempotency_class = match contract {
        Some(c) => classify_idempotency(c),
        None => IdempotencyClass::Unknown,
    };

    let (has_timeout, timeout_ms) = match contract {
        Some(c) => {
            let configured = c.timeout_ms > 0;
            let ms = if configured {
                Some(u32::try_from(c.timeout_ms).ok().unwrap_or(u32::MAX))
            } else {
                None
            };
            (configured, ms)
        }
        None => (false, None),
    };

    // Check for issues.
    if !has_timeout {
        issues.push(PolicyIssue::MissingTimeout);
    }

    if idempotency_class == IdempotencyClass::Unknown {
        issues.push(PolicyIssue::MissingIdempotency);
    }

    if let Some(c) = contract
        && c.retry_safety == RetrySafety::Unsafe
    {
        issues.push(PolicyIssue::UnsafeRetry);
    }

    let strict_eligible = compute_strict_eligibility(idempotency_class, has_timeout, &issues);

    ActionPolicyReport {
        action_id: action_raw,
        idempotency_class,
        has_timeout,
        timeout_ms,
        strict_eligible,
        issues,
    }
}

/// Finds the contract for a given action ID in the provided slice.
fn find_contract(action: ActionId, contracts: &[ActionContract]) -> Option<&ActionContract> {
    let mut i = 0;
    while i < contracts.len() {
        if let Some(contract) = contracts.get(i)
            && contract.id == action
        {
            return Some(contract);
        }
        i = match i.checked_add(1) {
            Some(next) => next,
            None => break,
        };
    }
    None
}

/// Maps the contract's Idempotency enum to the UI-facing IdempotencyClass.
fn classify_idempotency(contract: &ActionContract) -> IdempotencyClass {
    match contract.idempotency {
        Idempotency::DeterministicPure => IdempotencyClass::DeterministicPure,
        Idempotency::IdempotentExternal | Idempotency::AtLeastOnceExternal => {
            IdempotencyClass::AtLeastOnce
        }
    }
}

/// Strict-mode eligibility: DeterministicPure, has timeout, no unsafe retry, no issues.
fn compute_strict_eligibility(
    idempotency: IdempotencyClass,
    has_timeout: bool,
    issues: &[PolicyIssue],
) -> bool {
    if idempotency != IdempotencyClass::DeterministicPure {
        return false;
    }
    if !has_timeout {
        return false;
    }
    // If any issue remains (e.g. UnsafeRetry), not eligible.
    if !issues.is_empty() {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::action::{Idempotency, RetrySafety, SideEffect};
    use vb_core::ids::{ActionId, SlotIdx, StepIdx, WorkflowDigest};
    use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};

    /// Helper: build a minimal WorkflowParts with the given node kinds.
    fn make_parts(kinds: Vec<CompiledNodeKind>) -> WorkflowParts {
        let nodes: Vec<CompiledNode> = kinds
            .into_iter()
            .enumerate()
            .map(|(i, kind)| CompiledNode {
                id: StepIdx::new(i as u16),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind,
            })
            .collect();
        let count = nodes.len();
        WorkflowParts {
            name: String::from("action-policy-test").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 4,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: (0..count)
                .map(|_| Box::<str>::from(""))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        }
    }

    /// Helper: build an ActionContract.
    fn make_contract(
        id: u16,
        timeout_ms: u64,
        idempotency: Idempotency,
        retry_safety: RetrySafety,
    ) -> ActionContract {
        ActionContract {
            id: ActionId::new(id),
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms,
            idempotency,
            side_effect: SideEffect::None,
            retry_safety,
            required_capabilities: Box::new([]),
        }
    }

    // Test 1: Empty workflow with no Do nodes produces no reports.
    #[test]
    fn analyze_empty_workflow_produces_no_reports() {
        let parts = make_parts(vec![CompiledNodeKind::Nop]);
        let reports = analyze_action_policies(&parts, &[]);
        assert!(reports.is_empty());
    }

    // Test 2: Single Do node with no contract is classified Unknown with issues.
    #[test]
    fn analyze_do_node_without_contract_flags_unknown() {
        let parts = make_parts(vec![
            CompiledNodeKind::Do {
                action: ActionId::new(1),
                input: SlotIdx::new(0),
            },
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        ]);
        let reports = analyze_action_policies(&parts, &[]);
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].action_id, 1);
        assert_eq!(reports[0].idempotency_class, IdempotencyClass::Unknown);
        assert!(!reports[0].has_timeout);
        assert!(reports[0].timeout_ms.is_none());
        assert!(!reports[0].strict_eligible);
        assert!(reports[0].issues.contains(&PolicyIssue::MissingTimeout));
        assert!(reports[0].issues.contains(&PolicyIssue::MissingIdempotency));
    }

    // Test 3: Do node with a DeterministicPure contract and timeout is strict eligible.
    #[test]
    fn analyze_deterministic_pure_with_timeout_is_strict_eligible() {
        let parts = make_parts(vec![
            CompiledNodeKind::Do {
                action: ActionId::new(5),
                input: SlotIdx::new(0),
            },
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        ]);
        let contracts = vec![make_contract(
            5,
            5000,
            Idempotency::DeterministicPure,
            RetrySafety::Safe,
        )];
        let reports = analyze_action_policies(&parts, &contracts);
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].action_id, 5);
        assert_eq!(
            reports[0].idempotency_class,
            IdempotencyClass::DeterministicPure
        );
        assert!(reports[0].has_timeout);
        assert_eq!(reports[0].timeout_ms, Some(5000));
        assert!(reports[0].issues.is_empty());
        assert!(reports[0].strict_eligible);
    }

    // Test 4: Do node with AtLeastOnce idempotency is not strict eligible.
    #[test]
    fn analyze_at_least_once_is_not_strict_eligible() {
        let parts = make_parts(vec![
            CompiledNodeKind::Do {
                action: ActionId::new(10),
                input: SlotIdx::new(0),
            },
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        ]);
        let contracts = vec![make_contract(
            10,
            3000,
            Idempotency::AtLeastOnceExternal,
            RetrySafety::KeyRequired,
        )];
        let reports = analyze_action_policies(&parts, &contracts);
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].idempotency_class, IdempotencyClass::AtLeastOnce);
        assert!(!reports[0].strict_eligible);
    }

    // Test 5: Do node with IdempotentExternal maps to AtLeastOnce class.
    #[test]
    fn analyze_idempotent_external_maps_to_at_least_once() {
        let parts = make_parts(vec![
            CompiledNodeKind::Do {
                action: ActionId::new(20),
                input: SlotIdx::new(0),
            },
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        ]);
        let contracts = vec![make_contract(
            20,
            2000,
            Idempotency::IdempotentExternal,
            RetrySafety::Safe,
        )];
        let reports = analyze_action_policies(&parts, &contracts);
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].idempotency_class, IdempotencyClass::AtLeastOnce);
    }

    // Test 6: Zero timeout triggers MissingTimeout issue.
    #[test]
    fn analyze_zero_timeout_flags_missing_timeout() {
        let parts = make_parts(vec![
            CompiledNodeKind::Do {
                action: ActionId::new(30),
                input: SlotIdx::new(0),
            },
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        ]);
        let contracts = vec![make_contract(
            30,
            0,
            Idempotency::DeterministicPure,
            RetrySafety::Safe,
        )];
        let reports = analyze_action_policies(&parts, &contracts);
        assert_eq!(reports.len(), 1);
        assert!(!reports[0].has_timeout);
        assert!(reports[0].timeout_ms.is_none());
        assert!(reports[0].issues.contains(&PolicyIssue::MissingTimeout));
        assert!(!reports[0].strict_eligible);
    }

    // Test 7: Unsafe retry safety triggers UnsafeRetry issue.
    #[test]
    fn analyze_unsafe_retry_flags_unsafe_retry_issue() {
        let parts = make_parts(vec![
            CompiledNodeKind::Do {
                action: ActionId::new(40),
                input: SlotIdx::new(0),
            },
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        ]);
        let contracts = vec![make_contract(
            40,
            1000,
            Idempotency::AtLeastOnceExternal,
            RetrySafety::Unsafe,
        )];
        let reports = analyze_action_policies(&parts, &contracts);
        assert_eq!(reports.len(), 1);
        assert!(reports[0].issues.contains(&PolicyIssue::UnsafeRetry));
        assert!(!reports[0].strict_eligible);
    }

    // Test 8: Multiple Do nodes with the same action produce one report (dedup).
    #[test]
    fn analyze_duplicate_do_nodes_deduplicates_by_action_id() {
        let parts = make_parts(vec![
            CompiledNodeKind::Do {
                action: ActionId::new(1),
                input: SlotIdx::new(0),
            },
            CompiledNodeKind::Do {
                action: ActionId::new(1),
                input: SlotIdx::new(1),
            },
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        ]);
        let reports = analyze_action_policies(&parts, &[]);
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].action_id, 1);
    }

    // Test 9: Multiple different Do nodes produce multiple reports.
    #[test]
    fn analyze_multiple_different_do_nodes_produces_multiple_reports() {
        let parts = make_parts(vec![
            CompiledNodeKind::Do {
                action: ActionId::new(1),
                input: SlotIdx::new(0),
            },
            CompiledNodeKind::Do {
                action: ActionId::new(2),
                input: SlotIdx::new(1),
            },
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        ]);
        let contracts = vec![
            make_contract(1, 1000, Idempotency::DeterministicPure, RetrySafety::Safe),
            make_contract(2, 2000, Idempotency::AtLeastOnceExternal, RetrySafety::KeyRequired),
        ];
        let reports = analyze_action_policies(&parts, &contracts);
        assert_eq!(reports.len(), 2);
        assert_eq!(reports[0].action_id, 1);
        assert_eq!(reports[1].action_id, 2);
    }

    // Test 10: DeterministicPure with UnsafeRetry is not strict eligible.
    #[test]
    fn deterministic_pure_with_unsafe_retry_is_not_strict_eligible() {
        let parts = make_parts(vec![
            CompiledNodeKind::Do {
                action: ActionId::new(50),
                input: SlotIdx::new(0),
            },
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        ]);
        let contracts = vec![make_contract(
            50,
            5000,
            Idempotency::DeterministicPure,
            RetrySafety::Unsafe,
        )];
        let reports = analyze_action_policies(&parts, &contracts);
        assert_eq!(reports.len(), 1);
        assert!(reports[0].issues.contains(&PolicyIssue::UnsafeRetry));
        assert!(!reports[0].strict_eligible);
    }

    // Test 11: Contract lookup is by ActionId, not by position.
    #[test]
    fn find_contract_uses_action_id_not_index() {
        let contracts = vec![
            make_contract(100, 1000, Idempotency::DeterministicPure, RetrySafety::Safe),
            make_contract(200, 2000, Idempotency::AtLeastOnceExternal, RetrySafety::KeyRequired),
        ];
        // Action 200 is at index 1, not 200.
        let found = find_contract(ActionId::new(200), &contracts);
        assert!(found.is_some());
        let found = found.ok_or("expected Some").ok();
        let contract = found.as_ref().ok_or("expected contract").ok();
        if let Some(c) = contract {
            assert_eq!(c.id, ActionId::new(200));
            assert_eq!(c.timeout_ms, 2000);
        }
    }

    // Test 12: Timeout overflow handling for large u64 timeout values.
    #[test]
    fn analyze_large_timeout_truncates_to_u32_max() {
        let parts = make_parts(vec![
            CompiledNodeKind::Do {
                action: ActionId::new(60),
                input: SlotIdx::new(0),
            },
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        ]);
        let mut contract =
            make_contract(60, u64::from(u32::MAX) + 1, Idempotency::DeterministicPure, RetrySafety::Safe);
        contract.timeout_ms = u64::from(u32::MAX) + 1;
        let contracts = vec![contract];
        let reports = analyze_action_policies(&parts, &contracts);
        assert_eq!(reports.len(), 1);
        assert!(reports[0].has_timeout);
        // Truncated to u32::MAX due to overflow.
        assert_eq!(reports[0].timeout_ms, Some(u32::MAX));
    }
}
