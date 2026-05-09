#![forbid(unsafe_code)]
#![allow(unreachable_pub)]
//! Pure incident report computation logic, separated from I/O and formatting.

use vb_storage::events::JournalEvent;

/// Structured incident report for a single run.
pub struct IncidentReport {
    /// The run id string (as provided by the caller).
    pub run_id: String,
    /// Failure code, e.g. "RunFailed" or "RunCancelled". Empty if no failure.
    pub failure_code: String,
    /// Whether a failure event was found.
    pub failure_found: bool,
    /// Step at which the failure occurred, if known.
    pub failed_at_step: Option<u16>,
    /// Side effects collected from action completed/failed events.
    pub side_effects: Vec<serde_json::Value>,
    /// Repair hints based on failure type.
    pub repair_hints: Vec<serde_json::Value>,
}

/// Build an incident report from a run's event stream.
pub fn build_incident_report(run_id: &str, events: &[JournalEvent]) -> IncidentReport {
    let mut failure_found = false;
    let mut failure_code = String::new();
    let mut failed_at_step: Option<u16> = None;
    let mut side_effects: Vec<serde_json::Value> = Vec::new();
    let mut last_step_started: Option<u16> = None;

    for event in events {
        match event {
            JournalEvent::StepStarted { step, .. } => {
                last_step_started = Some(step.get());
            }
            JournalEvent::ActionCompletedEvent { step, action, .. } => {
                side_effects.push(serde_json::json!({
                    "step": step.get(),
                    "action": action.get(),
                    "certainty": "confirmed"
                }));
            }
            JournalEvent::ActionFailedEvent { step, action, .. } => {
                side_effects.push(serde_json::json!({
                    "step": step.get(),
                    "action": action.get(),
                    "certainty": "failed"
                }));
            }
            JournalEvent::RunFailedEvent { .. } => {
                failure_found = true;
                failure_code = "RunFailed".to_string();
                failed_at_step = last_step_started;
            }
            JournalEvent::RunCancelled { .. } => {
                failure_found = true;
                failure_code = "RunCancelled".to_string();
                failed_at_step = last_step_started;
            }
            _ => {}
        }
    }

    let repair_hints = build_repair_hints(&failure_code, &side_effects, failed_at_step);

    IncidentReport {
        run_id: run_id.to_string(),
        failure_code,
        failure_found,
        failed_at_step,
        side_effects,
        repair_hints,
    }
}

/// Build repair hints based on the failure code, side effects, and failed step.
pub fn build_repair_hints(
    failure_code: &str,
    side_effects: &[serde_json::Value],
    failed_at_step: Option<u16>,
) -> Vec<serde_json::Value> {
    let mut hints: Vec<serde_json::Value> = Vec::new();

    match failure_code {
        "RunFailed" => {
            hints.push(serde_json::Value::String(
                "investigate step output and engine logs for the failed step".to_string(),
            ));
            if !side_effects.is_empty() {
                hints.push(serde_json::Value::String(
                    "review side effects that completed before failure for compensating actions"
                        .to_string(),
                ));
            }
            if let Some(step) = failed_at_step {
                hints.push(serde_json::Value::String(format!(
                    "consider retry from step {step} using the retry command"
                )));
            }
        }
        "RunCancelled" => {
            hints.push(serde_json::Value::String(
                "run was cancelled; check if cancellation was intentional".to_string(),
            ));
            if !side_effects.is_empty() {
                hints.push(serde_json::Value::String(
                    "review completed side effects for partial cleanup needs".to_string(),
                ));
            }
        }
        _ => {}
    }

    hints
}
