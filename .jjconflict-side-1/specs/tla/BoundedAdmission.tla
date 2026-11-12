(* BoundedAdmission.tla
 *
 * Invariant: A run is not admitted unless its aggregate budget was verified and
 * resources were reserved. This ensures boundedness guarantees before run starts.
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
    pending_admission,
    verified_budget,
    rejected_budget

ResourceReservation == [slots: 0..MaxSlotsPerRun, actions: 0..MaxActionsPerRun, memory_bytes: 0..10]

Init ==
    /\ admitted_runs = {}
    /\ shard_runs = [shard \in ShardId |-> {}]
    /\ reserved_resources = [run \in RunId |-> [slots |-> 0, actions |-> 0, memory_bytes |-> 0]]
    /\ pending_admission = {}
    /\ verified_budget = {}
    /\ rejected_budget = {}

RequestAdmission(run, shard, reservation) ==
    /\ run \notin admitted_runs
    /\ run \notin rejected_budget
    /\ reservation.slots > 0
    /\ reservation.actions > 0
    /\ reservation.slots <= MaxSlotsPerRun
    /\ reservation.actions <= MaxActionsPerRun
    /\ pending_admission' = pending_admission \cup {<<run, shard, reservation>>}
    /\ verified_budget' = verified_budget \cup {run}
    /\ UNCHANGED <<admitted_runs, shard_runs, reserved_resources, rejected_budget>>

RejectUnverifiedBudget(run) ==
    /\ run \notin admitted_runs
    /\ run \notin verified_budget
    /\ rejected_budget' = rejected_budget \cup {run}
    /\ UNCHANGED <<admitted_runs, shard_runs, reserved_resources, pending_admission, verified_budget>>

RetryRejectedBudget(run) ==
    /\ run \in rejected_budget
    /\ run \notin admitted_runs
    /\ rejected_budget' = rejected_budget \ {run}
    /\ verified_budget' = verified_budget \ {run}
    /\ UNCHANGED <<admitted_runs, shard_runs, reserved_resources, pending_admission>>

AdmitRun(run, shard, reservation) ==
    /\ <<run, shard, reservation>> \in pending_admission
    /\ run \in verified_budget
    /\ run \notin rejected_budget
    /\ Cardinality(shard_runs[shard]) < MaxRunsPerShard
    /\ reservation.slots <= MaxSlotsPerRun
    /\ reservation.actions <= MaxActionsPerRun
    /\ admitted_runs' = admitted_runs \cup {run}
    /\ shard_runs' = [shard_runs EXCEPT ![shard] = shard_runs[shard] \cup {run}]
    /\ reserved_resources' = [reserved_resources EXCEPT ![run] = reservation]
    /\ pending_admission' = pending_admission \ {<<run, shard, reservation>>}
    /\ UNCHANGED <<verified_budget, rejected_budget>>

RejectAdmission(run, shard, reservation) ==
    /\ <<run, shard, reservation>> \in pending_admission
    /\ run \notin admitted_runs
    /\ pending_admission' = pending_admission \ {<<run, shard, reservation>>}
    /\ rejected_budget' = rejected_budget \cup {run}
    /\ UNCHANGED <<admitted_runs, shard_runs, reserved_resources, verified_budget>>

RunCompleted(run) ==
    /\ run \in admitted_runs
    /\ admitted_runs' = admitted_runs \ {run}
    /\ shard_runs' = [shard \in ShardId |-> shard_runs[shard] \ {run}]
    /\ reserved_resources' = [r \in RunId |-> IF r = run THEN [slots |-> 0, actions |-> 0, memory_bytes |-> 0] ELSE reserved_resources[r]]
    /\ UNCHANGED <<pending_admission, verified_budget, rejected_budget>>

NoRunAdmittedWithoutReservation ==
    \A run \in admitted_runs :
        reserved_resources[run].slots > 0 /\
        reserved_resources[run].actions > 0

ShardCapacityBounded ==
    \A shard \in ShardId :
        Cardinality(shard_runs[shard]) <= MaxRunsPerShard

NoRunAdmittedWithoutVerifiedBudget ==
    \A run \in admitted_runs :
        run \in verified_budget /\ run \notin rejected_budget

AdmittedResourcesArePositive ==
    \A run \in admitted_runs :
        reserved_resources[run].slots > 0 /\ reserved_resources[run].actions > 0

Next ==
    \/ \E run \in RunId, shard \in ShardId, reservation \in ResourceReservation :
           \/ RequestAdmission(run, shard, reservation)
           \/ AdmitRun(run, shard, reservation)
           \/ RejectAdmission(run, shard, reservation)
    \/ \E run \in RunId :
           \/ RejectUnverifiedBudget(run)
           \/ RetryRejectedBudget(run)
           \/ RunCompleted(run)

Spec == Init /\ [][Next]_<<admitted_runs, shard_runs, reserved_resources, pending_admission, verified_budget, rejected_budget>>

StateConstraint ==
    /\ Cardinality(pending_admission) <= 2
    /\ Cardinality(admitted_runs) <= 2
    /\ Cardinality(verified_budget) <= 2
    /\ Cardinality(rejected_budget) <= 2

THEOREM Spec => []NoRunAdmittedWithoutReservation
THEOREM Spec => []ShardCapacityBounded
THEOREM Spec => []NoRunAdmittedWithoutVerifiedBudget
THEOREM Spec => []AdmittedResourcesArePositive

====
