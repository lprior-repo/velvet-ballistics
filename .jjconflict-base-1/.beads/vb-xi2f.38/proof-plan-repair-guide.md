# Proof Plan Repair Guide: vb-xi2f.38

## Status: REJECTED — PROCESS DEFECT + 3 RESERVED OBLIGATIONS

---

## CRITICAL: Process Remediation Required FIRST

The proof plan is **technically sound** but **processially invalid**. Fix the process FIRST,
then address the 3 reserved obligations.

### Step 0: Generate Independent Review (BLOCKING)

**Problem**: No independent `proof-plan-reviewer` invocation exists.
`agent-invocation-ledger.jsonl` has only `vb-xi2f.38-invoke-1` (go-skill).
The planner wrote its own review.

**Required Action for Femdation Controller**:
1. Dispatch a NEW `proof-plan-reviewer` agent invocation (not the same session as planner)
2. The new invocation must produce its OWN `proof-plan-review.md` and
   `verifier-lane-review.jsonl` with:
   - `reviewer_invocation_id` set to the new invocation ID (e.g., `vb-xi2f.38-ppr-002`)
   - `planner_invocation_id` set to `vb-xi2f.38-invoke-1`
   - `review_date` set to the new review date
3. Do NOT reuse the planner's self-stamped artifacts

**Minimum valid state for re-run**:
- New `proof-plan-review.md` with `STATUS: APPROVED` from independent reviewer
- New `verifier-lane-review.jsonl` with 26 rows, each having valid `planner_invocation_id`
  and `reviewer_invocation_id` from separate agent sessions

---

## Reserved Obligations Requiring Remediation

### R1: PO-011 (Verus: CC-DIGEST-004) — CRITICAL SOURCE REF FIX

**Problem**: `proof-seeds.jsonl ps-011` and existing review reference
`part_03.rs:159-212` for `lower_canonical_collect`. But `digest_step_primitive`
is at `part_05.rs:140-161` AND `compile/mod.rs:243-261`. The Verus proof targets
the wrong location.

**Required Actions**:
1. Verify where `lower_canonical_collect` actually lives (likely part_03)
2. Verify where the Collect match arm in `digest_step_primitive` lives (part_05 and compile/mod — BOTH locations have the same bug)
3. If they are different functions, PO-011 must be split:
   - PO-011a: Verus proof for `lower_canonical_collect` (part_03)
   - PO-011b: Kani/Proptest for `digest_step_primitive` Collect match arm (part_05 + compile/mod)
4. If they are the same function, correct the source reference in ps-011

**Fix in proof-seeds.jsonl**:
```json
// Before (WRONG):
"model_boundary": "lower_canonical_collect (part_03.rs lines 159-212)"

// After (verify and correct):
"model_boundary": "lower_canonical_collect (PART_03_VERIFY: confirm location), digest_step_primitive Collect match arm (part_05.rs:140-161 AND compile/mod.rs:243-261)"
```

---

### R2: PO-019 (H-6: Pagination State) — INSUFFICIENT VERIFIER

**Problem**: PO-019 uses proptest to verify runtime Fjall state invariants
(cursor <= limit, page_size constant). Proptest generates test vectors — it cannot
formally prove runtime state invariants. Fjall ACID is in trusted base (assumed).

**Required Actions** (choose one):

**Option A — Downgrade to behavioral test**:
- Change PO-019 mode from `property-test` to `behavioral-test`
- Update expected_evidence to: "Proptest generates test vectors; runtime invariant
  guaranteed by Fjall ACID (trusted base T7)"
- Add explicit scope limitation to evidence description

**Option B — Promote to integration test**:
- Create integration test that:
  1. Runs actual Collect pagination with Fjall storage
  2. Inspects CollectPaginationState after each page fetch
  3. Verifies cursor <= limit AND page_size unchanged
- Update PO-019 artifact and command to integration test path

**Option C — Remove from formal obligations**:
- Remove PO-019 from proof-obligations.planned.jsonl
- Acknowledge pagination state is covered by integration tests and runtime QA,
  not formal proof in this bead

---

### R3: PO-012b (CC-DIGEST-005: Storage Admission) — UNVERIFIED TEST SCOPE

**Problem**: PO-012b references `vb_core_atomic_admission_red.rs:856` as evidence
but no verification that this test actually:
1. Computes `compute_compiled_digest` on raw artifact bytes
2. Injects a digest mismatch
3. Verifies `ArtifactDigestMismatch` error (fail-closed)

**Required Actions**:
1. Locate and read the actual integration test at the referenced path
2. Confirm it covers all 3 steps above
3. If not, create new integration test:
```rust
#[test]
fn artifact_digest_mismatch_rejected() {
    // 1. Create valid WorkflowSource
    // 2. Serialize to artifact bytes
    // 3. Corrupt the bytes (change one byte in body)
    // 4. Call storage admission with corrupted bytes
    // 5. Verify ArtifactDigestMismatch error returned (not accepted)
}
```
4. Update PO-012b expected_evidence to be precise

---

## Post-Process-Remediation Checklist

After the independent review invocation completes:

- [ ] New `proof-plan-review.md` with `STATUS: APPROVED` from independent reviewer
- [ ] New `verifier-lane-review.jsonl` with 26 rows, valid invocation IDs
- [ ] PO-011 source reference corrected and verified
- [ ] PO-019 either downgraded, promoted to integration test, or removed
- [ ] PO-012b test scope verified or new test created
- [ ] All 22 obligations have schema-version-compliant JSON with no legacy alias fields
- [ ] Two-location bug fix planned: part_05.rs AND compile/mod.rs both need Collect match arm

---

## Summary of Required State Transitions

| Item | Current State | Required State |
|------|--------------|----------------|
| Review process | Planner self-stamped | Independent proof-plan-reviewer invocation |
| verifier-lane-review.jsonl | No invocation IDs | 26 rows with planner/reviewer IDs |
| proof-plan-review.md | Self-stamped APPROVED | Independent APPROVED |
| PO-011 source ref | part_03 (unverified) | Correct file verified |
| PO-019 verifier | proptest (insufficient) | Downgrade/promote/remove |
| PO-012b test scope | Unverified | Verified or new test |
