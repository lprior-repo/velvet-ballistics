(* RecoveryReplay.tla
 *
 * Invariant: A non-idempotent action is never re-executed during replay.
 * This is the core safety property for crash recovery.
 *)

---- MODULE RecoveryReplay ----

EXTENDS Integers, Sequences, TLC, FiniteSets

CONSTANT RunId, StepId, ActionId, Attempt

VARIABLES
    journal,
    replay_index,
    idempotent_actions

IdempotencyPolicy == {"DeterministicPure", "IdempotentExternal", "AtLeastOnceExternal"}

ActionEvent == [type: {"ActionScheduled", "ActionCompleted", "ActionFailed"}, run: RunId, step: StepId, action: ActionId, attempt: Attempt, policy: IdempotencyPolicy]

Init ==
    /\ journal = <<>>
    /\ replay_index = 0
    /\ idempotent_actions = {}

ScheduleAction(run, step, action, attempt, policy) ==
    /\ journal' = Append(journal, [type |-> "ActionScheduled", run |-> run, step |-> step, action |-> action, attempt |-> attempt, policy |-> policy])
    /\ UNCHANGED <<replay_index, idempotent_actions>>

CompleteAction(run, step, action, attempt) ==
    /\ journal' = Append(journal, [type |-> "ActionCompleted", run |-> run, step |-> step, action |-> action, attempt |-> attempt])
    /\ UNCHANGED <<replay_index, idempotent_actions>>

FailAction(run, step, action, attempt) ==
    /\ journal' = Append(journal, [type |-> "ActionFailed", run |-> run, step |-> step, action |-> action, attempt |-> attempt])
    /\ UNCHANGED <<replay_index, idempotent_actions>>

ReplayNext ==
    /\ replay_index < Len(journal)
    /\ LET event == journal[replay_index + 1] IN
        /\ IF event.type = "ActionScheduled" THEN
            \/ (event.policy \in {"DeterministicPure", "IdempotentExternal"} /\ UNCHANGED idempotent_actions)
            \/ (event.policy = "AtLeastOnceExternal" /\ idempotent_actions' = idempotent_actions \cup {<<event.run, event.step, event.action, event.attempt>>})
        ELSE IF event.type = "ActionCompleted" THEN
            /\ <<event.run, event.step, event.action, event.attempt>> \notin idempotent_actions
        ELSE
            TRUE
    /\ replay_index' = replay_index + 1
    /\ UNCHANGED journal

IsIdempotent(event) ==
    event.type = "ActionCompleted" \/ event.type = "ActionFailed"

NoDuplicateNonIdempotent ==
    \A i \in 1..Len(journal) :
        \A j \in 1..Len(journal) :
            i /= j /\
            journal[i].type = "ActionScheduled" /\
            journal[j].type = "ActionScheduled" /\
            journal[i].run = journal[j].run /\
            journal[i].step = journal[j].step /\
            journal[i].action = journal[j].action /\
            journal[i].attempt = journal[j].attempt /\
            journal[i].policy /= "DeterministicPure" /\
            journal[i].policy /= "IdempotentExternal"
            => FALSE

ReplaySafe ==
    \A run \in RunId, step \in StepId, action \in ActionId, attempt \in Attempt :
        \A i, j \in 1..Len(journal) :
            i < j /\
            journal[i].type = "ActionCompleted" /\
            journal[i].run = run /\
            journal[i].step = step /\
            journal[i].action = action /\
            journal[i].attempt = attempt
            => journal[j].type /= "ActionScheduled" \/ journal[j].attempt /= attempt

Spec == Init /\ [][ReplayNext]_<<journal, replay_index, idempotent_actions>>

THEOREM Spec => []NoDuplicateNonIdempotent
THEOREM Spec => []ReplaySafe

====
