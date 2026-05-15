# Proof Plan Review Input: vb-qi37.4.2

## Bead
**vb-qi37.4.2** — runtime: Enforce admission gate before run creation
**State**: 4 (Proof Planning)
**Review Trigger**: Contract clauses INV-001, INV-002, POST-002 require verification

---

## What Must Be Proven

### INV-001: Run is NOT inserted when admission fails
- **Clause**: For Strict/Journaled policy, a run is NEVER inserted into `self.runs` unless `build_admission` returned `Ok`
- **Mechanism**: `?` operator at line 86 propagates `Err` → lines 87–125 skipped
- **Evidence required**: `active_run_count() == 0`, `runs_submitted == 0` after rejected submit

### INV-002: Admission gate sequencing
- **Clause**: `build_admission` (line 86) → `take_frame_for` (line 87) → `RunSubmitted` journaled (lines 91–100) → `RunAdmission` journaled (lines 102–111) → `runs.insert` (line 125)
- **Mechanism**: Rust sequential evaluation + `?` early return
- **Evidence required**: Source code inspection confirms ordering; integration tests confirm rejection skips all steps

### POST-002: No side effects on rejection
- **Clause**: On rejection: no frame allocated, no journal events written, no run state inserted, `active_run_count` unchanged
- **Evidence required**: `runs_submitted == 0` after rejected submit

### ERR-Rejection: Error taxonomy exhaustive
- **Clause**: `ArtifactNotFound` → `AdmissionArtifactNotFound`, `CapabilityDenied` → `AdmissionCapabilityDenied`, `ArtifactInvalid` → `AdmissionArtifactInvalid`
- **Evidence required**: Integration tests confirm correct error variants returned

---

## What is NOT Proven (Waived)

| Clause | Waiver | Reason |
|--------|--------|--------|
| INV-002 (temporal aspects) | WAIVER-TLA-001 | Single atomic step function; no branching, concurrency, or state-over-time behavior. Linear Rust control flow, not a protocol. |
| INV-001 (formal proof) | WAIVER-VERUS-001 | Deterministic `?` propagation; verifiable by inspection + integration tests. No ghost state or loop invariants required. |

---

## Key Finding: Missing Type

**`NeverPresentArtifactStore`** (implements `AcceptedArtifactStore`) does NOT exist at module level.

Existing `NeverPresentStore` in `admission.rs` tests (lines 859, 884, 901) only implements `ArtifactStore`, NOT `AcceptedArtifactStore`. The shard's `handle_submit_with_inputs_contracts_and_header_mode` calls `build_admission` which calls `admit_artifact_run` which requires `AcceptedArtifactStore`.

**Required**: Create `NeverPresentArtifactStore` in `admission.rs` that always returns `ArtifactNotFound` from `load_accepted_artifact`.

---

## Test Gap

**Existing test** (`chunk_003.rs:53` `admission_rejection_does_not_insert_run_state`):
- Uses `Shard::new(small_config())` where `small_config().policy == Relaxed`
- Asserts `active_run_count() == 1` and `runs_submitted == 1`
- **Confirms run IS inserted** — does NOT test rejection

**Required new tests**:
| Test | Policy | Store | Assertions |
|------|--------|-------|------------|
| `admission_rejection_does_not_insert_run_state_strict` | Strict | NeverPresentArtifactStore | `active_run_count() == 0`, `runs_submitted == 0` |
| `admission_rejection_does_not_insert_run_state_journaled` | Journaled | NeverPresentArtifactStore | `active_run_count() == 0`, `runs_submitted == 0` |
| `admission_capability_mismatch_does_not_insert` | Strict | AlwaysPresentArtifactStore | `active_run_count() == 0`, `AdmissionCapabilityDenied` |
| `admission_rejection_no_counter_increment` | Strict | NeverPresentArtifactStore | `runs_submitted == 0` |

---

## Reviewer Questions

1. **Is TLA+ waiver justified?** INV-002 is a control-flow ordering property, not a temporal protocol. Agree?

2. **Is Verus waiver justified?** The `?` propagation is deterministic Rust control flow. Agree integration tests are sufficient?

3. **Is `NeverPresentArtifactStore` creation in scope?** The contract defines it but it doesn't exist. This is a required artifact for the tests. Agree it should be created as part of this bead's implementation?

4. **Is the integration test approach correct?** Using `Shard::new_with_journal_and_artifact_store(strict_config(), NoopRuntimeJournal::shared(), NeverPresentArtifactStore::shared())` to construct shards for rejection tests.

---

## Verification Commands

```bash
# Compile
cargo build -p vb_runtime

# Clippy
cargo clippy -p vb_runtime --lib --bins -- -D warnings

# Unit tests (admission layer)
cargo test -p vb_runtime admission_admit_run_strict_without_artifact_rejected -- --nocapture
cargo test -p vb_runtime admission_admit_run_journaled_without_artifact_rejected -- --nocapture

# Integration tests (lifecycle layer)
cargo test -p vb_runtime admission_rejection_does_not_insert_run_state_strict -- --nocapture
cargo test -p vb_runtime admission_rejection_does_not_insert_run_state_journaled -- --nocapture
cargo test -p vb_runtime admission_capability_mismatch_does_not_insert -- --nocapture
cargo test -p vb_runtime admission_rejection_no_counter_increment -- --nocapture

# Miri UB check
MIRIENV='-Zmiri-strict-provenance=y' cargo miri test -p vb_runtime admission_rejection_does_not_insert_run_state_strict -- --nocapture
```
