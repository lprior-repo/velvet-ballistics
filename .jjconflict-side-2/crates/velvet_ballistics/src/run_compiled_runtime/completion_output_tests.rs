use super::completion_output::{
    completion_after_execution, completion_json, completion_text, matching_completion,
};
use core::num::NonZeroU16;
use serde_json::{Value, json};
use vb_core::ids::RunId;
use vb_core::value::FiniteF64;
use vb_runtime::RuntimeError;
use vb_runtime::runtime::completion_observation::{
    CompletionObservation, CompletionObservationSlot, CompletionValueObservation,
    CompletionValueObservation::{Available, Restricted, Unavailable},
    ObservationConflict, ObservedTickOutcome,
    OwnedCompletionScalar::{Bool, F64, I64, Null},
};

fn receipt(result: CompletionValueObservation) -> CompletionObservation {
    CompletionObservation {
        run: RunId::new(1),
        attempt: NonZeroU16::MIN,
        result,
    }
}

fn successful_tick(observation: Result<(), ObservationConflict>) -> ObservedTickOutcome {
    ObservedTickOutcome {
        execution: Ok(true),
        observation,
    }
}

fn assert_rendered(completion: Option<CompletionObservation>, expected: Value, text: &str) {
    assert_eq!(completion_json(completion), Some(expected));
    assert_eq!(
        completion_text(completion).map(|line| line.to_string()),
        Some(text.to_owned()),
    );
}

#[test]
fn matching_success_renders_literal_integer() -> Result<(), String> {
    let expected = receipt(Available(I64(99)));
    let slot = CompletionObservationSlot {
        first: Some(expected),
        conflict: false,
    };
    let candidate = completion_after_execution(successful_tick(Ok(())), slot)
        .map_err(|error| format!("{error:?}"))?;
    let selected = matching_completion(candidate, true, RunId::new(1), NonZeroU16::MIN);
    assert_eq!(selected, Some(expected));
    assert_rendered(
        selected,
        json!({
            "run_id": 1,
            "attempt": 1,
            "state": "clean_scalar",
            "value": 99
        }),
        "run result: 99",
    );
    Ok(())
}

#[test]
fn every_conflict_source_suppresses_present_receipt() -> Result<(), String> {
    let clear_slot = CompletionObservationSlot {
        first: Some(receipt(Available(I64(99)))),
        conflict: false,
    };
    let mut conflicted_slot = clear_slot;
    conflicted_slot.conflict = true;
    let global = completion_after_execution(
        successful_tick(Err(ObservationConflict::MultipleCompletions)),
        clear_slot,
    )
    .map_err(|error| format!("{error:?}"))?;
    assert_eq!(global, None);
    assert_eq!(json!(completion_json(global)), Value::Null);
    assert!(completion_text(global).is_none());
    let local = completion_after_execution(successful_tick(Ok(())), conflicted_slot)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(local, None);
    assert_eq!(completion_json(local), None);
    assert!(completion_text(local).is_none());
    let both = completion_after_execution(
        successful_tick(Err(ObservationConflict::MultipleCompletions)),
        conflicted_slot,
    )
    .map_err(|error| format!("{error:?}"))?;
    assert_eq!(both, None);
    assert_eq!(completion_json(both), None);
    assert!(completion_text(both).is_none());
    Ok(())
}

#[test]
fn identity_completion_and_absence_filters_reject_each_mismatch() -> Result<(), String> {
    let expected = receipt(Available(I64(99)));
    let attempt_two = NonZeroU16::new(2).ok_or("literal attempt must be nonzero")?;
    assert_eq!(
        matching_completion(Some(expected), true, RunId::new(1), NonZeroU16::MIN),
        Some(expected),
    );
    let mut wrong_run = expected;
    wrong_run.run = RunId::new(2);
    assert_eq!(
        matching_completion(Some(wrong_run), true, RunId::new(1), NonZeroU16::MIN),
        None,
    );
    let mut wrong_attempt = expected;
    wrong_attempt.attempt = attempt_two;
    assert_eq!(
        matching_completion(Some(wrong_attempt), true, RunId::new(1), NonZeroU16::MIN),
        None,
    );
    assert_eq!(
        matching_completion(Some(expected), false, RunId::new(1), NonZeroU16::MIN),
        None,
    );
    assert_eq!(
        matching_completion(None, true, RunId::new(1), NonZeroU16::MIN),
        None,
    );

    let result = completion_after_execution(
        successful_tick(Ok(())),
        CompletionObservationSlot::default(),
    )
    .map_err(|error| format!("{error:?}"))?;
    assert_eq!(result, None);
    assert_eq!(json!(completion_json(result)), Value::Null);
    assert!(completion_text(result).is_none());
    Ok(())
}

#[test]
fn restricted_renders_no_value() {
    let value = Some(receipt(Restricted));
    assert_rendered(
        value,
        json!({
            "run_id": 1,
            "attempt": 1,
            "state": "restricted"
        }),
        "run result: restricted",
    );
}

#[test]
fn unavailable_renders_no_value() {
    let value = Some(receipt(Unavailable));
    assert_rendered(
        value,
        json!({
            "run_id": 1,
            "attempt": 1,
            "state": "unavailable"
        }),
        "run result: unavailable",
    );
}

#[test]
fn clean_scalars_render_native_values_distinct_from_absence() -> Result<(), String> {
    let value = Some(receipt(Available(Null)));
    assert_rendered(
        value,
        json!({
            "run_id": 1,
            "attempt": 1,
            "state": "clean_scalar",
            "value": null
        }),
        "run result: null",
    );
    assert_eq!(completion_json(None), None);
    assert!(completion_text(None).is_none());

    let value = Some(receipt(Available(Bool(true))));
    assert_rendered(
        value,
        json!({
            "run_id": 1,
            "attempt": 1,
            "state": "clean_scalar",
            "value": true
        }),
        "run result: true",
    );

    let value = Some(receipt(Available(I64(-7))));
    assert_rendered(
        value,
        json!({
            "run_id": 1,
            "attempt": 1,
            "state": "clean_scalar",
            "value": -7
        }),
        "run result: -7",
    );

    let finite = FiniteF64::new(1.5).map_err(|error| format!("{error:?}"))?;
    let value = Some(receipt(Available(F64(finite))));
    assert_rendered(
        value,
        json!({
            "run_id": 1,
            "attempt": 1,
            "state": "clean_scalar",
            "value": 1.5
        }),
        "run result: 1.5",
    );
    Ok(())
}

#[test]
fn execution_error_wins_with_receipt_and_with_conflict() {
    let outcome = ObservedTickOutcome {
        execution: Err(RuntimeError::RunNotFound),
        observation: Ok(()),
    };
    let slot = CompletionObservationSlot {
        first: Some(receipt(Available(I64(99)))),
        conflict: false,
    };
    assert!(matches!(
        completion_after_execution(outcome, slot),
        Err(RuntimeError::RunNotFound)
    ));

    let outcome = ObservedTickOutcome {
        execution: Err(RuntimeError::RunNotFound),
        observation: Err(ObservationConflict::MultipleCompletions),
    };
    let slot = CompletionObservationSlot {
        first: Some(receipt(Available(I64(99)))),
        conflict: true,
    };
    assert!(matches!(
        completion_after_execution(outcome, slot),
        Err(RuntimeError::RunNotFound)
    ));
}
