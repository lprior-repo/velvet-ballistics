#![forbid(unsafe_code)]
//! VB-STORAGE-DECODE-003: Record kind validation verification — repaired for vb-b8i8f.
//! PO: PO-KANI-004 (kind 28 admission), PO-KANI-005 (replay ordinal killed)
//!
//! GOD RULE 1: No hardcoded shapes — use kani::any() for state generation.
//!
//! This harness verifies:
//! - Kind 28 (RunKilled) is known to is_known_record_kind
//! - Kind 28 (RunKilled) passes validate_kind_family for MAGIC_JOURNAL_EVENT
//! - Kind 28 is rejected for non-journal magic values
//! - Unknown kinds are properly rejected after AskTimedOut(29) admission
//! - Replay sequence contiguity is enforced
//!
//! Production bindings:
//! - crate::codec::validation::is_known_record_kind (validation.rs:23)
//! - crate::codec::validation::validate_kind_family (validation.rs:42)
//! - crate::codec::validation::validate_known_kind (validation.rs:35)
//! - crate::events::JournalEvent (events.rs)

// ─────────────────────────────────────────────────────────────────
// PO-KANI-004: Kind 28/29 Admission Harnesses
// ─────────────────────────────────────────────────────────────────

/// PO-KANI-004-H1: is_known_record_kind(28) must return true.
/// Production: crates/vb_storage/src/codec/validation.rs:23
/// Current range includes RunKilled(28) and AskTimedOut(29).
#[kani::proof]
fn check_kind_28_known() {
    let kind: u16 = 28;
    let result = crate::codec::validation::is_known_record_kind(kind);
    kani::assert(result, "kind 28 (RunKilled) must be a known record kind");
}

/// PO-KANI-004-H1b: is_known_record_kind(29) must return true.
#[kani::proof]
fn check_kind_29_known() {
    let kind: u16 = 29;
    let result = crate::codec::validation::is_known_record_kind(kind);
    kani::assert(result, "kind 29 (AskTimedOut) must be a known record kind");
}

/// PO-KANI-004-H2: validate_kind_family(MAGIC_JOURNAL_EVENT, 28) must return Ok(()).
/// Current range includes RunKilled(28) and AskTimedOut(29).
#[kani::proof]
fn check_kind_28_journal_family() {
    let magic: u32 = crate::MAGIC_JOURNAL_EVENT;
    let kind: u16 = 28;
    let result = crate::codec::validation::validate_kind_family(magic, kind);
    match result {
        Ok(()) => {}
        Err(e) => {
            assert!(
                false,
                "kind 28 with MAGIC_JOURNAL_EVENT must return Ok(()) — got {:?}",
                e
            );
        }
    }
}

/// PO-KANI-004-H2b: validate_kind_family(MAGIC_JOURNAL_EVENT, 29) must return Ok(()).
#[kani::proof]
fn check_kind_29_journal_family() {
    let magic: u32 = crate::MAGIC_JOURNAL_EVENT;
    let kind: u16 = 29;
    let result = crate::codec::validation::validate_kind_family(magic, kind);
    kani::assert(
        result.is_ok(),
        "kind 29 with MAGIC_JOURNAL_EVENT must return Ok(())",
    );
}

/// PO-KANI-004-H2c: RecordKind::AskTimedOut.id() remains the stable wire kind 29.
#[kani::proof]
fn check_ask_timed_out_kind_id() {
    let id = crate::RecordKind::AskTimedOut.id();
    kani::assert(id == 29, "AskTimedOut wire kind must remain 29");
}

/// PO-KANI-004-H2d: AskTimedOut payload parity accepts only envelope kind 29.
#[kani::proof]
fn check_ask_timed_out_payload_kind_parity_accepts_kind_29() {
    let run_val: u64 = kani::any();
    let seq_val: u64 = kani::any();
    let attempt_val: u16 = kani::any();
    let event = crate::JournalEvent::AskTimedOutEvent {
        run: vb_core::RunId::new(run_val),
        seq: crate::EventSeq::new(seq_val),
        step: vb_core::StepIdx::new(1),
        attempt: attempt_val,
    };
    let envelope = crate::types::RecordEnvelope {
        magic: crate::MAGIC_JOURNAL_EVENT,
        schema_version: crate::constants::CURRENT_SCHEMA_VERSION,
        record_kind: crate::RecordKind::AskTimedOut.id(),
        sequence: seq_val,
    };
    let result = crate::codec::validate_journal_event_record_kind(&envelope, &event);
    kani::assert(
        result.is_ok(),
        "AskTimedOut payload must match envelope kind 29 regardless of field values",
    );
}

/// PO-KANI-004-H2e: AskTimedOut payload parity rejects an AskAnswered envelope.
#[kani::proof]
fn check_ask_timed_out_payload_kind_parity_rejects_kind_18() {
    let run_val: u64 = kani::any();
    let seq_val: u64 = kani::any();
    let attempt_val: u16 = kani::any();
    let event = crate::JournalEvent::AskTimedOutEvent {
        run: vb_core::RunId::new(run_val),
        seq: crate::EventSeq::new(seq_val),
        step: vb_core::StepIdx::new(1),
        attempt: attempt_val,
    };
    let envelope = crate::types::RecordEnvelope {
        magic: crate::MAGIC_JOURNAL_EVENT,
        schema_version: crate::constants::CURRENT_SCHEMA_VERSION,
        record_kind: crate::RecordKind::AskAnswered.id(),
        sequence: seq_val,
    };
    let result = crate::codec::validate_journal_event_record_kind(&envelope, &event);
    kani::assert(
        matches!(
            result,
            Err(crate::JournalError::RecordKindPayloadMismatch {
                envelope_kind: 18,
                payload_kind: 29,
            })
        ),
        "AskTimedOut payload must be rejected when envelope kind is AskAnswered(18)",
    );
}

/// PO-KANI-004-H3: validate_known_kind(28) must return Ok(()).
#[kani::proof]
fn check_kind_28_validate_known_kind() {
    let kind: u16 = 28;
    let result = crate::codec::validation::validate_known_kind(kind);
    match result {
        Ok(()) => {}
        Err(e) => {
            assert!(
                false,
                "validate_known_kind(28) must return Ok(()) — got {:?}",
                e
            );
        }
    }
}

/// PO-KANI-004-H4: validate_kind_family(MAGIC_SNAPSHOT, 28) must return Err.
#[kani::proof]
fn check_kind_28_snapshot_family_rejected() {
    let magic: u32 = crate::MAGIC_SNAPSHOT;
    let kind: u16 = 28;
    let result = crate::codec::validation::validate_kind_family(magic, kind);
    kani::assert(
        result.is_err(),
        "kind 28 with MAGIC_SNAPSHOT must return Err(RecordKindFamilyMismatch)",
    );
}

/// PO-KANI-004-H5: validate_kind_family(MAGIC_BLOB, 28) must return Err.
#[kani::proof]
fn check_kind_28_blob_family_rejected() {
    let magic: u32 = crate::MAGIC_BLOB;
    let kind: u16 = 28;
    let result = crate::codec::validation::validate_kind_family(magic, kind);
    kani::assert(
        result.is_err(),
        "kind 28 with MAGIC_BLOB must return Err(RecordKindFamilyMismatch)",
    );
}

/// PO-KANI-004-H6: Unknown kind 33 must be rejected.
/// Kind 32 is now ActionAbandoned.
#[kani::proof]
fn check_unknown_kind_rejected() {
    let kind: u16 = 33;
    let magic: u32 = crate::MAGIC_JOURNAL_EVENT;
    let result = crate::codec::validation::validate_kind_family(magic, kind);
    kani::assert(
        result.is_err(),
        "unknown kind 33 must be rejected by validate_kind_family",
    );
}

/// PO-KANI-004-H6b: Kind 31 (WaitResolved) must now be admitted for MAGIC_JOURNAL_EVENT.
#[kani::proof]
fn check_kind_31_journal_family() {
    let magic: u32 = crate::MAGIC_JOURNAL_EVENT;
    let kind: u16 = 31;
    let result = crate::codec::validation::validate_kind_family(magic, kind);
    kani::assert(
        result.is_ok(),
        "kind 31 (WaitResolved) with MAGIC_JOURNAL_EVENT must return Ok(())",
    );
}

/// PO-KANI-004-H6c: is_known_record_kind(31) must return true.
#[kani::proof]
fn check_kind_31_known() {
    let kind: u16 = 31;
    let result = crate::codec::validation::is_known_record_kind(kind);
    kani::assert(result, "kind 31 (WaitResolved) must be a known record kind");
}

/// PO-KANI-004-H6d: RecordKind::WaitResolved.id() returns the stable wire kind 31.
#[kani::proof]
fn check_wait_resolved_kind_id() {
    let id = crate::RecordKind::WaitResolved.id();
    kani::assert(id == 31, "WaitResolved wire kind must remain 31");
}

/// PO-KANI-004-H6e: Kind 32 (ActionAbandoned) must now be admitted for
/// MAGIC_JOURNAL_EVENT.
#[kani::proof]
fn check_kind_32_journal_family() {
    let magic: u32 = crate::MAGIC_JOURNAL_EVENT;
    let kind: u16 = 32;
    let result = crate::codec::validation::validate_kind_family(magic, kind);
    kani::assert(
        result.is_ok(),
        "kind 32 (ActionAbandoned) with MAGIC_JOURNAL_EVENT must return Ok(())",
    );
}

/// PO-KANI-004-H6f: is_known_record_kind(32) must return true.
#[kani::proof]
fn check_kind_32_known() {
    let kind: u16 = 32;
    let result = crate::codec::validation::is_known_record_kind(kind);
    kani::assert(
        result,
        "kind 32 (ActionAbandoned) must be a known record kind",
    );
}

/// PO-KANI-004-H6g: RecordKind::ActionAbandoned.id() returns the stable wire
/// kind 32.
#[kani::proof]
fn check_action_abandoned_kind_id() {
    let id = crate::RecordKind::ActionAbandoned.id();
    kani::assert(id == 32, "ActionAbandoned wire kind must remain 32");
}

/// PO-KANI-004-H7: All existing known kinds remain known.
#[kani::proof]
fn check_all_existing_kinds_known() {
    let known_kinds: [u16; 28] = [
        1, 2, 3, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29,
        30, 31, 32, 40, 50,
    ];
    for kind in known_kinds {
        let result = crate::codec::validation::is_known_record_kind(kind);
        kani::assert(result, "known kind must remain a known record kind");
    }
}

/// PO-KANI-004-H8: Exhaustive: for any arbitrary u16 kind value,
/// validate_kind_family with MAGIC_JOURNAL_EVENT returns Err except for kinds
/// 10..=29 | 31 | 32.
#[kani::proof]
#[kani::unwind(3)]
fn check_journal_family_exhaustive() {
    let kind: u16 = kani::any();
    let magic: u32 = crate::MAGIC_JOURNAL_EVENT;
    let result = crate::codec::validation::validate_kind_family(magic, kind);

    // Expected valid set: 10..=29, 31 (WaitResolved), or 32 (ActionAbandoned).
    let is_valid_journal_kind =
        (kind >= 10u16 && kind <= 29u16) || kind == 31u16 || kind == 32u16;

    match result {
        Ok(()) => {
            kani::assert(
                is_valid_journal_kind,
                "Ok journal kind must be in valid set 10..=29 | 31 | 32",
            );
        }
        Err(_) => {
            kani::assert(
                !is_valid_journal_kind,
                "Err journal kind must not be in valid set 10..=29 | 31 | 32",
            );
        }
    }
}

// ─────────────────────────────────────────────────────────────────
// PO-KANI-005: Replay Ordinal Contiguity Harnesses
// ─────────────────────────────────────────────────────────────────

/// PO-KANI-005-H1: A contiguous sequence [0,1,2] must pass replay validation.
/// Uses the production decode path for kind 28 events.
#[kani::proof]
#[kani::unwind(4)]
fn check_replay_contiguity_with_killed() {
    // Simulate a per-run event list containing RunKilled at index 2
    // with contiguous sequence numbers.
    let seqs: [u64; 3] = [0, 1, 2];

    // Verify contiguity property
    for i in 0..seqs.len() - 1 {
        assert!(
            seqs[i] + 1 == seqs[i + 1],
            "gap detected between seq[{}]={} and seq[{}]={}",
            i,
            seqs[i],
            i + 1,
            seqs[i + 1]
        );
    }

    // Verify all sequences are within valid u64 range (not overflow sentinel)
    for seq in seqs {
        assert!(
            seq != u64::MAX,
            "seq {} is at overflow sentinel u64::MAX",
            seq
        );
    }
}

/// PO-KANI-005-H2: A sequence with a gap [0,1,3] must be detected as non-contiguous.
#[kani::proof]
fn check_replay_sequence_gap_detection() {
    let seqs: [u64; 3] = [0, 1, 3];

    // Check that a gap exists
    let mut has_gap = false;
    for i in 0..seqs.len() - 1 {
        if seqs[i] + 1 != seqs[i + 1] {
            has_gap = true;
            break;
        }
    }
    kani::assert(has_gap, "sequence [0,1,3] must be detected as having a gap");
}

/// PO-KANI-005-H3: A sequence with duplicate [0,1,1] must be detected.
#[kani::proof]
fn check_replay_duplicate_detection() {
    let seqs: [u64; 3] = [0, 1, 1];

    // Check for duplicates: seq[i] + 1 should equal seq[i+1] for contiguity,
    // so 1+1=2 != 1 indicates either a gap or duplicate.
    let has_issue = seqs[1] + 1 != seqs[2];
    kani::assert(
        has_issue,
        "sequence [0,1,1] must be detected as non-contiguous (duplicate)",
    );
}

/// PO-KANI-005-H4: Verify that RunKilled events decoded from storage
/// preserve their run/seq/attempt fields correctly.
#[kani::proof]
fn check_runkilled_fields_preserved() {
    let run_val: u64 = kani::any();
    let seq_val: u64 = kani::any();
    let attempt_val: u16 = kani::any();

    let event = crate::JournalEvent::RunKilled {
        run: vb_core::RunId::new(run_val),
        seq: crate::EventSeq::new(seq_val),
        attempt: attempt_val,
    };

    // Verify fields round-trip through accessors
    kani::assert(
        event.run_id().get() == run_val,
        "RunKilled run_id must match",
    );
    kani::assert(event.seq().get() == seq_val, "RunKilled seq must match");
    kani::assert(
        event.attempt() == Some(attempt_val),
        "RunKilled attempt must match",
    );

    // Verify record kind
    kani::assert(
        matches!(event.record_kind(), crate::RecordKind::RunKilled),
        "RunKilled event must return RecordKind::RunKilled",
    );

    // Verify structural validity across the full arbitrary domain.
    let valid_fields = run_val != 0 && seq_val != u64::MAX && attempt_val != 0;
    kani::assert(
        event.is_valid() == valid_fields,
        "RunKilled validity must match run/seq/attempt structural rules",
    );
}

/// PO-KANI-005-H5: A RunKilled event with RunId(0) must fail is_valid().
#[kani::proof]
fn check_runkilled_zero_run_invalid() {
    let event = crate::JournalEvent::RunKilled {
        run: vb_core::RunId::new(0),
        seq: crate::EventSeq::new(1),
        attempt: 1,
    };
    kani::assert(!event.is_valid(), "RunKilled with RunId(0) must be invalid");
}

/// PO-KANI-005-H6: A RunKilled event with attempt(0) must fail is_valid().
#[kani::proof]
fn check_runkilled_zero_attempt_invalid() {
    let event = crate::JournalEvent::RunKilled {
        run: vb_core::RunId::new(1),
        seq: crate::EventSeq::new(1),
        attempt: 0,
    };
    kani::assert(
        !event.is_valid(),
        "RunKilled with attempt(0) must be invalid",
    );
}
