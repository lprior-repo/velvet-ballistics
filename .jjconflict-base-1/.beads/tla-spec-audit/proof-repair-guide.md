# TLA+ Spec Non-Vacuity Repair Guide

**Status**: REJECTED — Do not merge until all LETHAL findings are resolved.
**Reviewer**: proof-reviewer
**Bead**: tla-spec-audit

---

## Quick Reference: LETHAL Findings by Spec

| Spec | LETHAL Finding IDs |
|------|-------------------|
| AskAnswerLifecycle | /v1/ask-answer-lifecycle/001, /v1/ask-answer-lifecycle/002, /v1/ask-answer-lifecycle/003 |
| RetryFSM | /v1/retry-fsm/001, /v1/retry-fsm/002, /v1/retry-fsm/003 |
| RetryJournal | /v1/retry-journal/001, /v1/retry-journal/002, /v1/retry-journal/003 |
| LifecycleJournal | /v1/lifecycle-journal/001 |
| ResumeStateMachine | /v1/resume-state-machine/001, /v1/resume-state-machine/002 |
| admission_header_before_ack | /v1/admission-header-before-ack/001 |

---

## AskAnswerLifecycle.tla — Repair Instructions

### Finding /v1/ask-answer-lifecycle/001 (LETHAL)

**Problem**: `MonotonicSeqNo` at line 148-150 is vacuous (`SeqNoCounter >= 0` always true).

**Fix**: Remove `MonotonicSeqNo` from the spec and cfg. If you need a monotonic sequence number property, replace with:
```tla
MonotonicSeqNoPerRun ==
    \A run \in RunId :
        \A i, j \in 1..Len(AnsweredLog) :
            AnsweredLog[i].run = run /\ AnsweredLog[j].run = run /\ i < j
                => AnsweredLog[i].seq < AnsweredLog[j].seq
```
This verifies that for each run, sequence numbers in the answer log are strictly increasing.

### Finding /v1/ask-answer-lifecycle/002 (LETHAL)

**Problem**: `PendingSubset` at line 145-146 is identical to the TypeOK constraint on `PendingAnswers`.

**Fix**: Remove `PendingSubset` from the spec and cfg. Replace with a meaningful invariant:
```tla
PendingMatchesAwaiting ==
    \A run \in RunId :
        AskState[run] = "awaiting" <=> \E step \in StepIdx, seq \in SeqNo : <<run, step, seq>> \in PendingAnswers
```
This verifies that `PendingAnswers` and `AskState` are always consistent.

### Finding /v1/ask-answer-lifecycle/003 (LETHAL)

**Problem**: `EventuallyAnswered` and `EventuallyAdvanced` are in cfg PROPERTIES but the `Fairness` block is not included in the cfg.

**Fix**: Add to the .cfg file:
```
FAIRNESS
    WF_vars(AnswerAny)
    WF_vars(AdvanceAny)
```
Or, verify the TLC output shows fairness was applied to those actions.

---

## RetryFSM.tla — Repair Instructions

### Finding /v1/retry-fsm/001 (LETHAL)

**Problem**: `NoStaleCompletion` at line 161-164 is vacuous (`actionAttempts >= 0` always true).

**Fix**: Remove `NoStaleCompletion`. The actual stale-completion safety property should be:
```tla
NoStaleCompletionAccepted ==
    \A run \in Runs, step \in Steps :
        stepState[run][step] = "Running" /\ actionAttempts[run][step] > 0
            => \E i \in 1..Len(journal) : journal[i].type = "ActionFailed" /\ journal[i].run = run /\ journal[i].step = step
```
This verifies that if a step has attempted retries, there is a corresponding journal entry.

### Finding /v1/retry-fsm/002 (HIGH)

**Problem**: `EventuallyTerminalOrExhausted` is declared in the spec but not in the cfg as a PROPERTY.

**Fix**: Add to the cfg:
```
PROPERTIES
    EventuallyTerminalOrExhausted
```
And verify TLC runs with `--fairness` or equivalent to apply weak fairness.

### Finding /v1/retry-fsm/003 (HIGH)

**Problem**: Bounds are too small (`RunId = {1}`, `StepId = {1, 2}`, `MaxAttemptsValue = 2`).

**Fix**: Update the cfg:
```
CONSTANT
    RunId = {1, 2, 3}
    StepId = {1, 2, 3, 4}
    MaxAttemptsValue = 3
```
This allows 3 runs, 4 steps, and 2 retries (3 total attempts).

---

## RetryJournal.tla — Repair Instructions

### Finding /v1/retry-journal/001 (LETHAL)

**Problem**: `JournalIdempotency` is mechanically guaranteed by the `AppendActionFailed` guard.

**Fix**: Replace with a meaningful idempotency property:
```tla
ObservableStateUnchangedByDuplicate ==
    \A run \in Runs, step \in Steps :
        LET duplicates == {i \in 1..Len(journal) : journal[i].type = "ActionFailed" /\ journal[i].run = run /\ journal[i].step = step}
        IN
        Cardinality(duplicates) >= 2
            => actionAttempts[run][step] = [j \in Runs |-> [s \in Steps |-> IF run = j /\ step = s THEN 1 ELSE 0]]
```
This verifies that duplicate appends do not change `actionAttempts`.

### Finding /v1/retry-journal/002 (HIGH)

**Problem**: `ActionFailedEventOrder` is structurally guaranteed.

**Fix**: Either remove this invariant, or replace with a meaningful ordering property that captures what you actually want to verify about event ordering.

### Finding /v1/retry-journal/003 (HIGH)

**Problem**: `MaxJournalAttempts = 1` is unrealistically small.

**Fix**: Update cfg to `MaxJournalAttempts = 3` minimum.

---

## LifecycleJournal.tla — Repair Instructions

### Finding /v1/lifecycle-journal/001 (LETHAL)

**Problem**: No `THEOREM` statements in the spec.

**Fix**: Add to the end of the spec file (before the final `====`):
```tla
THEOREM Spec => []NoOverwrite
THEOREM Spec => []SingleCanonicalState
THEOREM Spec => []InvalidTransitionBlocked
THEOREM Spec => []ReplayBitIdentical
THEOREM Spec => []TypeInvariant
THEOREM Spec => []JournalFullIsTyped
THEOREM Spec => []ResourceExhaustionDoesNotOverwrite
THEOREM Spec => <>[]EventuallyTerminalOrCancelled
```

---

## ResumeStateMachine.tla — Repair Instructions

### Finding /v1/resume-state-machine/001 (LETHAL)

**Problem**: No `THEOREM` statements.

**Fix**: Add before `====`:
```tla
THEOREM Spec => []TypeOK
THEOREM Spec => []NoDoubleRunning
THEOREM Spec => []FailedNotResumable
THEOREM Spec => []JournalImmutable
THEOREM Spec => []JournalAppendBeforeSuccess
```

### Finding /v1/resume-state-machine/002 (LETHAL)

**Problem**: `NoDoubleRunning` and `FailedNotResumable` are structurally guaranteed by action guards.

**Fix**: Remove these invariants. Replace with meaningful safety:
```tla
FailedCannotResumeDirectly ==
    \A r \in RunIds :
        runtimeState[r] = "Failed" => ~ENABLED BeginResume(r)

ResumedImpliesJournalHasResumedEvent ==
    \A r \in RunIds :
        r \in resumed =>
            \E i \in DOMAIN journal :
                journal[i].kind = "Resumed" /\ journal[i].run = r
```

---

## admission_header_before_ack.tla — Repair Instructions

### Finding /v1/admission-header-before-ack/001 (LETHAL)

**Problem**: No `THEOREM` statements.

**Fix**: Add before `====`:
```tla
THEOREM Spec => []TypeOK
THEOREM Spec => []FailurePreventsAck
THEOREM Spec => []DuplicateRejectsNoLiveState
THEOREM Spec => []AckRequiresPersistence
THEOREM Spec => []LiveStateRequiresPersistence
THEOREM Spec => []NoLiveStateBeforeDurableAdmission
THEOREM Spec => []FailureEventuallyRejected
THEOREM Spec => []SuccessEventuallyAcked
```

### Finding /v1/admission-header-before-ack/003 (MEDIUM)

**Problem**: `CHECK_DEADLOCK TRUE` conflicts with `TerminalStutter`.

**Fix**: In the cfg, change:
```
CHECK_DEADLOCK TRUE
```
to:
```
CHECK_DEADLOCK FALSE
```
Or remove `TerminalStutter` from `Next` and rely on stuttering semantics.

---

## Global Repairs

### Bounds Increase (all specs with RunId)

For every spec that models multi-run behavior, change `RunId = {1}` or `MaxRunId = 1` to `RunId = {1, 2}` or `MaxRunId = 2` minimum. Document why the bound is sufficient in a comment.

### Non-Vacuity Probe

For each invariant that is not TypeOK or structurally guaranteed, add a nonvacuity cfg that intentionally violates the invariant. Example:
```
INVARIANT
    NotVacuityProbe_NoDoubleRunning  \* negate NoDoubleRunning to confirm TLC detects violation
```
Run this probe and confirm TLC reports a violation. This proves the invariant is not vacuous.

### TLC Command Evidence

After making repairs, run TLC for each spec and capture the raw output:
```bash
java -jar tla2tools.jar -config specs/AskAnswerLifecycle.cfg specs/AskAnswerLifecycle.tla 2>&1 | tee AskAnswerLifecycle.tlc.log
```
Confirm in the log:
1. "Model checking completed. No error has been found."
2. The number of distinct states generated
3. For PROPERTIES: "Checking temporal properties..." and "Success" or failure details
4. Fairness statistics if applicable
