---- MODULE IpcSyncEvidence ----
EXTENDS Naturals, FiniteSets

\* Obligations: TLA-IPC-001..007 and capacity boundary rows.
\* Finite safety/enabledness abstraction of IPC submit admission, bounded
\* runtime ingress, terminal races, timers, shutdown, fanout, and slow-client
\* buffering. The specification includes weak fairness plus executable temporal
\* properties for queue drain, shutdown drain, and slow-client disconnect.

CONSTANTS RUNS, CLIENTS, QUEUE_CAPACITY, BUFFER_CAPACITY

Bool == {TRUE, FALSE}
TerminalStates == {"none", "completed", "cancelled"}
RunsCardinalityOk == Cardinality(RUNS) >= 1
ClientsCardinalityOk == Cardinality(CLIENTS) >= 1
CapacityOk == QUEUE_CAPACITY \in 1..2 /\ BUFFER_CAPACITY \in 1..2

VARIABLES artifact_ok, accepted, rejected, queued, queue_len,
          runtime_submitted, terminal, terminal_count,
          timer_eligible, timer_fired, timer_after_terminal,
          shutdown, admission_open, drained, buffer_used, connected

vars == <<artifact_ok, accepted, rejected, queued, queue_len,
          runtime_submitted, terminal, terminal_count,
          timer_eligible, timer_fired, timer_after_terminal,
          shutdown, admission_open, drained, buffer_used, connected>>

Init ==
  /\ RunsCardinalityOk
  /\ ClientsCardinalityOk
  /\ CapacityOk
  /\ artifact_ok \in [RUNS -> Bool]
  /\ accepted = [r \in RUNS |-> FALSE]
  /\ rejected = [r \in RUNS |-> FALSE]
  /\ queued = {}
  /\ queue_len = 0
  /\ runtime_submitted = [r \in RUNS |-> FALSE]
  /\ terminal = [r \in RUNS |-> "none"]
  /\ terminal_count = [r \in RUNS |-> 0]
  /\ timer_eligible = [r \in RUNS |-> FALSE]
  /\ timer_fired = [r \in RUNS |-> FALSE]
  /\ timer_after_terminal = [r \in RUNS |-> FALSE]
  /\ shutdown = FALSE
  /\ admission_open = TRUE
  /\ drained = FALSE
  /\ buffer_used = [c \in CLIENTS |-> 0]
  /\ connected = [c \in CLIENTS |-> TRUE]

CanSubmit(r) ==
  /\ r \in RUNS
  /\ admission_open
  /\ ~accepted[r]
  /\ ~rejected[r]

AcceptSubmit(r) ==
  /\ CanSubmit(r)
  /\ artifact_ok[r]
  /\ queue_len < QUEUE_CAPACITY
  /\ accepted' = [accepted EXCEPT ![r] = TRUE]
  /\ queued' = queued \cup {r}
  /\ queue_len' = queue_len + 1
  /\ UNCHANGED <<artifact_ok, rejected, runtime_submitted, terminal,
                  terminal_count, timer_eligible, timer_fired,
                  timer_after_terminal, shutdown, admission_open,
                  drained, buffer_used, connected>>

RejectMissingArtifact(r) ==
  /\ CanSubmit(r)
  /\ ~artifact_ok[r]
  /\ rejected' = [rejected EXCEPT ![r] = TRUE]
  /\ UNCHANGED <<artifact_ok, accepted, queued, queue_len,
                  runtime_submitted, terminal, terminal_count,
                  timer_eligible, timer_fired, timer_after_terminal,
                  shutdown, admission_open, drained, buffer_used, connected>>

RejectFullQueue(r) ==
  /\ CanSubmit(r)
  /\ artifact_ok[r]
  /\ queue_len = QUEUE_CAPACITY
  /\ rejected' = [rejected EXCEPT ![r] = TRUE]
  /\ UNCHANGED <<artifact_ok, accepted, queued, queue_len,
                  runtime_submitted, terminal, terminal_count,
                  timer_eligible, timer_fired, timer_after_terminal,
                  shutdown, admission_open, drained, buffer_used, connected>>

RejectAfterShutdown(r) ==
  /\ r \in RUNS
  /\ shutdown
  /\ ~accepted[r]
  /\ ~rejected[r]
  /\ rejected' = [rejected EXCEPT ![r] = TRUE]
  /\ UNCHANGED <<artifact_ok, accepted, queued, queue_len,
                  runtime_submitted, terminal, terminal_count,
                  timer_eligible, timer_fired, timer_after_terminal,
                  shutdown, admission_open, drained, buffer_used, connected>>

DrainOne(r) ==
  /\ r \in queued
  /\ runtime_submitted' = [runtime_submitted EXCEPT ![r] = TRUE]
  /\ queued' = queued \ {r}
  /\ queue_len' = queue_len - 1
  /\ timer_eligible' = [timer_eligible EXCEPT ![r] = TRUE]
  /\ UNCHANGED <<artifact_ok, accepted, rejected, terminal,
                  terminal_count, timer_fired, timer_after_terminal,
                  shutdown, admission_open, drained, buffer_used, connected>>

CompleteRun(r) ==
  /\ r \in RUNS
  /\ runtime_submitted[r]
  /\ terminal[r] = "none"
  /\ terminal' = [terminal EXCEPT ![r] = "completed"]
  /\ terminal_count' = [terminal_count EXCEPT ![r] = terminal_count[r] + 1]
  /\ timer_eligible' = [timer_eligible EXCEPT ![r] = FALSE]
  /\ UNCHANGED <<artifact_ok, accepted, rejected, queued, queue_len,
                  runtime_submitted, timer_fired, timer_after_terminal,
                  shutdown, admission_open, drained, buffer_used, connected>>

CancelRun(r) ==
  /\ r \in RUNS
  /\ runtime_submitted[r]
  /\ terminal[r] = "none"
  /\ terminal' = [terminal EXCEPT ![r] = "cancelled"]
  /\ terminal_count' = [terminal_count EXCEPT ![r] = terminal_count[r] + 1]
  /\ timer_eligible' = [timer_eligible EXCEPT ![r] = FALSE]
  /\ UNCHANGED <<artifact_ok, accepted, rejected, queued, queue_len,
                  runtime_submitted, timer_fired, timer_after_terminal,
                  shutdown, admission_open, drained, buffer_used, connected>>

StaleTerminalEvent(r) ==
  /\ r \in RUNS
  /\ terminal[r] # "none"
  /\ UNCHANGED vars

FireTimer(r) ==
  /\ r \in RUNS
  /\ timer_eligible[r]
  /\ terminal[r] = "none"
  /\ timer_fired' = [timer_fired EXCEPT ![r] = TRUE]
  /\ timer_eligible' = [timer_eligible EXCEPT ![r] = FALSE]
  /\ UNCHANGED <<artifact_ok, accepted, rejected, queued, queue_len,
                  runtime_submitted, terminal, terminal_count,
                  timer_after_terminal, shutdown, admission_open,
                  drained, buffer_used, connected>>

StaleTimerEvent(r) ==
  /\ r \in RUNS
  /\ terminal[r] # "none"
  /\ timer_after_terminal' = [timer_after_terminal EXCEPT ![r] = FALSE]
  /\ UNCHANGED <<artifact_ok, accepted, rejected, queued, queue_len,
                  runtime_submitted, terminal, terminal_count,
                  timer_eligible, timer_fired, shutdown, admission_open,
                  drained, buffer_used, connected>>

StartShutdown ==
  /\ ~shutdown
  /\ shutdown' = TRUE
  /\ admission_open' = FALSE
  /\ UNCHANGED <<artifact_ok, accepted, rejected, queued, queue_len,
                  runtime_submitted, terminal, terminal_count,
                  timer_eligible, timer_fired, timer_after_terminal,
                  drained, buffer_used, connected>>

MarkDrained ==
  /\ shutdown
  /\ queue_len = 0
  /\ drained' = TRUE
  /\ UNCHANGED <<artifact_ok, accepted, rejected, queued, queue_len,
                  runtime_submitted, terminal, terminal_count,
                  timer_eligible, timer_fired, timer_after_terminal,
                  shutdown, admission_open, buffer_used, connected>>

WriteToClient(c) ==
  /\ c \in CLIENTS
  /\ connected[c]
  /\ buffer_used[c] < BUFFER_CAPACITY
  /\ buffer_used' = [buffer_used EXCEPT ![c] = buffer_used[c] + 1]
  /\ UNCHANGED <<artifact_ok, accepted, rejected, queued, queue_len,
                  runtime_submitted, terminal, terminal_count,
                  timer_eligible, timer_fired, timer_after_terminal,
                  shutdown, admission_open, drained, connected>>

DisconnectSlowClient(c) ==
  /\ c \in CLIENTS
  /\ connected[c]
  /\ buffer_used[c] = BUFFER_CAPACITY
  /\ connected' = [connected EXCEPT ![c] = FALSE]
  /\ UNCHANGED <<artifact_ok, accepted, rejected, queued, queue_len,
                  runtime_submitted, terminal, terminal_count,
                  timer_eligible, timer_fired, timer_after_terminal,
                  shutdown, admission_open, drained, buffer_used>>

Next ==
  \/ \E r \in RUNS: AcceptSubmit(r)
  \/ \E r \in RUNS: RejectMissingArtifact(r)
  \/ \E r \in RUNS: RejectFullQueue(r)
  \/ \E r \in RUNS: RejectAfterShutdown(r)
  \/ \E r \in RUNS: DrainOne(r)
  \/ \E r \in RUNS: CompleteRun(r)
  \/ \E r \in RUNS: CancelRun(r)
  \/ \E r \in RUNS: StaleTerminalEvent(r)
  \/ \E r \in RUNS: FireTimer(r)
  \/ \E r \in RUNS: StaleTimerEvent(r)
  \/ StartShutdown
  \/ MarkDrained
  \/ \E c \in CLIENTS: WriteToClient(c)
  \/ \E c \in CLIENTS: DisconnectSlowClient(c)

Fairness ==
  /\ WF_vars(\E r \in RUNS: DrainOne(r))
  /\ WF_vars(MarkDrained)
  /\ WF_vars(\E c \in CLIENTS: DisconnectSlowClient(c))

Spec == Init /\ [][Next]_vars /\ Fairness

TypeOK ==
  /\ artifact_ok \in [RUNS -> Bool]
  /\ accepted \in [RUNS -> Bool]
  /\ rejected \in [RUNS -> Bool]
  /\ queued \subseteq RUNS
  /\ queue_len \in 0..QUEUE_CAPACITY
  /\ queue_len = Cardinality(queued)
  /\ runtime_submitted \in [RUNS -> Bool]
  /\ terminal \in [RUNS -> TerminalStates]
  /\ terminal_count \in [RUNS -> 0..1]
  /\ timer_eligible \in [RUNS -> Bool]
  /\ timer_fired \in [RUNS -> Bool]
  /\ timer_after_terminal \in [RUNS -> Bool]
  /\ shutdown \in Bool
  /\ admission_open \in Bool
  /\ drained \in Bool
  /\ buffer_used \in [CLIENTS -> 0..BUFFER_CAPACITY]
  /\ connected \in [CLIENTS -> Bool]

StrictAdmissionBeforeRuntimeSubmit ==
  \A r \in RUNS: runtime_submitted[r] => accepted[r] /\ artifact_ok[r]

NoSilentDrop ==
  \A r \in RUNS: accepted[r] => runtime_submitted[r] \/ r \in queued

AcceptedSubmitEventuallyQueuedOrRejected ==
  \A r \in RUNS: accepted[r] \/ rejected[r] \/ CanSubmit(r) \/ shutdown

QueueBoundsHold ==
  queue_len <= QUEUE_CAPACITY /\ queued \subseteq RUNS

FullSubmitEventuallyRejected ==
  \A r \in RUNS: queue_len = QUEUE_CAPACITY /\ CanSubmit(r) /\ artifact_ok[r]
    => ENABLED RejectFullQueue(r)

SingleTerminalOutcomePerRun ==
  \A r \in RUNS: terminal_count[r] <= 1

TerminalStateStable ==
  \A r \in RUNS: terminal[r] # "none" => ~timer_eligible[r]

RaceEventuallyResolved ==
  \A r \in RUNS: terminal[r] # "none" => terminal_count[r] = 1

NoTimerAfterTerminal ==
  \A r \in RUNS: ~timer_after_terminal[r]

TimerOrderPreserved ==
  \A r \in RUNS: timer_fired[r] => runtime_submitted[r]

EligibleTimerEventuallyFiresOrBecomesIneligible ==
  \A r \in RUNS: timer_eligible[r] => ENABLED FireTimer(r)

ShutdownNeverReopensAdmission ==
  shutdown => ~admission_open

NoSubmitAcceptedAfterShutdown ==
  \A r \in RUNS: shutdown /\ accepted[r] => runtime_submitted[r] \/ r \in queued

ShutdownEventuallyDrainedOrExplicitlyRejected ==
  shutdown => drained \/ queue_len > 0 \/ ENABLED MarkDrained \/ (\A r \in RUNS: accepted[r] \/ rejected[r])

ClientBuffersBounded ==
  \A c \in CLIENTS: buffer_used[c] <= BUFFER_CAPACITY

SlowClientEventuallyWritableOrDisconnected ==
  \A c \in CLIENTS: connected[c] => buffer_used[c] < BUFFER_CAPACITY \/ ENABLED DisconnectSlowClient(c)

QueuedWorkLiveness ==
  \A r \in RUNS: [](r \in queued => <>runtime_submitted[r])

ShutdownDrainLiveness ==
  [](shutdown /\ queue_len = 0 => <>drained)

SlowClientDisconnectLiveness ==
  \A c \in CLIENTS: [](connected[c] /\ buffer_used[c] = BUFFER_CAPACITY => <>~connected[c])

====
