//! IPC frame fuzzing targets.
#![allow(clippy::indexing_slicing)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::items_after_statements)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::let_underscore_must_use)]
#![allow(clippy::as_conversions)]
#![allow(clippy::arithmetic_side_effects)]
#![allow(clippy::len_zero)]

#[must_use]
pub fn assert_typed_ipc_error(error: vb_ipc::IpcError) -> bool {
    use vb_ipc::IpcError;
    matches!(
        error,
        IpcError::Full
            | IpcError::Disconnected
            | IpcError::PayloadTooLarge { .. }
            | IpcError::InvalidMagic { .. }
            | IpcError::UnsupportedVersion { .. }
            | IpcError::UnknownCommand(_)
            | IpcError::ReservedNonZero { .. }
            | IpcError::PayloadLengthMismatch { .. }
            | IpcError::HeaderEncodeFailed
            | IpcError::HeaderDecodeFailed
            | IpcError::PayloadLengthOutOfRange { .. }
            | IpcError::PayloadEncodeFailed
            | IpcError::PayloadDecodeFailed
            | IpcError::ResponseDecodeFailed
    )
}

pub fn fuzz_ipc_frame(data: &[u8]) {
    use vb_ipc::frame::{decode_frame_header, decode_frame_payload};

    let Some(header_bytes) = data.get(..vb_ipc::IPC_HEADER_LEN) else {
        return;
    };

    let mut header = [0u8; vb_ipc::IPC_HEADER_LEN];
    header.copy_from_slice(header_bytes);

    let header_result = decode_frame_header(&header);

    if let Ok(decoded_header) = header_result
        && let Ok(encoded) = decoded_header.encode()
    {
        assert_eq!(
            &encoded[..],
            header_bytes,
            "re-encoded header must match original bytes"
        );
    }

    let Some(payload) = data.get(vb_ipc::IPC_HEADER_LEN..) else {
        return;
    };

    if !payload.is_empty()
        && let Ok(header) = header_result
    {
        let payload_len_usize = header.payload_len as usize;
        let result = decode_frame_payload(&header, payload);
        match result {
            Ok(decoded) => {
                assert!(
                    payload.len() == payload_len_usize,
                    "decode must fail when payload len mismatches header"
                );
                match decoded {
                    vb_ipc::IpcPayload::SubmitRun(p) | vb_ipc::IpcPayload::SubmitRunInline(p) => {
                        let _ = p.run_id;
                        let _ = p.workflow;
                    }
                    vb_ipc::IpcPayload::CancelRun { run_id }
                    | vb_ipc::IpcPayload::InspectRun { run_id }
                    | vb_ipc::IpcPayload::ListEvents { run_id, .. }
                    | vb_ipc::IpcPayload::DrainTrace { run_id, .. } => {
                        let _ = run_id;
                    }
                    vb_ipc::IpcPayload::AnswerAsk { run_id, ticket, .. } => {
                        let _ = run_id;
                        let _ = ticket;
                    }
                    vb_ipc::IpcPayload::CompleteAction { run_id, ticket, .. }
                    | vb_ipc::IpcPayload::FailAction { run_id, ticket, .. } => {
                        let _ = run_id;
                        let _ = ticket;
                    }
                    vb_ipc::IpcPayload::Health | vb_ipc::IpcPayload::Shutdown => {}
                    _ => {}
                }
            }
            Err(e) => assert!(assert_typed_ipc_error(e)),
        }
    }
}

pub fn fuzz_ipc_decode(data: &[u8]) {
    use vb_ipc::frame::decode_frame_header;

    if data.len() >= vb_ipc::IPC_HEADER_LEN {
        let mut header_bytes = [0u8; vb_ipc::IPC_HEADER_LEN];
        header_bytes.copy_from_slice(&data[..vb_ipc::IPC_HEADER_LEN]);

        let bounds: &[usize] = &[0, 1, 16, 256, 1024, 65536, 1_048_576];
        for &b in bounds {
            if let Some(max) = std::num::NonZeroUsize::new(b) {
                let _ = vb_ipc::IpcFrameHeader::decode(
                    &header_bytes,
                    vb_ipc::MaxPayloadBytes::new(max),
                );
            }
        }

        let _ = decode_frame_header(&header_bytes);
    }

    for len in 0..vb_ipc::IPC_HEADER_LEN {
        if data.len() >= len {
            let mut bytes = [0u8; vb_ipc::IPC_HEADER_LEN];
            let end = len.min(data.len());
            bytes[..end].copy_from_slice(&data[..end]);
            let _ = vb_ipc::IpcFrameHeader::decode(&bytes, vb_ipc::MaxPayloadBytes::DEFAULT);
        }
    }
}

pub fn fuzz_ipc_frame_boundary(data: &[u8]) {
    use vb_ipc::frame::{decode_frame_header, validate_frame_magic};
    use vb_ipc::{IPC_HEADER_LEN, IpcError, MaxPayloadBytes};

    if data.is_empty() {
        return;
    }

    let magic_result = validate_frame_magic(data);
    if data.len() < 4 {
        assert!(
            matches!(magic_result, Err(IpcError::HeaderDecodeFailed)),
            "truncated frame (< 4 bytes) must return HeaderDecodeFailed"
        );
        return;
    }

    if magic_result.is_err() {
        assert!(
            matches!(
                magic_result,
                Err(IpcError::InvalidMagic { .. }) | Err(IpcError::HeaderDecodeFailed)
            ),
            "wrong magic must return InvalidMagic or HeaderDecodeFailed"
        );
        return;
    }

    if data.len() < IPC_HEADER_LEN {
        return;
    }

    let mut header_bytes = [0u8; IPC_HEADER_LEN];
    header_bytes.copy_from_slice(&data[..IPC_HEADER_LEN]);

    let Some(max_payload_nz) = std::num::NonZeroUsize::new(65536) else {
        return;
    };
    let max_payload = MaxPayloadBytes::new(max_payload_nz);
    let header_result = vb_ipc::IpcFrameHeader::decode(&header_bytes, max_payload);

    match header_result {
        Ok(header) => {
            let payload = data.get(IPC_HEADER_LEN..).unwrap_or(&[]);
            let Ok(expected_len) = usize::try_from(header.payload_len) else {
                return;
            };
            if payload.len() != expected_len && !payload.is_empty() {}
        }
        Err(e) => {
            assert!(assert_typed_ipc_error(e));
        }
    }

    if data.len() >= IPC_HEADER_LEN {
        let _ = decode_frame_header(&header_bytes);
    }
}

#[cfg(test)]
mod typed_ipc_error_tests {
    use super::*;

    #[test]
    fn test_all_14_explicit_variants_match() {
        let variants = [
            vb_ipc::IpcError::Full,
            vb_ipc::IpcError::Disconnected,
            vb_ipc::IpcError::PayloadTooLarge {
                actual: 1,
                limit: 0,
            },
            vb_ipc::IpcError::InvalidMagic { actual: 0 },
            vb_ipc::IpcError::UnsupportedVersion { actual: 0 },
            vb_ipc::IpcError::UnknownCommand(0),
            vb_ipc::IpcError::ReservedNonZero { actual: 1 },
            vb_ipc::IpcError::PayloadLengthMismatch {
                header: 1,
                actual: 0,
            },
            vb_ipc::IpcError::HeaderEncodeFailed,
            vb_ipc::IpcError::HeaderDecodeFailed,
            vb_ipc::IpcError::PayloadLengthOutOfRange { actual: 0 },
            vb_ipc::IpcError::PayloadEncodeFailed,
            vb_ipc::IpcError::PayloadDecodeFailed,
            vb_ipc::IpcError::ResponseDecodeFailed,
        ];
        for variant in variants {
            assert!(assert_typed_ipc_error(variant));
        }
    }

    #[test]
    fn test_malformed_header_decode() -> Result<(), Box<dyn std::error::Error>> {
        let header = [0u8; vb_ipc::IPC_HEADER_LEN];
        let error = vb_ipc::frame::decode_frame_header(&header)
            .err()
            .ok_or("expected malformed header rejection")?;
        assert_eq!(error, vb_ipc::IpcError::InvalidMagic { actual: 0 });
        assert!(assert_typed_ipc_error(error));
        Ok(())
    }
}
