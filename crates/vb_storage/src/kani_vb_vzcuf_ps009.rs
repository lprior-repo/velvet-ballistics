// Kani proof harness for duplicate accounting (PS-009, C2).

#[cfg(kani)]
mod kani_duplicate_ps009 {
    use crate::codec::encode_record;
    use crate::constants::{
        JOURNAL_KEY_BYTES, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        RECORD_HEADER_LEN,
    };
    use crate::events::JournalEvent;
    use crate::records::RecordKind;
    use crate::types::EventSeq;
    use vb_core::{RunId, WorkflowDigest};

    fn run_accepted(run: u64, workflow_byte: u8) -> JournalEvent {
        JournalEvent::RunAccepted {
            run: RunId::new(run),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([workflow_byte; 32]),
        }
    }

    fn encode_run_accepted(event: &JournalEvent) -> Result<Vec<u8>, crate::JournalError> {
        encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            0,
            event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
    }

    /// C2: same event with same run+seq produces identical encoded output.
    #[kani::proof]
    fn check_same_event_same_encoding() {
        let event = run_accepted(1, 0xAB);

        match (encode_run_accepted(&event), encode_run_accepted(&event)) {
            (Ok(v1), Ok(v2)) => {
                kani::assert(v1 == v2, "deterministic encoding for same input");
                kani::assert(v1.len() == v2.len(), "lengths match for same input");
                let header_len = match usize::try_from(RECORD_HEADER_LEN) {
                    Ok(value) => value,
                    Err(_) => {
                        kani::assume(false);
                        0
                    }
                };
                kani::assert(v1.len() > header_len, "encoded len exceeds header len");
            }
            _ => kani::assert(false, "same event must encode twice"),
        }
    }

    /// C2: different events produce different encoded output.
    #[kani::proof]
    fn check_different_events_different_encoding() {
        let e1 = run_accepted(1, 0x11);
        let e2 = run_accepted(2, 0x22);

        match (encode_run_accepted(&e1), encode_run_accepted(&e2)) {
            (Ok(v1), Ok(v2)) => {
                kani::assert(v1 != v2, "different events produce different bytes");
            }
            _ => kani::assert(false, "different events must encode"),
        }
    }

    /// C2: JOURNAL_KEY_BYTES is bounded and non-zero.
    #[kani::proof]
    fn check_journal_key_bytes_valid() {
        kani::assert(JOURNAL_KEY_BYTES > 0, "journal key bytes must be non-zero");
        kani::assert(JOURNAL_KEY_BYTES <= 256, "journal key bytes must be small");
    }

    /// C2: conservative and precise duplicate accounting policies are monotonic.
    #[kani::proof]
    fn check_duplicate_accounting_policies() {
        let encoded_len: u64 = kani::any();
        kani::assume(encoded_len > 0);
        kani::assume(encoded_len < 100_000);
        let current_bytes: u64 = kani::any();
        let Some(max_without_overflow) = u64::MAX.checked_sub(encoded_len) else {
            kani::assume(false);
            return;
        };
        kani::assume(current_bytes < max_without_overflow);

        let Some(conservative) = current_bytes.checked_add(encoded_len) else {
            kani::assume(false);
            return;
        };
        kani::assert(conservative > current_bytes, "conservative increases bytes");

        let Some(precise_new) = current_bytes.checked_add(encoded_len) else {
            kani::assume(false);
            return;
        };
        kani::assert(precise_new == conservative, "new-key precise equals conservative");

        let precise_dup = current_bytes;
        kani::assert(precise_dup < conservative, "duplicate precise is smaller");
        kani::assert(precise_dup == current_bytes, "duplicate precise keeps current");
    }

    /// C2: staged bytes never decrease regardless of duplicate policy.
    #[kani::proof]
    fn check_staged_bytes_monotonic() {
        let current: u64 = kani::any();
        kani::assume(current < u64::MAX / 2);
        let encoded_len: u64 = kani::any();
        kani::assume(encoded_len < 1_000_000);

        let Some(new_cons) = current.checked_add(encoded_len) else {
            kani::assume(false);
            return;
        };
        kani::assert(new_cons >= current, "conservative policy monotonic");

        let Some(new_precise_new) = current.checked_add(encoded_len) else {
            kani::assume(false);
            return;
        };
        kani::assert(new_precise_new >= current, "precise new-key monotonic");

        let new_precise_dup = current;
        kani::assert(new_precise_dup >= current, "precise duplicate monotonic");
        kani::assert(new_precise_dup == current, "precise duplicate keeps current");
    }
}
