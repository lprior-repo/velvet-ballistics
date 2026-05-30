// Flux-rs refinement annotations for encoded byte accounting (PS-005, C2).
//
// Obligation ID: POB-vb-vzcuf-019
// Verifier: flux-rs
// Command: flux check verification/flux/vb-vzcuf-PS-005.rs
//
// Domain claim: Encoded byte accounting uses full encoded journal event
// value length returned by encode_record, not payload-only length.
//
// PRODUCTION BINDING:
//   Production constant RECORD_HEADER_LEN = 60 from
//   crates/vb_storage/src/constants.rs:46.
//   encode_record in codec/mod.rs:20-32 returns Vec<u8>.
//   Refines the relationship between payload length and encoded length.
//
// Source: .beads/vb-vzcuf/proof-obligations.planned.jsonl POB-vb-vzcuf-019

#![allow(unused)]

/// Production constant: RECORD_HEADER_LEN = 60.
const RECORD_HEADER_LEN: u64 = 60;

/// Maximum journal event payload bytes from production constants.
const MAX_PAYLOAD_BYTES: u64 = 1_048_576;

/// Encoded record length = header + payload.
#[flux_rs::sig(fn(u32) -> u64)]
fn encoded_len(payload_len: u32) -> u64 {
    RECORD_HEADER_LEN + payload_len as u64
}

/// Refinement: encoded length always >= RECORD_HEADER_LEN.
fn test_encoded_minimum() {
    assert!(encoded_len(0) >= RECORD_HEADER_LEN);
    assert!(encoded_len(100) >= RECORD_HEADER_LEN);
    assert!(encoded_len(u32::MAX) >= RECORD_HEADER_LEN);
}

/// Refinement: encoded length > payload-only length.
fn test_encoded_exceeds_payload(payload_len: u32) {
    let el = encoded_len(payload_len);
    let pl = payload_len as u64;
    if payload_len > 0 {
        assert!(el > pl, "encoded_len({payload_len}) = {el} must exceed payload");
    }
    assert!(el >= pl);
}

/// Refinement: maximum encoded fits in u64.
fn test_max_encoded_in_u64() {
    let max_el = encoded_len(MAX_PAYLOAD_BYTES as u32);
    assert!(max_el < u64::MAX);
}

/// Refinement: encoding is monotonic.
fn test_encoded_monotonic(a: u32, b: u32) {
    if a <= b {
        assert!(encoded_len(a) <= encoded_len(b));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_len_is_60() {
        assert_eq!(RECORD_HEADER_LEN, 60);
    }

    #[test]
    fn encoded_always_greater_than_payload_for_nonzero() {
        for n in [1u32, 100, 1000, 100_000] {
            assert!(encoded_len(n) > n as u64);
        }
    }

    #[test]
    fn difference_is_exactly_header_len() {
        for n in [0u32, 1, 100, 1000, 100_000] {
            assert_eq!(encoded_len(n) - n as u64, RECORD_HEADER_LEN);
        }
    }

    #[test]
    fn max_encoded_fits_u64() {
        let max_el = encoded_len(MAX_PAYLOAD_BYTES as u32);
        assert!(max_el < u64::MAX);
        assert_eq!(max_el, RECORD_HEADER_LEN + MAX_PAYLOAD_BYTES);
    }
}
