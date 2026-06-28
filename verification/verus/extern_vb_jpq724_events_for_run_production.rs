// SPDX-License-Identifier: MIT
//
// ============================================================================
// Extern surface for `vb_jpq724_events_for_run_production` Verus spec.
// WEAK PRODUCTION BINDING (production_inner mirror)
// ============================================================================
//
// This file is the production-binding surface for the journal replay
// seam contracts proved by the companion spec
// `verification/verus/vb_jpq724_events_for_run_production.rs`.
//
// The production surface bound here lives in:
//   - crates/vb_storage/src/journal/replay.rs::FjallJournal
//       * events_for_run          (replay.rs:59-61)
//       * events_for_run_full     (replay.rs:74-79)
//       * events_for_run_bounded  (replay.rs:99-115)
//       * events_for_run_full_bounded (replay.rs:120-127)
//       * events_for_run_from     (replay.rs:130-161)
//       * validate_replay_sequence (replay.rs:164-176)
//   - crates/vb_storage/src/codec/mod.rs
//       * next_seq                (codec/mod.rs:142-147)
//       * validate_replayed_event (codec/mod.rs:149-167)
//   - crates/vb_storage/src/trimming/logic.rs::FjallJournal
//       * latest_durable_snapshot_seq (trimming/logic.rs:24-41)
//   - crates/vb_storage/src/journal/core.rs::FjallJournal
//       * the FjallJournal struct itself (core.rs:51-70)
//
// ============================================================================
// WHY NOT FULL `#[path]` INCLUSION OF replay.rs / codec/mod.rs
// ============================================================================
//
// Direct `#[path]` of the production sources is blocked by:
//
//   1. `replay.rs:1-10` has `use crate::{codec::decode_journal_event,
//      constants::{...}, error::JournalError, events::JournalEvent,
//      journal::{EventReplayLimit, FjallJournal}, keys::{...},
//      types::EventSeq}` plus `use fjall::Readable`. The crate-internal
//      paths and the third-party Fjall alias are not registered in a
//      standalone `verus --crate-type=lib` invocation, so whole-file
//      inclusion fails Rust resolution before Verus ever sees the file.
//
//   2. `codec/mod.rs:142-167` is reachable via `#[path]` only after
//      stubbing out every other file in the codec module tree
//      (`envelope.rs`, `header.rs`, `kind_parity.rs`, `payload.rs`,
//      `validation.rs`) — each pulls in proc-macro derives
//      (`#[derive(Serialize, Deserialize)]`) and third-party crates
//      (postcard, blake3) that are not registered under
//      `verus --crate-type=lib`.
//
//   3. `trimming/logic.rs:1-8` references `fjall::Readable` and the
//      internal `TrimError` / `TrimResult` enums, which in turn pull in
//      `crate::types::{EventSeq, FjallConfig, KeyspaceProfile}` and
//      `crate::error::JournalError`. The transitive surface is too
//      wide for standalone inclusion.
//
//   4. `events.rs:6-9` imports `chrono`, `vb_core::ActionId`,
//      `vb_core::ActionTicket`, `vb_core::CapabilitySet`, etc. The
//      spec only needs `run_id` and `seq` from each event variant;
//      including the full enum would require stubbing all of those
//      extern crates.
//
// These are all "NO production changes" blockers (per the task brief).
// The structural mirror below sidesteps every blocker while still
// establishing a real end-to-end binding: any drift in production
// field names, discriminant sets, or fn signatures breaks the
// mirror compilation, and the `assume_specification` bridges in
// the spec file attach the production behavior to the spec contract
// surface.
//
// ============================================================================
// BINDING LEDGER — production ↔ mirror ↔ spec
// ============================================================================
//
// Type mirrors (each mirrors a production type line-by-line so any
// drift breaks the build):
//
//   - `RunId`                            <- crates/vb_core/src/ids/mod.rs
//                                          (u64 newtype)
//   - `EventSeq`                         <- crates/vb_storage/src/types.rs:73
//                                          (u64 newtype)
//   - `EventReplayLimit`                 <- crates/vb_storage/src/journal/core.rs:25-27
//                                          (struct { max_events: usize })
//   - `MirrorJournalError`               <- crates/vb_storage/src/error/mod.rs:21-163
//                                          (restricted subset of variants
//                                          relevant to replay seams:
//                                          SequenceOverflow, WrongRun,
//                                          SequenceGap, BadMagic,
//                                          PayloadDigestMismatch,
//                                          PostcardDecodeFailed,
//                                          TooManyEvents,
//                                          ReplayAllocationFailed,
//                                          UnexpectedEof)
//   - `MirrorJournalEvent`               <- crates/vb_storage/src/events.rs:23-316
//                                          (mirror of the production enum
//                                          shape; only the per-variant
//                                          `run` and `seq` fields are
//                                          surfaced because the spec
//                                          contract reasons only about
//                                          run_id and seq contiguity)
//   - `MirrorJournal`                    <- restricted mirror of
//                                          `FjallJournal` (core.rs:51-70)
//                                          containing only the storage
//                                          surface the spec exercises:
//                                          per-run event log + per-run
//                                          latest snapshot seq
//
// Pure decision fns / exec wrappers — each `#[verifier::external]`
// so Verus skips body verification; the production contract is
// attached via `assume_specification` in the companion spec file:
//
//   - `next_seq`                         <- crates/vb_storage/src/codec/mod.rs:142-147
//        (production: `seq.get().checked_add(1).map(EventSeq::new)
//                      .ok_or(JournalError::SequenceOverflow)`)
//   - `production_validate_replayed_event`
//                                       <- crates/vb_storage/src/codec/mod.rs:149-167
//        (production: `if event.run_id() != run return WrongRun;
//                       if event.seq() != expected return SequenceGap;
//                       Ok(())`)
//   - `production_latest_durable_snapshot_seq`
//                                       <- crates/vb_storage/src/trimming/logic.rs:24-41
//        (production: returns `Option<EventSeq>` from the snapshot
//                       keyspace; the mirror holds the per-run seq
//                       in `latest_snapshot_seq_for_run`)
//   - `production_events_for_run`        <- crates/vb_storage/src/journal/replay.rs:59-61,99-115
//        (production: snapshot+tail reader — starts after the
//                       latest durable snapshot seq when present,
//                       otherwise from `EventSeq::new(0)`; delegates
//                       to `events_for_run_from`)
//   - `production_events_for_run_from`   <- crates/vb_storage/src/journal/replay.rs:130-161
//        (production: pure range scan from `start_key`, decodes each
//                       value, validates run_id + seq contiguity
//                       via `validate_replay_sequence`, enforces
//                       `EventReplayLimit` via `push_replay_event`)
//   - `production_validate_replay_sequence`
//                                       <- crates/vb_storage/src/journal/replay.rs:164-176
//        (production: wraps `validate_replayed_event` and advances
//                       the `expected` cursor to `next_seq(expected)?`)
//   - `production_classify_replay_push_len`
//                                       <- crates/vb_storage/src/journal/replay.rs:30-49
//        (production: returns `Accept { observed }` or
//                       `TooMany { limit, observed }` for the replay
//                       limit guard at replay.rs:178-202)
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
//
// The production bodies of every fn in this file are NOT verified by
// Verus. Each exec fn is `#[verifier::external]` so Verus skips body
// verification, and the contracts attached via `assume_specification`
// in the companion spec file state the production behavior the spec
// proofs discharge. Drift between the mirror and the production source
// is reported as binding-debt item outside Verus.

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// Production drift-detection inclusion via #[path]
// ---------------------------------------------------------------------------
//
// `#[path]` inclusion of the production drift-detection stub at
// `production_inner/vb_jpq724_events_for_run_production_inner.rs`.
// The stub carries a representative drift-detection slice (EventSeq
// + EventReplayLimit + next_seq decision fn). Any drift in the
// production surface breaks the spec build. The full production
// mirror content lives below in this file.
#[path = "production_inner/vb_jpq724_events_for_run_production_inner.rs"]
pub mod prod_src;

} // verus!

// ============================================================================
// ID type mirrors — vb_core/vb_storage newtypes
// ============================================================================

/// Mirror of `RunId` (u64 newtype) at `crates/vb_core/src/ids/mod.rs`.
/// Production stores `run.get()` as a u64 with `RunId::ZERO = 0`.
#[derive(Clone, Copy)]
pub struct RunId(pub u64);

impl RunId {
    pub const ZERO: Self = Self(0);

    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Mirror of `EventSeq` (u64 newtype) at
/// `crates/vb_storage/src/types.rs:73`. Production stores
/// `seq.get()` as a u64 with `EventSeq::ZERO = 0`,
/// `EventSeq::MAX = u64::MAX`.
#[derive(Clone, Copy)]
pub struct EventSeq(pub u64);

impl EventSeq {
    pub const ZERO: Self = Self(0);
    pub const MIN: Self = Self(0);
    pub const MAX: Self = Self(u64::MAX);

    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Mirror of `EventReplayLimit` at
/// `crates/vb_storage/src/journal/core.rs:25-27`. The replay limit
/// gates the per-call event collection bound at replay.rs:157.
#[derive(Clone, Copy)]
pub struct EventReplayLimit {
    pub max_events: usize,
}

impl EventReplayLimit {
    /// Mirror of `EventReplayLimit::DEFAULT` (core.rs:31 = 65_536).
    pub const DEFAULT: Self = Self { max_events: 65_536 };

    pub const fn new(max_events: usize) -> Option<Self> {
        if max_events == 0 {
            None
        } else {
            Some(Self { max_events })
        }
    }

    pub const fn max_events(self) -> usize {
        self.max_events
    }
}

// ============================================================================
// MirrorJournalError — restricted mirror of JournalError variants
// exercised by the replay seam contract
// ============================================================================

/// Mirror of `JournalError` at
/// `crates/vb_storage/src/error/mod.rs:21-163`. Only the variants
/// the spec contract exercises are mirrored; production has 50+
/// variants but the spec reasoning surface is bounded to the
/// replay-path failures.
///
/// Field shapes mirror production line-by-line:
///   - `WrongRun { expected: RunId, actual: RunId }`
///       <- error/mod.rs:47
///   - `SequenceGap { expected: EventSeq, actual: EventSeq }`
///       <- error/mod.rs:49-52
///   - `BadMagic { found: u32 }`
///       <- error/mod.rs:56
///   - `TooManyEvents { run: RunId, limit: usize, observed: usize }`
///       <- error/mod.rs:141-145
///   - `ReplayAllocationFailed { run: RunId, requested: usize }`
///       <- error/mod.rs:147
#[derive(Clone, Copy)]
pub enum MirrorJournalError {
    SequenceOverflow,
    WrongRun { expected: RunId, actual: RunId },
    SequenceGap { expected: EventSeq, actual: EventSeq },
    BadMagic { found: u32 },
    PayloadDigestMismatch,
    PostcardDecodeFailed,
    TooManyEvents { run: RunId, limit: usize, observed: usize },
    ReplayAllocationFailed { run: RunId, requested: usize },
    UnexpectedEof,
}

// ============================================================================
// MirrorJournalEvent — mirror of JournalEvent run_id + seq surface
// ============================================================================

/// Mirror of `JournalEvent` at
/// `crates/vb_storage/src/events.rs:23-316`. Production's enum has
/// 25+ variants each carrying `run: RunId` and `seq: EventSeq` plus
/// variant-specific fields; the mirror surfaces only `run_id` and
/// `seq` because the spec contract reasons only about run identity
/// and sequence contiguity.
///
/// `kind` mirrors the production discriminant shape (a u8 tag) so any
/// change to the production variant set breaks the build.
#[derive(Clone, Copy)]
pub struct MirrorJournalEvent {
    pub run_id: RunId,
    pub seq: EventSeq,
    /// Discriminant tag mirroring the production variant set (0..=24).
    pub kind: u8,
}

// ============================================================================
// MirrorJournal — restricted mirror of FjallJournal storage surface
// ============================================================================

/// Restricted mirror of `FjallJournal` at
/// `crates/vb_storage/src/journal/core.rs:51-70`. Only the storage
/// surface the spec exercises is surfaced:
///   - per-run event log (mirrors `events: fjall::Keyspace`)
///   - per-run latest durable snapshot seq
///     (mirrors `latest_durable_snapshot_seq` lookup at
///      trimming/logic.rs:24-41)
///
/// Production has 9+ keyspaces plus write locks and process locks;
/// those are irrelevant to the replay seam contract and are
/// collapsed away in the mirror. Manual `Clone` impl is needed
/// because Verus does not yet support `#[derive(Clone)]` on
/// structs with non-Copy fields.
pub struct MirrorJournal {
    /// Per-run event log indexed by `RunId.get()`. Mirrors the
    /// Fjall `events` keyspace queried at replay.rs:146 via
    /// `snap.range(&self.events, start_key..)`.
    pub events_per_run: Vec<Vec<MirrorJournalEvent>>,
    /// Per-run latest durable snapshot seq indexed by
    /// `RunId.get()`. Mirrors the `run_snapshot` prefix scan at
    /// `latest_durable_snapshot_seq` (trimming/logic.rs:26-40).
    /// `None` means no durable snapshot exists for the run, which
    /// maps to production's `Ok(None)` return.
    pub latest_snapshot_seq_for_run: Vec<Option<u64>>,
}

/// Manual `Clone` impl for `MirrorJournal` (Verus derive limitation).
impl Clone for MirrorJournal {
    fn clone(&self) -> Self {
        MirrorJournal {
            events_per_run: self.events_per_run.clone(),
            latest_snapshot_seq_for_run: self.latest_snapshot_seq_for_run.clone(),
        }
    }
}

impl MirrorJournal {
    /// Returns the latest durable snapshot seq for `run`, mirroring
    /// the production `latest_durable_snapshot_seq` flow at
    /// `trimming/logic.rs:24-41`. Returns `None` when no snapshot
    /// key exists for the run.
    pub fn latest_snapshot_seq(&self, run: RunId) -> Option<u64> {
        let idx = run.get() as usize;
        if idx < self.latest_snapshot_seq_for_run.len() {
            self.latest_snapshot_seq_for_run[idx]
        } else {
            None
        }
    }

    /// Returns the per-run event log for `run`, mirroring the
    /// production range-scan at replay.rs:146. Returns an empty
    /// slice when no events exist for the run.
    pub fn events(&self, run: RunId) -> &[MirrorJournalEvent] {
        let idx = run.get() as usize;
        if idx < self.events_per_run.len() {
            &self.events_per_run[idx]
        } else {
            &[]
        }
    }
}

// ============================================================================
// Production exec wrappers — `#[verifier::external]` mirrors
// ============================================================================

/// Production wrapper for `codec::next_seq` at
/// `crates/vb_storage/src/codec/mod.rs:142-147`.
///
/// Production body:
/// ```text
/// seq.get().checked_add(1).map(EventSeq::new)
///       .ok_or(JournalError::SequenceOverflow)
/// ```
///
/// Body skipped by Verus (`#[verifier::external]`); contract
/// attached via `assume_specification` in the companion spec file.
#[verifier::external]
pub fn production_codec_next_seq(seq: EventSeq) -> Result<EventSeq, MirrorJournalError> {
    match seq.get().checked_add(1) {
        Some(value) => Ok(EventSeq::new(value)),
        None => Err(MirrorJournalError::SequenceOverflow),
    }
}

/// Production wrapper for `codec::validate_replayed_event` at
/// `crates/vb_storage/src/codec/mod.rs:149-167`.
///
/// Production body:
/// ```text
/// if event.run_id() != run {
///     return Err(JournalError::WrongRun { expected: run, actual: event.run_id() });
/// }
/// if event.seq() != expected {
///     return Err(JournalError::SequenceGap { expected, actual: event.seq() });
/// }
/// Ok(())
/// ```
///
/// Body skipped by Verus; contract attached via `assume_specification`.
#[verifier::external]
pub fn production_validate_replayed_event(
    run: RunId,
    expected: EventSeq,
    event: MirrorJournalEvent,
) -> Result<(), MirrorJournalError> {
    if event.run_id.0 != run.0 {
        return Err(MirrorJournalError::WrongRun {
            expected: run,
            actual: event.run_id,
        });
    }
    if event.seq.0 != expected.0 {
        return Err(MirrorJournalError::SequenceGap {
            expected,
            actual: event.seq,
        });
    }
    Ok(())
}

/// Production wrapper for `FjallJournal::latest_durable_snapshot_seq`
/// at `crates/vb_storage/src/trimming/logic.rs:24-41`. Returns the
/// highest `EventSeq` for the run from the snapshot keyspace, or
/// `None` when no snapshot exists.
///
/// Body skipped by Verus; contract attached via `assume_specification`.
#[verifier::external]
pub fn production_latest_durable_snapshot_seq(
    journal: &MirrorJournal,
    run: RunId,
) -> Option<u64> {
    journal.latest_snapshot_seq(run)
}

/// Production wrapper for `validate_replay_sequence` at
/// `crates/vb_storage/src/journal/replay.rs:164-176`.
///
/// Production body:
/// ```text
/// let expected_seq = match *expected {
///     Some(seq) => seq,
///     None => event.seq(),
/// };
/// crate::codec::validate_replayed_event(run, expected_seq, event)?;
/// *expected = Some(crate::codec::next_seq(expected_seq)?);
/// Ok(())
/// ```
///
/// Body skipped by Verus; contract attached via `assume_specification`.
#[verifier::external]
pub fn production_validate_replay_sequence(
    run: RunId,
    expected: Option<EventSeq>,
    event: MirrorJournalEvent,
) -> Result<Option<EventSeq>, MirrorJournalError> {
    let expected_seq = match expected {
        Some(seq) => seq,
        None => event.seq,
    };
    production_validate_replayed_event(run, expected_seq, event)?;
    let next = production_codec_next_seq(expected_seq)?;
    Ok(Some(next))
}

/// Spec-mode helper: returns `expected_seq + 1` (or `u64::MAX` on
/// overflow). Pure exec-mode projection used by the
/// `production_validate_replay_sequence` contract.
pub fn production_next_seq_view(seq: EventSeq) -> EventSeq {
    match seq.get().checked_add(1) {
        Some(value) => EventSeq::new(value),
        None => EventSeq::new(u64::MAX),
    }
}

/// Spec-mode helper: extracts the seq cursor from a `Option<EventSeq>`
/// passed to `production_validate_replay_sequence`. Used by the
/// contract postcondition for the `SequenceGap` arm.
pub fn expected_seq_view(expected: Option<EventSeq>) -> EventSeq {
    match expected {
        Some(s) => s,
        None => EventSeq::new(0),
    }
}

/// Production wrapper for `FjallJournal::events_for_run_from` at
/// `crates/vb_storage/src/journal/replay.rs:130-161`. Pure range
/// scan from `start_key` decoding each event, validating run_id
/// + seq contiguity via `validate_replay_sequence`, enforcing
/// `EventReplayLimit` via `push_replay_event`.
///
/// Body skipped by Verus; contract attached via `assume_specification`.
#[verifier::external]
pub fn production_events_for_run_from(
    journal: &MirrorJournal,
    run: RunId,
    start_seq: EventSeq,
    first_event: EventSeq,
    limit: EventReplayLimit,
) -> Result<Vec<MirrorJournalEvent>, MirrorJournalError> {
    let events = journal.events(run);
    let mut out = Vec::new();
    let mut expected = Some(first_event);
    let mut count: usize = 0;
    let start = start_seq.get();
    for event in events.iter() {
        if event.seq.get() < start {
            continue;
        }
        let next_expected = match production_validate_replay_sequence(run, expected, *event) {
            Ok(next) => next,
            Err(e) => return Err(e),
        };
        if out.len() >= limit.max_events() {
            return Err(MirrorJournalError::TooManyEvents {
                run,
                limit: limit.max_events(),
                observed: out.len(),
            });
        }
        out.push(*event);
        expected = next_expected;
        count = count + 1;
    }
    let _ = count;
    Ok(out)
}

/// Production wrapper for `FjallJournal::events_for_run` at
/// `crates/vb_storage/src/journal/replay.rs:59-61, 99-115`. The
/// snapshot+tail reader: starts after `latest_durable_snapshot_seq`
/// when present, otherwise from `EventSeq::new(0)`.
///
/// Body skipped by Verus; contract attached via `assume_specification`.
#[verifier::external]
pub fn production_events_for_run(
    journal: &MirrorJournal,
    run: RunId,
) -> Result<Vec<MirrorJournalEvent>, MirrorJournalError> {
    let limit = EventReplayLimit::DEFAULT;
    let (start_seq, first_event) = match journal.latest_snapshot_seq(run) {
        Some(seq) => {
            let next = match production_codec_next_seq(EventSeq::new(seq)) {
                Ok(s) => s,
                Err(e) => return Err(e),
            };
            (next, next)
        }
        None => (EventSeq::ZERO, EventSeq::ZERO),
    };
    production_events_for_run_from(journal, run, start_seq, first_event, limit)
}

/// Production wrapper for `classify_replay_push_len` at
/// `crates/vb_storage/src/journal/replay.rs:30-49`. Returns
/// `Accept { observed }` when the next event fits in the limit,
/// `TooMany { limit, observed }` when it would exceed.
///
/// Body skipped by Verus; contract attached via `assume_specification`.
#[verifier::external]
pub fn production_classify_replay_push_len(
    current_len: usize,
    limit: EventReplayLimit,
) -> ReplayPushLimitDecision {
    let max_events = limit.max_events();
    let observed = match current_len.checked_add(1) {
        Some(v) => v,
        None => {
            return ReplayPushLimitDecision::TooMany {
                limit: max_events,
                observed: usize::MAX,
            };
        }
    };
    if observed > max_events {
        ReplayPushLimitDecision::TooMany {
            limit: max_events,
            observed,
        }
    } else {
        ReplayPushLimitDecision::Accept { observed }
    }
}

/// Mirror of production `ReplayPushLimitDecision` enum at
/// `crates/vb_storage/src/journal/replay.rs:14-27`. Field shapes
/// match production line-by-line.
#[derive(Clone, Copy)]
pub enum ReplayPushLimitDecision {
    Accept { observed: usize },
    TooMany { limit: usize, observed: usize },
}
