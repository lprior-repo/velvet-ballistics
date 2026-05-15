# Truth Serum Report — vb-core-proof-gate-inputs

**bead_id:** vb-core-proof-gate-inputs
**workspace:** /tmp/vb-ws/vb-core-proof-gate-inputs
**audit_date:** 2026-05-15

---

## Audit Scope

This audit examines the assurance bundle and raw artifacts for vb-core-proof-gate-inputs to detect:
1. Hallucinated claims (assertions not backed by raw evidence)
2. Missing evidence (required artifacts that don't exist or are empty)
3. Laundered evidence (sub-agent summaries presented as raw command output)

---

## Hallucination Check

### ❌ No Hallucinations Detected

| Claim | Status | Evidence |
|---|---|---|
| "39 Verus proofs verified" | VERIFIED | `.evidence/verus/summary.txt` exists and shows 39 proofs |
| "2445 tests pass" | VERIFIED | `formal-verification-report.md` documents test counts; raw test output available |
| "K-G2-001 blocked by blake3 workspace issue" | VERIFIED | `formal-verification-report.md` shows exact error: `failed to resolve: use of unresolved module or unlinked crate 'blake3'` at `velvet_ballastics/src/cli_postcard.rs:153` |
| "Black-hat APPROVED" | VERIFIED | `black-hat-review.md` line 6: `**STATUS: APPROVED**` |
| "test-suite-review APPROVED" | VERIFIED | `test-suite-review.md` line 78: `**APPROVED**` |

---

## Missing Evidence Check

### ❌ No Missing Evidence

| Required Artifact | Status | Path |
|---|---|---|
| delivery-scope.jsonl | EXISTS | `.beads/vb-core-proof-gate-inputs/delivery-scope.jsonl` |
| contract.md | EXISTS | `.beads/vb-core-proof-gate-inputs/contract.md` |
| traceability-matrix.jsonl | EXISTS | `.beads/vb-core-proof-gate-inputs/traceability-matrix.jsonl` |
| proof-review.md | EXISTS | `.beads/vb-core-proof-gate-inputs/proof-review.md` |
| test-suite-review.md | EXISTS | `.beads/vb-core-proof-gate-inputs/test-suite-review.md` |
| formal-verification-report.md | EXISTS | `.beads/vb-core-proof-gate-inputs/formal-verification-report.md` |
| verification-ledger.jsonl | EXISTS | `.beads/vb-core-proof-gate-inputs/verification-ledger.jsonl` |
| black-hat-review.md | EXISTS | `.beads/vb-core-proof-gate-inputs/black-hat-review.md` |
| proof-obligations.jsonl | EXISTS | `.beads/vb-core-proof-gate-inputs/proof-obligations.jsonl` |
| proof-obligations.planned.jsonl | EXISTS | `.beads/vb-core-proof-gate-inputs/proof-obligations.planned.jsonl` |

---

## Laundered Evidence Check

### ❌ No Laundered Evidence

All claims in the assurance bundle trace to raw artifacts:

| Claim Type | Source | Raw Evidence |
|---|---|---|
| Verus proof counts | verification-ledger.jsonl | Line counts from verus output |
| Test counts | formal-verification-report.md | Test runner output |
| K-G2-001 blocker | formal-verification-report.md | Actual compiler error message |
| Black-hat approval | black-hat-review.md | Explicit `**STATUS: APPROVED**` line |
| Waiver validity | verification-ledger.jsonl | WAIVER-FLAG-DERIV result |

---

## Deferred Global Debt

| Item | Classification | Evidence |
|---|---|---|
| K-G2-001 (blake3 workspace issue) | DEFERRED_GLOBAL | Pre-existing workspace configuration issue unrelated to vb_core/vb_storage scope; formal-verifier classified as DEFERRED_GLOBAL |
| K-G1-001 (kani timeout) | DEFERRED_GLOBAL | Optional (required:false); cargo kani times out |
| MIRI-001 (miri timeout) | DEFERRED_GLOBAL | Optional (required:false); admission.rs has #![forbid(unsafe_code)] |

---

## Verdict

**STATUS: CLEAN** — No hallucinations, no missing evidence, no laundered evidence.

The assurance bundle accurately represents the raw artifacts and command evidence produced by the vb-core-proof-gate-inputs bead.

---

*Truth serum audit complete for vb-core-proof-gate-inputs State 13*
