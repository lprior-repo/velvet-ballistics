---- MODULE BoundednessSlice ----

EXTENDS Naturals, TLC

CONSTANT MaxBudget

VARIABLES pc, remaining, signal, workflow_state

Vars == <<pc, remaining, signal, workflow_state>>

TerminalPc == {"Blocked", "Finished", "Errored", "Exhausted"}
TerminalSignal == {"ActionBlocked", "WaitBlocked", "Finished", "TypedError", "StepBudgetExhausted"}

InitSlice ==
    /\ pc = "Running"
    /\ remaining \in 0..MaxBudget
    /\ signal = "None"
    /\ workflow_state = "Active"

TakeStep ==
    /\ pc = "Running"
    /\ remaining > 0
    /\ remaining' = remaining - 1
    /\ signal' = "None"
    /\ workflow_state' = "Active"
    /\ UNCHANGED pc

BlockOnAction ==
    /\ pc = "Running"
    /\ pc' = "Blocked"
    /\ signal' = "ActionBlocked"
    /\ workflow_state' = "Blocked"
    /\ UNCHANGED remaining

BlockOnWait ==
    /\ pc = "Running"
    /\ pc' = "Blocked"
    /\ signal' = "WaitBlocked"
    /\ workflow_state' = "Blocked"
    /\ UNCHANGED remaining

Finish ==
    /\ pc = "Running"
    /\ pc' = "Finished"
    /\ signal' = "Finished"
    /\ workflow_state' = "Terminal"
    /\ UNCHANGED remaining

TypedError ==
    /\ pc = "Running"
    /\ pc' = "Errored"
    /\ signal' = "TypedError"
    /\ workflow_state' = "Terminal"
    /\ UNCHANGED remaining

ExhaustBudget ==
    /\ pc = "Running"
    /\ remaining = 0
    /\ pc' = "Exhausted"
    /\ signal' = "StepBudgetExhausted"
    /\ workflow_state' = "Terminal"
    /\ UNCHANGED remaining

TerminalStutter ==
    /\ pc \in TerminalPc
    /\ UNCHANGED Vars

Next ==
    \/ TakeStep
    \/ BlockOnAction
    \/ BlockOnWait
    \/ Finish
    \/ TypedError
    \/ ExhaustBudget
    \/ TerminalStutter

Spec ==
    /\ InitSlice
    /\ [][Next]_Vars
    /\ WF_Vars(TakeStep)
    /\ WF_Vars(ExhaustBudget)

BudgetNeverNegative == remaining >= 0

NoTransitionAfterExhaust == pc = "Exhausted" => signal = "StepBudgetExhausted" /\ remaining = 0

TypedTerminalOutcome == pc \in TerminalPc => signal \in TerminalSignal
EventuallyBlockedFinishedOrExhausted == <> (pc \in TerminalPc)

StateConstraint == remaining \in 0..MaxBudget

====
