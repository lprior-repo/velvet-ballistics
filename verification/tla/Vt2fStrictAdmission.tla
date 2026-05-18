---- MODULE Vt2fStrictAdmission ----
EXTENDS Naturals, Sequences, FiniteSets, TLC

CONSTANTS Runs, Digests, Policies, StoreModes, MaxQueue, MaxStep
VARIABLES runState, queue, accepted, policy, storeMode, errors, steps, heartbeat

vars == <<runState, queue, accepted, policy, storeMode, errors, steps, heartbeat>>
RunStates == {"missing", "rejected", "queued", "admitted"}
ErrorSet == {"missing_artifact", "pre_enqueue_rejection"}

Init ==
    /\ runState = [r \in Runs |-> "missing"]
    /\ queue = <<>>
    /\ accepted \in SUBSET Digests
    /\ policy \in Policies
    /\ storeMode \in StoreModes
    /\ errors = {}
    /\ steps = 0
    /\ heartbeat = FALSE

CanStep == steps < MaxStep
StrictLike == policy \in {"Strict", "Journaled"}
StoreAccepts(d) == storeMode = "AlwaysPresent" \/ (storeMode = "StorageBackedAccepted" /\ d \in accepted)

RelaxedSubmit(r, d) ==
    /\ CanStep /\ policy = "Relaxed" /\ runState[r] = "missing" /\ Len(queue) < MaxQueue
    /\ runState' = [runState EXCEPT ![r] = "queued"]
    /\ queue' = Append(queue, [run |-> r, digest |-> d])
    /\ accepted' = accepted /\ policy' = policy /\ storeMode' = storeMode
    /\ errors' = errors /\ steps' = steps + 1 /\ heartbeat' = heartbeat

StrictSubmitAccepted(r, d) ==
    /\ CanStep /\ StrictLike /\ StoreAccepts(d) /\ runState[r] = "missing" /\ Len(queue) < MaxQueue
    /\ runState' = [runState EXCEPT ![r] = "queued"]
    /\ queue' = Append(queue, [run |-> r, digest |-> d])
    /\ accepted' = accepted /\ policy' = policy /\ storeMode' = storeMode
    /\ errors' = errors /\ steps' = steps + 1 /\ heartbeat' = heartbeat

StrictRejectMissing(r, d) ==
    /\ CanStep /\ StrictLike /\ ~StoreAccepts(d) /\ runState[r] = "missing"
    /\ runState' = [runState EXCEPT ![r] = "rejected"]
    /\ queue' = queue
    /\ accepted' = accepted /\ policy' = policy /\ storeMode' = storeMode
    /\ errors' = errors \cup {"missing_artifact", "pre_enqueue_rejection"}
    /\ steps' = steps + 1 /\ heartbeat' = heartbeat

TickAccepted ==
    /\ CanStep /\ Len(queue) > 0 /\ runState[Head(queue).run] = "queued"
    /\ runState' = [runState EXCEPT ![Head(queue).run] = "admitted"]
    /\ queue' = Tail(queue)
    /\ accepted' = accepted /\ policy' = policy /\ storeMode' = storeMode
    /\ errors' = errors /\ steps' = steps + 1 /\ heartbeat' = heartbeat

Heartbeat ==
    /\ runState' = runState /\ queue' = queue /\ accepted' = accepted
    /\ policy' = policy /\ storeMode' = storeMode /\ errors' = errors
    /\ steps' = steps /\ heartbeat' = ~heartbeat

AdmissionProgress ==
    \/ \E r \in Runs : \E d \in Digests : RelaxedSubmit(r, d)
    \/ \E r \in Runs : \E d \in Digests : StrictSubmitAccepted(r, d)
    \/ \E r \in Runs : \E d \in Digests : StrictRejectMissing(r, d)
    \/ TickAccepted

Next ==
    \/ AdmissionProgress
    \/ Heartbeat

Spec == Init /\ [][Next]_vars /\ WF_vars(AdmissionProgress)

TypeOK ==
    /\ runState \in [Runs -> RunStates]
    /\ queue \in Seq([run : Runs, digest : Digests])
    /\ Len(queue) <= MaxQueue
    /\ accepted \subseteq Digests
    /\ policy \in Policies
    /\ storeMode \in StoreModes
    /\ errors \subseteq ErrorSet
    /\ steps \in 0..MaxStep
    /\ heartbeat \in BOOLEAN

StrictMissingRejectedBeforeEnqueue == StrictLike /\ storeMode = "Missing" => Len(queue) = 0
AcceptedDigestOnly == StrictLike /\ storeMode = "StorageBackedAccepted" => \A i \in 1..Len(queue) : queue[i].digest \in accepted
RelaxedSeparated == policy = "Relaxed" => "pre_enqueue_rejection" \notin errors
NoBypassAdmission == StrictLike /\ storeMode = "Missing" => \A r \in Runs : runState[r] # "admitted"
AlwaysPresentShardStoreSeparated == storeMode = "AlwaysPresent" /\ StrictLike => "missing_artifact" \notin errors

EverySubmitEventuallyAcceptedOrTypedRejectedWithinBounds ==
    <>(steps = MaxStep \/ \A r \in Runs : runState[r] \in {"rejected", "admitted"})

====
