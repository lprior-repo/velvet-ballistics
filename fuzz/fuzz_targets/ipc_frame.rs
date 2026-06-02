//! Fuzz target for IPC frame decode — full harness with bounded read paths.
//!
//! This target exercises three decode paths through `fuzz_lib::fuzz_ipc_frame`:
//!
//! 1. **Slice-based header decode**: `decode_frame_header` on the first 24 bytes,
//!    with round-trip re-encode verification.
//! 2. **Slice-based payload decode**: `decode_frame_payload` on remaining bytes,
//!    exercising postcard deserialization of every `IpcPayload` variant.
//! 3. **Bounded Cursor-based read**: `read_frame_header_bounded` +
//!    `read_frame_payload_bounded` with bounds `[1, 16, 256, 1024, 65536, 1048576]`.
//!    This path exercises the preallocation gate — oversized payloads must be
//!    rejected with typed `IpcError::PayloadTooLarge` before any allocation occurs.
//!
//! All error paths are validated through `assert_typed_ipc_error` to ensure
//! only known typed `IpcError` variants are returned (never panics).
//!
//! Corpus seeds are maintained in `fuzz/corpus/ipc_frame/`. The full hostile
//! corpus (11 seeds) is documented in `.beads/vb-jpq7.36/corpus-seeds-plan.md`.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    fuzz_lib::fuzz_ipc_frame(data);
});
