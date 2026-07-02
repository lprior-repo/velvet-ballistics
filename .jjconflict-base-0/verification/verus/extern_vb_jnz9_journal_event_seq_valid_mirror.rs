// SPDX-License-Identifier: MIT
// === companion to extern_vb_jnz9_journal_event_seq_valid.rs — split to clear source-length gate ===
//
// This file is the production-mirror body of the
// `extern_vb_jnz9_journal_event_seq_valid` extern surface. The split
// occurs at a clean verus!{...} block boundary: file 1 hosts the
// verus!-gated `#[path]` drift-detection inclusion only (a tiny
// 16-line verus! block), and this file hosts all the production
// structural mirrors (ID newtypes, ActionTicket, JournalEvent,
// support predicates) that the verus proofs in
// `vb_jnz9_journal_event_seq_valid.rs` consume via
// `production::MirrorXxx` references.
//
// The split is safe because:
//   - The verus! block in file 1 imports only `prod_src::*` types
//     from `production_inner/vb_jnz9_journal_event_seq_valid_production.rs`,
//     NOT any of the MirrorXxx types defined here.
//   - The MirrorXxx types here form a self-contained module:
//     they reference each other (e.g., MirrorJournalEvent uses
//     MirrorActionTicket), but they do not depend on the verus!
//     block or the prod_src types from file 1.
//   - The downstream spec file `vb_jnz9_journal_event_seq_valid.rs`
//     imports the MirrorXxx types via `pub use
//     production::MirrorXxx;` against the parent extern file's
//     module path. The parent file (extern_vb_jnz9_*.rs) does not
//     need to re-export them because the spec file resolves them
//     through the extern file's scope.
//
// Companion files:
//   - extern_vb_jnz9_journal_event_seq_valid.rs      (file 1, 177 lines: verus! + #[path] import)
//   - extern_vb_jnz9_journal_event_seq_valid_mirror.rs  (this file: 699 lines of Mirror structs)
//   - vb_jnz9_journal_event_seq_valid.rs             (downstream spec proofs, unchanged)

// ============================================================================
// ID newtype mirrors — vb_storage / vb_core production newtypes
// ============================================================================

/// Mirror of production `EventSeq` at
/// `crates/vb_storage/src/types.rs:73`.
///
/// Production declaration:
/// ```ignore
/// pub struct EventSeq(u64);
/// ```
///
/// `EventSeq` is `#[repr(transparent)]` over `u64` in production. The
/// mirror preserves the same shape: `EventSeq(pub u64)` with a public
/// `get() -> u64` accessor mirroring production line 84.
#[derive(Clone, Copy)]
pub struct EventSeq(pub u64);

impl EventSeq {
    /// Mirror of `EventSeq::new` at production types.rs:78-80.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Mirror of `EventSeq::get` at production types.rs:84-86.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Mirror of `EventSeq::MAX` at production types.rs:93.
    /// `EventSeq(u64::MAX)` is the overflow sentinel rejected by
    /// `JournalEvent::is_valid()` at production events.rs:519-522.
    pub const MAX: Self = Self(u64::MAX);

    /// Mirror of `EventSeq::ZERO` at production types.rs:89.
    pub const ZERO: Self = Self(0);
}

/// Mirror of production `RunId` at `crates/vb_core/src/ids/mod.rs:80`.
///
/// Production declaration: `pub struct RunId(u64);`. The mirror
/// preserves the same shape. `JournalEvent::is_valid()` rejects events
/// whose `run_id().get() == 0` at production events.rs:515-518.
#[derive(Clone, Copy)]
pub struct RunId(pub u64);

impl RunId {
    /// Mirror of `RunId::new` at production ids/mod.rs:67.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Mirror of `RunId::get` at production ids/mod.rs:70.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Mirror of `RunId::ZERO` at production ids/mod.rs.
    pub const ZERO: Self = Self(0);
}

/// Mirror of production `ActionTicket` at
/// `crates/vb_core/src/action/ticket.rs:6-21`.
///
/// Production declaration:
/// ```ignore
/// pub struct ActionTicket {
///     pub run: RunId,
///     pub step: StepIdx,
///     pub seq: SeqNo,
///     pub action: ActionId,
///     pub attempt: u16,
///     pub idempotency_key: u128,
///     pub capacity: u16,
/// }
/// ```
///
/// Only the `attempt: u16` field is read by `JournalEvent::is_valid()`
/// at production events.rs:527. The mirror preserves all field names
/// and types so any production drift in `ActionTicket` shape breaks
/// the mirror at compile time. Numeric primitives are used in place of
/// `RunId`/`StepIdx`/`SeqNo`/`ActionId` newtypes because
/// `JournalEvent::is_valid()` does not inspect them — the production
/// `is_valid()` body uses only `ticket.attempt != 0`.
#[derive(Clone, Copy)]
pub struct ActionTicket {
    /// Mirror of production field `run: RunId` (ticket.rs:8).
    pub run: u64,
    /// Mirror of production field `step: StepIdx` (ticket.rs:10).
    pub step: u16,
    /// Mirror of production field `seq: SeqNo` (ticket.rs:12).
    pub seq: u64,
    /// Mirror of production field `action: ActionId` (ticket.rs:14).
    pub action: u64,
    /// Mirror of production field `attempt: u16` (ticket.rs:16).
    /// This is the field read by `JournalEvent::is_valid()` at
    /// production events.rs:527.
    pub attempt: u16,
    /// Mirror of production field `idempotency_key: u128` (ticket.rs:18).
    pub idempotency_key: u128,
    /// Mirror of production field `capacity: u16` (ticket.rs:20).
    pub capacity: u16,
}

// ============================================================================
// MirrorJournalEvent — production-bound mirror of vb_storage::JournalEvent
// ============================================================================
//
// Production declaration at crates/vb_storage/src/events.rs:23-316.
//
// The 24-variant enum is mirrored line-by-line. Field names match
// production exactly. Field types are simplified to numeric primitives
// where `is_valid()` does not inspect the value (see binding ledger
// D1/D2 above). The fields `seq: EventSeq`, `run: u64` (extracted via
// `run_id()`), `attempt: u16` (for attempt-bearing variants), and
// `ticket.attempt: u16` (for ticket-bearing variants) are the only
// fields `is_valid()` reads; all other fields are present for
// structural parity (any drift in their names breaks the mirror at
// compile time) but their values are placeholders.
//
// Variant → field table (mirror types in parentheses; production
// fields annotated with their source lines):
//
//   - RunAccepted                { run: u64, seq: EventSeq, workflow: u64 }
//                                events.rs:25-32
//   - RunAdmission               { run: u64, seq: EventSeq, artifact_digest: u64,
//                                  granted_capabilities: (), policy: () }
//                                events.rs:34-45
//   - StepStarted                { run: u64, seq: EventSeq, step: u16,
//                                  attempt: u16 }
//                                events.rs:47-56
//   - StepSucceeded              { run: u64, seq: EventSeq, step: u16,
//                                  output: u16 }
//                                events.rs:58-67
//   - ActionScheduled            { run: u64, seq: EventSeq, step: u16,
//                                  action: u64, attempt: u16 }
//                                events.rs:69-80
//   - ActionCompletedEvent       { run: u64, seq: EventSeq, step: u16,
//                                  action: u64, attempt: u16 }
//                                events.rs:82-93
//   - ActionScheduledTicket      { run: u64, seq: EventSeq,
//                                  ticket: ActionTicket, input: u16,
//                                  output: u16 }
//                                events.rs:95-106
//   - ActionCompletedEnvelope    { run: u64, seq: EventSeq,
//                                  ticket: ActionTicket, output: u16,
//                                  outcome: u8, value: (), encoded_len: u32,
//                                  taint: u8, value_digest: () }
//                                events.rs:108-127
//   - ActionFailedEvent          { run: u64, seq: EventSeq, step: u16,
//                                  action: u64, attempt: u16 }
//                                events.rs:129-140
//   - ActionAbandoned            { run: u64, seq: EventSeq, ticket: ActionTicket }
//                                events.rs:148-158
//   - SlotWrittenEvent           { run: u64, seq: EventSeq, slot: u16,
//                                  value: Option<()>, extra: Option<()>,
//                                  attempt: u16 }
//                                events.rs:160-174
//   - WaitScheduledEvent         { run: u64, seq: EventSeq, step: u16,
//                                  attempt: u16 }
//                                events.rs:176-185
//   - AskScheduledEvent          { run: u64, seq: EventSeq, step: u16,
//                                  attempt: u16 }
//                                events.rs:187-196
//   - AskAnsweredEvent           { run: u64, seq: EventSeq, step: u16,
//                                  attempt: u16 }
//                                events.rs:198-207
//   - WaitResolvedEvent          { run: u64, seq: EventSeq, step: u16,
//                                  attempt: u16 }
//                                events.rs:213-222
//   - RetryScheduledEvent        { run: u64, seq: EventSeq, step: u16,
//                                  attempt: u16 }
//                                events.rs:224-233
//   - RunCancelled               { run: u64, seq: EventSeq, attempt: u16,
//                                  reason: Option<()> }
//                                events.rs:235-244
//   - RunKilled                  { run: u64, seq: EventSeq, attempt: u16 }
//                                events.rs:246-253
//   - RunFinished                { run: u64, seq: EventSeq, result: u16,
//                                  attempt: u16 }
//                                events.rs:255-264
//   - RunFailedEvent             { run: u64, seq: EventSeq, attempt: u16 }
//                                events.rs:266-273
//   - RunResumed                 { run: u64, seq: EventSeq, timestamp: () }
//                                events.rs:275-282
//   - RunRetried                 { run: u64, seq: EventSeq, timestamp: () }
//                                events.rs:284-291
//   - RunAnswered                { run: u64, seq: EventSeq, slot_idx: u16,
//                                  answer: (), timestamp: () }
//                                events.rs:293-304
//   - AskTimedOutEvent           { run: u64, seq: EventSeq, step: u16,
//                                  attempt: u16 }
//                                events.rs:306-315

/// Mirror of production `JournalEvent` enum.
#[derive(Clone)]
pub enum MirrorJournalEvent {
    /// Mirror of `JournalEvent::RunAccepted` at events.rs:25-32.
    RunAccepted {
        /// Mirror of `run: RunId`.
        run: u64,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `workflow: WorkflowDigest` (placeholder u64).
        workflow: u64,
    },
    /// Mirror of `JournalEvent::RunAdmission` at events.rs:34-45.
    RunAdmission {
        /// Mirror of `run: RunId`.
        run: u64,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `artifact_digest: WorkflowDigest` (placeholder u64).
        artifact_digest: u64,
        /// Mirror of `granted_capabilities: CapabilitySet` (placeholder u64).
        granted_capabilities: u64,
        /// Mirror of `policy: RuntimePolicy` (placeholder u64).
        policy: u64,
    },
    /// Mirror of `JournalEvent::StepStarted` at events.rs:47-56.
    StepStarted {
        /// Mirror of `run: RunId`.
        run: u64,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `step: StepIdx` (placeholder u16).
        step: u16,
        /// Mirror of `attempt: u16`.
        attempt: u16,
    },
    /// Mirror of `JournalEvent::StepSucceeded` at events.rs:58-67.
    StepSucceeded {
        /// Mirror of `run: RunId`.
        run: u64,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `step: StepIdx` (placeholder u16).
        step: u16,
        /// Mirror of `output: SlotIdx` (placeholder u16).
        output: u16,
    },
    /// Mirror of `JournalEvent::ActionScheduled` at events.rs:69-80.
    ActionScheduled {
        /// Mirror of `run: RunId`.
        run: u64,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `step: StepIdx` (placeholder u16).
        step: u16,
        /// Mirror of `action: ActionId` (placeholder u64).
        action: u64,
        /// Mirror of `attempt: u16`.
        attempt: u16,
    },
    /// Mirror of `JournalEvent::ActionCompletedEvent` at events.rs:82-93.
    ActionCompletedEvent {
        /// Mirror of `run: RunId`.
        run: u64,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `step: StepIdx` (placeholder u16).
        step: u16,
        /// Mirror of `action: ActionId` (placeholder u64).
        action: u64,
        /// Mirror of `attempt: u16`.
        attempt: u16,
    },
    /// Mirror of `JournalEvent::ActionScheduledTicket` at events.rs:95-106.
    ActionScheduledTicket {
        /// Mirror of `run: RunId`.
        run: u64,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `ticket: ActionTicket`.
        ticket: ActionTicket,
        /// Mirror of `input: SlotIdx` (placeholder u16).
        input: u16,
        /// Mirror of `output: SlotIdx` (placeholder u16).
        output: u16,
    },
    /// Mirror of `JournalEvent::ActionCompletedEnvelope` at events.rs:108-127.
    ActionCompletedEnvelope {
        /// Mirror of `run: RunId`.
        run: u64,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `ticket: ActionTicket`.
        ticket: ActionTicket,
        /// Mirror of `output: SlotIdx` (placeholder u16).
        output: u16,
        /// Mirror of `outcome: DurableActionOutcome` (placeholder u8).
        outcome: u8,
        /// Mirror of `value: Vec<u8>` (placeholder u64).
        value: u64,
        /// Mirror of `encoded_len: u32`.
        encoded_len: u32,
        /// Mirror of `taint: Taint` (placeholder u8).
        taint: u8,
        /// Mirror of `value_digest: [u8; 32]` (placeholder u64).
        value_digest: u64,
    },
    /// Mirror of `JournalEvent::ActionFailedEvent` at events.rs:129-140.
    ActionFailedEvent {
        /// Mirror of `run: RunId`.
        run: u64,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `step: StepIdx` (placeholder u16).
        step: u16,
        /// Mirror of `action: ActionId` (placeholder u64).
        action: u64,
        /// Mirror of `attempt: u16`.
        attempt: u16,
    },
    /// Mirror of `JournalEvent::ActionAbandoned` at events.rs:148-158.
    ActionAbandoned {
        /// Mirror of `run: RunId`.
        run: u64,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `ticket: ActionTicket`.
        ticket: ActionTicket,
    },
    /// Mirror of `JournalEvent::SlotWrittenEvent` at events.rs:160-174.
    SlotWrittenEvent {
        /// Mirror of `run: RunId`.
        run: u64,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `slot: SlotIdx` (placeholder u16).
        slot: u16,
        /// Mirror of `value: Option<Vec<u8>>` (placeholder Option<u64>).
        value: Option<u64>,
        /// Mirror of `extra: Option<Vec<u8>>` (placeholder Option<u64>).
        extra: Option<u64>,
        /// Mirror of `attempt: u16`.
        attempt: u16,
    },
    /// Mirror of `JournalEvent::WaitScheduledEvent` at events.rs:176-185.
    WaitScheduledEvent {
        /// Mirror of `run: RunId`.
        run: u64,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `step: StepIdx` (placeholder u16).
        step: u16,
        /// Mirror of `attempt: u16`.
        attempt: u16,
    },
    /// Mirror of `JournalEvent::AskScheduledEvent` at events.rs:187-196.
    AskScheduledEvent {
        /// Mirror of `run: RunId`.
        run: u64,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `step: StepIdx` (placeholder u16).
        step: u16,
        /// Mirror of `attempt: u16`.
        attempt: u16,
    },
    /// Mirror of `JournalEvent::AskAnsweredEvent` at events.rs:198-207.
    AskAnsweredEvent {
        /// Mirror of `run: RunId`.
        run: u64,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `step: StepIdx` (placeholder u16).
        step: u16,
        /// Mirror of `attempt: u16`.
        attempt: u16,
    },
    /// Mirror of `JournalEvent::WaitResolvedEvent` at events.rs:213-222.
    WaitResolvedEvent {
        /// Mirror of `run: RunId`.
        run: u64,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `step: StepIdx` (placeholder u16).
        step: u16,
        /// Mirror of `attempt: u16`.
        attempt: u16,
    },
    /// Mirror of `JournalEvent::RetryScheduledEvent` at events.rs:224-233.
    RetryScheduledEvent {
        /// Mirror of `run: RunId`.
        run: u64,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `step: StepIdx` (placeholder u16).
        step: u16,
        /// Mirror of `attempt: u16`.
        attempt: u16,
    },
    /// Mirror of `JournalEvent::RunCancelled` at events.rs:235-244.
    RunCancelled {
        /// Mirror of `run: RunId`.
        run: u64,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `attempt: u16`.
        attempt: u16,
        /// Mirror of `reason: Option<String>` (placeholder Option<u64>).
        reason: Option<u64>,
    },
    /// Mirror of `JournalEvent::RunKilled` at events.rs:246-253.
    RunKilled {
        /// Mirror of `run: RunId`.
        run: u64,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `attempt: u16`.
        attempt: u16,
    },
    /// Mirror of `JournalEvent::RunFinished` at events.rs:255-264.
    RunFinished {
        /// Mirror of `run: RunId`.
        run: u64,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `result: SlotIdx` (placeholder u16).
        result: u16,
        /// Mirror of `attempt: u16`.
        attempt: u16,
    },
    /// Mirror of `JournalEvent::RunFailedEvent` at events.rs:266-273.
    RunFailedEvent {
        /// Mirror of `run: RunId`.
        run: u64,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `attempt: u16`.
        attempt: u16,
    },
    /// Mirror of `JournalEvent::RunResumed` at events.rs:275-282.
    RunResumed {
        /// Mirror of `run: RunId`.
        run: u64,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `timestamp: DateTime<Utc>` (placeholder u64).
        timestamp: u64,
    },
    /// Mirror of `JournalEvent::RunRetried` at events.rs:284-291.
    RunRetried {
        /// Mirror of `run: RunId`.
        run: u64,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `timestamp: DateTime<Utc>` (placeholder u64).
        timestamp: u64,
    },
    /// Mirror of `JournalEvent::RunAnswered` at events.rs:293-304.
    RunAnswered {
        /// Mirror of `run: RunId`.
        run: u64,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `slot_idx: SlotIdx` (placeholder u16).
        slot_idx: u16,
        /// Mirror of `answer: ConstValue` (placeholder u64).
        answer: u64,
        /// Mirror of `timestamp: DateTime<Utc>` (placeholder u64).
        timestamp: u64,
    },
    /// Mirror of `JournalEvent::AskTimedOutEvent` at events.rs:306-315.
    AskTimedOutEvent {
        /// Mirror of `run: RunId`.
        run: u64,
        /// Mirror of `seq: EventSeq`.
        seq: EventSeq,
        /// Mirror of `step: StepIdx` (placeholder u16).
        step: u16,
        /// Mirror of `attempt: u16`.
        attempt: u16,
    },
}

// ============================================================================
// Production-bound mirror methods
// ============================================================================
//
// Each method body mirrors the corresponding production method body
// line-by-line. The mirror bodies are NOT verified by Verus (they are
// plain Rust fns outside `verus!`). The companion spec file
// (`vb_jnz9_journal_event_seq_valid.rs`) attaches production
// contracts via `assume_specification` bridges and exercises those
// contracts via spec proofs and exec proofs.

impl MirrorJournalEvent {
    /// Mirror of production `JournalEvent::run_id()` at
    /// `crates/vb_storage/src/events.rs:332-363`.
    ///
    /// Production body (verbatim, all 25 variant arms):
    /// ```ignore
    /// pub const fn run_id(&self) -> RunId {
    ///     match self {
    ///         Self::RunAccepted { run, .. }
    ///         | Self::RunAdmission { run, .. }
    ///         | Self::StepStarted { run, .. }
    ///         | ... (all 25 variants) => *run,
    ///     }
    /// }
    /// ```
    ///
    /// The mirror returns `RunId(run)` rather than `*run` so the
    /// match arms have a uniform return type. This matches
    /// production behavior: production `RunId` is a transparent u64
    /// newtype, so `RunId(run)` and `*run` are equivalent.
    #[must_use]
    #[verifier::external]
    pub const fn run_id(&self) -> RunId {
        match self {
            Self::RunAccepted { run, .. }
            | Self::RunAdmission { run, .. }
            | Self::StepStarted { run, .. }
            | Self::StepSucceeded { run, .. }
            | Self::ActionScheduled { run, .. }
            | Self::ActionCompletedEvent { run, .. }
            | Self::ActionScheduledTicket { run, .. }
            | Self::ActionCompletedEnvelope { run, .. }
            | Self::ActionFailedEvent { run, .. }
            | Self::ActionAbandoned { run, .. }
            | Self::SlotWrittenEvent { run, .. }
            | Self::WaitScheduledEvent { run, .. }
            | Self::AskScheduledEvent { run, .. }
            | Self::AskAnsweredEvent { run, .. }
            | Self::WaitResolvedEvent { run, .. }
            | Self::RetryScheduledEvent { run, .. }
            | Self::RunCancelled { run, .. }
            | Self::RunKilled { run, .. }
            | Self::RunFinished { run, .. }
            | Self::RunFailedEvent { run, .. }
            | Self::RunResumed { run, .. }
            | Self::RunRetried { run, .. }
            | Self::RunAnswered { run, .. }
            | Self::AskTimedOutEvent { run, .. } => RunId(*run),
        }
    }

    /// Mirror of production `JournalEvent::seq()` at
    /// `crates/vb_storage/src/events.rs:366-397`.
    #[must_use]
    #[verifier::external]
    pub const fn seq(&self) -> EventSeq {
        match self {
            Self::RunAccepted { seq, .. }
            | Self::RunAdmission { seq, .. }
            | Self::StepStarted { seq, .. }
            | Self::StepSucceeded { seq, .. }
            | Self::ActionScheduled { seq, .. }
            | Self::ActionCompletedEvent { seq, .. }
            | Self::ActionScheduledTicket { seq, .. }
            | Self::ActionCompletedEnvelope { seq, .. }
            | Self::ActionFailedEvent { seq, .. }
            | Self::ActionAbandoned { seq, .. }
            | Self::SlotWrittenEvent { seq, .. }
            | Self::WaitScheduledEvent { seq, .. }
            | Self::AskScheduledEvent { seq, .. }
            | Self::AskAnsweredEvent { seq, .. }
            | Self::WaitResolvedEvent { seq, .. }
            | Self::RetryScheduledEvent { seq, .. }
            | Self::RunCancelled { seq, .. }
            | Self::RunKilled { seq, .. }
            | Self::RunFinished { seq, .. }
            | Self::RunFailedEvent { seq, .. }
            | Self::RunResumed { seq, .. }
            | Self::RunRetried { seq, .. }
            | Self::RunAnswered { seq, .. }
            | Self::AskTimedOutEvent { seq, .. } => *seq,
        }
    }

    /// Mirror of production `JournalEvent::is_valid()` at
    /// `crates/vb_storage/src/events.rs:514-550`.
    ///
    /// Production body (verbatim):
    /// ```ignore
    /// pub const fn is_valid(&self) -> bool {
    ///     // RunId(0) is the zero/placeholder value - valid events must have a real run
    ///     if self.run_id().get() == 0 {
    ///         return false;
    ///     }
    ///     // Sequence must not be at the max value (overflow sentinel)
    ///     if self.seq().get() == u64::MAX {
    ///         return false;
    ///     }
    ///     // Attempt numbers must be non-zero when present (zero is ambiguous)
    ///     match self {
    ///         Self::ActionScheduled { attempt, .. }
    ///         | Self::ActionCompletedEvent { attempt, .. }
    ///         | Self::ActionFailedEvent { attempt, .. }
    ///         | Self::SlotWrittenEvent { attempt, .. }
    ///         | Self::WaitScheduledEvent { attempt, .. }
    ///         | Self::AskScheduledEvent { attempt, .. }
    ///         | Self::AskAnsweredEvent { attempt, .. }
    ///         | Self::WaitResolvedEvent { attempt, .. }
    ///         | Self::RetryScheduledEvent { attempt, .. }
    ///         | Self::StepStarted { attempt, .. }
    ///         | Self::RunCancelled { attempt, .. }
    ///         | Self::RunKilled { attempt, .. }
    ///         | Self::RunFinished { attempt, .. }
    ///         | Self::RunFailedEvent { attempt, .. }
    ///         | Self::AskTimedOutEvent { attempt, .. } => *attempt != 0,
    ///         Self::ActionScheduledTicket { ticket, .. }
    ///         | Self::ActionCompletedEnvelope { ticket, .. }
    ///         | Self::ActionAbandoned { ticket, .. } => ticket.attempt != 0,
    ///         Self::RunAccepted { .. }
    ///         | Self::RunAdmission { .. }
    ///         | Self::StepSucceeded { .. }
    ///         | Self::RunResumed { .. }
    ///         | Self::RunRetried { .. }
    ///         | Self::RunAnswered { .. } => true,
    ///     }
    /// }
    /// ```
    ///
    /// Three decision branches:
    ///   1. `run_id == 0`           → reject (events.rs:515-518)
    ///   2. `seq == u64::MAX`       → reject (events.rs:519-522)
    ///   3. attempt-bearing variants → `attempt != 0` (events.rs:509-524)
    ///      ticket-bearing variants  → `ticket.attempt != 0` (events.rs:525-527)
    ///      no-field variants        → `true` (events.rs:528-533)
    #[must_use]
    #[verifier::external]
    pub const fn is_valid(&self) -> bool {
        // RunId(0) is the zero/placeholder value - valid events must have a real run
        if self.run_id().get() == 0 {
            return false;
        }
        // Sequence must not be at the max value (overflow sentinel)
        if self.seq().get() == u64::MAX {
            return false;
        }
        // Attempt numbers must be non-zero when present (zero is ambiguous)
        match self {
            Self::ActionScheduled { attempt, .. }
            | Self::ActionCompletedEvent { attempt, .. }
            | Self::ActionFailedEvent { attempt, .. }
            | Self::SlotWrittenEvent { attempt, .. }
            | Self::WaitScheduledEvent { attempt, .. }
            | Self::AskScheduledEvent { attempt, .. }
            | Self::AskAnsweredEvent { attempt, .. }
            | Self::WaitResolvedEvent { attempt, .. }
            | Self::RetryScheduledEvent { attempt, .. }
            | Self::StepStarted { attempt, .. }
            | Self::RunCancelled { attempt, .. }
            | Self::RunKilled { attempt, .. }
            | Self::RunFinished { attempt, .. }
            | Self::RunFailedEvent { attempt, .. }
            | Self::AskTimedOutEvent { attempt, .. } => *attempt != 0,
            Self::ActionScheduledTicket { ticket, .. }
            | Self::ActionCompletedEnvelope { ticket, .. }
            | Self::ActionAbandoned { ticket, .. } => ticket.attempt != 0,
            Self::RunAccepted { .. }
            | Self::RunAdmission { .. }
            | Self::StepSucceeded { .. }
            | Self::RunResumed { .. }
            | Self::RunRetried { .. }
            | Self::RunAnswered { .. } => true,
        }
    }
}

// ============================================================================
// Pure decision fns — line-by-line production decision lattice
// ============================================================================
//
// These are direct lifts of the production decision branches in
// `is_valid()` at events.rs:514-550. Each fn takes the relevant
// field values and returns the decision the production code would
// make. The spec proofs and exec proofs in the companion spec file
// use these to reason about the production behavior.

/// Production decision at events.rs:515-518:
///   `if self.run_id().get() == 0 { return false; }`
#[must_use]
pub const fn is_valid_run_id_zero(run_id_value: u64) -> bool {
    run_id_value != 0
}

/// Production decision at events.rs:519-522:
///   `if self.seq().get() == u64::MAX { return false; }`
#[must_use]
pub const fn is_valid_seq_overflow(seq_value: u64) -> bool {
    seq_value != u64::MAX
}

/// Production decision at events.rs:523-549 for attempt-bearing
/// variants (the union of all variants whose match arm reads
/// `*attempt != 0` or `ticket.attempt != 0`).
#[must_use]
pub const fn is_valid_attempt_nonzero(attempt: u16) -> bool {
    attempt != 0
}
