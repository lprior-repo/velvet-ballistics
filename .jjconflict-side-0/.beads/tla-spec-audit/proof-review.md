# TLA+ Spec Non-Vacuity Audit Report

**Reviewer**: proof-reviewer
**Date**: 2026-05-26
**Artifacts Audited**: AskAnswerLifecycle.tla, RetryFSM.tla, RetryJournal.tla, LifecycleJournal.tla, ResumeStateMachine.tla, admission_header_before_ack.tla (plus corresponding .cfg files)
**Standard**: Non-vacuity audit per proof-reviewer skill and AGENTS.md Formal Verification Mandate (Rule 3: No Unbounded TLA+ Math)

---

## Summary Table

| Spec | THEOREMs Verified by TLC? | Non-Trivial Invariants? | Temporal/Liveness Properties? | Fairness Justified? | Bounds Realistic? | STATUS |
|------|--------------------------|------------------------|------------------------------|---------------------|-------------------|--------|
| AskAnswerLifecycle | ✓ (5 invariants) | Partial | ✓ but NOT in cfg | Partial | Too small (1 run, 4 seqs) | **REJECTED** |
| RetryFSM | ✓ (5 invariants) | Partial | ✗ NONE | ✗ NONE | Too small (1 run, 2 steps) | **REJECTED** |
| RetryJournal | ✓ (2 invariants) | ✗ VACUOUS | ✗ NONE | ✗ NONE | Severely limited | **REJECTED** |
| LifecycleJournal | ✗ NO THEOREMs | ✓ Meaningful | ✓ but NOT in cfg | ✓ defined but NOT in cfg | 2 beads, journal 10 | **REJECTED** |
| ResumeStateMachine | ✗ NO THEOREMs | ✗ VACUOUS | ✗ NONE | ✗ NONE | 2 runs, journal 4 | **REJECTED** |
| admission_header_before_ack | N/A (no THEOREMs) | ✓ Meaningful | ✓ but NOT in cfg | ✓ defined but NOT in cfg | Adequate | **REJECTED** |

---

## Finding /v1/ask-answer-lifecycle/001 — Vacuous MonotonicSeqNo

**Severity**: LETHAL  
**Artifact**: `specs/AskAnswerLifecycle.tla:148-150`, `specs/AskAnswerLifecycle.cfg:7`  
**Obligation**: MonotonicSeqNo theorem  
**Evidence**:  
```
MonotonicSeqNo ==
    \A run \in RunId :
        SeqNoCounter[run] >= 0
```
**Finding**: This invariant is **trivially true** in all reachable states. `SeqNoCounter` is initialized at 0 and only incremented via `SeqNoCounter' = [SeqNoCounter EXCEPT ![run] = SeqNoCounter[run] + 1]` with the guard `SeqNoCounter[run] < MaxSeqNo`. Since all arithmetic is bounded non-negative and the upper bound is enforced by the action guard, this is a type-check invariant, not a safety property. Proving `x >= 0` when `x` is a `0..MaxSeqNo` integer proves nothing beyond TypeOK.

**Required Fix**: Remove `MonotonicSeqNo` as a separate theorem. If monotonicity of sequence numbers within a run is the property to verify, encode it as: once `SeqNoCounter[run] = n`, it can never return to any value `< n` for the same run. This requires a temporal property or a stronger inductive invariant.

---

## Finding /v1/ask-answer-lifecycle/002 — PendingSubset is Subsumed by TypeOK

**Severity**: LETHAL  
**Artifact**: `specs/AskAnswerLifecycle.tla:145-146`, `specs/AskAnswerLifecycle.cfg:6`  
**Obligation**: PendingSubset theorem  
**Evidence**:  
```
PendingSubset ==
    PendingAnswers \subseteq (RunId \X StepIdx \X SeqNo)
```
**Finding**: `TypeOK` at line 36-40 already constrains `PendingAnswers \in SUBSET (RunId \X StepIdx \X SeqNo)`. The `PendingSubset` invariant is **identical** to the type constraint for `PendingAnswers`. This is not an independent proof obligation; it proves nothing beyond what TypeOK already guarantees.

**Required Fix**: Either remove `PendingSubset` as redundant, or redefine it as a meaningful property such as: "Every pending answer ticket has a corresponding run in `awaiting` state" (i.e., `PendingAnswers` tracks in-flight work that matches the `AskState`).

---

## Finding /v1/ask-answer-lifecycle/003 — Temporal Properties Not Verified by TLC

**Severity**: LETHAL  
**Artifact**: `specs/AskAnswerLifecycle.tla:152-158`, `specs/AskAnswerLifecycle.cfg:9-11`  
**Obligation**: `EventuallyAnswered`, `EventuallyAdvanced`  
**Evidence**:  
```
EventuallyAnswered ==
    \A run \in RunId :
        (AskState[run] = "awaiting") ~> (AskState[run] \in {"answered", "failed"})

EventuallyAdvanced ==
    \A run \in RunId :
        (AskState[run] = "answered") ~> (AskState[run] = "idle")
```
The .cfg declares these as `PROPERTIES` (lines 9-11), meaning TLC **should** check them. However:
1. The `Fairness` definition in the spec (lines 125-127) specifies `WF_vars(AnswerAny)` and `WF_vars(AdvanceAny)`, but **the cfg does not include the Fairness block**.
2. Without fairness, TLC's default behavior for PROPERTIES is **undefined** — the model may report "property proved" or "property not checked" depending on the configuration.
3. The liveness properties require `~> (leads-to)` to hold, which needs weak fairness on the enabling actions to be meaningful.

**Required Fix**: Add `Fairness` to the SPECIFICATION definition in the cfg, or explicitly declare the fairness constraints separately. Without this, the temporal properties are not verified.

---

## Finding /v1/ask-answer-lifecycle/004 — SubmitAsk Has No Fairness (May Deadlock on Progress)

**Severity**: HIGH  
**Artifact**: `specs/AskAnswerLifecycle.tla:79-88`, `specs/AskAnswerLifecycle.cfg`  
**Obligation**: Progress/liveness  
**Evidence**: The spec comments at lines 72-78 state: "No fairness is assumed for SubmitAsk, so the model does not require the environment to submit new asks forever." However, `SubmitAny` is part of `Next`, and if no run ever submits an ask, the system stutters forever. The cfg has `CHECK_DEADLOCK TRUE` implicitly (default).

The comment justifies the design choice, but this means **there is no guarantee the system ever makes progress**. The `EventuallyAnswered` and `EventuallyAdvanced` properties are vacuously true if no ask is ever submitted. This is a design choice, not a proof defect, but it must be documented and the cfg should explicitly set `CHECK_DEADLOCK` to reflect the intended behavior.

**Required Fix**: Add `CHECK_DEADLOCK FALSE` if intentional, or add `SF_vars(SubmitAny)` if progress is required.

---

## Finding /v1/retry-fsm/001 — NoStaleCompletion is Vacuous

**Severity**: LETHAL  
**Artifact**: `specs/RetryFSM.tla:161-164`, `specs/RetryFSM.cfg:20-21`  
**Obligation**: NoStaleCompletion theorem  
**Evidence**:  
```
NoStaleCompletion ==
    \A run \in Runs, step \in Steps :
        stepState[run][step] = "Running"
            => actionAttempts[run][step] >= 0
```
`actionAttempts` is initialized to 0 and only incremented. By the type signature in `TypeOK` (line 132), `actionAttempts[run][step] \in 0..MAX_U16`. A counter that can only be 0..65535 is always `>= 0`. This invariant can never be violated.

**Required Fix**: Remove `NoStaleCompletion`. If the intended property is "no stale completion is accepted", the actual property would need to reference journal state or compare attempt counts.

---

## Finding /v1/retry-fsm/002 — No Liveness Properties or Temporal THEOREMs

**Severity**: HIGH  
**Artifact**: `specs/RetryFSM.tla:177-181`, `specs/RetryFSM.cfg`  
**Obligation**: Temporal properties  
**Evidence**: The spec defines `EventuallyTerminalOrExhausted` at lines 173-175:
```
EventuallyTerminalOrExhausted ==
    <>(/\ runs # {}
       /\ \E run \in Runs, step \in Steps : stepState[run][step] = "Failed")
```
But this is **not declared as a PROPERTY** in the cfg, and there is no `THEOREM Spec => <>[]` or similar temporal theorem. The cfg only checks safety invariants. The spec comment at line 6 states "Liveness: every retryable failed action eventually reaches terminal or exhaustion" but this is never verified.

**Required Fix**: Add `EventuallyTerminalOrExhausted` as a PROPERTY in the cfg and verify it with TLC. Or remove the claim from the spec comment if liveness is not required.

---

## Finding /v1/retry-fsm/003 — Model Bounds Unrealistically Small

**Severity**: HIGH  
**Artifact**: `specs/RetryFSM.cfg:3-6`  
**Obligation**: Realistic bounds for Rust constraints  
**Evidence**:  
```
CONSTANT
    RunId = {1}
    StepId = {1, 2}
    MaxAttemptsValue = 2
```
- `RunId = {1}` — only 1 run. Real Rust systems handle thousands of concurrent runs.
- `StepId = {1, 2}` — only 2 steps. A real workflow may have dozens of steps.
- `MaxAttemptsValue = 2` — only 1 retry allowed before exhaustion. The spec comment claims this models "retry transitions" but a single retry is not representative of any real retry policy.

The `NoDoubleRetryAfterExhaustion` invariant (line 145) says once `actionAttempts >= maxAttempts`, stepState must be "Failed". With `MaxAttemptsValue = 2`, this means the retry FSM exhausts after just 1 retry. This does not verify the correctness of retry logic for realistic retry budgets (e.g., 3 retries = 4 attempts total in Rust production code).

**Required Fix**: Increase bounds to at least `RunId = {1, 2, 3}`, `StepId = {1, 2, 3, 4}`, `MaxAttemptsValue = 3` to demonstrate the retry exhaustion logic is not path-dependent.

---

## Finding /v1/retry-journal/001 — JournalIdempotency is Guaranteed by Action Guard

**Severity**: LETHAL  
**Artifact**: `specs/RetryJournal.tla:104-106`, `specs/RetryJournal.cfg:8-9`  
**Obligation**: JournalIdempotency theorem  
**Evidence**:  
```
JournalIdempotency ==
    \A run \in Runs, step \in Steps :
        actionAttempts[run][step] <= MaxAttempts
```
The only action that increments `actionAttempts` is `AppendActionFailed` (line 61), which has the guard `actionAttempts[run][step] < MaxAttempts`. Therefore `actionAttempts` can **never** exceed `MaxAttempts` by construction. This invariant is not verified; it is mechanically guaranteed by the action definition.

**Required Fix**: The actual idempotency property should be: "Appending the same ActionFailed event twice does not change observable state." This requires checking that `actionAttempts`, `stepState`, and `framePC` are unchanged by `AppendDuplicateActionFailed`, which is already defined. Rename and refine the invariant to capture the intended semantics.

---

## Finding /v1/retry-journal/002 — ActionFailedEventOrder is Subsumed by StateConstraint

**Severity**: HIGH  
**Artifact**: `specs/RetryJournal.tla:112-116`, `specs/RetryJournal.cfg:10`  
**Obligation**: ActionFailedEventOrder theorem  
**Evidence**:  
```
ActionFailedEventOrder ==
    \A i \in 1..Len(journal), j \in 1..Len(journal) :
        i < j /\ journal[i].type = "ActionFailed" /\ journal[j].type = "ActionFailed"
            => (journal[i].run # journal[j].run \/ journal[i].step # journal[j].step
                \/ journal[i].attempt <= journal[j].attempt)
```
This property is **automatically satisfied** because:
1. `AppendActionFailed` only appends when `actionAttempts[run][step] < MaxAttempts`
2. The `attempt` stored is `actionAttempts[run][step]` at the time of append
3. Since `actionAttempts` only increases, later appends always have `attempt >=` earlier appends for the same (run, step)

The cfg also has `StateConstraint` with `Len(journal) <= 10` and `duplicateCount <= 2`, which already limits journal growth. The `ActionFailedEventOrder` adds no constraint beyond the action guard semantics.

**Required Fix**: If the intent is to prove ordering across **different** (run, step) pairs is unconstrained, this property doesn't capture that. If the intent is to prove per-(run, step) ordering, the formulation with `journal[i].run # journal[j].run \/ ...` complicates the quantifier. Simplify to per-(run,step) ordering if that is the intended property.

---

## Finding /v1/retry-journal/003 — MaxJournalAttempts=1 Renders Model Toy-Size

**Severity**: HIGH  
**Artifact**: `specs/RetryJournal.cfg:6`  
**Obligation**: Realistic model bounds  
**Evidence**: `MaxJournalAttempts = 1` means each (run, step) can only have **one** ActionFailed event appended before the guard blocks further appends. This makes the "duplicate" test case (`AppendDuplicateActionFailed`) the only way to grow the journal beyond 1 event per (run, step). The journal length constraint of `Len(journal) <= 10` is barely exercised with `MaxJournalAttempts = 1`.

Compare to the Rust constraint this models: `actionAttempts` is a `u16` counter (0..65535). The model with `MaxJournalAttempts = 1` cannot represent a single real retry scenario.

**Required Fix**: Set `MaxJournalAttempts = 3` minimum to represent a realistic retry budget.

---

## Finding /v1/lifecycle-journal/001 — No THEOREM Statements Despite Invariant Claims

**Severity**: LETHAL  
**Artifact**: `specs/LifecycleJournal.tla` (entire file)  
**Obligation**: Proof obligation traceability  
**Evidence**: The spec defines `Init`, `Next`, `Spec`, and multiple invariants, but has **no `THEOREM` statements**. The spec header at lines 3-14 describes this as a "Bounded TLC model for vb-qi37.16.5 LifecycleJournal obligations" with "Finite reductions used by TLC". However, without `THEOREM` statements, there is no machine-verifiable link between the spec invariants and the cfg invariants.

TLC will check the invariants declared in the .cfg file, but the absence of `THEOREM` declarations means:
1. The proof obligation tracker cannot map spec properties to cfg verification results
2. There is no declarative statement of what the spec is supposed to prove
3. Any "proof" is implicit in TLC's invariant checking, not declared in the spec itself

**Required Fix**: Add `THEOREM` statements for each invariant: `THEOREM Spec => []NoOverwrite`, `THEOREM Spec => []SingleCanonicalState`, etc.

---

## Finding /v1/lifecycle-journal/002 — ReplayBitIdentical Does Not Verify ReplayState Correctness

**Severity**: HIGH  
**Artifact**: `specs/LifecycleJournal.tla:265-270`, `specs/LifecycleJournal.cfg:10`  
**Obligation**: Replay correctness  
**Evidence**:  
```
ReplayStateFor(b) ==
    LET relevant == {i \in 1..Len(journal) : journal[i].bead_id = b} IN
        IF relevant = {} THEN "Pending"
        ELSE journal[MaxOf(relevant)].next

ReplayState == [b \in Beads |-> ReplayStateFor(b)]
...
ReplayBitIdentical == bead_state = ReplayState
```
`ReplayBitIdentical` checks that after `Replay`, `bead_state` equals the computed `ReplayState`. But `ReplayState` is defined as the `next` field of the **last journal event** for each bead. This assumes:
1. The journal's `next` field correctly records the state after each transition
2. `ReplayStateFor` correctly selects the last event's `next` field

**There is no invariant verifying `ReplayState` is correctly computed from the journal.** The invariant only verifies post-Replay state matches the computed value, not that the computation is correct.

**Required Fix**: Add an invariant such as `ReplayStateIsLastNext` that verifies `ReplayStateFor(b)` equals the `next` field of the most recent event for bead `b` in the journal, independent of the `Replay` action.

---

## Finding /v1/lifecycle-journal/003 — Fairness in Spec Not Mirrored in .cfg

**Severity**: HIGH  
**Artifact**: `specs/LifecycleJournal.tla:308-314`, `specs/LifecycleJournal.cfg`  
**Obligation**: Fairness assumptions  
**Evidence**: The spec defines:
```
Spec ==
    /\ Init
    /\ [][Next]_vars
    /\ WF_vars(Process)
    /\ WF_vars(Replay)
    /\ \A b \in Beads : SF_vars(Start(b))
    /\ \A b \in Beads : SF_vars(SubmitTerminal(b))
```
But the .cfg file only contains `SPECIFICATION Spec` with `INVARIANT` declarations. **The cfg does not include the fairness declarations**. When TLC processes `SPECIFICATION Spec`, it should honor the fairness defined inside `Spec`. However, the absence of explicit fairness configuration in the cfg means the model checker uses its defaults, which may not match the intended strong fairness on `Start(b)` and `SubmitTerminal(b)`.

**Required Fix**: Verify in the TLC output that fairness is actually being applied. If the cfg is the authoritative verification configuration, fairness should be explicitly declared in the cfg as well.

---

## Finding /v1/lifecycle-journal/004 — EventuallyTerminalOrCancelled Not in .cfg

**Severity**: MEDIUM  
**Artifact**: `specs/LifecycleJournal.tla:329-330`, `specs/LifecycleJournal.cfg`  
**Obligation**: Liveness property verification  
**Evidence**:  
```
EventuallyTerminalOrCancelled ==
    \A b \in Beads : <> (bead_state[b] \in TerminalState)
```
This liveness property states every bead eventually reaches a terminal state. It is **not declared as a PROPERTY** in the cfg. Without this, the liveness guarantee is not verified by TLC.

**Required Fix**: Add `EventuallyTerminalOrCancelled` to the `PROPERTIES` section of the cfg.

---

## Finding /v1/resume-state-machine/001 — No THEOREM Statements

**Severity**: LETHAL  
**Artifact**: `specs/ResumeStateMachine.tla` (entire file)  
**Obligation**: Proof obligation traceability  
**Evidence**: The spec defines `Spec`, `TypeOK`, invariants, but has **no `THEOREM` statements**. The cfg declares `INVARIANTS` for `TypeOK`, `ValidTransition`, `NoDoubleRunning`, `FailedNotResumable`, `JournalImmutable`, `JournalAppendBeforeSuccess`, but without `THEOREM` declarations in the spec, there is no machine-verifiable proof contract.

**Required Fix**: Add `THEOREM` statements for each invariant: `THEOREM Spec => []NoDoubleRunning`, etc.

---

## Finding /v1/resume-state-machine/002 — NoDoubleRunning and FailedNotResumable are Vacuous

**Severity**: LETHAL  
**Artifact**: `specs/ResumeStateMachine.tla:85-87`, `specs/ResumeStateMachine.cfg:10-11`  
**Obligation**: Non-trivial safety properties  
**Evidence**:  
```
NoDoubleRunning == \A r \in RunIds: runtimeState[r] = "Running" => r \notin pending
```
`pending` is only modified by `BeginResume(r)` which requires `runtimeState[r] = "Resumable"` and by `CompleteResume(r)` which removes from `pending` when `runtimeState[r] = "Resuming"`. **No action ever adds to `pending` while `runtimeState[r] = "Running"`.** Therefore `r \notin pending` is always true when `runtimeState[r] = "Running"` — this is guaranteed by the action guards, not a meaningful invariant.

```
FailedNotResumable == \A r \in RunIds: runtimeState[r] = "Failed" => ~ENABLED BeginResume(r)
```
`BeginResume(r)` requires `runtimeState[r] = "Resumable"`. If `runtimeState[r] = "Failed"`, then `runtimeState[r] \neq "Resumable"`, so `ENABLED BeginResume(r)` is false. This invariant is structurally guaranteed and proves nothing.

**Required Fix**: Remove these vacuous invariants. Replace with meaningful properties such as: "If a run reaches `Failed`, it cannot transition back to `Running` without going through `Resuming`" or "Every `Resuming` run that is not `CompleteResume`-d within bounded steps reaches `Failed`."

---

## Finding /v1/resume-state-machine/003 — JournalImmutable Duplicates StateConstraint

**Severity**: MEDIUM  
**Artifact**: `specs/ResumeStateMachine.tla:95`, `specs/ResumeStateMachine.cfg:12`  
**Obligation**: Non-redundant invariants  
**Evidence**:  
```
JournalImmutable == Len(journal) <= MaxJournalLength
```
The cfg already has an implicit `StateConstraint` from the `CanAppend` guard and journal append logic. The `JournalImmutable` invariant simply restates this bound. This is not a safety property over state transitions; it is a state bound already enforced by the action definitions.

**Required Fix**: Remove `JournalImmutable` or replace with a meaningful property such as: "If a run is in `resumed`, the journal contains a `Resumed` event for that run" (partially captured by `JournalAppendBeforeSuccess`).

---

## Finding /v1/resume-state-machine/004 — No Liveness Properties

**Severity**: MEDIUM  
**Artifact**: `specs/ResumeStateMachine.tla` (entire file)  
**Obligation**: Meaningful temporal properties  
**Evidence**: The spec has only safety invariants (`[]` properties). There is no `EventuallyResumed`, no `EventuallyFailed`, no `<>[]` or `~>` temporal property. The spec models a resume FSM but does not verify that any run eventually makes progress (reaches `Running` after `Resuming`) or eventually terminates.

**Required Fix**: Add a liveness property such as `EventuallySomeRunResumed` or `PendingRunEventuallyCompletes`.

---

## Finding /v1/admission-header-before-ack/001 — No THEOREM Statements

**Severity**: LETHAL  
**Artifact**: `specs/admission_header_before_ack.tla` (entire file)  
**Obligation**: Proof obligation traceability  
**Evidence**: The spec defines `Spec` with invariants and PROPERTIES, but has **no `THEOREM` statements**. The cfg declares invariants and properties to check, but without `THEOREM` declarations, there is no formal link between the TLA+ spec and what is being verified.

**Required Fix**: Add `THEOREM` statements: `THEOREM Spec => []TypeOK`, `THEOREM Spec => []FailurePreventsAck`, `THEOREM Spec => []AckRequiresPersistence`, etc.

---

## Finding /v1/admission-header-before-ack/002 — Temporal Properties Not Verified (cfg has PROPERTIES but spec lacks THEOREMs)

**Severity**: HIGH  
**Artifact**: `specs/admission_header_before_ack.tla:101-105`, `specs/admission_header_before_ack.cfg:17-19`  
**Obligation**: Liveness verification  
**Evidence**: The cfg declares:
```
PROPERTIES
    FailureEventuallyRejected
    SuccessEventuallyAcked
```
But the spec has no `THEOREM` statements linking these properties to `Spec`. Without `THEOREM Spec => []FailureEventuallyRejected` etc., TLC will check the properties but there is no machine-verifiable proof contract in the spec itself.

**Required Fix**: Add `THEOREM` statements for the temporal properties: `THEOREM Spec => []FailureEventuallyRejected`, `THEOREM Spec => []SuccessEventuallyAcked`.

---

## Finding /v1/admission-header-before-ack/003 — CHECK_DEADLOCK TRUE with TerminalStutter

**Severity**: MEDIUM  
**Artifact**: `specs/admission_header_before_ack.cfg:21`, `specs/admission_header_before_ack.tla:67-69`  
**Obligation**: Deadlock analysis  
**Evidence**: The cfg has `CHECK_DEADLOCK TRUE`. The spec has `TerminalStutter` which stutters when `state \in {"Rejected", "Acked"}`. If the system reaches a terminal state and `TerminalStutter` is enabled, TLC will report a **deadlock** because `CHECK_DEADLOCK TRUE` means TLC expects the system to make progress from all states.

This is a contradiction:
- `TerminalStutter` is in `Next` and stutters when terminal
- `CHECK_DEADLOCK TRUE` means TLC will error if no action is enabled

Unless the intent is for `TerminalStutter` to be the only enabled action at terminal states (making deadlock checking moot), this configuration is inconsistent.

**Required Fix**: Either set `CHECK_DEADLOCK FALSE` if `TerminalStutter` is the intended terminal behavior, or remove `TerminalStutter` from `Next` and rely on stuttering semantics with `[][Next]_vars`.

---

## Cross-Cutting Finding /v1/global/001 — All Specs Use Toy Bounds for MaxRunId/RunId

**Severity**: HIGH  
**Artifact**: All six specs  
**Obligation**: Realistic model bounds per AGENTS.md Rule 3 (bounded hardware limits)  
**Evidence**:  

| Spec | RunId Bound | Realistic? |
|------|-------------|------------|
| AskAnswerLifecycle | `MaxRunId = 1` | ✗ Real systems handle N runs |
| RetryFSM | `RunId = {1}` | ✗ |
| RetryJournal | `RunId = {1}` | ✗ |
| LifecycleJournal | `Beads = {b1, b2}` (2 beads) | ✗ Real systems handle many |
| ResumeStateMachine | `RunIds = {r1, r2}` | ✗ |
| admission_header_before_ack | N/A (no run concept) | ✓ |

**AGENTS.md Formal Verification Mandate Rule 3** states: "TLA+ specifications MUST model the exact bounded hardware limits of the target architecture (e.g., integer overflows at MAX_U64). You cannot use unbounded Nat to assume away arithmetic failures."

While the toy bounds are typical for TLC model-checking (state explosion is real), the specs do not document why the bounds are sufficient to catch bugs. A bound of `RunId = {1}` means cross-run interactions (which exist in the Rust implementation) are not verified.

**Required Fix**: Add comments explaining why each bound is sufficient to catch the targeted bug class. For cross-run interference bugs, at least `RunId = {1, 2}` is needed.

---

## Cross-Cutting Finding /v1/global/002 — Temporal Properties Missing from .cfg Files

**Severity**: HIGH  
**Artifact**: RetryFSM, RetryJournal, LifecycleJournal, ResumeStateMachine  
**Obligation**: Meaningful liveness verification  

**Finding**: Of the six specs reviewed:
- 2 have PROPERTIES declared in cfg and should verify liveness (AskAnswerLifecycle, admission_header_before_ack)
- 4 have NO temporal properties in cfg (RetryFSM, RetryJournal, LifecycleJournal, ResumeStateMachine)

The RetryFSM spec comment explicitly claims "Liveness: every retryable failed action eventually reaches terminal or exhaustion" but this is not verified. The RetryJournal spec has no liveness claims but models an idempotency journal that should verify "duplicate appends don't affect observable state" — this is currently only a safety invariant.

**Required Fix**: For each spec with a liveness claim in its comment, add that property to the cfg and verify with TLC.

---

---

## Verdict

**STATUS: REJECTED**

All six specs have at least one LETHAL finding:
- 5 specs have vacuous/TypeOK-subsumed invariants that prove nothing
- 5 specs lack THEOREM statements mapping spec properties to cfg verification
- 4 specs have liveness/temporal properties that are not verified
- All specs use toy model bounds (1-2 runs) that cannot catch cross-run interference bugs
- 2 specs (ResumeStateMachine, RetryFSM) have invariants that are structurally guaranteed by action guards and provide zero safety value

The `.cfg` files confirm TLC is running and invariant checks pass, but passing TypeOK-subsumed or structurally-guaranteed invariants is not evidence of meaningful verification. The specs fail the non-vacuity requirement: "Demand evidence that the verifier could fail."

---

## Required Remediation Before Re-Approval

1. Remove all vacuous invariants: `MonotonicSeqNo`, `PendingSubset`, `NoStaleCompletion`, `NoDoubleRunning`, `FailedNotResumable`, `JournalImmutable`
2. Replace each removed invariant with a non-trivial property that could conceivably fail
3. Add `THEOREM` statements to AskAnswerLifecycle, LifecycleJournal, ResumeStateMachine, admission_header_before_ack
4. Add PROPERTIES for liveness claims in RetryFSM, RetryJournal, LifecycleJournal, ResumeStateMachine
5. Increase model bounds to at least `RunId = {1, 2}` (2 runs) to catch cross-run interference
6. Add nonvacuity probes: for each invariant, add a cfg variant that intentionally violates the invariant to confirm TLC can detect the failure
7. Verify fairness is actually applied (check TLC output for fairness statistics)
8. Add `CHECK_DEADLOCK FALSE` where `TerminalStutter` is the intended terminal behavior

---

STATUS: REJECTED
