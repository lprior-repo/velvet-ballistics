verus! {
        /// Mirror of `run: RunId`.
        run: RunId,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `step: StepIdx`.
        step: StepIdx,
        /// Mirror of `action: ActionId`.
        action: ActionId,
        /// Mirror of `attempt: u16`.
        attempt: u16,
    },
    /// Mirror of `JournalEvent::ActionScheduledTicket` at events.rs:95-106.
    ActionScheduledTicket {
        /// Mirror of `run: RunId`.
        run: RunId,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `ticket: ActionTicket`.
        ticket: ActionTicket,
        /// Mirror of `input: SlotIdx`.
        input: SlotIdx,
        /// Mirror of `output: SlotIdx`.
        output: SlotIdx,
    },
    /// Mirror of `JournalEvent::ActionCompletedEnvelope` at events.rs:108-127.
    ActionCompletedEnvelope {
        /// Mirror of `run: RunId`.
        run: RunId,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `ticket: ActionTicket`.
        ticket: ActionTicket,
        /// Mirror of `output: SlotIdx`.
        output: SlotIdx,
        /// Mirror of `outcome: DurableActionOutcome` (placeholder u8).
        outcome: u8,
        /// Mirror of `value: Vec<u8>` (placeholder unit).
        value: (),
        /// Mirror of `encoded_len: u32`.
        encoded_len: u32,
        /// Mirror of `taint: Taint` (placeholder u8).
        taint: u8,
        /// Mirror of `value_digest: [u8; 32]` (placeholder unit).
        value_digest: (),
    },
    /// Mirror of `JournalEvent::ActionFailedEvent` at events.rs:129-140.
    ActionFailedEvent {
        /// Mirror of `run: RunId`.
        run: RunId,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `step: StepIdx`.
        step: StepIdx,
        /// Mirror of `action: ActionId`.
        action: ActionId,
        /// Mirror of `attempt: u16`.
        attempt: u16,
    },
    /// Mirror of `JournalEvent::ActionAbandoned` at events.rs:148-158.
    ActionAbandoned {
        /// Mirror of `run: RunId`.
        run: RunId,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `ticket: ActionTicket`.
        ticket: ActionTicket,
    },
    /// Mirror of `JournalEvent::SlotWrittenEvent` at events.rs:160-174.
    SlotWrittenEvent {
        /// Mirror of `run: RunId`.
        run: RunId,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `slot: SlotIdx`.
        slot: SlotIdx,
        /// Mirror of `value: Option<Vec<u8>>` (placeholder Option<()>).
        value: Option<()>,
        /// Mirror of `extra: Option<Vec<u8>>` (placeholder Option<()>).
        extra: Option<()>,
        /// Mirror of `attempt: u16`.
        attempt: u16,
    },
    /// Mirror of `JournalEvent::WaitScheduledEvent` at events.rs:176-185.
    WaitScheduledEvent {
        /// Mirror of `run: RunId`.
        run: RunId,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `step: StepIdx`.
        step: StepIdx,
        /// Mirror of `attempt: u16`.
        attempt: u16,
    },
    /// Mirror of `JournalEvent::AskScheduledEvent` at events.rs:187-196.
    AskScheduledEvent {
        /// Mirror of `run: RunId`.
        run: RunId,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `step: StepIdx`.
        step: StepIdx,
        /// Mirror of `attempt: u16`.
        attempt: u16,
    },
    /// Mirror of `JournalEvent::AskAnsweredEvent` at events.rs:198-207.
    AskAnsweredEvent {
        /// Mirror of `run: RunId`.
        run: RunId,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `step: StepIdx`.
        step: StepIdx,
        /// Mirror of `attempt: u16`.
        attempt: u16,
    },
    /// Mirror of `JournalEvent::WaitResolvedEvent` at events.rs:213-222.
    WaitResolvedEvent {
        /// Mirror of `run: RunId`.
        run: RunId,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `step: StepIdx`.
        step: StepIdx,
        /// Mirror of `attempt: u16`.
        attempt: u16,
    },
    /// Mirror of `JournalEvent::RetryScheduledEvent` at events.rs:224-233.
    RetryScheduledEvent {
        /// Mirror of `run: RunId`.
        run: RunId,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `step: StepIdx`.
        step: StepIdx,
        /// Mirror of `attempt: u16`.
        attempt: u16,
    },
    /// Mirror of `JournalEvent::RunCancelled` at events.rs:235-244.
    RunCancelled {
        /// Mirror of `run: RunId`.
        run: RunId,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `attempt: u16`.
        attempt: u16,
        /// Mirror of `reason: Option<String>` (placeholder Option<()>).
        reason: Option<()>,
    },
    /// Mirror of `JournalEvent::RunKilled` at events.rs:246-253.
    RunKilled {
        /// Mirror of `run: RunId`.
        run: RunId,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `attempt: u16`.
        attempt: u16,
    },
    /// Mirror of `JournalEvent::RunFinished` at events.rs:255-264.
    RunFinished {
        /// Mirror of `run: RunId`.
        run: RunId,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `result: SlotIdx`.
        result: SlotIdx,
        /// Mirror of `attempt: u16`.
        attempt: u16,
    },
    /// Mirror of `JournalEvent::RunFailedEvent` at events.rs:266-273.
    RunFailedEvent {
        /// Mirror of `run: RunId`.
        run: RunId,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `attempt: u16`.
        attempt: u16,
    },
    /// Mirror of `JournalEvent::RunResumed` at events.rs:275-282.
    RunResumed {
        /// Mirror of `run: RunId`.
        run: RunId,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `timestamp: DateTime<Utc>` (placeholder unit).
        timestamp: (),
    },
    /// Mirror of `JournalEvent::RunRetried` at events.rs:284-291.
    RunRetried {
        /// Mirror of `run: RunId`.
        run: RunId,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `timestamp: DateTime<Utc>` (placeholder unit).
        timestamp: (),
    },
    /// Mirror of `JournalEvent::RunAnswered` at events.rs:293-304.
    RunAnswered {
        /// Mirror of `run: RunId`.
        run: RunId,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `slot_idx: SlotIdx`.
        slot_idx: SlotIdx,
        /// Mirror of `answer: ConstValue`.
        answer: ConstValue,
        /// Mirror of `timestamp: DateTime<Utc>` (placeholder unit).
        timestamp: (),
    },
    /// Mirror of `JournalEvent::AskTimedOutEvent` at events.rs:306-315.
    AskTimedOutEvent {
        /// Mirror of `run: RunId`.
        run: RunId,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `step: StepIdx`.
        step: StepIdx,
        /// Mirror of `attempt: u16`.
        attempt: u16,
    },
}

// ============================================================================
// TraceEntry / TraceStatus / TraceFilters — production-bound mirrors
// ============================================================================
//
// Mirror of `crates/vb_cli/src/commands_journal.rs:14-59`.

/// Mirror of production `TraceEntry`
/// (`crates/vb_cli/src/commands_journal.rs:14-24`).
///
/// Production derives: `Debug, Clone, PartialEq`. The mirror
/// preserves the same derives (no `Eq` because `serde_json::Value`
/// does not implement `Eq` in production).
#[derive(Clone, Debug, PartialEq)]
pub struct TraceEntry {
    /// Mirror of production `index: usize` (commands_journal.rs:16).
    pub index: usize,
    /// Mirror of production `event_type: &'static str` (commands_journal.rs:17).
    pub event_type: &'static str,
    /// Mirror of production `step: Option<u16>` (commands_journal.rs:18).
    pub step: Option<u16>,
    /// Mirror of production `status: Option<TraceStatus>` (commands_journal.rs:19).
    pub status: Option<TraceStatus>,
    /// Mirror of production `action: Option<u16>` (commands_journal.rs:20).
    pub action: Option<u16>,
    /// Mirror of production `seq: u64` (commands_journal.rs:21).
    pub seq: u64,
    /// Mirror of production `extra_json: Vec<(&'static str, serde_json::Value)>`
    /// (commands_journal.rs:23).
    pub extra_json: Vec<(&'static str, serde_json::Value)>,
}

/// Mirror of production `TraceStatus`
/// (`crates/vb_cli/src/commands_journal.rs:27-35`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TraceStatus {
    /// Mirror of `TraceStatus::Pending`.
    Pending,
    /// Mirror of `TraceStatus::Active`.
    Active,
    /// Mirror of `TraceStatus::WaitingAnswer`.
    WaitingAnswer,
    /// Mirror of `TraceStatus::Cancelled`.
    Cancelled,
    /// Mirror of `TraceStatus::Completed`.
    Completed,
    /// Mirror of `TraceStatus::Failed`.
    Failed,
}

impl TraceStatus {
    /// Mirror of `TraceStatus::as_str`
    /// (`crates/vb_cli/src/commands_journal.rs:38-48`).
    #[allow(dead_code)]
    pub const fn as_str(self) -> &'static str {
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

// ============================================================================
// mirror_trace_one — production-bound mirror of trace_one
// ============================================================================
//
// Mirror of `crates/vb_cli/src/commands_journal.rs:100-311`.
//
// The mirror body is line-by-line equivalent to the production body:
// each explicit match arm produces the same `event_type`, `step`,
// `status`, `action`, `seq`, and `extra_json_len` as production.
// The catch-all `_ =>` arm mirrors the production "Unknown" case.
//
// This fn is plain Rust (NOT inside `verus!`); Verus treats it as
// opaque. The companion spec file attaches a spec contract via
// `assume_specification[ mirror_trace_one ]` and discharges
// production-bound obligations through exec proofs.
#[allow(dead_code)]
pub fn mirror_trace_one(idx: usize, event: &MirrorJournalEvent) -> TraceEntry {
    match event {
        // Production: commands_journal.rs:102-113
        MirrorJournalEvent::RunAccepted { seq, run, workflow } => TraceEntry {
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
        // Production: commands_journal.rs:114-138
        MirrorJournalEvent::RunAdmission {
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
        // Production: commands_journal.rs:139-147
        MirrorJournalEvent::StepStarted { seq, step, .. } => TraceEntry {
            index: idx,
            event_type: "StepStarted",
            step: Some(step.get()),
            status: Some(TraceStatus::Active),
            action: None,
            seq: seq.get(),
            extra_json: vec![],
        },
        // Production: commands_journal.rs:148-158
        MirrorJournalEvent::StepSucceeded {
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
        // Production: commands_journal.rs:159-169
        MirrorJournalEvent::ActionScheduled {
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
        // Production: commands_journal.rs:170-180
        MirrorJournalEvent::ActionCompletedEvent {
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
        // Production: commands_journal.rs:181-191
        MirrorJournalEvent::ActionFailedEvent {
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
        // Production: commands_journal.rs:192-200
        MirrorJournalEvent::SlotWrittenEvent { seq, slot, .. } => TraceEntry {
            index: idx,
            event_type: "SlotWritten",
            step: None,
            status: Some(TraceStatus::Completed),
            action: None,
            seq: seq.get(),
            extra_json: vec![("slot", serde_json::Value::from(slot.get()))],
        },
        // Production: commands_journal.rs:201-209
        MirrorJournalEvent::WaitScheduledEvent { seq, step, .. } => TraceEntry {
            index: idx,
            event_type: "WaitScheduled",
            step: Some(step.get()),
            status: Some(TraceStatus::Active),
            action: None,
            seq: seq.get(),
            extra_json: vec![],
        },
        // Production: commands_journal.rs:210-218
        MirrorJournalEvent::AskScheduledEvent { seq, step, .. } => TraceEntry {
            index: idx,
            event_type: "AskScheduled",
            step: Some(step.get()),
            status: Some(TraceStatus::WaitingAnswer),
            action: None,
            seq: seq.get(),
            extra_json: vec![],
        },
        // Production: commands_journal.rs:219-227
        MirrorJournalEvent::AskAnsweredEvent { seq, step, .. } => TraceEntry {
            index: idx,
            event_type: "AskAnswered",
            step: Some(step.get()),
            status: Some(TraceStatus::Completed),
            action: None,
            seq: seq.get(),
            extra_json: vec![],
        },
        // Production: commands_journal.rs:228-236
        MirrorJournalEvent::RetryScheduledEvent { seq, step, .. } => TraceEntry {
            index: idx,
            event_type: "RetryScheduled",
            step: Some(step.get()),
            status: Some(TraceStatus::Active),
            action: None,
            seq: seq.get(),
            extra_json: vec![],
        },
        // Production: commands_journal.rs:237-245
        MirrorJournalEvent::RunCancelled { seq, .. } => TraceEntry {
            index: idx,
            event_type: "RunCancelled",
            step: None,
            status: Some(TraceStatus::Cancelled),
            action: None,
            seq: seq.get(),
            extra_json: vec![],
        },
        // Production: commands_journal.rs:246-254
        MirrorJournalEvent::RunFinished { seq, result, .. } => TraceEntry {
            index: idx,
            event_type: "RunFinished",
            step: None,
            status: Some(TraceStatus::Completed),
            action: None,
            seq: seq.get(),
            extra_json: vec![("result", serde_json::Value::from(result.get()))],
        },
        // Production: commands_journal.rs:255-263
        MirrorJournalEvent::RunFailedEvent { seq, .. } => TraceEntry {
            index: idx,
            event_type: "RunFailed",
            step: None,
            status: Some(TraceStatus::Failed),
            action: None,
            seq: seq.get(),
            extra_json: vec![],
        },
        // Production: commands_journal.rs:264-272
        MirrorJournalEvent::RunResumed { run, seq, .. } => TraceEntry {
            index: idx,
            event_type: "RunResumed",
            step: None,
            status: Some(TraceStatus::Active),
            action: None,
            seq: seq.get(),
            extra_json: vec![("run", serde_json::Value::from(run.get()))],
        },
        // Production: commands_journal.rs:273-281
        MirrorJournalEvent::RunRetried { run, seq, .. } => TraceEntry {
            index: idx,
            event_type: "RunRetried",
            step: None,
            status: Some(TraceStatus::Active),
            action: None,
            seq: seq.get(),
            extra_json: vec![("run", serde_json::Value::from(run.get()))],
        },
        // Production: commands_journal.rs:282-300
        MirrorJournalEvent::RunAnswered {
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
        // Production: commands_journal.rs:301-309 (catch-all `_ =>`)
        // Mirrors all 6 variants not explicitly handled:
        // ActionScheduledTicket, ActionCompletedEnvelope, ActionAbandoned,
        // WaitResolvedEvent, RunKilled, AskTimedOutEvent.
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

// ============================================================================
// mirror_build_trace — production-bound mirror of build_trace
// ============================================================================
//
// Mirror of `crates/vb_cli/src/commands_journal.rs:62-68`. Uses
// `mirror_trace_one` internally so any drift in `trace_one` cascades
// here.
#[allow(dead_code)]
pub fn mirror_build_trace(events: &[MirrorJournalEvent]) -> Vec<TraceEntry> {
    events
        .iter()
        .enumerate()
        .map(|(idx, event)| mirror_trace_one(idx, event))
        .collect()
}
