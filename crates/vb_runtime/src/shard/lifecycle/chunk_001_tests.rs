#[allow(clippy::expect_used, clippy::unwrap_used, clippy::as_conversions, clippy::panic)]
#[cfg(test)]
fn test_first_do_action(workflow: &CompiledWorkflow) -> Option<vb_core::ids::ActionId> {
    let mut index = 0u16;
    let count = workflow.node_count();
    while index < count {
        let step = StepIdx::new(index);
        if let Some(node) = workflow.node(step) {
            if let vb_core::workflow::CompiledNodeKind::Do { action, .. } = node.kind {
                return Some(action);
            }
        }
        index = index.saturating_add(1);
    }
    None
}

#[cfg(test)]
fn test_contract_required_capability(
    action: vb_core::ids::ActionId,
) -> vb_core::capability::Capability {
    vb_core::capability::Capability::new("__contract_required__".into(), action)
}

#[cfg(test)]
fn test_contract_grants(action: vb_core::ids::ActionId) -> CapabilitySet {
    CapabilitySet::from_grants(Box::from([test_contract_required_capability(action)]))
}

#[cfg(test)]
fn test_action_contract(
    action: vb_core::ids::ActionId,
    required: bool,
) -> vb_core::action::ActionContract {
    let required_capabilities = if required {
        Box::from([test_contract_required_capability(action)])
    } else {
        Box::from([])
    };
    vb_core::action::ActionContract {
        id: action,
        name: vb_core::action::ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: vb_core::action::Idempotency::DeterministicPure,
        side_effect: vb_core::action::SideEffect::Pure,
        retry_safety: vb_core::action::RetrySafety::Idempotent,
        required_capabilities,
    }
}

#[cfg(test)]
fn test_contracts_through(
    action: vb_core::ids::ActionId,
) -> Box<[vb_core::action::ActionContract]> {
    let target = action.get();
    let mut contracts = Vec::with_capacity(usize::from(target).saturating_add(1));
    let mut id = 0u16;
    loop {
        let current = vb_core::ids::ActionId::new(id);
        contracts.push(test_action_contract(current, id == target));
        if id == target {
            break;
        }
        id = id.saturating_add(1);
    }
    contracts.into_boxed_slice()
}

// =========================================================================
// vb-u09ai: 4-variant RetrySafety chunk_001 test (Tier 1).
// =========================================================================

/// Tier 1: `vb_core::action::is_idempotent(RetrySafety::Idempotent) == true`
/// per the master §65 contract (C6). The `is_idempotent(RetrySafety)` const
/// fn is a TDD target State 11 will add — on 3-variant code this test
/// fails to compile (preserves the failing-first signal).
#[test]
fn chunk_001_idempotent_retry_safety_recognized() {
    use vb_core::action::{is_idempotent, RetrySafety};
    assert!(
        is_idempotent(RetrySafety::Idempotent),
        "Idempotent must be considered idempotent (C6)"
    );
}