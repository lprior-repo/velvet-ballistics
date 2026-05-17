# Contract: vb-wgew - IPC Fuzz Must Call Real Decoder

## Requirement

The fuzz target `fuzz_ipc_frame` at `fuzz/src/lib.rs:137-174` currently performs static byte inspection only. It must call the actual decoder functions (`IpcFrameHeader::decode()`, `decode_frame_header()`, etc.) to ensure the decoder is robust against adversarial input.

## Non-Goals

- Not implementing new decoder logic (decoder already exists)
- Not adding new IPC error variants

## Constraints

1. Fuzz target must compile under `#[cfg(feature = "fuzz")]`
2. Must use `libfuzzer_sys::fuzz_target` macro
3. Decoder errors must be mapped to `IpcError` variants correctly

## Assumptions

1. `IpcFrameHeader::decode()` exists and takes `&[u8; IPC_HEADER_LEN]`
2. `decode_frame_payload()` exists for payload decoding
3. `IpcError` enum has required variants

## Verification Criteria

| ID | Criterion | File | Command |
|----|-----------|------|---------|
| IPC-FUZZ-001 | Fuzz target calls `IpcFrameHeader::decode()` | `fuzz/src/lib.rs` | `cargo fuzz build` |
| IPC-FUZZ-002 | Fuzz target calls `decode_frame_payload()` | `fuzz/src/lib.rs` | `cargo fuzz build` |
| IPC-FUZZ-003 | Decoder panic-free for adversarial bytes | `fuzz/src/lib.rs` | `cargo fuzz run ipc_frame -- -runs=10000` |
| IPC-FUZZ-004 | Typed errors returned for malformed frames | `fuzz/src/lib.rs` | `cargo fuzz run ipc_frame -- -runs=10000` |

## Statement

The current `fuzz_ipc_frame` implementation only performs manual byte inspection:

```rust
// Current (WRONG) - static inspection only
fn fuzz_ipc_frame(data: &[u8]) {
    if data.len() < 24 { return; }
    let magic = read_u32_le(&data[0..4]);
    if magic != IPC_MAGIC { return; }
    // ... more manual byte reading
}
```

It must call the real decoder:

```rust
// Required (CORRECT) - calls actual decoder
fn fuzz_ipc_frame(data: &[u8]) {
    let mut header_bytes = [0u8; 24];
    if data.len() < 24 { return; }
    header_bytes.copy_from_slice(&data[..24]);
    if let Ok(header) = IpcFrameHeader::decode(&header_bytes) {
        // Call decode_frame_payload if payload_len > 0
    }
}
```