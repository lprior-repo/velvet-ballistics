# Final Evidence Decision — vb-qi37.1.4

## State: 13 (evidence-packaging + truth-serum)
## Date: 2026-05-14
## Bead: vb-qi37.1.4 — runtime/recovery: Fail closed on incomplete recovery

---

## STATUS: REJECTED

---

## Decision Summary

| Criterion | Status | Evidence |
|---|---|---|
| GAP-2 fix applied correctly | ✓ CONFIRMED | Code inspection: line 84 fixed |
| Verus proofs verified | ✓ PASS | 7 verified, 0 errors |
| JSONL artifacts valid | ✓ PASS | jq validation passed |
| Test suite runnable | ✗ BLOCKED | verus dependency not on crates.io |
| DEFECT-1 present | ✓ PRESENT | test-plan.md:73-80 expects wrong outcome |
| Truth serum audit | ⚠ UNVERIFIED | Tooling limitation |

---

## Evidence Summary

### Proof Layer
- **Verus**: 7 proofs verified, 0 errors (PASS)
- **TLC**: Not run (tooling issue)
- **Kani**: Not run (tooling issue)

### Test Layer
- **Unit tests**: FAIL_LOCAL (verus dependency blocks cargo)
- **Integration tests**: FAIL_LOCAL (verus dependency blocks cargo)
- **Clippy**: UNRUN (verus dependency blocks cargo)

### Review Layer
- **contract-verification-review**: APPROVED
- **proof-review**: APPROVED
- **test-plan-review**: APPROVED WITH MINOR FINDINGS
- **formal-verification-report**: REJECTED (GAP-2 bug — now fixed)
- **black-hat-review**: REJECTED (DEFECT-1)

---

## Blocker: DEFECT-1

**Test expects wrong behavior**: test-plan.md:73-80 `reject_returns_ok_when_pending_actions_unsupported_but_empty` expects `Ok(())` but POST-002 requires `Err(RuntimeError::InvalidRecoveryHydration)` when `unsupported.pending_actions=true` regardless of `pending_actions.is_empty()`.

**Impact**: After the GAP-2 fix (now applied), the correct behavior is `Err`. The test expects `Ok`. The test would FAIL when executable.

---

## Tooling Limitation

The `verus = "^1"` workspace dependency is not available on crates.io. This pre-existing environmental issue blocks all cargo-based verification commands.

**Compensating evidence**:
- 7 Verus proofs verified independently
- Code inspection confirms GAP-2 fix is correct
- JSONL artifacts are valid

---

## Required Actions to Unblock

1. Fix test `reject_returns_ok_when_pending_actions_unsupported_but_empty` to expect `Err(RuntimeError::InvalidRecoveryHydration)`
2. Update test-plan-review.md:37 to reflect corrected expectation

---

## Waiver: GAP-3 (Action ABI + Policy Digests)

- **Owner**: TBD
- **Expiry**: 2026-07-01
- **Reason**: Implementation deferred; formal waiver obtained
- **Compensating Evidence**: Verus spec documents deferred behavior

---

## Summary

The GAP-2 fix at line 84 of recovery.rs is verified correct by code inspection. Verus proofs pass. However:

1. **DEFECT-1** (test expects wrong behavior) blocks landing
2. **Tooling limitation** prevents command execution verification

**Verdict**: REJECTED — fix DEFECT-1 to unblock landing.

---

*final-evidence-decision.md: State 13 for vb-qi37.1.4 — STATUS: REJECTED*
