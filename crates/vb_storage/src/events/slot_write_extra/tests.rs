use super::*;
use crate::slot_extra::SlotWrittenExtraEnvelope;
use vb_core::Taint;

#[test]
fn parse_rejects_empty_bytes() {
    let err = SlotWriteExtra::parse(&[]).expect_err("empty bytes must fail");
    assert!(matches!(err, JournalError::InvalidEvent));
}

#[test]
fn parse_round_trips_versioned_envelope() {
    let envelope = SlotWrittenExtraEnvelope {
        taint: Taint::Clean,
        frame_extra: Some(vec![1, 2, 3]),
    };
    let bytes = encode_slot_written_extra(envelope.taint, envelope.frame_extra.clone())
        .expect("encode should succeed");
    let parsed = SlotWriteExtra::parse(&bytes).expect("versioned parse should succeed");
    assert_eq!(parsed, SlotWriteExtra::Versioned(envelope));
}

#[test]
fn parse_classifies_legacy_payload() {
    let legacy = vec![0xAB, 0xCD, 0xEF, 0x42];
    let parsed = SlotWriteExtra::parse(&legacy).expect("legacy parse should succeed");
    assert_eq!(parsed, SlotWriteExtra::Legacy(legacy.clone()));
    assert_eq!(parsed.frame_extra(), Some(legacy.as_slice()));
}

#[test]
fn parse_rejects_corrupt_envelope_payload() {
    let mut bytes = SLOT_WRITTEN_EXTRA_PREFIX.to_vec();
    bytes.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]);
    let err = SlotWriteExtra::parse(&bytes).expect_err("corrupt envelope must fail");
    assert!(matches!(err, JournalError::PostcardDecodeFailed));
}

#[test]
fn versioned_frame_extra_returns_envelope_payload() {
    let envelope = SlotWrittenExtraEnvelope {
        taint: Taint::Secret,
        frame_extra: Some(vec![9, 9, 9]),
    };
    let bytes = encode_slot_written_extra(envelope.taint, envelope.frame_extra.clone())
        .expect("encode should succeed");
    let parsed = SlotWriteExtra::parse(&bytes).expect("parse should succeed");
    assert_eq!(parsed.frame_extra(), Some([9u8, 9, 9].as_slice()));
}

#[test]
fn versioned_without_frame_extra_returns_none() {
    let envelope = SlotWrittenExtraEnvelope {
        taint: Taint::DerivedFromSecret,
        frame_extra: None,
    };
    let bytes = encode_slot_written_extra(envelope.taint, envelope.frame_extra.clone())
        .expect("encode should succeed");
    let parsed = SlotWriteExtra::parse(&bytes).expect("parse should succeed");
    assert_eq!(parsed.frame_extra(), None);
}

#[test]
fn decode_rejects_oversized_envelope_payload() {
    use crate::constants::MAX_FRAME_EXTRA_BYTES;
    use crate::slot_extra::{SLOT_WRITTEN_EXTRA_PREFIX, SlotWrittenExtraError};

    // Build a raw payload just over the cap so the size gate must fire
    // before postcard gets a chance to read the varint length.
    let oversized_len = MAX_FRAME_EXTRA_BYTES + 1;
    let mut bytes = SLOT_WRITTEN_EXTRA_PREFIX.to_vec();
    bytes.extend(std::iter::repeat(0u8).take(oversized_len));

    let err = decode_slot_written_extra(&bytes).expect_err("oversized payload must fail");
    let SlotWrittenExtraError::Oversized { len, max } = err else {
        panic!("oversized payload must return Oversized, got {err:?}");
    };
    assert_eq!(len, oversized_len, "reported len must match payload size");
    assert_eq!(max, MAX_FRAME_EXTRA_BYTES, "reported max must match cap");
}

#[test]
fn decode_accepts_envelope_payload_at_cap_boundary() {
    use crate::constants::MAX_FRAME_EXTRA_BYTES;

    // A real envelope whose post-prefix payload sits just under the
    // cap must still decode; the size gate must NOT fire on
    // cap-sized payloads.
    let envelope = SlotWrittenExtraEnvelope {
        taint: Taint::Clean,
        frame_extra: Some(vec![0u8; MAX_FRAME_EXTRA_BYTES - 16]),
    };
    let bytes = encode_slot_written_extra(envelope.taint, envelope.frame_extra.clone())
        .expect("encode should succeed");
    let payload_len = bytes.len() - SLOT_WRITTEN_EXTRA_PREFIX.len();
    assert!(
        payload_len <= MAX_FRAME_EXTRA_BYTES,
        "encoded payload {payload_len} must be at or under cap {MAX_FRAME_EXTRA_BYTES}"
    );
    assert!(
        payload_len > MAX_FRAME_EXTRA_BYTES / 2,
        "test must exercise the upper end of the cap"
    );

    let result = decode_slot_written_extra(&bytes);
    assert!(
        !matches!(result, Err(SlotWrittenExtraError::Oversized { .. })),
        "size gate must accept cap-sized payload, got {result:?}"
    );
}
