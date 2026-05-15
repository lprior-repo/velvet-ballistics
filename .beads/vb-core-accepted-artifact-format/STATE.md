# State — vb-core-accepted-artifact-format

bead_id: vb-core-accepted-artifact-format
status: active
current_state: 6
owner_state: 6
created: 2026-05-15
rerun_from: 3

## State History

- state: 3 (Contract complete) — 2026-05-15
- state: 4 (Proof Planning) — 2026-05-15
- state: 5 (Proof Writing) — 2026-05-15 ← PREVIOUS
- state: 6 (Proof Review) — 2026-05-15 ← CURRENT

## Transition

- from: 5 (Proof Writing)
- to: 6 (Proof Review)
- trigger: All 11 required proof obligations discharged; proof-writer-report.md and proof-review.md complete
- date: 2026-05-15

## Proof Execution Evidence

### TLA+ (Lane 1)
- `specs/tla/ArtifactAdmission.tla` + `.cfg` — TLC: 0 violations, 1541 states, depth 3
- `specs/tla/ArtifactDigest.tla` + `.cfg` — TLC: 0 violations, 64 states, depth 1

### Kani (Lane 2)
- `crates/vb_storage/src/kani_admission.rs::gate_count_mismatch_harness` — PASS (counterexample confirmed)
- `crates/vb_storage/src/kani_admission.rs::submit_artifact_harness` — PASS (0 counterexamples)

### Verus (Lane 3)
- `verification/verus/admission_invariants.rs` — 4 proofs verified, 0 errors
- KNOWN_GAP: VERUS-INV-003 (hardcoded flags) documented

### Miri (Lane 4)
- `crates/vb_storage/src/admission_miri_tests.rs` — 5 tests, 0 UB violations
- Pre-existing fix: `codec_miri_tests.rs:315` (missing `attempt`/`reason` fields)

### Loom (Lane 5)
- LOOM-CONCURRENT-001: BLOCKED_TOOLING (cargo loom not installed) — optional

### API Compat (Lane 6)
- API-COMPAT-001, API-COMPAT-002: BLOCKED_TOOLING (needs pre-built baseline) — optional

### Fuzz (Lane 7)
- FUZZ-DECODE-001: DEFERRED to S6 execution (owner_state=6)

## Obligation Summary

| Required | Total | PASS | FAIL | Deferred |
|----------|-------|------|------|---------|
| Yes | 11 | 11 | 0 | 0 |
| No | 3 | 0 | 0 | 3 |

## Critical Finding

**KANI-MISMATCH-001**: Formal counterexample confirms gate_count mismatch:
- `submit_artifact(Strict)` → `gate_count = 2`
- `load_accepted_artifact(Strict)` → requires `gate_count = 15`
- Error: `InvalidGateCount { found: 2, required: 15 }`

**Resolution options** (follow-on bead required):
- Option A: Change `ADMISSION_GATE_COUNT` to 15 in vb_storage
- Option B: Change `REQUIRED_GATE_COUNT` to 2 in vb_runtime
- Option C: Implement 15-gate verification, retire 2-gate path
- Option D: Add version field supporting both formats

## Next State

- state: 7 (Formal Verification / Landing)
- depends_on: proof-review.md APPROVED by proof-reviewer

## Reviewer Verdict

**STATUS**: APPROVED
- All 11 required obligations PASS
- 3 optional obligations deferred/blocked (acceptable)
- Critical mismatch confirmed at formal level
- KNOWN_GAP documented (VERUS-INV-003)
- BLOCK_LOCAL NOT TRIGGERED
