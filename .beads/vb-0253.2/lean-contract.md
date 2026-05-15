# Theorem Kernel Projection: vb-0253.2

## Boundary

- **TLA+-owned temporal model**: None — no temporal behavior in this refactor
- **Verus-owned Rust core**: Not required — facade conversion is structural refactor; all behavioral contracts are exercised by existing tests
- **Theorem-owned kernel**: None — no tiny algebraic theorem kernels, protocol lattices, arithmetic bounds, parser/codec theorems, or refinement claims beyond what the existing test suite covers
- **Rust/runtime shell**: The facade conversion touches only module declarations and re-exports in `lib.rs`. All behavior is delegated to the unchanged canonical modules.
- **External systems excluded from theorem proof**: `crossbeam_channel` (trusted runtime), `postcard` (trusted serialization), `bytes` (trusted buffer)

## Theorem-Owned Clauses

- **None** — explicit waiver: no theorem kernel projection is required for a pure facade-conversion refactor that removes duplicate definitions and adds module re-exports.

## Theorem Obligations

### THM-NONE-001

- **Contract clause**: N/A
- **Rust/spec target**: N/A
- **Lean module**: N/A
- **Theorem shape**: N/A
- **Model**: N/A
- **Refinement**: N/A
- **Shell exclusions**: N/A
- **Evidence command**: N/A

## Waivers

| Clause | Owner | Reason | Expiry | Compensating Evidence |
|---|---|---|---|---|
| Any theorem/proof kernel projection | vb-0253.2 agent | Facade refactor is a pure re-export reorganization; no pure/deterministic critical invariants beyond the existing test suite | N/A | `cargo test -p vb_ipc` passes with 60+ test cases covering QueueCapacity, MaxPayloadBytes, BoundedPayload, IngressFrame, MemoryIngress, IpcError, encode/decode_payload, frame roundtrips, adversarial cases |
| Verus proof for BoundedPayload parse-don't-validate | vb-0253.2 agent | Parse-don't-validate contract (payload.len() > max.get() -> Err(IpcError::PayloadTooLarge)) is covered by unit tests including boundary cases | N/A | `bounded_payload_rejects_oversized_with_exact_counts`, `bounded_payload_rejects_one_over_max`, `ingress_frame_rejects_payload_exceeding_max`, `adversarial_bounded_payload_rejects_exactly_one_over_max` |
| Verus proof for MemoryIngress channel semantics | vb-0253.2 agent | crossbeam_channel is trusted runtime; facade conversion does not change channel capacity, FIFO ordering, or disconnect behavior | N/A | `bounded_queue_applies_backpressure`, `try_submit_returns_full_when_at_capacity`, `memory_ingress_disconnected_after_sender_drop`, `adversarial_memory_ingress_full_then_drain_then_submit` |
