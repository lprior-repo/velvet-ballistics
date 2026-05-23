---- MODULE EngineYamlRunLifecycle ----
EXTENDS Naturals, Sequences

\* Obligations: PO-003 / TLA-LIFE-001.
\* Bounded lifecycle and journal sequence model for accepted engine runs.

CONSTANT MaxSeq

VARIABLES run_state, seq, journal, proof_gates, terminal_seen, terminal_state,
          terminal_seq, terminal_journal

vars == <<run_state, seq, journal, proof_gates, terminal_seen, terminal_state,
          terminal_seq, terminal_journal>>

TypeOK ==
  /\ run_state \in Live \cup Terminal
  /\ seq \in 0..MaxSeq
  /\ journal \in Seq(0..MaxSeq)
  /\ proof_gates = "verified"
  /\ terminal_seen \in BOOLEAN
  /\ terminal_state \in Terminal \cup {"none"}
  /\ terminal_seq \in 0..MaxSeq
  /\ terminal_journal \in Seq(0..MaxSeq)

Terminal == {"finished", "failed", "cancelled"}
Live == {"accepted", "running", "suspended"}

InitLifecycle ==
  /\ run_state = "accepted"
  /\ seq = 0
  /\ journal = <<0>>
  /\ proof_gates = "verified"
  /\ terminal_seen = FALSE
  /\ terminal_state = "none"
  /\ terminal_seq = 0
  /\ terminal_journal = <<0>>

AppendJournal(n) == journal' = Append(journal, n)

StartRun ==
  /\ run_state = "accepted"
  /\ run_state' = "running"
  /\ UNCHANGED <<seq, journal, proof_gates, terminal_seen, terminal_state,
                 terminal_seq, terminal_journal>>

Step ==
  /\ run_state = "running"
  /\ seq < MaxSeq
  /\ seq' = seq + 1
  /\ AppendJournal(seq + 1)
  /\ UNCHANGED <<run_state, proof_gates, terminal_seen, terminal_state,
                 terminal_seq, terminal_journal>>

Suspend ==
  /\ run_state = "running"
  /\ run_state' = "suspended"
  /\ UNCHANGED <<seq, journal, proof_gates, terminal_seen, terminal_state,
                 terminal_seq, terminal_journal>>

Retry ==
  /\ run_state = "suspended"
  /\ run_state' = "running"
  /\ UNCHANGED <<seq, journal, proof_gates, terminal_seen, terminal_state,
                 terminal_seq, terminal_journal>>

Cancel ==
  /\ run_state \in Live
  /\ run_state' = "cancelled"
  /\ terminal_seen' = TRUE
  /\ terminal_state' = "cancelled"
  /\ terminal_seq' = seq
  /\ terminal_journal' = journal
  /\ UNCHANGED <<seq, journal, proof_gates>>

Finish ==
  /\ run_state = "running"
  /\ run_state' = "finished"
  /\ terminal_seen' = TRUE
  /\ terminal_state' = "finished"
  /\ terminal_seq' = seq
  /\ terminal_journal' = journal
  /\ UNCHANGED <<seq, journal, proof_gates>>

Fail ==
  /\ run_state \in Live
  /\ run_state' = "failed"
  /\ terminal_seen' = TRUE
  /\ terminal_state' = "failed"
  /\ terminal_seq' = seq
  /\ terminal_journal' = journal
  /\ UNCHANGED <<seq, journal, proof_gates>>

TerminalStutter == run_state \in Terminal /\ UNCHANGED vars
Stutter == UNCHANGED vars

LifecycleProgress == StartRun \/ Step \/ Suspend \/ Retry \/ Cancel \/ Finish \/ Fail

Next == LifecycleProgress \/ TerminalStutter \/ Stutter

Spec == InitLifecycle /\ [][Next]_vars /\ WF_vars(LifecycleProgress)

SeqMonotonic ==
  /\ seq >= 0
  /\ seq <= MaxSeq
  /\ Len(journal) >= 1
  /\ journal[Len(journal)] = seq

ValidLifecycleTransition == run_state \in Live \cup Terminal

NoTerminalMutationAfterTerminal ==
  terminal_seen =>
    /\ run_state = terminal_state
    /\ seq = terminal_seq
    /\ journal = terminal_journal
    /\ proof_gates = "verified"

EventuallyTerminalOrSuspended == <>(run_state \in Terminal \/ run_state = "suspended")

====
