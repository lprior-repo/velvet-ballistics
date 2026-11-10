use super::*;
use vb_core::action::{
    ActionContract, ActionFailure, ActionFailureCode, ActionName, ActionOutputReady, ActionTicket,
    Idempotency, RetryPolicy, RetrySafety, SideEffect, compute_action_idempotency_key,
};
use vb_core::engine::{resume_action_completion, resume_action_failure};
use vb_core::ids::{ActionId, RunId, SeqNo, SlotIdx};
use vb_core::value::{SlotValue, Taint};
use vb_core::{EngineSignal, RunFrame, StepBudget, ValueStore, drive_deterministic};

#[test]
fn accepted_action_success_and_error_follow_distinct_runtime_edges() -> TestResult {
    execute_action_edges(super::errors::action(2), slot(2), None)
}

#[test]
fn legacy_wrapper_action_failure_preserves_direct_error_slot_precedence() -> TestResult {
    execute_action_edges(
        super::errors::action_with_legacy_wrapper(2),
        slot(2),
        Some(slot(4)),
    )
}

#[test]
fn legacy_wrapper_action_failure_falls_back_only_without_direct_slot() -> TestResult {
    let mut parts = super::errors::action_with_legacy_wrapper(4);
    parts.nodes.get_mut(1).ok_or("action")?.routing.error =
        vb_core::workflow::ErrorRouting::Handler {
            handler: step(3),
            error_slot: None,
        };
    execute_action_edges(parts, slot(4), Some(slot(2)))
}

fn execute_action_edges(
    parts: WorkflowParts,
    error_slot: SlotIdx,
    untouched_slot: Option<SlotIdx>,
) -> TestResult {
    let plan = CompiledWorkflow::try_from_parts(parts)?;
    let contract = ActionContract {
        id: ActionId::new(0),
        name: ActionName::new("slot-flow")?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 128,
        max_output_bytes: 128,
        timeout_ms: 1000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    };
    for succeeds in [false, true] {
        let mut run = RunFrame::new(
            RunId::new(1),
            plan.entry(),
            plan.node_count(),
            plan.slot_count(),
        )?;
        let mut store = ValueStore::new();
        assert_eq!(
            drive_deterministic(&plan, &mut run, &mut StepBudget::new(8), &mut store)?,
            EngineSignal::AwaitingAction
        );
        let Err(error) = run.read_slot(slot(1)) else {
            return Err("success output was initialized before completion".into());
        };
        assert_eq!(
            error,
            vb_core::CoreError::SlotUninitialized { slot: slot(1) }
        );
        let ticket = ActionTicket {
            run: run.run_id(),
            step: step(1),
            seq: SeqNo::new(1),
            action: contract.id,
            run_attempt: 1,
            attempt: 1,
            idempotency_key: compute_action_idempotency_key(
                run.run_id(),
                1,
                SeqNo::new(1),
                contract.id,
            ),
            capacity: 1,
            fencing_token: 0,
        };
        let expected = if succeeds {
            let (signal, _) = resume_action_completion(
                &plan,
                &mut run,
                ticket,
                ActionOutputReady {
                    output_slot: slot(1),
                    value: SlotValue::I64(7),
                    taint: Taint::Clean,
                    encoded_len: 0,
                },
                &[],
                &contract,
            )?;
            assert_eq!(signal, EngineSignal::Continue);
            assert_eq!(run.pc(), step(2));
            SlotValue::I64(7)
        } else {
            let (signal, _) = resume_action_failure(
                &plan,
                &mut run,
                ticket,
                ActionFailure {
                    code: ActionFailureCode::Timeout,
                    retry_policy: RetryPolicy::NonRetryable,
                    taint: Taint::Clean,
                    detail: None,
                    encoded_len: 0,
                },
                &[],
                &contract,
            )?;
            assert_eq!(signal, EngineSignal::Continue);
            assert_eq!(run.pc(), step(3));
            let Err(error) = run.read_slot(slot(1)) else {
                return Err("success output was initialized before completion".into());
            };
            assert_eq!(
                error,
                vb_core::CoreError::SlotUninitialized { slot: slot(1) }
            );
            assert_eq!(run.read_slot(error_slot)?, &SlotValue::I64(1));
            assert_eq!(run.read_taint(error_slot)?, Taint::Clean);
            assert_eq!(run.step_state(step(1))?, vb_core::StepState::Failed);
            SlotValue::I64(1)
        };
        if let Some(slot) = untouched_slot {
            assert_eq!(
                run.read_slot(slot),
                Err(vb_core::CoreError::SlotUninitialized { slot })
            );
        }
        let EngineSignal::Finished(value, taint) =
            drive_deterministic(&plan, &mut run, &mut StepBudget::new(8), &mut store)?
        else {
            return Err("action continuation did not finish".into());
        };
        assert_eq!(value, expected);
        assert_eq!(taint, Taint::Clean);
    }
    Ok(())
}
