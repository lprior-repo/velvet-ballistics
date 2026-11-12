# Stale-ticket classification candidate

The 5090 invocation `vb-ticket-stale` generated the precise mismatch
rejection branch, but emitted it as XML pseudo-tool text instead of editing
the file. The coordinator inspected and applied that generated branch to
`shard/lifecycle/chunk_004.rs`. This is not a successful local tool invocation.

The only successful ownership branch remains `pending == ticket`. A valid
older ticket with matching immutable context returns `Err(StaleAttempt)`;
other mismatches remain errors. Master section 72 explicitly requires the
stale error classification. The earlier step/action-only acceptance edit
was rejected, restored, and is absent.

Actual command after applying the generated patch:

`env RUSTC_WRAPPER= CARGO_BUILD_JOBS=2 timeout 300 cargo test -p vb_runtime --lib`

Result: 1903 passed, zero failed or ignored. This is existing-suite regression
evidence only. New stale/forged-ticket cases and the nine integration repairs
still require execution and independent review before acceptance.

The public `Shard::pending_action_get` already exists in
`shard/impl_parts/chunk_001.rs`; no visibility/API expansion is required.
