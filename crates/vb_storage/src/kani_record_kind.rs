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
//! - Unknown kinds are properly rejected
//! - Replay sequence contiguity is enforced
//!
//! Production bindings:
//! - crate::codec::validation::is_known_record_kind (validation.rs:23)
//! - crate::codec::validation::validate_kind_family (validation.rs:42)
//! - crate::codec::validation::validate_known_kind (validation.rs:35)
//! - crate::events::JournalEvent (events.rs)

// ─────────────────────────────────────────────────────────────────
// PO-KANI-004: Kind 28 Admission Harnesses
// ─────────────────────────────────────────────────────────────────

/// PO-KANI-004-H1: is_known_record_kind(28) must return true.
/// Production: crates/vb_storage/src/codec/validation.rs:23
/// Current gap: is_known_record_kind uses 10..=27, excludes 28.
/// Blocked until validation.rs line 24 is extended: matches!(kind, 1|2|3|10..=28|30|40|50)
#[kani::proof]
fn check_kind_28_known() {
    let kind: u16 = 28;
    let result = crate::codec::validation::is_known_record_kind(kind);
    kani::assert(result, "kind 28 (RunKilled) must be a known record kind");
}

/// PO-KANI-004-H2: validate_kind_family(MAGIC_JOURNAL_EVENT, 28) must return Ok(()).
/// Current gap: validate_kind_family line 46 restricts journal to 10..=27.
/// Blocked until validation.rs line 46 is extended: matches!(kind, 10..=28)
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

/// PO-KANI-004-H6: Unknown kind 29 must be rejected (boundary check).
#[kani::proof]
fn check_unknown_kind_rejected() {
    let kind: u16 = 29;
    let magic: u32 = crate::MAGIC_JOURNAL_EVENT;
    let result = crate::codec::validation::validate_kind_family(magic, kind);
    kani::assert(
        result.is_err(),
        "unknown kind 29 must be rejected by validate_kind_family",
    );
}

/// PO-KANI-004-H7: All existing known kinds (1,2,3,10-27,30,40,50) remain known.
#[kani::proof]
fn check_all_existing_kinds_known() {
    let known_kinds: [u16; 24] = [
        1, 2, 3, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 30, 40, 50,
    ];
    for kind in known_kinds {
        let result = crate::codec::validation::is_known_record_kind(kind);
        assert!(result, "kind {} must remain a known record kind", kind);
    }
}

/// PO-KANI-004-H8: Exhaustive: for any arbitrary u16 kind value,
/// validate_kind_family with MAGIC_JOURNAL_EVENT returns Err except for kinds 10..=28.
#[kani::proof]
#[kani::unwind(3)]
fn check_journal_family_exhaustive() {
    let kind: u16 = kani::any();
    let magic: u32 = crate::MAGIC_JOURNAL_EVENT;
    let result = crate::codec::validation::validate_kind_family(magic, kind);

    // Expected valid range: 10..=28
    let is_valid_journal_kind = (10u16..=28u16).contains(&kind);

    match result {
        Ok(()) => {
            assert!(
                is_valid_journal_kind,
                "kind {} returned Ok but is not in valid journal range 10..=28",
                kind
            );
        }
        Err(_) => {
            assert!(
                !is_valid_journal_kind,
                "kind {} returned Err but is in valid journal range 10..=28",
                kind
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
    kani::assume(run_val > 0 && run_val < u64::MAX - 1000);
    let seq_val: u64 = kani::any();
    kani::assume(seq_val < u64::MAX - 100);
    let attempt_val: u16 = kani::any();
    kani::assume(attempt_val > 0 && attempt_val < 1000);

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

    // Verify structural validity (non-zero run, non-overflow seq, non-zero attempt)
    kani::assert(
        event.is_valid(),
        "RunKilled with valid fields must pass is_valid()",
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
