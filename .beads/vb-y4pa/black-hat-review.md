# Black-Hat Review: vb-y4pa (State 12 — re-review after conditional fix)

## Reviewer: black-hat-reviewer (attempt 2)

## Summary

Re-review after `jump_to_body` conditional fix. The unconditional `mark_pending` that was breaking Waiting/Asking states has been replaced with a guard.

## Contract Compliance

| Contract Item | Implementation | Status |
|---|---|---|
| Succeeded→Pending conditional reset | `if current == StepState::Succeeded { mark_pending }` at helpers.rs:65 | ✓ |
| Waiting preserved (no reset) | Guard skips mark_pending for Waiting | ✓ |
| Asking preserved (no reset) | Guard skips mark_pending for Asking | ✓ |
| 6 primitives wired to jump_to_body | for_each.rs:84, reduce.rs:82, collect.rs:397, collect.rs:521, repeat.rs:88, repeat.rs:115 | ✓ |
| Succeeded→Pending transition valid | step_state.rs allows it | ✓ |

## Formal Verification Evidence

- `cargo build --workspace`: PASS (4 crates)
- `cargo nextest -p vb_runtime`: 1651/1651 PASS

## Findings

No defects. The conditional guard correctly implements the BodyReentryPrecondition from contract.md: only Succeeded triggers reset; Waiting/Asking pass through unchanged.

## Verdict

**STATUS: APPROVED**
