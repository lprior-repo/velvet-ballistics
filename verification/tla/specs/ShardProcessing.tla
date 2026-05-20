---- MODULE ShardProcessing ----
EXTENDS Naturals, FiniteSets, Sequences, TLC

\* Obligation: PO-002, PO-006
\* Requirement: TLA-WF-002 (INV-007), TLA-WF-006 (POST-005)
\* Model: Single-shard FIFO command processing with bounded queue.
\* Bounds: MaxQueueDepth <= 3, shard_count <= 4

CONSTANTS MAX_QUEUE_DEPTH, SHARD_COUNT, MAX_RUNS

ASSUME MAX_QUEUE_DEPTH \in Nat \ {0}
ASSUME MAX_QUEUE_DEPTH <= 3
ASSUME SHARD_COUNT \in Nat \ {0}
ASSUME SHARD_COUNT <= 4
ASSUME MAX_RUNS \in Nat \ {0}
ASSUME MAX_RUNS <= 8

VARIABLES
    queue,          \* Sequence of [cmd, seq] entries in FIFO order
    processing,     \* Currently processing command (or null marker)
    shard_state,    \* Shard operational state
    commands_issued,\* Count of commands issued this tick
    insert_counter  \* Monotonic counter for FIFO ordering

vars == <<queue, processing, shard_state, commands_issued, insert_counter>>

RunIds == 1..MAX_RUNS
Commands == {"submit", "cancel", "resume", "timer_fired", "action_complete", "inspect"}
NullCommand == [cmd |-> "null_cmd", run |-> 0]
CommandPayload == [cmd: Commands, run: RunIds]

\* Maximum insert counter value to bound state space
MaxInsertCounter == 5

Init ==
    /\ queue = <<>>
    /\ processing = NullCommand
    /\ shard_state = "active"
    /\ commands_issued = 0
    /\ insert_counter = 0

CanEnqueue == Len(queue) < MAX_QUEUE_DEPTH

Enqueue(cmd) ==
    /\ CanEnqueue
    /\ shard_state = "active"
    /\ insert_counter < MaxInsertCounter
    /\ insert_counter' = insert_counter + 1
    /\ queue' = Append(queue, [cmd |-> cmd, seq |-> insert_counter'])
    /\ processing' = processing
    /\ shard_state' = shard_state
    /\ commands_issued' = commands_issued

Dequeue ==
    /\ queue # <<>>
    /\ processing.cmd = "null_cmd"
    /\ shard_state = "active"
    /\ processing' = Head(queue).cmd
    /\ queue' = Tail(queue)
    /\ commands_issued' = commands_issued + 1
    /\ shard_state' = shard_state
    /\ UNCHANGED insert_counter

Complete ==
    /\ processing.cmd # "null_cmd"
    /\ processing' = NullCommand
    /\ queue' = queue
    /\ commands_issued' = 0  \* Reset on completion
    /\ shard_state' = shard_state
    /\ UNCHANGED insert_counter

FailCmd ==
    /\ processing.cmd # "null_cmd"
    /\ processing' = NullCommand
    /\ queue' = queue
    /\ commands_issued' = 0  \* Reset on failure
    /\ shard_state' = shard_state
    /\ UNCHANGED insert_counter

\* ShutdownDrain handles the case where shutdown occurs with pending commands
\* in queue. This resolves the Dequeue/Shutdown mutual-enabling conflict.
ShutdownDrain ==
    /\ shard_state = "active"
    /\ queue # <<>>
    /\ processing.cmd = "null_cmd"
    /\ shard_state' = "shutdown"
    /\ processing' = NullCommand
    /\ queue' = <<>>
    /\ commands_issued' = commands_issued
    /\ UNCHANGED insert_counter

Shutdown ==
    /\ shard_state = "active"
    /\ queue = <<>>
    /\ shard_state' = "shutdown"
    /\ processing' = NullCommand
    /\ queue' = <<>>
    /\ commands_issued' = commands_issued
    /\ UNCHANGED insert_counter

Progress ==
    \/ Dequeue
    \/ Complete
    \/ FailCmd
    \/ ShutdownDrain
    \/ Shutdown
    \/ (\E cmd \in CommandPayload : Enqueue(cmd))
    \/ (shard_state = "shutdown" /\ UNCHANGED <<queue, processing, shard_state, commands_issued, insert_counter>>)

Spec == Init /\ [][Progress]_vars

\* Invariant: QueueFIFO — commands are dequeued in FIFO order (by seq number stored in queue entry)
QueueFIFO ==
    \A i \in 1..Len(queue), j \in 1..Len(queue):
        i < j => queue[i].seq < queue[j].seq

\* Invariant: OneCommandPerTick — at most one command issued per tick
OneCommandPerTick ==
    commands_issued <= 1

\* Invariant: ShutdownCorrectness — after shutdown, queue is empty and no processing
ShutdownCorrectness ==
    shard_state = "shutdown" =>
        /\ queue = <<>>
        /\ processing.cmd = "null_cmd"

===============================================================================
