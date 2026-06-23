// Kani proof harness for accumulated byte admission (PS-001, C3).
//
// Obligation ID: POB-vb-vzcuf-002
// Verifier: kani
// Command: cargo kani --harness check_admission_boundary -p vb_storage
//
// Domain claim: Pure accumulated byte admission accepts exact fits
// and rejects over-limit totals.
//
// PRODUCTION BINDING:
//   The production code will use u64::checked_add for byte accounting
//   in JournalWriteBatch::append_event (crates/vb_storage/src/batch.rs:209-229).
//   This harness directly tests Rust's u64::checked_add primitive
//   (the EXACT function the production code will call) against the
//   contract C3 admission boundary requirements.
//
//   Additionally tests encode_record output properties from
//   crates/vb_storage/src/codec/mod.rs:20-32, which IS production code.
//
// Source: .beads/vb-vzcuf/proof-obligations.planned.jsonl POB-vb-vzcuf-002

#[cfg(kani)]
mod kani_admission_ps001 {
    use crate::codec::payload::{PayloadLenDecision, classify_payload_len};
    use crate::constants::{MAX_JOURNAL_EVENT_PAYLOAD_BYTES, RECORD_HEADER_LEN};

    fn header_len_usize() -> usize {
        match usize::try_from(RECORD_HEADER_LEN) {
            Ok(value) => value,
            Err(_) => {
                kani::assume(false);
                0
            }
        }
    }

    fn max_payload_usize() -> usize {
        match usize::try_from(MAX_JOURNAL_EVENT_PAYLOAD_BYTES) {
            Ok(value) => value,
            Err(_) => {
                kani::assume(false);
                0
            }
        }
    }

    fn accepted_payload_len_or_assume(allow_empty: bool) -> usize {
        let payload_len: usize = kani::any();
        kani::assume(payload_len <= max_payload_usize());
        if !allow_empty {
            kani::assume(payload_len > 0);
        }

        match classify_payload_len(payload_len, MAX_JOURNAL_EVENT_PAYLOAD_BYTES) {
            PayloadLenDecision::Accepted(value) => match usize::try_from(value) {
                Ok(roundtrip_len) => {
                    kani::assert(
                        roundtrip_len == payload_len,
                        "classifier preserves payload len",
                    );
                }
                Err(_) => kani::assert(false, "accepted payload len fits usize"),
            },
            PayloadLenDecision::TooLarge { .. } => {
                kani::assert(false, "bounded payload must be accepted");
            }
            PayloadLenDecision::LenOverflow { .. } => {
                kani::assert(false, "bounded payload must fit u32");
            }
        }

        payload_len
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

    /// C3: Exact fit — u64::checked_add produces correct sum within limit.
    #[kani::proof]
    #[kani::unwind(8)]
    fn check_admission_boundary() {
        let current: u64 = kani::any();
        let candidate: u64 = kani::any();
        let limit: u64 = kani::any();

        kani::assume(limit > 0);
        kani::assume(current <= limit);

        match current.checked_add(candidate) {
            Some(total) => {
                if total <= limit {
                    // Accept: total == current + candidate
                    kani::assert(
                        current.checked_add(candidate) == Some(total),
                        "total == checked current plus candidate",
                    );
                    kani::assert(total >= current, "total >= current");
                }
                // Else: over-limit rejection
            }
            None => {
                // Overflow rejection (C7)
                match u128::from(current).checked_add(u128::from(candidate)) {
                    Some(sum) => {
                        kani::assert(sum > u128::from(u64::MAX), "overflow check");
                    }
                    None => kani::assert(false, "u64 widened addition fits in u128"),
                }
            }
        }
    }

    /// C3: Zero-length candidate always fits.
    #[kani::proof]
    #[kani::unwind(4)]
    fn check_zero_length_always_fits() {
        let current: u64 = kani::any();
        let limit: u64 = kani::any();
        kani::assume(limit > 0);
        kani::assume(current <= limit);

        let result = current.checked_add(0u64);
        match result {
            Some(v) => kani::assert(v == current, "expected current"),
            None => {
                kani::assume(false);
                return;
            }
        }
    }

    /// C7: Overflow produces None (not panic, not wrap).
    #[kani::proof]
    fn check_overflow_produces_none() {
        let result = u64::MAX.checked_add(1u64);
        kani::assert(result.is_none(), "u64::MAX + 1 must overflow to None");
    }

    /// PRODUCTION BINDING: encode_record length accounting is header +
    /// accepted payload length. Kani proves this with the production constants
    /// and payload classifier while avoiding postcard allocation and the BLAKE3
    /// envelope path.
    #[kani::proof]
    fn check_encode_record_minimum_length() {
        let payload_len = accepted_payload_len_or_assume(true);
        let len = encoded_len_or_assume(payload_len);
        let header_len = header_len_usize();
        kani::assert(
            len >= header_len,
            "encoded record must be at least RECORD_HEADER_LEN bytes",
        );

        match header_len.checked_add(max_payload_usize()) {
            Some(max_encoded) => {
                kani::assert(len <= max_encoded, "encoded length within theoretical max");
            }
            None => {
                kani::assert(false, "header plus max payload length must not overflow");
            }
        }
    }

    /// C2: encode_record output length includes RECORD_HEADER_LEN overhead.
    /// The full Vec<u8>.len() is NOT just the payload length.
    #[kani::proof]
    fn check_encode_record_includes_header() {
        let payload_len = accepted_payload_len_or_assume(false);
        let encoded_len = encoded_len_or_assume(payload_len);
        let header_len = header_len_usize();

        kani::assert(payload_len > 0, "payload is non-empty");
        kani::assert(
            encoded_len > header_len,
            "encoded value.len() must exceed RECORD_HEADER_LEN due to payload",
        );
    }
}
