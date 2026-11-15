use vb_core::action::{
    ActionContract, ActionFailure, ActionFailureCode, ActionName, ActionOutcome, ActionTicket,
    Idempotency, RetryPolicy, RetrySafety, SideEffect,
};
use vb_core::engine::EngineError;
use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};
use vb_core::value::Taint;
use vb_runtime::engine::action::resume_action_outcome;
use vb_runtime::engine::types::{RuntimeEngineError, RuntimeSignal};

fn contract(side_effect: SideEffect, retry_safety: RetrySafety) -> Result<ActionContract, String> {
    Ok(ActionContract {
        id: ActionId::new(5),
        name: ActionName::new("resume-contract").map_err(|error| format!("{error:?}"))?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect,
        retry_safety,
        required_capabilities: Box::new([]),
    })
}

fn ticket(capacity: u16) -> ActionTicket {
    ActionTicket {
        run: RunId::new(10),
        step: StepIdx::new(7),
        seq: SeqNo::new(11),
        action: ActionId::new(5),
        run_attempt: 1,
        attempt: 1,
        idempotency_key: 123,
        capacity,
        fencing_token: 17,
    }
}

fn failure(retry_policy: RetryPolicy) -> ActionOutcome {
    ActionOutcome::Failed(ActionFailure {
        code: ActionFailureCode::Timeout,
        retry_policy,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    })
}

fn refusal() -> RuntimeEngineError {
    RuntimeEngineError::Core(EngineError::UnsupportedPrimitive {
        primitive: "action_failed_non_retryable".into(),
    })
}

#[test]
fn resume_refuses_retryable_unsafe_effects_before_issuing_another_ticket() -> Result<(), String> {
    for side_effect in [
        SideEffect::Writes,
        SideEffect::Sends,
        SideEffect::Creates,
        SideEffect::Destroys,
    ] {
        for capacity in [3, 1] {
            let contract = contract(side_effect, RetrySafety::Unsafe)?;
            assert_eq!(
                resume_action_outcome(
                    &ticket(capacity),
                    failure(RetryPolicy::Retryable),
                    &contract
                ),
                Err(refusal()),
                "{side_effect:?}, capacity={capacity}"
            );
        }
    }
    Ok(())
}

#[test]
fn resume_preserves_permitted_retry_and_pure_bypass_outcomes() -> Result<(), String> {
    for (side_effect, retry_safety) in [
        (SideEffect::None, RetrySafety::Safe),
        (SideEffect::None, RetrySafety::KeyRequired),
        (SideEffect::None, RetrySafety::Unsafe),
        (SideEffect::Writes, RetrySafety::Safe),
        (SideEffect::Writes, RetrySafety::KeyRequired),
        (SideEffect::Sends, RetrySafety::Safe),
        (SideEffect::Sends, RetrySafety::KeyRequired),
        (SideEffect::Creates, RetrySafety::Safe),
        (SideEffect::Creates, RetrySafety::KeyRequired),
        (SideEffect::Destroys, RetrySafety::Safe),
        (SideEffect::Destroys, RetrySafety::KeyRequired),
    ] {
        let contract = contract(side_effect, retry_safety)?;
        let result = resume_action_outcome(&ticket(3), failure(RetryPolicy::Retryable), &contract);
        match result {
            Ok(RuntimeSignal::AwaitingAction(retry)) => {
                assert_eq!(retry.run, RunId::new(10));
                assert_eq!(retry.step, StepIdx::new(7));
                assert_eq!(retry.action, ActionId::new(5));
                assert_eq!(retry.seq, SeqNo::new(12));
                assert_eq!(retry.run_attempt, 1);
                assert_eq!(retry.attempt, 2);
                assert_eq!(retry.capacity, 3);
                assert_eq!(retry.fencing_token, 0);
            }
            other => return Err(format!("{side_effect:?}/{retry_safety:?}: {other:?}")),
        }
        assert_eq!(
            resume_action_outcome(&ticket(1), failure(RetryPolicy::Retryable), &contract),
            Err(RuntimeEngineError::RetryExhausted {
                action: ActionId::new(5),
                attempts: 1,
            })
        );
    }
    Ok(())
}

#[test]
fn resume_refuses_nonretryable_failure_for_every_current_contract_class() -> Result<(), String> {
    for side_effect in [
        SideEffect::None,
        SideEffect::Writes,
        SideEffect::Sends,
        SideEffect::Creates,
        SideEffect::Destroys,
    ] {
        for retry_safety in [
            RetrySafety::Safe,
            RetrySafety::KeyRequired,
            RetrySafety::Unsafe,
        ] {
            let contract = contract(side_effect, retry_safety)?;
            assert_eq!(
                resume_action_outcome(&ticket(3), failure(RetryPolicy::NonRetryable), &contract),
                Err(refusal()),
                "{side_effect:?}/{retry_safety:?}"
            );
        }
    }
    Ok(())
}

#[test]
fn resume_reports_sequence_overflow_for_permitted_retry_below_capacity() -> Result<(), String> {
    let original = ActionTicket {
        seq: SeqNo::new(u64::MAX),
        ..ticket(3)
    };
    for (side_effect, retry_safety) in [
        (SideEffect::Writes, RetrySafety::Safe),
        (SideEffect::Writes, RetrySafety::KeyRequired),
        (SideEffect::None, RetrySafety::Unsafe),
    ] {
        let contract = contract(side_effect, retry_safety)?;
        assert_eq!(
            resume_action_outcome(&original, failure(RetryPolicy::Retryable), &contract),
            Err(RuntimeEngineError::Core(
                EngineError::InternalInvariantViolation {
                    reason: "seq_overflow_on_retry".into(),
                }
            )),
            "{side_effect:?}/{retry_safety:?}"
        );
    }
    Ok(())
}

#[test]
fn resume_refuses_unsafe_effects_before_sequence_overflow() -> Result<(), String> {
    let contract = contract(SideEffect::Writes, RetrySafety::Unsafe)?;
    for capacity in [3, 1] {
        let original = ActionTicket {
            seq: SeqNo::new(u64::MAX),
            ..ticket(capacity)
        };
        assert_eq!(
            resume_action_outcome(&original, failure(RetryPolicy::Retryable), &contract),
            Err(refusal()),
            "capacity={capacity}"
        );
    }
    Ok(())
}

#[test]
fn resume_reports_exhaustion_before_sequence_overflow_for_permitted_retry() -> Result<(), String> {
    let original = ActionTicket {
        seq: SeqNo::new(u64::MAX),
        ..ticket(1)
    };
    for retry_safety in [RetrySafety::Safe, RetrySafety::KeyRequired] {
        let contract = contract(SideEffect::Writes, retry_safety)?;
        assert_eq!(
            resume_action_outcome(&original, failure(RetryPolicy::Retryable), &contract),
            Err(RuntimeEngineError::RetryExhausted {
                action: ActionId::new(5),
                attempts: 1,
            }),
            "{retry_safety:?}"
        );
    }
    Ok(())
}
