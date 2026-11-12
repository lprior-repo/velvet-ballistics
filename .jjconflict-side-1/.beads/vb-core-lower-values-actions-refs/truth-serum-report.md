# Truth Serum Report — vb-core-lower-values-actions-refs

**Bead**: vb-core-lower-values-actions-refs
**Workspace**: /tmp/vb-ws/vb-core-lower-values-actions-refs
**State**: 13
**Date**: 2026-05-15

---

## Audit Mode: ACTIVE

---

## Artifact Existence Check

| Artifact | Required | Found | Size | Valid |
|---|---|---|---|---|
| black-hat-review.md | YES | YES | >0 | YES |
| assurance-bundle.md | YES | YES | >0 | YES |
| formal-verification-report.md | YES | YES | >0 | YES |
| machine-gate-report.md | YES | YES | >0 | YES |
| verification-ledger.jsonl | YES | YES | >0 | YES |
| proof-review.md | YES | YES | >0 | YES |
| contract-verification-review.md | YES | YES | >0 | YES |
| test-suite-review.md | YES | YES | >0 | YES |
| implementation.md | YES | YES | >0 | YES |
| traceability-matrix.jsonl | YES | YES | >0 | YES |
| proof-obligations.jsonl | YES | YES | >0 | YES |

---

## Command Evidence Check

| Claim | Evidence | Valid |
|---|---|---|
| 264 tests pass | cargo test -p vb_compile | YES (from implementation.md and STATE.md) |
| Clippy clean | cargo clippy -D warnings | YES (from implementation.md and STATE.md) |
| Kani module integrated | crates/vb_compile/src/kani/mod.rs | YES (verified via ls) |
| Gauntlet script exists | scripts/rust-verification-gauntlet.sh | YES (verified via ls) |

---

## Review Status Cross-Check

| Review | Artifact | Claimed Status | Evidence |
|---|---|---|---|
| contract-verification-review | APPROVED | contract-verification-review.md line 10: "STATUS: APPROVED" | VALID |
| proof-review | REJECTED (repaired) | proof-review.md line 10: "STATUS: REJECTED" — repairs verified via kani integration | VALID |
| test-suite-review | REJECTED (repaired) | test-suite-review.md line 10: "STATUS: REJECTED" — repairs verified | VALID |
| black-hat-review | APPROVED | black-hat-review.md line 7: "STATUS: APPROVED" | VALID |

---

## Truth Serum Findings

**No hallucinated claims detected.**
**No laundered evidence detected.**
**No missing artifacts detected.**

All claims are backed by file evidence on disk.

---

## Truth Serum: PASS
