# State 7 Test Plan

Scope: `vb_ipc` facade modularization and ingress dedupe.

Required tests:
- Public API imports compile through crate-root re-exports.
- `MemoryIngress::bounded` accepts one frame and rejects second frame at capacity one with `IpcError::Full`.
- Empty ingress returns `Ok(None)`.
- FIFO receive order remains stable.
- Oversized payload returns `IpcError::PayloadTooLarge`.
- IPC command surface retains command IDs 1-16.

Commands:
- `rtk cargo test -p vb_ipc`
- `rtk cargo clippy -p vb_ipc --lib -- -D warnings`
