(* ShardOwnership.tla
 *
 * Invariant: A run has exactly one shard owner.
 * This ensures no two shards can mutate the same run concurrently.
 *)

---- MODULE ShardOwnership ----

EXTENDS Integers, Sequences, TLC, FiniteSets

CONSTANT RunId, ShardId

VARIABLES
    run_owner,
    shard_runs,
    pending_transfers

Init ==
    /\ run_owner = [run \in RunId |-> Nil]
    /\ shard_runs = [shard \in ShardId |-> {}]
    /\ pending_transfers = {}

AssignShard(run, shard) ==
    /\ run_owner[run] = Nil
    /\ run_owner' = [run_owner EXCEPT ![run] = shard]
    /\ shard_runs' = [shard_runs EXCEPT ![shard] = shard_runs[shard] \cup {run}]
    /\ UNCHANGED pending_transfers

InitiateTransfer(run, new_shard) ==
    /\ run_owner[run] /= Nil
    /\ run_owner[run] /= new_shard
    /\ pending_transfers' = pending_transfers \cup {<<run, new_shard>>}
    /\ UNCHANGED <<run_owner, shard_runs>>

CompleteTransfer(run, new_shard) ==
    /\ <<run, new_shard>> \in pending_transfers
    /\ LET old_shard == run_owner[run] IN
        /\ run_owner' = [run_owner EXCEPT ![run] = new_shard]
        /\ shard_runs' = [shard_runs EXCEPT ![old_shard] = shard_runs[old_shard] \ {run}, ![new_shard] = shard_runs[new_shard] \cup {run}]
        /\ pending_transfers' = pending_transfers \ {<<run, new_shard>>}

SingleOwner ==
    \A run \in RunId :
        \E shard \in ShardId :
            run_owner[run] = shard /\
            \A other \in ShardId \ {shard} :
                run \notin shard_runs[other]

NoPendingTransferConflict ==
    \A <<run, new_shard>> \in pending_transfers :
        run_owner[run] /= new_shard /\
        \A shard \in ShardId \ {new_shard} :
            run \notin shard_runs[shard] \ {run}

RunOnExactlyOneShard ==
    \A run \in RunId :
        \A shard1, shard2 \in ShardId :
            run \in shard_runs[shard1] /\ run \in shard_runs[shard2]
            => shard1 = shard2

Next ==
    \E run \in RunId, shard \in ShardId :
        \/ AssignShard(run, shard)
        \/ InitiateTransfer(run, shard)
        \/ CompleteTransfer(run, shard)

Spec == Init /\ [][Next]_<<run_owner, shard_runs, pending_transfers>>

THEOREM Spec => []SingleOwner
THEOREM Spec => []NoPendingTransferConflict
THEOREM Spec => []RunOnExactlyOneShard

====
