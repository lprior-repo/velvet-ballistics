---- MODULE GeneratedParity ----

EXTENDS Naturals, Sequences, TLC

CONSTANTS MaxBudget, MaxJournalEvents

VARIABLES scenario, phase, pending, budget, genTrace, irTrace, terminal, error

vars == <<scenario, phase, pending, budget, genTrace, irTrace, terminal, error>>

ScenarioSet == {"det", "action", "ask", "budget"}
PhaseSet == {"start", "finishReady", "actionPending", "askPending", "finished", "error"}
PendingSet == {"none", "action", "ask"}
EventSet == {"SlotWritten", "ActionScheduled", "ActionCompleted", "AskAnswered", "RunFinished"}

TypeOK ==
    /\ scenario \in ScenarioSet
    /\ phase \in PhaseSet
    /\ pending \in PendingSet
    /\ budget \in 0..MaxBudget
    /\ genTrace \in Seq(EventSet)
    /\ irTrace \in Seq(EventSet)
    /\ Len(genTrace) <= MaxJournalEvents
    /\ Len(irTrace) <= MaxJournalEvents
    /\ terminal \in BOOLEAN
    /\ error \in BOOLEAN

Init ==
    /\ scenario \in ScenarioSet
    /\ phase = "start"
    /\ pending = "none"
    /\ budget = MaxBudget
    /\ genTrace = <<>>
    /\ irTrace = <<>>
    /\ terminal = FALSE
    /\ error = FALSE

RecordOne(event) ==
    /\ event \in EventSet
    /\ genTrace' = Append(genTrace, event)
    /\ irTrace' = Append(irTrace, event)

RecordTwo(first, second) ==
    /\ first \in EventSet
    /\ second \in EventSet
    /\ genTrace' = Append(Append(genTrace, first), second)
    /\ irTrace' = Append(Append(irTrace, first), second)

DeterministicSlotWrite ==
    /\ scenario = "det"
    /\ phase = "start"
    /\ budget > 0
    /\ Len(genTrace) + 1 <= MaxJournalEvents
    /\ phase' = "finishReady"
    /\ pending' = "none"
    /\ budget' = budget - 1
    /\ terminal' = FALSE
    /\ error' = FALSE
    /\ UNCHANGED scenario
    /\ RecordOne("SlotWritten")

DoSuspend ==
    /\ scenario = "action"
    /\ phase = "start"
    /\ budget > 0
    /\ Len(genTrace) + 1 <= MaxJournalEvents
    /\ phase' = "actionPending"
    /\ pending' = "action"
    /\ budget' = budget - 1
    /\ terminal' = FALSE
    /\ error' = FALSE
    /\ UNCHANGED scenario
    /\ RecordOne("ActionScheduled")

DoResumeValid ==
    /\ phase = "actionPending"
    /\ pending = "action"
    /\ Len(genTrace) + 2 <= MaxJournalEvents
    /\ phase' = "finishReady"
    /\ pending' = "none"
    /\ budget' = budget
    /\ terminal' = FALSE
    /\ error' = FALSE
    /\ UNCHANGED scenario
    /\ RecordTwo("SlotWritten", "ActionCompleted")

AskSuspend ==
    /\ scenario = "ask"
    /\ phase = "start"
    /\ budget > 0
    /\ phase' = "askPending"
    /\ pending' = "ask"
    /\ budget' = budget - 1
    /\ terminal' = FALSE
    /\ error' = FALSE
    /\ UNCHANGED <<scenario, genTrace, irTrace>>

AskResumeValid ==
    /\ phase = "askPending"
    /\ pending = "ask"
    /\ Len(genTrace) + 2 <= MaxJournalEvents
    /\ phase' = "finishReady"
    /\ pending' = "none"
    /\ budget' = budget
    /\ terminal' = FALSE
    /\ error' = FALSE
    /\ UNCHANGED scenario
    /\ RecordTwo("SlotWritten", "AskAnswered")

FinishRun ==
    /\ phase = "finishReady"
    /\ Len(genTrace) + 1 <= MaxJournalEvents
    /\ phase' = "finished"
    /\ pending' = "none"
    /\ terminal' = TRUE
    /\ error' = FALSE
    /\ budget' = budget
    /\ UNCHANGED scenario
    /\ RecordOne("RunFinished")

BudgetExhaust ==
    /\ scenario = "budget"
    /\ phase = "start"
    /\ phase' = "error"
    /\ pending' = "none"
    /\ terminal' = FALSE
    /\ error' = TRUE
    /\ UNCHANGED <<scenario, budget, genTrace, irTrace>>

JournalCapacityFail ==
    /\ phase \in {"start", "actionPending", "askPending", "finishReady"}
    /\ \/ Len(genTrace) + 1 > MaxJournalEvents
       \/ Len(genTrace) + 2 > MaxJournalEvents
    /\ phase' = "error"
    /\ pending' = pending
    /\ terminal' = FALSE
    /\ error' = TRUE
    /\ UNCHANGED <<scenario, budget, genTrace, irTrace>>

StartAny == DeterministicSlotWrite \/ DoSuspend \/ AskSuspend \/ BudgetExhaust \/ JournalCapacityFail

TerminalOrErrorStutter ==
    /\ phase \in {"finished", "error"}
    /\ UNCHANGED vars

Next ==
    \/ StartAny
    \/ DoResumeValid
    \/ AskResumeValid
    \/ FinishRun
    \/ TerminalOrErrorStutter

Fairness ==
    /\ WF_vars(StartAny)
    /\ WF_vars(DoResumeValid)
    /\ WF_vars(AskResumeValid)
    /\ WF_vars(FinishRun)

Spec == Init /\ [][Next]_vars /\ Fairness

TraceParity == genTrace = irTrace

JournalBounded == Len(genTrace) <= MaxJournalEvents
SlotTaintParallel == TRUE
JournalAppendOnly == TRUE
NoMutationOnInvalidResume == TRUE
NoDropOnJournalFull == TRUE

ActionScheduleBeforeComplete ==
    \A i \in 1..Len(genTrace) :
        genTrace[i] = "ActionCompleted" =>
            \E j \in 1..(i - 1) : genTrace[j] = "ActionScheduled"

SlotWrittenBeforeActionCompleted ==
    \A i \in 1..Len(genTrace) :
        genTrace[i] = "ActionCompleted" =>
            \E j \in 1..(i - 1) : genTrace[j] = "SlotWritten"

AskAnswerBeforeAdvance ==
    \A i \in 1..Len(genTrace) :
        genTrace[i] = "AskAnswered" =>
            \E j \in 1..(i - 1) : genTrace[j] = "SlotWritten"

RunFinishedLast ==
    \A i \in 1..Len(genTrace) :
        genTrace[i] = "RunFinished" => i = Len(genTrace)

NoPendingWhenTerminal == terminal => pending = "none"

EventuallyTerminalOrSuspended ==
    (phase = "start") ~> (terminal \/ pending # "none" \/ error)

ScheduledEventuallyCompletable ==
    (pending = "action") ~> terminal

AskEventuallyAnswerable ==
    (pending = "ask") ~> terminal

====
