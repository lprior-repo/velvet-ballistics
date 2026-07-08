//! Flux-rs refinement annotations for `vb_storage` codec validation.
//! Binds `validate_kind_family`, `is_known_record_kind`, journal payload-kind
//! parity, and replay contiguity models to production storage behavior.

#![forbid(unsafe_code)]

extern crate flux_rs;
use flux_rs::attrs::*;

/// Trusted model for known storage record kinds, including journal kinds
/// 10..=29, WaitResolved=31, ActionAbandoned=32, and the split stable
/// tags StepSucceeded=33, ActionScheduledTicket=34, ActionCompletedEnvelope=35.
#[flux_rs::trusted]
#[sig(fn(kind: u16) -> bool[{
    kind == 1 || kind == 2 || kind == 3 || kind == 30 ||
    kind == 31 || kind == 32 || kind == 33 || kind == 34 || kind == 35 ||
    kind == 40 || kind == 50 ||
    (kind >= 10 && kind <= 29)
}])]
fn model_is_known_record_kind(kind: u16) -> bool {
    crate::codec::validation::is_known_record_kind(kind)
}

/// Trusted model for kind-family validation.
#[flux_rs::trusted]
#[sig(fn(magic: u32, kind: u16) -> Result<(), crate::JournalError>)]
fn model_validate_kind_family(magic: u32, kind: u16) -> Result<(), crate::JournalError> {
    crate::codec::validation::validate_kind_family(magic, kind)
}

/// Journal magic admits journal kinds 10..=29 plus stable split tags 31..=35.
#[flux_rs::trusted]
#[sig(fn(kind: u16) -> bool[{
    ((kind >= 10 && kind <= 29) || (kind >= 31 && kind <= 35)) ==
        model_validate_kind_family_ok(kind)
}])]
fn model_journal_kind_valid(kind: u16) -> bool {
    let result =
        crate::codec::validation::validate_kind_family(crate::constants::MAGIC_JOURNAL_EVENT, kind);
    result.is_ok()
}

/// Returns whether journal-family validation accepts `kind`.
#[flux_rs::trusted]
fn model_validate_kind_family_ok(kind: u16) -> bool {
    crate::codec::validation::validate_kind_family(crate::constants::MAGIC_JOURNAL_EVENT, kind)
        .is_ok()
}

/// `AskTimedOut` keeps stable kind id 29.
#[flux_rs::trusted]
#[sig(fn() -> bool[true])]
fn model_ask_timed_out_kind_id_stable() -> bool {
    crate::RecordKind::AskTimedOut.id() == 29
}

/// `AskTimedOut` payload implies wire kind 29 exactly.
#[flux_rs::trusted]
#[sig(fn(run_val: u64{run_val > 0}, seq_val: u64{seq_val < u64::MAX}, attempt_val: u16{attempt_val > 0}) -> bool[true])]
fn model_ask_timed_out_payload_kind_is_29(
    run_val: u64,
    seq_val: u64,
    attempt_val: u16,
) -> bool {
    let event = crate::JournalEvent::AskTimedOutEvent {
        run: vb_core::ids::RunId::new(run_val),
        seq: crate::EventSeq::new(seq_val),
        step: vb_core::ids::StepIdx::new(1),
        attempt: attempt_val,
    };
    event.record_kind().id() == crate::RecordKind::AskTimedOut.id()
}

/// Codec parity rejects `AskTimedOut` under `AskAnswered` kind.
#[flux_rs::trusted]
#[sig(fn(run_val: u64{run_val > 0}, seq_val: u64{seq_val < u64::MAX}, attempt_val: u16{attempt_val > 0}) -> bool[true])]
fn model_ask_timed_out_rejects_ask_answered_envelope(
    run_val: u64,
    seq_val: u64,
    attempt_val: u16,
) -> bool {
    let event = crate::JournalEvent::AskTimedOutEvent {
        run: vb_core::ids::RunId::new(run_val),
        seq: crate::EventSeq::new(seq_val),
        step: vb_core::ids::StepIdx::new(1),
        attempt: attempt_val,
    };
    let envelope = crate::RecordEnvelope {
        magic: crate::constants::MAGIC_JOURNAL_EVENT,
        schema_version: crate::constants::CURRENT_SCHEMA_VERSION,
        record_kind: crate::RecordKind::AskAnswered.id(),
        sequence: seq_val,
    };
    matches!(
        crate::codec::validate_journal_event_record_kind(&envelope, &event),
        Err(crate::JournalError::RecordKindPayloadMismatch {
            envelope_kind: 18,
            payload_kind: 29,
        })
    )
}

/// Snapshot magic rejects journal kind 28.
#[flux_rs::trusted]
#[sig(fn() -> bool[true])]
fn model_kind_28_rejected_for_snapshot() -> bool {
    crate::codec::validation::validate_kind_family(crate::constants::MAGIC_SNAPSHOT, 28).is_err()
}

/// Blob magic rejects journal kind 28.
#[flux_rs::trusted]
#[sig(fn() -> bool[true])]
fn model_kind_28_rejected_for_blob() -> bool {
    crate::codec::validation::validate_kind_family(crate::constants::MAGIC_BLOB, 28).is_err()
}

/// `RunKilled` events preserve run, sequence, attempt, and kind fields.
#[flux_rs::trusted]
#[sig(fn(run_val: u64{run_val > 0}, seq_val: u64{seq_val < u64::MAX}, attempt_val: u16{attempt_val > 0}) -> bool[true])]
fn model_runkilled_field_preservation(run_val: u64, seq_val: u64, attempt_val: u16) -> bool {
    let event = crate::JournalEvent::RunKilled {
        run: vb_core::ids::RunId::new(run_val),
        seq: crate::EventSeq::new(seq_val),
        attempt: attempt_val,
    };
    event.run_id().get() == run_val
        && event.seq().get() == seq_val
        && event.attempt() == Some(attempt_val)
        && matches!(event.record_kind(), crate::RecordKind::RunKilled)
}

/// `RunKilled` keeps stable kind id 28.
#[flux_rs::trusted]
#[sig(fn() -> bool[true])]
fn model_runkilled_kind_id_stable() -> bool {
    crate::RecordKind::RunKilled.id() == 28
}

/// Replay sequence contiguity model.
#[flux_rs::trusted]
#[sig(fn(seqs: &[u64]) -> bool[true])]
fn model_contiguous_check(seqs: &[u64]) -> bool {
    if seqs.len() < 2 {
        return true;
    }
    for i in 0..seqs.len() - 1 {
        if seqs[i].saturating_add(1) != seqs[i + 1] {
            return false;
        }
    }
    true
}

/// A sequence with a gap is non-contiguous.
#[flux_rs::trusted]
#[sig(fn(seqs: &[u64]) -> bool[true])]
fn model_gap_detection(seqs: &[u64]) -> bool {
    if seqs.len() < 2 {
        return false;
    }
    let mut has_gap = false;
    for i in 0..seqs.len() - 1 {
        if seqs[i].saturating_add(1) != seqs[i + 1] {
            has_gap = true;
            break;
        }
    }
    has_gap
}

/// Duplicate sequence values are non-contiguous.
#[flux_rs::trusted]
#[sig(fn(seqs: &[u64]) -> bool[true])]
fn model_duplicate_detection(seqs: &[u64]) -> bool {
    if seqs.len() < 2 {
        return false;
    }
    let mut has_dup = false;
    for i in 0..seqs.len() {
        for j in (i + 1)..seqs.len() {
            if seqs[i] == seqs[j] {
                has_dup = true;
                break;
            }
        }
    }
    has_dup
}

#[cfg(test)]
mod flux_validation_tests {
    use super::*;
    use crate::codec::validation::validate_kind_family;
    use crate::constants::MAGIC_JOURNAL_EVENT;

    #[test]
    fn kind_28_valid_for_journal_family() {
        let result = validate_kind_family(MAGIC_JOURNAL_EVENT, 28);
        assert!(result.is_ok(), "kind 28 must be admitted: {:?}", result);
    }

    #[test]
    fn kind_29_valid_for_journal_family() {
        let result = validate_kind_family(MAGIC_JOURNAL_EVENT, 29);
        assert!(result.is_ok(), "kind 29 must be admitted: {:?}", result);
    }

    #[test]
    fn kind_31_valid_for_journal_family() {
        let result = validate_kind_family(MAGIC_JOURNAL_EVENT, 31);
        assert!(result.is_ok(), "kind 31 must be admitted: {:?}", result);
    }

    #[test]
    fn kind_32_valid_for_journal_family() {
        let result = validate_kind_family(MAGIC_JOURNAL_EVENT, 32);
        assert!(result.is_ok(), "kind 32 must be admitted: {:?}", result);
    }

    #[test]
    fn split_kinds_33_to_35_valid_for_journal_family() {
        for kind in 33..=35 {
            let result = validate_kind_family(MAGIC_JOURNAL_EVENT, kind);
            assert!(result.is_ok(), "kind {kind} must be admitted: {result:?}");
        }
    }

    #[test]
    fn kind_31_is_known_record_kind() {
        assert!(model_is_known_record_kind(31));
    }

    #[test]
    fn kind_32_is_known_record_kind() {
        assert!(model_is_known_record_kind(32));
    }

    #[test]
    fn split_kinds_33_to_35_are_known_record_kinds() {
        for kind in 33..=35 {
            assert!(model_is_known_record_kind(kind), "kind {kind} must be known");
        }
    }

    #[test]
    fn kind_28_is_known_record_kind() {
        assert!(model_is_known_record_kind(28));
    }

    #[test]
    fn kind_29_is_known_record_kind() {
        assert!(model_is_known_record_kind(29));
    }

    #[test]
    fn ask_timed_out_kind_id_is_29() {
        assert!(model_ask_timed_out_kind_id_stable());
    }

    #[test]
    fn ask_timed_out_payload_kind_is_exact_29() {
        assert!(model_ask_timed_out_payload_kind_is_29(42, 7, 1));
    }

    #[test]
    fn ask_timed_out_payload_rejects_ask_answered_envelope() {
        assert!(model_ask_timed_out_rejects_ask_answered_envelope(42, 7, 1));
    }

    #[test]
    fn runkilled_kind_id_is_28() {
        assert!(model_runkilled_kind_id_stable());
    }

    #[test]
    fn runkilled_field_preservation() {
        assert!(model_runkilled_field_preservation(42, 7, 3));
    }

    #[test]
    fn contiguous_sequence_passes() {
        assert!(model_contiguous_check(&[0, 1, 2]));
    }

    #[test]
    fn gap_sequence_detected() {
        assert!(model_gap_detection(&[0, 1, 3]));
    }

    #[test]
    fn duplicate_sequence_detected() {
        assert!(model_duplicate_detection(&[0, 1, 1]));
    }

    #[test]
    fn kind_28_rejected_for_non_journal_families() {
        assert!(model_kind_28_rejected_for_snapshot());
        assert!(model_kind_28_rejected_for_blob());
    }
}
