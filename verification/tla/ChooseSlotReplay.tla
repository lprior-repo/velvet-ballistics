---- MODULE ChooseSlotReplay ----

EXTENDS Naturals, Sequences, TLC

CONSTANTS MaxBranches, MaxSlots, MaxSteps, MaxExecuted, NoOtherwise

VARIABLES branchCount, branches, slotState, otherwise, scanIndex,
          pc, pc0, executed, executed0, selectedIndex, outcome, phase

SlotValues == {"true", "false", "uninitialized", "non_bool", "out_of_bounds"}
Outcomes == {"pending", "continue_branch", "continue_otherwise", "err_slot_not_available",
             "err_non_bool", "err_no_otherwise", "err_executed_overflow"}
Phases == {"scanning", "done"}
Branch == [condition: 0..MaxSlots, target: 0..MaxSteps]

vars == <<branchCount, branches, slotState, otherwise, scanIndex,
          pc, pc0, executed, executed0, selectedIndex, outcome, phase>>

TypeOK ==
    /\ branchCount \in 0..MaxBranches
    /\ branches \in [1..MaxBranches -> Branch]
    /\ slotState \in [0..MaxSlots -> SlotValues]
    /\ otherwise \in {NoOtherwise} \cup 0..MaxSteps
    /\ scanIndex \in 1..(MaxBranches + 1)
    /\ pc \in 0..MaxSteps
    /\ pc0 \in 0..MaxSteps
    /\ executed \in 0..MaxExecuted
    /\ executed0 \in 0..MaxExecuted
    /\ selectedIndex \in 0..MaxBranches
    /\ outcome \in Outcomes
    /\ phase \in Phases

Init ==
    /\ branchCount \in 0..MaxBranches
    /\ branches \in [1..MaxBranches -> Branch]
    /\ slotState \in [0..MaxSlots -> SlotValues]
    /\ otherwise \in {NoOtherwise} \cup 0..MaxSteps
    /\ pc \in 0..MaxSteps
    /\ pc0 = pc
    /\ executed \in 0..MaxExecuted
    /\ executed0 = executed
    /\ scanIndex = 1
    /\ selectedIndex = 0
    /\ outcome = "pending"
    /\ phase = "scanning"

CurrentSlot == branches[scanIndex].condition
CurrentTarget == branches[scanIndex].target

ReadSlotUnavailable ==
    /\ phase = "scanning"
    /\ scanIndex <= branchCount
    /\ slotState[CurrentSlot] \in {"uninitialized", "out_of_bounds"}
    /\ outcome' = "err_slot_not_available"
    /\ phase' = "done"
    /\ UNCHANGED <<branchCount, branches, slotState, otherwise, scanIndex,
                  pc, pc0, executed, executed0, selectedIndex>>

ReadSlotNonBool ==
    /\ phase = "scanning"
    /\ scanIndex <= branchCount
    /\ slotState[CurrentSlot] = "non_bool"
    /\ outcome' = "err_non_bool"
    /\ phase' = "done"
    /\ UNCHANGED <<branchCount, branches, slotState, otherwise, scanIndex,
                  pc, pc0, executed, executed0, selectedIndex>>

ScanFalseBranch ==
    /\ phase = "scanning"
    /\ scanIndex <= branchCount
    /\ slotState[CurrentSlot] = "false"
    /\ scanIndex' = scanIndex + 1
    /\ UNCHANGED <<branchCount, branches, slotState, otherwise,
                  pc, pc0, executed, executed0, selectedIndex, outcome, phase>>

SelectTrueBranch ==
    /\ phase = "scanning"
    /\ scanIndex <= branchCount
    /\ slotState[CurrentSlot] = "true"
    /\ pc' = CurrentTarget
    /\ selectedIndex' = scanIndex
    /\ phase' = "done"
    /\ IF executed = MaxExecuted THEN
        /\ executed' = executed
        /\ outcome' = "err_executed_overflow"
       ELSE
        /\ executed' = executed + 1
        /\ outcome' = "continue_branch"
    /\ UNCHANGED <<branchCount, branches, slotState, otherwise, scanIndex, pc0, executed0>>

SelectOtherwise ==
    /\ phase = "scanning"
    /\ scanIndex = branchCount + 1
    /\ otherwise # NoOtherwise
    /\ pc' = otherwise
    /\ selectedIndex' = 0
    /\ phase' = "done"
    /\ IF executed = MaxExecuted THEN
        /\ executed' = executed
        /\ outcome' = "err_executed_overflow"
       ELSE
        /\ executed' = executed + 1
        /\ outcome' = "continue_otherwise"
    /\ UNCHANGED <<branchCount, branches, slotState, otherwise, scanIndex, pc0, executed0>>

FailNoOtherwise ==
    /\ phase = "scanning"
    /\ scanIndex = branchCount + 1
    /\ otherwise = NoOtherwise
    /\ outcome' = "err_no_otherwise"
    /\ phase' = "done"
    /\ UNCHANGED <<branchCount, branches, slotState, otherwise, scanIndex,
                  pc, pc0, executed, executed0, selectedIndex>>

Next ==
    \/ ReadSlotUnavailable
    \/ ReadSlotNonBool
    \/ ScanFalseBranch
    \/ SelectTrueBranch
    \/ SelectOtherwise
    \/ FailNoOtherwise

Spec == Init /\ [][Next]_vars

ScanIndexBounded == scanIndex \in 1..(branchCount + 1)

FirstTrueWins ==
    outcome = "continue_branch" =>
        /\ selectedIndex \in 1..branchCount
        /\ slotState[branches[selectedIndex].condition] = "true"
        /\ \A i \in 1..(selectedIndex - 1) : slotState[branches[i].condition] = "false"

OtherwiseOnlyAfterAllFalse ==
    outcome = "continue_otherwise" =>
        \A i \in 1..branchCount : slotState[branches[i].condition] = "false"

SuccessIncrementsExecutedExactlyOnce ==
    outcome \in {"continue_branch", "continue_otherwise"} => executed = executed0 + 1

FailureDoesNotIncrementExecuted ==
    outcome \in {"err_slot_not_available", "err_non_bool", "err_no_otherwise", "err_executed_overflow"} => executed = executed0

EarlyErrorsDoNotAdvancePc ==
    outcome \in {"err_slot_not_available", "err_non_bool", "err_no_otherwise"} => pc = pc0

THEOREM Spec => []TypeOK
THEOREM Spec => []ScanIndexBounded
THEOREM Spec => []FirstTrueWins
THEOREM Spec => []OtherwiseOnlyAfterAllFalse
THEOREM Spec => []SuccessIncrementsExecutedExactlyOnce
THEOREM Spec => []FailureDoesNotIncrementExecuted
THEOREM Spec => []EarlyErrorsDoNotAdvancePc

====
