//! Fuzz target for IPC frame decode.
//!
//! ## INVARIANT Oracle
//!
//! Replaces crash-only fuzzing with structural assertions on
//! `decode_frame_header` and `decode_frame_payload`:
//! - Decoded `IpcFrameHeader` is guaranteed to have matched `IPC_VERSION` on
//!   decode (the version slot is validated during decode and not stored on the
//!   struct). The wire-format version slot of any re-encoded header MUST equal
//!   `IPC_VERSION`.
//! - On Ok: re-encoding the decoded header round-trips to the exact input
//!   bytes (`encode ∘ decode = id`).
//! - `decode_frame_payload` MUST return `Err` whenever `payload.len()` differs
//!   from the header's `payload_len` field.
//! - All errors are typed `IpcError` variants (enforced inside
//!   `fuzz_lib::fuzz_ipc_frame`).
//!
//! Every `decode_frame_header` call is matched into `Ok / Err` so that no
//! decoder `Result` is silently discarded and no wildcard `IpcPayload` arm is
//! reached. Decode and encode failures are routed through the typed
//! `fuzz_lib::assert_typed_ipc_error` oracle.
//!
//! Corpus seeds are maintained in `fuzz/corpus/ipc_frame/`.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Target-level oracle: a successfully decoded header re-encodes to bytes
    // whose version slot equals IPC_VERSION. This is the only externally
    // observable form of the "decoded header has version == IPC_VERSION"
    // invariant, since `IpcFrameHeader` does not store a version field.
    //
    // Both decoder calls are matched into `Ok / Err` so the typed oracle in
    // `fuzz_lib` exercises every error class. The strict lint gate forbids
    // `let _ = decode(...)` discard patterns and any wildcard `IpcPayload`
    // arm, so we route both branches through the oracle.
    if let Some(header_bytes) = data.first_chunk::<{ vb_ipc::IPC_HEADER_LEN }>() {
        match vb_ipc::frame::decode_frame_header(header_bytes) {
            Ok(decoded) => match decoded.encode() {
                Ok(encoded) => {
                    let version_bytes: [u8; 2] = encoded
                        .as_slice()
                        .get(4..6)
                        .and_then(|s| <[u8; 2]>::try_from(s).ok())
                        .unwrap_or([0u8; 2]);
                    let version = u16::from_le_bytes(version_bytes);
                    assert_eq!(
                        version,
                        vb_ipc::IPC_VERSION,
                        "decoded IpcFrameHeader re-encodes with version != IPC_VERSION"
                    );
                    assert_eq!(
                        encoded.as_slice(),
                        header_bytes.as_slice(),
                        "re-encoded header must match original bytes"
                    );
                }
                Err(encode_error) => {
                    fuzz_lib::assert_typed_ipc_error(encode_error);
                }
            },
            Err(decode_error) => {
                fuzz_lib::assert_typed_ipc_error(decode_error);
            }
        }
    }

    fuzz_lib::fuzz_ipc_frame(data);
});
