//! Runtime helpers for guarded rollback tests.

use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
use vb_core::ids::{ActionId, SlotIdx, StepIdx};
use vb_core::value::SlotValue;

pub(crate) fn action_output_with_slot(
    output_slot: SlotIdx,
    value: SlotValue,
) -> Result<vb_core::action::ActionOutputReady, String> {
    let encoded = postcard::to_allocvec(&value)
        .map_err(|e| format!("failed to encode action output: {e:?}"))?;
    let encoded_len = u32::try_from(encoded.len())
        .map_err(|e| format!("encoded length does not fit u32: {e:?}"))?;
    Ok(vb_core::action::ActionOutputReady {
        output_slot,
        value,
        taint: vb_core::Taint::Clean,
        encoded_len,
    })
}

pub(crate) fn f1_contracts_through(target: ActionId) -> Result<Box<[ActionContract]>, String> {
    let target_id = target.get();
    let contracts: Box<[ActionContract]> = (0..=target_id)
        .map(|id| {
            let current = ActionId::new(id);
            let name = ActionName::new("f1-action")
                .map_err(|e| format!("failed to construct ActionName: {e:?}"))?;
            Ok(ActionContract {
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
                required_capabilities: Box::new([]),
            })
        })
        .collect::<Result<Vec<_>, String>>()?
        .into_boxed_slice();
    Ok(contracts)
}

pub(super) fn complete(
    runtime: &mut vb_runtime::runtime::Runtime,
    run: vb_core::RunId,
    step: u16,
    action: u16,
    slot: u16,
    value: vb_core::SlotValue,
) -> Result<vb_core::action::ActionTicket, String> {
    let ticket = runtime
        .lookup_pending_action_ticket(run, u64::from(step))
        .ok_or_else(|| format!("no pending action ticket for run {run:?} step {step}"))?;

    if ticket.run != run {
        return Err(format!(
            "ticket run mismatch: expected {run:?}, got {:?}",
            ticket.run
        ));
    }
    if ticket.step != StepIdx::new(step) {
        return Err(format!(
            "ticket step mismatch: expected {step}, got {:?}",
            ticket.step
        ));
    }
    if ticket.action != ActionId::new(action) {
        return Err(format!(
            "ticket action mismatch: expected {action}, got {:?}",
            ticket.action
        ));
    }

    let output = action_output_with_slot(SlotIdx::new(slot), value)?;
    runtime
        .complete_action_with_output(ticket, output)
        .map_err(|e| format!("failed to complete action: {e:?}"))?;
    runtime
        .tick_all()
        .map_err(|e| format!("failed to tick: {e:?}"))?;

    Ok(ticket)
}
