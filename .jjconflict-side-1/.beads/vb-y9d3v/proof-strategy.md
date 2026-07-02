# Proof Strategy — vb-y9d3v ActionTicket Generation Fence

## Bead Context

- Bead: vb-y9d3v (fresh replacement for vb-8mdp.5)
- State: 4 (proof-planner)
- Source: accepted State 3 contract, 12 proof seeds, hazard analysis, traceability matrix
- Prior vb-8mdp.5 evidence is REJECTED and serves as context only

## Strategy Overview

This bead requires implementation-bound proof that the ActionTicket generation fence prevents race-conditioned ticket issuance, duplicate authority, and lost authority under concurrent external completion/failure/cancellation scenarios. The fence operates at the shard boundary: external callers supply untrusted `ActionTicket` DTOs; only the shard-owned runtime state grants authority to mutate.

## Domain Decisions Driving Lane Selection

1. **Future attempts are invalid authority** (Domain Decision 1, contract ACT-006). Fresh-main code currently accepts future attempts within capacity; this gap must be closed by implementation or explicitly waived by owner.
2. **Invalid authority is non-mutating** (Domain Decision 5). All authority rejections occur before journal/frame/trace/counter mutation.
3. **Retry capacity is a bound, not authority** (Domain Decision 3). `capacity` limits attempts but does not authorize unrecorded future attempts.
4. **Timer generation is monotonic** (Domain Decision from type-contracts). Stale timer entries must not resume wait/ask state.

## Risk Classification Summary

| Risk Class | Seeds | Primary Verifier(s) |
|---|---|---|
| Rust-local invariant (attempt freshness, key validation, bounds) | 001, 002, 003, 004, 005, 006, 007 | Verus, Kani, Flux, proptest |
| Rust-local (retry, state machine) | 003, 005, 008 | Kani |
| Rust-local (type refinement) | 009 | Flux |
| Rust-local (property test) | 010 | proptest |
| Codec boundary (retry counter encode/decode) | 011 | cargo-fuzz |
| Temporal/protocol (action authority lifecycle, timer replacement) | 012 | tla-plus (not_applicable — globally removed) |
| Concurrent interleaving | None in scope | Loom not_applicable |
| Unsafe/UB | None in scope | Miri not_applicable |

## Lane Profile Assignment

### Default Rust-Implementation Lanes (Seeds 001-010)

Every rust-local seed (001-010) receives Verus, Kani, Flux-rs, and proptest lanes. This provides defense-in-depth:
- **Verus** proves pure/core invariants (attempt equality, non-mutation, retry arithmetic)
- **Kani** provides bounded model checking for panic-freedom, bounds violations, and counterexample generation
- **Flux** enforces refinement types (attempt non-zero, capacity > 0, attempt ≤ capacity)
- **proptest** exercises the public API with generated hostile inputs

### Seed 011: Codec Boundary

Seed 011 (`REQ-fuzz-retry-gate`) targets the retry counter codec boundary. Only cargo-fuzz is required; the four Rust-default lanes (Verus, Kani, Flux-rs, proptest) are `not_applicable` because the codec is a trivially invertible u16↔bytes mapping where cargo-fuzz+ASAN provides exhaustive coverage. See VLD-vb-y9d3v-0081 through VLD-vb-y9d3v-0084.

### Conditional / Not-Applicable Lanes

- **cargo-fuzz** (seed 011 only): Required for retry counter codec boundary. The retry counter is serialized in journal/IPC and must be robust against arbitrary byte sequences.
- **tla-plus** (seed 012): `not_applicable` — TLA+ has been globally removed from the verifier whitelist per project decision. Seed 012 remains as temporal design context only; Rust-local invariants are covered by seeds 001-010.
- **Loom**: `not_applicable`. Authority checks are synchronous within the shard boundary. No concurrent atomics, locks, channels, or task-ownership interleaving occurs in the authority fence logic. External completions are processed sequentially by the shard.
- **Miri**: `not_applicable`. All files under scope enforce `#![forbid(unsafe_code)]`. No FFI, raw pointers, or UB-sensitive constructs exist in the authority path.

## Obligation Grouping

Obligations are grouped by verifier and target production function, covering multiple proof seeds where they share the same code path:

### Kani (10 obligations)
- PO-0001, PO-0005, PO-0009, PO-0013, PO-0017, PO-0021, PO-0025, PO-0029, PO-0033, PO-0037: One Kani harness per seed (seeds 001-010), targeting `kani_attempt_fence_harnesses::check_attempt_fence` with per-seed domain claims.

### Verus (10 obligations)
- PO-0002, PO-0006, PO-0010, PO-0014, PO-0018, PO-0022, PO-0026, PO-0030, PO-0034, PO-0038: One Verus proof per seed (seeds 001-010), targeting `proof fn action_fence_correct` with per-seed domain claims.

### Flux-rs (10 obligations)
- PO-0003, PO-0007, PO-0011, PO-0015, PO-0019, PO-0023, PO-0027, PO-0031, PO-0035, PO-0039: One Flux refinement obligation per seed (seeds 001-010), targeting `#[sig] on validate_ticket_attempt` with per-seed domain claims.

### proptest (10 obligations)
- PO-0004, PO-0008, PO-0012, PO-0016, PO-0020, PO-0024, PO-0028, PO-0032, PO-0036, PO-0040: One proptest property per seed (seeds 001-010), targeting `proptest_attempt_fence::prop_attempt_freshness` with per-seed domain claims.

### cargo-fuzz (1 obligation)
- PO-0041: Retry counter codec fuzz target (seed 011). Exercises retry-counter encode/decode with arbitrary byte sequences under ASAN.

**No TLA+ obligations exist.** TLA+ has been globally removed from the verifier whitelist and is `not_applicable` for all seeds.

## Trusted Base Planning

The following trusted surfaces require explicit trusted-base-ledger rows (detailed in `trusted-base-plan.md`):
- `compute_action_idempotency_key` wrapping arithmetic (trusted as total, no overflow concern due to wrapping_mul/add)
- `vb_core` public types (`ActionTicket`, `RunId`, `StepIdx`, `SeqNo`) as trusted DTO boundaries
- Postcard serialization as trusted codec for length validation
- Workflow compilation as trusted (compiled by host, not part of runtime fence)

## Implementation Gap Handling

The primary implementation gap (future attempts accepted within capacity in `validate_ticket_attempt`) is reflected in:
1. Contract ACT-006 explicitly requires future-attempt rejection
2. Proof obligations seed-002 and related Verus/Kani/Flux/proptest obligations all target the corrected semantics
3. The bridge (`proof-to-implementation-input.md`) maps these to explicit source locations that must change

## Bridge Readiness

All planned obligations target production functions (`crates/vb_runtime/src/shard/helpers.rs`, `crates/vb_runtime/src/shard/lifecycle/chunk_003.rs`, `crates/vb_core/src/action.rs`, `crates/vb_runtime/src/shard/timer_wheel.rs`). No detached model-only proofs are planned. The `proof-to-implementation-input.md` maps each obligation to source refs, behavior test refs, and refinement harness refs.

## Non-Applicability Evidence Summary

| Verifier | Seeds | Reason | Evidence |
|---|---|---|---|
| Loom | 001-012 | Synchronous shard boundary; no concurrent atomics/locks/channels in authority path | boundary-map.md §Async/Concurrency Boundary; `#![forbid(unsafe_code)]` in-scope files |
| Miri | 001-012 | No unsafe, FFI, raw pointers, or UB-sensitive code in authority path | All in-scope files enforce `#![forbid(unsafe_code)]` |
| cargo-fuzz | 001-010, 012 | No parser/codec boundary in these seeds' domain claims | Seeds 001-010/012 risk tags lack codec/fuzz triggers |
| tla-plus | 001-012 | TLA+ globally removed from verifier whitelist | TLA_PLUS_REMOVED_GLOBAL; temporal design evidence only per seed 012 notes |
| Verus | 011 | Trivial u16 codec; cargo-fuzz+ASAN sufficient | VLD-vb-y9d3v-0081 |
| Kani | 011 | Trivial u16 codec; cargo-fuzz+ASAN sufficient; bounds covered by seed 003/008 Kani obligations | VLD-vb-y9d3v-0082 |
| Flux-rs | 011 | Trivial u16 codec; cargo-fuzz+ASAN sufficient; u16 type bounds covered by seed 009 Flux obligations | VLD-vb-y9d3v-0083 |
| proptest | 011 | Trivial u16 codec; cargo-fuzz+ASAN sufficient | VLD-vb-y9d3v-0084 |

## Artifact Inventory

| Artifact | Status |
|---|---|
| `proof-strategy.md` | this file |
| `verifier-lane-matrix.md` | written |
| `verifier-lane-decisions.jsonl` | written (96 rows) |
| `proof-coverage-matrix.md` | written |
| `proof-obligations.planned.jsonl` | written (41 obligations) |
| `trusted-base-plan.md` | written (8 rows planned) |
| `waiver-candidates.jsonl` | written (0 candidates — no non-behavior exceptions identified) |
| `proof-to-implementation-input.md` | written |
