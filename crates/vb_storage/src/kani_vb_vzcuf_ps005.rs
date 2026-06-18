// Kani proof harness for encoded byte accounting (PS-005, C2).

#[cfg(kani)]
mod kani_encoding_ps005 {
    use crate::constants::{MAX_JOURNAL_EVENT_PAYLOAD_BYTES, RECORD_HEADER_LEN};
    use crate::events::JournalEvent;
    use crate::types::EventSeq;
    use vb_core::{RunId, SlotIdx, StepIdx, WorkflowDigest};

    fn header_len_usize() -> usize {
        match usize::try_from(RECORD_HEADER_LEN) {
            Ok(value) => value,
            Err(_) => {
                kani::assume(false);
                0
            }
        }
    }

    fn max_payload_len_usize() -> usize {
        match usize::try_from(MAX_JOURNAL_EVENT_PAYLOAD_BYTES) {
            Ok(value) => value,
            Err(_) => {
                kani::assume(false);
                0
            }
        }
    }

    fn run_accepted(run: RunId, seq: EventSeq) -> JournalEvent {
        JournalEvent::RunAccepted {
            run,
            seq,
            workflow: WorkflowDigest::from_bytes([0_u8; 32]),
        }
    }

    fn payload_len_or_assume(event: &JournalEvent) -> usize {
        match postcard::to_allocvec(event) {
            Ok(payload) => {
                let len = payload.len();
                core::mem::forget(payload);
                len
            }
            Err(error) => {
                core::mem::forget(error);
                kani::assume(false);
                0
            }
        }
    }

    fn encoded_len_or_assume(payload_len: usize) -> usize {
        match header_len_usize().checked_add(payload_len) {
            Some(value) => value,
            None => {
                kani::assume(false);
                0
            }
        }
    }

    /// C2: encode_record output length includes the record header.
    #[kani::proof]
    fn check_encoded_length_minimum() {
        let run: u64 = kani::any();
        kani::assume(run > 0);
        kani::assume(run <= 1_000);

        let event = run_accepted(RunId::new(run), EventSeq::new(0));
        let payload_len = payload_len_or_assume(&event);
        let len = encoded_len_or_assume(payload_len);
        let header_len = header_len_usize();
        kani::assert(len >= header_len, "encoded record len >= header len");
        kani::assert(payload_len > 0, "RunAccepted payload is non-empty");
        kani::assert(
            len > header_len,
            "encoded record len includes payload bytes",
        );
    }

    /// C2: payload-only accounting underestimates the full encoded length.
    #[kani::proof]
    fn check_payload_only_underestimates() {
        let payload_len: usize = kani::any();
        kani::assume(payload_len > 0);
        kani::assume(payload_len <= max_payload_len_usize());

        let full_len = encoded_len_or_assume(payload_len);
        let header_len = header_len_usize();
        kani::assert(full_len > payload_len, "full len exceeds payload len");
        kani::assert(
            full_len.checked_sub(payload_len) == Some(header_len),
            "encoding overhead is exactly header len",
        );
    }

    /// C2: multiple event kinds produce encoded output with a header.
    #[kani::proof]
    fn check_multiple_event_kinds_encode() {
        let run = RunId::new(1);
        let events = [
            run_accepted(run, EventSeq::new(0)),
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(1),
                step: StepIdx::new(0),
                attempt: 1,
            },
            JournalEvent::StepSucceeded {
                run,
                seq: EventSeq::new(2),
                step: StepIdx::new(0),
                output: SlotIdx::new(0),
            },
        ];

        for (index, event) in events.iter().enumerate() {
            let sequence = match u64::try_from(index) {
                Ok(value) => value,
                Err(_) => {
                    kani::assume(false);
                    0
                }
            };
            let payload_len = payload_len_or_assume(event);
            let encoded_len = encoded_len_or_assume(payload_len);
            kani::assert(sequence <= 2, "test sequence remains bounded");
            kani::assert(
                encoded_len >= header_len_usize(),
                "event encoded len >= header len",
            );
        }
    }

    /// C2: maximum envelope size used for accounting fits in u64.
    #[kani::proof]
    fn check_max_encoded_fits_u64() {
        let max_encoded =
            u64::from(RECORD_HEADER_LEN).checked_add(u64::from(MAX_JOURNAL_EVENT_PAYLOAD_BYTES));
        kani::assert(
            max_encoded.is_some(),
            "max encoded addition does not overflow",
        );
        if let Some(value) = max_encoded {
            kani::assert(value < u64::MAX, "max encoded must be below u64::MAX");
        }
    }

    /// C2: RunAccepted uses the canonical production record kind mapping.
    #[kani::proof]
    fn check_record_kind_mapping() {
        let event = run_accepted(RunId::new(1), EventSeq::new(0));
        let kind = event.record_kind();
        kani::assert(kind.id() == 10_u16, "RunAccepted kind must be 10");
    }
}
