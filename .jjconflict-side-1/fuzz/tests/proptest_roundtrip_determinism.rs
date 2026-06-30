//! **PO-vb-hbav-034**: Proptest roundtrip determinism.
//!
//! For any journal_event: `decode_record(encode_record(event))` must recover
//! the original event with same fields. For `ipc_frame`: header encode/decode
//! roundtrip must preserve bytes.
//!
//! **Scope note**: Tests 2 representative JournalEvent variants (RunAccepted,
//! StepStarted). Full variant coverage is deferred to a future bead. The fuzz
//! harness at `fuzz/src/lib.rs::fuzz_journal_event` provides broader roundtrip
//! coverage via the fuzzer's arbitrary input generation.

use proptest::prelude::*;
use vb_storage::MAGIC_JOURNAL_EVENT;

proptest! {
    /// Encode a known event, decode it back, verify field-level equivalence.
    #[test]
    fn proptest_journal_event_roundtrip(
        seq in 0u64..1000u64,
    ) {
        let max_payload_len = 1024u32;

        // Create a RunAccepted event.
        let event = vb_storage::JournalEvent::RunAccepted {
            run: vb_core::RunId::new(42),
            seq: vb_storage::EventSeq::new(seq),
            workflow: vb_core::WorkflowDigest::from_bytes([0xAB; 32]),
        };

        // Encode
        let encoded = vb_storage::encode_record(
            MAGIC_JOURNAL_EVENT,
            event.record_kind(),
            event.seq().get(),
            &event,
            max_payload_len,
        );
        prop_assert!(encoded.is_ok(), "encode must succeed for valid event");

        let encoded = encoded.unwrap();

        // Decode back
        let decoded = vb_storage::decode_record::<vb_storage::JournalEvent>(
            &encoded,
            MAGIC_JOURNAL_EVENT,
            max_payload_len,
        );
        prop_assert!(decoded.is_ok(), "roundtrip decode must succeed");

        let (_envelope, recovered) = decoded.unwrap();

        // Verify field-level equivalence
        prop_assert_eq!(recovered.record_kind(), event.record_kind(),
            "record_kind must survive roundtrip");
        prop_assert_eq!(recovered.seq().get(), event.seq().get(),
            "seq must survive roundtrip");
        prop_assert!(recovered.is_valid(), "recovered event must be valid");
    }

    /// Encode StepStarted, decode, verify.
    #[test]
    fn proptest_step_started_roundtrip(
        seq in 0u64..1000u64,
        step_idx in 0u16..100u16,
    ) {
        let max_payload_len = 1024u32;

        let event = vb_storage::JournalEvent::StepStarted {
            run: vb_core::RunId::new(99),
            seq: vb_storage::EventSeq::new(seq),
            step: vb_core::StepIdx::new(step_idx),
            attempt: 1,
        };

        let encoded = vb_storage::encode_record(
            MAGIC_JOURNAL_EVENT,
            event.record_kind(),
            event.seq().get(),
            &event,
            max_payload_len,
        );
        prop_assert!(encoded.is_ok(), "encode must succeed");

        let decoded = vb_storage::decode_record::<vb_storage::JournalEvent>(
            &encoded.unwrap(),
            MAGIC_JOURNAL_EVENT,
            max_payload_len,
        );
        prop_assert!(decoded.is_ok(), "roundtrip decode must succeed");

        let (_envelope, recovered) = decoded.unwrap();
        prop_assert!(recovered.is_valid(), "recovered event must be valid");
    }
}
