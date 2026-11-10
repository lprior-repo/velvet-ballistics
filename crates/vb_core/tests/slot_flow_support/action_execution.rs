use super::*;
use vb_core::action::{
    ActionContract, ActionFailure, ActionFailureCode, ActionName, ActionOutputReady, ActionTicket,
    Idempotency, RetryPolicy, RetrySafety, SideEffect, compute_action_idempotency_key,
};
use vb_core::engine::{resume_action_completion, resume_action_failure};
use vb_core::ids::{ActionId, RunId, SeqNo};
use vb_core::value::{SlotValue, Taint};
use vb_core::{EngineSignal, RunFrame, StepBudget, ValueStore, drive_deterministic};

#[test]
fn accepted_action_success_and_error_follow_distinct_runtime_edges() -> TestResult {
    let plan = CompiledWorkflow::try_from_parts(super::errors::action(2))?;
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
            assert_eq!(run.read_slot(slot(2))?, &SlotValue::I64(1));
            SlotValue::I64(1)
        };
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
