---- MODULE RecoveryHydration ----
EXTENDS Naturals, Sequences

\* Obligations: PO-001..PO-010, PO-023, PO-024, PO-026.
\* Finite recovery-hydration protocol model for one requested run. Fjall I/O,
\* byte decoding, hashing, and OS crash mechanics are trusted shell boundaries.

CONSTANT MaxSeq

Runs == {"run_a", "run_b"}
Bool == {TRUE, FALSE}
HydrationStates == {"none", "summary", "frame"}
ErrorStates == {"none", "replay_divergence", "unsupported", "no_recovery_data", "terminal_mismatch"}
TerminalStates == {"none", "ok", "failed"}
AckStates == {"none", "before_ack", "after_ack"}

Event == [ run: Runs,
           seq: 1..MaxSeq,
           has_header: Bool,
           has_pc: Bool,
           has_slot: Bool,
           has_taint: Bool,
           has_step: Bool,
           has_action: Bool,
           has_wait: Bool,
           has_ask: Bool,
           has_retry: Bool,
           has_collect: Bool,
           has_ticket: Bool,
           unsupported: Bool,
           terminal: TerminalStates,
           yaml_input: Bool ]

BaseEvent == [run |-> "run_a", seq |-> 1, has_header |-> TRUE,
              has_pc |-> TRUE, has_slot |-> TRUE, has_taint |-> TRUE,
              has_step |-> TRUE, has_action |-> TRUE, has_wait |-> TRUE,
              has_ask |-> TRUE, has_retry |-> TRUE, has_collect |-> TRUE,
              has_ticket |-> TRUE, unsupported |-> FALSE,
              terminal |-> "none", yaml_input |-> FALSE]
TerminalEvent == [BaseEvent EXCEPT !.seq = 2, !.terminal = "ok"]
FailedTerminalEvent == [BaseEvent EXCEPT !.seq = 2, !.terminal = "failed"]
MixedRunEvent == [BaseEvent EXCEPT !.run = "run_b"]
UnsupportedEvent == [BaseEvent EXCEPT !.unsupported = TRUE]
YamlEvent == [BaseEvent EXCEPT !.yaml_input = TRUE]
MissingFrameFactsEvent == [BaseEvent EXCEPT !.has_pc = FALSE, !.has_step = FALSE,
                           !.has_action = FALSE, !.has_wait = FALSE,
                           !.has_ask = FALSE, !.has_retry = FALSE,
                           !.has_collect = FALSE, !.has_ticket = FALSE]

EventStreams == {<<>>, <<BaseEvent>>, <<BaseEvent, TerminalEvent>>,
                 <<TerminalEvent, BaseEvent>>, <<MixedRunEvent>>,
                 <<UnsupportedEvent>>, <<YamlEvent>>,
                 <<MissingFrameFactsEvent>>,
                 <<BaseEvent, FailedTerminalEvent>>}

Snapshot == [ run: Runs,
              seq: 0..MaxSeq,
              present: Bool,
              has_header: Bool,
              has_pc: Bool,
              has_slot: Bool,
              has_taint: Bool,
              has_step: Bool,
              has_action: Bool,
              has_wait: Bool,
              has_ask: Bool,
              has_retry: Bool,
              has_collect: Bool,
              has_ticket: Bool,
              terminal: TerminalStates ]

SnapshotChoices == {s \in Snapshot : s.present = FALSE => s.seq = 0}

VARIABLES requested_run, journal, snapshot, ack_state, persisted_header,
          crashed, restarted, hydration, error, recovered_header,
          recovered_pc, recovered_slot, recovered_taint, recovered_step,
          recovered_action, recovered_wait, recovered_ask, recovered_retry,
          recovered_collect, recovered_ticket, terminal

vars == <<requested_run, journal, snapshot, ack_state, persisted_header,
          crashed, restarted, hydration, error, recovered_header,
          recovered_pc, recovered_slot, recovered_taint, recovered_step,
          recovered_action, recovered_wait, recovered_ask, recovered_retry,
          recovered_collect, recovered_ticket, terminal>>

TypeOK ==
  /\ requested_run \in Runs
  /\ journal \in EventStreams
  /\ snapshot \in SnapshotChoices
  /\ ack_state \in AckStates
  /\ persisted_header \in BOOLEAN
  /\ crashed \in BOOLEAN
  /\ restarted \in BOOLEAN
  /\ hydration \in HydrationStates
  /\ error \in ErrorStates
  /\ recovered_header \in BOOLEAN
  /\ recovered_pc \in BOOLEAN
  /\ recovered_slot \in BOOLEAN
  /\ recovered_taint \in BOOLEAN
  /\ recovered_step \in BOOLEAN
  /\ recovered_action \in BOOLEAN
  /\ recovered_wait \in BOOLEAN
  /\ recovered_ask \in BOOLEAN
  /\ recovered_retry \in BOOLEAN
  /\ recovered_collect \in BOOLEAN
  /\ recovered_ticket \in BOOLEAN
  /\ terminal \in TerminalStates

Init ==
  /\ requested_run \in Runs
  /\ journal \in EventStreams
  /\ snapshot \in SnapshotChoices
  /\ ack_state = "none"
  /\ persisted_header = FALSE
  /\ crashed = FALSE
  /\ restarted = FALSE
  /\ hydration = "none"
  /\ error = "none"
  /\ recovered_header = FALSE
  /\ recovered_pc = FALSE
  /\ recovered_slot = FALSE
  /\ recovered_taint = FALSE
  /\ recovered_step = FALSE
  /\ recovered_action = FALSE
  /\ recovered_wait = FALSE
  /\ recovered_ask = FALSE
  /\ recovered_retry = FALSE
  /\ recovered_collect = FALSE
  /\ recovered_ticket = FALSE
  /\ terminal = "none"

TailAfterSnapshot ==
  snapshot.present => \A i \in 1..Len(journal): journal[i].seq > snapshot.seq

JournalForRequestedRun ==
  \A i \in 1..Len(journal): journal[i].run = requested_run

SnapshotForRequestedRun ==
  snapshot.present => snapshot.run = requested_run

JournalSeqOrdered ==
  \A i, j \in 1..Len(journal): i < j => journal[i].seq < journal[j].seq

NoYamlInJournal ==
  \A i \in 1..Len(journal): journal[i].yaml_input = FALSE

HasDurableInput == Len(journal) > 0 \/ snapshot.present

JournalHas(field) == \E i \in 1..Len(journal): journal[i][field]
JournalUnsupported == \E i \in 1..Len(journal): journal[i].unsupported
JournalTerminalEvents == {journal[i].terminal : i \in 1..Len(journal)} \ {"none"}

DurableHasHeader == persisted_header \/ snapshot.has_header \/ JournalHas("has_header")
DurableHasPc == snapshot.has_pc \/ JournalHas("has_pc")
DurableHasSlot == snapshot.has_slot \/ JournalHas("has_slot")
DurableHasTaint == snapshot.has_taint \/ JournalHas("has_taint")
DurableHasStep == snapshot.has_step \/ JournalHas("has_step")
DurableHasAction == snapshot.has_action \/ JournalHas("has_action")
DurableHasWait == snapshot.has_wait \/ JournalHas("has_wait")
DurableHasAsk == snapshot.has_ask \/ JournalHas("has_ask")
DurableHasRetry == snapshot.has_retry \/ JournalHas("has_retry")
DurableHasCollect == snapshot.has_collect \/ JournalHas("has_collect")
DurableHasTicket == snapshot.has_ticket \/ JournalHas("has_ticket")

AllFrameFactsDurable ==
  /\ DurableHasHeader
  /\ DurableHasPc
  /\ DurableHasSlot
  /\ DurableHasTaint
  /\ DurableHasStep
  /\ DurableHasAction
  /\ DurableHasWait
  /\ DurableHasAsk
  /\ DurableHasRetry
  /\ DurableHasCollect
  /\ DurableHasTicket

UnsupportedDurableState == JournalUnsupported \/ ~AllFrameFactsDurable

TerminalContradiction ==
  "ok" \in JournalTerminalEvents /\ "failed" \in JournalTerminalEvents

ValidRecoveryInput ==
  /\ HasDurableInput
  /\ JournalForRequestedRun
  /\ SnapshotForRequestedRun
  /\ JournalSeqOrdered
  /\ TailAfterSnapshot
  /\ NoYamlInJournal
  /\ ~TerminalContradiction

TerminalFromDurable ==
  IF JournalTerminalEvents = {} THEN snapshot.terminal ELSE CHOOSE t \in JournalTerminalEvents: TRUE

PersistHeader ==
  /\ ~persisted_header
  /\ ack_state = "none"
  /\ ~crashed
  /\ persisted_header' = TRUE
  /\ ack_state' = "before_ack"
  /\ UNCHANGED <<requested_run, journal, snapshot, crashed, restarted,
                  hydration, error, recovered_header, recovered_pc,
                  recovered_slot, recovered_taint, recovered_step,
                  recovered_action, recovered_wait, recovered_ask,
                  recovered_retry, recovered_collect, recovered_ticket, terminal>>

AcknowledgeRun ==
  /\ persisted_header
  /\ ack_state = "before_ack"
  /\ ~crashed
  /\ ack_state' = "after_ack"
  /\ UNCHANGED <<requested_run, journal, snapshot, persisted_header, crashed,
                  restarted, hydration, error, recovered_header, recovered_pc,
                  recovered_slot, recovered_taint, recovered_step,
                  recovered_action, recovered_wait, recovered_ask,
                  recovered_retry, recovered_collect, recovered_ticket, terminal>>

Crash ==
  /\ persisted_header
  /\ ack_state \in {"before_ack", "after_ack"}
  /\ ~crashed
  /\ crashed' = TRUE
  /\ UNCHANGED <<requested_run, journal, snapshot, ack_state, persisted_header,
                  restarted, hydration, error, recovered_header, recovered_pc,
                  recovered_slot, recovered_taint, recovered_step,
                  recovered_action, recovered_wait, recovered_ask,
                  recovered_retry, recovered_collect, recovered_ticket, terminal>>

Restart ==
  /\ crashed
  /\ ~restarted
  /\ restarted' = TRUE
  /\ UNCHANGED <<requested_run, journal, snapshot, ack_state, persisted_header,
                  crashed, hydration, error, recovered_header, recovered_pc,
                  recovered_slot, recovered_taint, recovered_step,
                  recovered_action, recovered_wait, recovered_ask,
                  recovered_retry, recovered_collect, recovered_ticket, terminal>>

RecoverSummary ==
  /\ restarted
  /\ hydration = "none"
  /\ error = "none"
  /\ ValidRecoveryInput
  /\ hydration' = "summary"
  /\ terminal' = TerminalFromDurable
  /\ UNCHANGED <<requested_run, journal, snapshot, ack_state, persisted_header,
                  crashed, restarted, error, recovered_header, recovered_pc,
                  recovered_slot, recovered_taint, recovered_step,
                  recovered_action, recovered_wait, recovered_ask,
                  recovered_retry, recovered_collect, recovered_ticket>>

RecoverFrameSeed ==
  /\ restarted
  /\ hydration = "summary"
  /\ error = "none"
  /\ ValidRecoveryInput
  /\ ~UnsupportedDurableState
  /\ hydration' = "frame"
  /\ recovered_header' = DurableHasHeader
  /\ recovered_pc' = DurableHasPc
  /\ recovered_slot' = DurableHasSlot
  /\ recovered_taint' = DurableHasTaint
  /\ recovered_step' = DurableHasStep
  /\ recovered_action' = DurableHasAction
  /\ recovered_wait' = DurableHasWait
  /\ recovered_ask' = DurableHasAsk
  /\ recovered_retry' = DurableHasRetry
  /\ recovered_collect' = DurableHasCollect
  /\ recovered_ticket' = DurableHasTicket
  /\ terminal' = TerminalFromDurable
  /\ UNCHANGED <<requested_run, journal, snapshot, ack_state, persisted_header,
                  crashed, restarted, error>>

FailClosed ==
  /\ restarted
  /\ hydration # "frame"
  /\ error = "none"
  /\ error' = IF ~HasDurableInput THEN "no_recovery_data"
              ELSE IF TerminalContradiction THEN "terminal_mismatch"
              ELSE IF ~ValidRecoveryInput THEN "replay_divergence"
              ELSE "unsupported"
  /\ hydration' = "none"
  /\ recovered_header' = FALSE
  /\ recovered_pc' = FALSE
  /\ recovered_slot' = FALSE
  /\ recovered_taint' = FALSE
  /\ recovered_step' = FALSE
  /\ recovered_action' = FALSE
  /\ recovered_wait' = FALSE
  /\ recovered_ask' = FALSE
  /\ recovered_retry' = FALSE
  /\ recovered_collect' = FALSE
  /\ recovered_ticket' = FALSE
  /\ terminal' = "none"
  /\ UNCHANGED <<requested_run, journal, snapshot, ack_state, persisted_header,
                  crashed, restarted>>

TerminalStutter ==
  /\ hydration = "frame" \/ error # "none"
  /\ UNCHANGED vars

Next == PersistHeader \/ AcknowledgeRun \/ Crash \/ Restart \/ RecoverSummary \/ RecoverFrameSeed \/ FailClosed \/ TerminalStutter

Spec ==
  /\ Init
  /\ [][Next]_vars
  /\ WF_vars(PersistHeader)
  /\ WF_vars(AcknowledgeRun)
  /\ WF_vars(Restart)
  /\ WF_vars(RecoverSummary)
  /\ WF_vars(RecoverFrameSeed)
  /\ WF_vars(FailClosed)

NoMixedRunRecovery ==
  hydration # "none" => JournalForRequestedRun /\ SnapshotForRequestedRun

JournalSeqMonotonic ==
  hydration # "none" => JournalSeqOrdered

AllRecoveredFacts ==
  /\ recovered_header
  /\ recovered_pc
  /\ recovered_slot
  /\ recovered_taint
  /\ recovered_step
  /\ recovered_action
  /\ recovered_wait
  /\ recovered_ask
  /\ recovered_retry
  /\ recovered_collect
  /\ recovered_ticket

NoSuccessfulEmptyFrame ==
  HasDurableInput => ~(hydration = "frame" /\ ~AllRecoveredFacts)

NoFabricatedDurableFacts ==
  hydration = "frame" =>
    /\ recovered_header = DurableHasHeader
    /\ recovered_pc = DurableHasPc
    /\ recovered_slot = DurableHasSlot
    /\ recovered_taint = DurableHasTaint
    /\ recovered_step = DurableHasStep
    /\ recovered_action = DurableHasAction
    /\ recovered_wait = DurableHasWait
    /\ recovered_ask = DurableHasAsk
    /\ recovered_retry = DurableHasRetry
    /\ recovered_collect = DurableHasCollect
    /\ recovered_ticket = DurableHasTicket

FrameSeedCompleteOrUnsupported ==
  hydration = "frame" => AllFrameFactsDurable /\ ~UnsupportedDurableState

UnsupportedRejectsHydration ==
  UnsupportedDurableState => hydration # "frame"

SnapshotThenTailOnly ==
  hydration # "none" => TailAfterSnapshot

TerminalConsistent ==
  hydration # "none" => terminal = TerminalFromDurable

NoYamlRecoveryInput ==
  hydration # "none" => NoYamlInJournal

BeforeAckCrashIsObservable ==
  restarted /\ ack_state = "before_ack" => persisted_header

AfterAckCrashIsObservable ==
  restarted /\ ack_state = "after_ack" => persisted_header

CrashRestartPreservesDurableFactsOrFailsClosed ==
  restarted /\ (hydration = "frame" \/ error # "none") =>
    \/ error # "none"
    \/ (hydration = "frame" /\ AllRecoveredFacts /\ NoFabricatedDurableFacts)

ReplayDivergenceFailsClosed ==
  restarted /\ (~JournalForRequestedRun \/ ~JournalSeqOrdered \/ ~TailAfterSnapshot) =>
    hydration # "frame"

NoRecoveryDataFailsClosed ==
  restarted /\ ~HasDurableInput => hydration # "frame"

TerminalMismatchFailsClosed ==
  restarted /\ TerminalContradiction => hydration # "frame"

RestartEventuallyRecoversOrFailsClosed ==
  restarted => <>(hydration = "frame" \/ error # "none")

====
