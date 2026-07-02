# Final Evidence Decision: vb-xi2f.29

**Packager**: evidence-packaging agent (p14)
**Date**: 2026-05-25
**Workspace**: /home/lewis/src/vb-workspaces/vb-xi2f.29
**Prior gates**: Black-hat APPROVED WITH FIXES (MJ-1/MJ-2 fixed)

---

## STATUS: APPROVED

---

## Decision Rationale

### 1. Contract Coverage — PASS

All 8 contract clauses (C-01 through C-08) have non-vacuous evidence:

| Clause | Key Evidence | Non-Vacuous? |
|---|---|---|
| C-01 (canonical name == "together") | Kani: 0/432 failed, VERIFICATION SUCCESSFUL | YES — harness WOULD fail before line 105 fix |
| C-02 (branch count in digest) | Proptest: 6/6 PASS (was 1/6 before fix) | YES — 5 sensitivity tests FAILED before fix |
| C-03 (branch labels in digest) | Proptest: 6/6 PASS | YES |
| C-04 (sub-step contents hashed) | Proptest: 6/6 PASS + Unit: 67/67 PASS | YES |
| C-05 (branch ordering) | Proptest: 6/6 PASS | YES |
| C-06 (determinism preserved) | Proptest: 15/15 PASS | YES |
| C-07 (non-together regression) | Proptest: 15/15 PASS | YES |
| C-08 (Kani proof updated) | Kani: canonical_name_together_harness VERIFIED | YES |

### 2. Proof Obligations — 12 PASS, 3 BLOCKED (compensated), 1 DEFERRED, 1 MERGED

| Verdict | Count | Details |
|---|---|---|
| PASS | 12 | PO-001 through PO-007, PO-011 through PO-015 (PO-015 merged into PO-001) |
| BLOCKED (compensated) | 3 | PO-009, PO-010, PO-010b — blake3 InlineAsm. Full compensating proptest/unit evidence. |
| DEFERRED | 1 | PO-008b — Aggregate canonical name out of scope per contract non-goals. |
| MERGED | 1 | PO-015 — Kani PO-001 provides definitive C-01 evidence. |

**No unresolved FAIL_GLOBAL or BLOCK_GLOBAL evidence.**

### 3. Production Code — VERIFIED BY INDEPENDENT SOURCE INSPECTION

| Fix | Location | Status |
|---|---|---|
| `"parallel"` → `"together"` | `part_05.rs:105` | ✅ CONFIRMED |
| Together arm in `digest_step_primitive` | `part_05.rs:198-216` | ✅ CONFIRMED (branch count LE, labels, recursive sub-steps) |
| `digest_sub_step` function | `part_05.rs:225-232` | ✅ CONFIRMED |
| No regressions in other 11 variants | `part_05.rs:100-113` | ✅ CONFIRMED (only line 105 changed) |
| No unwrap/expect/panic/unsafe/dbg | Full file scan | ✅ CONFIRMED |
| Dead code not compiled | `compile/mod.rs` not in `lib.rs` | ✅ CONFIRMED |

### 4. Review Gates — ALL APPROVED

| Review | Artifact | Status |
|---|---|---|
| Proof Plan Review | proof-plan-review.md | APPROVED (ppr-vb-xi2f29-2026-05-24-001) |
| Proof Review (REPAIR-2) | proof-review.md | APPROVED (ppr-vb-xi2f29-2026-05-25-002) |
| Proof-to-Rust Bridge Review (RETRY) | proof-to-rust-review.md | APPROVED (ptr-vb-xi2f29-2026-05-25-002) |
| Black-Hat Review | External/owner-stated | APPROVED WITH FIXES (MJ-1/MJ-2 fixed) |
| Formal Verification Report | reports/formal-verification-report.md | PASS (12 PASS, 3 BLOCKED compensated, 1 DEFERRED) |

### 5. Artifact Gaps — NON-BLOCKING

| Gap | Compensating Evidence |
|---|---|
| test-plan-review.md MISSING | test-plan.md (520 lines, 18 behaviors) exists. Test evidence in proof-review.md. |
| black-hat-review.md MISSING from bead dir | Owner states APPROVED. Adversarial review in proof-review.md: APPROVED, 0 lethal findings. |
| machine-gate-report.md MISSING | formal-verification-report.md at reports/. verification-ledger.jsonl at root. |
| regression-diff.md MISSING | Production diff verified as minimal (1+10+4 lines, 3 visibility changes). |

### 6. Truth Serum Audit — PASS

Active-context truth-serum audit executed. All source claims independently verified. All file references resolved. All review status lines confirmed. No hallucinated evidence detected. Detailed findings in `truth-serum-report.md`.

---

## Disposition

**STATUS: APPROVED**

The evidence package meets the acceptance criteria for vb-xi2f.29. The core property — that `canonical_digest` correctly reflects Together semantics — is verified across Kani, proptest, and unit test lanes with non-vacuity proven by test trajectory (FAIL before fix → PASS after fix). All 8 contract clauses are covered. All 4 prior lethal proof-review findings (LF-001 through LF-004) are resolved. All BLOCKED obligations have compensating evidence. Non-lethal findings (NLF-004 through NLF-008) are for traceability and documentation clarity and do not block landing.

The 4 missing gate artifacts (test-plan-review.md, black-hat-review.md, machine-gate-report.md, regression-diff.md) from the bead directory are documented as non-blocking gaps with compensating evidence available through alternate channels. The owner's stated black-hat approval with MJ-1/MJ-2 fixes confirms the bead passed adversarial review.

**Ready for landing.**
