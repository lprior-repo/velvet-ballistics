# Test Plan Review: vb-99n6

## STATUS: APPROVED WITH COMMENTS

---

## 1. Acceptance Criterion Coverage

| Contract AC | Test(s) | Coverage |
|---|---|---|
| AT-1 Timer fire advances wait | IT-TIMER-001 | ✓ |
| AT-2 Ask answer cleans timer | IT-TIMER-002 | ✓ |
| AT-3 Cancel cleans timer | IT-CANCEL-001 | ✓ |
| AT-4 Resume re-drives action-suspended | IT-RESUME-001 | ✓ |
| AT-5 Resume re-drives without consuming timer | IT-RESUME-002 | ✓ |
| AT-6 TimerFired on unknown run | IT-TIMER-004 | ✓ |
| AT-7 TimerFired with no pending timer | IT-TIMER-005 | ✓ |
| AT-8 TimerFired after cancel | IT-TIMER-006 | ✓ |
| AT-9 TimerFired after ask answer (stale) | IT-TIMER-003 | ✓ |
| AT-10 Resume on unknown run | IT-RESUME-004 | ✓ |
| AT-11 Cancel on non-existent run silent | IT-CANCEL-002 | ✓ |
| AT-12 Duplicate cancel idempotent | IT-CANCEL-003 | ✓ |
| AT-13 TimerFired after finish | IT-TIMER-007 | ✓ |
| AT-14/15/16 Timer wheel | TW-UT-005,003,002 | ✓ |

All 16 acceptance tests have corresponding test cases.

---

## 2. Invariant Coverage

| Invariant | Test(s) | Status |
|---|---|---|
| I-1: At most one timer per run | PB-SM-001, TW-UT-002 | ✓ |
| I-2: Timer implies run exists | PB-SM-002 | ✓ |
| I-3: Timer kind matches suspension | PB-SM-003 | ✓ |
| I-4: Ask timer eventual completion | Covered by IT-TIMER-002, IT-TIMER-003, BDD-001 | ✓ |
| I-5: pending_timers.len() <= runs.len() | PB-GLOBAL-001 | ✓ |
| I-6: Cancel idempotent | IT-CANCEL-003, PB-GLOBAL-002 | ✓ |
| I-7: RunNotFound vs InvalidTimerFire | PB-GLOBAL-003 | ✓ |
| I-8: Timer removed after handle_timer | IT-TIMER-001 | ✓ (implicit) |
| I-9: Run+timer removed after cancel | IT-CANCEL-001 | ✓ |
| I-10: finish_run removes timer | TR-UT-004 | ✓ |
| I-11: fail_run_state removes timer | TR-UT-005 | ✓ |

All 11 invariants have test coverage.

---

## 3. Edge Case Coverage

| Edge Case (contract 5.x) | Test(s) | Status |
|---|---|---|
| 5.1 Resume While Timer Pending | IT-RESUME-002, BDD-003 | ✓ |
| 5.2 Timer Fire Race (AskAnswer) | IT-TIMER-003, BDD-001 | ✓ |
| 5.3 Cancel Then Timer Fire | IT-TIMER-006, BDD-002 | ✓ |
| 5.4 Resume After Timer Fire | BDD-004 | ✓ |
| 5.5 Timer Fire After Finish | IT-TIMER-007 | ✓ |
| 5.6 Last-Wins Replacement | TW-UT-002, BDD-005 | ✓ |

All 6 edge cases covered.

---

## 4. Error Taxonomy Coverage

Contract section 4 defines 10 error variants. The test plan's Error Path Matrix (section 7) covers all variants except:
- `ShutdownInProgress` — covered by BDD-006 but **not in contract error taxonomy** (BDD-006 is a behavioral edge case not listed in section 5)
- `QueueFull`, `ActiveRunCapacityExceeded`, `RunAlreadyExists`, `StaleAttempt`, `AttemptBeyondMax`, `InvalidActionCompletion` — not covered in this test plan, but these relate to submit/action completion not timer wheel/resume/cancel

This is acceptable — the test plan scope is "timer wheel, resume, cancellation, hardening" per contract section 1.

---

## 5. Gaps / Observations

1. **BDD-006 (Shutdown blocks resume)** — BDD-006 tests `ShutdownInProgress` error but the contract does not define this error variant for `handle_resume`. Contract error taxonomy has `ShutdownInProgress` but describes it as "drain_for_shutdown called on non-empty queue." The test expectation is reasonable and aligns with the ubiquitous precondition (shard.shutting_down == false), but the error variant assignment is inferred.

2. **IT-RESUME-003 (Resume past deadline does NOT auto-fire)** — This integration test validates a critical behavioral constraint from contract section 2.1 postcondition: "If the run was suspended on AwaitingWait but the deadline has already passed: handle_resume MUST NOT auto-fire the timer." No explicit AT covers this. The integration test fills the gap.

3. **Await timer preconditions (contract 2.4)** — Contract section 2.4 defines preconditions for `await_timer` but unit tests TR-UT-001/002/003 validate behavior rather than explicitly testing preconditions. This is acceptable since the behavior tests implicitly validate the preconditions.

---

## 6. Testing Trophy Distribution

| Layer | Planned | Appropriate |
|---|---|---|
| Unit tests | ~42 | ✓ TimerWheel dual-index ops well isolated |
| Integration tests | ~18 | ✓ Command × scenario matrix |
| Property-based | ~12 | ✓ Invariant preservation |
| BDD scenarios | ~8 | ✓ Cross-component edge cases |

Distribution is appropriate for the component type (isolated timer logic + stateful command handlers).

---

## 7. Summary

The test plan provides comprehensive coverage of:
- All 16 acceptance tests from contract section 6
- All 11 invariants from contract sections 3.1–3.2
- All 6 behavioral edge cases from contract section 5
- All error variants in scope (timer/resume/cancel)

The two observations above are minor inference gaps that do not constitute coverage failures. The test plan is sound.
