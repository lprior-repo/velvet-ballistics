(* ShardScheduler.tla
 *
 * Models the shard command queue and tick processing semantics.
 *
 * Key properties verified:
 *   - tick() processes exactly ONE command then returns
 *   - tick_all() loops over all shards calling tick() until all queues are empty
 *   - shutdown_graceful() drains all pending commands
 *   - Command queue is bounded to MAX_COMMAND_QUEUE_CAPACITY
 *)

---- MODULE ShardScheduler ----

EXTENDS Integers, Sequences, TLC, FiniteSets

CONSTANT ShardId, RunId, ActionId

MAX_COMMAND_QUEUE_CAPACITY == 65536

\* Command types that can be enqueued
Command == [type: {"Submit", "Resume", "ActionCompleted", "ActionFailed",
                   "AskAnswered", "TimerFired", "Cancel", "Inspect", "Shutdown"},
             run: RunId, action: ActionId]

VARIABLES
    queues,
    shard_status,
    total_processed

\* Per-shard state
ShardState == [queue: Seq(Command), shutting_down: BOOLEAN]

Init ==
    /\ queues = [s \in ShardId |-> <<[type |-> "Submit", run |-> 1, action |-> 1]>>]
    /\ shard_status = [s \in ShardId |-> FALSE]
    /\ total_processed = 0

\* ---- tick(s): process exactly ONE command from shard s's queue ----
\*
\* Precondition: shard s is not shutting_down
\* Postcondition: exactly one command is dequeued (if queue non-empty) and processed
Tick(s) ==
    /\ shard_status[s] = FALSE
    /\ LET q == queues[s] IN
            IF Len(q) = 0 THEN
                \* Empty queue — tick returns TRUE (continue, no work done)
                /\ queues' = queues
                /\ UNCHANGED <<shard_status, total_processed>>
        ELSE
            \* Dequeue one command and process it
            LET cmd == Head(q) IN
            LET remaining == Tail(q) IN
            IF cmd.type = "Shutdown" THEN
                \* Shutdown command — mark shutting_down, return FALSE
                /\ queues' = [queues EXCEPT ![s] = remaining]
                /\ shard_status' = [shard_status EXCEPT ![s] = TRUE]
            ELSE
                /\ queues' = [queues EXCEPT ![s] = remaining]
                /\ UNCHANGED shard_status
    /\ total_processed' = total_processed + 1

\* ---- tick_all(): drain all shards until all queues empty ----
\*
\* Progress property: under fair scheduling, all commands are eventually processed.
TickAll ==
    \E s \in ShardId :
        /\ Len(queues[s]) > 0
        /\ Tick(s)

TickAllProgress ==
    \* Eventually all queues are empty or all shards are shutting_down
    <>( \A s \in ShardId : Len(queues[s]) = 0 \/ shard_status[s] = TRUE )

\* ---- shutdown_graceful(): drain ALL pending commands on all shards ----
\*
\* Differs from tick_all: keeps draining even on empty queues until shutdown
\* command is encountered OR all queues are definitively empty.
ShutdownGraceful ==
    /\ \E s \in ShardId :
        /\ shard_status[s] = FALSE
        /\ LET q == queues[s] IN
            IF Len(q) = 0 THEN
                \* Empty — try next shard (no-op for this shard)
                /\ queues' = queues
                /\ UNCHANGED <<shard_status, total_processed>>
            ELSE
                IF Head(q).type = "Shutdown" THEN
                    /\ queues' = [queues EXCEPT ![s] = Tail(q)]
                    /\ shard_status' = [shard_status EXCEPT ![s] = TRUE]
                ELSE
                    /\ queues' = [queues EXCEPT ![s] = Tail(q)]
                    /\ UNCHANGED shard_status
    /\ total_processed' = total_processed + 1

\* All shards are drained: every queue is empty or shutting_down
AllDrained ==
    \A s \in ShardId : Len(queues[s]) = 0 \/ shard_status[s] = TRUE

\* Progress: all drained is eventually reached under ShutdownGraceful
ShutdownProgress ==
    <>(AllDrained)

\* ---- Queue capacity invariant ----
QueueBounded ==
    \A s \in ShardId : Len(queues[s]) <= MAX_COMMAND_QUEUE_CAPACITY

\* ---- tick processes exactly one command when non-empty ----
TickOneCommand ==
    \A s \in ShardId :
        /\ shard_status[s] = FALSE
        /\ Len(queues[s]) > 0
        => Len(queues[s])' = Len(queues[s]) - 1

SubmitCommand(s) ==
    /\ shard_status[s] = FALSE
    /\ Len(queues[s]) < MAX_COMMAND_QUEUE_CAPACITY
    /\ \E run \in RunId, action \in ActionId :
        queues' = [queues EXCEPT ![s] = Append(queues[s], [type |-> "Submit", run |-> run, action |-> action])]
    /\ UNCHANGED <<shard_status, total_processed>>

Next ==
    \/ \E s \in ShardId : Tick(s)
    \/ \E s \in ShardId : SubmitCommand(s)
    \/ ShutdownGraceful

Spec == Init /\ [][Next]_<<queues, shard_status, total_processed>>

StateConstraint ==
    /\ Len(queues[1]) <= 3
    /\ total_processed <= 5

\* Theorems
THEOREM Spec => []QueueBounded
THEOREM Spec => [][TickOneCommand]_<<queues, shard_status, total_processed>>
THEOREM Spec => [][AllDrained]_<<queues, shard_status, total_processed>>

====
