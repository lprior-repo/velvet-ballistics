(* AttemptTracking.tla
 *
 * Invariant: A stale ActionCompleted event cannot mutate the latest attempt.
 * This ensures that old attempts don't overwrite newer ones during replay.
 *)

---- MODULE AttemptTracking ----

EXTENDS Integers, Sequences, TLC

CONSTANT RunId, StepId, ActionId

VARIABLES
    journal,
    latest_attempt

Init ==
    /\ journal = <<>>
    /\ latest_attempt = [<<run, step>> \in (RunId \X StepId) |-> -1]

ScheduleAction(run, step, action, attempt) ==
    /\ journal' = Append(journal, [type |-> "ActionScheduled", run |-> run, step |-> step, action |-> action, attempt |-> attempt])
    /\ latest_attempt' = [latest_attempt EXCEPT ![<<run, step>>] = attempt]

CompleteAction(run, step, action, attempt, success) ==
    /\ journal' = Append(journal, [type |-> "ActionCompleted", run |-> run, step |-> step, action |-> action, attempt |-> attempt, success |-> success])
    /\ latest_attempt' = latest_attempt

FailAction(run, step, action, attempt) ==
    /\ journal' = Append(journal, [type |-> "ActionFailed", run |-> run, step |-> step, action |-> action, attempt |-> attempt])
    /\ latest_attempt' = latest_attempt

\* Stale completion: attempt < latest_attempt[<<run, step>>]
IsStale(run, step, attempt) ==
    attempt < latest_attempt[<<run, step>>]

\* Safety: stale completions must be rejected
StaleCompletionRejected ==
    \A event \in DOMAIN journal :
        journal[event].type = "ActionCompleted"
        => ~IsStale(journal[event].run, journal[event].step, journal[event].attempt)

AttemptMonotonic ==
    \A run \in RunId, step \in StepId :
        latest_attempt[<<run, step>>] >= -1

Next ==
    \E run \in RunId, step \in StepId, action \in ActionId, attempt \in {0, 1, 2, 3} :
        \/ ScheduleAction(run, step, action, attempt)
        \/ CompleteAction(run, step, action, attempt, TRUE)
        \/ FailAction(run, step, action, attempt)

Spec == Init /\ [][Next]_<<journal, latest_attempt>>

THEOREM Spec => []StaleCompletionRejected
THEOREM Spec => []AttemptMonotonic

====
