---- MODULE ConcurrencyControl ----
EXTENDS Naturals, FiniteSets, Sequences

\* Obligations: VB-CONC-001 through VB-CONC-005.

CONSTANTS Shards, Frames, Resources, Machines, MaxQueue

VARIABLES framePool, globalLock, waitQueue

vars == <<framePool, globalLock, waitQueue>>

OwnedFrames == UNION {framePool[s] : s \in Shards}

SingleShardOwner ==
  \A f \in Frames : Cardinality({s \in Shards : f \in framePool[s]}) <= 1

FramePoolBounded == OwnedFrames \subseteq Frames

NoCrossShardAlias ==
  \A r \in Resources : globalLock[r] = "none" \/ globalLock[r] \in Machines

BoundedState == Len(waitQueue) <= MaxQueue

Init ==
  /\ framePool = [s \in Shards |-> {}]
  /\ globalLock = [r \in Resources |-> "none"]
  /\ waitQueue = <<>>

AcquireFrame ==
  \E s \in Shards, f \in Frames :
    /\ f \notin OwnedFrames
    /\ framePool' = [framePool EXCEPT ![s] = @ \cup {f}]
    /\ UNCHANGED <<globalLock, waitQueue>>

ReleaseFrame ==
  \E s \in Shards :
    \E f \in framePool[s] :
      /\ framePool' = [framePool EXCEPT ![s] = @ \ {f}]
      /\ UNCHANGED <<globalLock, waitQueue>>

MigrateFrame ==
  \E from \in Shards, to \in Shards :
    \E f \in framePool[from] :
      /\ from # to
      /\ framePool' = [framePool EXCEPT ![from] = @ \ {f}, ![to] = @ \cup {f}]
      /\ UNCHANGED <<globalLock, waitQueue>>

AcquireLock ==
  \E m \in Machines, r \in Resources :
    /\ globalLock[r] = "none"
    /\ \A held \in Resources : globalLock[held] = "none"
    /\ globalLock' = [globalLock EXCEPT ![r] = m]
    /\ UNCHANGED <<framePool, waitQueue>>

ReleaseLock ==
  \E r \in Resources :
    /\ globalLock[r] # "none"
    /\ globalLock' = [globalLock EXCEPT ![r] = "none"]
    /\ UNCHANGED <<framePool, waitQueue>>

EnqueueWait ==
  \E m \in Machines :
    /\ Len(waitQueue) = 0
    /\ waitQueue' = Append(waitQueue, m)
    /\ UNCHANGED <<framePool, globalLock>>

DequeueWait ==
  /\ Len(waitQueue) > 0
  /\ waitQueue' = Tail(waitQueue)
  /\ UNCHANGED <<framePool, globalLock>>

Next == AcquireFrame \/ ReleaseFrame \/ MigrateFrame \/ AcquireLock \/ ReleaseLock \/ EnqueueWait \/ DequeueWait

Spec == Init /\ [][Next]_vars /\ WF_vars(AcquireFrame) /\ WF_vars(ReleaseFrame) /\ WF_vars(ReleaseLock) /\ WF_vars(DequeueWait)

NoDeadlockOnLocks == []<>(\A r \in Resources : globalLock[r] = "none")
NoStarvation == []<>(OwnedFrames # {})
LockNoStarvation == []<>(Len(waitQueue) = 0)

====
