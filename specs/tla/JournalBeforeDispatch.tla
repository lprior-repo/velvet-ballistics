(* JournalBeforeDispatch.tla
 *
 * Invariant: An action is never dispatched before ActionScheduled is committed.
 * This is the core safety property for durable execution.
 *)

---- MODULE JournalBeforeDispatch ----

EXTENDS Integers, Sequences, TLC

CONSTANT RunId, StepId, ActionId, Attempt

VARIABLES
    journal,
    dispatched,
    pending_dispatch

Journal == [type: {"RunSubmitted", "ActionScheduled", "ActionCompleted", "ActionFailed"}, run: RunId, step: StepId, action: ActionId, attempt: Attempt]

Init == 
    /\ journal = <<>>
    /\ dispatched = {}
    /\ pending_dispatch = {}

ActionScheduled(run, step, action, attempt) ==
    /\ journal' = Append(journal, [type |-> "ActionScheduled", run |-> run, step |-> step, action |-> action, attempt |-> attempt])
    /\ pending_dispatch' = pending_dispatch \cup {<<run, step, action, attempt>>}

Dispatch(run, step, action, attempt) ==
    /\ <<run, step, action, attempt>> \in pending_dispatch
    /\ dispatched' = dispatched \cup {<<run, step, action, attempt>>}
    /\ UNCHANGED <<journal, pending_dispatch>>

ActionCompleted(run, step, action, attempt, success) ==
    /\ journal' = Append(journal, [type |-> IF success THEN "ActionCompleted" ELSE "ActionFailed", run |-> run, step |-> step, action |-> action, attempt |-> attempt])
    /\ UNCHANGED <<dispatched, pending_dispatch>>

\* Safety property: dispatch only after scheduled
DispatchSafety ==
    \A <<run, step, action, attempt>> \in dispatched :
        \E event \in DOMAIN journal :
            journal[event].type = "ActionScheduled" /\
            journal[event].run = run /\
            journal[event].step = step /\
            journal[event].action = action /\
            journal[event].attempt = attempt

\* Stronger: dispatch only after scheduled AND committed
DispatchBeforeCommit ==
    \A <<run, step, action, attempt>> \in dispatched :
        \E idx \in DOMAIN journal :
            journal[idx].type = "ActionScheduled" /\
            journal[idx].run = run /\
            journal[idx].step = step /\
            journal[idx].action = action /\
            journal[idx].attempt = attempt /\
            idx < Len(journal)

Next ==
    \E run \in RunId, step \in StepId, action \in ActionId, attempt \in Attempt :
        \/ ActionScheduled(run, step, action, attempt)
        \/ Dispatch(run, step, action, attempt)
        \/ ActionCompleted(run, step, action, attempt, TRUE)
        \/ ActionCompleted(run, step, action, attempt, FALSE)

Spec == Init /\ [][Next]_<<journal, dispatched, pending_dispatch>>

THEOREM Spec => []DispatchSafety
THEOREM Spec => []DispatchBeforeCommit

====
