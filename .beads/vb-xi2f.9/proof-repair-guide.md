# Proof Repair Guide: vb-xi2f.9 (R4 — RETRY-2 REJECTION)

**Bead:** vb-xi2f.9
**Review:** pr-vb-xi2f.9-004 (REJECTED)
**Schema:** proof-repair-guide/v1
**Date:** 2026-05-26

## Current State

12 of 21 obligations APPROVED, 1 WAIVED, 2 BLOCKED, 4 REJECTED.

6 of 9 prior rejection findings (PF-R2-001, PF-R2-002, PF-R2-006, PF-R2-007, PF-R2-009) resolved. 3 findings (PF-R2-004, PF-R2-005, PF-R2-008) + 1 (PF-R2-003 partial) unresolved.

## Blockers (P0 — MUST FIX)

### 1. Fix moon-ci failures (F-R4-001, F-R4-007)

**Problem A:** Unused import `CompileError` in proptest_ast_marks.rs:18
**Fix:** 
```bash
# Edit crates/vb_compile/tests/proptest_ast_marks.rs, line 18:
# Change: use vb_compile::{CompileError, YamlCompiler};
# To:     use vb_compile::YamlCompiler;
```

**Problem B:** WeakenedAssertion in phase1_core_types.rs
**Fix:** Restore the removed assertion in `crates/vb_core/tests/phase1_core_types.rs`, OR file a bead-linked waiver. The test-integrity gate reports `removed_exact=1 added_exact=0 added_weak=0`.

**Verify:**
```bash
moon ci 2>&1 | tee .evidence/vb-xi2f.9/logs/moon-ci-v2.log
# Must show: Tasks: N completed, 0 failed, M skipped
```

### 2. Fix cargo test --workspace (F-R4-002)

**Problem:** 151 errors, 6 warnings. Primary error at `crates/vb_validate/tests/capability_schema_kani.rs:306`

**Fix:**
Open `crates/vb_validate/tests/capability_schema_kani.rs` and move the `use vb_core::span::Span;` at line 306 to file-level scope (before any function/module definition). This is a misplaced `use` statement inside a function body where it's not valid.

Fix all remaining compilation errors in the 5 affected crates.

**Verify:**
```bash
cargo test --workspace 2>&1 | tee .evidence/vb-xi2f.9/logs/cargo-test-workspace-v2.log
# Must show: test result: ok. N passed; 0 failed
```

### 3. Capture PO-K02 individual Kani evidence (F-R4-003, F-R4-006)

**Problem:** Both evidence logs (po-k02-nev.log 4.6MB, po-k02-nev-v2.log 5.1MB) have 0 VERIFICATION SUCCESSFUL markers — all 7 harnesses timed out in batch. Proof-writer-report claims individual `nev_len_ge_one` succeeded but no evidence file exists.

**Fix A — Run individual harnesses with captured evidence:**
```bash
# Run each harness individually with bounded state
cargo kani -p vb_core --harness nev_len_ge_one 2>&1 | tee .evidence/vb-xi2f.9/kani/po-k02-nev-individual.log
cargo kani -p vb_core --harness nev_from_vec_empty 2>&1 | tee -a .evidence/vb-xi2f.9/kani/po-k02-nev-individual.log
cargo kani -p vb_core --harness nev_is_empty_false 2>&1 | tee -a .evidence/vb-xi2f.9/kani/po-k02-nev-individual.log
# Continue for remaining harnesses...
```

**Fix B — If individual harnesses also time out, formally waive:**
Update `proof-obligations.planned.jsonl` PO-K02:
```json
{
  "...": "...",
  "status": "waived",
  "waiver": {
    "reason": "Kani state-space explosion on Vec<T> with generic T via Arbitrary. Proptest PO-P02 (8/8 PASS) provides primary non-vacuous coverage of NonEmptyVec invariants including round-trip preservation, from_vec(empty)==None, with_tail count, and is_empty() always false.",
    "compensating_evidence": "PO-P02",
    "expiry": "bead-landing"
  }
}
```

Update `proof-evidence.md` PO-K02 section accordingly.

### 4. Disposition trusted-base ledger (F-R4-008)

**Problem:** All 47 entries have `reviewer_disposition: "pending"`. Minimum required: TB-039 through TB-042.

```bash
# For each entry in trusted-base-ledger.jsonl, set reviewer_disposition:
# - "accepted" — assumption is valid and well-bounded
# - "rejected" — assumption is invalid or over-broad  
# - "waived" — acceptable with documented rationale
# - "blocked" — requires implementation (include tracking bead ref)

# Critical entries that MUST be resolved:
# TB-039: proof-writer implementation discovery → "accepted" (documented gaps)
# TB-040: span bridge absent → "blocked" (PO-K07 implemented bridge, update status)
# TB-041: ValidationError span absent → "blocked" (tracking: implement span fields)
# TB-042: extract_span absent → "blocked" (tracking: implement CanonicalYaml mark)
# TB-043: AstMarks pub(super) → "accepted" (proptest PO-P06 works through public API)
```

### 5. Document PO-K05 and PO-K06 blocked status (F-R4-004, F-R4-005)

Update `proof-obligations.planned.jsonl`:

```json
# PO-K05: change status to "blocked"
{"...": "...", "status": "blocked", "blocker": "Contract C5.2: CompileError::CanonicalYaml requires mark: SourceMark field which does not exist. Implementation bead: vb-xi2f.36 or equivalent."}

# PO-K06: change status to "blocked"  
{"...": "...", "status": "blocked", "blocker": "Contract C6.1: Most ValidationError variants require span: Span field which does not exist. Only DuplicateKey has span. Implementation bead: vb-xi2f.36 or equivalent."}
```

## High Priority (P1 — SHOULD FIX)

### 6. Qualify PO-K08 non-vacuity (F-R4-009)

Update `proof-evidence.md` PO-K08 section to add:
> **Non-vacuity scope:** Kani harnesses (0 `kani::any()` calls) verify the deterministic empty-AstMarks subdomain — all lookup methods return None, never panic, and are deterministic. Populated-AstMarks coverage is provided by proptest PO-P06 (7/7 PASS) which exercises realistic YAML with known source locations.

### 7. Run remaining PO-K05 harnesses (F-R4-010)

```bash
# Run harnesses individually from kani_canonical_yaml_enrich.rs
# (only those not blocked by missing mark: SourceMark field)
cargo kani -p vb_compile --harness yaml_error_category_exhaustive 2>&1 | tee .evidence/vb-xi2f.9/kani/po-k05-individual.log
# Continue for other harnesses as implementation allows
```

Update `proof-writer-report.md` PO-K05 status from "VERIFIED (2/2)" to accurate count.

## Recommended (P2 — CAN DEFER)

### 8. Add agent invocation ledger entries (A-R4-001)

Add entries to `agent-invocation-ledger.jsonl`:
- proof-plan-reviewer (ppr-vb-xi2f.9-001 state 4, ppr-vb-xi2f.9-002 state 4)
- proof-writer (pw-vb-xi2f.9-001 state 5, pw-vb-xi2f.9-002 state 5 REPAIR-2)
- proof-reviewer (pr-vb-xi2f.9-001, pr-vb-xi2f.9-002, pr-vb-xi2f.9-003, pr-vb-xi2f.9-004)

### 9. Reconcile YamlError variant count (A-R4-003)

Update `contract.md` C4.1 to reflect actual variant count, or add rationale for counting methodology (e.g., "19 error variants + limit-exceeded variants excluded from count").

## Re-review Checklist

Before re-submitting to proof-reviewer:
- [ ] moon ci passes (exit 0, 0 failures)
- [ ] cargo test --workspace passes (exit 0, 0 failures)
- [ ] PO-K02 individual Kani evidence captured OR formally waived
- [ ] 47 trusted-base entries dispositioned
- [ ] PO-K05 and PO-K06 marked BLOCKED in obligations
- [ ] PO-K08 non-vacuity qualified in proof-evidence.md
- [ ] All evidence files captured to .evidence/vb-xi2f.9/
- [ ] proof-evidence.md updated with accurate obligation statuses
- [ ] proof-writer-report.md updated with accurate harness counts
