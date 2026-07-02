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
//! Corpus seeds are maintained in `fuzz/corpus/ipc_frame/`.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Target-level oracle: a successfully decoded header re-encodes to bytes
    // whose version slot equals IPC_VERSION. This is the only externally
    // observable form of the "decoded header has version == IPC_VERSION"
    // invariant, since `IpcFrameHeader` does not store a version field.
    if let Some(header_bytes) = data.get(..vb_ipc::IPC_HEADER_LEN) {
        let mut header = [0u8; vb_ipc::IPC_HEADER_LEN];
        header.copy_from_slice(header_bytes);
        if let Ok(decoded) = vb_ipc::frame::decode_frame_header(&header) {
            if let Ok(encoded) = decoded.encode() {
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
                    &encoded[..],
                    header_bytes,
                    "re-encoded header must match original bytes"
                );
            }
        }
    }

    fuzz_lib::fuzz_ipc_frame(data);
});
