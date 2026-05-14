# Architecture Refactor: vb_ipc/lib.rs Decomposition

## Bead: vb-jul7

## Summary
Decomposed vb_ipc/src/lib.rs from 3085 lines to 45 lines by extracting types and functions into focused submodules.

## Changes Made

### New Module Files Created

| File | Lines | Purpose |
|------|-------|---------|
| `constants.rs` | 8 | IPC_MAGIC, IPC_VERSION, IPC_HEADER_LEN |
| `commands.rs` | 79 | IpcCommand enum with from_u16/as_u16 |
| `error.rs` | 160 | IpcError enum with diagnostic codes |
| `bounded.rs` | 68 | QueueCapacity, MaxPayloadBytes, BoundedPayload |
| `frame_types.rs` | 175 | IpcFrameHeader, IpcFrame, decode_frame |
| `payloads.rs` | 120 | IpcPayload, SubmitRunPayload, RunListState, RunSummary |
| `metrics.rs` | 74 | RuntimeMetrics, ShardMetrics, JournalMetrics, IpcMetrics, AggregateMetrics |
| `action_output.rs` | 29 | IpcActionOutputPayload |
| `trace.rs` | 49 | IpcTraceEvent, IpcTraceEventKind |
| `ingress.rs` | 98 | IngressFrame, MemoryIngress |
| `codec.rs` | 21 | encode_payload, decode_payload |

### Modified Files
- `lib.rs`: Reduced from 3085 to 45 lines. Now declares modules and re-exports public types for backward compatibility with existing submodules (frame.rs, client.rs, server.rs).
- `client.rs`: Fixed import of `WorkflowDigest` to use `vb_core::WorkflowDigest` instead of `crate::WorkflowDigest`.

### Deleted Files (Orphaned)
- `ipc_types.rs`: Duplicate definitions, not compiled
- `tests.rs`: Duplicate tests, not compiled

## Backward Compatibility
- Existing submodules (frame.rs, client.rs, server.rs) import types from `crate::`
- lib.rs re-exports all public types at crate root to maintain compatibility
- No changes to public API surface

## Files Still Exceeding 300 Lines
Note: These are pre-existing submodule files NOT part of the original monolithic lib.rs:
- `frame.rs` (1189 lines): IPC frame I/O utilities - pre-existing
- `frame/tests.rs` (646 lines): Tests for frame.rs - pre-existing
- `server/handlers.rs` (351 lines): Server request handlers - pre-existing
- `client.rs` (349 lines): IPC client - pre-existing

These were not in scope for this decomposition (they are submodules, not the 3085-line lib.rs).

## Verification
- `cargo check -p vb_ipc`: Passes
- `cargo test -p vb_ipc`: 84 tests pass
