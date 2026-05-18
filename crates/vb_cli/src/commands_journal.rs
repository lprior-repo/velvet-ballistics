#![forbid(unsafe_code)]
//! Pure journal analysis logic for trace, retry, resume, answer commands.
//!
//! All functions in this module are pure: they accept `&[JournalEvent]` and
//! return structured data. No I/O, no formatting, no side effects.

use vb_storage::JournalEvent;

// ---------------------------------------------------------------------------
// Trace
// ---------------------------------------------------------------------------

/// Structured trace entry produced by [`build_trace`].
#[derive(Debug, Clone)]
pub(crate) struct TraceEntry {
    pub index: usize,
    pub event_type: &'static str,
    pub step: Option<u16>,
    pub seq: u64,
    /// Extra key-value pairs for JSON output (variant-specific fields).
    pub extra_json: Vec<(&'static str, serde_json::Value)>,
}

/// Scan a slice of journal events and return one structured entry per event.
pub(crate) fn build_trace(events: &[JournalEvent]) -> Vec<TraceEntry> {
    events
        .iter()
        .enumerate()
        .map(|(idx, event)| trace_one(idx, event))
        .collect()
}

fn trace_one(idx: usize, event: &JournalEvent) -> TraceEntry {
    match event {
        JournalEvent::RunAccepted { seq, run, workflow } => TraceEntry {
            index: idx,
            event_type: "RunAccepted",
            step: None,
            seq: seq.get(),
            extra_json: vec![
                ("run", serde_json::Value::from(run.get())),
                ("workflow", serde_json::Value::from(format!("{workflow:?}"))),
            ],
        },
        JournalEvent::RunAdmission {
            seq,
            artifact_digest,
            granted_capabilities,
            policy,
            ..
        } => TraceEntry {
            index: idx,
            event_type: "RunAdmission",
            step: None,
            seq: seq.get(),
            extra_json: vec![
                (
                    "artifact_digest",
                    serde_json::Value::from(format!("{artifact_digest:?}")),
                ),
                (
                    "granted_capabilities",
                    serde_json::Value::from(format!("{granted_capabilities:?}")),
                ),
                ("policy", serde_json::Value::from(format!("{policy:?}"))),
            ],
        },
        JournalEvent::StepStarted { seq, step, .. } => TraceEntry {
            index: idx,
            event_type: "StepStarted",
            step: Some(step.get()),
            seq: seq.get(),
            extra_json: vec![],
        },
        JournalEvent::StepSucceeded {
            seq, step, output, ..
        } => TraceEntry {
            index: idx,
            event_type: "StepSucceeded",
            step: Some(step.get()),
            seq: seq.get(),
            extra_json: vec![("output", serde_json::Value::from(output.get()))],
        },
        JournalEvent::ActionScheduled {
            seq, step, action, ..
        } => TraceEntry {
            index: idx,
            event_type: "ActionScheduled",
            step: Some(step.get()),
            seq: seq.get(),
            extra_json: vec![("action", serde_json::Value::from(action.get()))],
        },
        JournalEvent::ActionCompletedEvent {
            seq, step, action, ..
        } => TraceEntry {
            index: idx,
            event_type: "ActionCompleted",
            step: Some(step.get()),
            seq: seq.get(),
            extra_json: vec![("action", serde_json::Value::from(action.get()))],
        },
        JournalEvent::ActionFailedEvent {
            seq, step, action, ..
        } => TraceEntry {
            index: idx,
            event_type: "ActionFailed",
            step: Some(step.get()),
            seq: seq.get(),
            extra_json: vec![("action", serde_json::Value::from(action.get()))],
        },
        JournalEvent::SlotWrittenEvent { seq, slot, .. } => TraceEntry {
            index: idx,
            event_type: "SlotWritten",
            step: None,
            seq: seq.get(),
            extra_json: vec![("slot", serde_json::Value::from(slot.get()))],
        },
        JournalEvent::WaitScheduledEvent { seq, step, .. } => TraceEntry {
            index: idx,
            event_type: "WaitScheduled",
            step: Some(step.get()),
            seq: seq.get(),
            extra_json: vec![],
        },
        JournalEvent::AskScheduledEvent { seq, step, .. } => TraceEntry {
            index: idx,
            event_type: "AskScheduled",
            step: Some(step.get()),
            seq: seq.get(),
            extra_json: vec![],
        },
        JournalEvent::AskAnsweredEvent { seq, step, .. } => TraceEntry {
            index: idx,
            event_type: "AskAnswered",
            step: Some(step.get()),
            seq: seq.get(),
            extra_json: vec![],
        },
        JournalEvent::RetryScheduledEvent { seq, step, .. } => TraceEntry {
            index: idx,
            event_type: "RetryScheduled",
            step: Some(step.get()),
            seq: seq.get(),
            extra_json: vec![],
        },
        JournalEvent::RunCancelled { seq, .. } => TraceEntry {
            index: idx,
            event_type: "RunCancelled",
            step: None,
            seq: seq.get(),
            extra_json: vec![],
        },
        JournalEvent::RunFinished { seq, result, .. } => TraceEntry {
            index: idx,
            event_type: "RunFinished",
            step: None,
            seq: seq.get(),
            extra_json: vec![("result", serde_json::Value::from(result.get()))],
        },
        JournalEvent::RunFailedEvent { seq, .. } => TraceEntry {
            index: idx,
            event_type: "RunFailed",
            step: None,
            seq: seq.get(),
            extra_json: vec![],
        },
        JournalEvent::RunResumed { run, .. } => TraceEntry {
            index: idx,
            event_type: "RunResumed",
            step: None,
            seq: 0,
            extra_json: vec![("run", serde_json::Value::from(run.get()))],
        },
        JournalEvent::RunRetried { run, .. } => TraceEntry {
            index: idx,
            event_type: "RunRetried",
            step: None,
            seq: 0,
            extra_json: vec![("run", serde_json::Value::from(run.get()))],
        },
        JournalEvent::RunAnswered {
            run,
            slot_idx,
            answer,
            ..
        } => TraceEntry {
            index: idx,
            event_type: "RunAnswered",
            step: None,
            seq: 0,
            extra_json: vec![
                ("run", serde_json::Value::from(run.get())),
                ("slot_idx", serde_json::Value::from(slot_idx.get())),
                ("answer", serde_json::Value::from(format!("{:?}", answer))),
            ],
        },
        _ => TraceEntry {
            index: idx,
            event_type: "Unknown",
            step: None,
            seq: 0,
            extra_json: vec![],
        },
    }
}

// ---------------------------------------------------------------------------
// Retry
// ---------------------------------------------------------------------------

/// Analysis result produced by [`analyze_retry`].
#[derive(Debug, Clone)]
pub(crate) struct RetryAnalysis {
    pub failed_at_step: Option<u16>,
    pub last_successful_step: Option<u16>,
    pub can_retry: bool,
    pub reason: String,
}

/// Scan a journal and decide whether a retry is possible.
///
/// Returns structured data; the caller is responsible for formatting.
pub(crate) fn analyze_retry(events: &[JournalEvent]) -> RetryAnalysis {
    let mut last_successful_step: Option<u16> = None;
    let mut failed_step: Option<u16> = None;

    for event in events {
        match event {
            JournalEvent::StepSucceeded { step, .. } => {
                last_successful_step = Some(step.get());
            }
            JournalEvent::ActionFailedEvent { step, .. } => {
                failed_step = Some(step.get());
            }
            _ => {}
        }
    }

    // Check terminal status
    let terminal = events.last();
    let is_failed = matches!(
        terminal,
        Some(JournalEvent::RunFailedEvent { .. }) | Some(JournalEvent::RunCancelled { .. })
    );

    if !is_failed {
        return RetryAnalysis {
            failed_at_step: None,
            last_successful_step,
            can_retry: false,
            reason: "run did not fail (no retry needed)".into(),
        };
    }

    let failure_step = failed_step.or(last_successful_step.map(|s| s.saturating_add(1)));

    RetryAnalysis {
        failed_at_step: failure_step,
        last_successful_step,
        can_retry: true,
        reason: String::new(),
    }
}

// ---------------------------------------------------------------------------
// Resume
// ---------------------------------------------------------------------------

/// Analysis result produced by [`analyze_resume`].
#[derive(Debug, Clone)]
pub(crate) struct ResumeAnalysis {
    pub suspended_at_step: Option<u16>,
    pub can_resume: bool,
    pub reason: String,
}

/// Scan a journal and decide whether a resume is possible.
///
/// Returns structured data; the caller is responsible for formatting.
pub(crate) fn analyze_resume(events: &[JournalEvent]) -> ResumeAnalysis {
    // Scan for suspension indicators: WaitScheduled or AskScheduled
    let mut suspended_at_step: Option<u16> = None;
    for event in events {
        match event {
            JournalEvent::WaitScheduledEvent { step, .. } => {
                suspended_at_step = Some(step.get());
            }
            JournalEvent::AskScheduledEvent { step, .. } => {
                suspended_at_step = Some(step.get());
            }
            _ => {}
        }
    }

    // Check terminal status - the run must not be finished/failed/cancelled
    let terminal = events.last();
    let is_terminal = matches!(
        terminal,
        Some(JournalEvent::RunFinished { .. })
            | Some(JournalEvent::RunFailedEvent { .. })
            | Some(JournalEvent::RunCancelled { .. })
    );

    if is_terminal {
        let status = match terminal {
            Some(JournalEvent::RunFinished { .. }) => "finished",
            Some(JournalEvent::RunFailedEvent { .. }) => "failed",
            Some(JournalEvent::RunCancelled { .. }) => "cancelled",
            _ => "unknown",
        };
        return ResumeAnalysis {
            suspended_at_step,
            can_resume: false,
            reason: format!("run is {status}, not suspended"),
        };
    }

    ResumeAnalysis {
        suspended_at_step,
        can_resume: true,
        reason: String::new(),
    }
}
