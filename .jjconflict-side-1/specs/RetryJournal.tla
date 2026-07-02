(* RetryJournal.tla
 *
 * Rust-aligned storage idempotency model for ActionFailed events.
 *
 * Rust refinement target:
 * - vb_storage::journal::internal::append_unpersisted keys events by (run, seq)
 *   and rejects any existing key.
 * - append_queued_unpersisted accepts an exact duplicate as an idempotent no-op,
 *   but rejects same-key/different-payload writes.
 *)

---- MODULE RetryJournal ----

EXTENDS Integers, Sequences, TLC, FiniteSets

CONSTANT RunId, SeqId, StepId, AttemptId

VARIABLES
    events,
    eventKeys,
    lastResult,
    attemptedEvent,
    insertedCount,
    acceptedDuplicateCount,
    rejectedDuplicateCount,
    appendHistory

Runs == RunId
Seqs == SeqId
Steps == StepId
Attempts == AttemptId

RunSeqKeys == Runs \X Seqs
ResultKinds == {"None", "Inserted", "DuplicateExactAccepted", "DuplicateRejected"}

NullEvent == [type |-> "Null",
              run |-> CHOOSE r \in Runs : TRUE,
              seq |-> CHOOSE s \in Seqs : TRUE,
              step |-> CHOOSE st \in Steps : TRUE,
              attempt |-> CHOOSE a \in Attempts : TRUE]

ActionFailedEvents ==
    { [type |-> "ActionFailed", run |-> r, seq |-> s, step |-> st, attempt |-> a] :
        r \in Runs, s \in Seqs, st \in Steps, a \in Attempts }

EventDomain == ActionFailedEvents \cup {NullEvent}

EventKey(e) == <<e.run, e.seq>>

vars == <<events, eventKeys, lastResult, attemptedEvent,
          insertedCount, acceptedDuplicateCount, rejectedDuplicateCount, appendHistory>>

Init ==
    /\ eventKeys = {}
    /\ events = [k \in RunSeqKeys |-> NullEvent]
    /\ lastResult = "None"
    /\ attemptedEvent = NullEvent
    /\ insertedCount = 0
    /\ acceptedDuplicateCount = 0
    /\ rejectedDuplicateCount = 0
    /\ appendHistory = <<>>

AppendUnpersisted(e) ==
    LET k == EventKey(e) IN
    /\ attemptedEvent' = e
    /\ IF k \notin eventKeys THEN
        /\ eventKeys' = eventKeys \cup {k}
        /\ events' = [events EXCEPT ![k] = e]
        /\ lastResult' = "Inserted"
        /\ insertedCount' = insertedCount + 1
        /\ appendHistory' = Append(appendHistory, e)
        /\ UNCHANGED <<acceptedDuplicateCount, rejectedDuplicateCount>>
       ELSE
        /\ eventKeys' = eventKeys
        /\ events' = events
        /\ lastResult' = "DuplicateRejected"
        /\ rejectedDuplicateCount' = rejectedDuplicateCount + 1
        /\ UNCHANGED <<insertedCount, acceptedDuplicateCount, appendHistory>>

AppendQueuedUnpersisted(e) ==
    LET k == EventKey(e) IN
    /\ attemptedEvent' = e
    /\ IF k \notin eventKeys THEN
        /\ eventKeys' = eventKeys \cup {k}
        /\ events' = [events EXCEPT ![k] = e]
        /\ lastResult' = "Inserted"
        /\ insertedCount' = insertedCount + 1
        /\ appendHistory' = Append(appendHistory, e)
        /\ UNCHANGED <<acceptedDuplicateCount, rejectedDuplicateCount>>
       ELSE IF events[k] = e THEN
        /\ eventKeys' = eventKeys
        /\ events' = events
        /\ lastResult' = "DuplicateExactAccepted"
        /\ acceptedDuplicateCount' = acceptedDuplicateCount + 1
        /\ UNCHANGED <<insertedCount, rejectedDuplicateCount, appendHistory>>
       ELSE
        /\ eventKeys' = eventKeys
        /\ events' = events
        /\ lastResult' = "DuplicateRejected"
        /\ rejectedDuplicateCount' = rejectedDuplicateCount + 1
        /\ UNCHANGED <<insertedCount, acceptedDuplicateCount, appendHistory>>

Next ==
    \E e \in ActionFailedEvents :
        \/ AppendUnpersisted(e)
        \/ AppendQueuedUnpersisted(e)

Spec == Init /\ [][Next]_vars

TypeOK ==
    /\ eventKeys \subseteq RunSeqKeys
    /\ events \in [RunSeqKeys -> EventDomain]
    /\ lastResult \in ResultKinds
    /\ attemptedEvent \in EventDomain
    /\ insertedCount \in Nat
    /\ acceptedDuplicateCount \in Nat
    /\ rejectedDuplicateCount \in Nat
    /\ appendHistory \in Seq(ActionFailedEvents)

StoredEventsHaveTheirOwnKey ==
    \A k \in eventKeys : EventKey(events[k]) = k

AbsentKeysAreNull ==
    \A k \in RunSeqKeys \ eventKeys : events[k] = NullEvent

InsertedCountEqualsStoredKeyCount ==
    insertedCount = Cardinality(eventKeys)

AcceptedQueuedDuplicateDoesNotInsert ==
    lastResult = "DuplicateExactAccepted" => insertedCount = Cardinality(eventKeys)

HistoryMatchesStoredKeys ==
    /\ Len(appendHistory) = Cardinality(eventKeys)
    /\ \A i, j \in 1..Len(appendHistory) :
        i # j => EventKey(appendHistory[i]) # EventKey(appendHistory[j])
    /\ \A i \in 1..Len(appendHistory) :
        events[EventKey(appendHistory[i])] = appendHistory[i]

SameStepAttemptDifferentSeqCanCoexist ==
    \A i, j \in 1..Len(appendHistory) :
        /\ appendHistory[i].run = appendHistory[j].run
        /\ appendHistory[i].step = appendHistory[j].step
        /\ appendHistory[i].attempt = appendHistory[j].attempt
        /\ appendHistory[i].seq # appendHistory[j].seq
        => /\ EventKey(appendHistory[i]) \in eventKeys
           /\ EventKey(appendHistory[j]) \in eventKeys
           /\ events[EventKey(appendHistory[i])] = appendHistory[i]
           /\ events[EventKey(appendHistory[j])] = appendHistory[j]

StateConstraint ==
    /\ acceptedDuplicateCount <= 1
    /\ rejectedDuplicateCount <= 1

THEOREM Spec => []TypeOK
THEOREM Spec => []StoredEventsHaveTheirOwnKey
THEOREM Spec => []AbsentKeysAreNull
THEOREM Spec => []InsertedCountEqualsStoredKeyCount
THEOREM Spec => []AcceptedQueuedDuplicateDoesNotInsert
THEOREM Spec => []HistoryMatchesStoredKeys
THEOREM Spec => []SameStepAttemptDifferentSeqCanCoexist

====
