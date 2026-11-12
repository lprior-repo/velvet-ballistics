# Verification Layers: vb-te1i Binary IPC BDD Acceptance

## Boundary

- **Verus-owned kernel**: Pure frame decode/encode, command ID mapping, payload bounds, correlation roundtrip, error variant exhaustiveness
- **TLA+ temporal model**: None (not applicable — see tla-spec.md)
- **Theorem projection**: None (Verus sufficient — see lean-contract.md)
- **Runtime shell**: Async `serve_ipc`, Unix socket I/O, `MemoryIngress` queue push/pop, mio event loop
- **External systems**: Unix kernel socket buffers, OS scheduler

---

## Layer Assignment

| Contract Clause | Primary Layer | Secondary Layer(s) | Notes |
|---|---|---|---|
| PRE-001 | unit_tests | — | Fixed 24-byte slice passed to decode |
| PRE-002 | unit_tests | — | MaxPayloadBytes is NonZeroUsize; enforced at construction |
| PRE-003 | unit_tests | — | IpcFrame::new checks length agreement |
| POST-001 (roundtrip) | unit_tests | kani | Roundtrip encode→decode for all valid headers |
| POST-002 (health) | bdd_integration | — | Real IPC scenario: Health command roundtrip |
| POST-003 (shutdown) | bdd_integration | — | Real IPC scenario: Shutdown command |
| POST-004 (submit_run) | bdd_integration | — | Real IPC scenario: SubmitRun roundtrip with correlation |
| POST-005 (bad_magic) | kani | unit_tests | Kani harness proves InvalidMagic before payload access |
| POST-006 (version_mismatch) | unit_tests | — | Test rejects version ≠ 1 |
| POST-007 (unknown_command) | unit_tests | — | from_u16 returns UnknownCommand for 0, 17+ |
| POST-008 (reserved_nonzero) | unit_tests | — | Reserved field enforcement |
| POST-009 (payload_too_large) | kani | unit_tests | Kani harness: payload_len > max → PayloadTooLarge |
| POST-010 (payload_length_mismatch) | unit_tests | — | IpcFrame::new rejects mismatch |
| POST-011 (backpressure_full) | unit_tests | proptest | Queue capacity exhaustion property |
| POST-012 (disconnected) | unit_tests | — | Queue disconnect error |
| INV-001 (header_len) | unit_tests | — | Compile-time constant 24 |
| INV-002 (magic_immutable) | unit_tests | — | IPC_MAGIC constant test |
| INV-003 (command_range) | unit_tests | verus | Exhaustive command ID mapping |
| INV-004 (decode_before_alloc) | kani | verus | Kani proves magic/version/command/reserved/bounds checked before payload read |
| INV-005 (bounded_payload) | verus | unit_tests | Verus invariant + unit test |
| INV-006 (correlation_preserved) | unit_tests | verus | Roundtrip + Verus proof |
| INV-007 (diagnostic_code_stable) | unit_tests | — | Every error variant has a stable code |

---

## Verus Scope

**Target**: `crates/vb_ipc/src/frame_types.rs`, `crates/vb_ipc/src/bounded.rs`, `crates/vb_ipc/src/commands.rs`

**Spec/proof surface**:
- `spec fn header_decode_spec(bytes: [u8; 24], max: MaxPayloadBytes) -> Result<IpcFrameHeader, IpcError>`
- `proof fn decode_before_alloc_proof(bytes, max)` — proves all checks happen before payload access
- `proof fn bounded_payload_invariant(payload, max)` — proves len ≤ max
- `proof fn correlation_preserved_proof(header)` — proves `decode(encode(h)) == h`
- `proof fn command_range_proof(n: u16)` — proves from_u16 exhaustive on 1..=16

**Trusted boundary**: `MaxPayloadBytes`, `QueueCapacity` constructors require `NonZeroUsize`; `BoundedPayload::new` requires length check; no other constructors can bypass bounds.

**Shell exclusions**: I/O, async scheduling, storage, Unix sockets, mio event loop.

**Evidence command**: `moon run :verify-proof` (Verus lane) targeting `vb_ipc` crate.

---

## Kani Scope

**Target**: `crates/vb_ipc/src/kani_ipc_header.rs`, `crates/vb_ipc/src/kani_ipc_header_rejects_oversize.rs`

**Property**: Bounded panic-freedom and correct error return for all 24-byte inputs.

**Evidence command**: `cargo kani --workspace` or `moon run :verify-proof` (Kani lane).

---

## BDD / Integration Layer

**Target**: `crates/workspace_tests/tests/vb_te1i_binary_ipc_acceptance.rs` (to be created)

**Scenarios**:
- `ipc_submit_run_roundtrips_when_frame_is_valid` — happy path with real Unix socket
- `ipc_health_and_shutdown_return_expected_responses` — Health/Shutdown commands
- `ipc_rejects_bad_magic_before_payload_allocation` — adversarial frame
- `ipc_returns_queue_full_when_backpressure_limit_is_hit` — backpressure scenario
- `ipc_rejects_oversize_payload` — bounds scenario
- `ipc_correlation_ids_preserved_across_roundtrip` — correlation scenario
- `ipc_all_16_commands_have_typed_responses` — exhaustive command coverage

**Evidence**: Scenario runner pass/fail with exact scenario IDs.

---

## Loom Scope

**Target**: `crates/vb_ipc/src/queue/tests/array_queue_tests.rs`, concurrent `serve_ipc` operations.

**Property**: SPSC queue push/pop with simulated mio event loop interleavings.

**Evidence**: Loom permutation test passes with no deadlocks or assertion failures.

---

## Proptest Scope

**Target**: `crates/vb_ipc/src/queue/tests/array_queue_tests.rs`

**Property**: Backpressure behavior holds across randomly generated queue capacities and payload sizes.

---

## Fuzzing Scope

**Target**: `crates/vb_ipc/src/frame.rs` / `frame_types.rs` adversarial decode paths.

**Evidence**: `cargo fuzz run parse` (existing fuzz target) completes 1000 runs without panic.

---

## Waivers

| Clause | Reason | Owner | Compensating Evidence |
|---|---|---|---|
| TLA+ for server concurrency | mio event loop is not a TLA+-suitable model | vb-te1i | Loom for concurrency; Kani for header decode |
| TLA+ for frame decode | Pure function, not temporal | vb-te1i | Kani harness + unit tests |
| Lean/Aeneas/Hax | Verus covers all critical properties | vb-te1i | Verus + Kani + unit tests |
| Verus for async `serve_ipc` loop | Async shell; not pure | vb-te1i | Loom + integration tests |
