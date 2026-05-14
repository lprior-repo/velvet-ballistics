(* LifecycleJournal.tla
 *
 * Bounded TLC model for vb-qi37.16.5 LifecycleJournal obligations.
 *
 * Contract boundary:
 * - TLA+ owns temporal lifecycle/journal/replay behavior.
 * - Rust/Verus owns local typestate and function-level proof obligations.
 *
 * Finite reductions used by TLC:
 * - one in-flight command at a time (serial CLI/runtime dispatch);
 * - bounded journal length from LifecycleJournal.cfg;
 * - bounded answer domain from LifecycleJournal.cfg;
 * - no symmetry reduction.
 *)

---- MODULE LifecycleJournal ----

EXTENDS Naturals, Sequences, TLC, FiniteSets

CONSTANTS
    MaxJournalLen,
    MaxAnswer,
    Beads

ASSUME MaxJournalLen \in Nat \ {0}
ASSUME MaxAnswer \in Nat
ASSUME IsFiniteSet(Beads)
ASSUME Cardinality(Beads) >= 1

VARIABLES
    bead_state,
    journal,
    commands,
    crashed

vars == <<bead_state, journal, commands, crashed>>

LifecycleState == {"Pending", "Active", "WaitingAnswer", "Completed", "Cancelled", "Failed"}
TerminalState == {"Completed", "Cancelled"}
LifecycleCommand == {"Cancel", "Resume", "Retry", "Answer"}
InternalEvent == {"Start", "NeedAnswer", "Fail"}
EventKind == LifecycleCommand \cup InternalEvent

CommandDomain == [bead_id: Beads, command: LifecycleCommand, answer: 0..MaxAnswer]
EventDomain == [
    bead_id: Beads,
    command: EventKind,
    index: 1..MaxJournalLen,
    answer: 0..MaxAnswer,
    prior: LifecycleState,
    next: LifecycleState
]

TypeInvariant ==
    /\ bead_state \in [Beads -> LifecycleState]
    /\ journal \in Seq(EventDomain)
    /\ Len(journal) <= MaxJournalLen
    /\ commands \in SUBSET CommandDomain
    /\ Cardinality(commands) <= 1
    /\ crashed \in BOOLEAN

Init ==
    /\ bead_state = [b \in Beads |-> "Pending"]
    /\ journal = <<>>
    /\ commands = {}
    /\ crashed = FALSE

CanAppend == Len(journal) < MaxJournalLen

IsActiveOrWaiting(s) == s \in {"Active", "WaitingAnswer"}

IsValidCommandForState(cmd, state) ==
    \/ /\ cmd.command = "Cancel"
       /\ IsActiveOrWaiting(state)
    \/ /\ cmd.command = "Resume"
       /\ state = "Cancelled"
    \/ /\ cmd.command = "Retry"
       /\ state = "Failed"
    \/ /\ cmd.command = "Answer"
       /\ state = "WaitingAnswer"

IsValidCommand(cmd, b) == IsValidCommandForState(cmd, bead_state[b])

CommandNextState(cmd, state) ==
    IF cmd.command = "Cancel" THEN "Cancelled"
    ELSE IF cmd.command = "Resume" THEN "Active"
    ELSE IF cmd.command = "Retry" THEN "Active"
    ELSE IF cmd.command = "Answer" THEN "Completed"
    ELSE state

EventNextState(e) ==
    IF e.command = "Start" THEN "Active"
    ELSE IF e.command = "NeedAnswer" THEN "WaitingAnswer"
    ELSE IF e.command = "Fail" THEN "Failed"
    ELSE IF e.command = "Cancel" THEN "Cancelled"
    ELSE IF e.command = "Resume" THEN "Active"
    ELSE IF e.command = "Retry" THEN "Active"
    ELSE IF e.command = "Answer" THEN "Completed"
    ELSE e.prior

ValidEvent(e) ==
    /\ e.next = EventNextState(e)
    /\ CASE e.command = "Start" -> e.prior = "Pending"
          [] e.command = "NeedAnswer" -> e.prior = "Active"
          [] e.command = "Fail" -> e.prior = "Active"
          [] e.command \in LifecycleCommand ->
                IsValidCommandForState([bead_id |-> e.bead_id,
                                        command |-> e.command,
                                        answer |-> e.answer], e.prior)

EventFor(b, kind, ans, prior, nextState) ==
    [bead_id |-> b,
     command |-> kind,
     index |-> Len(journal) + 1,
     answer |-> ans,
     prior |-> prior,
     next |-> nextState]

AlreadyAccepted(cmd) ==
    \E i \in 1..Len(journal) :
        /\ journal[i].bead_id = cmd.bead_id
        /\ journal[i].command = cmd.command
        /\ journal[i].answer = cmd.answer

EventAccepted(b, kind) ==
    \E i \in 1..Len(journal) :
        /\ journal[i].bead_id = b
        /\ journal[i].command = kind

AppendEvent(b, kind, ans, nextState) ==
    /\ CanAppend
    /\ journal' = Append(journal, EventFor(b, kind, ans, bead_state[b], nextState))
    /\ bead_state' = [bead_state EXCEPT ![b] = nextState]
    /\ crashed' = crashed

Start(b) ==
    /\ ~crashed
    /\ commands = {}
    /\ bead_state[b] = "Pending"
    /\ AppendEvent(b, "Start", 0, "Active")
    /\ UNCHANGED commands

NeedAnswer(b) ==
    /\ ~crashed
    /\ commands = {}
    /\ bead_state[b] = "Active"
    /\ ~EventAccepted(b, "NeedAnswer")
    /\ AppendEvent(b, "NeedAnswer", 0, "WaitingAnswer")
    /\ UNCHANGED commands

Fail(b) ==
    /\ ~crashed
    /\ commands = {}
    /\ bead_state[b] = "Active"
    /\ ~EventAccepted(b, "Fail")
    /\ AppendEvent(b, "Fail", 0, "Failed")
    /\ UNCHANGED commands

Submit(cmd) ==
    /\ ~crashed
    /\ commands = {}
    /\ commands' = {cmd}
    /\ UNCHANGED <<bead_state, journal, crashed>>

TerminalCommandFor(b) ==
    IF bead_state[b] = "Active" THEN [bead_id |-> b, command |-> "Cancel", answer |-> 0]
    ELSE IF bead_state[b] = "WaitingAnswer" THEN [bead_id |-> b, command |-> "Answer", answer |-> 0]
    ELSE IF bead_state[b] = "Failed" THEN [bead_id |-> b, command |-> "Retry", answer |-> 0]
    ELSE [bead_id |-> b, command |-> "Cancel", answer |-> 0]

SubmitTerminal(b) ==
    /\ bead_state[b] \in {"Active", "WaitingAnswer", "Failed"}
    /\ Submit(TerminalCommandFor(b))

InvalidCommandFor(b) ==
    IF bead_state[b] = "Pending" THEN [bead_id |-> b, command |-> "Cancel", answer |-> 0]
    ELSE IF bead_state[b] = "Active" THEN [bead_id |-> b, command |-> "Answer", answer |-> 0]
    ELSE IF bead_state[b] = "WaitingAnswer" THEN [bead_id |-> b, command |-> "Resume", answer |-> 0]
    ELSE IF bead_state[b] = "Failed" THEN [bead_id |-> b, command |-> "Answer", answer |-> 0]
    ELSE [bead_id |-> b, command |-> "Cancel", answer |-> 0]

SubmitInvalid(b) == Submit(InvalidCommandFor(b))

SubmitDuplicate(b) ==
    /\ \E i \in 1..Len(journal) :
        /\ journal[i].bead_id = b
        /\ journal[i].command \in LifecycleCommand
        /\ Submit([bead_id |-> b,
                   command |-> journal[i].command,
                   answer |-> journal[i].answer])

AcceptCommand(cmd) ==
    LET b == cmd.bead_id IN
    LET nextState == CommandNextState(cmd, bead_state[b]) IN
        /\ IsValidCommand(cmd, b)
        /\ ~AlreadyAccepted(cmd)
        /\ AppendEvent(b, cmd.command, cmd.answer, nextState)
        /\ commands' = {}

RejectCommand(cmd) ==
    /\ \/ ~IsValidCommand(cmd, cmd.bead_id)
       \/ AlreadyAccepted(cmd)
       \/ ~CanAppend
    /\ commands' = {}
    /\ UNCHANGED <<bead_state, journal, crashed>>

ProcessCommand(cmd) ==
    /\ ~crashed
    /\ cmd \in commands
    /\ IF IsValidCommand(cmd, cmd.bead_id) /\ ~AlreadyAccepted(cmd) /\ CanAppend
       THEN AcceptCommand(cmd)
       ELSE RejectCommand(cmd)

Process == \E cmd \in CommandDomain : ProcessCommand(cmd)

Crash ==
    /\ ~crashed
    /\ commands = {}
    /\ journal /= <<>>
    /\ crashed' = TRUE
    /\ commands' = {}
    /\ UNCHANGED <<bead_state, journal>>

MaxOf(s) == CHOOSE x \in s : \A y \in s : y <= x

ReplayStateFor(b) ==
    LET relevant == {i \in 1..Len(journal) : journal[i].bead_id = b} IN
        IF relevant = {} THEN "Pending"
        ELSE journal[MaxOf(relevant)].next

ReplayState == [b \in Beads |-> ReplayStateFor(b)]

Replay ==
    /\ crashed
    /\ bead_state' = ReplayState
    /\ crashed' = FALSE
    /\ commands' = {}
    /\ UNCHANGED journal

Next ==
    \/ \E b \in Beads : Start(b)
    \/ \E b \in Beads : NeedAnswer(b)
    \/ \E b \in Beads : Fail(b)
    \/ \E b \in Beads : SubmitTerminal(b)
    \/ \E b \in Beads : SubmitInvalid(b)
    \/ \E b \in Beads : SubmitDuplicate(b)
    \/ Process
    \/ Crash
    \/ Replay

Spec ==
    /\ Init
    /\ [][Next]_vars
    /\ WF_vars(Process)
    /\ WF_vars(Replay)
    /\ \A b \in Beads : SF_vars(Start(b))
    /\ \A b \in Beads : SF_vars(SubmitTerminal(b))

NoOverwrite ==
    /\ \A i \in 1..Len(journal) : journal[i].index = i
    /\ \A i, j \in 1..Len(journal) :
        (i /= j) => journal[i].index /= journal[j].index

SingleCanonicalState ==
    \A b \in Beads : bead_state[b] \in LifecycleState

InvalidTransitionBlocked ==
    \A i \in 1..Len(journal) : ValidEvent(journal[i])

ReplayBitIdentical == bead_state = ReplayState

EventuallyTerminalOrCancelled ==
    \A b \in Beads : <> (bead_state[b] \in TerminalState)

JournalGrowth ==
    [] (Len(journal) <= MaxJournalLen)

StateConstraint == Len(journal) <= MaxJournalLen

====
