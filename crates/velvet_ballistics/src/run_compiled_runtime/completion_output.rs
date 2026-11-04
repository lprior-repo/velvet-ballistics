use core::num::NonZeroU16;
use std::fmt;
use vb_core::ids::RunId;
use vb_runtime::RuntimeResult;
use vb_runtime::runtime::completion_observation::{
    CompletionObservation, CompletionObservationSlot, CompletionValueObservation,
    ObservedTickOutcome, OwnedCompletionScalar,
};

pub(crate) fn completion_after_execution(
    observed: ObservedTickOutcome,
    slot: CompletionObservationSlot,
) -> RuntimeResult<Option<CompletionObservation>> {
    observed.execution?;
    if observed.observation.is_err() || slot.conflict {
        return Ok(None);
    }
    Ok(slot.first)
}

pub(crate) fn matching_completion(
    candidate: Option<CompletionObservation>,
    completed_successfully: bool,
    run: RunId,
    attempt: NonZeroU16,
) -> Option<CompletionObservation> {
    candidate.filter(|receipt| {
        completed_successfully && receipt.run == run && receipt.attempt == attempt
    })
}

pub(crate) fn completion_json(
    completion: Option<CompletionObservation>,
) -> Option<serde_json::Value> {
    completion.map(|receipt| match receipt.result {
        CompletionValueObservation::Available(scalar) => {
            let value = match scalar {
                OwnedCompletionScalar::Null => serde_json::Value::Null,
                OwnedCompletionScalar::Bool(value) => serde_json::json!(value),
                OwnedCompletionScalar::I64(value) => serde_json::json!(value),
                OwnedCompletionScalar::F64(value) => serde_json::json!(value.get()),
            };
            serde_json::json!({
                "run_id": receipt.run.get(),
                "attempt": receipt.attempt.get(),
                "state": "clean_scalar",
                "value": value
            })
        }
        CompletionValueObservation::Restricted => serde_json::json!({
            "run_id": receipt.run.get(),
            "attempt": receipt.attempt.get(),
            "state": "restricted"
        }),
        CompletionValueObservation::Unavailable => serde_json::json!({
            "run_id": receipt.run.get(),
            "attempt": receipt.attempt.get(),
            "state": "unavailable"
        }),
    })
}

pub(crate) struct CompletionText(CompletionValueObservation);

pub(crate) fn completion_text(completion: Option<CompletionObservation>) -> Option<CompletionText> {
    completion.map(|receipt| CompletionText(receipt.result))
}

impl fmt::Display for CompletionText {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            CompletionValueObservation::Available(OwnedCompletionScalar::Null) => {
                write!(formatter, "run result: null")
            }
            CompletionValueObservation::Available(OwnedCompletionScalar::Bool(value)) => {
                write!(formatter, "run result: {value}")
            }
            CompletionValueObservation::Available(OwnedCompletionScalar::I64(value)) => {
                write!(formatter, "run result: {value}")
            }
            CompletionValueObservation::Available(OwnedCompletionScalar::F64(value)) => {
                write!(formatter, "run result: {}", value.get())
            }
            CompletionValueObservation::Restricted => write!(formatter, "run result: restricted"),
            CompletionValueObservation::Unavailable => write!(formatter, "run result: unavailable"),
        }
    }
}
