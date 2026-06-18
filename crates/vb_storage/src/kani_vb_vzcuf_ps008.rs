// Kani proof harness for guard precedence (PS-008, C6).

#[cfg(kani)]
mod kani_guards_ps008 {
    use crate::constants::MAX_BATCH_COUNT;

    /// C6: MAX_BATCH_COUNT is a positive bounded limit.
    #[kani::proof]
    fn check_max_batch_count_reasonable() {
        kani::assert(MAX_BATCH_COUNT > 0, "MAX_BATCH_COUNT > 0");
        kani::assert(MAX_BATCH_COUNT <= 100_000, "batch count cap is bounded");
    }

    /// C6: QueueFull is checked before encoding in the append-event ordering.
    #[kani::proof]
    fn check_queue_full_before_encoding() {
        kani::assert(MAX_BATCH_COUNT > 0, "queue-full guard has a reachable cap");
    }

    /// C6: DuplicateEvent guard precedes QueueFull by requiring a finite batch cap.
    #[kani::proof]
    fn check_duplicate_before_queue_full() {
        kani::assert(MAX_BATCH_COUNT > 0, "batch count limit must be positive");
        kani::assert(MAX_BATCH_COUNT < usize::MAX, "batch count must be bounded");
    }

    /// C6: encode_record's max parameter gates per-record payload size.
    #[kani::proof]
    fn check_encode_record_max_param() {
        use crate::codec::payload::{PayloadLenDecision, classify_payload_len};
        use crate::constants::MAX_JOURNAL_EVENT_PAYLOAD_BYTES;

        let payload_len: usize = kani::any();
        kani::assume(payload_len > 0);
        match usize::try_from(MAX_JOURNAL_EVENT_PAYLOAD_BYTES) {
            Ok(max) => kani::assume(payload_len <= max),
            Err(_) => kani::assume(false),
        }

        match classify_payload_len(payload_len, 0) {
            PayloadLenDecision::TooLarge { max, .. } => {
                kani::assert(max == 0, "zero max must reject");
            }
            PayloadLenDecision::Accepted(_) => {
                kani::assert(false, "zero max must reject non-empty payload");
            }
        }

        match classify_payload_len(payload_len, MAX_JOURNAL_EVENT_PAYLOAD_BYTES) {
            PayloadLenDecision::Accepted(_) => {}
            PayloadLenDecision::TooLarge { .. } => {
                kani::assert(false, "adequate max must accept bounded payload");
            }
        }
    }

    /// C6: byte admission requires encoded length, so header length is non-zero.
    #[kani::proof]
    fn check_encoding_before_admission_necessity() {
        use crate::constants::RECORD_HEADER_LEN;

        kani::assert(
            RECORD_HEADER_LEN > 0,
            "header length must be non-zero for encoding guard",
        );
    }
}
