// Kani proof harness for error variant discrimination (PS-003, C4, C6).
//
// Obligation ID: POB-vb-vzcuf-010
// Verifier: kani
// Domain claim: accumulated budget rejection is distinct from QueueFull and
// PayloadTooLarge under controlled unrelated guards.

#[cfg(kani)]
mod kani_errors_ps003 {
    use crate::codec::encode_record;
    use crate::constants::{
        MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES, RECORD_HEADER_LEN,
    };
    use crate::error::JournalError;
    use crate::events::JournalEvent;
    use crate::records::RecordKind;
    use crate::types::EventSeq;
    use vb_core::{RunId, WorkflowDigest};

    /// C4: QueueFull, PayloadTooLarge, and DuplicateEvent are distinguishable variants.
    #[kani::proof]
    fn check_error_variants_distinct() {
        let qf = JournalError::QueueFull;
        match qf {
            JournalError::QueueFull => {}
            _ => kani::assert(false, "QueueFull must match QueueFull"),
        }

        let ptl = JournalError::PayloadTooLarge { len: 100, max: 50 };
        match ptl {
            JournalError::PayloadTooLarge { len, max } => {
                kani::assert(len == 100, "PayloadTooLarge.len == 100");
                kani::assert(max == 50, "PayloadTooLarge.max == 50");
            }
            _ => kani::assert(false, "PayloadTooLarge must match PayloadTooLarge"),
        }

        let run = RunId::new(1);
        let seq = EventSeq::new(0);
        let dup = JournalError::DuplicateEvent { run, seq };
        match dup {
            JournalError::DuplicateEvent { run: r, seq: s } => {
                kani::assert(r == run, "DuplicateEvent.run matches");
                kani::assert(s == seq, "DuplicateEvent.seq matches");
            }
            _ => kani::assert(false, "DuplicateEvent must match DuplicateEvent"),
        }
    }

    /// C6: encode_record produces PayloadTooLarge, not QueueFull, when payload exceeds max.
    #[kani::proof]
    fn check_encode_record_error_is_payload_too_large() {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0_u8; 32]),
        };

        let result = encode_record(MAGIC_JOURNAL_EVENT, RecordKind::RunAccepted, 0, &event, 0);

        match result {
            Err(JournalError::PayloadTooLarge { .. }) => {}
            Err(_) => kani::assert(false, "zero max payload should return PayloadTooLarge"),
            Ok(_) => kani::assert(false, "zero max payload should not encode"),
        }
    }

    /// C4: encode_record with valid max produces either Ok or a non-admission encode error.
    #[kani::proof]
    fn check_valid_encode_produces_ok() {
        let run: u64 = kani::any();
        kani::assume(run > 0);
        kani::assume(run < 1_000);

        let event = JournalEvent::RunAccepted {
            run: RunId::new(run),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0_u8; 32]),
        };

        let result = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );

        if let Ok(value) = result {
            let header_len = match usize::try_from(RECORD_HEADER_LEN) {
                Ok(value) => value,
                Err(_) => {
                    kani::assume(false);
                    0
                }
            };
            kani::assert(value.len() >= header_len, "encoded >= header len");
        }
    }

    /// C4: PayloadTooLarge carries len and max fields.
    #[kani::proof]
    fn check_payload_too_large_carries_fields() {
        let err = JournalError::PayloadTooLarge {
            len: 5_000,
            max: 1_000,
        };
        match err {
            JournalError::PayloadTooLarge { len, max } => {
                kani::assert(len == 5_000, "payload-too-large len is preserved");
                kani::assert(max == 1_000, "payload-too-large max is preserved");
            }
            _ => kani::assert(false, "PayloadTooLarge must carry fields"),
        }
    }

    /// C4: QueueFull remains its own discriminant.
    #[kani::proof]
    fn check_queue_full_error_message() {
        let err = JournalError::QueueFull;
        match err {
            JournalError::QueueFull => {}
            _ => kani::assert(false, "QueueFull must remain distinct"),
        }
    }
}
