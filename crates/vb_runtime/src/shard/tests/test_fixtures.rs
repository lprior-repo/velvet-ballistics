//! Shared test fixture helpers for action contracts and capability grants.

use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
use vb_core::capability::{Capability, CapabilitySet};
use vb_core::ids::{ActionId, RunId, SlotIdx};
use vb_core::SlotValue;

/// Create action contracts for all action IDs from 0 through the given maximum.
pub(crate) fn contracts_through(max_action: ActionId) -> Box<[ActionContract]> {
    let target = u16::from(max_action.get());
    let mut contracts = Vec::new();
    let mut id = 0u16;
    loop {
        let current = ActionId::new(id);
        let name = ActionName::from_static_infallible("test-action");
        contracts.push(ActionContract {
            id: current,
            name,
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 5000,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            required_capabilities: Box::new([Capability::new("__contract_required__".into(), current)]),
        });
        if id == target {
            break;
        }
        id = id.saturating_add(1);
    }
    contracts.into_boxed_slice()
}

/// Create a capability set that grants the contract capability for the given action.
pub(crate) fn contract_capability_for(action: ActionId) -> CapabilitySet {
    CapabilitySet::from_grants(Box::new([Capability::new("__contract_required__".into(), action)]))
}

/// Submit a suspended workflow with proper action contracts and capabilities.
/// This is the canonical way to submit workflows with Do actions in tests.
pub(crate) fn submit_suspended(
    shard: &mut super::super::types::Shard,
    run: RunId,
    workflow: vb_core::workflow::CompiledWorkflow,
) -> crate::RuntimeResult<()> {
    shard.handle_submit_with_inputs_and_contracts(
        run,
        workflow,
        &[(SlotIdx::new(0), SlotValue::I64(0))],
        contract_capability_for(ActionId::new(0)),
        &contracts_through(ActionId::new(0)),
    )
}
