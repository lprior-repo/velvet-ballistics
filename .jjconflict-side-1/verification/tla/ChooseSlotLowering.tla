---- MODULE ChooseSlotLowering ----

EXTENDS Naturals, FiniteSets, TLC

CONSTANTS FanoutLimit, MaxInputBranches, MaxSlots, MaxSteps, MaxLabels, MaxU16, NoOtherwise


VARIABLES inputKind, branchCount, branchConditions, branchTargets, branchBodiesEmpty,
          hasOtherwise, otherwiseLabelKnown, otherwiseLabelIndex, hasNext, nextTarget,
          builderRecordedSlots, loweredBranches, loweredOtherwise, result, phase

Kinds == {"slot", "canonical"}
Phases == {"start", "done"}
Results == {"pending", "ok_choose_slot", "err_fanout_limit", "err_empty_branch_table",
            "err_missing_next", "err_nonempty_branch_body", "err_unknown_otherwise_label",
            "err_otherwise_target_over_u16"}

vars == <<inputKind, branchCount, branchConditions, branchTargets, branchBodiesEmpty,
          hasOtherwise, otherwiseLabelKnown, otherwiseLabelIndex, hasNext, nextTarget,
          builderRecordedSlots, loweredBranches, loweredOtherwise, result, phase>>

SlotTarget == [condition: 0..MaxSlots, target: 0..MaxSteps]

TypeOK ==
    /\ inputKind \in Kinds
    /\ branchCount \in 0..MaxInputBranches
    /\ branchConditions \in [1..MaxInputBranches -> 0..MaxSlots]
    /\ branchTargets \in [1..MaxInputBranches -> 0..MaxSteps]
    /\ branchBodiesEmpty \in BOOLEAN
    /\ hasOtherwise \in BOOLEAN
    /\ otherwiseLabelKnown \in BOOLEAN
    /\ otherwiseLabelIndex \in 0..MaxLabels
    /\ hasNext \in BOOLEAN
    /\ nextTarget \in 0..MaxSteps
    /\ builderRecordedSlots \subseteq 0..MaxSlots
    /\ loweredBranches \in [1..MaxInputBranches -> SlotTarget]
    /\ loweredOtherwise \in {NoOtherwise} \cup 0..MaxSteps
    /\ result \in Results
    /\ phase \in Phases

Init ==
    /\ inputKind \in Kinds
    /\ branchCount \in 0..MaxInputBranches
    /\ branchConditions \in [1..MaxInputBranches -> 0..MaxSlots]
    /\ branchTargets \in [1..MaxInputBranches -> 0..MaxSteps]
    /\ branchBodiesEmpty \in BOOLEAN
    /\ hasOtherwise \in BOOLEAN
    /\ otherwiseLabelKnown \in BOOLEAN
    /\ otherwiseLabelIndex \in 0..MaxLabels
    /\ hasNext \in BOOLEAN
    /\ nextTarget \in 0..MaxSteps
    /\ builderRecordedSlots = {}
    /\ loweredBranches = [i \in 1..MaxInputBranches |-> [condition |-> 0, target |-> 0]]
    /\ loweredOtherwise = NoOtherwise
    /\ result = "pending"
    /\ phase = "start"

ConditionSlots == {branchConditions[i] : i \in 1..branchCount}

SlotLoweredBranches ==
    [i \in 1..MaxInputBranches |->
        IF i \in 1..branchCount THEN
            [condition |-> branchConditions[i], target |-> branchTargets[i]]
        ELSE [condition |-> 0, target |-> 0]]

CanonicalLoweredBranches ==
    [i \in 1..MaxInputBranches |->
        IF i \in 1..branchCount THEN
            [condition |-> branchConditions[i], target |-> nextTarget]
        ELSE [condition |-> 0, target |-> 0]]

LowerSlot ==
    /\ inputKind = "slot"
    /\ phase = "start"
    /\ phase' = "done"
    /\ UNCHANGED <<inputKind, branchCount, branchConditions, branchTargets,
                  branchBodiesEmpty, hasOtherwise, otherwiseLabelKnown,
                  otherwiseLabelIndex, hasNext, nextTarget>>
    /\ IF branchCount > FanoutLimit THEN
        /\ result' = "err_fanout_limit"
        /\ builderRecordedSlots' = builderRecordedSlots
        /\ loweredBranches' = loweredBranches
        /\ loweredOtherwise' = loweredOtherwise
       ELSE IF branchCount = 0 /\ ~hasOtherwise THEN
        /\ result' = "err_empty_branch_table"
        /\ builderRecordedSlots' = builderRecordedSlots
        /\ loweredBranches' = loweredBranches
        /\ loweredOtherwise' = loweredOtherwise
       ELSE
        /\ result' = "ok_choose_slot"
        /\ builderRecordedSlots' = ConditionSlots
        /\ loweredBranches' = SlotLoweredBranches
        /\ loweredOtherwise' = IF hasOtherwise THEN otherwiseLabelIndex ELSE NoOtherwise

LowerCanonical ==
    /\ inputKind = "canonical"
    /\ phase = "start"
    /\ phase' = "done"
    /\ UNCHANGED <<inputKind, branchCount, branchConditions, branchTargets,
                  branchBodiesEmpty, hasOtherwise, otherwiseLabelKnown,
                  otherwiseLabelIndex, hasNext, nextTarget>>
    /\ IF branchCount > FanoutLimit THEN
        /\ result' = "err_fanout_limit"
        /\ builderRecordedSlots' = builderRecordedSlots
        /\ loweredBranches' = loweredBranches
        /\ loweredOtherwise' = loweredOtherwise
       ELSE IF branchCount = 0 /\ ~hasOtherwise THEN
        /\ result' = "err_empty_branch_table"
        /\ builderRecordedSlots' = builderRecordedSlots
        /\ loweredBranches' = loweredBranches
        /\ loweredOtherwise' = loweredOtherwise
       ELSE IF ~hasNext THEN
        /\ result' = "err_missing_next"
        /\ builderRecordedSlots' = builderRecordedSlots
        /\ loweredBranches' = loweredBranches
        /\ loweredOtherwise' = loweredOtherwise
       ELSE IF ~branchBodiesEmpty THEN
        /\ result' = "err_nonempty_branch_body"
        /\ builderRecordedSlots' = builderRecordedSlots
        /\ loweredBranches' = loweredBranches
        /\ loweredOtherwise' = loweredOtherwise
       ELSE IF hasOtherwise /\ ~otherwiseLabelKnown THEN
        /\ result' = "err_unknown_otherwise_label"
        /\ builderRecordedSlots' = builderRecordedSlots
        /\ loweredBranches' = loweredBranches
        /\ loweredOtherwise' = loweredOtherwise
       ELSE IF hasOtherwise /\ otherwiseLabelIndex > MaxU16 THEN
        /\ result' = "err_otherwise_target_over_u16"
        /\ builderRecordedSlots' = builderRecordedSlots
        /\ loweredBranches' = loweredBranches
        /\ loweredOtherwise' = loweredOtherwise
       ELSE
        /\ result' = "ok_choose_slot"
        /\ builderRecordedSlots' = ConditionSlots
        /\ loweredBranches' = CanonicalLoweredBranches
        /\ loweredOtherwise' = IF hasOtherwise THEN otherwiseLabelIndex ELSE NoOtherwise

Next == LowerSlot \/ LowerCanonical
Spec == Init /\ [][Next]_vars

NoSuccessfulOverLimitLowering ==
    branchCount > FanoutLimit => result # "ok_choose_slot"

NoEmptyBranchTableSuccessWithoutOtherwise ==
    branchCount = 0 /\ ~hasOtherwise => result # "ok_choose_slot"

SuccessRecordsEveryConditionSlot ==
    result = "ok_choose_slot" => builderRecordedSlots = ConditionSlots

SlotSuccessPreservesBranches ==
    result = "ok_choose_slot" /\ inputKind = "slot" =>
        \A i \in 1..branchCount :
            /\ loweredBranches[i].condition = branchConditions[i]
            /\ loweredBranches[i].target = branchTargets[i]

CanonicalSuccessTargetsNext ==
    result = "ok_choose_slot" /\ inputKind = "canonical" =>
        \A i \in 1..branchCount :
            /\ loweredBranches[i].condition = branchConditions[i]
            /\ loweredBranches[i].target = nextTarget

SuccessPreservesOtherwise ==
    result = "ok_choose_slot" => loweredOtherwise = IF hasOtherwise THEN otherwiseLabelIndex ELSE NoOtherwise

THEOREM Spec => []TypeOK
THEOREM Spec => []NoSuccessfulOverLimitLowering
THEOREM Spec => []NoEmptyBranchTableSuccessWithoutOtherwise
THEOREM Spec => []SuccessRecordsEveryConditionSlot
THEOREM Spec => []SlotSuccessPreservesBranches
THEOREM Spec => []CanonicalSuccessTargetsNext
THEOREM Spec => []SuccessPreservesOtherwise

====
