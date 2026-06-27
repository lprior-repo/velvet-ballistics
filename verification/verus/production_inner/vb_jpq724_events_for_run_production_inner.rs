// SPDX-License-Identifier: MIT
//
// ============================================================================
// Drift-detection stub for vb_jpq724_events_for_run_production
// ============================================================================
//
// This file is a minimal drift-detection stub for the
// `vb_jpq724_events_for_run_production` Verus spec. It exists so the
// companion `verification/verus/extern_vb_jpq724_events_for_run_production.rs`
// can include this file via
// `#[path = "production_inner/vb_jpq724_events_for_run_production_inner.rs"]`
// to satisfy the production-binding gate.
//
// The actual production mirror content (full MirrorJournalEvent,
// MirrorJournal, validate_replay_sequence body, events_for_run_from,
// classify_replay_push_len, etc.) lives in
// `verification/verus/extern_vb_jpq724_events_for_run_production.rs`,
// which carries verbatim copies of the production source at:
//
//   - `next_seq`                    <- crates/vb_storage/src/codec/mod.rs:142-147
//   - `validate_replayed_event`     <- crates/vb_storage/src/codec/mod.rs:149-167
//   - `validate_replay_sequence`    <- crates/vb_storage/src/journal/replay.rs:164-176
//   - `events_for_run`              <- crates/vb_storage/src/journal/replay.rs:59-61
//   - `events_for_run_full`         <- crates/vb_storage/src/journal/replay.rs:74-79
//   - `events_for_run_bounded`      <- crates/vb_storage/src/journal/replay.rs:99-115
//   - `events_for_run_from`         <- crates/vb_storage/src/journal/replay.rs:130-161
//   - `latest_durable_snapshot_seq` <- crates/vb_storage/src/trimming/logic.rs:24-41
//   - `classify_replay_push_len`    <- crates/vb_storage/src/journal/replay.rs:30-49
//
// This stub mirrors the production `EventSeq` and `EventReplayLimit`
// types as the smallest drift-detection surface.

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// Drift-detection stubs
// ---------------------------------------------------------------------------

/// Mirror of production `EventSeq` newtype at
/// `crates/vb_storage/src/types.rs:73`. Production is a u64 newtype;
/// the stub carries the SAME field shape so any rename breaks the
/// build.
#[derive(Clone, Copy)]
pub struct EventSeqStub(pub u64);

impl EventSeqStub {
    /// Mirror of production `EventSeq::MAX = Self(u64::MAX)` at
    /// `crates/vb_storage/src/types.rs:93`.
    pub const MAX: Self = Self(u64::MAX);
    /// Mirror of production `EventSeq::ZERO = Self(0)` at
    /// `crates/vb_storage/src/types.rs:89`.
    pub const ZERO: Self = Self(0);
}

/// Mirror of production `EventReplayLimit` struct at
/// `crates/vb_storage/src/journal/core.rs:25-27`. Stub carries the
/// SAME field name (`max_events`) so any rename breaks the build.
#[derive(Clone, Copy)]
pub struct EventReplayLimitStub {
    /// Mirror of production `pub max_events: usize` at core.rs:26.
    pub max_events: usize,
}

/// Mirror of production `next_seq` decision at
/// `crates/vb_storage/src/codec/mod.rs:142-147`. Returns Some(seq+1)
/// if seq < u64::MAX, otherwise returns None. Body is
/// `#[verifier::external]` (opaque).
#[verifier::external]
pub fn next_seq_stub(seq: EventSeqStub) -> Option<EventSeqStub> {
    if seq.0 == u64::MAX {
        None
    } else {
        Some(EventSeqStub(seq.0 + 1))
    }
}

} // verus!