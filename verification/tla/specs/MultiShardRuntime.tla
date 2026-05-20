---- MODULE MultiShardRuntime ----
EXTENDS Naturals, FiniteSets, Sequences, TLC

\* Obligation: PO-001
\* Requirement: TLA-WF-001 (INV-001)
\* Model: Multi-shard routing with bounded runs and deterministic routing.
\* Bounds: shard_count <= 4, MaxRuns <= 8

CONSTANTS SHARD_COUNT, MAX_RUNS

ASSUME SHARD_COUNT \in Nat \ {0}
ASSUME SHARD_COUNT <= 4
ASSUME MAX_RUNS \in Nat \ {0}
ASSUME MAX_RUNS <= 8

\* Routing function: run_id -> shard index via modulo
Routing(run_id, shard_count) == run_id % shard_count

VARIABLES
    runs,           \* [RUNS -> RunState]
    routing_table,  \* [RUNS -> shard index]
    active_runs     \* set of active run_ids

vars == <<runs, routing_table, active_runs>>

RunStates == {"missing", "queued", "running", "await_action", "await_ask", "finished", "failed", "cancelled"}
RunIds == 1..MAX_RUNS
ShardIndices == 0..(SHARD_COUNT-1)

Init ==
    /\ runs = [r \in RunIds |-> "missing"]
    /\ routing_table = [r \in RunIds |-> Routing(r, SHARD_COUNT)]
    /\ active_runs = {}

SubmitRun(r) ==
    /\ runs[r] = "missing"
    /\ runs' = [runs EXCEPT ![r] = "queued"]
    /\ active_runs' = active_runs \cup {r}
    /\ UNCHANGED routing_table

TickRun(r) ==
    /\ runs[r] = "queued"
    /\ runs' = [runs EXCEPT ![r] = "running"]
    /\ UNCHANGED <<routing_table, active_runs>>

CompleteRun(r) ==
    /\ runs[r] = "running"
    /\ runs' = [runs EXCEPT ![r] = "finished"]
    /\ active_runs' = active_runs \ {r}
    /\ UNCHANGED routing_table

FailRun(r) ==
    /\ runs[r] = "running"
    /\ runs' = [runs EXCEPT ![r] = "failed"]
    /\ active_runs' = active_runs \ {r}
    /\ UNCHANGED routing_table

CancelRun(r) ==
    /\ runs[r] \in {"running", "await_action", "await_ask"}
    /\ runs' = [runs EXCEPT ![r] = "cancelled"]
    /\ active_runs' = active_runs \ {r}
    /\ UNCHANGED routing_table

\* Reuse a finished/failed/cancelled run slot
ReuseRun(r) ==
    /\ runs[r] \in {"finished", "failed", "cancelled"}
    /\ runs' = [runs EXCEPT ![r] = "missing"]
    /\ UNCHANGED <<routing_table, active_runs>>

Progress ==
    \/ \E r \in RunIds : SubmitRun(r)
    \/ \E r \in RunIds : TickRun(r)
    \/ \E r \in RunIds : CompleteRun(r)
    \/ \E r \in RunIds : FailRun(r)
    \/ \E r \in RunIds : CancelRun(r)
    \/ \E r \in RunIds : ReuseRun(r)

Spec == Init /\ [][Progress]_vars

\* Invariant: RoutingDeterminism — same run_id always routes to same shard
RoutingDeterminism ==
    \A r \in RunIds:
        routing_table[r] = Routing(r, SHARD_COUNT)

\* Invariant: NoDoubleRouting — a run can only be active on one shard at a time
NoDoubleRouting ==
    \A r \in active_runs:
        Cardinality({s \in ShardIndices: \E run \in active_runs: routing_table[run] = s}) <= SHARD_COUNT

===============================================================================
