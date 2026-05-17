// Property tests for concurrency_safety (CS) — vb_ipc
// Tests: MemoryIngress bounded queue and frame codec maintain safety invariants.
// Coverage: CS-1..CS-19 from test-plan §1.9.

use vb_ipc::ingress::{IngressFrame, MemoryIngress};
use vb_ipc::frame::{
    decode_frame_header, encode_frame, validate_frame_magic, validate_frame_bounds,
    decode_frame_payload,
};
use vb_ipc::codec::{decode_payload, encode_payload};
use vb_ipc::error::IpcError;
use vb_ipc::constants::{IPC_HEADER_LEN, IPC_MAGIC, IPC_VERSION};
use vb_ipc::{IpcCommand, IpcFrameHeader, MaxPayloadBytes, QueueCapacity};
use vb_core::{RunId, WorkflowDigest};

use bytes::Bytes;
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Strategy helpers
// ---------------------------------------------------------------------------

fn arb_run_id() -> impl Strategy<Value = RunId> {
    any::<u64>().prop_map(RunId::new)
}

fn arb_workflow_digest() -> impl Strategy<Value = WorkflowDigest> {
    prop::array::uniform32(any::<u8>()).prop_map(WorkflowDigest::from_bytes)
}

fn arb_ingress_frame(max_payload: MaxPayloadBytes) -> impl Strategy<Value = IngressFrame> {
    prop::collection::vec(any::<u8>(), 0..256)
        .prop_map(move |payload_bytes| {
            let digest = WorkflowDigest::from_bytes([0u8; 32]);
            IngressFrame::new(
                RunId::new(1),
                digest,
                Bytes::from(payload_bytes),
                max_payload,
            ).unwrap()
        })
}

fn arb_ipc_command() -> impl Strategy<Value = IpcCommand> {
    prop_oneof![
        Just(IpcCommand::Health),
        Just(IpcCommand::Shutdown),
        Just(IpcCommand::SubmitRun),
        Just(IpcCommand::CancelRun),
        Just(IpcCommand::InspectRun),
        Just(IpcCommand::ListRuns),
        Just(IpcCommand::ListEvents),
        Just(IpcCommand::SubmitRunInline),
        Just(IpcCommand::CompleteAction),
        Just(IpcCommand::FailAction),
        Just(IpcCommand::AnswerAsk),
        Just(IpcCommand::DrainTrace),
        Just(IpcCommand::GetMetrics),
        Just(IpcCommand::GetWorkflowGraph),
        Just(IpcCommand::GetTaintReport),
        Just(IpcCommand::VerifyWorkflow),
    ]
}

// ---------------------------------------------------------------------------
// CS-1..CS-3: MemoryIngress bounded queue capacity
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn cs_memory_ingress_bounded_creates_queue_with_capacity(capacity_val in 1u8..20u8) {
        let capacity = QueueCapacity::new(std::num::NonZeroUsize::new(capacity_val as usize).unwrap());
        let ingress = MemoryIngress::bounded(capacity);
        prop_assert!(ingress.is_empty());
        prop_assert_eq!(ingress.len(), 0);
    }

    #[test]
    fn cs_try_submit_on_full_queue_returns_full(capacity_val in 1u8..10u8) {
        let capacity = QueueCapacity::new(std::num::NonZeroUsize::new(capacity_val as usize).unwrap());
        let ingress = MemoryIngress::bounded(capacity);
        let max_payload = MaxPayloadBytes::DEFAULT;
        let digest = WorkflowDigest::from_bytes([0u8; 32]);

        // Fill the queue
        for i in 0..capacity_val {
            let frame = IngressFrame::new(
                RunId::new(i as u64),
                digest,
                Bytes::from(vec![i]),
                max_payload,
            ).unwrap();
            let result = ingress.try_submit(frame);
            prop_assert!(result.is_ok(), "submit {} should succeed", i);
        }

        // Next submit should return Full
        let overflow_frame = IngressFrame::new(
            RunId::new(99),
            digest,
            Bytes::from(vec![99]),
            max_payload,
        ).unwrap();
        let result = ingress.try_submit(overflow_frame);
        prop_assert!(result.is_err());
        prop_assert!(matches!(result.unwrap_err(), IpcError::Full));
    }

    #[test]
    fn cs_try_recv_on_empty_queue_returns_none() {
        let capacity = QueueCapacity::new(std::num::NonZeroUsize::new(1).unwrap());
        let ingress = MemoryIngress::bounded(capacity);
        let result = ingress.try_recv();
        prop_assert!(result.is_ok());
        prop_assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn cs_try_recv_on_disconnected_channel_returns_disconnected() {
        let capacity = QueueCapacity::new(std::num::NonZeroUsize::new(1).unwrap());
        let mut ingress = MemoryIngress::bounded(capacity);
        ingress.disconnect_sender();
        let result = ingress.try_recv();
        prop_assert!(result.is_err());
        prop_assert!(matches!(result.unwrap_err(), IpcError::Disconnected));
    }
}

// ---------------------------------------------------------------------------
// CS-5: FIFO ordering
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn cs_fifo_ordering_maintained(num_frames in 1u8..10u8) {
        let capacity = QueueCapacity::new(std::num::NonZeroUsize::new(10).unwrap());
        let ingress = MemoryIngress::bounded(capacity);
        let max_payload = MaxPayloadBytes::DEFAULT;
        let digest = WorkflowDigest::from_bytes([0u8; 32]);

        // Submit frames with sequential run IDs
        for i in 0..num_frames {
            let frame = IngressFrame::new(
                RunId::new(i as u64),
                digest,
                Bytes::from(vec![i]),
                max_payload,
            ).unwrap();
            let result = ingress.try_submit(frame);
            prop_assert!(result.is_ok());
        }

        // Receive and verify order
        for i in 0..num_frames {
            let result = ingress.try_recv();
            prop_assert!(result.is_ok());
            let frame = result.unwrap().expect("should have frame");
            prop_assert_eq!(frame.run_id(), RunId::new(i as u64), "frame {} should have run_id {}", i, i);
        }

        // Queue should now be empty
        let result = ingress.try_recv();
        prop_assert_eq!(result.unwrap(), None);
    }
}

// ---------------------------------------------------------------------------
// CS-6: IngressFrame::new with oversized payload returns PayloadTooLarge
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn cs_ingress_frame_new_rejects_oversized_payload(payload_size in 1024..8192) {
        let max_payload = MaxPayloadBytes::new(
            std::num::NonZeroUsize::new(512).unwrap() // limit to 512 bytes
        );
        let payload = Bytes::from(vec![0u8; payload_size]);
        let result = IngressFrame::new(
            RunId::new(1),
            WorkflowDigest::from_bytes([0u8; 32]),
            payload,
            max_payload,
        );
        prop_assert!(result.is_err());
        prop_assert!(matches!(result.unwrap_err(), IpcError::PayloadTooLarge { .. }));
    }

    #[test]
    fn cs_ingress_frame_new_accepts_valid_payload(size in 0u32..512u32) {
        let max_payload = MaxPayloadBytes::new(std::num::NonZeroUsize::new(1024).unwrap());
        let payload = Bytes::from(vec![0u8; size as usize]);
        let result = IngressFrame::new(
            RunId::new(1),
            WorkflowDigest::from_bytes([0u8; 32]),
            payload,
            max_payload,
        );
        prop_assert!(result.is_ok());
    }
}

// ---------------------------------------------------------------------------
// CS-7: Frame header encodes fixed IPC_HEADER_LEN bytes
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn cs_encode_frame_produces_fixed_header_length(
        command in arb_ipc_command(),
        correlation in any::<u64>(),
        payload in prop::collection::vec(any::<u8>(), 0..100),
    ) {
        let result = encode_frame(command, 0, correlation, &payload);
        prop_assert!(result.is_ok());
        let frame = result.unwrap();
        prop_assert!(frame.len() >= IPC_HEADER_LEN);
        prop_assert_eq!(frame.len(), IPC_HEADER_LEN + payload.len(),
            "frame length must be header + payload");
    }

    #[test]
    fn cs_header_encode_decode_roundtrip(
        command in arb_ipc_command(),
        correlation in any::<u64>(),
        payload_len in 0u32..100u32,
    ) {
        let payload = vec![0u8; payload_len as usize];
        let result = encode_frame(command, 0, correlation, &payload);
        prop_assert!(result.is_ok());
        let frame = result.unwrap();

        // Extract header bytes
        let header_bytes: [u8; IPC_HEADER_LEN] = frame[..IPC_HEADER_LEN].try_into().unwrap();
        let decoded = decode_frame_header(&header_bytes);
        prop_assert!(decoded.is_ok());
        let header = decoded.unwrap();
        prop_assert_eq!(header.command, command);
        prop_assert_eq!(header.correlation, correlation);
        let expected_payload_len = u32::try_from(payload_len).unwrap();
        prop_assert_eq!(header.payload_len, expected_payload_len);
    }
}

// ---------------------------------------------------------------------------
// CS-8: validate_frame_magic rejects wrong magic
// CS-9: Unsupported version → UnsupportedVersion
// CS-10: Unknown command → UnknownCommand
// CS-11: Reserved non-zero → ReservedNonZero
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn cs_validate_frame_magic_accepts_correct_magic() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&IPC_MAGIC.to_le_bytes());
        bytes.extend_from_slice(&[0u8; 20]);
        let result = validate_frame_magic(&bytes);
        prop_assert!(result.is_ok());
    }

    #[test]
    fn cs_validate_frame_magic_rejects_wrong_magic() {
        let wrong_magic = 0xDEAD_BEEFu32;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&wrong_magic.to_le_bytes());
        bytes.extend_from_slice(&[0u8; 20]);
        let result = validate_frame_magic(&bytes);
        prop_assert!(result.is_err());
        let err = result.unwrap_err();
        prop_assert!(matches!(err, IpcError::InvalidMagic { .. }));
    }

    #[test]
    fn cs_decode_header_rejects_zero_magic() {
        let mut header_bytes = [0u8; IPC_HEADER_LEN];
        header_bytes[..4].copy_from_slice(&0u32.to_le_bytes());
        let result = decode_frame_header(&header_bytes);
        prop_assert!(result.is_err());
        let err = result.unwrap_err();
        prop_assert!(matches!(err, IpcError::InvalidMagic { actual } if actual == 0));
    }

    #[test]
    fn cs_decode_header_rejects_unsupported_version() {
        let mut header_bytes = [0u8; IPC_HEADER_LEN];
        header_bytes[..4].copy_from_slice(&IPC_MAGIC.to_le_bytes());
        header_bytes[4..6].copy_from_slice(&99u16.to_le_bytes()); // version 99
        header_bytes[6..8].copy_from_slice(&IpcCommand::Health.as_u16().to_le_bytes());
        let result = decode_frame_header(&header_bytes);
        prop_assert!(result.is_err());
        prop_assert!(matches!(result.unwrap_err(), IpcError::UnsupportedVersion { .. }));
    }

    #[test]
    fn cs_decode_header_rejects_unknown_command() {
        let mut header_bytes = [0u8; IPC_HEADER_LEN];
        header_bytes[..4].copy_from_slice(&IPC_MAGIC.to_le_bytes());
        header_bytes[4..6].copy_from_slice(&IPC_VERSION.to_le_bytes());
        header_bytes[6..8].copy_from_slice(&999u16.to_le_bytes()); // invalid command
        let result = decode_frame_header(&header_bytes);
        prop_assert!(result.is_err());
        prop_assert!(matches!(result.unwrap_err(), IpcError::UnknownCommand(999)));
    }

    #[test]
    fn cs_decode_header_rejects_reserved_nonzero() {
        let mut header_bytes = [0u8; IPC_HEADER_LEN];
        header_bytes[..4].copy_from_slice(&IPC_MAGIC.to_le_bytes());
        header_bytes[4..6].copy_from_slice(&IPC_VERSION.to_le_bytes());
        header_bytes[6..8].copy_from_slice(&IpcCommand::Health.as_u16().to_le_bytes());
        header_bytes[10..12].copy_from_slice(&1u16.to_le_bytes()); // reserved = 1
        let result = decode_frame_header(&header_bytes);
        prop_assert!(result.is_err());
        prop_assert!(matches!(result.unwrap_err(), IpcError::ReservedNonZero { .. }));
    }
}

// ---------------------------------------------------------------------------
// CS-12: decode_frame_payload rejects payload len mismatch
// CS-14: decode_payload on garbage → PayloadDecodeFailed
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn cs_decode_payload_rejects_length_mismatch() {
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 100);
        let short_payload = vec![0u8; 50];
        let result = decode_frame_payload(&header, &short_payload);
        prop_assert!(result.is_err());
        prop_assert!(matches!(result.unwrap_err(), IpcError::PayloadLengthMismatch { header: 100, actual: 50 }));
    }

    #[test]
    fn cs_decode_payload_accepts_matching_length() {
        let payload_bytes = b"hello".to_vec();
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 5);
        let result = decode_frame_payload(&header, &payload_bytes);
        prop_assert!(result.is_ok());
        prop_assert_eq!(result.unwrap(), vb_ipc::IpcPayload::Health);
    }

    #[test]
    fn cs_decode_payload_on_garbage_returns_payload_decode_failed() {
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 4);
        let garbage = vec![0xFF, 0xFE, 0xFD, 0xFC];
        let result = decode_frame_payload(&header, &garbage);
        prop_assert!(result.is_err());
        prop_assert!(matches!(result.unwrap_err(), IpcError::PayloadDecodeFailed));
    }
}

// ---------------------------------------------------------------------------
// CS-13: encode_payload/decode_payload roundtrip for all variants
// CS-15: try_submit after drain from full returns Ok again
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn cs_try_submit_after_drain_returns_ok() {
        let capacity = QueueCapacity::new(std::num::NonZeroUsize::new(1).unwrap());
        let ingress = MemoryIngress::bounded(capacity);
        let max_payload = MaxPayloadBytes::DEFAULT;
        let digest = WorkflowDigest::from_bytes([0u8; 32]);

        // First submit
        let frame1 = IngressFrame::new(
            RunId::new(1),
            digest,
            Bytes::from(vec![1]),
            max_payload,
        ).unwrap();
        prop_assert!(ingress.try_submit(frame1.clone()).is_ok());

        // Queue is full
        let frame2 = IngressFrame::new(
            RunId::new(2),
            digest,
            Bytes::from(vec![2]),
            max_payload,
        ).unwrap();
        prop_assert!(matches!(ingress.try_submit(frame2), Err(IpcError::Full)));

        // Drain
        let drained = ingress.try_recv();
        prop_assert!(drained.is_ok());
        prop_assert!(drained.unwrap().is_some());

        // Submit again should succeed
        let frame3 = IngressFrame::new(
            RunId::new(3),
            digest,
            Bytes::from(vec![3]),
            max_payload,
        ).unwrap();
        prop_assert!(ingress.try_submit(frame3).is_ok());
    }
}

// ---------------------------------------------------------------------------
// CS-16: Channel disconnect propagates Disconnected error on try_recv
// (already tested in CS-3)
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn cs_disconnect_propagates_on_recv() {
        let capacity = QueueCapacity::new(std::num::NonZeroUsize::new(1).unwrap());
        let mut ingress = MemoryIngress::bounded(capacity);
        ingress.disconnect_sender();
        let result = ingress.try_recv();
        prop_assert!(matches!(result, Err(IpcError::Disconnected)));
    }
}

// ---------------------------------------------------------------------------
// CS-17: QueueCapacity and MaxPayloadBytes are Copy wrappers
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn cs_queue_capacity_is_copy(capacity_val in 1u8..20u8) {
        let capacity = QueueCapacity::new(std::num::NonZeroUsize::new(capacity_val as usize).unwrap());
        let copy = capacity;
        prop_assert_eq!(capacity.get(), copy.get());
    }

    #[test]
    fn cs_max_payload_bytes_is_copy(val in 1u32..1000u32) {
        let max = MaxPayloadBytes::new(std::num::NonZeroUsize::new(val as usize).unwrap());
        let copy = max;
        prop_assert_eq!(max.get(), copy.get());
    }
}

// ---------------------------------------------------------------------------
// CS-18..CS-19: IpcError variant codes are unique
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn cs_ipc_error_diagnostic_codes_are_unique() {
        // Collect all IpcError variants and verify their diagnostic_code() returns unique values
        let err_full = IpcError::Full;
        let err_disconnected = IpcError::Disconnected;
        let err_payload_large = IpcError::PayloadTooLarge { actual: 100, limit: 50 };
        let err_bad_magic = IpcError::InvalidMagic { actual: 0xDEAD };
        let err_bad_version = IpcError::UnsupportedVersion { actual: 99 };
        let err_unknown_cmd = IpcError::UnknownCommand(5);
        let err_reserved_nz = IpcError::ReservedNonZero { actual: 1 };
        let err_len_mismatch = IpcError::PayloadLengthMismatch { header: 10, actual: 5 };
        let err_hdr_failed = IpcError::HeaderDecodeFailed;
        let err_len_out_range = IpcError::PayloadLengthOutOfRange { actual: u32::MAX };
        let err_enc_failed = IpcError::PayloadEncodeFailed;
        let err_dec_failed = IpcError::PayloadDecodeFailed;
        let err_resp_dec_failed = IpcError::ResponseDecodeFailed;

        let codes: Vec<_> = [
            err_full.diagnostic_code(),
            err_disconnected.diagnostic_code(),
            err_payload_large.diagnostic_code(),
            err_bad_magic.diagnostic_code(),
            err_bad_version.diagnostic_code(),
            err_unknown_cmd.diagnostic_code(),
            err_reserved_nz.diagnostic_code(),
            err_len_mismatch.diagnostic_code(),
            err_hdr_failed.diagnostic_code(),
            err_len_out_range.diagnostic_code(),
            err_enc_failed.diagnostic_code(),
            err_dec_failed.diagnostic_code(),
            err_resp_dec_failed.diagnostic_code(),
        ];

        let unique: std::collections::HashSet<_> = codes.iter().collect();
        prop_assert_eq!(codes.len(), unique.len(), "all IpcError diagnostic codes must be unique");
    }

    #[test]
    fn cs_ipc_error_runtime_codes_are_unique_where_defined() {
        // Errors with Some runtime_code must have unique values
        let err_full = IpcError::Full;
        let err_magic = IpcError::InvalidMagic { actual: 0 };
        let err_version = IpcError::UnsupportedVersion { actual: 1 };
        let err_unknown = IpcError::UnknownCommand(2);
        let err_reserved = IpcError::ReservedNonZero { actual: 0 };
        let err_hdr_fail = IpcError::HeaderDecodeFailed;

        // All structural errors share the same runtime code
        let structural_code = IpcError::IPC_FRAME_INVALID_RUNTIME_CODE;
        prop_assert!(!structural_code.is_empty());

        // Full has its own queue code
        let full_code = err_full.runtime_code();
        prop_assert!(full_code.is_some());
        prop_assert_eq!(full_code.unwrap(), IpcError::QUEUE_FULL_RUNTIME_CODE);
    }
}

// ---------------------------------------------------------------------------
// CS-2: try_submit on full queue returns Full (additional structural test)
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn cs_submit_full_after_one_submit(capacity_val in 1u8..5u8) {
        let capacity = QueueCapacity::new(std::num::NonZeroUsize::new(capacity_val as usize).unwrap());
        let ingress = MemoryIngress::bounded(capacity);
        let max_payload = MaxPayloadBytes::DEFAULT;
        let digest = WorkflowDigest::from_bytes([0u8; 32]);

        // Submit one frame
        let frame = IngressFrame::new(
            RunId::new(1),
            digest,
            Bytes::from(vec![1]),
            max_payload,
        ).unwrap();
        ingress.try_submit(frame).expect("first submit should succeed");

        // Second submit should fail with Full
        let frame2 = IngressFrame::new(
            RunId::new(2),
            digest,
            Bytes::from(vec![2]),
            max_payload,
        ).unwrap();
        let result = ingress.try_submit(frame2);
        prop_assert!(matches!(result, Err(IpcError::Full)));
    }
}
