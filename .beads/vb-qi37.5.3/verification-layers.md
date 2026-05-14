# Verification Layers — vb-qi37.5.3

## Boundary

- **Verus-owned kernel**: RunAdmission construction invariants, IdempotencyTracker capacity bound, field-copy correctness
- **TLA+ temporal model**: None — this is a data-flow/type-propagation change, no temporal behavior introduced
- **Theorem projection**: None required
- **Runtime shell**: I/O (file system, network), async scheduling, FFI surfaces are not involved in this change
- **External systems**: None — vb_storage and vb_core are first-party crates

---

## Layer Assignment

| Contract Clause | Verification Layer(s) | Notes |
|-----------------|----------------------|-------|
| PRE-01 | proptest + kani | artifact envelope validation at admission |
| PRE-02 | kani | StorageArtifactStore::load_accepted_artifact bounded check |
| PRE-03 | verus + proptest | non-null field existence |
| POST-01 | verus + proptest | field propagation from VerificationProof to RunAdmission |
| POST-02 | verus | Box<[ActionId]> type match |
| POST-03 | cargo test | regression on existing fields |
| POST-04 | proptest + cargo test | caller sites updated |
| POST-05 | cargo test + miri | IdempotencyTracker tracking operations |
| POST-06 | miri + cargo test | no panics in admission path |
| INV-01 | verus | field-length equality at construction |
| INV-02 | verus | field-length equality at construction |
| INV-03 | verus + proptest | capacity bound, eviction |
| INV-04 | miri + loom | Send+Sync thread-safety for IdempotencyTracker |
| INV-05 | kani | bounded model check on proof flag conditions |
| ERR-01 | cargo test | error path coverage |

---

## Verus Scope

### Target: `vb_runtime::admission::RunAdmission::new`

- **Spec function**: `spec_new` capturing field-copy semantics
- **Invariant**: `idempotency_keyed.len() == proof.idempotency_keyed.len()`
- **Trusted boundary**: Validated `AcceptedArtifact` and `VerificationProof` from storage
- **Shell exclusions**: I/O, async scheduling, storage reads are handled at the `admit_artifact_run` layer before `new` is called

### Target: `vb_runtime::idempotency::IdempotencyTracker`

- **Invariant**: `completed.len() <= DEFAULT_CAPACITY` after every `track_for_policy` call
- **Proof function**: `proof_capacity_invariant` with decreases clause
- **Trusted boundary**: Internal HashMap operations
- **Shell exclusions**: Thread-safety of concurrent access is proven by loom/miri, not Verus

### Target: `vb_runtime::admission::admit_artifact_run`

- **Spec**: extracts `idempotency_keyed` and `idempotency_attested` from `AcceptedArtifact.verification`
- **Postcondition**: returned `RunAdmission` contains the same slice lengths as the proof
- **Shell exclusions**: Storage load, envelope validation

---

## Miri Scope

### Target: `IdempotencyTracker` HashMap operations

- **Purpose**: Detect UB on all supported concurrent access patterns
- **Command**: `MIRIFLAGS="-Zmiri-strict-provenance" cargo miri test idempotency`
- **Expected evidence**: No UB reports, no data races, no use-after-free
- **Scope**: `track_for_policy`, `is_completed_for_policy`, construction

### Target: RunAdmission field propagation

- **Purpose**: Ensure no UB when copying `Box<[ActionId]>` slices
- **Command**: `cargo miri test run_admission`
- **Expected evidence**: No UB on field copy

---

## Loom Scope

### Target: `IdempotencyTracker` thread-safety

- **Purpose**: Permutation testing for concurrent HashMap access
- **Command**: `cargo loom test idempotency --persist`
- **Expected evidence**: No data races detected in loom's interleaving model
- **Scope**: `track_for_policy` + `is_completed_for_policy` from multiple simulated threads

### Scope Limitation

- Loom tests are permutationally complete for a bounded number of threads (default: 2-4)
- Does not exhaustively prove absence of data races for all thread counts
- Compensating evidence: miri for UB, cargo test for unit coverage

---

## Kani Scope

### Target: `StorageArtifactStore::load_accepted_artifact`

- **Purpose**: Bounded model checking on the artifact loading path
- **Command**: `cargo kani --harness load_accepted_artifact_harness`
- **Expected evidence**: No panics, no assertion violations, no index out-of-bounds
- **Scope**: Artifact loading, proof extraction, error propagation

### Target: VerificationProof flag conditions

- **Purpose**: Check that INV-05 holds for all flag combinations
- **Command**: `cargo kani --harness verification_proof_flags_harness`
- **Expected evidence**: Kani proves flag conditions correctly gate idempotency semantics

---

## Proptest Scope

### Target: RunAdmission field propagation

- **Purpose**: Property-based test for idempotency field propagation
- **Command**: `cargo test run_admission_idempotency_proptest`
- **Strategy**: `any::<(Vec<ActionId>, Vec<ActionId>)>().prop_map(|...)|`
- **Expected evidence**: 1000 iterations pass without failure

### Target: IdempotencyTracker capacity

- **Purpose**: Eviction correctness on overflow
- **Command**: `cargo test idempotency_tracker_capacity_proptest`
- **Expected evidence**: Capacity never exceeds DEFAULT_CAPACITY after eviction

---

## Cargo Test Scope

- Unit tests for `admit_run`, `admit_artifact_run`, `admit_run_with_budget`
- Regression tests ensuring existing fields unchanged
- Error path tests for `ArtifactEnvelopeError` and `AdmissionError`
- `IdempotencyTracker` unit tests for `track_for_policy` and `is_completed_for_policy`

---

## Waiver: TLA+

- **Reason**: This bead adds data fields to an existing struct and copies them through an existing admission function. No new temporal behavior, workflow state machine, scheduler, queue, retry loop, claim/lease, lifecycle transition, distributed coordination, eventuality, liveness, fairness, or deadlock condition is introduced.
- **Owner**: State 3 (rust-contract)
- **Expiry**: This waiver applies only to vb-qi37.5.3; future beads that introduce temporal behavior must use TLA+
- **Compensating evidence**: verus for construction invariants, kani for load path, proptest for field propagation, miri/loom for concurrency

---

## Waiver: Lean/Aeneas/Hax

- **Reason**: The proof obligations are expressible entirely in Verus spec/proof functions and do not require algebraic extraction to a theorem prover. No parser grammar, codec, protocol lattice, or arithmetic bound theorem exceeds Verus's expressiveness for this domain.
- **Owner**: State 3 (rust-contract)
- **Compensating evidence**: Verus for construction invariants and capacity bound

---

## Pre-existing DEFERRED_GLOBAL

- `vb_runtime` does not compile due to missing `chunk_001.rs`
- Formal verification (miri, loom, kani) on `vb_runtime` cannot execute until this is resolved
- Implementation can proceed in parallel; the contract specifies the correct behavior for when the build is fixed
- **Owner**: External/DEFERRED_GLOBAL
- **Tracking**: Pre-existing at commit ffbe7f5cd
