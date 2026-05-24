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
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TraceEntry {
    pub index: usize,
    pub event_type: &'static str,
    pub step: Option<u16>,
    pub status: Option<TraceStatus>,
    pub action: Option<u16>,
    pub seq: u64,
    /// Extra key-value pairs for JSON output (variant-specific fields).
    pub extra_json: Vec<(&'static str, serde_json::Value)>,
}

/// Trace status categories used by CLI filters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TraceStatus {
    Pending,
    Active,
    WaitingAnswer,
    Cancelled,
    Completed,
    Failed,
}

impl TraceStatus {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Active => "active",
            Self::WaitingAnswer => "waiting_answer",
            Self::Cancelled => "cancelled",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }
}

/// Optional trace filters. All populated filters must match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct TraceFilters {
    pub(crate) step: Option<u16>,
    pub(crate) action: Option<u16>,
    pub(crate) status: Option<TraceStatus>,
    pub(crate) since_seq: Option<u64>,
    pub(crate) until_seq: Option<u64>,
    pub(crate) limit: Option<usize>,
}

/// Scan a slice of journal events and return one structured entry per event.
pub(crate) fn build_trace(events: &[JournalEvent]) -> Vec<TraceEntry> {
    events
        .iter()
        .enumerate()
        .map(|(idx, event)| trace_one(idx, event))
        .collect()
}

/// Apply trace filters while preserving the surviving entries' original order and index.
pub(crate) fn filter_trace(entries: Vec<TraceEntry>, filters: TraceFilters) -> Vec<TraceEntry> {
    let filtered = entries
        .into_iter()
        .filter(|entry| trace_entry_matches_filters(entry, filters));
    match filters.limit {
        Some(limit) => filtered.take(limit).collect(),
        None => filtered.collect(),
    }
}

fn trace_entry_matches_filters(entry: &TraceEntry, filters: TraceFilters) -> bool {
    let step_matches = filters
        .step
        .is_none_or(|expected_step| entry.step == Some(expected_step));
    let action_matches = filters
        .action
        .is_none_or(|expected_action| entry.action == Some(expected_action));
    let status_matches = filters
        .status
        .is_none_or(|expected_status| entry.status == Some(expected_status));
    let since_seq_matches = filters
        .since_seq
        .is_none_or(|minimum_seq| entry.seq >= minimum_seq);
    let until_seq_matches = filters
        .until_seq
        .is_none_or(|maximum_seq| entry.seq <= maximum_seq);
    step_matches && action_matches && status_matches && since_seq_matches && until_seq_matches
}

fn trace_one(idx: usize, event: &JournalEvent) -> TraceEntry {
    match event {
        JournalEvent::RunAccepted { seq, run, workflow } => TraceEntry {
            index: idx,
            event_type: "RunAccepted",
            step: None,
            status: Some(TraceStatus::Pending),
            action: None,
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
            status: Some(TraceStatus::Pending),
            action: None,
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
            status: Some(TraceStatus::Active),
            action: None,
            seq: seq.get(),
            extra_json: vec![],
        },
        JournalEvent::StepSucceeded {
            seq, step, output, ..
        } => TraceEntry {
            index: idx,
            event_type: "StepSucceeded",
            step: Some(step.get()),
            status: Some(TraceStatus::Completed),
            action: None,
            seq: seq.get(),
            extra_json: vec![("output", serde_json::Value::from(output.get()))],
        },
        JournalEvent::ActionScheduled {
            seq, step, action, ..
        } => TraceEntry {
            index: idx,
            event_type: "ActionScheduled",
            step: Some(step.get()),
            status: Some(TraceStatus::Active),
            action: Some(action.get()),
            seq: seq.get(),
            extra_json: vec![("action", serde_json::Value::from(action.get()))],
        },
        JournalEvent::ActionCompletedEvent {
            seq, step, action, ..
        } => TraceEntry {
            index: idx,
            event_type: "ActionCompleted",
            step: Some(step.get()),
            status: Some(TraceStatus::Completed),
            action: Some(action.get()),
            seq: seq.get(),
            extra_json: vec![("action", serde_json::Value::from(action.get()))],
        },
        JournalEvent::ActionFailedEvent {
            seq, step, action, ..
        } => TraceEntry {
            index: idx,
            event_type: "ActionFailed",
            step: Some(step.get()),
            status: Some(TraceStatus::Failed),
            action: Some(action.get()),
            seq: seq.get(),
            extra_json: vec![("action", serde_json::Value::from(action.get()))],
        },
        JournalEvent::SlotWrittenEvent { seq, slot, .. } => TraceEntry {
            index: idx,
            event_type: "SlotWritten",
            step: None,
            status: Some(TraceStatus::Completed),
            action: None,
            seq: seq.get(),
            extra_json: vec![("slot", serde_json::Value::from(slot.get()))],
        },
        JournalEvent::WaitScheduledEvent { seq, step, .. } => TraceEntry {
            index: idx,
            event_type: "WaitScheduled",
            step: Some(step.get()),
            status: Some(TraceStatus::Active),
            action: None,
            seq: seq.get(),
            extra_json: vec![],
        },
        JournalEvent::AskScheduledEvent { seq, step, .. } => TraceEntry {
            index: idx,
            event_type: "AskScheduled",
            step: Some(step.get()),
            status: Some(TraceStatus::WaitingAnswer),
            action: None,
            seq: seq.get(),
            extra_json: vec![],
        },
        JournalEvent::AskAnsweredEvent { seq, step, .. } => TraceEntry {
            index: idx,
            event_type: "AskAnswered",
            step: Some(step.get()),
            status: Some(TraceStatus::Completed),
            action: None,
            seq: seq.get(),
            extra_json: vec![],
        },
        JournalEvent::RetryScheduledEvent { seq, step, .. } => TraceEntry {
            index: idx,
            event_type: "RetryScheduled",
            step: Some(step.get()),
            status: Some(TraceStatus::Active),
            action: None,
            seq: seq.get(),
            extra_json: vec![],
        },
        JournalEvent::RunCancelled { seq, .. } => TraceEntry {
            index: idx,
            event_type: "RunCancelled",
            step: None,
            status: Some(TraceStatus::Cancelled),
            action: None,
            seq: seq.get(),
            extra_json: vec![],
        },
        JournalEvent::RunFinished { seq, result, .. } => TraceEntry {
            index: idx,
            event_type: "RunFinished",
            step: None,
            status: Some(TraceStatus::Completed),
            action: None,
            seq: seq.get(),
            extra_json: vec![("result", serde_json::Value::from(result.get()))],
        },
        JournalEvent::RunFailedEvent { seq, .. } => TraceEntry {
            index: idx,
            event_type: "RunFailed",
            step: None,
            status: Some(TraceStatus::Failed),
            action: None,
            seq: seq.get(),
            extra_json: vec![],
        },
        JournalEvent::RunResumed { run, seq, .. } => TraceEntry {
            index: idx,
            event_type: "RunResumed",
            step: None,
            status: Some(TraceStatus::Active),
            action: None,
            seq: seq.get(),
            extra_json: vec![("run", serde_json::Value::from(run.get()))],
        },
        JournalEvent::RunRetried { run, seq, .. } => TraceEntry {
            index: idx,
            event_type: "RunRetried",
            step: None,
            status: Some(TraceStatus::Active),
            action: None,
            seq: seq.get(),
            extra_json: vec![("run", serde_json::Value::from(run.get()))],
        },
        JournalEvent::RunAnswered {
            run,
            seq,
            slot_idx,
            answer,
            ..
        } => TraceEntry {
            index: idx,
            event_type: "RunAnswered",
            step: None,
            status: Some(TraceStatus::Completed),
            action: None,
            seq: seq.get(),
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
            status: None,
            action: None,
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

    // A run that is currently waiting for an answer cannot be resumed.
    let is_waiting = matches!(
        terminal,
        Some(JournalEvent::AskScheduledEvent { .. })
            | Some(JournalEvent::WaitScheduledEvent { .. })
    );

    if is_waiting {
        return ResumeAnalysis {
            suspended_at_step,
            can_resume: false,
            reason: "run is waiting for answer — cannot resume".to_string(),
        };
    }

    ResumeAnalysis {
        suspended_at_step,
        can_resume: true,
        reason: String::new(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use proptest::proptest;
    use vb_core::ids::{ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest};
    use vb_core::value::ConstValue;
    use vb_storage::JournalEvent;
    use vb_storage::types::EventSeq;

    fn make_run_id(v: u64) -> RunId {
        RunId::new(v)
    }

    fn make_event_seq(v: u64) -> EventSeq {
        EventSeq::new(v)
    }

    fn make_step_idx(v: u16) -> StepIdx {
        StepIdx::new(v)
    }

    fn make_action_id(v: u16) -> ActionId {
        ActionId::new(v)
    }

    fn make_slot_idx(v: u16) -> SlotIdx {
        SlotIdx::new(v)
    }

    fn dummy_digest() -> WorkflowDigest {
        WorkflowDigest::from_bytes([0xAB_u8; 32])
    }

    // -------------------------------------------------------------------------
    // build_trace — pure behavior
    // -------------------------------------------------------------------------

    #[test]
    fn build_trace_returns_identical_output_for_identical_events() {
        let events = [
            JournalEvent::RunAccepted {
                run: make_run_id(1),
                seq: make_event_seq(0),
                workflow: dummy_digest(),
            },
            JournalEvent::StepStarted {
                run: make_run_id(1),
                seq: make_event_seq(1),
                step: make_step_idx(0),
                attempt: 1,
            },
            JournalEvent::StepSucceeded {
                run: make_run_id(1),
                seq: make_event_seq(2),
                step: make_step_idx(0),
                output: make_slot_idx(0),
            },
        ];
        let trace = build_trace(&events);
        assert_eq!(trace.len(), 3);
        assert_eq!(trace[0].event_type, "RunAccepted");
        assert_eq!(trace[0].step, None);
        assert_eq!(trace[1].event_type, "StepStarted");
        assert_eq!(trace[1].step, Some(0));
        assert_eq!(trace[2].event_type, "StepSucceeded");
        assert_eq!(trace[2].step, Some(0));
    }

    #[test]
    fn build_trace_preserves_event_order() {
        let events = [
            JournalEvent::RunAccepted {
                run: make_run_id(1),
                seq: make_event_seq(0),
                workflow: dummy_digest(),
            },
            JournalEvent::StepStarted {
                run: make_run_id(1),
                seq: make_event_seq(1),
                step: make_step_idx(0),
                attempt: 1,
            },
            JournalEvent::StepSucceeded {
                run: make_run_id(1),
                seq: make_event_seq(2),
                step: make_step_idx(0),
                output: make_slot_idx(0),
            },
            JournalEvent::ActionScheduled {
                run: make_run_id(1),
                seq: make_event_seq(3),
                step: make_step_idx(1),
                action: make_action_id(1),
                attempt: 1,
            },
        ];
        let trace = build_trace(&events);
        for (i, entry) in trace.iter().enumerate() {
            assert_eq!(entry.index, i, "index must match position for entry {i}");
        }
    }

    #[test]
    fn build_trace_empty_input_returns_empty_output() {
        let events: [JournalEvent; 0] = [];
        let trace = build_trace(&events);
        assert!(trace.is_empty());
    }

    #[test]
    fn build_trace_step_events_have_step_value() {
        let events = [
            JournalEvent::RunAccepted {
                run: make_run_id(1),
                seq: make_event_seq(0),
                workflow: dummy_digest(),
            },
            JournalEvent::StepStarted {
                run: make_run_id(1),
                seq: make_event_seq(1),
                step: make_step_idx(5),
                attempt: 1,
            },
        ];
        let trace = build_trace(&events);
        assert_eq!(trace[1].step, Some(5));
    }

    #[test]
    fn build_trace_run_level_events_have_no_step() {
        let events = [
            JournalEvent::RunAccepted {
                run: make_run_id(1),
                seq: make_event_seq(0),
                workflow: dummy_digest(),
            },
            JournalEvent::RunFinished {
                run: make_run_id(1),
                seq: make_event_seq(1),
                result: make_slot_idx(0),
                attempt: 1,
            },
        ];
        let trace = build_trace(&events);
        assert_eq!(trace[0].step, None);
        assert_eq!(trace[1].step, None);
    }

    #[test]
    fn build_trace_maps_seq_field_correctly() {
        let events = [JournalEvent::RunAccepted {
            run: make_run_id(1),
            seq: make_event_seq(42),
            workflow: dummy_digest(),
        }];
        let trace = build_trace(&events);
        assert_eq!(trace[0].seq, 42);
    }

    #[test]
    fn filter_trace_by_step_preserves_original_indices() {
        let events = [
            JournalEvent::StepStarted {
                run: make_run_id(1),
                seq: make_event_seq(1),
                step: make_step_idx(1),
                attempt: 1,
            },
            JournalEvent::StepStarted {
                run: make_run_id(1),
                seq: make_event_seq(2),
                step: make_step_idx(7),
                attempt: 1,
            },
        ];
        let filtered = filter_trace(
            build_trace(&events),
            TraceFilters {
                step: Some(7),
                ..TraceFilters::default()
            },
        );

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].index, 1);
        assert_eq!(filtered[0].step, Some(7));
    }

    #[test]
    fn filter_trace_by_action_matches_action_id_only() {
        let events = [
            JournalEvent::ActionScheduled {
                run: make_run_id(1),
                seq: make_event_seq(1),
                step: make_step_idx(2),
                action: make_action_id(3),
                attempt: 1,
            },
            JournalEvent::ActionFailedEvent {
                run: make_run_id(1),
                seq: make_event_seq(2),
                step: make_step_idx(2),
                action: make_action_id(9),
                attempt: 1,
            },
        ];
        let filtered = filter_trace(
            build_trace(&events),
            TraceFilters {
                action: Some(9),
                ..TraceFilters::default()
            },
        );

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].action, Some(9));
        assert_eq!(filtered[0].event_type, "ActionFailed");
    }

    #[test]
    fn filter_trace_by_status_uses_event_status_categories() {
        let events = [
            JournalEvent::RunAccepted {
                run: make_run_id(1),
                seq: make_event_seq(0),
                workflow: dummy_digest(),
            },
            JournalEvent::StepStarted {
                run: make_run_id(1),
                seq: make_event_seq(1),
                step: make_step_idx(0),
                attempt: 1,
            },
            JournalEvent::RunFinished {
                run: make_run_id(1),
                seq: make_event_seq(2),
                result: make_slot_idx(0),
                attempt: 1,
            },
        ];
        let filtered = filter_trace(
            build_trace(&events),
            TraceFilters {
                status: Some(TraceStatus::Completed),
                ..TraceFilters::default()
            },
        );

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].event_type, "RunFinished");
        assert_eq!(filtered[0].status, Some(TraceStatus::Completed));
    }

    #[test]
    fn filter_trace_limit_applies_after_filters() {
        let events = [
            JournalEvent::StepStarted {
                run: make_run_id(1),
                seq: make_event_seq(1),
                step: make_step_idx(0),
                attempt: 1,
            },
            JournalEvent::StepStarted {
                run: make_run_id(1),
                seq: make_event_seq(2),
                step: make_step_idx(0),
                attempt: 1,
            },
            JournalEvent::RunFinished {
                run: make_run_id(1),
                seq: make_event_seq(3),
                result: make_slot_idx(0),
                attempt: 1,
            },
        ];
        let filtered = filter_trace(
            build_trace(&events),
            TraceFilters {
                status: Some(TraceStatus::Active),
                limit: Some(1),
                ..TraceFilters::default()
            },
        );

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].seq, 1);
    }

    #[test]
    fn filter_trace_by_sequence_range_is_inclusive() {
        let events = [
            JournalEvent::RunAccepted {
                run: make_run_id(1),
                seq: make_event_seq(9),
                workflow: dummy_digest(),
            },
            JournalEvent::StepStarted {
                run: make_run_id(1),
                seq: make_event_seq(10),
                step: make_step_idx(0),
                attempt: 1,
            },
            JournalEvent::StepSucceeded {
                run: make_run_id(1),
                seq: make_event_seq(20),
                step: make_step_idx(0),
                output: make_slot_idx(0),
            },
            JournalEvent::RunFinished {
                run: make_run_id(1),
                seq: make_event_seq(21),
                result: make_slot_idx(0),
                attempt: 1,
            },
        ];

        let filtered = filter_trace(
            build_trace(&events),
            TraceFilters {
                since_seq: Some(10),
                until_seq: Some(20),
                ..TraceFilters::default()
            },
        );

        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].seq, 10);
        assert_eq!(filtered[1].seq, 20);
    }

    #[test]
    fn build_trace_run_resumed_uses_event_seq() {
        let events = [JournalEvent::RunResumed {
            run: make_run_id(1),
            seq: make_event_seq(20),
            timestamp: Utc::now(),
        }];
        let trace = build_trace(&events);
        assert_eq!(trace[0].seq, 20);
        assert_eq!(trace[0].event_type, "RunResumed");
    }

    #[test]
    fn build_trace_length_matches_input() {
        let events: Vec<JournalEvent> = (0..10)
            .map(|i| JournalEvent::RunAccepted {
                run: make_run_id(1),
                seq: make_event_seq(i),
                workflow: dummy_digest(),
            })
            .collect();
        let trace = build_trace(&events);
        assert_eq!(trace.len(), events.len());
    }

    // -------------------------------------------------------------------------
    // trace_one — per-variant correctness
    // -------------------------------------------------------------------------

    #[test]
    fn trace_one_run_accepted_maps_correct_fields() {
        let event = JournalEvent::RunAccepted {
            run: make_run_id(1),
            seq: make_event_seq(5),
            workflow: dummy_digest(),
        };
        let entry = trace_one(0, &event);
        assert_eq!(entry.event_type, "RunAccepted");
        assert_eq!(entry.seq, 5);
        assert_eq!(entry.step, None);
        assert_eq!(entry.index, 0);
    }

    #[test]
    fn trace_one_run_admission_maps_correct_fields() {
        let event = JournalEvent::RunAdmission {
            run: make_run_id(2),
            seq: make_event_seq(3),
            artifact_digest: dummy_digest(),
            granted_capabilities: vb_core::CapabilitySet::empty(),
            policy: vb_core::RuntimePolicy::Strict,
        };
        let entry = trace_one(1, &event);
        assert_eq!(entry.event_type, "RunAdmission");
        assert_eq!(entry.seq, 3);
        assert_eq!(entry.step, None);
        assert_eq!(entry.index, 1);
    }

    #[test]
    fn trace_one_step_started_maps_correct_fields() {
        let event = JournalEvent::StepStarted {
            run: make_run_id(1),
            seq: make_event_seq(7),
            step: make_step_idx(3),
            attempt: 2,
        };
        let entry = trace_one(2, &event);
        assert_eq!(entry.event_type, "StepStarted");
        assert_eq!(entry.seq, 7);
        assert_eq!(entry.step, Some(3));
        assert_eq!(entry.index, 2);
    }

    #[test]
    fn trace_one_step_succeeded_maps_correct_fields() {
        let event = JournalEvent::StepSucceeded {
            run: make_run_id(1),
            seq: make_event_seq(8),
            step: make_step_idx(3),
            output: make_slot_idx(1),
        };
        let entry = trace_one(3, &event);
        assert_eq!(entry.event_type, "StepSucceeded");
        assert_eq!(entry.seq, 8);
        assert_eq!(entry.step, Some(3));
        assert_eq!(entry.index, 3);
    }

    #[test]
    fn trace_one_action_scheduled_maps_correct_fields() {
        let event = JournalEvent::ActionScheduled {
            run: make_run_id(1),
            seq: make_event_seq(9),
            step: make_step_idx(2),
            action: make_action_id(4),
            attempt: 1,
        };
        let entry = trace_one(4, &event);
        assert_eq!(entry.event_type, "ActionScheduled");
        assert_eq!(entry.seq, 9);
        assert_eq!(entry.step, Some(2));
        assert_eq!(entry.index, 4);
    }

    #[test]
    fn trace_one_action_completed_maps_correct_fields() {
        let event = JournalEvent::ActionCompletedEvent {
            run: make_run_id(1),
            seq: make_event_seq(10),
            step: make_step_idx(2),
            action: make_action_id(4),
            attempt: 1,
        };
        let entry = trace_one(5, &event);
        assert_eq!(entry.event_type, "ActionCompleted");
        assert_eq!(entry.seq, 10);
        assert_eq!(entry.step, Some(2));
        assert_eq!(entry.index, 5);
    }

    #[test]
    fn trace_one_action_failed_maps_correct_fields() {
        let event = JournalEvent::ActionFailedEvent {
            run: make_run_id(1),
            seq: make_event_seq(11),
            step: make_step_idx(2),
            action: make_action_id(4),
            attempt: 1,
        };
        let entry = trace_one(6, &event);
        assert_eq!(entry.event_type, "ActionFailed");
        assert_eq!(entry.seq, 11);
        assert_eq!(entry.step, Some(2));
        assert_eq!(entry.index, 6);
    }

    #[test]
    fn trace_one_slot_written_maps_correct_fields() {
        let event = JournalEvent::SlotWrittenEvent {
            run: make_run_id(1),
            seq: make_event_seq(12),
            slot: make_slot_idx(3),
            value: Some(vec![0x01, 0x02]),
            extra: None,
            attempt: 1,
        };
        let entry = trace_one(7, &event);
        assert_eq!(entry.event_type, "SlotWritten");
        assert_eq!(entry.seq, 12);
        assert_eq!(entry.step, None);
        assert_eq!(entry.index, 7);
    }

    #[test]
    fn trace_one_wait_scheduled_maps_correct_fields() {
        let event = JournalEvent::WaitScheduledEvent {
            run: make_run_id(1),
            seq: make_event_seq(13),
            step: make_step_idx(4),
            attempt: 1,
        };
        let entry = trace_one(8, &event);
        assert_eq!(entry.event_type, "WaitScheduled");
        assert_eq!(entry.seq, 13);
        assert_eq!(entry.step, Some(4));
        assert_eq!(entry.index, 8);
    }

    #[test]
    fn trace_one_ask_scheduled_maps_correct_fields() {
        let event = JournalEvent::AskScheduledEvent {
            run: make_run_id(1),
            seq: make_event_seq(14),
            step: make_step_idx(5),
            attempt: 1,
        };
        let entry = trace_one(9, &event);
        assert_eq!(entry.event_type, "AskScheduled");
        assert_eq!(entry.seq, 14);
        assert_eq!(entry.step, Some(5));
        assert_eq!(entry.index, 9);
    }

    #[test]
    fn trace_one_ask_answered_maps_correct_fields() {
        let event = JournalEvent::AskAnsweredEvent {
            run: make_run_id(1),
            seq: make_event_seq(15),
            step: make_step_idx(5),
            attempt: 1,
        };
        let entry = trace_one(10, &event);
        assert_eq!(entry.event_type, "AskAnswered");
        assert_eq!(entry.seq, 15);
        assert_eq!(entry.step, Some(5));
        assert_eq!(entry.index, 10);
    }

    #[test]
    fn trace_one_retry_scheduled_maps_correct_fields() {
        let event = JournalEvent::RetryScheduledEvent {
            run: make_run_id(1),
            seq: make_event_seq(16),
            step: make_step_idx(3),
            attempt: 1,
        };
        let entry = trace_one(11, &event);
        assert_eq!(entry.event_type, "RetryScheduled");
        assert_eq!(entry.seq, 16);
        assert_eq!(entry.step, Some(3));
        assert_eq!(entry.index, 11);
    }

    #[test]
    fn trace_one_run_cancelled_maps_correct_fields() {
        let event = JournalEvent::RunCancelled {
            run: make_run_id(1),
            seq: make_event_seq(17),
            attempt: 1,
            reason: Some("user requested".to_string()),
        };
        let entry = trace_one(12, &event);
        assert_eq!(entry.event_type, "RunCancelled");
        assert_eq!(entry.seq, 17);
        assert_eq!(entry.step, None);
        assert_eq!(entry.index, 12);
    }

    #[test]
    fn trace_one_run_finished_maps_correct_fields() {
        let event = JournalEvent::RunFinished {
            run: make_run_id(1),
            seq: make_event_seq(18),
            result: make_slot_idx(7),
            attempt: 1,
        };
        let entry = trace_one(13, &event);
        assert_eq!(entry.event_type, "RunFinished");
        assert_eq!(entry.seq, 18);
        assert_eq!(entry.step, None);
        assert_eq!(entry.index, 13);
    }

    #[test]
    fn trace_one_run_failed_maps_correct_fields() {
        let event = JournalEvent::RunFailedEvent {
            run: make_run_id(1),
            seq: make_event_seq(19),
            attempt: 1,
        };
        let entry = trace_one(14, &event);
        assert_eq!(entry.event_type, "RunFailed");
        assert_eq!(entry.seq, 19);
        assert_eq!(entry.step, None);
        assert_eq!(entry.index, 14);
    }

    #[test]
    fn trace_one_run_resumed_uses_event_seq() {
        let event = JournalEvent::RunResumed {
            run: make_run_id(1),
            seq: make_event_seq(20),
            timestamp: Utc::now(),
        };
        let entry = trace_one(15, &event);
        assert_eq!(entry.event_type, "RunResumed");
        assert_eq!(entry.seq, 20);
        assert_eq!(entry.step, None);
        assert_eq!(entry.index, 15);
    }

    #[test]
    fn trace_one_run_retried_uses_event_seq() {
        let event = JournalEvent::RunRetried {
            run: make_run_id(1),
            seq: make_event_seq(21),
            timestamp: Utc::now(),
        };
        let entry = trace_one(16, &event);
        assert_eq!(entry.event_type, "RunRetried");
        assert_eq!(entry.seq, 21);
        assert_eq!(entry.step, None);
        assert_eq!(entry.index, 16);
    }

    #[test]
    fn trace_one_run_answered_uses_event_seq() {
        let event = JournalEvent::RunAnswered {
            run: make_run_id(1),
            seq: make_event_seq(22),
            slot_idx: make_slot_idx(2),
            answer: ConstValue::Null,
            timestamp: Utc::now(),
        };
        let entry = trace_one(17, &event);
        assert_eq!(entry.event_type, "RunAnswered");
        assert_eq!(entry.seq, 22);
        assert_eq!(entry.step, None);
        assert_eq!(entry.index, 17);
    }

    // -------------------------------------------------------------------------
    // Invariants — deterministic, length-preserving, index-correct
    // -------------------------------------------------------------------------

    #[test]
    fn build_trace_is_deterministic() {
        // Test with a mixed vector of events
        let events = vec![
            JournalEvent::RunAccepted {
                run: make_run_id(1),
                seq: make_event_seq(0),
                workflow: dummy_digest(),
            },
            JournalEvent::StepStarted {
                run: make_run_id(1),
                seq: make_event_seq(1),
                step: make_step_idx(0),
                attempt: 1,
            },
            JournalEvent::StepSucceeded {
                run: make_run_id(1),
                seq: make_event_seq(2),
                step: make_step_idx(0),
                output: make_slot_idx(0),
            },
            JournalEvent::RunFinished {
                run: make_run_id(1),
                seq: make_event_seq(3),
                result: make_slot_idx(0),
                attempt: 1,
            },
        ];
        let trace1 = build_trace(&events);
        let trace2 = build_trace(&events);
        assert_eq!(trace1, trace2, "build_trace must be deterministic");
    }

    #[test]
    fn build_trace_preserves_length_empty() {
        let events: Vec<JournalEvent> = vec![];
        let trace = build_trace(&events);
        assert_eq!(trace.len(), 0);
    }

    #[test]
    fn build_trace_preserves_length_small() {
        let events = vec![
            JournalEvent::RunAccepted {
                run: make_run_id(1),
                seq: make_event_seq(0),
                workflow: dummy_digest(),
            },
            JournalEvent::StepStarted {
                run: make_run_id(1),
                seq: make_event_seq(1),
                step: make_step_idx(0),
                attempt: 1,
            },
        ];
        let trace = build_trace(&events);
        assert_eq!(trace.len(), 2);
    }

    #[test]
    fn build_trace_preserves_length_large() {
        let events: Vec<JournalEvent> = (0..50)
            .map(|i| JournalEvent::RunAccepted {
                run: make_run_id(1),
                seq: make_event_seq(i),
                workflow: dummy_digest(),
            })
            .collect();
        let trace = build_trace(&events);
        assert_eq!(trace.len(), 50);
    }

    proptest! {
        #[test]
        fn trace_entry_index_matches_position(idx in 0usize..1000_usize) {
            // Create a deterministic event for this index
            let event = JournalEvent::RunAccepted {
                run: make_run_id(1),
                seq: make_event_seq(idx as u64),
                workflow: dummy_digest(),
            };
            let entry = trace_one(idx, &event);
            assert_eq!(entry.index, idx, "trace_one index must match provided index");
        }
    }
}
