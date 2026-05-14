# Proof Plan Review Input — vb-qi37.5.3

**Bead**: runtime: Carry idempotency evidence into admission
**State**: 4 — proof-planner output for State 6 reviewer
**Generated**: 2026-05-14

---

## 1. Bead Summary

**Title**: runtime: Carry idempotency evidence into admission
**Gap**: `RunAdmission` lacks `idempotency_keyed` and `idempotency_attested` fields from `VerificationProof`; `IdempotencyTracker` thread-safety unverified.
**Touched crates**: vb_runtime (primary), vb_storage (upstream), vb_core (types)
**DEFERRED_GLOBAL**: vb_runtime fails to build due to pre-existing missing `chunk_001.rs` — blocks formal verification only; implementation can proceed.

---

## 2. Obligation Inventory (18 total)

| ID | Clause | Verifier | Target | Status |
|----|--------|----------|--------|--------|
| VERUS-POST-01 | POST-01 | verus | RunAdmission::new | planned |
| VERUS-POST-02 | POST-02 | verus | RunAdmission | planned |
| VERUS-INV-01 | INV-01 | verus | RunAdmission::new | planned |
| VERUS-INV-02 | INV-02 | verus | RunAdmission::new | planned |
| VERUS-INV-03 | INV-03 | verus | IdempotencyTracker::track_for_policy | planned |
| MIRI-INV-04 | INV-04 | miri | IdempotencyTracker HashMap | BLOCKED_DEFERRED_GLOBAL |
| MIRI-POST-06 | POST-06 | miri | RunAdmission Box<[ActionId]> copy | BLOCKED_DEFERRED_GLOBAL |
| LOOM-INV-04 | INV-04 | loom | IdempotencyTracker Send+Sync | BLOCKED_DEFERRED_GLOBAL |
| KANI-POST-05 | POST-05 | kani | StorageArtifactStore::load_accepted_artifact | BLOCKED_DEFERRED_GLOBAL |
| KANI-INV-05 | INV-05 | kani | VerificationProof flags (vb_storage) | planned |
| PROPTEST-POST-01 | POST-01 | proptest | RunAdmission field propagation | planned |
| PROPTEST-INV-03 | INV-03 | proptest | IdempotencyTracker capacity eviction | planned |
| TEST-POST-03 | POST-03 | cargo-test | RunAdmission existing fields regression | planned |
| TEST-POST-04 | POST-04 | cargo-test | admit_run callers updated | planned |
| TEST-ERR-01 | ERR-01 | cargo-test | ArtifactEnvelopeError propagation | planned |
| TEST-INV-05 | INV-05 | cargo-test | VerificationProof flags | planned |
| TEST-POST-05 | POST-05 | cargo-test | IdempotencyTracker unit tests | planned |
| DEFERRED-GLOBAL-01 | N/A | waiver | chunk_001.rs pre-existing build failure | WAIVED |

---

## 3. Proof Obligations Detail

### 3.1 Verus (5 obligations)

**Scope**: `vb_runtime::admission::RunAdmission` construction invariants and `vb_runtime::idempotency::IdempotencyTracker` capacity bound.

All 5 Verus obligations target vb_runtime which currently fails to build.

#### VERUS-POST-01
- **Claim**: `idempotency_keyed` and `idempotency_attested` copied from `VerificationProof` at `admit_artifact_run` construction time
- **Spec fn**: `spec_new_evidence_copy`
- **Proof fn**: `proof_evidence_copy_preserves_len`
- **Invariants**: `idempotency_keyed.len() == proof.idempotency_keyed.len()`, `idempotency_attested.len() == proof.idempotency_attested.len()`
- **Trusted boundary**: Validated `AcceptedArtifact` from `StorageArtifactStore`
- **Shell exclusions**: I/O, async, network, filesystem

#### VERUS-POST-02
- **Claim**: Fields stored as `Box<[ActionId]>` matching VerificationProof type
- **Spec fn**: `spec_field_types`
- **Proof fn**: `proof_field_type_match`
- **Trusted boundary**: Type system enforces exact type match

#### VERUS-INV-01
- **Claim**: `RunAdmission.idempotency_keyed.len() == VerificationProof.idempotency_keyed.len()` at construction
- **Proof fn**: `proof_idempotency_keyed_len`
- **Trusted boundary**: Non-null VerificationProof from admit_artifact_run

#### VERUS-INV-02
- **Claim**: `RunAdmission.idempotency_attested.len() == VerificationProof.idempotency_attested.len()` at construction
- **Proof fn**: `proof_idempotency_attested_len`
- **Trusted boundary**: Non-null VerificationProof from admit_artifact_run

#### VERUS-INV-03
- **Claim**: `completed.len() <= DEFAULT_CAPACITY` after every `track_for_policy` call; oldest evicted on overflow
- **Spec fn**: `spec_track_for_policy`
- **Proof fn**: `proof_capacity_invariant` with decreases clause
- **Trusted boundary**: Internal HashMap encapsulated

**Reviewer question**: Are the shell exclusions for Verus adequate? `admit_artifact_run` performs storage I/O to load the `AcceptedArtifact` — does Verus need to model this I/O or is the trusted-boundary assumption (caller validates the artifact) sufficient?

---

### 3.2 Miri (2 obligations — BLOCKED DEFERRED_GLOBAL)

Both target vb_runtime which fails to build.

#### MIRI-INV-04
- **Claim**: `IdempotencyTracker` HashMap operations free of UB on concurrent access
- **Command**: `MIRIFLAGS="-Zmiri-strict-provenance" cargo miri test -p vb_runtime idempotency -- --nocapture`
- **Expected**: No UB, data races, or use-after-free

#### MIRI-POST-06
- **Claim**: No UB when copying `Box<[ActionId]>` slices in `RunAdmission` construction
- **Command**: `cargo miri test -p vb_runtime run_admission -- --nocapture`
- **Expected**: No UB on slice copy

**BLOCKED**: Cannot execute until DEFERRED_GLOBAL resolved.

---

### 3.3 Loom (1 obligation — BLOCKED DEFERRED_GLOBAL)

#### LOOM-INV-04
- **Claim**: `IdempotencyTracker` safe for concurrent access (Send+Sync or properly serialized)
- **Command**: `cargo loom test -p vb_runtime idempotency --persist 2>&1 | tee loom-report.txt`
- **Expected**: No data races in `track_for_policy` and `is_completed_for_policy` interleavings
- **Scope**: 2-4 thread permutations

**BLOCKED**: Cannot execute until DEFERRED_GLOBAL resolved.

**Compensating evidence**: Miri for UB, Verus INV-03 for capacity, cargo test for unit coverage.

---

### 3.4 Kani (2 obligations, 1 blocked)

#### KANI-INV-05 (NOT BLOCKED — targets vb_storage)
- **Claim**: If `durable && bounded && taint_safe && retry_safe && replayable` then idempotency_keyed actions have deterministic replay semantics
- **Command**: `cargo kani --harness verification_proof_flags_harness --workspace crates/vb_storage 2>&1 | tee kani-report.txt`
- **Expected**: Kani proves flag conditions correctly gate idempotency semantics

#### KANI-POST-05 (BLOCKED — targets vb_runtime)
- **Claim**: `load_accepted_artifact` returns correct `AcceptedArtifact` with valid `VerificationProof` idempotency fields
- **Command**: `cargo kani --harness load_accepted_artifact_harness --workspace crates/vb_runtime 2>&1 | tee kani-report.txt`
- **Expected**: No panics, assertion failures, or index out-of-bounds

**BLOCKED**: Cannot execute until DEFERRED_GLOBAL resolved.

---

### 3.5 Proptest (2 obligations — NOT BLOCKED)

Both run against vb_storage/vb_core which build.

#### PROPTEST-POST-01
- **Claim**: `idempotency_keyed` and `idempotency_attested` correctly propagated from `VerificationProof` to `RunAdmission` for arbitrary input pairs
- **Command**: `cargo test -p vb_runtime run_admission_idempotency_proptest -- --nocapture 2>&1`
- **Strategy**: `proptest![...]` generating random `(Vec<ActionId>, Vec<ActionId>)` pairs
- **Assertions**: Field lengths equal, contents match

#### PROPTEST-INV-03
- **Claim**: `IdempotencyTracker` capacity never exceeds `DEFAULT_CAPACITY` after eviction on overflow
- **Command**: `cargo test -p vb_runtime idempotency_tracker_capacity_proptest -- --nocapture 2>&1`
- **Strategy**: `proptest![...]` generating sequences of `track_for_policy` calls
- **Assertions**: `completed.len() <= 1024` after every eviction

---

### 3.6 Cargo Test (5 obligations — NOT BLOCKED)

All run against vb_storage/vb_core.

#### TEST-POST-03
- **Claim**: All existing `RunAdmission` fields remain unchanged after adding idempotency fields
- **Command**: `cargo test -p vb_runtime admit_run -- --nocapture 2>&1`

#### TEST-POST-04
- **Claim**: All existing callers of `admit_run`, `admit_artifact_run`, `admit_run_with_budget` provide idempotency evidence
- **Command**: `cargo test -p vb_runtime admission -- --nocapture 2>&1`

#### TEST-ERR-01
- **Claim**: `ArtifactEnvelopeError` and `AdmissionError` variants correctly propagated
- **Command**: `cargo test -p vb_runtime artifact_envelope_error -- --nocapture 2>&1`

#### TEST-INV-05
- **Claim**: `VerificationProof` flags gate idempotency semantics correctly
- **Command**: `cargo test -p vb_storage verification_proof_flags -- --nocapture 2>&1`

#### TEST-POST-05
- **Claim**: `IdempotencyTracker` correctly tracks idempotency_keyed actions; `is_completed_for_policy` returns accurate results
- **Command**: `cargo test -p vb_runtime idempotency -- --nocapture 2>&1`

---

## 4. Waiver: DEFERRED-GLOBAL-01

- **Reason**: Pre-existing `vb_runtime` build failure at commit `ffbe7f5cd` due to missing `chunk_001.rs`. Formal verification blocked.
- **Owner**: External/DEFERRED_GLOBAL
- **Compensating evidence**: Implementation can proceed in parallel; proptest and cargo-test cover vb_storage/vb_core; when chunk_001.rs is restored, all formal lanes can execute.
- **Trigger**: Resolution of `chunk_001.rs` — either restore the file or remove the `include!("runtime/chunk_001.rs")` directive from `runtime.rs`

---

## 5. Review Questions for State 6 (proof-reviewer)

1. **Verus shell exclusions**: Are the I/O exclusions (storage load, envelope validation) appropriately deferred to the caller, or does Verus need to model the storage boundary explicitly?
2. **Loom scope adequacy**: Is 2-4 thread permutation testing sufficient for a HashMap-based tracker, or should we model an explicit mutex around `IdempotencyTracker` and prove the mutex is sufficient?
3. **KANI-INV-05 scope**: Does enumerating all 32 flag combinations constitute adequate coverage, or should we also verify the semantic meaning (e.g., `durable && bounded` implies something specific about replay semantics)?
4. **Waiver adequacy**: Is the DEFERRED_GLOBAL waiver for `chunk_001.rs` properly scoped, or should there be a time-bound follow-up trigger?
5. **Proptest strategy**: Should `PROPTEST-POST-01` use a `Vec<ActionId>` strategy that generates empty slices, singleton slices, and large slices, or is uniform random sufficient?
