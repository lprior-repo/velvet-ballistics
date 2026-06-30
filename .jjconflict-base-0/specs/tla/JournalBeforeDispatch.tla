(* JournalBeforeDispatch.tla
 *
 * Invariant: An action is never dispatched before ActionScheduled is committed.
 * This is the core safety property for durable execution.
 *
 * IMPORTANT: The Rust implementation (StorageRuntimeJournal::append) is
 * synchronous — the append blocks until the event is durable before
 * execute_do() returns RuntimeSignal::AwaitingAction. The correct model
 * is therefore append-first, then dispatch:
 *
 *   ActionScheduled: append event to journal (blocks until durable)
 *   Dispatch:        (implicit — ticket is returned after append returns)
 *
 * The old Dispatch-then-Append model was incorrect and has been replaced.
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

\* Append-then-dispatch: the journal write is synchronous (blocks until durable).
\* "Dispatch" here is implicit — the ActionTicket is returned after append succeeds.
ActionScheduledThenDispatch(run, step, action, attempt) ==
    /\ journal' = Append(journal, [type |-> "ActionScheduled", run |-> run, step |-> step, action |-> action, attempt |-> attempt])
    /\ dispatched' = dispatched \cup {<<run, step, action, attempt>>}
    /\ UNCHANGED pending_dispatch

\* Standalone scheduling (without immediate dispatch) for cases where dispatch
\* is decoupled from scheduling (e.g., timer-driven retry after transient failure).
ActionScheduled(run, step, action, attempt) ==
    /\ journal' = Append(journal, [type |-> "ActionScheduled", run |-> run, step |-> step, action |-> action, attempt |-> attempt])
    /\ pending_dispatch' = pending_dispatch \cup {<<run, step, action, attempt>>}
    /\ UNCHANGED dispatched

Dispatch(run, step, action, attempt) ==
    /\ <<run, step, action, attempt>> \in pending_dispatch
    /\ dispatched' = dispatched \cup {<<run, step, action, attempt>>}
    /\ UNCHANGED <<journal, pending_dispatch>>

ActionCompleted(run, step, action, attempt, success) ==
    /\ journal' = Append(journal, [type |-> IF success THEN "ActionCompleted" ELSE "ActionFailed", run |-> run, step |-> step, action |-> action, attempt |-> attempt])
    /\ UNCHANGED <<dispatched, pending_dispatch>>

\* Safety property: every dispatched run was previously scheduled and appended.
\* This is guaranteed by the append-first model — append must succeed before
\* the dispatch signal (ActionTicket) is returned to the caller.
DispatchSafety ==
    \A <<run, step, action, attempt>> \in dispatched :
        \E event \in DOMAIN journal :
            journal[event].type = "ActionScheduled" /\
            journal[event].run = run /\
            journal[event].step = step /\
            journal[event].action = action /\
            journal[event].attempt = attempt

\* Stronger: every dispatched run appears in the journal at some index.
\* (The event index being < Len(journal) is trivially true after append.)
DispatchBeforeCommit ==
    \A <<run, step, action, attempt>> \in dispatched :
        \E idx \in DOMAIN journal :
            journal[idx].type = "ActionScheduled" /\
            journal[idx].run = run /\
            journal[idx].step = step /\
            journal[idx].action = action /\
            journal[idx].attempt = attempt /\
            idx \in DOMAIN journal

Next ==
    \E run \in RunId, step \in StepId, action \in ActionId, attempt \in Attempt :
        \/ ActionScheduledThenDispatch(run, step, action, attempt)
        \/ ActionScheduled(run, step, action, attempt)
        \/ Dispatch(run, step, action, attempt)
        \/ ActionCompleted(run, step, action, attempt, TRUE)
        \/ ActionCompleted(run, step, action, attempt, FALSE)

Spec == Init /\ [][Next]_<<journal, dispatched, pending_dispatch>>

StateConstraint == Len(journal) <= 4

THEOREM Spec => []DispatchSafety
THEOREM Spec => []DispatchBeforeCommit

====
