//!
//! Kani harnesses for journal key injectivity and duplicate rejection — TLA bridge RRO-TLA-RETRY-JOURNAL-001.
//!
//! Bead: vb-282my
//! Obligations: PO-vb282my-RJ-KANI-001 through PO-vb282my-RJ-KANI-004
//!
//! Target: crate::keys::run_event_key
//!         crate::journal::internal::append_unpersisted
//!         crate::journal::internal::append_queued_unpersisted
//!
//! GOD RULE 1: All inputs use kani::any() for non-journal types.
//! GOD RULE 2: Calls actual production key functions. Journal operations
//!   require runtime support (Fjall uses file-backed LSM + sync::Mutex);
//!   harnesses test key encoding logic and document journal-level trust boundaries.
//!
//! Trust boundaries:
//!   TB-vb282my-storage-fjall-001: FjallJournal construction is trusted to provide
//!     a valid Keyspace for testing. In Kani, we test key properties and encoding
//!     determinism; full duplicate-rejection testing requires runtime integration tests.

#![forbid(unsafe_code)]
#![cfg(kani)]

use crate::{EventSeq, JournalError, keys::run_event_key};
use vb_core::ids::RunId;

// =========================================================================
// Helpers
// =========================================================================

fn any_run_id() -> RunId {
    RunId::new(kani::any::<u64>())
}

fn any_seq() -> EventSeq {
    EventSeq::new(kani::any::<u64>())
}

// =========================================================================
// PO-vb282my-RJ-KANI-001: Key encoding injectivity
// For distinct (run1, seq1) ≠ (run2, seq2), run_event_key(run1, seq1) != run_event_key(run2, seq2)
// =========================================================================

#[kani::proof]
#[kani::unwind(5)]
fn kani_journal_key_injectivity() {
    let run1: u64 = kani::any();
    let seq1: u64 = kani::any();
    let run2: u64 = kani::any();
    let seq2: u64 = kani::any();

    // Only test distinct pairs
    kani::assume(run1 != run2 || seq1 != seq2);

    let key1 = run_event_key(RunId::new(run1), EventSeq::new(seq1));
    let key2 = run_event_key(RunId::new(run2), EventSeq::new(seq2));

    match (&key1, &key2) {
        (Ok(k1), Ok(k2)) => {
            // Keys must differ when (run, seq) pairs differ
            //!
//! Kani harnesses for journal key injectivity and duplicate rejection — TLA bridge RRO-TLA-RETRY-JOURNAL-001.
//!
//! Bead: vb-282my
//! Obligations: PO-vb282my-RJ-KANI-001 through PO-vb282my-RJ-KANI-004
//!
//! Target: crate::keys::run_event_key
//!         crate::journal::internal::append_unpersisted
//!         crate::journal::internal::append_queued_unpersisted
//!
//! GOD RULE 1: All inputs use kani::any() for non-journal types.
//! GOD RULE 2: Calls actual production key functions. Journal operations
//!   require runtime support (Fjall uses file-backed LSM + sync::Mutex);
//!   harnesses test key encoding logic and document journal-level trust boundaries.
//!
//! Trust boundaries:
//!   TB-vb282my-storage-fjall-001: FjallJournal construction is trusted to provide
//!     a valid Keyspace for testing. In Kani, we test key properties and encoding
//!     determinism; full duplicate-rejection testing requires runtime integration tests.

#![forbid(unsafe_code)]
#![cfg(kani)]

use crate::{EventSeq, JournalError, keys::run_event_key};
use vb_core::ids::RunId;

// =========================================================================
// Helpers
// =========================================================================

fn any_run_id() -> RunId {
    RunId::new(kani::any::<u64>())
}

fn any_seq() -> EventSeq {
    EventSeq::new(kani::any::<u64>())
}

// =========================================================================
// PO-vb282my-RJ-KANI-001: Key encoding injectivity
// For distinct (run1, seq1) ≠ (run2, seq2), run_event_key(run1, seq1) != run_event_key(run2, seq2)
// =========================================================================

#[kani::proof]
#[kani::unwind(5)]
fn kani_journal_key_injectivity() {
    let run1: u64 = kani::any();
    let seq1: u64 = kani::any();
    let run2: u64 = kani::any();
    let seq2: u64 = kani::any();

    // Only test distinct pairs
    kani::assume(run1 != run2 || seq1 != seq2);

    let key1 = run_event_key(RunId::new(run1), EventSeq::new(seq1));
    let key2 = run_event_key(RunId::new(run2), EventSeq::new(seq2));

    match (&key1, &key2) {
        (Ok(k1), Ok(k2)) => {
            // Keys must differ when (run, seq) pairs differ
            kani::assert(
                k1 != k2,
                "distinct (run, seq) pairs must produce distinct keys",
            );
            // Verify key length invariant
            kani::assert(k1.len() == 17, "journal event key is 17 bytes");
        }
        _ => {}
    }
    kani::cover!(key1.is_ok() && key2.is_ok(), "both_keys_ok");
}

// =========================================================================
// PO-vb282my-RJ-KANI-001 supplemental: RunId-specific injectivity
// Same seq but different RunId must produce different keys
// =========================================================================

#[kani::proof]
#[kani::unwind(5)]
fn kani_journal_key_runid_injectivity() {
    let seq: u64 = kani::any();
    let run1: u64 = kani::any();
    let run2: u64 = kani::any();
    kani::assume(run1 != run2);

    let key1 = run_event_key(RunId::new(run1), EventSeq::new(seq));
    let key2 = run_event_key(RunId::new(run2), EventSeq::new(seq));

    let both_ok_run = key1.is_ok() && key2.is_ok();
    match (&key1, &key2) {
        (Ok(k1), Ok(k2)) => {
             == 17, "journal event key is 17 bytes");
        }
        _ => {}
    }
    kani::cover!(key1.is_ok() && key2.is_ok(), "both_keys_ok");
}

// =========================================================================
// PO-vb282my-RJ-KANI-001 supplemental: RunId-specific injectivity
// Same seq but different RunId must produce different keys
// =========================================================================

#[kani::proof]
#[kani::unwind(5)]
fn kani_journal_key_runid_injectivity() {
    let seq: u64 = kani::any();
    let run1: u64 = kani::any();
    let run2: u64 = kani::any();
    kani::assume(run1 != run2);

    let key1 = run_event_key(RunId::new(run1), EventSeq::new(seq));
    let key2 = run_event_key(RunId::new(run2), EventSeq::new(seq));

    let both_ok_run = key1.is_ok() && key2.is_ok();
    match (&key1, &key2) {
        (Ok(k1), Ok(k2)) => {
            kani::assert(k1 != k2, "different run ids must produce different keys");
        }
        _ => {}
    }
    kani::cover!(both_ok_run, "runid_injectivity_ok");
}

// =========================================================================
// PO-vb282my-RJ-KANI-001 supplemental: EventSeq-specific injectivity
// Same RunId but different seq must produce different keys
// =========================================================================

#[kani::proof]
#[kani::unwind(5)]
fn kani_journal_key_seq_injectivity() {
    let run: u64 = kani::any();
    let seq1: u64 = kani::any();
    let seq2: u64 = kani::any();
    kani::assume(seq1 != seq2);

    let key1 = run_event_key(RunId::new(run), EventSeq::new(seq1));
    let key2 = run_event_key(RunId::new(run), EventSeq::new(seq2));

    let both_ok_seq = key1.is_ok() && key2.is_ok();

    match (&key1, &key2) {
        (Ok(k1), Ok(k2)) => {
            , EventSeq::new(seq1));
    let key2 = run_event_key(RunId::new(run), EventSeq::new(seq2));

    let both_ok_seq = key1.is_ok() && key2.is_ok();

    match (&key1, &key2) {
        (Ok(k1), Ok(k2)) => {
            kani::assert(
                k1 != k2,
                "different seq numbers must produce different keys",
            );
        }
        _ => {}
    }
    kani::cover!(both_ok_seq, "seq_injectivity_ok");
}

// =========================================================================
// PO-vb282my-RJ-KANI-001 supplemental: Key prefix stability
// The first byte of every journal event key must be the RUN_EVENT prefix (0x11)
// =========================================================================

#[kani::proof]
#[kani::unwind(5)]
fn kani_journal_key_prefix() {
    let run: u64 = kani::any();
    let seq: u64 = kani::any();

    let key = run_event_key(RunId::new(run), EventSeq::new(seq));

    if let Ok(k) = key {
        , EventSeq::new(seq));

    if let Ok(k) = key {
        kani::assert(
            k[0] == 0x11,
            "journal event key must start with 0x11 prefix",
        );
        // Verify big-endian encoding of run_id (bytes 1-8) and seq (bytes 9-16)
        let run_be = run.to_be_bytes();
        for i in 0..8 {
            kani::assert(k[1 + i] == run_be[i], "run_id byte in big-endian order");
        }
        let seq_be = seq.to_be_bytes();
        for i in 0..8 {
            kani::assert(k[9 + i] == seq_be[i], "seq byte in big-endian order");
        }
    }
    kani::cover!(key.is_ok(), "key_prefix_ok");
}

// =========================================================================
// PO-vb282my-RJ-KANI-002 through 004: Journal duplicate rejection
//
// BLOCKED: FjallJournal requires file-backed LSM tree + Mutex<()> which
// Kani cannot model directly. The duplicate-rejection logic in
// append_unpersisted and append_queued_unpersisted is tested via:
//   1. Runtime integration tests (crates/vb_storage/tests/)
//   2. The key injectivity harnesses above (proving keys are unique)
//   3. The proptest round-trip (PO-vb282my-RJ-PROP-001)
//
// The key encoding guarantees that distinct (run, seq) pairs produce
// distinct keys. Combined with Fjall's Keyspace uniqueness guarantee,
// this implies that append_unpersisted will return DuplicateEvent on
// re-insertion. Append_queued_unpersisted's idempotency check relies
// on postcard deserialization which is tested via proptest.
//
// Trust boundary TB-vb282my-storage-fjall-001: Fjall Keyspace behavior
// (contains_key exact match, insert atomicity) is trusted per the
// Fjall 3.x specification.
// =========================================================================

#[kani::proof]
#[kani::unwind(5)]
fn kani_journal_duplicate_invariant() {
    // Prove the logical invariant: duplicate detection is equivalent to key collision.
    // If keys are injective (proved above), then duplicate detection is sound.
    let run1: u64 = kani::any();
    let run2: u64 = kani::any();
    let seq1: u64 = kani::any();
    let seq2: u64 = kani::any();

    // Same (run, seq) => same key (deterministic encoding)
    if run1 == run2 && seq1 == seq2 {
        let key1 = run_event_key(RunId::new(run1), EventSeq::new(seq1));
        let key2 = run_event_key(RunId::new(run2), EventSeq::new(seq2));
        match (key1, key2) {
            (Ok(k1), Ok(k2)) => {
                , "key_prefix_ok");
}

// =========================================================================
// PO-vb282my-RJ-KANI-002 through 004: Journal duplicate rejection
//
// BLOCKED: FjallJournal requires file-backed LSM tree + Mutex<()> which
// Kani cannot model directly. The duplicate-rejection logic in
// append_unpersisted and append_queued_unpersisted is tested via:
//   1. Runtime integration tests (crates/vb_storage/tests/)
//   2. The key injectivity harnesses above (proving keys are unique)
//   3. The proptest round-trip (PO-vb282my-RJ-PROP-001)
//
// The key encoding guarantees that distinct (run, seq) pairs produce
// distinct keys. Combined with Fjall's Keyspace uniqueness guarantee,
// this implies that append_unpersisted will return DuplicateEvent on
// re-insertion. Append_queued_unpersisted's idempotency check relies
// on postcard deserialization which is tested via proptest.
//
// Trust boundary TB-vb282my-storage-fjall-001: Fjall Keyspace behavior
// (contains_key exact match, insert atomicity) is trusted per the
// Fjall 3.x specification.
// =========================================================================

#[kani::proof]
#[kani::unwind(5)]
fn kani_journal_duplicate_invariant() {
    // Prove the logical invariant: duplicate detection is equivalent to key collision.
    // If keys are injective (proved above), then duplicate detection is sound.
    let run1: u64 = kani::any();
    let run2: u64 = kani::any();
    let seq1: u64 = kani::any();
    let seq2: u64 = kani::any();

    // Same (run, seq) => same key (deterministic encoding)
    if run1 == run2 && seq1 == seq2 {
        let key1 = run_event_key(RunId::new(run1), EventSeq::new(seq1));
        let key2 = run_event_key(RunId::new(run2), EventSeq::new(seq2));
        match (key1, key2) {
            (Ok(k1), Ok(k2)) => {
                kani::assert(k1 == k2, "identical inputs must produce identical keys");
            }
            _ => {}
        }
    }

    // Different (run, seq) => different key (injectivity, tested above)
    if run1 != run2 || seq1 != seq2 {
        let key1 = run_event_key(RunId::new(run1), EventSeq::new(seq1));
        let key2 = run_event_key(RunId::new(run2), EventSeq::new(seq2));
        match (key1, key2) {
            (Ok(k1), Ok(k2)) => {
                , EventSeq::new(seq1));
        let key2 = run_event_key(RunId::new(run2), EventSeq::new(seq2));
        match (key1, key2) {
            (Ok(k1), Ok(k2)) => {
                kani::assert(k1 != k2, "distinct inputs must produce distinct keys");
            }
            _ => {}
        }
    }
}
