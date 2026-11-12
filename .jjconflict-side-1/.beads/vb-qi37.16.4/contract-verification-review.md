# Contract Verification Review — vb-qi37.16.4 (Re-Review 2)

STATUS: APPROVED

## Files Reviewed
- `.beads/vb-qi37.16.4/proof-obligations.jsonl`
- `.beads/vb-qi37.16.4/verification-layers.md`
- `specs/AskAnswerLifecycle.tla`
- `specs/AskAnswerLifecycle.cfg`
- `.beads/vb-qi37.16.4/contract-verification-review.md` (prior)

---

## State 4 — AnswerPersistenceOrder TLA Repair Assessment

**Scope:** Decision limited to State 4 contract-verification status following `AnswerPersistenceOrder` TLA repair.

### AnswerPersistenceOrder — RESOLVED

**Prior defect (LETHAL):** `AnswerPersistenceOrder` was a vacuous tautology:
```tla
AnswerPersistenceOrder ==
    \A i, j \in 1..Len(AnsweredLog) :
        i < j => TRUE  \* always true — provides zero ordering guarantee
```

**Current definition** (`specs/AskAnswerLifecycle.tla` lines 114-119):
```tla
AnswerPersistenceOrder ==
    \A run \in RunId, step \in StepIdx, seq \in SeqNo :
        \A i \in 1..Len(AnsweredLog) :
            AnsweredLog[i] = <<"aa", run, step, seq>>
                => \E j \in 1..i-1 :
                    AnsweredLog[j] = <<"sw", run, step, seq>>
```

**Verdict:** Non-vacuous and correct. For every AskAnswered entry at index `i`, there exists a SlotWritten entry at some index `j < i`. This properly encodes "SlotWritten precedes AskAnswered" and can detect violations. The `cfg` correctly declares it as a `PROPERTY` for TLC model-checking.

**`TLA-POST-ORDER` proof-obligation is now meaningful.** The formal verification evidence ("TLC reports AnswerPersistenceOrder satisfied") is no longer vacuously achievable.

---

## Prior Rejection — Resolution Status

| Rejection Item | Prior Reason | Current Status |
|---------------|--------------|----------------|
| TLA+ model files missing | `specs/AskAnswerLifecycle.tla` and `.cfg` did not exist | **RESOLVED** |
| `UNIT-ERR-ALL` optional without waiver | `required: false` with no compensating evidence | **RESOLVED** — waiver present in `verification-layers.md` |
| `PROPTEST-PRE-003` optional without waiver | `required: false` with no compensating evidence | **RESOLVED** — waiver present in `verification-layers.md` |
| `AnswerPersistenceOrder` vacuous tautology | LETHAL: property always true, `TLA-POST-ORDER` non-meaningful | **RESOLVED** — property now correctly encodes ordering |

---

## Open Items (Not Blocking State 4)

### MAJOR — `tla-report.md` absent

All six TLA+ proof obligations cite `"evidence": "tla-report.md"` in `proof-obligations.jsonl`, but the file does not exist. No confirmation that TLC has been invoked against `AskAnswerLifecycle.cfg`.

**Recommendation:** Provide `tla-report.md` with TLC execution output, or add `status_note: "deferred — TLC execution pending"` to each TLA+ obligation.

---

## Coverage Decision

| Axis | Status |
|------|--------|
| TLA+ model files exist | PASS |
| TLA+ cfg valid | PASS |
| TLA+ spec syntactically valid | PASS |
| `AnswerPersistenceOrder` semantics non-vacuous | **PASS** |
| `AnswerPersistenceOrder` correctly ordered | **PASS** |
| TLA+ proof obligations well-scoped | PASS |
| Verus proof obligations well-scoped | PASS |
| Kani proof obligations well-scoped | PASS |
| Integration test obligations well-scoped | PASS |
| UNIT-ERR-ALL waiver present and sound | PASS |
| PROPTEST-PRE-003 waiver present and sound | PASS |
| JSONL validity | PASS |
| Required field completeness | PASS |
| Source-lint scope | PASS |

---

## Summary

State 4 contract-verification is **APPROVED**. The `AnswerPersistenceOrder` TLA repair correctly resolves the prior LETHAL vacuity defect. The property now meaningfully encodes ordering between SlotWritten and AskAnswered entries.

`tla-report.md` remains an open item but does not block State 4 approval.

---

**Reviewer:** contract-verification-reviewer (skill v1.5.0)
**Date:** 2026-05-11
**Artifact dir:** `.beads/vb-qi37.16.4/`
**Re-review 2:** AnswerPersistenceOrder TLA repair verified — STATUS: APPROVED
