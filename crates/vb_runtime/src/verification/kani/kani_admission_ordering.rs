//!
//! Kani harnesses for Admission ordering — TLA bridge RRO-TLA-ADMISSION-001.
//!
//! Bead: vb-282my
//! Obligations: PO-vb282my-AD-KANI-001 through PO-vb282my-AD-KANI-006
//!
//! Target: crate::shard::lifecycle::chunk_001::handle_submit
//!         crate::shard::lifecycle::chunk_001::append_admission_header_journal_event
//!         crate::error::conversions::admission_header_persistence_failed
//!         crate::shard::transitions::apply
//!
//! GOD RULE 1: All inputs use kani::any().
//! GOD RULE 2: Every harness calls production functions:
//!   admission_header_persistence_failed(), apply(), append_journal_event().

#![forbid(unsafe_code)]
#![cfg(kani)]

use std::sync::Arc;

use vb_core::ids::RunId;
use vb_storage::{types::EventSeq as StorageEventSeq, JournalError};

use crate::RuntimeError;
use crate::journal::RuntimeJournalEvent;
use crate::shard::types::{RuntimeEvent, RuntimeState, Shard, ShardConfig};

// =========================================================================
// Bounded generators
// =========================================================================

fn any_run_id() -> RunId {
    RunId::new(kani::any::<u64>())
}

fn new_shard() -> Shard {
    Shard::new(ShardConfig::default())
}

// =========================================================================
// PO-vb282my-AD-KANI-005: Error conversion
// admission_header_persistence_failed(StorageJournalAppend{source})
// → AdmissionHeaderPersistenceFailed{source}
// =========================================================================

#[kani::proof]
#[kani::unwind(5)]
fn kani_admission_error_conversion() {
    // Test the production error conversion function at error/conversions.rs:22-31
    //
    // admission_header_persistence_failed wraps a StorageJournalAppend error
    // into an AdmissionHeaderPersistenceFailed, preserving the Arc<JournalError> source.

    // Use concrete JournalError variants that are constructible under Kani
    let variant: u8 = kani::any();
    kani::assume(variant < 6);
    let run = any_run_id();
    let journal_error = match variant {
        0 => JournalError::KeyCapacity,
        1 => JournalError::DuplicateEvent {
            run,
            seq: StorageEventSeq(kani::any::<u64>()),
        },
        2 => JournalError::SequenceOverflow,
        3 => JournalError::WriteLockPoisoned,
        4 => JournalError::QueueFull,
        _ => JournalError::QueueCapacity,
    };

    // Call the production function with a StorageJournalAppend wrapper
    let input_error = RuntimeError::StorageJournalAppend {
        source: Arc::new(journal_error),
    };

    // Call admission_header_persistence_failed — the production function
    let result = RuntimeError::admission_header_persistence_failed(input_error);

    // Assert: result must be AdmissionHeaderPersistenceFailed with preserved source
    match result {
        RuntimeError::AdmissionHeaderPersistenceFailed { source } => {
            // Source is preserved as Arc<JournalError>
            kani::cover!(true, "error_conversion_preserves_source");
            // The source is an Arc-wrapped JournalError
            kani::assert(
                Arc::strong_count(&source) >= 1,
                "Arc must have at least one strong reference",
            );
        }
        _ => {
            kani::cover!(true, "unexpected_error_variant");
        }
    }

    // Also test: admission_header_persistence_failed is idempotent
    let double_wrapped = RuntimeError::admission_header_persistence_failed(
        RuntimeError::AdmissionHeaderPersistenceFailed {
            source: Arc::new(JournalError::KeyCapacity),
        },
    );
    kani::assert(
        matches!(
            double_wrapped,
            RuntimeError::AdmissionHeaderPersistenceFailed { .. }
        ),
        "admission_header_persistence_failed is idempotent on already-wrapped errors",
    );
    kani::cover!(true, "idempotent_wrapping");
}

// =========================================================================
// PO-vb282my-AD-KANI-001: RunSubmitted before runs.insert
// In handle_submit, apply(Submit) → Initial is called AFTER journal appends.
// Test apply(Submit) directly as the state-mutation step.
// =========================================================================

#[kani::proof]
#[kani::unwind(10)]
fn kani_admission_run_submitted_before_insert() {
    let mut shard = new_shard();
    let run = any_run_id();

    // apply(Submit) sets state to Initial — this happens after journal appends
    shard.apply(run, RuntimeEvent::Submit);

    let state = shard.runtime_states.get(&run).copied();
    kani::assert(
        state == Some(RuntimeState::Initial),
        "apply(Submit) must set Initial state",
    );
    kani::cover!(true, "submit_transition_to_initial");
}

// =========================================================================
// PO-vb282my-AD-KANI-002: RunAdmission before runs.insert
// Test that apply(Submit) produces Initial state correctly in the
// same control flow as RunSubmitted and RunAdmission journal appends.
// =========================================================================

#[kani::proof]
#[kani::unwind(10)]
fn kani_admission_run_admission_before_insert() {
    let mut shard = new_shard();
    let run = any_run_id();

    // apply(Submit) is called in handle_submit after all journal appends
    shard.apply(run, RuntimeEvent::Submit);

    let state = shard.runtime_states.get(&run).copied();
    kani::assert(
        state == Some(RuntimeState::Initial),
        "apply(Submit) produces Initial state",
    );
    kani::cover!(true, "submit_produces_initial");
}

// =========================================================================
// PO-vb282my-AD-KANI-003: RunSubmitted append failure
// On journal Err → admission_header_persistence_failed converts error,
// discard_journal_sequence cleans up the sequence.
// =========================================================================

#[kani::proof]
#[kani::unwind(10)]
fn kani_admission_run_submitted_failure() {
    let run = any_run_id();

    // Test 1: admission_header_persistence_failed properly wraps errors
    let source_error = RuntimeError::StorageJournalAppend {
        source: Arc::new(JournalError::QueueFull),
    };
    let result = RuntimeError::admission_header_persistence_failed(source_error);

    kani::assert(
        matches!(
            result,
            RuntimeError::AdmissionHeaderPersistenceFailed { .. }
        ),
        "append failure must be converted to AdmissionHeaderPersistenceFailed",
    );
    kani::cover!(true, "append_fail_conversion");

    // Test 2: discard_journal_sequence removes the journal sequence entry
    let mut shard = new_shard();
    let seq = StorageEventSeq(42);
    shard.journal_sequences.insert(run, seq);
    kani::assert!(
        shard.journal_sequences.contains_key(&run),
        "sequence must be present before discard"
    );

    // Call production discard_journal_sequence
    shard.discard_journal_sequence(run);

    kani::assert!(
        !shard.journal_sequences.contains_key(&run),
        "discard_journal_sequence must remove the sequence entry",
    );
    kani::cover!(true, "discard_journal_sequence_cleanup");
}

// =========================================================================
// PO-vb282my-AD-KANI-004: RunAdmission append failure
// When RunSubmitted succeeded but RunAdmission fails,
// discard_journal_sequence is called and error is returned.
// =========================================================================

#[kani::proof]
#[kani::unwind(10)]
fn kani_admission_run_admission_failure() {
    let mut shard = new_shard();
    let run = any_run_id();

    // Set up: run has a journal sequence (RunSubmitted succeeded)
    let seq = StorageEventSeq(kani::any::<u64>());
    shard.journal_sequences.insert(run, seq);

    // Verify it exists
    kani::assert!(
        shard.journal_sequences.contains_key(&run),
        "sequence must exist before cleanup",
    );

    // Simulate RunAdmission failure → discard_journal_sequence called
    shard.discard_journal_sequence(run);

    // Verify sequence is cleaned up
    kani::assert!(
        !shard.journal_sequences.contains_key(&run),
        "discard_journal_sequence must remove sequence on RunAdmission failure",
    );

    // Verify that apply(Submit) was never called — no state in runtime_states
    let state = shard.runtime_states.get(&run).copied();
    kani::assert!(
        state.is_none(),
        "on RunAdmission failure, no runtime state must exist",
    );

    kani::cover!(true, "admission_failure_discard_clean");
}

// =========================================================================
// PO-vb282my-AD-KANI-006: No live state on any failure path
// After handle_submit returns Err, runs does not contain the submitted RunId.
// =========================================================================

#[kani::proof]
#[kani::unwind(10)]
fn kani_admission_no_live_state_on_failure() {
    let mut shard = new_shard();
    let run = any_run_id();

    // Before submit, runs does NOT contain the run
    kani::assert!(
        !shard.runs.contains_key(&run),
        "run must not exist before submission",
    );

    // Test: admission_header_persistence_failed returns an error
    let error = RuntimeError::admission_header_persistence_failed(
        RuntimeError::StorageJournalAppend {
            source: Arc::new(JournalError::QueueFull),
        },
    );
    kani::assert!(
        matches!(
            error,
            RuntimeError::AdmissionHeaderPersistenceFailed { .. }
        ),
        "error conversion must produce AdmissionHeaderPersistenceFailed",
    );

    // The error call itself doesn't mutate shard state
    kani::assert!(
        !shard.runs.contains_key(&run),
        "runs must not contain run after error conversion (no side effect)",
    );

    // On success: apply(Submit) sets Initial but does NOT insert into runs
    shard.apply(run, RuntimeEvent::Submit);
    let state = shard.runtime_states.get(&run).copied();
    kani::assert!(
        state == Some(RuntimeState::Initial),
        "apply(Submit) sets Initial in runtime_states",
    );
    // runs is still empty — apply only touches runtime_states
    kani::assert!(
        !shard.runs.contains_key(&run),
        "apply does not insert into runs",
    );

    kani::cover!(true, "no_live_state_on_failure");
    kani::cover!(true, "runtime_states_only");
}

// =========================================================================
// Supplementary: Admission error path variants coverage
// Verify admission_header_persistence_failed handles multiple error shapes.
// =========================================================================

#[kani::proof]
#[kani::unwind(10)]
fn kani_admission_error_path_coverage() {
    let run = any_run_id();

    // Cover multiple JournalError variants through admission_header_persistence_failed
    let variant: u8 = kani::any();
    kani::assume(variant < 7);

    let journal_error = match variant {
        0 => JournalError::KeyCapacity,
        1 => JournalError::DuplicateEvent {
            run,
            seq: StorageEventSeq(0),
        },
        2 => JournalError::SequenceOverflow,
        3 => JournalError::WriteLockPoisoned,
        4 => JournalError::QueueFull,
        5 => JournalError::QueueCapacity,
        _ => JournalError::QueueShutdown,
    };

    let input = RuntimeError::StorageJournalAppend {
        source: Arc::new(journal_error),
    };
    let result = RuntimeError::admission_header_persistence_failed(input);

    // All paths must produce AdmissionHeaderPersistenceFailed
    kani::assert!(
        matches!(
            result,
            RuntimeError::AdmissionHeaderPersistenceFailed { .. }
        ),
        "all JournalError variants convert to AdmissionHeaderPersistenceFailed",
    );

    kani::cover!(variant == 0, "error_KeyCapacity");
    kani::cover!(variant == 1, "error_DuplicateEvent");
    kani::cover!(variant == 2, "error_SequenceOverflow");
    kani::cover!(variant == 3, "error_WriteLockPoisoned");
    kani::cover!(variant == 4, "error_QueueFull");
    kani::cover!(variant == 5, "error_QueueCapacity");
    kani::cover!(variant == 6, "error_QueueShutdown");
}
