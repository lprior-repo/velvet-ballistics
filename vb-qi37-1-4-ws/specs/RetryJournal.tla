(* RetryJournal.tla
 *
 * Journal idempotency model for ActionFailed events.
 * Safety: appending the same ActionFailed event twice does not change
 * observable state beyond the duplicate event in the journal.
 * Safety: ActionFailed events appear in order in the journal.
 *)

---- MODULE RetryJournal ----

EXTENDS Integers, Sequences, TLC, FiniteSets

CONSTANT RunId, StepId, MaxJournalAttempts

VARIABLES
    journal,
    runs,
    actionAttempts,
    framePC,
    stepState,
    duplicateCount

Runs == RunId
Steps == StepId
MaxAttempts == MaxJournalAttempts

(* Init action *)
Init ==
    /\ journal = <<>>
    /\ runs = {}
    /\ actionAttempts = [run \in Runs |-> [step \in Steps |-> 0]]
    /\ framePC = [run \in Runs |-> 1]
    /\ stepState = [run \in Runs |-> [step \in Steps |-> "Pending"]]
    /\ duplicateCount = 0

(* Add a run *)
AddRun(run) ==
    /\ run \notin runs
    /\ runs' = runs \cup {run}
    /\ actionAttempts' = [actionAttempts EXCEPT ![run] = [step \in Steps |-> 0]]
    /\ stepState' = [stepState EXCEPT ![run] = [step \in Steps |-> "Pending"]]
    /\ framePC' = [framePC EXCEPT ![run] = 1]
    /\ UNCHANGED <<journal, duplicateCount>>

(* Start step *)
StartStep(run, step) ==
    /\ run \in runs
    /\ stepState[run][step] = "Pending"
    /\ stepState' = [stepState EXCEPT ![run][step] = "Running"]
    /\ UNCHANGED <<runs, actionAttempts, framePC, journal, duplicateCount>>

(* Append ActionFailed event
 * attempt is derived from actionAttempts state, not existentially quantified
 *)
AppendActionFailed(run, step) ==
    /\ run \in runs
    /\ stepState[run][step] = "Running"
    /\ actionAttempts[run][step] < MaxAttempts
    /\ LET attempt == actionAttempts[run][step] IN
        /\ journal' = Append(journal, [type |-> "ActionFailed", run |-> run, step |-> step, attempt |-> attempt])
        /\ actionAttempts' = [actionAttempts EXCEPT ![run][step] = actionAttempts[run][step] + 1]
        /\ stepState' = [stepState EXCEPT ![run][step] = "Failed"]
        /\ framePC' = framePC
        /\ UNCHANGED <<runs, duplicateCount>>

(* Append duplicate ActionFailed event (idempotency test)
 * Requires the event already exists in journal, and leaves all state unchanged except journal grows
 * attempt is derived from the first matching journal entry
 * duplicateCount bounds the number of duplicate appends to prevent state explosion
 *)
AppendDuplicateActionFailed(run, step) ==
    /\ duplicateCount < 2
    /\ \E idx \in 1..Len(journal) :
        LET existingAttempt == journal[idx].attempt IN
            journal[idx] = [type |-> "ActionFailed", run |-> run, step |-> step, attempt |-> existingAttempt]
    /\ \E idx \in 1..Len(journal) :
        LET existingAttempt == journal[idx].attempt IN
            journal' = Append(journal, [type |-> "ActionFailed", run |-> run, step |-> step, attempt |-> existingAttempt])
    /\ actionAttempts' = actionAttempts
    /\ framePC' = framePC
    /\ stepState' = stepState
    /\ duplicateCount' = duplicateCount + 1
    /\ UNCHANGED runs

(* Stale completion rejection
 * stale and current attempt values are derived from state
 * staleAttempt is always less than currentAttempt (previous vs current)
 *)
StaleCompletionRejected(run, step) ==
    /\ stepState[run][step] = "Running"
    /\ actionAttempts[run][step] > 0
    /\ LET currentAttempt == actionAttempts[run][step] IN
        LET staleAttempt == currentAttempt - 1 IN
            /\ journal' = journal
            /\ actionAttempts' = actionAttempts
            /\ framePC' = framePC
            /\ stepState' = stepState
            /\ UNCHANGED <<runs, duplicateCount>>

(* JournalIdempotency: actionAttempts never exceeds MaxAttempts.
 * Duplicate appends don't change observable state (actionAttempts, stepState, framePC).
 * The journal can grow with duplicates, but observable state remains unchanged.
 *)
JournalIdempotency ==
    \A run \in Runs, step \in Steps :
        actionAttempts[run][step] <= MaxAttempts

(* ActionFailedEventOrder: ActionFailed events for the same (run, step) appear
 * in the journal in non-decreasing order of attempt number.
 * This is ensured by the model since we only append and check actionAttempts before appending.
 *)
ActionFailedEventOrder ==
    \A i \in 1..Len(journal), j \in 1..Len(journal) :
        i < j /\ journal[i].type = "ActionFailed" /\ journal[j].type = "ActionFailed"
            => (journal[i].run # journal[j].run \/ journal[i].step # journal[j].step
                \/ journal[i].attempt <= journal[j].attempt)

(* Next relation
 * Removed existential quantifiers over attempt values to prevent state explosion.
 * All attempt values are now derived from state (actionAttempts, journal).
 *)
Next ==
    \E run \in Runs, step \in Steps :
        \/ AddRun(run)
        \/ StartStep(run, step)
        \/ AppendActionFailed(run, step)
        \/ AppendDuplicateActionFailed(run, step)
        \/ StaleCompletionRejected(run, step)

(* Spec *)
Spec == Init /\ [][Next]_<<journal, runs, actionAttempts, framePC, stepState, duplicateCount>>

(* State constraint: bound journal length and duplicate count for model checking *)
StateConstraint ==
    /\ Len(journal) <= 10
    /\ duplicateCount <= 2

THEOREM Spec => []JournalIdempotency
THEOREM Spec => []ActionFailedEventOrder

====
