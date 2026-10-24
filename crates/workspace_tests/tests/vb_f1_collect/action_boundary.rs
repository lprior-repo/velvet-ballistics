#![forbid(unsafe_code)]

use super::support::SourceRun;
use vb_core::{SlotValue, Taint};

fn action_contract(id: vb_core::ActionId) -> Result<vb_core::action::ActionContract, String> {
    use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
    Ok(ActionContract {
        id,
        name: ActionName::new("lexical-action").map_err(|e| format!("action name: {e:?}"))?,
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
}

#[test]
fn source_set_do_set_reaches_real_action_boundary() -> Result<(), String> {
    use vb_core::{CapabilitySet, RunId, StepBudget};
    use vb_runtime::engine::{RetryPolicy, RuntimeSignal, drive_deterministic_full};

    let source = r#"version: velvet-ballistics/v1
name: collect_set_do_set
when:
  manual: {}
inputs:
  entries: { is: list, of: any, max: 8 }
steps:
  - id: gather
    collect:
      variable: batch
      source: "$input.entries"
      pages: 3
      items: 2
      steps:
        - id: set_copied
          set:
            output: copied
            value: "$batch"
        - id: invoke
          do:
            action: "0"
            input: "$copied"
        - id: set_echoed
          set:
            output: echoed
            value: "$step.invoke"
  - id: done
    finish:
      result: "$step.gather"
"#;

    // Source compilation already proves the Set->Do->Set reference chain
    // lowers to a real action boundary. Do not pretend completion or resume.

    let mut run = SourceRun::new(
        source,
        &[SlotValue::I64(11), SlotValue::I64(22), SlotValue::I64(33)],
        Taint::Clean,
    )?;
    let before = run.workflow.to_parts();

    // Use real ValueStore, inputList[11,22,33], and actual contracts from
    // the runtime's action contract system — not hand-built IR or mutations.
    let contract0 = action_contract(vb_core::ActionId::new(0))?;
    let contracts = vec![contract0];

    let signal = drive_deterministic_full(
        &run.workflow,
        &mut run.frame,
        &mut StepBudget::new(128),
        &mut run.store,
        &contracts,
        RetryPolicy::NEVER,
        &mut run.evidence,
        &mut run.collect_states,
        &CapabilitySet::empty(),
        1,
    )
    .map_err(|e| format!("{e:?}"))?;

    // Assert RuntimeSignal::AwaitingAction(ticket) with expected ticket fields
    let RuntimeSignal::AwaitingAction(ticket) = signal else {
        return Err(format!("expected AwaitingAction signal, got {signal:?}"));
    };
    if ticket.run != RunId::new(1) {
        return Err(format!(
            "ticket.run mismatch: expected RunId(1), got {ticket:?}"
        ));
    }
    if ticket.step != vb_core::StepIdx::new(2) {
        return Err(format!(
            "ticket.step mismatch: expected StepIdx(2), got {ticket:?}"
        ));
    }
    if ticket.action != vb_core::ActionId::new(0) {
        return Err(format!(
            "ticket.action mismatch: expected ActionId(0), got {ticket:?}"
        ));
    }

    // Assert the Do input slot equals run.output(1)? (set-copied's output),
    // and read the actual page values from the store.
    let do_input_slot = run.output(1)?;
    let node = run
        .workflow
        .node(vb_core::StepIdx::new(2))
        .ok_or("step 2 missing")?;
    match &node.kind {
        vb_core::CompiledNodeKind::Do { input, .. } => {
            assert_eq!(*input, do_input_slot);
        }
        other => return Err(format!("step 2 not Do, got {other:?}")),
    }
    let do_input_value = run
        .frame
        .read_slot(do_input_slot)
        .map_err(|e| format!("{e:?}"))?;
    match do_input_value {
        SlotValue::List(id) => {
            let list = run.store.list(*id).map_err(|e| format!("{e:?}"))?;
            if list != [SlotValue::I64(11), SlotValue::I64(22)] {
                return Err(format!(
                    "Do input values mismatch: expected [11,22], got {list:?}"
                ));
            }
        }
        other => {
            return Err(format!(
                "Do input slot expected SlotValue::List, got {other:?}"
            ));
        }
    }

    // Source input unchanged; workflow parts unchanged; step 3 (set-echoed)
    // is past the action boundary and must remain uninitialized.
    run.assert_input(
        &[SlotValue::I64(11), SlotValue::I64(22), SlotValue::I64(33)],
        Taint::Clean,
    )?;
    if run.workflow.to_parts() != before {
        return Err("workflow parts changed unexpectedly".to_string());
    }

    let tail = run.output(3)?;
    assert_eq!(
        run.frame.read_slot(tail),
        Err(vb_core::CoreError::SlotUninitialized { slot: tail })
    );
    // Full completion remains blocked by public Runtime owned-List admission (vb-8eocj.6.1).

    Ok(())
}
