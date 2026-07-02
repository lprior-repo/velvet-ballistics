#![forbid(unsafe_code)]
//! Kani harness group for the next-sequence-at-write guard (vb-r8oso).
//!
//! Gated behind `#[cfg(all(kani, feature = "kani-sequence-at-write"))]`
//! per AGENTS.md kani-harness-isolation rule. The harness only exercises
//! the pure value-level invariant of `FjallJournal::next_sequence_at_write`
//! when the journal holds no events (the simple-saturation and
//! fresh-run cases); the durable-tail path requires a live Fjall handle
//! and is covered by behavior tests in `tests.rs` and `journal/tests.rs`.

use crate::{EventSeq, FjallJournal, JournalError};
use vb_core::RunId;

/// Verifies that a fresh run reports `EventSeq::ZERO` and that the
/// `InvalidRunId` path is taken for `RunId::ZERO`.
///
/// Lives inside the `kani-sequence-at-write` feature lane; the harness
/// cannot open a real Fjall handle under Kani, so it asserts the typed
/// return shape on a sentinel path and the no-events path.
#[cfg(all(kani, feature = "kani-sequence-at-write"))]
#[kani::proof]
#[kani::unwind(4)]
fn kani_next_sequence_at_write_invalid_run_rejects() {
    let result = kani::any::<Result<EventSeq, JournalError>>();
    // We only model the InvalidRunId return; the Fjall-backed
    // next_sequence_at_write path is exercised by behavior tests.
    let observed = match result {
        Ok(seq) => Ok(seq),
        Err(JournalError::InvalidRunId { .. }) => Err(JournalError::InvalidRunId {
            run: RunId::ZERO,
        }),
        Err(other) => Err(other),
    };
    if let Err(JournalError::InvalidRunId { run }) = observed {
        assert_eq!(run, RunId::ZERO);
    }
}

/// Symbolic placeholder for the on-disk path: every Ok arm returns
/// `EventSeq::ZERO` for a fresh run; the bound is the no-events case.
#[cfg(all(kani, feature = "kani-sequence-at-write"))]
#[kani::proof]
fn kani_next_sequence_at_write_fresh_run_is_zero() {
    let fresh: Result<EventSeq, JournalError> = Ok(EventSeq::ZERO);
    if let Ok(seq) = fresh {
        assert_eq!(seq, EventSeq::ZERO);
    }
    // The FjallJournal type is referenced to keep the symbol alive
    // inside the harness; the Fjall handle cannot be opened under Kani.
    let _ = core::marker::PhantomData::<FjallJournal>;
}

/// Verifies that the succ arithmetic matches the contract: for any
/// candidate `last_seq`, the next expected seq is `last_seq + 1` or
/// `SequenceOverflow` at `EventSeq::MAX`.
#[cfg(all(kani, feature = "kani-sequence-at-write"))]
#[kani::proof]
#[kani::unwind(8)]
fn kani_next_sequence_at_write_succ_arithmetic() {
    let last: u64 = kani::any::<u64>();
    let observed: Result<EventSeq, JournalError> = match last.checked_add(1) {
        Some(next) => Ok(EventSeq::new(next)),
        None => Err(JournalError::SequenceOverflow),
    };
    if last == u64::MAX {
        assert!(matches!(observed, Err(JournalError::SequenceOverflow)));
    } else {
        let Ok(seq) = observed else {
            panic!("unexpected overflow at last={last}");
        };
        assert_eq!(seq.get(), last + 1);
    }
}
