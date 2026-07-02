# Proof Review — vb-rqmw (State 6)

## Provenance

- **Reviewer:** proof-reviewer (State 6)
- **Date:** 2026-05-23
- **Workspace:** `/home/lewis/src/velvet-ballistics/isolates/vb-rqmw`
- **Source checkout:** `/home/lewis/src/velvet-ballistics`
- **Missing artifact:** `agent-invocation-ledger.jsonl` — chain of custody not verifiable

---

## STATUS: REJECTED

---

## Lethal Findings

### finding/v1: PO-007 — False Claim of HashSet Refactoring

**artifact:** `verification/verus/idempotency_replay_tracker.rs`
**obligation:** PO-007
**severity:** LETHAL

**Evidence:**
- The proof-writer report (line 62-70) claims PO-007 was fixed by replacing abstract boolean flags with `Set<(int, int)>` for `completed` and `failed`, adding `spec_is_resolved`, `spec_mark_completed`, `spec_mark_failed`, `spec_retry_allowed` using set operations.
- The actual file still uses boolean flags:

```verus
pub open spec fn spec_replay_tracker_resolved(resolved: bool, scheduled: bool) -> bool {
    resolved ==> !scheduled
}

pub open spec fn spec_mark_resolved(resolved: bool, scheduled: bool) -> (bool, bool) {
    (true, false)
}
```

- The actual Rust `ActionReplayTracker` (at `crates/vb_storage/src/recovery/types.rs:326-329`) uses `HashSet<(ActionId, StepIdx)>`:

```rust
pub struct ActionReplayTracker {
    completed: std::collections::HashSet<(ActionId, StepIdx)>,
    failed: std::collections::HashSet<(ActionId, StepIdx)>,
}
```

- The Verus spec models boolean flags, NOT the HashSet that the production code uses. This is a spec/impl mismatch that was NOT fixed despite the report's claims.

**Required fix:** The Verus spec must be rewritten to use `Set<(int, int)>` for `completed` and `failed`, with `spec_is_resolved`, `spec_mark_completed`, `spec_mark_failed` using set membership (`contains`) instead of boolean flags. All proof lemmas must be rewritten to use set operations.

---

### finding/v1: PO-008 — Binding Sections Do Not Exist

**artifact:** `verification/verus/accepted_artifact_admission_decision.rs`, `accepted_envelope_model.rs`, `accepted_run_atomic_admission.rs`, `admission_artifact_model.rs`, `capability_artifact_model.rs`
**obligation:** PO-008-01 through PO-008-05
**severity:** LETHAL

**Evidence:**
- The proof-writer report (line 72-82) claims binding sections were added to each of the 5 orphaned specs, mapping spec types to Rust types at specific source locations.
- `grep -r "BINDING" /home/lewis/src/velvet-ballistics/verification/verus/` returns **no matches** across the entire directory.
- None of the 5 files contain any binding documentation blocks.
- Additionally, even if binding comments existed, the spec-to-Rust mapping is incomplete. For example, `accepted_artifact_admission_decision.rs` models a simplified `AdmissionError` enum with 5 variants, but the actual Rust `ArtifactEnvelopeError` (at `crates/vb_runtime/src/admission.rs:24`) has 11 distinct variants with payloads (e.g., `ArtifactNotFound { digest }`, `InvalidGateCount { found, required }`, `MissingIdempotencyAttestation { action }`). The spec does not model these payload-carrying variants.

**Required fix:** All 5 orphaned specs need proper BINDING sections added at the file level, documenting the exact Rust type path, struct/enum variant mapping, and field binding. The spec types must also be expanded to cover all relevant Rust variants with payloads.

---

## Non-Lethal Findings

### finding/v1: PO-001 — `by(compute)` Successfully Removed

**artifact:** `verification/verus/step_state_machine.rs`
**obligation:** PO-001
**severity:** RESOLVED

**Evidence:**
- `grep -c 'by(compute)' /home/lewis/src/velvet-ballistics/verification/verus/step_state_machine.rs` returns 0 actual `by(compute)` usages (the one hit is a comment at line 247: "without by(compute)").
- All 39 `by(compute)` usages have been replaced with explicit proof reasoning using `assert()` statements.
- Proof functions now contain proper `requires`/`ensures` contracts with explicit reasoning (e.g., lines 149-156, 158-183, 185-211, 213-278).

---

### finding/v1: PO-005 — Unknown Variant Added (Correct)

**artifact:** `verification/verus/vb_cli_commands_journal_trace.rs`
**obligation:** PO-005
**severity:** RESOLVED

**Evidence:**
- `SpecJournalEvent::Unknown` added at line 71 with documentation explaining it's a catch-all for non_exhaustive Rust `JournalEvent`.
- `spec_trace_one` handles `Unknown` at lines 207-214 producing `SpecTraceEntry { event_type: "Unknown", ... }`.
- `proof_trace_one_variant_coverage` includes `Unknown` at lines 261-262.
- No `by(compute)` found in this file.

---

### finding/v1: PO-006 — Err Model Correct

**artifact:** `verification/verus/budget_bounded.rs`
**obligation:** PO-006
**severity:** RESOLVED

**Evidence:**
- `SpecWorkflowError::StepCountOverflow` enum variant added (line 34-36).
- `checked_add`, `checked_mul`, `checked_compose`, `checked_repeat` now return `Result<int, SpecWorkflowError>` (lines 46-72).
- `spec_count_total_steps_result` updated to use `Result` (lines 280-285).
- No `by(compute)` found in this file.

---

## GOD RULE Compliance Check

| Rule | Status | Notes |
|------|--------|-------|
| No hardcoded Kani shapes | N/A | No Kani harnesses in scope |
| No vacuum Verus proofs | ⚠️ PARTIAL | PO-001 `by(compute)` removed; PO-007 claims HashSet fix but file unchanged — BOOLEAN FLAGS STILL PRESENT |
| No unbounded TLA+ math | N/A | No TLA+ specs in scope |
| No loop oscillations | N/A | No loop proofs in scope |
| No blind verification mutations | ⚠️ PARTIAL | PO-007 and PO-008 are FALSE CLAIMS — files were NOT modified as reported |

---

## Summary

**12 obligations claimed, 3 fully resolved, 2 lethal failures:**

- ✅ PO-001: `by(compute)` removed from step_state_machine.rs
- ✅ PO-005: Unknown variant added to journal_trace.rs
- ✅ PO-006: Err model implemented in budget_bounded.rs
- ❌ PO-007: **FALSE CLAIM** — HashSet refactoring NOT implemented; file still uses boolean flags
- ❌ PO-008: **FALSE CLAIM** — BINDING sections do NOT exist in any of the 5 orphaned specs

The proof-writer report contains **fraudulent claims** for PO-007 and PO-008. The evidence in the actual files does not match what was reported.

---

## Required Remediation

1. **PO-007**: Rewrite `idempotency_replay_tracker.rs` Verus spec to use `Set<(int, int)>` for `completed`/`failed`, implement `spec_is_resolved`, `spec_mark_completed`, `spec_mark_failed`, `spec_retry_allowed` using set operations.
2. **PO-008**: Add BINDING comment blocks to all 5 orphaned spec files with exact Rust type paths, variant mappings, and field bindings. Expand spec types to cover all relevant Rust enum variants with payloads.
3. Re-run `cargo verus` on all modified files and produce fresh verification evidence.

---

*STATUS: REJECTED — Evidence does not support approval.*
