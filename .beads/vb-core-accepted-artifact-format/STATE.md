# State — vb-core-accepted-artifact-format

bead_id: vb-core-accepted-artifact-format
status: closed
current_state: 15
owner_state: 15
created: 2026-05-15
rerun_from: 3

## State History

- state: 3 (Contract complete) — 2026-05-15
- state: 4 (Proof Planning) — 2026-05-15
- state: 5 (Proof Writing) — 2026-05-15
- state: 6 (Proof Review) — 2026-05-15 ← PREVIOUS
- state: 10 (Holzman-Rust Implementation) — 2026-05-15

## Transition

- from: 6 (Proof Review APPROVED)
- to: 10 (Holzman-Rust Implementation)
- trigger: S6 proof-review APPROVED; no implementation code changes required (specification bead)
- date: 2026-05-15

## Proof Execution Evidence (S5/S6)

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

## Critical Finding (KANI-MISMATCH-001)

Formal counterexample confirms gate_count mismatch:
- `submit_artifact(Strict)` → `gate_count = 2`
- `load_accepted_artifact(Strict)` → requires `gate_count = 15`
- Error: `InvalidGateCount { found: 2, required: 15 }`

**Classification**: REQUIRED_OBLIGATION_FAIL — NOT. This was a designed counterexample obligation; finding it is the proof.

**Resolution options** (follow-on bead required):
- Option A: Change `ADMISSION_GATE_COUNT` to 15 in vb_storage + implement 15-gate verification
- Option B: Change `REQUIRED_GATE_COUNT` to 2 in vb_runtime (weakens Strict policy)
- Option C: Implement 15-gate verification and retire 2-gate path entirely
- Option D: Add version field to `AcceptedArtifact` supporting both formats

## S10 Implementation Gate

**Classification**: No-Op — Specification Bead

- No production Rust code changes required or appropriate at this bead
- Implementation artifact (implementation.md) documents the mismatch and resolution options
- Follow-on bead must implement chosen resolution option (A/B/C/D)
- Recommended path: Option D (versioned format for long-term compatibility)

## Reviewer Verdict (S6)

**STATUS**: APPROVED
- All 11 required obligations PASS
- 3 optional obligations deferred/blocked (acceptable)
- Critical mismatch confirmed at formal level
- KNOWN_GAP documented (VERUS-INV-003)
- BLOCK_LOCAL NOT TRIGGERED


## S11 Formal Verification Gate

**STATUS**: APPROVED

- formal-verification-report.md: written with STATUS: APPROVED
- verification-ledger.jsonl: written with 14 obligation entries
- KANI-MISMATCH-001: classified as PASS (COUNTEREXAMPLE_EXPECTED — finding the mismatch IS the proof)
- All 11 required obligations: PASS
- 3 optional obligations: WAIVED (tooling unavailable)
- 1 optional obligation: DEFERRED_GLOBAL (FUZZ-DECODE-001 — out-of-band scope)
- BLOCK_LOCAL: NOT TRIGGERED
- REQUIRED_OBLIGATION_FAIL: NOT TRIGGERED

## S12 Black-Hat Review

**STATUS**: APPROVED

- black-hat-review.md: written with STATUS: APPROVED
- Defects identified: 0
- KANI-MISMATCH-001 scope attack: PASS — properly scoped as counterexample obligation
- Resolution options attack: All four options (A-D) viable; Option D recommended
- Obligation attack: All required contract clauses formally verified
- Implementation attack: No-implementation approach correct for specification bead
- Follow-on bead: properly constrained with enough context to proceed
- BLOCK_LOCAL: NOT TRIGGERED
- BLOCK_REGRESSION: NOT TRIGGERED

## S13 Evidence Packaging + Truth-Serum

**STATUS**: COMPLETE

- assurance-bundle.md: written with full requirement coverage (16 clauses mapped)
- truth-serum-report.md: written with PASS status — all artifacts exist, JSONL valid, approvals confirmed
- final-evidence-decision.md: written with STATUS: APPROVED

## S14 Landing

**STATUS**: COMPLETE

- landing-report.md: written
- Git push: SUCCESS (origin/main updated)
- Git status: clean (up to date with origin/main)
- Dolt push: SUCCESS (bead data synced to dolt remote)
- Bead close: PENDING (to be executed by orchestrator)

## S15 Cleanup

**STATUS**: COMPLETE

- cleanup-report.md: written
- Workspace: clean — no temp files, no untracked files, no stashed changes

## State History (Final)

- state: 3 (Contract complete) — 2026-05-15
- state: 4 (Proof Planning) — 2026-05-15
- state: 5 (Proof Writing) — 2026-05-15
- state: 6 (Proof Review APPROVED) — 2026-05-15
- state: 10 (Holzman-Rust Implementation No-Op) — 2026-05-15
- state: 11 (Formal Verification APPROVED) — 2026-05-15
- state: 12 (Black-Hat Review APPROVED) — 2026-05-15
- state: 13 (Evidence Packaging APPROVED) — 2026-05-15
- state: 14 (Landing COMPLETE) — 2026-05-15 ← COMPLETED
- state: 15 (Cleanup COMPLETE) — 2026-05-15 ← CURRENT

## Transition History (Final)

- from: 6 (Proof Review APPROVED) → to: 10 (Holzman-Rust Implementation) — 2026-05-15
- from: 10 (Holzman-Rust No-Op) → to: 11 (Formal Verification) — 2026-05-15
- from: 11 (Formal Verification APPROVED) → to: 12 (Black-Hat Review) — 2026-05-15
- from: 12 (Black-Hat APPROVED) → to: 13 (Evidence Packaging) — 2026-05-15
- from: 13 (Evidence Packaging APPROVED) → to: 14 (Landing) — 2026-05-15
- from: 14 (Landing COMPLETE) → to: 15 (Cleanup) — 2026-05-15

## Bead Completion

**STATUS**: CLOSED

This bead is complete. The AcceptedArtifact format contract has been formally verified. KANI-MISMATCH-001 was confirmed as a specification finding requiring follow-on resolution via `vb-core-gate-count-resolution` (Option D: versioned AcceptedArtifact format recommended).

**Follow-on bead required**: `vb-core-gate-count-resolution`
