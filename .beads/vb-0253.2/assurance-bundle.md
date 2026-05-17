# State 13 Assurance Bundle

Bead: `vb-0253.2`

Claims:
- One canonical `MemoryIngress`, `IngressFrame`, `QueueCapacity`, `MaxPayloadBytes`, `BoundedPayload`, and `IpcError` implementation remains.
- Public `vb_ipc` symbols remain available through facade re-exports.
- Scoped behavior gates pass.
- Workspace/reference hygiene is repaired; local `main` resolves and the change is rebased onto `5ba93c4ddc9375cd85c1d21d5419202d228a9816`.

Raw evidence:
- `rtk cargo check -p vb_ipc` -> PASS.
- `rtk cargo test -p vb_ipc` -> PASS, `628 passed`.
- `rtk cargo clippy -p vb_ipc --lib -- -D warnings` -> PASS.
- `cargo kani -p vb_ipc --harness kani_ipc_header_decode_valid --quiet` -> PASS, `exit_code=0`.
- duplicate definition grep found six canonical definitions in `bounded.rs`, `error.rs`, `ingress.rs` only.
- `rtk wc -l crates/vb_ipc/src/lib.rs` -> `58`.

Global evidence:
- `moon ci` rerun with `main` resolvable -> FAIL_GLOBAL on unrelated `xtask` format/lint, `vb_storage` test warning debt, and `vb_cli` mode-module/import drift.
- This is documented as global debt and not a `vb_ipc` bookmark blocker.
