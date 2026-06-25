//! Standalone Verus model for vb_storage journal replay seam contracts.
//!
//! Obligation: VB-STORAGE-REPLAY-001 (events_for_run seam contracts)
//!
//! This module is intentionally self-contained because the registry runner invokes
//! it with standalone `verus --crate-type=lib`, without Cargo crate resolution for
//! production crates. It mirrors the replay properties expected of:
//!   - crates/vb_storage/src/journal/replay.rs::FjallJournal::events_for_run
//!   - crates/vb_storage/src/journal/replay.rs::FjallJournal::events_for_run_from
//!
//! It does not bind to those production functions in this standalone form; no
//! external production imports or trusted function specifications are declared
//! here. Production binding remains separate proof debt outside this artifact.
//!
//! Production concepts mirrored:
//!   - vb_core::RunId  (u64-based numeric identifier)
//!   - vb_storage::EventSeq  (u64-based per-run event sequence)
//!   - vb_storage::JournalEvent  (run admission, step, action, slot events)
//!   - vb_storage::JournalError  (WrongRun, SequenceGap, BadMagic, etc.)
//!
//! Contracts:
//!   events_for_run(run):
//!     requires: run is any valid RunId (RunId::new is total)
//!     ensures:  latest snapshot authority failures return typed errors;
//!               if Ok(events), then all events have run_id == run and the
//!               first event, when present, starts at snapshot seq + 1 or 0
//!   events_for_run_from(run, start_seq):
//!     requires: run is any valid RunId, start_seq is any valid EventSeq
//!     ensures:  if Ok(events), then all events have run_id == run
//!               first returned event equals start_seq when present
//!               sequences are strictly increasing by 1
use vstd::prelude::*;

verus! {

// ============================================================
// Spec mirror types (verification-only abstractions)
// ============================================================
// Spec mirror of EventSeq (crates/vb_storage/src/types.rs)
pub struct SpecEventSeq {
    pub value: int,
}

impl SpecEventSeq {
    pub open spec fn value_valid(self) -> bool {
        self.value >= 0
    }

    pub open spec fn into_int(self) -> int {
        self.value
    }
}

// Spec mirror of RunId (crates/vb_core/src/ids/mod.rs)
pub struct SpecRunId {
    pub value: int,
}

impl SpecRunId {
    pub open spec fn value_valid(self) -> bool {
        self.value >= 0
    }
}

// ============================================================
// Spec mirror of JournalEvent (subset of crates/vb_storage/src/events.rs)
// We model only the fields relevant to replay contract verification
// ============================================================
pub enum SpecJournalEventKind {
    RunAccepted,
    RunAdmission,
    StepStarted,
    StepSucceeded,
    ActionScheduled,
    ActionCompleted,
    ActionFailed,
    SlotWritten,
    WaitScheduled,
    AskScheduled,
    AskAnswered,
    RetryScheduled,
    RunCancelled,
    RunFinished,
    RunFailed,
    RunResumed,
    RunRetried,
    RunAnswered,
}

pub struct SpecJournalEvent {
    pub run_id: SpecRunId,
    pub seq: SpecEventSeq,
    pub kind: SpecJournalEventKind,
}

impl SpecJournalEvent {
    pub open spec fn run_id_valid(self) -> bool {
        self.run_id.value_valid()
    }

    pub open spec fn seq_valid(self) -> bool {
        self.seq.value_valid()
    }

    pub open spec fn well_formed(self) -> bool {
        &&& self.run_id_valid()
        &&& self.seq_valid()
    }
}

// ============================================================
// JournalError spec variants relevant to replay
// ============================================================
pub enum SpecJournalErrorKind {
    Fjall,
    Encode,
    KeyCapacity,
    DuplicateEvent,
    WriteLockPoisoned,
    QueueCapacity,
    QueueFull,
    QueueShutdown,
    WrongRun,
    SequenceGap,
    SequenceOverflow,
    BadMagic,
    UnsupportedSchemaVersion,
    MigrationRequired,
    UnknownRecordKind,
    RecordKindFamilyMismatch,
    PayloadDigestMismatch,
    PostcardDecodeFailed,
    PayloadTooLarge,
    TooManyEvents,
    ReplayAllocationFailed,
    StrictDurabilityFailed,
}

pub enum SpecSnapshotStatus {
    NoSnapshot,
    ValidSnapshot,
    BadMagic,
    PayloadDigestMismatch,
    PostcardDecodeFailed,
    WrongRun,
    WrongSeq,
}

pub open spec fn spec_next_seq(seq: SpecEventSeq, max_seq: int) -> Result<
    SpecEventSeq,
    SpecJournalErrorKind,
> {
    if seq.value < max_seq {
        Ok(SpecEventSeq { value: seq.value + 1 })
    } else {
        Err(SpecJournalErrorKind::SequenceOverflow)
    }
}

pub open spec fn snapshot_authority_result(
    status: SpecSnapshotStatus,
    snapshot_seq: SpecEventSeq,
    max_seq: int,
) -> Result<SpecEventSeq, SpecJournalErrorKind> {
    match status {
        SpecSnapshotStatus::NoSnapshot => Ok(SpecEventSeq { value: 0 }),
        SpecSnapshotStatus::ValidSnapshot => spec_next_seq(snapshot_seq, max_seq),
        SpecSnapshotStatus::BadMagic => Err(SpecJournalErrorKind::BadMagic),
        SpecSnapshotStatus::PayloadDigestMismatch => Err(
            SpecJournalErrorKind::PayloadDigestMismatch,
        ),
        SpecSnapshotStatus::PostcardDecodeFailed => Err(SpecJournalErrorKind::PostcardDecodeFailed),
        SpecSnapshotStatus::WrongRun => Err(SpecJournalErrorKind::WrongRun),
        SpecSnapshotStatus::WrongSeq => Err(SpecJournalErrorKind::SequenceGap),
    }
}

// ============================================================
// Contract: events_for_run
// crates/vb_storage/src/journal/replay.rs::FjallJournal::events_for_run
// ============================================================
// events_for_run(run) first validates latest snapshot authority. Snapshot
// failures are not erased. A valid snapshot at sequence N delegates to replay
// from N + 1; no snapshot delegates from zero. If Ok(events), the first event
// when present must equal the delegated start sequence exactly.
//
// Intended production correspondence, not mechanically proved in this
// standalone target:
//   snapshot_seq = latest_durable_snapshot_seq(run)?
//   start_seq = snapshot_seq.map_or(Ok(EventSeq::ZERO), codec::next_seq)?
//   events_for_run_from(run, start_seq, limit)
//
// Model relation to audit against production code:
//   snapshot_authority_result -> trimming::latest_durable_snapshot_seq +
//                                codec::next_seq in journal/replay.rs
//   spec_events_for_run_from_contract -> validate_replay_sequence.
pub open spec fn spec_events_for_run_contract(
    run: SpecRunId,
    snapshot_status: SpecSnapshotStatus,
    snapshot_seq: SpecEventSeq,
    max_seq: int,
    result: Result<Seq<SpecJournalEvent>, SpecJournalErrorKind>,
) -> bool {
    match snapshot_authority_result(snapshot_status, snapshot_seq, max_seq) {
        Ok(start_seq) => spec_events_for_run_from_contract(run, start_seq, result),
        Err(error) => result == Err::<Seq<SpecJournalEvent>, SpecJournalErrorKind>(error),
    }
}

// ============================================================
// Contract: events_for_run_from
// crates/vb_storage/src/journal/replay.rs::FjallJournal::events_for_run_from
// ============================================================
// If events_for_run_from(run, start_seq) returns Ok(events), then:
//   1. All events have run_id == run
//   2. The first returned event, when present, has seq == start_seq
//   3. Sequences are strictly increasing by 1
pub open spec fn spec_events_for_run_from_contract(
    run: SpecRunId,
    start_seq: SpecEventSeq,
    result: Result<Seq<SpecJournalEvent>, SpecJournalErrorKind>,
) -> bool {
    match result {
        Ok(events) => {
            &&& (forall|i: int|
                0 <= i && i < events.len() as int ==> #[trigger] events[i as int].run_id == run)
            &&& (events.len() as int > 0 ==> events[0].seq.value == start_seq.value)
            &&& (forall|i: int|
                0 <= i && i < events.len() as int - 1 ==> #[trigger] events[i as int].seq.value + 1
                    == events[(i + 1) as int].seq.value)
        },
        Err(_) => true,
    }
}

// ============================================================
// Proof: events_for_run_from contract implies sequence strict ordering
// ============================================================
pub proof fn proof_events_for_run_from_strict_ordering(
    run: SpecRunId,
    start_seq: SpecEventSeq,
    events: Seq<SpecJournalEvent>,
)
    requires
        spec_events_for_run_from_contract(run, start_seq, Ok(events)),
        events.len() as int >= 2,
    ensures
        (forall|i: int|
            0 <= i && i < events.len() as int - 1 ==> #[trigger] events[i as int].seq.value
                < events[(i + 1) as int].seq.value),
{
    reveal(spec_events_for_run_from_contract);
}

// ============================================================
// Proof: events_for_run_from preserves run identity
// ============================================================
pub proof fn proof_events_for_run_from_run_preserved(
    run: SpecRunId,
    start_seq: SpecEventSeq,
    events: Seq<SpecJournalEvent>,
)
    requires
        spec_events_for_run_from_contract(run, start_seq, Ok(events)),
    ensures
        forall|i: int|
            0 <= i && i < events.len() as int ==> #[trigger] events[i as int].run_id == run,
{
    assert_forall_by(
        |i: int|
            {
                requires(0 <= i && i < events.len() as int);
                ensures(#[trigger] events[i as int].run_id == run);
                reveal(spec_events_for_run_from_contract);
            },
    );
}

// ============================================================
// Proof: events_for_run_from starts exactly at start_seq when non-empty
// ============================================================
pub proof fn proof_events_for_run_from_first_event_matches_start(
    run: SpecRunId,
    start_seq: SpecEventSeq,
    events: Seq<SpecJournalEvent>,
)
    requires
        spec_events_for_run_from_contract(run, start_seq, Ok(events)),
        events.len() as int > 0,
    ensures
        events[0].seq.value == start_seq.value,
{
    reveal(spec_events_for_run_from_contract);
}

// ============================================================
// Proof: events_for_run contract is satisfied by events_for_run_from contract
// when snapshot authority computes the exact start sequence
// ============================================================
pub proof fn proof_events_for_run_subsumes_events_for_run_from(
    run: SpecRunId,
    snapshot_status: SpecSnapshotStatus,
    snapshot_seq: SpecEventSeq,
    max_seq: int,
    start_seq: SpecEventSeq,
    events: Seq<SpecJournalEvent>,
)
    requires
        snapshot_authority_result(snapshot_status, snapshot_seq, max_seq) == Ok::<
            SpecEventSeq,
            SpecJournalErrorKind,
        >(start_seq),
        spec_events_for_run_from_contract(run, start_seq, Ok(events)),
    ensures
        spec_events_for_run_contract(run, snapshot_status, snapshot_seq, max_seq, Ok(events)),
{
    reveal(spec_events_for_run_from_contract);
    reveal(spec_events_for_run_contract);
    reveal(snapshot_authority_result);
}

// ============================================================
// Proof: snapshot authority failures are propagated as typed replay errors
// ============================================================
pub proof fn proof_events_for_run_propagates_snapshot_error(
    run: SpecRunId,
    snapshot_status: SpecSnapshotStatus,
    snapshot_seq: SpecEventSeq,
    max_seq: int,
    error: SpecJournalErrorKind,
)
    requires
        snapshot_authority_result(snapshot_status, snapshot_seq, max_seq) == Err::<
            SpecEventSeq,
            SpecJournalErrorKind,
        >(error),
    ensures
        spec_events_for_run_contract(run, snapshot_status, snapshot_seq, max_seq, Err(error)),
{
    reveal(spec_events_for_run_contract);
    reveal(snapshot_authority_result);
}

} // verus!
fn main() {}
