# Proof Plan Review Input: vb-te1i

## Quick Summary

Bead vb-te1i targets binary IPC frame codec acceptance — 24-byte fixed header, 16 v1 commands, bounded payload, SPSC queue backpressure, Unix domain socket server. Proof obligations cover header decode-before-alloc (Kani), command range exhaustive mapping (Verus), bounded payload invariant (Verus), encode/decode roundtrip (Verus), payload length agreement (Verus), 10 unit test scenarios, 7 BDD integration scenarios, and 1 static clippy scan.

## Risk Posture

| Tag | Severity | Lane | Rationale |
|---|---|---|---|
| parser/codec | high | Kani + Fuzz | Adversarial magic/version/command/bounds; Fuzz blocked_tooling |
| backpressure | high | proptest + unit | Queue capacity exhaustion; Proptest waived |
| concurrency | medium | Loom | SPSC/mio poll loop; Loom waived (tooling missing) |
| public_api | high | BDD integration | Unix socket public surface; BDD file MISSING |

## Verdict by Lane

| Lane | Status | Count | Notes |
|---|---|---|---|
| TLA+ | **not_applicable** | 0 | Pure data-validation; no temporal/concurrent/state-machine behavior |
| Verus | **planned** | 4 | VERUS-001..004; tool available |
| Kani | **planned** | 3 | KAN-001..003; tool + harnesses available |
| Miri | **not_applicable** | 0 | `#[forbid(unsafe_code)]` throughout vb_ipc |
| Loom | **waived** | 1 | `cargo-loom` not installed; compensating BDD+unit coverage |
| Proptest | **waived** | 1 | Not in scope; compensating unit+BDD coverage |
| Fuzz | **blocked_tooling** | 1 | `cargo-fuzz` not installed; Kani+unit compensate |
| Unit | **planned** | 10 | All test files exist; runnable |
| BDD integration | **planned** | 7 | `vb_te1i_binary_ipc_acceptance.rs` **MISSING** — must create |
| Clippy | **planned** | 1 | Runnable |

## Critical Open Items

1. **BDD acceptance file missing**: `crates/workspace_tests/tests/vb_te1i_binary_ipc_acceptance.rs` must be created before BDD-001..BDD-007 can execute. Tracked as part of the bead implementation.
2. **FUZZ-001 blocked**: `cargo-fuzz` not installed. Risk is partially mitigated by Kani (KAN-001/KAN-003) and adversarial unit tests (UNIT-002). Consider libFuzzer or `cargo-fuzz` installation.
3. **LOOM-001 waived**: Concurrency risk (SPSC queue + mio event loop) is covered by BDD integration tests and UNIT-008. Acceptable for this bead.

## Coverage Matrix (selected high-risk)

| Contract clause | Kani | Verus | Unit | BDD |
|---|---|---|---|---|
| POST-005 bad_magic_before_alloc | KAN-001 | — | UNIT-002 | BDD-003 |
| POST-009 payload_too_large | KAN-002 | — | UNIT-006 | BDD-007 |
| INV-004 decode_before_alloc | KAN-003 | — | UNIT-002,003,005,006 | — |
| INV-003 command_range | — | VERUS-001 | UNIT-004 | BDD-005 |
| INV-005 bounded_payload | — | VERUS-002 | UNIT-006 | — |
| INV-006 correlation_preserved | — | VERUS-003 | — | BDD-006 |
| POST-011 backpressure | — | — | UNIT-008 | BDD-004 |

## Reviewer Action Required

Reviewer should validate:
- Waiver rationale for LOOM-001, PROPTEST-001, FUZZ-001 is acceptable given compensating evidence.
- BDD file creation is tracked and blocked on implementation.
- Kani/Verus obligations are correctly scoped to vb_ipc only (no cross-crate proofs required).
