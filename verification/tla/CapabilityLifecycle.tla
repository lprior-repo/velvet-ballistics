---- MODULE CapabilityLifecycle ----
EXTENDS Naturals, FiniteSets

\* Obligations: VB-REPLAY-006, VB-REPLAY-007.

CONSTANTS Machines, Capabilities

VARIABLES held, accessLog

vars == <<held, accessLog>>

HeldCaps == UNION {held[m] : m \in Machines}

CapabilityUniqueOwner ==
  \A c \in Capabilities : Cardinality({m \in Machines : c \in held[m]}) <= 1

ValidCapabilityAccess ==
  \A e \in accessLog : e.cap \in held[e.machine]

Init ==
  /\ held = [m \in Machines |-> {}]
  /\ accessLog = {}

AcquireCapability ==
  \E m \in Machines, c \in Capabilities :
    /\ c \notin HeldCaps
    /\ held' = [held EXCEPT ![m] = @ \cup {c}]
    /\ UNCHANGED accessLog

ReleaseCapability ==
  \E m \in Machines :
    \E c \in held[m] :
      /\ \A e \in accessLog : e.cap # c
      /\ held' = [held EXCEPT ![m] = @ \ {c}]
      /\ UNCHANGED accessLog

AccessCapability ==
  \E m \in Machines :
    \E c \in held[m] :
      /\ accessLog' = accessLog \cup {[machine |-> m, cap |-> c]}
      /\ UNCHANGED held

Next == AcquireCapability \/ ReleaseCapability \/ AccessCapability

Spec == Init /\ [][Next]_vars

====
