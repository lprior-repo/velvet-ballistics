# BLACK HAT REVIEW — vb-rpch RecoveryReplayFull.tla
## STATUS: **REJECTED**

**Bead**: vb-rpch
**Date**: 2026-05-19
**Reviewer**: black-hat-reviewer
**Counterexample Source**: TLC model checking (not simulation)
**Violation**: `NoResolvedReExecution` violated at state 388,000+

---

## Counterexample Trace (Provided)

```
State 2: ActionCompleted(run=1, step=1, action=1, attempt=1)
         → AppendEvent to journal; tracker UNCHANGED

State 3: ActionScheduled(run=1, step=1, action=1, attempt=1)
         → AppendEvent to journal; VIOLATION of NoResolvedReExecution
```

**Root Cause Claim (from femdation controller)**:
> The spec models "replay" as ReplayEvents producing a filtered subset, but the actual journal is being appended to. tracker never gets updated because resolved events are not removed from journal — they're just filtered in the ReplayEvents output.

**BLACK HAT VERDICT**: The root cause analysis is PARTIALLY CORRECT but INCOMPLETE. There are multiple fundamental semantic defects, not just one.

---

## DEFECT-1: ReplayEvents Does Not Update Tracker on ActionCompleted Append

### Location
`RecoveryReplayFull.tla:110-113` (AppendEvent) and `RecoveryReplayFull.tla:132-144` (ReplayEvents)

### Analysis
When `ActionCompleted` is appended via `AppendEvent`, the action is:
```tla
AppendEvent(e) ==
    /\ Len(journal) < MAX_EVENTS
    /\ journal' = Append(journal, e)
    /\ UNCHANGED <<snapshot_seq, tracker, digest_level, recovered_runs, last_error>>
```

`tracker` is explicitly UNCHANGED. Therefore after State 2 (ActionCompleted), `tracker.completed` does NOT contain `{action:1, step:1}`.

### Consequence
The completion is ONLY in the journal, not in the tracker. ReplayEvents has no way to know this action was completed unless it examines the journal directly — but it doesn't filter out completed actions.

**VERDICT**: DEFECT CONFIRMED. This is a real semantic error.

---

## DEFECT-2: ReplayEvents Filters Journal But Events Remain In Place

### Location
`RecoveryReplayFull.tla:138-140`

```tla
LET new_journal == IF filtered_idx = {} THEN <<>>
    ELSE IF filtered_idx = DOMAIN journal THEN journal
    ELSE BuildSeqFromIndices(filtered_idx, <<>>)
```

### Analysis
- `filtered_idx` selects indices where `journal[i].run = run /\ journal[i].attempt = max_att`
- `BuildSeqFromIndices` reconstructs a sequence from those indices
- But `filtered_idx = DOMAIN journal` returns the FULL journal unchanged

The logic only rebuilds when `filtered_idx ≠ DOMAIN journal`. If a run's events span the entire journal, the journal is passed through unchanged. This means:
1. Previous completions are NOT removed from journal
2. ReplayEvents doesn't actually filter completed actions
3. The `resolved` set is used only to update tracker, not to filter journal

### Consequence
`NoResolvedReExecution` checks journal for "does ActionCompleted exist before ActionScheduled?" But if completions aren't removed from journal AND tracker doesn't block scheduling, the model allows re-execution.

**VERDICT**: DEFECT CONFIRMED. Journal semantics are broken.

---

## DEFECT-3: tracker.completed Loses Attempt Information

### Location
`RecoveryReplayFull.tla:69` (tracker type) and `RecoveryReplayFull.tla:137` (resolved construction)

```tla
tracker \in [completed: SUBSET [action: ActionId, step: StepId], failed: SUBSET [action: ActionId, step: StepId]]
```

```tla
LET resolved == {[action |-> journal[i].action, step |-> journal[i].step] : i \in scheduled}
```

### Analysis
`tracker.completed` stores only `{action, step}` pairs. The `attempt` field is DROPPED when constructing `resolved`.

But `NoResolvedReExecution` checks:
```tla
journal[j].attempt = journal[i].attempt
```

This comparison uses the attempt from journal events, NOT from tracker. There is NO invariant that says:
```
action+step in tracker.completed => action+step never scheduled at same attempt
```

The tracker is action+step only, but the invariant expects attempt-aware blocking.

### Consequence
Even if tracker.was_completed({action:1, step:1}) were checked before scheduling, it would block ALL attempts, not just attempt=1. The model cannot distinguish "completed at attempt=1" from "completed at any attempt."

**VERDICT**: DEFECT CONFIRMED. Type system and invariant are mismatched.

---

## DEFECT-4: No Mechanism to Block ActionScheduled After ActionCompleted

### Location
`RecoveryReplayFull.tla:210-219` (Next action)

```tla
Next ==
    \/ \E type \in EventType, run \in RunId, step \in StepId,
          action \in ActionId, attempt \in Attempt, seq \in EventSeqNum :
        AppendEvent(MakeEvent(type, run, step, action, attempt, seq, 1, 1))
```

### Analysis
`AppendEvent` appends ANY event type, including `ActionScheduled`, without checking whether:
1. This (action, step, attempt) was already completed
2. This (action, step, attempt) is already in the journal

There is no guard in `AppendEvent` or in a wrapper that prevents re-scheduling.

### Consequence
The model allows appending `ActionScheduled` for (action:1, step:1, attempt:1) even after `ActionCompleted` for the same keys exists in journal. No state variable prevents this.

**VERDICT**: DEFECT CONFIRMED. No pre-condition for scheduling.

---

## DEFECT-5: ActionFailed Is Tracked in tracker.failed But Never Used to Block Scheduling

### Location
`RecoveryReplayFull.tla:69` and `RecoveryReplayFull.tla:137`

### Analysis
`tracker.failed` is defined but never updated by ReplayEvents. The `resolved` set only comes from `scheduled` (ActionScheduled), not from ActionFailed events.

This means failed actions can be re-scheduled without constraint, which may or may not be intentional but is certainly underspecified.

**VERDICT**: Potential defect depending on intended semantics.

---

## DEFECT-6: BuildSeqFromIndices Is a No-Op When filtered_idx = DOMAIN journal

### Location
`RecoveryReplayFull.tla:139`

```tla
ELSE IF filtered_idx = DOMAIN journal THEN journal
```

### Analysis
When `filtered_idx` equals all indices in journal, the journal passes through UNCHANGED. This is supposed to handle "no replay needed" but actually means replay does nothing.

The comment says "Replayed events produce filtered subset" but the code only filters when `filtered_idx ≠ DOMAIN journal`. If a single run spans the entire journal, no filtering happens.

**VERDICT**: Misleading semantics. The comment promises filtering; the code doesn't filter in the common case.

---

## Formal Waiver Analysis

The formal-verification-report.md shows:
```
| TLA+ | TLA-NONIDEM-001 (NoResolvedReExecution) | WAIVED | State space explosion; simulation mode used (21,404 states) |
```

**This waiver is INVALID**:
1. The counterexample occurs at **388k states**, far beyond simulation coverage
2. The waiver was granted for "state space explosion" but the actual violation is a **structural semantic error** that would appear in any exhaustive check
3. A waiver for performance cannot excuse a **model bug**

The spec is wrong. It cannot be waived into correctness.

---

## GOD RULES Non-Compliance

| GOD RULE | Violation |
|----------|-----------|
| No hardcoded Kani shapes | N/A — this is TLA+ |
| No vacuum Verus proofs | N/A |
| No unbounded TLA+ math | The model uses `1..Len(journal)` which is bounded by MAX_EVENTS=20, but the semantic error manifests before bounds are hit |
| No loop oscillations | N/A |
| No blind verification mutations | N/A |

**Critical TLA+ Issue**: The spec claims to model "replay" but the model does not prevent re-execution. This is NOT a performance problem — it is a **correctness problem**. Waiving exhaustive checking does not fix the model.

---

## What Must Be Fixed

1. **AppendEvent must check tracker**: Before appending ActionScheduled, verify (action, step, attempt) ∉ tracker.completed AND not already in journal at same attempt

2. **Or: Journal must be filtered on replay**: ReplayEvents must actually remove completed events from journal, not just update tracker

3. **Or: NoResolvedReExecution must be dropped**: If the intent is to only track tracker state, the invariant must be restated in terms of tracker

4. **tracker.completed must include attempt**: If attempt-aware blocking is needed, tracker.completed must be `SUBSET [action: ActionId, step: StepId, attempt: Attempt]`

5. **BuildSeqFromIndices semantics must be clarified**: Either filter always (even if filtered_idx = DOMAIN journal returns a copy), or document when filtering doesn't happen

---

## Black Hat Verdict

| Finding | Severity | Status |
|---------|----------|--------|
| DEFECT-1: AppendEvent doesn't update tracker | CRITICAL | CONFIRMED |
| DEFECT-2: ReplayEvents doesn't filter completed from journal | CRITICAL | CONFIRMED |
| DEFECT-3: tracker.completed loses attempt | HIGH | CONFIRMED |
| DEFECT-4: No guard on ActionScheduled append | CRITICAL | CONFIRMED |
| DEFECT-5: ActionFailed not tracked | LOW | UNCERTAIN |
| DEFECT-6: BuildSeqFromIndices no-op | MEDIUM | CONFIRMED |
| Formal waiver invalid | CRITICAL | WAIVER REJECTED |

**OVERALL STATUS**: **REJECTED**

The model is **semantically broken**. It does not correctly implement replay-with-no-re-execution. The counterexample at 388k states proves the invariant is violated by design, not by state explosion.

**No amount of waivers, tooling changes, or performance optimization will fix this**. The spec must be rewritten.

---

*Black Hat Review: REJECTED*
*Reviewer: black-hat-reviewer*
*Date: 2026-05-19*
*Counterexample: 388k+ states, NoResolvedReExecution violated*
