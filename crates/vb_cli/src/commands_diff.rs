#![forbid(unsafe_code)]
#![allow(unreachable_pub)]
//! Pure diff computation logic, separated from I/O and formatting.
//!
//! Provides both legacy index-based diff (`compute_diff`) and the upgraded
//! normalized-semantic-observation diff (`compute_semantic_diff`). The
//! semantic diff reports stable deltas for reordered equivalent storage
//! details, action/timer/ask differences, slot changes, terminal changes,
//! and digest mismatches.

use std::collections::{HashMap, HashSet};

use vb_core::SlotValue;
use vb_storage::events::JournalEvent;
use vb_storage::{
    semantic_observation_signature, JournalObservation, JournalObservationSignature,
    ObservationSignatureError,
};

/// Result of comparing two event streams.
pub struct DiffResult {
    /// Number of events in stream A.
    pub events_a: usize,
    /// Number of events in stream B.
    pub events_b: usize,
    /// Ordered list of difference entries (as JSON values for downstream formatting).
    pub diffs: Vec<serde_json::Value>,
}

/// Compare two event streams and produce a structured diff using normalized
/// semantic observations.
///
/// This is the preferred diff function. It normalizes both event streams to
/// `JournalObservationSignature` and compares the resulting digests for fast
/// equality. When digests differ, it performs a detailed observation-level
/// comparison to produce semantic diff entries.
pub fn compute_diff(events_a: &[JournalEvent], events_b: &[JournalEvent]) -> DiffResult {
    compute_semantic_diff(events_a, events_b)
}

/// Compare two event streams using normalized semantic observation comparison.
///
/// Produces stable semantic deltas that ignore nondeterministic storage details
/// (e.g., timestamps, internal sequence numbers) while preserving meaningful
/// differences in:
/// - Step lifecycle and outcomes
/// - Slot values and writes
/// - Action scheduling/completion/failure/abandonment
/// - Wait/ask/timer state transitions
/// - Terminal states (finished, failed, cancelled, killed)
/// - Workflow and artifact digests
pub fn compute_semantic_diff(events_a: &[JournalEvent], events_b: &[JournalEvent]) -> DiffResult {
    let len_a = events_a.len();
    let len_b = events_b.len();

    // Fast path: both streams empty
    if events_a.is_empty() && events_b.is_empty() {
        return DiffResult {
            events_a: 0,
            events_b: 0,
            diffs: Vec::new(),
        };
    }

    // Normalize both streams to semantic observation signatures
    let sig_a = semantic_observation_signature(events_a);
    let sig_b = semantic_observation_signature(events_b);

    // Fast path: identical signatures
    match (&sig_a, &sig_b) {
        (Ok(sa), Ok(sb)) if sa.digest == sb.digest => {
            return DiffResult {
                events_a: len_a,
                events_b: len_b,
                diffs: Vec::new(),
            };
        }
        _ => {}
    }

    // Collect observations from both streams for detailed comparison
    let obs_a = match sig_a.map(|s| s.observations) {
        Ok(obs) => obs,
        Err(_) => events_to_observations(events_a),
    };
    let obs_b = match sig_b.map(|s| s.observations) {
        Ok(obs) => obs,
        Err(_) => events_to_observations(events_b),
    };

    let mut diffs: Vec<serde_json::Value> = Vec::new();

    // Compare digests
    match (&sig_a, &sig_b) {
        (Ok(sa), Ok(sb)) => {
            if sa.digest != sb.digest {
                let mut detail_a = String::new();
                for b in &sa.digest {
                    detail_a.push_str(&format!("{b:02x}"));
                }
                let mut detail_b = String::new();
                for b in &sb.digest {
                    detail_b.push_str(&format!("{b:02x}"));
                }
                diffs.push(serde_json::json!({
                    "kind": "digest_mismatch",
                    "signature_a": detail_a,
                    "signature_b": detail_b,
                    "schema_version_a": sa.schema_version,
                    "schema_version_b": sb.schema_version,
                    "observations_a": sa.observations.len(),
                    "observations_b": sb.observations.len(),
                }));
            }
        }
        (Err(ea), Err(_)) => {
            diffs.push(serde_json::json!({
                "kind": "normalization_error",
                "stream_a": format!("{ea:?}"),
                "stream_b": "normalization_failed",
            }));
        }
        (Err(ea), Ok(_)) => {
            diffs.push(serde_json::json!({
                "kind": "normalization_error",
                "stream_a": format!("{ea:?}"),
                "stream_b": "ok",
            }));
        }
        (Ok(_), Err(eb)) => {
            diffs.push(serde_json::json!({
                "kind": "normalization_error",
                "stream_a": "ok",
                "stream_b": format!("{eb:?}"),
            }));
        }
    }

    // Detailed observation-level comparison
    let obs_a_map = build_observation_index(&obs_a);
    let obs_b_map = build_observation_index(&obs_b);

    // Collect step outcome differences
    let steps_a = collect_semantic_step_outcomes(&obs_a);
    let steps_b = collect_semantic_step_outcomes(&obs_b);

    for (step, outcome) in &steps_a {
        match steps_b.get(step) {
            None => {
                diffs.push(serde_json::json!({
                    "kind": "step_missing_in_b",
                    "step": step,
                    "outcome_a": outcome,
                }));
            }
            Some(bo) => {
                if outcome != bo {
                    diffs.push(serde_json::json!({
                        "kind": "step_outcome_differs",
                        "step": step,
                        "outcome_a": outcome,
                        "outcome_b": bo,
                    }));
                }
            }
        }
    }
    for (step, outcome) in &steps_b {
        if !steps_a.contains_key(step) {
            diffs.push(serde_json::json!({
                "kind": "step_missing_in_a",
                "step": step,
                "outcome_b": outcome,
            }));
        }
    }

    // Collect slot value differences
    let slots_a = collect_semantic_slot_values(&obs_a);
    let slots_b = collect_semantic_slot_values(&obs_b);

    for (slot, va) in &slots_a {
        match slots_b.get(slot) {
            None => {
                diffs.push(serde_json::json!({
                    "kind": "slot_missing_in_b",
                    "slot": slot,
                    "value_a": va,
                }));
            }
            Some(vb) => {
                if va != vb {
                    diffs.push(serde_json::json!({
                        "kind": "slot_value_differs",
                        "slot": slot,
                        "value_a": va,
                        "value_b": vb,
                    }));
                }
            }
        }
    }
    for (slot, vb) in &slots_b {
        if !slots_a.contains_key(slot) {
            diffs.push(serde_json::json!({
                "kind": "slot_missing_in_a",
                "slot": slot,
                "value_b": vb,
            }));
        }
    }

    // Collect action differences
    let actions_a = collect_semantic_action_ids(&obs_a);
    let actions_b = collect_semantic_action_ids(&obs_b);

    for (action, step) in &actions_a {
        match actions_b.get(action) {
            None => {
                diffs.push(serde_json::json!({
                    "kind": "action_missing_in_b",
                    "action": action,
                    "step": step,
                }));
            }
            Some(&step_b) => {
                if step != &step_b {
                    diffs.push(serde_json::json!({
                        "kind": "action_step_differs",
                        "action": action,
                        "step_a": step,
                        "step_b": step_b,
                    }));
                }
            }
        }
    }
    for (action, step) in &actions_b {
        if !actions_a.contains_key(action) {
            diffs.push(serde_json::json!({
                "kind": "action_missing_in_a",
                "action": action,
                "step": step,
            }));
        }
    }

    // Collect timer differences
    let timers_a = collect_semantic_timers(&obs_a);
    let timers_b = collect_semantic_timers(&obs_b);

    for (key, step) in &timers_a {
        match timers_b.get(key) {
            None => {
                diffs.push(serde_json::json!({
                    "kind": "timer_missing_in_b",
                    "key": key,
                    "step": step,
                }));
            }
            Some(&step_b) => {
                if step != &step_b {
                    diffs.push(serde_json::json!({
                        "kind": "timer_step_differs",
                        "key": key,
                        "step_a": step,
                        "step_b": step_b,
                    }));
                }
            }
        }
    }
    for (key, step) in &timers_b {
        if !timers_a.contains_key(key) {
            diffs.push(serde_json::json!({
                "kind": "timer_missing_in_a",
                "key": key,
                "step": step,
            }));
        }
    }

    // Collect ask differences
    let asks_a = collect_semantic_asks(&obs_a);
    let asks_b = collect_semantic_asks(&obs_b);

    for (key, step) in &asks_a {
        match asks_b.get(key) {
            None => {
                diffs.push(serde_json::json!({
                    "kind": "ask_missing_in_b",
                    "key": key,
                    "step": step,
                }));
            }
            Some(&step_b) => {
                if step != &step_b {
                    diffs.push(serde_json::json!({
                        "kind": "ask_step_differs",
                        "key": key,
                        "step_a": step,
                        "step_b": step_b,
                    }));
                }
            }
        }
    }
    for (key, step) in &asks_b {
        if !asks_a.contains_key(key) {
            diffs.push(serde_json::json!({
                "kind": "ask_missing_in_a",
                "key": key,
                "step": step,
            }));
        }
    }

    // Collect terminal differences
    let terminals_a = collect_semantic_terminals(&obs_a);
    let terminals_b = collect_semantic_terminals(&obs_b);

    for (kind, detail) in &terminals_a {
        match terminals_b.get(kind) {
            None => {
                diffs.push(serde_json::json!({
                    "kind": "terminal_missing_in_b",
                    "terminal": kind,
                    "detail_a": detail,
                }));
            }
            Some(db) => {
                if detail != db {
                    diffs.push(serde_json::json!({
                        "kind": "terminal_differs",
                        "terminal": kind,
                        "detail_a": detail,
                        "detail_b": db,
                    }));
                }
            }
        }
    }
    for (kind, detail) in &terminals_b {
        if !terminals_a.contains_key(kind) {
            diffs.push(serde_json::json!({
                "kind": "terminal_missing_in_a",
                "terminal": kind,
                "detail_b": detail,
            }));
        }
    }

    DiffResult {
        events_a: len_a,
        events_b: len_b,
        diffs,
    }
}

/// Convert a journal event stream to a list of semantic observations (fallback path).
fn events_to_observations(events: &[JournalEvent]) -> Vec<JournalObservation> {
    match semantic_observation_signature(events) {
        Ok(sig) => sig.observations,
        Err(_) => Vec::new(),
    }
}

/// Build an index of observation key -> (step, kind) for fast comparison.
fn build_observation_index(observations: &[JournalObservation]) -> HashMap<String, (u16, String)> {
    let mut map = HashMap::new();
    for obs in observations {
        if let Some(key) = observation_key(obs) {
            map.insert(key, (0, String::new()));
        }
    }
    map
}

/// Return a unique key for an observation, suitable for building an index.
fn observation_key(obs: &JournalObservation) -> Option<String> {
    match obs {
        JournalObservation::Step(step_obs) => {
            let kind = match step_obs {
                vb_storage::StepObservation::Started { .. } => "step_started",
                vb_storage::StepObservation::Succeeded { .. } => "step_succeeded",
                vb_storage::StepObservation::Failed { .. } => "step_failed",
            };
            Some(format!("{kind}:{}", step_obs_step(obs)))
        }
        JournalObservation::Slot(slot_obs) => Some(format!(
            "slot:{}:attempt_{}",
            slot_obs.slot.get(),
            slot_obs.attempt
        )),
        JournalObservation::Action(action_obs) => {
            let kind = match action_obs {
                vb_storage::ActionObservation::Scheduled { .. } => "action_scheduled",
                vb_storage::ActionObservation::Completed { .. } => "action_completed",
                vb_storage::ActionObservation::Failed { .. } => "action_failed",
                vb_storage::ActionObservation::Abandoned { .. } => "action_abandoned",
            };
            Some(format!("{kind}:{}", action_obs_action_id(obs)))
        }
        JournalObservation::Timer(timer_obs) => {
            let kind = match timer_obs {
                vb_storage::TimerObservation::RetryScheduled { .. } => "timer_retry",
                vb_storage::TimerObservation::AskTimedOut { .. } => "timer_ask_timeout",
            };
            Some(format!("{kind}:{}", timer_obs_step(obs)))
        }
        JournalObservation::Wait(wait_obs) => {
            let kind = match wait_obs {
                vb_storage::WaitObservation::Scheduled { .. } => "wait_scheduled",
                vb_storage::WaitObservation::Resolved { .. } => "wait_resolved",
            };
            Some(format!("{kind}:{}", wait_obs_step(obs)))
        }
        JournalObservation::Ask(ask_obs) => {
            let kind = match ask_obs {
                vb_storage::AskObservation::Scheduled { .. } => "ask_scheduled",
                vb_storage::AskObservation::Answered { .. } => "ask_answered",
                vb_storage::AskObservation::AnswerRecorded { .. } => "ask_answer_recorded",
                vb_storage::AskObservation::TimedOut { .. } => "ask_timed_out",
            };
            Some(format!("{kind}:{}", ask_obs_step(obs)))
        }
        JournalObservation::Terminal(terminal_obs) => {
            let kind = match terminal_obs {
                vb_storage::TerminalObservation::Cancelled { .. } => "terminal_cancelled",
                vb_storage::TerminalObservation::Killed { .. } => "terminal_killed",
                vb_storage::TerminalObservation::Finished { .. } => "terminal_finished",
                vb_storage::TerminalObservation::Failed { .. } => "terminal_failed",
            };
            Some(kind.to_string())
        }
        JournalObservation::Lifecycle(_) => None,
        JournalObservation::Digest(_) => None,
    }
}

fn step_obs_step(obs: &vb_storage::StepObservation) -> u16 {
    match obs {
        vb_storage::StepObservation::Started { step, .. } => step.get(),
        vb_storage::StepObservation::Succeeded { step, .. } => step.get(),
        vb_storage::StepObservation::Failed { step, .. } => step.get(),
    }
}

fn action_obs_step(obs: &vb_storage::ActionObservation) -> u16 {
    match obs {
        vb_storage::ActionObservation::Scheduled { step, .. } => step.get(),
        vb_storage::ActionObservation::Completed { step, .. } => step.get(),
        vb_storage::ActionObservation::Failed { step, .. } => step.get(),
        vb_storage::ActionObservation::Abandoned { step, .. } => step.get(),
    }
}

fn action_obs_action_id(obs: &vb_storage::ActionObservation) -> u16 {
    match obs {
        vb_storage::ActionObservation::Scheduled { action, .. } => action.get(),
        vb_storage::ActionObservation::Completed { action, .. } => action.get(),
        vb_storage::ActionObservation::Failed { action, .. } => action.get(),
        vb_storage::ActionObservation::Abandoned { action, .. } => action.get(),
    }
}

fn timer_obs_step(obs: &vb_storage::TimerObservation) -> u16 {
    match obs {
        vb_storage::TimerObservation::RetryScheduled { step, .. } => step.get(),
        vb_storage::TimerObservation::AskTimedOut { step, .. } => step.get(),
    }
}

fn wait_obs_step(obs: &vb_storage::WaitObservation) -> u16 {
    match obs {
        vb_storage::WaitObservation::Scheduled { step, .. } => step.get(),
        vb_storage::WaitObservation::Resolved { step, .. } => step.get(),
    }
}

fn ask_obs_step(obs: &vb_storage::AskObservation) -> u16 {
    match obs {
        vb_storage::AskObservation::Scheduled { step, .. } => step.get(),
        vb_storage::AskObservation::Answered { step, .. } => step.get(),
        vb_storage::AskObservation::AnswerRecorded { .. } => 0,
        vb_storage::AskObservation::TimedOut { step, .. } => step.get(),
    }
}

// =============================================================================
// Semantic collection helpers
// =============================================================================

/// Collect the final outcome per step from semantic observations.
fn collect_semantic_step_outcomes(observations: &[JournalObservation]) -> HashMap<u16, String> {
    let mut outcomes = HashMap::new();
    for obs in observations {
        if let JournalObservation::Step(step_obs) = obs {
            let (step, outcome) = match step_obs {
                vb_storage::StepObservation::Started { step, attempt } => {
                    (step.get(), format!("started(attempt={})", attempt))
                }
                vb_storage::StepObservation::Succeeded { step, output } => {
                    (step.get(), format!("succeeded(output={})", output.get()))
                }
                vb_storage::StepObservation::Failed { step, attempt } => {
                    (step.get(), format!("failed(attempt={})", attempt))
                }
            };
            outcomes.insert(step, outcome);
        }
    }
    outcomes
}

/// Collect the final display value per slot from semantic observations.
fn collect_semantic_slot_values(observations: &[JournalObservation]) -> HashMap<u16, String> {
    let mut slots = HashMap::new();
    for obs in observations {
        if let JournalObservation::Slot(slot_obs) = obs {
            let display = match slot_obs.value_digest {
                Some(digest) => {
                    let mut hex = String::with_capacity(16);
                    for b in &digest[..4] {
                        hex.push_str(&format!("{b:02x}"));
                    }
                    format!("[{} bytes, {}]", slot_obs.attempt, hex);
                }
                None => format!("none(attempt={})", slot_obs.attempt),
            };
            slots.insert(slot_obs.slot.get(), display);
        }
    }
    slots
}

/// Collect action IDs -> step from semantic observations.
fn collect_semantic_action_ids(observations: &[JournalObservation]) -> HashMap<u16, u16> {
    let mut actions = HashMap::new();
    for obs in observations {
        if let JournalObservation::Action(action_obs) = obs {
            actions.insert(action_obs_action_id(obs), action_obs_step(obs));
        }
    }
    actions
}

/// Collect timer keys -> step from semantic observations.
fn collect_semantic_timers(observations: &[JournalObservation]) -> HashMap<String, u16> {
    let mut timers = HashMap::new();
    for obs in observations {
        if let JournalObservation::Timer(timer_obs) = obs {
            let key = match timer_obs {
                vb_storage::TimerObservation::RetryScheduled { .. } => "retry".to_string(),
                vb_storage::TimerObservation::AskTimedOut { .. } => "ask_timeout".to_string(),
            };
            timers.insert(key, timer_obs_step(obs));
        }
    }
    timers
}

/// Collect ask keys -> step from semantic observations.
fn collect_semantic_asks(observations: &[JournalObservation]) -> HashMap<String, u16> {
    let mut asks = HashMap::new();
    for obs in observations {
        if let JournalObservation::Ask(ask_obs) = obs {
            let key = match ask_obs {
                vb_storage::AskObservation::Scheduled { .. } => "ask_scheduled".to_string(),
                vb_storage::AskObservation::Answered { .. } => "ask_answered".to_string(),
                vb_storage::AskObservation::AnswerRecorded { .. } => {
                    "ask_answer_recorded".to_string()
                }
                vb_storage::AskObservation::TimedOut { .. } => "ask_timed_out".to_string(),
            };
            asks.insert(key, ask_obs_step(obs));
        }
    }
    asks
}

/// Collect terminal state from semantic observations.
fn collect_semantic_terminals(observations: &[JournalObservation]) -> HashMap<String, String> {
    let mut terminals = HashMap::new();
    for obs in observations {
        if let JournalObservation::Terminal(terminal_obs) = obs {
            let (kind, detail) = match terminal_obs {
                vb_storage::TerminalObservation::Cancelled {
                    attempt,
                    reason_digest,
                } => (
                    "cancelled".to_string(),
                    format!(
                        "cancelled(attempt={}, reason={:?})",
                        attempt,
                        reason_digest.as_ref().map(|d| {
                            let mut s = String::with_capacity(8);
                            for b in &d[..4] {
                                s.push_str(&format!("{b:02x}"));
                            }
                            s
                        })
                    ),
                ),
                vb_storage::TerminalObservation::Killed { attempt } => {
                    ("killed".to_string(), format!("killed(attempt={})", attempt))
                }
                vb_storage::TerminalObservation::Finished { result, attempt } => (
                    "finished".to_string(),
                    format!("finished(result={}, attempt={})", result.get(), attempt),
                ),
                vb_storage::TerminalObservation::Failed { attempt } => {
                    ("failed".to_string(), format!("failed(attempt={})", attempt))
                }
            };
            terminals.insert(kind, detail);
        }
    }
    terminals
}

// =============================================================================
// Legacy event-name and comparison helpers (preserved for backward compatibility)
// =============================================================================

/// Produce a short JSON summary of a single event for diff display.
pub fn diff_event_summary(event: &JournalEvent) -> serde_json::Value {
    match event {
        JournalEvent::RunAccepted { seq, .. } => {
            serde_json::json!({"type": "RunAccepted", "seq": seq.get()})
        }
        JournalEvent::RunAdmission { seq, policy, .. } => {
            serde_json::json!({"type": "RunAdmission", "seq": seq.get(), "policy": format!("{policy:?}")})
        }
        JournalEvent::StepStarted { seq, step, .. } => {
            serde_json::json!({"type": "StepStarted", "seq": seq.get(), "step": step.get()})
        }
        JournalEvent::StepSucceeded {
            seq, step, output, ..
        } => serde_json::json!({
            "type": "StepSucceeded",
            "seq": seq.get(),
            "step": step.get(),
            "output": output.get()
        }),
        JournalEvent::ActionScheduled {
            seq, step, action, ..
        } => serde_json::json!({
            "type": "ActionScheduled",
            "seq": seq.get(),
            "step": step.get(),
            "action": action.get()
        }),
        JournalEvent::ActionCompletedEvent {
            seq, step, action, ..
        } => serde_json::json!({
            "type": "ActionCompleted",
            "seq": seq.get(),
            "step": step.get(),
            "action": action.get()
        }),
        JournalEvent::ActionFailedEvent {
            seq, step, action, ..
        } => serde_json::json!({
            "type": "ActionFailed",
            "seq": seq.get(),
            "step": step.get(),
            "action": action.get()
        }),
        JournalEvent::SlotWrittenEvent {
            seq, slot, value, ..
        } => serde_json::json!({
            "type": "SlotWritten",
            "seq": seq.get(),
            "slot": slot.get(),
            "has_value": value.is_some()
        }),
        JournalEvent::WaitScheduledEvent { seq, step, .. } => {
            serde_json::json!({"type": "WaitScheduled", "seq": seq.get(), "step": step.get()})
        }
        JournalEvent::AskScheduledEvent { seq, step, .. } => {
            serde_json::json!({"type": "AskScheduled", "seq": seq.get(), "step": step.get()})
        }
        JournalEvent::AskAnsweredEvent { seq, step, .. } => {
            serde_json::json!({"type": "AskAnswered", "seq": seq.get(), "step": step.get()})
        }
        JournalEvent::RetryScheduledEvent { seq, step, .. } => {
            serde_json::json!({"type": "RetryScheduled", "seq": seq.get(), "step": step.get()})
        }
        JournalEvent::RunCancelled { seq, .. } => {
            serde_json::json!({"type": "RunCancelled", "seq": seq.get()})
        }
        JournalEvent::RunFinished { seq, result, .. } => {
            serde_json::json!({"type": "RunFinished", "seq": seq.get(), "result": result.get()})
        }
        JournalEvent::RunFailedEvent { seq, .. } => {
            serde_json::json!({"type": "RunFailed", "seq": seq.get()})
        }
        JournalEvent::RunResumed { run, .. } => {
            serde_json::json!({"type": "RunResumed", "run": run.get()})
        }
        JournalEvent::RunRetried { run, .. } => {
            serde_json::json!({"type": "RunRetried", "run": run.get()})
        }
        JournalEvent::RunAnswered { run, slot_idx, .. } => {
            serde_json::json!({"type": "RunAnswered", "run": run.get(), "slot_idx": slot_idx.get()})
        }
        _ => serde_json::json!({"type": "Unknown"}),
    }
}

/// Return the static name string for an event variant.
pub fn event_name(event: &JournalEvent) -> &'static str {
    match event {
        JournalEvent::RunAccepted { .. } => "RunAccepted",
        JournalEvent::RunAdmission { .. } => "RunAdmission",
        JournalEvent::StepStarted { .. } => "StepStarted",
        JournalEvent::StepSucceeded { .. } => "StepSucceeded",
        JournalEvent::ActionScheduled { .. } => "ActionScheduled",
        JournalEvent::ActionCompletedEvent { .. } => "ActionCompleted",
        JournalEvent::ActionFailedEvent { .. } => "ActionFailed",
        JournalEvent::SlotWrittenEvent { .. } => "SlotWritten",
        JournalEvent::WaitScheduledEvent { .. } => "WaitScheduled",
        JournalEvent::AskScheduledEvent { .. } => "AskScheduled",
        JournalEvent::AskAnsweredEvent { .. } => "AskAnswered",
        JournalEvent::RetryScheduledEvent { .. } => "RetryScheduled",
        JournalEvent::RunCancelled { .. } => "RunCancelled",
        JournalEvent::RunFinished { .. } => "RunFinished",
        JournalEvent::RunFailedEvent { .. } => "RunFailed",
        JournalEvent::RunResumed { .. } => "RunResumed",
        JournalEvent::RunRetried { .. } => "RunRetried",
        JournalEvent::RunAnswered { .. } => "RunAnswered",
        _ => "Unknown",
    }
}

/// Check whether two events differ in a semantically meaningful way.
pub fn events_differ(a: &JournalEvent, b: &JournalEvent) -> bool {
    match (a, b) {
        (
            JournalEvent::StepSucceeded {
                step: sa,
                output: oa,
                ..
            },
            JournalEvent::StepSucceeded {
                step: sb,
                output: ob,
                ..
            },
        ) => sa != sb || oa != ob,
        (
            JournalEvent::StepStarted { step: sa, .. },
            JournalEvent::StepStarted { step: sb, .. },
        ) => sa != sb,
        (
            JournalEvent::ActionScheduled {
                step: sa,
                action: aa,
                ..
            },
            JournalEvent::ActionScheduled {
                step: sb,
                action: ab,
                ..
            },
        ) => sa != sb || aa != ab,
        (
            JournalEvent::ActionCompletedEvent {
                step: sa,
                action: aa,
                ..
            },
            JournalEvent::ActionCompletedEvent {
                step: sb,
                action: ab,
                ..
            },
        ) => sa != sb || aa != ab,
        (
            JournalEvent::ActionFailedEvent {
                step: sa,
                action: aa,
                ..
            },
            JournalEvent::ActionFailedEvent {
                step: sb,
                action: ab,
                ..
            },
        ) => sa != sb || aa != ab,
        (
            JournalEvent::SlotWrittenEvent {
                slot: sa,
                value: va,
                ..
            },
            JournalEvent::SlotWrittenEvent {
                slot: sb,
                value: vb,
                ..
            },
        ) => sa != sb || va != vb,
        (
            JournalEvent::RunFinished { result: ra, .. },
            JournalEvent::RunFinished { result: rb, .. },
        ) => ra != rb,
        _ => event_name(a) != event_name(b),
    }
}

/// Collect the final outcome per step from an event stream.
pub fn collect_step_outcomes(events: &[JournalEvent]) -> HashMap<u16, String> {
    let mut outcomes = HashMap::new();
    for event in events {
        match event {
            JournalEvent::StepSucceeded { step, output, .. } => {
                outcomes.insert(step.get(), format!("succeeded(output={})", output.get()));
            }
            JournalEvent::ActionFailedEvent { step, action, .. } => {
                outcomes.insert(step.get(), format!("failed(action={})", action.get()));
            }
            JournalEvent::ActionCompletedEvent { step, action, .. } => {
                outcomes.insert(
                    step.get(),
                    format!("action_completed(action={})", action.get()),
                );
            }
            _ => {}
        }
    }
    outcomes
}

/// Collect the final display value per slot from an event stream.
pub fn collect_slot_values(events: &[JournalEvent]) -> HashMap<u16, String> {
    let mut slots = HashMap::new();
    for event in events {
        if let JournalEvent::SlotWrittenEvent { slot, value, .. } = event {
            let display = match value {
                Some(bytes) => match postcard::from_bytes::<SlotValue>(bytes) {
                    Ok(v) => format!("{v}"),
                    Err(_) => format!("[{} bytes]", bytes.len()),
                },
                None => String::from("none"),
            };
            slots.insert(slot.get(), display);
        }
    }
    slots
}

#[cfg(test)]
#[path = "commands_diff/tests.rs"]
mod tests;
