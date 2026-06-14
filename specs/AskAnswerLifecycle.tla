(* AskAnswerLifecycle.tla
 *
 * Rust-aligned model for ask answer journaling and timer handling.
 * This model intentionally uses Rust-shaped state: live runs, RuntimeState,
 * pending timer step/kind, per-run journal sequence, and concrete journal events.
 *)

---- MODULE AskAnswerLifecycle ----

EXTENDS Integers, Sequences, TLC, FiniteSets

CONSTANTS RunIds, StepIdxs, SlotIdxs, MaxSeq, MaxJournalEvents, NoStep, NoSlot

VARIABLES runs, runtimeState, terminalRuns, pendingTimerStep, pendingTimerKind,
          nextSeq, journal, framePc, slotWritten, answerPhase, answerRun,
          answerAskStep, answerSlot, result

States == {"Absent", "Initial", "Running", "Resumable", "Resuming", "Failed"}
TimerKinds == {"None", "Ask", "Wait"}
JournalKinds == {"RunAccepted", "AskScheduled", "WaitScheduled", "SlotWritten",
                 "AskAnswered", "StepSucceeded", "RunFinished", "RunFailed"}
AnswerPhases == {"None", "SlotWrittenDone", "AskAnsweredDone"}
Results == {"Ok", "SequenceOverflow", "JournalFull"}

JournalRecord == [kind: JournalKinds,
                  run: RunIds,
                  seq: 0..MaxSeq,
                  step: {NoStep} \cup StepIdxs,
                  slot: {NoSlot} \cup SlotIdxs]

vars == <<runs, runtimeState, terminalRuns, pendingTimerStep, pendingTimerKind,
          nextSeq, journal, framePc, slotWritten, answerPhase, answerRun,
          answerAskStep, answerSlot, result>>

TypeOK ==
    /\ runs \subseteq RunIds
    /\ terminalRuns \subseteq RunIds
    /\ runtimeState \in [RunIds -> States]
    /\ pendingTimerStep \in [RunIds -> {NoStep} \cup StepIdxs]
    /\ pendingTimerKind \in [RunIds -> TimerKinds]
    /\ nextSeq \in [RunIds -> 0..MaxSeq]
    /\ journal \in Seq(JournalRecord)
    /\ Len(journal) <= MaxJournalEvents
    /\ framePc \in [RunIds -> StepIdxs]
    /\ slotWritten \subseteq RunIds \X SlotIdxs
    /\ answerPhase \in AnswerPhases
    /\ answerRun \in RunIds
    /\ answerAskStep \in StepIdxs
    /\ answerSlot \in SlotIdxs
    /\ result \in Results

Init ==
    /\ runs = {}
    /\ runtimeState = [r \in RunIds |-> "Absent"]
    /\ terminalRuns = {}
    /\ pendingTimerStep = [r \in RunIds |-> NoStep]
    /\ pendingTimerKind = [r \in RunIds |-> "None"]
    /\ nextSeq = [r \in RunIds |-> 0]
    /\ journal = <<>>
    /\ framePc = [r \in RunIds |-> CHOOSE s \in StepIdxs : TRUE]
    /\ slotWritten = {}
    /\ answerPhase = "None"
    /\ answerRun = CHOOSE r \in RunIds : TRUE
    /\ answerAskStep = CHOOSE s \in StepIdxs : TRUE
    /\ answerSlot = CHOOSE sl \in SlotIdxs : TRUE
    /\ result = "Ok"

AppendEvent(run, kind, step, slot) ==
    /\ IF Len(journal) < MaxJournalEvents THEN
        /\ journal' = Append(journal, [kind |-> kind, run |-> run, seq |-> nextSeq[run],
                                      step |-> step, slot |-> slot])
        /\ IF nextSeq[run] < MaxSeq THEN
            /\ nextSeq' = [nextSeq EXCEPT ![run] = nextSeq[run] + 1]
            /\ result' = "Ok"
           ELSE
            /\ nextSeq' = nextSeq
            /\ result' = "SequenceOverflow"
       ELSE
        /\ journal' = journal
        /\ nextSeq' = nextSeq
        /\ result' = "JournalFull"

CanAppendOk(run) == Len(journal) < MaxJournalEvents /\ nextSeq[run] < MaxSeq

Submit(run) ==
    /\ result = "Ok"
    /\ run \notin runs
    /\ runs' = runs \cup {run}
    /\ runtimeState' = [runtimeState EXCEPT ![run] = "Initial"]
    /\ terminalRuns' = terminalRuns \ {run}
    /\ framePc' = [framePc EXCEPT ![run] = CHOOSE s \in StepIdxs : TRUE]
    /\ UNCHANGED <<pendingTimerStep, pendingTimerKind, slotWritten,
                  answerPhase, answerRun, answerAskStep, answerSlot>>
    /\ AppendEvent(run, "RunAccepted", NoStep, NoSlot)

AwaitAsk(run, step) ==
    /\ result = "Ok"
    /\ run \in runs
    /\ runtimeState[run] \in {"Initial", "Running", "Resuming"}
    /\ IF CanAppendOk(run) THEN
        /\ runtimeState' = [runtimeState EXCEPT ![run] = "Resumable"]
        /\ pendingTimerStep' = [pendingTimerStep EXCEPT ![run] = step]
        /\ pendingTimerKind' = [pendingTimerKind EXCEPT ![run] = "Ask"]
        /\ framePc' = [framePc EXCEPT ![run] = step]
       ELSE
        /\ runtimeState' = runtimeState
        /\ pendingTimerStep' = pendingTimerStep
        /\ pendingTimerKind' = pendingTimerKind
        /\ framePc' = framePc
    /\ UNCHANGED <<runs, terminalRuns, slotWritten, answerPhase,
                  answerRun, answerAskStep, answerSlot>>
    /\ AppendEvent(run, "AskScheduled", step, NoSlot)

StartAnswer(run, askStep, slot) ==
    /\ result = "Ok"
    /\ run \in runs
    /\ answerPhase = "None"
    /\ pendingTimerKind[run] = "Ask"
    /\ pendingTimerStep[run] = askStep
    /\ pendingTimerStep' = [pendingTimerStep EXCEPT ![run] = NoStep]
    /\ pendingTimerKind' = [pendingTimerKind EXCEPT ![run] = "None"]
    /\ slotWritten' = slotWritten \cup {<<run, slot>>}
    /\ answerPhase' = "SlotWrittenDone"
    /\ answerRun' = run
    /\ answerAskStep' = askStep
    /\ answerSlot' = slot
    /\ UNCHANGED <<runs, runtimeState, terminalRuns, framePc>>
    /\ AppendEvent(run, "SlotWritten", NoStep, slot)

AppendAskAnswered ==
    /\ result = "Ok"
    /\ answerPhase = "SlotWrittenDone"
    /\ answerPhase' = "AskAnsweredDone"
    /\ UNCHANGED <<runs, runtimeState, terminalRuns, pendingTimerStep,
                  pendingTimerKind, framePc, slotWritten, answerRun,
                  answerAskStep, answerSlot>>
    /\ AppendEvent(answerRun, "AskAnswered", answerAskStep, answerSlot)

AppendAskStepSucceeded ==
    /\ result = "Ok"
    /\ answerPhase = "AskAnsweredDone"
    /\ answerPhase' = "None"
    /\ UNCHANGED <<pendingTimerStep, pendingTimerKind, framePc, slotWritten,
                  answerRun, answerAskStep, answerSlot>>
    /\ AppendEvent(answerRun, "StepSucceeded", answerAskStep, answerSlot)
    /\ \/ /\ runtimeState' = [runtimeState EXCEPT ![answerRun] = "Running"]
          /\ runs' = runs
          /\ terminalRuns' = terminalRuns
       \/ /\ runtimeState' = [runtimeState EXCEPT ![answerRun] = "Resumable"]
          /\ runs' = runs
          /\ terminalRuns' = terminalRuns
       \/ /\ runtimeState' = [runtimeState EXCEPT ![answerRun] = "Failed"]
          /\ runs' = runs \ {answerRun}
          /\ terminalRuns' = terminalRuns \cup {answerRun}

Next ==
    \/ \E run \in RunIds : Submit(run)
    \/ \E run \in RunIds, step \in StepIdxs : AwaitAsk(run, step)
    \/ \E run \in RunIds, step \in StepIdxs, slot \in SlotIdxs : StartAnswer(run, step, slot)
    \/ AppendAskAnswered
    \/ AppendAskStepSucceeded
    \/ /\ result # "Ok"
       /\ UNCHANGED vars
    \/ /\ \A r \in RunIds : runtimeState[r] \in {"Absent", "Resumable", "Failed"}
       /\ answerPhase = "None"
       /\ UNCHANGED vars

Fairness ==
    /\ WF_vars(AppendAskAnswered)
    /\ WF_vars(AppendAskStepSucceeded)

Spec == Init /\ [][Next]_vars /\ Fairness

LiveRunsHaveNonAbsentState ==
    \A r \in runs : runtimeState[r] # "Absent"

TerminalRunsNotLive == terminalRuns \cap runs = {}

PendingTimerOnlyForLiveResumableRun ==
    \A r \in RunIds :
        pendingTimerKind[r] # "None" =>
            /\ r \in runs
            /\ runtimeState[r] = "Resumable"

AskTimerImpliesAskScheduled ==
    \A r \in RunIds :
        pendingTimerKind[r] = "Ask" =>
            \E i \in 1..Len(journal) :
                /\ journal[i].kind = "AskScheduled"
                /\ journal[i].run = r
                /\ journal[i].step = pendingTimerStep[r]

NoDuplicateRunSeq ==
    \A i, j \in 1..Len(journal) :
        /\ journal[i].run = journal[j].run
        /\ journal[i].seq = journal[j].seq
        => i = j

PerRunSeqStrictlyIncreasing ==
    \A i, j \in 1..Len(journal) :
        /\ i < j
        /\ journal[i].run = journal[j].run
        => journal[i].seq < journal[j].seq

AskAnsweredAfterSlotWritten ==
    \A i \in 1..Len(journal) :
        journal[i].kind = "AskAnswered" =>
            \E j \in 1..(i - 1) :
                /\ journal[j].kind = "SlotWritten"
                /\ journal[j].run = journal[i].run
                /\ journal[j].slot = journal[i].slot

StateConstraint == Len(journal) <= MaxJournalEvents

THEOREM Spec => []TypeOK
THEOREM Spec => []LiveRunsHaveNonAbsentState
THEOREM Spec => []TerminalRunsNotLive
THEOREM Spec => []PendingTimerOnlyForLiveResumableRun
THEOREM Spec => []AskTimerImpliesAskScheduled
THEOREM Spec => []NoDuplicateRunSeq
THEOREM Spec => []PerRunSeqStrictlyIncreasing
THEOREM Spec => []AskAnsweredAfterSlotWritten

====
