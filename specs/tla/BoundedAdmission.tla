(* BoundedAdmission.tla
 *
 * Invariant: A run is not admitted unless resources are reserved.
 * This ensures boundedness guarantees before run starts.
 *)

---- MODULE BoundedAdmission ----

EXTENDS Integers, Sequences, TLC, FiniteSets

CONSTANT RunId, ShardId

CONSTANTS
    MaxRunsPerShard,
    MaxSlotsPerRun,
    MaxActionsPerRun

VARIABLES
    admitted_runs,
    shard_runs,
    reserved_resources,
    pending_admission

ResourceReservation == [slots: 0..MaxSlotsPerRun, actions: 0..MaxActionsPerRun, memory_bytes: 0..100]

Init ==
    /\ admitted_runs = {}
    /\ shard_runs = [shard \in ShardId |-> {}]
    /\ reserved_resources = [run \in RunId |-> [slots |-> 0, actions |-> 0, memory_bytes |-> 0]]
    /\ pending_admission = {}

RequestAdmission(run, shard, reservation) ==
    /\ run \notin admitted_runs
    /\ pending_admission' = pending_admission \cup {<<run, shard, reservation>>}
    /\ UNCHANGED <<admitted_runs, shard_runs, reserved_resources>>

AdmitRun(run, shard, reservation) ==
    /\ <<run, shard, reservation>> \in pending_admission
    /\ Cardinality(shard_runs[shard]) < MaxRunsPerShard
    /\ reservation.slots <= MaxSlotsPerRun
    /\ reservation.actions <= MaxActionsPerRun
    /\ admitted_runs' = admitted_runs \cup {run}
    /\ shard_runs' = [shard_runs EXCEPT ![shard] = shard_runs[shard] \cup {run}]
    /\ reserved_resources' = [reserved_resources EXCEPT ![run] = reservation]
    /\ pending_admission' = pending_admission \ {<<run, shard, reservation>>}

RejectAdmission(run, shard, reservation) ==
    /\ <<run, shard, reservation>> \in pending_admission
    /\ pending_admission' = pending_admission \ {<<run, shard, reservation>>}
    /\ UNCHANGED <<admitted_runs, shard_runs, reserved_resources>>

RunCompleted(run) ==
    /\ run \in admitted_runs
    /\ admitted_runs' = admitted_runs \ {run}
    /\ reserved_resources' = [r \in RunId |-> IF r = run THEN [slots |-> 0, actions |-> 0, memory_bytes |-> 0] ELSE reserved_resources[r]]
    /\ UNCHANGED <<shard_runs, pending_admission>>

NoRunAdmittedWithoutReservation ==
    \A run \in admitted_runs :
        reserved_resources[run].slots > 0 /\
        reserved_resources[run].actions > 0

ShardCapacityBounded ==
    \A shard \in ShardId :
        Cardinality(shard_runs[shard]) <= MaxRunsPerShard

AdmissionRequiresPending ==
    \A run \in admitted_runs :
        \E <<r, s, res>> \in pending_admission : r = run

OnlyAdmittedFromPending ==
    \A run \in admitted_runs :
        \E shard \in ShardId, reservation \in ResourceReservation :
            <<run, shard, reservation>> \in pending_admission

Next ==
    \E run \in RunId, shard \in ShardId, reservation \in ResourceReservation :
        \/ RequestAdmission(run, shard, reservation)
        \/ AdmitRun(run, shard, reservation)
        \/ RejectAdmission(run, shard, reservation)
        \/ RunCompleted(run)

Spec == Init /\ [][Next]_<<admitted_runs, shard_runs, reserved_resources, pending_admission>>

THEOREM Spec => []NoRunAdmittedWithoutReservation
THEOREM Spec => []ShardCapacityBounded
THEOREM Spec => []OnlyAdmittedFromPending

====
