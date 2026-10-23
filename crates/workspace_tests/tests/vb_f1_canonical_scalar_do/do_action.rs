//! Do action suspension, completion, and value verification tests.
//! Verifies actual slot values through journal events rather than counters.

use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
use vb_core::capability::{Capability, CapabilitySet};
use vb_core::ids::{ActionId, RunId, SlotIdx, StepIdx};
use vb_core::value::SlotValue;

use crate::vb_f1_scalar_do_support::{
    action_output, compile_f1_workflow, decode_slot_value, test_runtime_with_journal,
};

const DO_YAML: &str = "version: velvet-ballistics/v1\nname: f1_do\nwhen: { manual: {} }\nsteps:\n  - id: s1\n    set:\n      output: input_val\n      value: \"10\"\n  - id: s2\n    do:\n      action: \"7\"\n      input: \"$s1\"\n  - id: s3\n    finish:\n      result: 1\n";

fn f1_action_contract(action: ActionId) -> Result<ActionContract, String> {
    let name = ActionName::new("f1-action")
        .map_err(|e| format!("failed to construct ActionName: {e:?}"))?;
    Ok(ActionContract {
        id: action,
        name,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::from([Capability::new(
            Box::from("f1.action.required"),
            action,
        )]),
    })
}

fn f1_action_grants(action: ActionId) -> CapabilitySet {
    CapabilitySet::from_grants(Box::from([Capability::new(
        Box::from("f1.action.required"),
        action,
    )]))
}

fn f1_contracts_through(target: ActionId) -> Result<Box<[ActionContract]>, String> {
    let target_id = target.get();
    let contracts: Box<[ActionContract]> = (0..=target_id)
        .map(|id| {
            let current = ActionId::new(id);
            let mut contract = f1_action_contract(current)?;
            if id != target_id {
                contract.required_capabilities = Box::new([]);
            }
            Ok(contract)
        })
        .collect::<Result<Vec<_>, String>>()?
        .into_boxed_slice();
    Ok(contracts)
}

// Positive: real Do suspension + completion to Finish
// Proves actual slot values through journal events: input slot 0 = I64(10),
// action completion output slot 1 = I64(999), RunFinished result slot = 1.
#[test]
fn test_do_action_suspension_and_completion() -> Result<(), String> {
    let workflow = compile_f1_workflow(DO_YAML)?;
    let (mut runtime, journal) = test_runtime_with_journal()?;
    let run = RunId::new(1001);
    let action = ActionId::new(7);

    // Verify compiled IR: Do node at step 1 should have input slot 0
    // (the Set step writes to slot 0, Do reads from slot 0)
    let step_1_node = workflow
        .node(StepIdx::new(1))
        .ok_or("workflow should have step 1")?;
    match &step_1_node.kind {
        vb_core::workflow::CompiledNodeKind::Do { input, .. } => {
            if input != &SlotIdx::new(0) {
                return Err(format!(
                    "Do node input slot should be 0 (Set step output), got {:?}",
                    input
                ));
            }
        }
        other => {
            return Err(format!("step 1 should be a Do node, got {:?}", other));
        }
    }

    let contracts = f1_contracts_through(action)?;
    let grants = f1_action_grants(action);

    // Submit with empty input assignments — the source Set step owns input_val
    runtime
        .submit_direct_with_inputs_grants_and_contracts(
            run,
            workflow,
            Box::new([]),
            grants,
            contracts,
        )
        .map_err(|e| format!("submit failed: {e:?}"))?;

    runtime
        .tick_all()
        .map_err(|e| format!("tick failed: {e:?}"))?;

    // Verify suspension at the Do step via actual snapshot_run API
    let snap = runtime.snapshot_run(run, 1);
    match snap {
        Ok(vb_runtime::shard::InspectResponse::Found(snapshot)) => {
            if snapshot.pc != StepIdx::new(1) {
                return Err(format!(
                    "expected suspension at step 1 (Do), pc={:?}",
                    snapshot.pc
                ));
            }
        }
        Ok(vb_runtime::shard::InspectResponse::NotFound { .. }) => {
            return Err("expected run to be active at suspension".to_owned());
        }
        Ok(_) => {
            return Err("unexpected InspectResponse variant".to_owned());
        }
        Err(e) => {
            return Err(format!("snapshot_run failed: {e:?}"));
        }
    }

    let ticket = runtime
        .lookup_pending_action_ticket(run, u64::from(StepIdx::new(1).get()))
        .ok_or("expected pending action ticket for Do step")?;
    if ticket.action != action {
        return Err(format!(
            "action ticket action ID mismatch: {:#?}",
            ticket.action
        ));
    }
    if ticket.step != StepIdx::new(1) {
        return Err(format!("action ticket step mismatch: {:#?}", ticket.step));
    }

    let output = action_output(SlotValue::I64(999))?;
    runtime
        .complete_action_with_output(ticket, output)
        .map_err(|e| format!("action completion failed: {e:?}"))?;
    runtime
        .tick_all()
        .map_err(|e| format!("tick failed after completion: {e:?}"))?;

    // Verify actual values through journal events rather than counters.
    let journal_events = journal
        .snapshot()
        .map_err(|e| format!("journal snapshot failed: {e:?}"))?;

    // 1. Verify Set step wrote I64(10) to slot 0 (the Do input slot)
    let mut found_set_slot = false;
    for event in &journal_events {
        if let vb_runtime::journal::RuntimeJournalEvent::SlotWritten {
            run: r,
            slot,
            value,
            ..
        } = event
        {
            if *r == run && *slot == SlotIdx::new(0) {
                let decoded = decode_slot_value(value)?;
                if decoded != SlotValue::I64(10) {
                    return Err(format!(
                        "Set step wrote slot 0 with {:?} but expected I64(10)",
                        decoded
                    ));
                }
                found_set_slot = true;
            }
        }
    }
    if !found_set_slot {
        return Err(
            "expected SlotWritten event for slot 0 (Set step output) in journal".to_owned(),
        );
    }

    // 2. Verify action completion output slot 1 has I64(999)
    let mut found_action_output = false;
    for event in &journal_events {
        if let vb_runtime::journal::RuntimeJournalEvent::ActionCompletedEnvelope {
            ticket: t,
            output,
            value,
            ..
        } = event
        {
            if t.run == run && *output == SlotIdx::new(1) {
                let decoded = decode_slot_value(value)?;
                if decoded != SlotValue::I64(999) {
                    return Err(format!(
                        "Action completed with slot 1 = {:?} but expected I64(999)",
                        decoded
                    ));
                }
                found_action_output = true;
            }
        }
    }
    if !found_action_output {
        return Err("expected ActionCompletedEnvelope event for slot 1 in journal".to_owned());
    }

    // 3. Verify RunFinished event with correct result slot
    let mut found_run_finished = false;
    for event in &journal_events {
        if let vb_runtime::journal::RuntimeJournalEvent::RunFinished { run: r, result, .. } = event
        {
            if *r == run && *result == SlotIdx::new(1) {
                found_run_finished = true;
            }
        }
    }
    if !found_run_finished {
        return Err("expected RunFinished event with result slot 1 in journal".to_owned());
    }

    // After completion, the run should not be found in active snapshots
    let snap = runtime.snapshot_run(run, 2);
    match snap {
        Ok(vb_runtime::shard::InspectResponse::NotFound { .. }) => {
            // Expected — run completed and is no longer active
        }
        Ok(vb_runtime::shard::InspectResponse::Found(s)) => {
            return Err(format!(
                "expected run to be completed, still active at pc={:?}",
                s.pc
            ));
        }
        Ok(_) => {
            return Err("unexpected InspectResponse variant".to_owned());
        }
        Err(e) => {
            return Err(format!("snapshot_run failed: {e:?}"));
        }
    }

    let counters = runtime.counters_snapshot();
    if counters.runs_completed != 1 {
        return Err(format!(
            "expected 1 completed run, got {}",
            counters.runs_completed
        ));
    }
    Ok(())
}
