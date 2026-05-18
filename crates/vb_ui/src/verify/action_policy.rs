#![forbid(unsafe_code)]
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
#[non_exhaustive]
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
#[non_exhaustive]
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

impl ActionPolicyReport {
    /// Constructs a policy report for a single action.
    ///
    /// When `contract` is `None`, the action is treated as having no contract
    /// and will receive `MissingTimeout` and `MissingIdempotency` issues.
    pub fn for_action(action: ActionId, contract: Option<&ActionContract>) -> Self {
        let action_raw = action.get();
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

    /// Inserts a deduplicated report into the map.
    ///
    /// Only inserts if the action_id is not already present.
    /// This is used when processing workflows where multiple Do nodes
    /// may reference the same action.
    pub fn insert_deduplicated(
        reports: &mut std::collections::HashMap<ActionId, ActionPolicyReport>,
        action: ActionId,
        contract: Option<&ActionContract>,
    ) {
        reports
            .entry(action)
            .or_insert_with(|| Self::for_action(action, contract));
    }
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

/// Analyzes a workflow given as a slice of action IDs and a slice of contracts.
///
/// This is a simpler interface for cases where the full `WorkflowParts`
/// is not available. Each action in the workflow is analyzed once (deduplicated).
pub fn analyze_actions(
    workflow: &[ActionId],
    contracts: &[ActionContract],
) -> Vec<ActionPolicyReport> {
    let mut reports: Vec<ActionPolicyReport> = Vec::new();
    let mut seen: Vec<u16> = Vec::new();

    for &action in workflow.iter() {
        let action_raw = action.get();
        if seen.contains(&action_raw) {
            continue;
        }
        seen.push(action_raw);
        let contract = find_contract(action, contracts);
        let report = ActionPolicyReport::for_action(action, contract);
        reports.push(report);
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
    use vb_core::ids::{ActionId, ConstIdx, SlotIdx, StepIdx, WorkflowDigest};
    use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};

    /// Helper: build a minimal WorkflowParts with the given node kinds.
    fn make_parts(kinds: Vec<CompiledNodeKind>) -> WorkflowParts {
        let nodes: Vec<CompiledNode> = kinds
            .into_iter()
            .enumerate()
            .map(|(i, kind)| CompiledNode {
                id: StepIdx::new(u16::try_from(i).unwrap_or(u16::MAX)),
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
            make_contract(
                2,
                2000,
                Idempotency::AtLeastOnceExternal,
                RetrySafety::KeyRequired,
            ),
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
            make_contract(
                200,
                2000,
                Idempotency::AtLeastOnceExternal,
                RetrySafety::KeyRequired,
            ),
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
        let mut contract = make_contract(
            60,
            u64::from(u32::MAX) + 1,
            Idempotency::DeterministicPure,
            RetrySafety::Safe,
        );
        contract.timeout_ms = u64::from(u32::MAX) + 1;
        let contracts = vec![contract];
        let reports = analyze_action_policies(&parts, &contracts);
        assert_eq!(reports.len(), 1);
        assert!(reports[0].has_timeout);
        // Truncated to u32::MAX due to overflow.
        assert_eq!(reports[0].timeout_ms, Some(u32::MAX));
    }

    // Test 13: classify_idempotency for all variants.
    #[test]
    fn classify_idempotency_all_variants() {
        let pure_contract =
            make_contract(1, 1000, Idempotency::DeterministicPure, RetrySafety::Safe);
        assert_eq!(
            classify_idempotency(&pure_contract),
            IdempotencyClass::DeterministicPure
        );

        let at_least_once = make_contract(
            2,
            1000,
            Idempotency::AtLeastOnceExternal,
            RetrySafety::KeyRequired,
        );
        assert_eq!(
            classify_idempotency(&at_least_once),
            IdempotencyClass::AtLeastOnce
        );

        let idempotent_ext =
            make_contract(3, 1000, Idempotency::IdempotentExternal, RetrySafety::Safe);
        assert_eq!(
            classify_idempotency(&idempotent_ext),
            IdempotencyClass::AtLeastOnce
        );
    }

    // Test 14: compute_strict_eligibility requires all conditions.
    #[test]
    fn compute_strict_eligibility_all_conditions_required() {
        // All conditions met: DeterministicPure, has_timeout, no issues.
        assert!(compute_strict_eligibility(
            IdempotencyClass::DeterministicPure,
            true,
            &[],
        ));

        // Missing timeout.
        assert!(!compute_strict_eligibility(
            IdempotencyClass::DeterministicPure,
            false,
            &[],
        ));

        // Not DeterministicPure.
        assert!(!compute_strict_eligibility(
            IdempotencyClass::AtLeastOnce,
            true,
            &[],
        ));

        // Unknown idempotency.
        assert!(!compute_strict_eligibility(
            IdempotencyClass::Unknown,
            true,
            &[],
        ));

        // Has issues.
        assert!(!compute_strict_eligibility(
            IdempotencyClass::DeterministicPure,
            true,
            &[PolicyIssue::UnsafeRetry],
        ));
    }

    // Test 15: find_contract returns None for unknown action.
    #[test]
    fn find_contract_returns_none_for_unknown() {
        let contracts = vec![make_contract(
            1,
            1000,
            Idempotency::DeterministicPure,
            RetrySafety::Safe,
        )];
        assert!(find_contract(ActionId::new(999), &contracts).is_none());
    }

    // Test 16: find_contract returns None for empty slice.
    #[test]
    fn find_contract_returns_none_for_empty() {
        assert!(find_contract(ActionId::new(0), &[]).is_none());
    }

    // Test 17: ActionPolicyReport fields are populated correctly.
    #[test]
    fn action_policy_report_fields_correct() {
        let parts = make_parts(vec![
            CompiledNodeKind::Do {
                action: ActionId::new(42),
                input: SlotIdx::new(0),
            },
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        ]);
        let contracts = vec![make_contract(
            42,
            3000,
            Idempotency::DeterministicPure,
            RetrySafety::Safe,
        )];
        let reports = analyze_action_policies(&parts, &contracts);
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].action_id, 42);
        assert_eq!(
            reports[0].idempotency_class,
            IdempotencyClass::DeterministicPure
        );
        assert!(reports[0].has_timeout);
        assert_eq!(reports[0].timeout_ms, Some(3000));
        assert!(reports[0].strict_eligible);
        assert!(reports[0].issues.is_empty());
    }

    // Test 18: DeterministicPure with zero timeout has MissingTimeout issue.
    #[test]
    fn deterministic_pure_zero_timeout_has_missing_timeout() {
        let parts = make_parts(vec![
            CompiledNodeKind::Do {
                action: ActionId::new(1),
                input: SlotIdx::new(0),
            },
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        ]);
        let contracts = vec![make_contract(
            1,
            0,
            Idempotency::DeterministicPure,
            RetrySafety::Safe,
        )];
        let reports = analyze_action_policies(&parts, &contracts);
        assert_eq!(reports.len(), 1);
        assert!(!reports[0].has_timeout);
        assert!(reports[0].issues.contains(&PolicyIssue::MissingTimeout));
        assert!(!reports[0].strict_eligible);
    }

    // Test 19: All issue types can coexist.
    #[test]
    fn all_issues_can_coexist() {
        // Unknown idempotency (no contract) -> MissingIdempotency + MissingTimeout
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
        assert!(reports[0].issues.contains(&PolicyIssue::MissingTimeout));
        assert!(reports[0].issues.contains(&PolicyIssue::MissingIdempotency));
    }

    // Test 20: IdempotencyClass derive traits.
    #[test]
    fn idempotency_class_equality() {
        assert_eq!(
            IdempotencyClass::DeterministicPure,
            IdempotencyClass::DeterministicPure
        );
        assert_eq!(IdempotencyClass::AtLeastOnce, IdempotencyClass::AtLeastOnce);
        assert_eq!(IdempotencyClass::Unknown, IdempotencyClass::Unknown);
        assert_ne!(
            IdempotencyClass::DeterministicPure,
            IdempotencyClass::AtLeastOnce
        );
    }

    // Test 21: PolicyIssue derive traits.
    #[test]
    fn policy_issue_equality() {
        assert_eq!(PolicyIssue::MissingTimeout, PolicyIssue::MissingTimeout);
        assert_ne!(PolicyIssue::MissingTimeout, PolicyIssue::UnsafeRetry);
    }

    // =========================================================================
    // Additional coverage: timeout gaps, capability detection, strict-mode
    // edge cases, policy panel completeness, retry-safety interaction.
    // =========================================================================

    // Test 22: Timeout coverage gap -- one Do node has a timeout, another does not.
    // Verifies the panel correctly identifies which action has the gap.
    #[test]
    fn timeout_coverage_gap_mixed_do_nodes() {
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
            make_contract(1, 5000, Idempotency::DeterministicPure, RetrySafety::Safe),
            make_contract(2, 0, Idempotency::DeterministicPure, RetrySafety::Safe),
        ];
        let reports = analyze_action_policies(&parts, &contracts);
        assert_eq!(reports.len(), 2);

        // Action 1 has timeout, no gap.
        let r1 = reports.get(0);
        assert!(r1.is_some());
        let Some(r1) = r1 else { return };
        assert!(r1.has_timeout);
        assert!(!r1.issues.contains(&PolicyIssue::MissingTimeout));

        // Action 2 has no timeout -- gap detected.
        let r2 = reports.get(1);
        assert!(r2.is_some());
        let Some(r2) = r2 else { return };
        assert!(!r2.has_timeout);
        assert!(r2.issues.contains(&PolicyIssue::MissingTimeout));
    }

    // Test 23: Missing-capability detection -- Do node whose contract has no match.
    // An action without a contract in the slice receives Unknown classification,
    // which means capability information is unavailable and MissingIdempotency is flagged.
    #[test]
    fn missing_capability_detection_for_orphan_do_node() {
        let parts = make_parts(vec![
            CompiledNodeKind::Do {
                action: ActionId::new(99),
                input: SlotIdx::new(0),
            },
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        ]);
        // Contract exists for action 1 but NOT for action 99.
        let contracts = vec![make_contract(
            1,
            1000,
            Idempotency::DeterministicPure,
            RetrySafety::Safe,
        )];
        let reports = analyze_action_policies(&parts, &contracts);
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].action_id, 99);
        assert_eq!(reports[0].idempotency_class, IdempotencyClass::Unknown);
        assert!(reports[0].issues.contains(&PolicyIssue::MissingIdempotency));
        assert!(reports[0].issues.contains(&PolicyIssue::MissingTimeout));
        // No retry safety info available -- UnsafeRetry is NOT flagged (no contract to check).
        assert!(!reports[0].issues.contains(&PolicyIssue::UnsafeRetry));
    }

    // Test 24: Strict-mode eligibility -- AtLeastOnceExternal with timeout and Safe retry
    // is still NOT strict-eligible because idempotency class is not DeterministicPure.
    #[test]
    fn strict_eligible_at_least_once_with_safe_retry_still_rejected() {
        let parts = make_parts(vec![
            CompiledNodeKind::Do {
                action: ActionId::new(7),
                input: SlotIdx::new(0),
            },
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        ]);
        let contracts = vec![make_contract(
            7,
            3000,
            Idempotency::AtLeastOnceExternal,
            RetrySafety::Safe,
        )];
        let reports = analyze_action_policies(&parts, &contracts);
        assert_eq!(reports.len(), 1);
        assert!(reports[0].has_timeout);
        assert_eq!(reports[0].idempotency_class, IdempotencyClass::AtLeastOnce);
        assert!(reports[0].issues.is_empty());
        // Still not strict-eligible because not DeterministicPure.
        assert!(!reports[0].strict_eligible);
    }

    // Test 25: Policy panel with all Do nodes fully covered -- every action has a complete
    // contract with DeterministicPure, timeout, and Safe retry.
    #[test]
    fn policy_panel_all_do_nodes_covered_clean() {
        let parts = make_parts(vec![
            CompiledNodeKind::Do {
                action: ActionId::new(10),
                input: SlotIdx::new(0),
            },
            CompiledNodeKind::Do {
                action: ActionId::new(20),
                input: SlotIdx::new(1),
            },
            CompiledNodeKind::Do {
                action: ActionId::new(30),
                input: SlotIdx::new(2),
            },
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        ]);
        let contracts = vec![
            make_contract(10, 1000, Idempotency::DeterministicPure, RetrySafety::Safe),
            make_contract(20, 2000, Idempotency::DeterministicPure, RetrySafety::Safe),
            make_contract(30, 3000, Idempotency::DeterministicPure, RetrySafety::Safe),
        ];
        let reports = analyze_action_policies(&parts, &contracts);
        assert_eq!(reports.len(), 3);

        let mut all_clean = true;
        let mut all_strict = true;
        for report in &reports {
            if !report.issues.is_empty() {
                all_clean = false;
            }
            if !report.strict_eligible {
                all_strict = false;
            }
        }
        assert!(all_clean, "all reports should have zero issues");
        assert!(all_strict, "all reports should be strict-eligible");
    }

    // Test 26: Policy panel with some uncovered Do nodes -- mixing covered and uncovered
    // actions in the same workflow.
    #[test]
    fn policy_panel_partial_coverage_flags_uncovered_nodes() {
        let parts = make_parts(vec![
            CompiledNodeKind::Do {
                action: ActionId::new(1),
                input: SlotIdx::new(0),
            },
            CompiledNodeKind::Do {
                action: ActionId::new(2),
                input: SlotIdx::new(1),
            },
            CompiledNodeKind::Do {
                action: ActionId::new(3),
                input: SlotIdx::new(2),
            },
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        ]);
        // Only actions 1 and 3 have contracts; action 2 is uncovered.
        let contracts = vec![
            make_contract(1, 1000, Idempotency::DeterministicPure, RetrySafety::Safe),
            make_contract(3, 3000, Idempotency::DeterministicPure, RetrySafety::Safe),
        ];
        let reports = analyze_action_policies(&parts, &contracts);
        assert_eq!(reports.len(), 3);

        // Action 1: covered, clean.
        let r1 = reports.get(0);
        assert!(r1.is_some());
        let Some(r1) = r1 else { return };
        assert!(r1.issues.is_empty());
        assert!(r1.strict_eligible);

        // Action 2: uncovered, two issues.
        let r2 = reports.get(1);
        assert!(r2.is_some());
        let Some(r2) = r2 else { return };
        assert!(r2.issues.contains(&PolicyIssue::MissingTimeout));
        assert!(r2.issues.contains(&PolicyIssue::MissingIdempotency));
        assert!(!r2.strict_eligible);

        // Action 3: covered, clean.
        let r3 = reports.get(2);
        assert!(r3.is_some());
        let Some(r3) = r3 else { return };
        assert!(r3.issues.is_empty());
        assert!(r3.strict_eligible);
    }

    // Test 27: Empty workflow with only Nop and Finish produces a clean (empty) policy panel.
    #[test]
    fn empty_workflow_with_nop_and_finish_is_clean() {
        let parts = make_parts(vec![
            CompiledNodeKind::Nop,
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        ]);
        let reports = analyze_action_policies(&parts, &[]);
        assert!(reports.is_empty());
    }

    // Test 28: Multiple Do nodes with different policies -- mix of DeterministicPure,
    // IdempotentExternal, and AtLeastOnceExternal, verifying each gets its own class.
    #[test]
    fn multiple_do_nodes_with_distinct_idempotency_classes() {
        let parts = make_parts(vec![
            CompiledNodeKind::Do {
                action: ActionId::new(1),
                input: SlotIdx::new(0),
            },
            CompiledNodeKind::Do {
                action: ActionId::new(2),
                input: SlotIdx::new(1),
            },
            CompiledNodeKind::Do {
                action: ActionId::new(3),
                input: SlotIdx::new(2),
            },
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        ]);
        let contracts = vec![
            make_contract(1, 1000, Idempotency::DeterministicPure, RetrySafety::Safe),
            make_contract(
                2,
                2000,
                Idempotency::IdempotentExternal,
                RetrySafety::KeyRequired,
            ),
            make_contract(
                3,
                3000,
                Idempotency::AtLeastOnceExternal,
                RetrySafety::Unsafe,
            ),
        ];
        let reports = analyze_action_policies(&parts, &contracts);
        assert_eq!(reports.len(), 3);

        // Action 1: DeterministicPure, Safe -> strict eligible, no issues.
        let r1 = reports.get(0);
        assert!(r1.is_some());
        let Some(r1) = r1 else { return };
        assert_eq!(r1.idempotency_class, IdempotencyClass::DeterministicPure);
        assert!(r1.strict_eligible);
        assert!(r1.issues.is_empty());

        // Action 2: AtLeastOnce (mapped from IdempotentExternal), KeyRequired -> no UnsafeRetry.
        let r2 = reports.get(1);
        assert!(r2.is_some());
        let Some(r2) = r2 else { return };
        assert_eq!(r2.idempotency_class, IdempotencyClass::AtLeastOnce);
        assert!(!r2.strict_eligible);
        assert!(!r2.issues.contains(&PolicyIssue::UnsafeRetry));

        // Action 3: AtLeastOnce (mapped from AtLeastOnceExternal), Unsafe -> has UnsafeRetry.
        let r3 = reports.get(2);
        assert!(r3.is_some());
        let Some(r3) = r3 else { return };
        assert_eq!(r3.idempotency_class, IdempotencyClass::AtLeastOnce);
        assert!(!r3.strict_eligible);
        assert!(r3.issues.contains(&PolicyIssue::UnsafeRetry));
    }

    // Test 29: RetrySafety::KeyRequired does NOT trigger UnsafeRetry issue.
    // Only RetrySafety::Unsafe triggers that issue.
    #[test]
    fn key_required_retry_does_not_flag_unsafe_retry() {
        let parts = make_parts(vec![
            CompiledNodeKind::Do {
                action: ActionId::new(55),
                input: SlotIdx::new(0),
            },
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        ]);
        let contracts = vec![make_contract(
            55,
            5000,
            Idempotency::IdempotentExternal,
            RetrySafety::KeyRequired,
        )];
        let reports = analyze_action_policies(&parts, &contracts);
        assert_eq!(reports.len(), 1);
        assert!(!reports[0].issues.contains(&PolicyIssue::UnsafeRetry));
        // Not strict eligible because AtLeastOnce, not DeterministicPure.
        assert!(!reports[0].strict_eligible);
    }

    // Test 30: RetrySafety::Unsafe with AtLeastOnceExternal produces UnsafeRetry + not strict.
    // Also, no MissingTimeout because timeout is configured.
    #[test]
    fn unsafe_retry_with_timeout_has_only_unsafe_retry_issue() {
        let parts = make_parts(vec![
            CompiledNodeKind::Do {
                action: ActionId::new(66),
                input: SlotIdx::new(0),
            },
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        ]);
        let contracts = vec![make_contract(
            66,
            5000,
            Idempotency::AtLeastOnceExternal,
            RetrySafety::Unsafe,
        )];
        let reports = analyze_action_policies(&parts, &contracts);
        assert_eq!(reports.len(), 1);
        assert!(reports[0].has_timeout);
        assert!(!reports[0].issues.contains(&PolicyIssue::MissingTimeout));
        assert!(!reports[0].issues.contains(&PolicyIssue::MissingIdempotency));
        assert!(reports[0].issues.contains(&PolicyIssue::UnsafeRetry));
        assert!(!reports[0].strict_eligible);
    }

    // Test 31: DeterministicPure with Safe retry and zero timeout is NOT strict eligible.
    // The only issue is MissingTimeout.
    #[test]
    fn deterministic_pure_safe_retry_zero_timeout_not_strict_eligible() {
        let parts = make_parts(vec![
            CompiledNodeKind::Do {
                action: ActionId::new(77),
                input: SlotIdx::new(0),
            },
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        ]);
        let contracts = vec![make_contract(
            77,
            0,
            Idempotency::DeterministicPure,
            RetrySafety::Safe,
        )];
        let reports = analyze_action_policies(&parts, &contracts);
        assert_eq!(reports.len(), 1);
        assert_eq!(
            reports[0].idempotency_class,
            IdempotencyClass::DeterministicPure
        );
        assert!(reports[0].issues.contains(&PolicyIssue::MissingTimeout));
        assert!(!reports[0].issues.contains(&PolicyIssue::MissingIdempotency));
        assert!(!reports[0].issues.contains(&PolicyIssue::UnsafeRetry));
        assert!(!reports[0].strict_eligible);
    }

    // Test 32: RetrySafety interaction -- Unsafe retry always blocks strict eligibility
    // even when idempotency is DeterministicPure and timeout is present.
    #[test]
    fn unsafe_retry_blocks_strict_eligibility_even_with_deterministic_pure() {
        let parts = make_parts(vec![
            CompiledNodeKind::Do {
                action: ActionId::new(88),
                input: SlotIdx::new(0),
            },
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        ]);
        let contracts = vec![make_contract(
            88,
            10000,
            Idempotency::DeterministicPure,
            RetrySafety::Unsafe,
        )];
        let reports = analyze_action_policies(&parts, &contracts);
        assert_eq!(reports.len(), 1);
        assert!(reports[0].has_timeout);
        assert_eq!(
            reports[0].idempotency_class,
            IdempotencyClass::DeterministicPure
        );
        assert!(reports[0].issues.contains(&PolicyIssue::UnsafeRetry));
        assert!(!reports[0].strict_eligible);
        // UnsafeRetry is the sole issue -- timeout and idempotency are fine.
        assert!(!reports[0].issues.contains(&PolicyIssue::MissingTimeout));
        assert!(!reports[0].issues.contains(&PolicyIssue::MissingIdempotency));
    }

    // Test 33: Workflow with only non-Do nodes (SetConst, Copy, Nop, Finish)
    // produces zero reports regardless of contracts provided.
    #[test]
    fn workflow_with_only_non_do_nodes_produces_no_reports() {
        let parts = make_parts(vec![
            CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
            CompiledNodeKind::Copy {
                source: SlotIdx::new(0),
            },
            CompiledNodeKind::Nop,
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        ]);
        let contracts = vec![make_contract(
            1,
            1000,
            Idempotency::DeterministicPure,
            RetrySafety::Safe,
        )];
        let reports = analyze_action_policies(&parts, &contracts);
        assert!(reports.is_empty());
    }

    // Test 34: build_report for action with no contract returns Unknown
    // with exactly MissingTimeout and MissingIdempotency issues (not UnsafeRetry).
    #[test]
    fn build_report_no_contract_has_exact_two_issues() {
        let parts = make_parts(vec![
            CompiledNodeKind::Do {
                action: ActionId::new(200),
                input: SlotIdx::new(0),
            },
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        ]);
        let reports = analyze_action_policies(&parts, &[]);
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].issues.len(), 2);
        assert!(reports[0].issues.contains(&PolicyIssue::MissingTimeout));
        assert!(reports[0].issues.contains(&PolicyIssue::MissingIdempotency));
    }

    // Test 35: find_contract finds the first matching contract when duplicates exist.
    #[test]
    fn find_contract_returns_first_match_with_duplicate_ids() {
        let contracts = vec![
            make_contract(5, 1000, Idempotency::DeterministicPure, RetrySafety::Safe),
            make_contract(
                5,
                2000,
                Idempotency::AtLeastOnceExternal,
                RetrySafety::KeyRequired,
            ),
        ];
        let found = find_contract(ActionId::new(5), &contracts);
        assert!(found.is_some());
        let Some(found) = found else { return };
        assert_eq!(found.timeout_ms, 1000);
        assert_eq!(found.idempotency, Idempotency::DeterministicPure);
    }

    // Test 36: AtLeastOnceExternal with zero timeout gets both MissingTimeout
    // and is not strict eligible (but no MissingIdempotency since class is AtLeastOnce).
    #[test]
    fn at_least_once_zero_timeout_has_missing_timeout_only() {
        let parts = make_parts(vec![
            CompiledNodeKind::Do {
                action: ActionId::new(33),
                input: SlotIdx::new(0),
            },
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        ]);
        let contracts = vec![make_contract(
            33,
            0,
            Idempotency::AtLeastOnceExternal,
            RetrySafety::Safe,
        )];
        let reports = analyze_action_policies(&parts, &contracts);
        assert_eq!(reports.len(), 1);
        assert!(!reports[0].has_timeout);
        assert!(reports[0].issues.contains(&PolicyIssue::MissingTimeout));
        assert!(!reports[0].issues.contains(&PolicyIssue::MissingIdempotency));
        assert!(!reports[0].strict_eligible);
    }

    // =========================================================================
    // BLACKHAT security-focused tests
    // =========================================================================

    /// BLACKHAT_policy_timeout_u64_to_u32_truncation [MEDIUM]:
    /// build_report converts timeout_ms from u64 to u32 via
    /// `u32::try_from(c.timeout_ms).ok().unwrap_or(u32::MAX)`. Values above
    /// u32::MAX (~4.3 billion ms ~= 49 days) are silently clamped to u32::MAX.
    /// This means a timeout of 50 days appears as ~49.7 days, which could
    /// mislead users about the actual timeout.
    #[test]
    fn blackhat_policy_timeout_truncation_to_u32_max() {
        let huge_timeout: u64 = u64::from(u32::MAX) + 1000;
        assert!(
            huge_timeout > u64::from(u32::MAX),
            "test timeout should exceed u32::MAX"
        );
        let truncated = u32::try_from(huge_timeout).unwrap_or(u32::MAX);
        assert_eq!(
            truncated,
            u32::MAX,
            "BLACKHAT [MEDIUM]: timeout > u32::MAX is silently clamped to u32::MAX"
        );
    }

    /// BLACKHAT_policy_seen_actions_linear_scan [LOW]:
    /// analyze_action_policies uses `seen_actions.contains(&action_raw)` which
    /// is O(n) per Do node, making the total dedup O(n^2). For workflows with
    /// many Do nodes calling different actions, this is inefficient but not a
    /// correctness bug.
    #[test]
    fn blackhat_policy_many_actions_linear_dedup() {
        let mut kinds = Vec::new();
        // 50 Do nodes with different action IDs.
        for i in 0u16..50 {
            kinds.push(CompiledNodeKind::Do {
                action: ActionId::new(i),
                input: SlotIdx::new(0),
            });
        }
        kinds.push(CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        });
        let parts = make_parts(kinds);
        let reports = analyze_action_policies(&parts, &[]);
        assert_eq!(
            reports.len(),
            50,
            "50 distinct action IDs should produce 50 reports"
        );
    }

    /// BLACKHAT_policy_strict_eligible_needs_all_clean [CONFIRMED-SAFE]:
    /// compute_strict_eligibility correctly requires DeterministicPure,
    /// has_timeout=true, AND empty issues. A single issue (even UnsafeRetry)
    /// blocks strict eligibility.
    #[test]
    fn blackhat_policy_strict_eligible_blocked_by_single_issue() {
        assert!(
            !compute_strict_eligibility(
                IdempotencyClass::DeterministicPure,
                true,
                &[PolicyIssue::UnsafeRetry],
            ),
            "a single UnsafeRetry issue must block strict eligibility"
        );
        assert!(
            !compute_strict_eligibility(
                IdempotencyClass::DeterministicPure,
                true,
                &[PolicyIssue::MissingTimeout],
            ),
            "a single MissingTimeout issue must block strict eligibility"
        );
    }

    /// BLACKHAT_policy_action_id_zero_treated_as_unknown [LOW]:
    /// A Do node with action_id 0 and no matching contract is classified
    /// as Unknown, which is correct. But action_id 0 is a valid ID in the
    /// system, so this tests the edge case.
    #[test]
    fn blackhat_policy_action_id_zero_without_contract() {
        let parts = make_parts(vec![
            CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::new(0),
            },
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        ]);
        let reports = analyze_action_policies(&parts, &[]);
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].action_id, 0);
        assert_eq!(reports[0].idempotency_class, IdempotencyClass::Unknown);
        assert!(reports[0].issues.contains(&PolicyIssue::MissingIdempotency));
        assert!(reports[0].issues.contains(&PolicyIssue::MissingTimeout));
    }

    /// BLACKHAT_policy_find_contract_returns_first_match [CONFIRMED-SAFE]:
    /// When multiple contracts share the same action ID, find_contract returns
    /// the first match. This is correct behavior but means duplicate contracts
    /// silently mask later entries.
    #[test]
    fn blackhat_policy_duplicate_contracts_first_wins() {
        let contracts = vec![
            make_contract(1, 1000, Idempotency::DeterministicPure, RetrySafety::Safe),
            make_contract(
                1,
                5000,
                Idempotency::AtLeastOnceExternal,
                RetrySafety::Unsafe,
            ),
        ];
        let found = find_contract(ActionId::new(1), &contracts);
        assert!(found.is_some());
        let Some(c) = found else { return };
        // First contract wins.
        assert_eq!(c.timeout_ms, 1000);
        assert_eq!(c.idempotency, Idempotency::DeterministicPure);
    }

    /// BLACKHAT_policy_no_contract_no_unsafe_retry [CONFIRMED-SAFE]:
    /// When a Do node has no contract, UnsafeRetry is NOT flagged because
    /// there is no retry_safety field to check. Only MissingTimeout and
    /// MissingIdempotency are reported.
    #[test]
    fn blackhat_policy_no_contract_only_two_issues() {
        let parts = make_parts(vec![
            CompiledNodeKind::Do {
                action: ActionId::new(99),
                input: SlotIdx::new(0),
            },
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        ]);
        let reports = analyze_action_policies(&parts, &[]);
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].issues.len(), 2);
        assert!(reports[0].issues.contains(&PolicyIssue::MissingTimeout));
        assert!(reports[0].issues.contains(&PolicyIssue::MissingIdempotency));
        assert!(!reports[0].issues.contains(&PolicyIssue::UnsafeRetry));
    }
}
