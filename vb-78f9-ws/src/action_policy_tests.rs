#![forbid(unsafe_code)]
#![cfg(test)]

use vb_core::action::{
    ActionContract, Idempotency, SideEffect, RetrySafety,
};
use vb_core::ids::ActionId;
use vb_ui::verify::action_policy::{
    ActionPolicyReport, PolicyIssue, IdempotencyClass, analyze_actions,
};

#[test]
fn test_action_policy_report_missing_contract_has_timeout_issue() {
    let contract: Option<&ActionContract> = None;
    let action = ActionId::new(1);
    let report = ActionPolicyReport::for_action(action, contract);
    assert!(report.issues.contains(&PolicyIssue::MissingTimeout),
        "missing contract should produce MissingTimeout issue");
}

#[test]
fn test_action_policy_report_missing_contract_has_missing_idempotency_issue() {
    let contract: Option<&ActionContract> = None;
    let action = ActionId::new(2);
    let report = ActionPolicyReport::for_action(action, contract);
    assert!(report.issues.contains(&PolicyIssue::MissingIdempotency),
        "missing contract should produce MissingIdempotency issue");
}

#[test]
fn test_action_policy_report_unsafe_retry_contract_has_unsafe_retry_issue() {
    let contract = ActionContract {
        id: ActionId::new(3),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::AtLeastOnceExternal,
        side_effect: SideEffect::Destroys,
        retry_safety: RetrySafety::Unsafe,
        required_capabilities: Box::new([]),
    };
    let report = ActionPolicyReport::for_action(ActionId::new(3), Some(&contract));
    assert!(report.issues.contains(&PolicyIssue::UnsafeRetry),
        "Unsafe retry_safety should produce UnsafeRetry issue");
}

#[test]
fn test_action_policy_report_strict_eligible_requires_all_conditions() {
    let contract = ActionContract {
        id: ActionId::new(4),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    };
    let report = ActionPolicyReport::for_action(ActionId::new(4), Some(&contract));
    assert!(report.strict_eligible, "fully valid contract should be strict eligible");
    assert!(report.issues.is_empty(), "strict eligible report should have no issues");
    assert_eq!(report.idempotency_class, IdempotencyClass::DeterministicPure);
    assert!(report.has_timeout, "strict eligible should have timeout");
}

#[test]
fn test_action_policy_report_duplicate_dos_deduplicated() {
    use std::collections::HashMap;
    let mut reports: HashMap<ActionId, ActionPolicyReport> = HashMap::new();
    let action = ActionId::new(5);
    let contract: Option<&ActionContract> = None;
    ActionPolicyReport::insert_deduplicated(&mut reports, action, contract);
    ActionPolicyReport::insert_deduplicated(&mut reports, action, contract);
    assert_eq!(reports.len(), 1, "duplicate Do nodes should be deduplicated");
}

#[test]
fn test_action_policy_report_timeout_zero_implies_missing_timeout() {
    let contract = ActionContract {
        id: ActionId::new(6),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 0,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    };
    let report = ActionPolicyReport::for_action(ActionId::new(6), Some(&contract));
    assert!(report.issues.contains(&PolicyIssue::MissingTimeout),
        "zero timeout should produce MissingTimeout issue");
}

#[test]
fn test_action_policy_report_strict_eligible_false_when_issues_present() {
    let contract = ActionContract {
        id: ActionId::new(7),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    };
    let mut report = ActionPolicyReport::for_action(ActionId::new(7), Some(&contract));
    assert!(report.strict_eligible, "initial report should be strict eligible");
    report.issues.push(PolicyIssue::MissingTimeout);
    assert!(!report.strict_eligible, "report with issues should not be strict eligible");
}

#[test]
fn test_analyze_policies_on_fully_covered_workflow() {
    use vb_runtime::action::ActionRegistry;
    let mut registry = ActionRegistry::new();
    let contract1 = ActionContract {
        id: ActionId::new(10),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    };
    let contract2 = ActionContract {
        id: ActionId::new(11),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    };
    registry.register(contract1).expect("register should succeed");
    registry.register(contract2).expect("register should succeed");
    let workflow = vec![
        ActionId::new(10),
        ActionId::new(11),
    ];
    let registry_contracts: Vec<ActionContract> = registry.registered_contracts().into_iter().cloned().collect();
    let reports = analyze_actions(&workflow, &registry_contracts[..]);
    assert_eq!(reports.len(), 2, "should have report for each action");
    for report in &reports {
        assert!(report.strict_eligible, "fully covered workflow should have all strict_eligible");
    }
}

#[test]
fn test_analyze_policies_reports_missing_contracts() {
    use vb_runtime::action::ActionRegistry;
    let mut registry = ActionRegistry::new();
    let workflow = vec![ActionId::new(999)];
    let registry_contracts: Vec<ActionContract> = registry.registered_contracts().into_iter().cloned().collect();
    let reports = analyze_actions(&workflow, &registry_contracts[..]);
    assert!(!reports.is_empty(), "should have reports");
    let report = &reports[0];
    assert!(report.issues.contains(&PolicyIssue::MissingTimeout),
        "missing contract should produce MissingTimeout");
    assert!(report.issues.contains(&PolicyIssue::MissingIdempotency),
        "missing contract should produce MissingIdempotency");
}

#[test]
fn test_analyze_policies_reports_unsafe_retry() {
    use vb_runtime::action::ActionRegistry;
    let mut registry = ActionRegistry::new();
    let contract = ActionContract {
        id: ActionId::new(20),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: Idempotency::AtLeastOnceExternal,
        side_effect: SideEffect::Destroys,
        retry_safety: RetrySafety::Unsafe,
        required_capabilities: Box::new([]),
    };
    registry.register(contract).expect("register should succeed");
    let workflow = vec![ActionId::new(20)];
    let registry_contracts: Vec<ActionContract> = registry.registered_contracts().into_iter().cloned().collect();
    let reports = analyze_actions(&workflow, &registry_contracts[..]);
    assert!(reports[0].issues.contains(&PolicyIssue::UnsafeRetry),
        "Unsafe contract should produce UnsafeRetry issue");
}
