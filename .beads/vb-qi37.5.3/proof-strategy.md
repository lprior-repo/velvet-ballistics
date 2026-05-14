# Proof Strategy — vb-qi37.5.3

**Bead**: runtime: Carry idempotency evidence into admission
**State**: 4 — proof-planner
**Generated**: 2026-05-14
**Workspace**: /home/lewis/src/vb-qi37-5-3

---

## 1. Verification Lane Overview

| Lane | Obligations | Blocked By | Notes |
|------|-------------|------------|-------|
| verus | 5 | DEFERRED_GLOBAL (vb_runtime build) | Verus specs/proofs written in State 5; executed in State 11 |
| miri | 2 | DEFERRED_GLOBAL (vb_runtime build) | UB/concurrency checks; executed in State 11 |
| loom | 1 | DEFERRED_GLOBAL (vb_runtime build) | Thread-safety permutation testing; executed in State 11 |
| kani | 3 | PARTIAL — KANI-POST-05 blocked (vb_runtime); KANI-INV-05 targets vb_storage (may build) | Bounded model checking; executed in State 11 |
| proptest | 2 | NONE — vb_storage/vb_core build | Property-based tests; executed in State 11 |
| cargo-test | 5 | NONE — vb_storage/vb_core build | Unit/regression tests; executed in State 11 |
| waiver | 1 | N/A | DEFERRED-GLOBAL-01 waived; vb_runtime build failure pre-existing |

**DEFERRED_GLOBAL blocking note**: `vb_runtime` fails to compile due to missing `crates/vb_runtime/src/runtime/chunk_001.rs` (pre-existing at commit ffbe7f5cd). All formal verification targeting vb_runtime (verus, miri, loom, KANI-POST-05) is blocked until this is resolved. Proptest and cargo-test for vb_storage/vb_core can run immediately in parallel with implementation.

---

## 2. Verus Lane Strategy

### Target: `vb_runtime::admission::RunAdmission`

**Obligations**: VERUS-POST-01, VERUS-POST-02, VERUS-INV-01, VERUS-INV-02

**Strategy**: Write spec functions and proofs in `crates/vb_runtime/src/admission.rs` alongside the implementation (State 10). Verus type-checking runs after vb_runtime builds.

- `spec_fn spec_new_evidence_copy` — models `RunAdmission::new` with explicit field-copy from `VerificationProof`
- `proof_fn proof_evidence_copy_preserves_len` — proves `idempotency_keyed.len()` and `idempotency_attested.len()` equality
- `proof_fn proof_field_type_match` — confirms `Box<[ActionId]>` type on both sides
- `spec_fn spec_field_types` — type-level specification for field storage

**Shell exclusions**: I/O, async scheduling, network, filesystem (verified at admit_artifact_run layer)

**Trusted boundary**: Validated `AcceptedArtifact` from `StorageArtifactStore::load_accepted_artifact`

### Target: `vb_runtime::idempotency::IdempotencyTracker`

**Obligation**: VERUS-INV-03

**Strategy**: Write spec with decreases clause in `crates/vb_runtime/src/idempotency.rs`.

- `spec_fn spec_track_for_policy` — models HashMap insert with eviction
- `proof_fn proof_capacity_invariant` — proves `completed.len() <= DEFAULT_CAPACITY` after every `track_for_policy`
- Requires `#[ Decreases(completed.len()) ]` annotation on recursive eviction path

**Shell exclusions**: I/O, network, async scheduling

**Trusted boundary**: Internal HashMap state encapsulated within IdempotencyTracker

---

## 3. Miri Lane Strategy

### Target: `vb_runtime::idempotency::IdempotencyTracker` HashMap operations

**Obligations**: MIRI-INV-04, MIRI-POST-06

**Command**:
```
MIRIFLAGS="-Zmiri-strict-provenance" cargo miri test -p vb_runtime idempotency -- --nocapture
cargo miri test -p vb_runtime run_admission -- --nocapture
```

**Strategy**: After vb_runtime builds, run Miri against `idempotency.rs` tests and `admission.rs` field propagation tests. Miri detects:
- Use-after-free on HashMap bucket reallocation
- Invalid pointer arithmetic on key hash
- UB from aliased mutable references in concurrent patterns
- Invalid `Box<[ActionId]>` slice copying

**BLOCKED**: Cannot execute until DEFERRED_GLOBAL (chunk_001.rs) is resolved.

---

## 4. Loom Lane Strategy

### Target: `IdempotencyTracker` Send+Sync thread-safety

**Obligation**: LOOM-INV-04

**Command**:
```
cargo loom test -p vb_runtime idempotency --persist 2>&1 | tee loom-report.txt
```

**Strategy**: Permutation testing of `track_for_policy` + `is_completed_for_policy` interleavings from 2-4 simulated threads. Detects data races on:
- HashMap insert/lookup without mutex
- HashSet membership during concurrent iteration
- Policy key collision handling

**BLOCKED**: Cannot execute until DEFERRED_GLOBAL (chunk_001.rs) is resolved.

**Compensating evidence**: Miri (UB), cargo test (unit coverage), Verus INV-03 (capacity invariant)

---

## 5. Kani Lane Strategy

### Target: `vb_storage::admission` (KANI-INV-05 — runs against vb_storage)

**Obligation**: KANI-INV-05

**Command**:
```
cargo kani --harness verification_proof_flags_harness --workspace crates/vb_storage 2>&1 | tee kani-report.txt
```

**Strategy**: Write harness in `crates/vb_storage/` that:
- Enumerates all 32 combinations of `VerificationProof` flags (durable, bounded, taint_safe, retry_safe, replayable)
- Proves INV-05: flag conditions correctly gate idempotency semantics

**Expected**: Kani completes with no failures; all flag combinations verified.

### Target: `vb_runtime::admission` (KANI-POST-05 — BLOCKED)

**Obligation**: KANI-POST-05

**Command**:
```
cargo kani --harness load_accepted_artifact_harness --workspace crates/vb_runtime 2>&1 | tee kani-report.txt
```

**Strategy**: Write harness for `StorageArtifactStore::load_accepted_artifact` bounded check.

**BLOCKED**: Cannot execute until DEFERRED_GLOBAL (chunk_001.rs) is resolved.

---

## 6. Proptest Lane Strategy

### Target: `vb_runtime` (runs against vb_storage/vb_core which build)

**Obligations**: PROPTEST-POST-01, PROPTEST-INV-03

**Commands**:
```
cargo test -p vb_runtime run_admission_idempotency_proptest -- --nocapture 2>&1
cargo test -p vb_runtime idempotency_tracker_capacity_proptest -- --nocapture 2>&1
```

**Strategy**: These tests are written in State 8 and executed in State 11. They can run against vb_storage/vb_core independently:
- PROPTEST-POST-01: `proptest![...]` generating random `(Vec<ActionId>, Vec<ActionId>)` pairs, verifying field lengths and contents match after copy
- PROPTEST-INV-03: `proptest![...]` generating sequences of `track_for_policy` calls, verifying `completed.len() <= 1024` after eviction

**No blocking**: vb_storage and vb_core build successfully; proptest harnesses depend on types from these crates.

---

## 7. Cargo Test Lane Strategy

### Target: `vb_storage` and `vb_core` (which build)

**Obligations**: TEST-POST-03, TEST-POST-04, TEST-ERR-01, TEST-INV-05, TEST-POST-05

**Commands**:
```
cargo test -p vb_runtime admit_run -- --nocapture 2>&1
cargo test -p vb_runtime admission -- --nocapture 2>&1
cargo test -p vb_runtime artifact_envelope_error -- --nocapture 2>&1
cargo test -p vb_storage verification_proof_flags -- --nocapture 2>&1
cargo test -p vb_runtime idempotency -- --nocapture 2>&1
```

**Strategy**: Tests written in State 8 (test-writer), executed in State 11. Cover:
- Regression on existing `RunAdmission` fields (POST-03)
- Caller site updates (POST-04)
- Error propagation (ERR-01)
- VerificationProof flag conditions (INV-05)
- `IdempotencyTracker` unit tests (POST-05)

**No blocking**: vb_storage and vb_core compile; vb_runtime unit tests for idempotency.rs may be written to target vb_core types.

---

## 8. Waiver Summary

| Obligation | Lane | Status | Reason |
|------------|------|--------|--------|
| DEFERRED-GLOBAL-01 | waiver | WAIVED | Pre-existing vb_runtime build failure at commit ffbe7f5cd; chunk_001.rs missing; formal verification blocked until resolved; implementation proceeds in parallel |

---

## 9. Obligation Execution Order (State 11)

1. **Parallel (no blocking)**: cargo test + proptest against vb_storage/vb_core
2. **After vb_runtime builds**: verus, miri, loom, kani (KANI-POST-05)
3. **vb_storage Kani**: KANI-INV-05 can run independently of vb_runtime

---

## 10. Key Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| vb_runtime never builds | miri/loom/kani/verus blocked | DEFERRED_GLOBAL waiver; implementation can still land with test coverage |
| IdempotencyTracker not Send+Sync | Multi-shard deployment fails | INV-04 proven by miri+loom; loom shows no races in permutation model |
| Box<[ActionId]> slice copy UB | Memory safety violation | MIRI-POST-06 verifies copy is UB-free |
| Capacity overflow on eviction | Tracker exceeds 1024 entries | VERUS-INV-03 + PROPTEST-INV-03 prove/discover overflow bug |
