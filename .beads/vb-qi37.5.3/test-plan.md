# Test Plan: vb-qi37.5.3 — Carry idempotency evidence into RunAdmission

## Summary

| Category | Count |
|----------|-------|
| Behaviors identified | 9 |
| Trophy allocation | 4 unit / 2 integration / 0 e2e / 1 static |
| Proptest invariants | 2 |
| Fuzz targets | 0 |
| Kani harnesses | 1 (KANI-INV-05 unblocked) |
| Mutation checkpoints | 2 |
| Blocked by DEFERRED_GLOBAL | 5 vb_runtime cargo-test lanes, 4 formal lanes |

---

## 1. Behavior Inventory

### vb_storage — VerificationProof (idempotency fields on artifact)

1. **VerificationProof stores idempotency_keyed and idempotency_attested as Box<[ActionId]>**
2. **VerificationProof flags (durable, bounded, taint_safe, retry_safe, replayable) gate deterministic replay semantics (INV-05)**
3. **submit_artifact produces AcceptedArtifact with populated VerificationProof under all three policies (Relaxed, Journaled, Strict)**
4. **submit_artifact validates checksum and structure before admission; rejects mismatches**
5. **admit_compiled_artifact is idempotent — same workflow returns same digest**

### vb_storage — VerificationWarning

6. **VerificationWarning.is_valid() returns true only for gate values 1-2 inclusive**
7. **VerificationWarning Display formats "gate {gate}: [{code}] {message}"**
8. **VerificationProof and AcceptedArtifact survive postcard round-trip serialization**

### vb_storage — Flag Conditions (INV-05)

9. **VerificationProof flag condition (durable && bounded && taint_safe && retry_safe && replayable) correctly gates idempotency replay semantics for all 32 boolean combinations**

---

## 2. Trophy Allocation

| Layer | Count | Rationale |
|-------|-------|-----------|
| Unit / Calc | 4 | Pure domain types: VerificationWarning bounds, serde roundtrips, flag condition logic |
| Integration | 2 | Storage-backed flows: submit_artifact with real FjallJournal, admit_compiled_artifact roundtrip |
| E2E | 0 | No user-facing CLI/API surface changed in this bead |
| Static | 1 | KANI-INV-05 harness compiles cleanly; no clippy issues on changed code |

**vb_storage dominates**: All testable behavior lives in vb_storage which compiles cleanly. vb_runtime tests blocked by DEFERRED_GLOBAL (missing chunk_001.rs at commit ffbe7f5cd).

---

## 3. BDD Scenarios

### Behavior 1: VerificationProof idempotency fields exist and are properly typed

**Subject**: VerificationProof stores idempotency evidence

```
Given: a VerificationProof constructed with idempotency_keyed=[ActionId(1), ActionId(2)]
       and idempotency_attested=[ActionId(3)]
When:  the fields are read back
Then:  idempotency_keyed.len() == 2
And:   idempotency_attested.len() == 1
And:   the ActionId values match exactly

fn verification_proof_idempotency_fields_populated_when_constructed()
```

**Error variant**:
```
Given: a VerificationProof constructed with empty slices
When:  the fields are read back
Then:  idempotency_keyed.len() == 0
And:   idempotency_attested.len() == 0

fn verification_proof_idempotency_fields_empty_when_no_actions()
```

### Behavior 2: VerificationProof flag conditions gate INV-05

**Subject**: All 32 flag combinations are handled correctly

```
Given: a VerificationProof with arbitrary flag combination
When:  all five flags are true (durable && bounded && taint_safe && retry_safe && replayable)
Then:  the artifact has deterministic replay semantics
And:   idempotency_keyed actions are safe to replay

fn verification_proof_all_flags_true_enables_deterministic_replay()
```

```
Given: a VerificationProof with any flag false
When:  the flags are checked
Then:  the artifact does NOT claim full deterministic replay guarantees

fn verification_proof_any_flag_false_disables_full_replay_guarantees()
```

### Behavior 3: submit_artifact policies produce correct VerificationProof

```
Given: a valid CompiledWorkflow with self-consistent BLAKE3 digest
When:  submit_artifact is called with RuntimePolicy::Relaxed
Then:  gate_count == 0
And:   durable == false
And:   idempotency_keyed and idempotency_attested are empty slices

fn submit_artifact_relaxed_skips_gates_and_sets_no_flags()
```

```
Given: a valid CompiledWorkflow
When:  submit_artifact is called with RuntimePolicy::Journaled
Then:  gate_count == 2
And:   durable == false
And:   bounded == true && taint_safe == true && retry_safe == true && replayable == true

fn submit_artifact_journaled_passes_two_gates_not_durable()
```

```
Given: a valid CompiledWorkflow
When:  submit_artifact is called with RuntimePolicy::Strict
Then:  gate_count == 2
And:   durable == true

fn submit_artifact_strict_passes_two_gates_and_is_durable()
```

### Behavior 4: checksum gate rejects spoofed digests

```
Given: a structurally valid CompiledWorkflow with a mismatched digest (content doesn't match claimed digest)
When:  submit_artifact is called with RuntimePolicy::Strict
Then:  Err(ArtifactChecksumMismatch) is returned

fn submit_artifact_rejects_checksum_mismatch_when_digest_spoofed()
```

### Behavior 5: admit_compiled_artifact is idempotent

```
Given: a valid CompiledWorkflow
When:  admit_compiled_artifact is called twice with the same workflow
Then:  both calls return the same WorkflowDigest

fn admit_compiled_artifact_returns_same_digest_on_repeated_calls()
```

### Behavior 6: VerificationWarning bounds

```
Given: VerificationWarning with gate values [0, 3, 4, 14, 255]
When:  is_valid() is called on each
Then:  gate=0 returns false
And:   gate=3 returns false
And:   gate=4 returns false
And:   gate=14 returns false
And:   gate=255 returns false

fn verification_warning_is_valid_returns_false_outside_gate_range()
```

```
Given: VerificationWarning with gate values [1, 2]
When:  is_valid() is called on each
Then:  both return true

fn verification_warning_is_valid_returns_true_for_gates_one_and_two()
```

### Behavior 7: Serde roundtrips

```
Given: a VerificationProof with non-empty idempotency fields and warnings
When:  serialized with postcard and deserialized
Then:  the deserialized proof equals the original exactly

fn verification_proof_serde_roundtrip_preserves_all_fields()
```

```
Given: an AcceptedArtifact with VerificationProof
When:  serialized with postcard and deserialized
Then:  the deserialized artifact equals the original exactly

fn accepted_artifact_serde_roundtrip_preserves_all_fields()
```

### Behavior 8: INV-05 Kani formal proof

```
Given: all 32 combinations of VerificationProof boolean flags
When:  Kani evaluates the flag condition logic
Then:  no assertion failures occur
And:   Kani proves that when all_flags_true, idempotency_keyed has deterministic replay semantics

fn kani_verification_proof_flags_all_combinations_pass()
```

---

## 4. Proptest Invariants

### Proptest 1: VerificationProof flag condition (INV-05)

**Function**: INV-05 condition `durable && bounded && taint_safe && retry_safe && replayable`

**Invariant**: When all five flags are true, the artifact is safe for deterministic replay. When any flag is false, the safety claim is weakened proportionally. The flag condition correctly gates idempotency semantics.

**Strategy**: `any::<(bool, bool, bool, bool, bool)>()`

**Anti-invariant**: A proof where all flags are true but the condition does not hold — Kani covers this exhaustively.

### Proptest 2: Idempotency field bounds

**Function**: `VerificationProof.idempotency_keyed.len()` and `idempotency_attested.len()`

**Invariant**: Both lengths are always >= 0 and <= some reasonable bound (10000) regardless of how the proof was constructed.

**Strategy**: `any::<(Vec<ActionId>, Vec<ActionId>)>().prop_map(|...|)` — construct VerificationProof with arbitrary action vectors.

**Anti-invariant**: Negative lengths are impossible due to Rust type system (usize).

---

## 5. Fuzz Targets

**None identified**: All parsing boundaries in vb_storage (codec, postcard) are pre-existing and already covered by existing tests. This bead only adds fields to an existing struct; no new parsing surface is introduced.

---

## 6. Kani Harnesses

### Kani Harness: verification_proof_flags_harness (KANI-INV-05)

**Status**: COMPILE-PASS — unblocked, ready to run

**Property**: For all 32 boolean combinations of (durable, bounded, taint_safe, retry_safe, replayable), when all are true, `proof.durable && proof.bounded && proof.taint_safe && proof.retry_safe && proof.replayable` holds.

**Bound**: 5 boolean variables = 32 combinations — exhaustively checked

**Command**: `cargo kani --harness verification_proof_flags_harness --workspace crates/vb_storage`

**Second harness**: `verification_proof_idempotency_fields_harness` additionally checks field length bounds under all flag conditions.

**Rationale**: INV-05 is a critical invariant. Property testing cannot exhaust all 32 combinations reliably. Kani provides mathematical certainty within the bounded model.

---

## 7. Mutation Checkpoints

### Checkpoint 1: submit_artifact gate count

**Critical mutation**: Changing `gate_count` assignment in `submit_artifact` from 0 (Relaxed) or 2 (Journaled/Strict) to a wrong value must be caught by existing gate-count assertions.

**Test**: `submit_artifact_relaxed_skips_gates_and_sets_no_flags`, `submit_artifact_journaled_passes_two_gates_not_durable`, `submit_artifact_strict_passes_two_gates_and_is_durable`

**Threshold**: 90% mutation kill rate

### Checkpoint 2: checksum validation

**Critical mutation**: Removing or short-circuiting the BLAKE3 checksum comparison in `submit_artifact` or `admit_compiled_artifact` must be caught by checksum-mismatch tests.

**Test**: `submit_artifact_rejects_checksum_mismatch_when_digest_spoofed`, `submit_artifact_checksum_mismatch_rejected`

**Threshold**: 90% mutation kill rate

---

## 8. Combinatorial Coverage Matrix

### vb_storage::admission::VerificationProof

| Scenario | Input | Expected Output | Layer |
|----------|-------|-----------------|-------|
| empty idempotency fields | VerificationProof::new | len() == 0 for both | unit |
| populated idempotency fields | Box::new([ActionId(1), ActionId(2)]) | len() == 2 | unit |
| all 32 flag combos | bool × 5 | correct flag gate behavior | kani |
| serde roundtrip | any VerificationProof | original == deserialized | unit |
| is_valid: gate 0 | gate=0 | false | unit |
| is_valid: gate 1 | gate=1 | true | unit |
| is_valid: gate 2 | gate=2 | true | unit |
| is_valid: gate 3+ | gate=3 | false | unit |
| Display format | VerificationWarning | "gate N: [code] message" | unit |

### vb_storage::admission::submit_artifact

| Scenario | Input | Expected Output | Layer |
|----------|-------|-----------------|-------|
| Relaxed policy | valid workflow | gate_count=0, durable=false | integration |
| Journaled policy | valid workflow | gate_count=2, durable=false | integration |
| Strict policy | valid workflow | gate_count=2, durable=true | integration |
| checksum mismatch | spoofed digest | Err(ArtifactChecksumMismatch) | integration |
| stale digest replay | same digest different content | Err(ArtifactChecksumMismatch) | integration |
| duplicate submit | same workflow twice | both succeed, second replaces | integration |
| roundtrip | Journaled artifact | stored == retrieved | integration |

### vb_storage::admission::admit_compiled_artifact

| Scenario | Input | Expected Output | Layer |
|----------|-------|-----------------|-------|
| valid workflow | minimal workflow | Ok(digest) | integration |
| idempotent | same workflow twice | same digest both times | unit |
| checksum mismatch | spoofed | Err(ArtifactChecksumMismatch) | unit |

---

## 9. Blocked Lanes (DEFERRED_GLOBAL)

vb_runtime cannot compile due to missing `chunk_001.rs` (pre-existing at commit ffbe7f5cd). The following lanes are blocked until DEFERRED_GLOBAL is resolved:

| Obligation | Layer | Blocker |
|------------|-------|---------|
| TEST-POST-03 (existing RunAdmission fields regression) | cargo-test | vb_runtime won't compile |
| TEST-POST-04 (caller sites provide idempotency evidence) | cargo-test | vb_runtime won't compile |
| TEST-ERR-01 (error propagation) | cargo-test | vb_runtime won't compile |
| TEST-POST-05 (IdempotencyTracker unit tests) | cargo-test | vb_runtime won't compile |
| PROPTEST-POST-01 (RunAdmission field propagation) | proptest | vb_runtime won't compile |
| PROPTEST-INV-03 (IdempotencyTracker capacity) | proptest | vb_runtime won't compile |
| MIRI-INV-04 (IdempotencyTracker UB check) | miri | vb_runtime won't compile |
| MIRI-POST-06 (Box<[ActionId]> slice copy UB) | miri | vb_runtime won't compile |
| LOOM-INV-04 (Send+Sync thread-safety) | loom | vb_runtime won't compile |
| KANI-POST-05 (load_accepted_artifact) | kani | vb_runtime won't compile |

**DEFERRED_GLOBAL-01 waiver**: Valid and documented in proof-review.md. These lanes are outside this bead's scope and do not prevent test-plan finalization.

---

## 10. Open Questions

1. **vb_runtime RunAdmission structure**: The exact field layout of `RunAdmission` (idempotency_keyed, idempotency_attested) in vb_runtime is defined in contract.md but not yet visible in source because vb_runtime won't compile. Test scenarios for TEST-POST-03/04 assume the contract-specified structure.

2. **IdempotencyTracker implementation**: The HashMap-based IdempotencyTracker in vb_runtime cannot be unit-tested until vb_runtime compiles. PROPTEST-INV-03 and TEST-POST-05 are deferred.

3. **KANI-INV-05 execution**: The harness compiles cleanly. Need to run `cargo kani --harness verification_proof_flags_harness --workspace crates/vb_storage` to get actual formal evidence.

4. **vb_storage proptest coverage**: No proptest currently exists for VerificationProof idempotency field propagation. The proptests.rs covers key encoding and journal events but not the admission module's new idempotency fields. A proptest for POST-01 should be added when vb_runtime is unblocked.

---

## 11. Execution Commands

### Immediately executable (vb_storage only):

```bash
# Unit tests for vb_storage admission module
cargo test -p vb_storage admission -- --nocapture

# Unit tests for VerificationWarning bounds
cargo test -p vb_storage verification_warning -- --nocapture

# Unit tests for submit_artifact
cargo test -p vb_storage submit_artifact -- --nocapture

# Unit tests for serde roundtrips
cargo test -p vb_storage serde_roundtrip -- --nocapture

# Proptests for vb_storage key encoding (existing, unrelated to this bead)
cargo test -p vb_storage proptests -- --nocapture

# Kani formal verification (KANI-INV-05) — compiles unblocked
cargo kani --harness verification_proof_flags_harness --workspace crates/vb_storage

# Clippy static analysis on changed code
cargo clippy -p vb_storage -- -D warnings
```

### Blocked until DEFERRED_GLOBAL resolved (vb_runtime):

```bash
# All vb_runtime tests blocked
cargo test -p vb_runtime admit_run -- --nocapture
cargo test -p vb_runtime idempotency -- --nocapture
cargo test -p vb_runtime run_admission -- --nocapture

# Proptest blocked
cargo test -p vb_runtime run_admission_idempotency_proptest -- --nocapture
cargo test -p vb_runtime idempotency_tracker_capacity_proptest -- --nocapture

# Miri blocked
MIRIFLAGS="-Zmiri-strict-provenance" cargo miri test -p vb_runtime idempotency -- --nocapture

# Loom blocked
cargo loom test -p vb_runtime idempotency --persist
```
