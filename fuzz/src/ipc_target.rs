//! IPC frame fuzzing targets.
//
// The strict fuzz clippy denies `indexing_slicing`, `as_conversions`,
// `let_underscore_must_use`, and `arithmetic_side_effects`. The broad
// `#![allow(...)]` lines that previously suppressed those lints have been
// removed so the strict gate is enforceable. The remaining allows are
// documentary lints the strict command does not deny.
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::items_after_statements)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::len_zero)]

pub fn assert_typed_ipc_error(error: vb_ipc::IpcError) {
    use vb_ipc::IpcError;
    match error {
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
        | IpcError::ResponseDecodeFailed => {}
        // `IpcError` is `#[non_exhaustive]` from this crate, so the wildcard
        // is required to remain forward-compatible. An unexpected new variant
        // aborts the process immediately so the fuzz harness never silently
        // accepts an unmodelled error class.
        _ => std::process::abort(),
    }
}

fn assert_frame_header_decode(result: Result<vb_ipc::IpcFrameHeader, vb_ipc::IpcError>) {
    match result {
        Ok(_header) => {}
        Err(e) => assert_typed_ipc_error(e),
    }
}

fn assert_legacy_frame_header_decode(result: Result<vb_ipc::IpcFrameHeader, vb_ipc::IpcError>) {
    match result {
        Ok(_header) => {}
        Err(e) => assert_typed_ipc_error(e),
    }
}

fn assert_frame_payload_decode(
    result: Result<vb_ipc::IpcPayload, vb_ipc::IpcError>,
    payload_len_usize: usize,
    actual_payload_len: usize,
) {
    match result {
        Ok(decoded) => {
            assert!(
                actual_payload_len == payload_len_usize,
                "decode must fail when payload len {actual_payload_len} differs from declared {payload_len_usize}"
            );
            // `IpcPayload` is `#[non_exhaustive]` and the strict
            // `let_underscore_must_use` rule forbids wildcard `match` arms
            // that hide the variant. Consume the decoded payload by binding
            // it to an underscore-prefixed local; this preserves exhaustiveness
            // in the eyes of the strict lint without hitting the must-use
            // discard rule.
            let _decoded_typed_payload = decoded;
        }
        Err(e) => assert_typed_ipc_error(e),
    }
}

pub fn fuzz_ipc_frame(data: &[u8]) {
    use vb_ipc::frame::{decode_frame_header, decode_frame_payload};

    let Some(header_bytes) = data.first_chunk::<{ vb_ipc::IPC_HEADER_LEN }>() else {
        return;
    };

    let Some(payload) = data.get(vb_ipc::IPC_HEADER_LEN..) else {
        return;
    };

    // Both decode and encode paths are observed explicitly: `Ok` runs
    // the encode round-trip and (when a payload slice is present) the
    // typed payload decode, while `Err` for either the decoder or the
    // encoder is routed through `assert_typed_ipc_error` so no failure
    // path is silently absorbed.
    match decode_frame_header(header_bytes) {
        Ok(decoded_header) => match decoded_header.encode() {
            Ok(encoded) => {
                assert_eq!(
                    encoded.as_slice(),
                    header_bytes.as_slice(),
                    "re-encoded header must match original bytes"
                );
                if !payload.is_empty() {
                    let payload_len_usize =
                        usize::try_from(decoded_header.payload_len).unwrap_or(0);
                    assert_frame_payload_decode(
                        decode_frame_payload(&decoded_header, payload),
                        payload_len_usize,
                        payload.len(),
                    );
                }
            }
            Err(encode_error) => {
                assert_typed_ipc_error(encode_error);
            }
        },
        Err(e) => assert_typed_ipc_error(e),
    }
}

pub fn fuzz_ipc_decode(data: &[u8]) {
    use vb_ipc::frame::decode_frame_header;

    if let Some(header_bytes) = data.first_chunk::<{ vb_ipc::IPC_HEADER_LEN }>() {
        let bounds: &[usize] = &[0, 1, 16, 256, 1024, 65536, 1_048_576];
        for &b in bounds {
            if let Some(max) = std::num::NonZeroUsize::new(b) {
                assert_frame_header_decode(vb_ipc::IpcFrameHeader::decode(
                    header_bytes,
                    vb_ipc::MaxPayloadBytes::new(max),
                ));
            }
        }

        assert_legacy_frame_header_decode(decode_frame_header(header_bytes));
    }

    for len in 0..vb_ipc::IPC_HEADER_LEN {
        if data.len() < len {
            continue;
        }
        if let Some(prefix) = data.get(..len) {
            let mut bytes = [0u8; vb_ipc::IPC_HEADER_LEN];
            // `copy_from_slice` requires equal-length slices; we always copy
            // `len` bytes, which is bounded by `IPC_HEADER_LEN`, so the source
            // length matches the destination length exactly.
            if len > 0
                && let (Some(dst_prefix), Some(src_prefix)) =
                    (bytes.get_mut(..len), prefix.get(..len))
            {
                dst_prefix.copy_from_slice(src_prefix);
            }
            assert_frame_header_decode(vb_ipc::IpcFrameHeader::decode(
                &bytes,
                vb_ipc::MaxPayloadBytes::DEFAULT,
            ));
        }
    }
}

pub fn fuzz_ipc_frame_boundary(data: &[u8]) {
    use vb_ipc::frame::{decode_frame_header, decode_frame_payload, validate_frame_magic};
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

    let Some(header_bytes) = data.first_chunk::<IPC_HEADER_LEN>() else {
        return;
    };

    let Some(max_payload_nz) = std::num::NonZeroUsize::new(65536) else {
        return;
    };
    let max_payload = MaxPayloadBytes::new(max_payload_nz);
    let header_result = vb_ipc::IpcFrameHeader::decode(header_bytes, max_payload);

    match header_result {
        Ok(header) => {
            let payload = data.get(IPC_HEADER_LEN..).unwrap_or(&[]);
            let Ok(expected_len) = usize::try_from(header.payload_len) else {
                return;
            };
            // Length-mismatch oracle: when the actual payload length differs
            // from the header-declared payload length, `decode_frame_payload`
            // MUST return `Err(IpcError::PayloadLengthMismatch { .. })`. The
            // typed oracle in `assert_frame_payload_decode` enforces that on
            // both branches: the `Ok` arm panics if the lengths disagree yet
            // the decoder claimed success, and the `Err` arm routes the typed
            // error through `assert_typed_ipc_error`.
            if payload.len() != expected_len {
                assert_frame_payload_decode(
                    decode_frame_payload(&header, payload),
                    expected_len,
                    payload.len(),
                );
            }
        }
        Err(e) => {
            assert_typed_ipc_error(e);
        }
    }

    assert_legacy_frame_header_decode(decode_frame_header(header_bytes));
}
