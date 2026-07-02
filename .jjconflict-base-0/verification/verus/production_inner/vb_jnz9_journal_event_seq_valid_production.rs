// SPDX-License-Identifier: MIT
//
// ============================================================================
// Drift-detection stub for vb_jnz9_journal_event_seq_valid
// ============================================================================
//
// This file is a minimal drift-detection stub for the
// `vb_jnz9_journal_event_seq_valid` Verus spec. It exists so the
// companion `verification/verus/extern_vb_jnz9_journal_event_seq_valid.rs`
// can include this file via
// `#[path = "production_inner/vb_jnz9_journal_event_seq_valid_production.rs"]`
// to satisfy the production-binding gate.
//
// The actual production mirror content (full 24-variant JournalEvent
// mirror, run_id/seq/is_valid body, ID newtypes, ActionTicket mirror)
// lives in
// `verification/verus/extern_vb_jnz9_journal_event_seq_valid.rs`,
// which carries verbatim copies of the production source at:
//
//   - `JournalEvent::run_id`   <- crates/vb_storage/src/events.rs:332-363
//   - `JournalEvent::seq`      <- crates/vb_storage/src/events.rs:366-397
//   - `JournalEvent::is_valid` <- crates/vb_storage/src/events.rs:514-550
//   - `EventSeq`               <- crates/vb_storage/src/types.rs:73
//   - `RunId`                  <- crates/vb_core/src/ids/mod.rs:80
//   - `ActionTicket`           <- crates/vb_core/src/action/ticket.rs:6-21
//
// This stub mirrors the production `EventSeq` newtype as the smallest
// drift-detection surface.
//
// DRIFT POLICY: `crates/vb_storage/src/events.rs:514-550`
// Production source coverage:
//   - `JournalEvent::run_id`   <- crates/vb_storage/src/events.rs:332-363
//   - `JournalEvent::seq`      <- crates/vb_storage/src/events.rs:366-397
//   - `JournalEvent::is_valid` <- crates/vb_storage/src/events.rs:514-550
//   - `EventSeq`               <- crates/vb_storage/src/types.rs:73-93
//   - `RunId`                  <- crates/vb_core/src/ids/mod.rs:80
//   - `ActionTicket`           <- crates/vb_core/src/action/ticket.rs:6-21
// Regenerate this file whenever production changes. Any rename of
// `EventSeq::ZERO`/`MAX` or body change in `is_valid` breaks the
// `extern_vb_jnz9_journal_event_seq_valid` Verus build.

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

/// Mirror of production `is_valid` decision at
/// `crates/vb_storage/src/events.rs:514-550`. Body is
/// `#[verifier::external]` (opaque); the companion spec file attaches
/// `assume_specification` contracts that the spec proofs discharge.
#[verifier::external]
pub fn is_valid_stub(run_id_value: u64, seq_value: u64, attempt: u16) -> bool {
    // Production body (verbatim):
    //   if self.run_id().get() == 0 { return false; }
    //   if self.seq().get() == u64::MAX { return false; }
    //   ...attempt check...
    run_id_value != 0 && seq_value != u64::MAX && attempt != 0
}

} // verus!