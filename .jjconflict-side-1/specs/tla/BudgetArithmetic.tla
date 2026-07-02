(* BudgetArithmetic.tla
 *
 * Bounded model of AggregateResourceUsage::{try_add_budget,try_subtract_budget}
 * from crates/vb_core/src/budget.rs.
 *
 * TLC's integer implementation cannot fingerprint u32::MAX/u64::MAX directly.
 * This model represents every Rust u64 as four exact 16-bit limbs and performs
 * checked add/subtract with explicit carry/borrow. MAX_U16 is numeric; MAX_U32
 * is [MAX_U16, MAX_U16, 0, 0]; MAX_U64 is all MAX_U16 limbs.
 *)

---- MODULE BudgetArithmetic ----

EXTENDS Integers, FiniteSets, TLC

MAX_U16 == 65535
BASE == 65536

UsageFields == {
    "max_steps_executable",
    "max_action_tickets",
    "max_parallel_in_flight",
    "max_gather_pages",
    "max_gather_items",
    "max_result_bytes",
    "max_total_slots_written",
    "max_active_runs",
    "max_queue_depth",
    "max_journal_batch_bytes",
    "max_step_budget_per_tick",
    "max_transitions_per_tick"
}

BudgetFields == UsageFields \ {"max_active_runs"}

Phases == {"Ready", "Done"}

Statuses == {"Running", "Suspended", "Failed"}

GracefulErrorStatuses == {"Suspended", "Failed"}

U16BudgetFields == {"max_parallel_in_flight"}

U32BudgetFields == {
    "max_steps_executable",
    "max_action_tickets",
    "max_gather_pages",
    "max_gather_items",
    "max_result_bytes",
    "max_total_slots_written",
    "max_queue_depth",
    "max_journal_batch_bytes"
}

ZeroWord == [l0 |-> 0, l1 |-> 0, l2 |-> 0, l3 |-> 0]
OneWord == [l0 |-> 1, l1 |-> 0, l2 |-> 0, l3 |-> 0]
MaxU16Word == [l0 |-> MAX_U16, l1 |-> 0, l2 |-> 0, l3 |-> 0]
NearMaxU16Word == [l0 |-> MAX_U16 - 1, l1 |-> 0, l2 |-> 0, l3 |-> 0]
OnePastU16Word == [l0 |-> 0, l1 |-> 1, l2 |-> 0, l3 |-> 0]
MaxU32Word == [l0 |-> MAX_U16, l1 |-> MAX_U16, l2 |-> 0, l3 |-> 0]
NearMaxU32Word == [l0 |-> MAX_U16 - 1, l1 |-> MAX_U16, l2 |-> 0, l3 |-> 0]
OnePastU32Word == [l0 |-> 0, l1 |-> 0, l2 |-> 1, l3 |-> 0]
NearMaxU64Word == [l0 |-> MAX_U16 - 1, l1 |-> MAX_U16, l2 |-> MAX_U16, l3 |-> MAX_U16]
MaxU64Word == [l0 |-> MAX_U16, l1 |-> MAX_U16, l2 |-> MAX_U16, l3 |-> MAX_U16]

WordTypeOK(word) ==
    /\ DOMAIN word = {"l0", "l1", "l2", "l3"}
    /\ word.l0 \in 0..MAX_U16
    /\ word.l1 \in 0..MAX_U16
    /\ word.l2 \in 0..MAX_U16
    /\ word.l3 \in 0..MAX_U16

WordLT(a, b) ==
    \/ a.l3 < b.l3
    \/ /\ a.l3 = b.l3
       /\ a.l2 < b.l2
    \/ /\ a.l3 = b.l3
       /\ a.l2 = b.l2
       /\ a.l1 < b.l1
    \/ /\ a.l3 = b.l3
       /\ a.l2 = b.l2
       /\ a.l1 = b.l1
       /\ a.l0 < b.l0

WordLE(a, b) == WordLT(a, b) \/ a = b

BudgetFieldMax(field) ==
    IF field \in U16BudgetFields THEN MaxU16Word
    ELSE IF field \in U32BudgetFields THEN MaxU32Word
    ELSE MaxU64Word

BudgetAmount(budget, field) ==
    IF field = "max_active_runs" THEN OneWord ELSE budget[field]

Carry(sum) == IF sum <= MAX_U16 THEN 0 ELSE 1

Limb(sum) == IF sum <= MAX_U16 THEN sum ELSE sum - BASE

AddWord(a, b) ==
    LET s0 == a.l0 + b.l0
        r0 == Limb(s0)
        c0 == Carry(s0)
        s1 == a.l1 + b.l1 + c0
        r1 == Limb(s1)
        c1 == Carry(s1)
        s2 == a.l2 + b.l2 + c1
        r2 == Limb(s2)
        c2 == Carry(s2)
        s3 == a.l3 + b.l3 + c2
        r3 == Limb(s3)
        c3 == Carry(s3)
    IN IF c3 = 0
       THEN [tag |-> "Ok", value |-> [l0 |-> r0, l1 |-> r1, l2 |-> r2, l3 |-> r3]]
       ELSE [tag |-> "Err", error |-> "Overflow"]

Borrow(diff) == IF diff < 0 THEN 1 ELSE 0

SubLimb(diff) == IF diff < 0 THEN diff + BASE ELSE diff

SubWord(a, b) ==
    IF WordLE(b, a)
    THEN LET d0 == a.l0 - b.l0
             r0 == SubLimb(d0)
             b0 == Borrow(d0)
             d1 == a.l1 - b.l1 - b0
             r1 == SubLimb(d1)
             b1 == Borrow(d1)
             d2 == a.l2 - b.l2 - b1
             r2 == SubLimb(d2)
             b2 == Borrow(d2)
             d3 == a.l3 - b.l3 - b2
             r3 == SubLimb(d3)
         IN [tag |-> "Ok", value |-> [l0 |-> r0, l1 |-> r1, l2 |-> r2, l3 |-> r3]]
    ELSE [tag |-> "Err", error |-> "Underflow"]

UsageTypeOK(usage) ==
    /\ DOMAIN usage = UsageFields
    /\ \A field \in UsageFields : WordTypeOK(usage[field])

BudgetTypeOK(budget) ==
    /\ DOMAIN budget = BudgetFields
    /\ \A field \in BudgetFields :
        /\ WordTypeOK(budget[field])
        /\ WordLE(budget[field], BudgetFieldMax(field))

ZeroUsage == [field \in UsageFields |-> ZeroWord]
OneUsage == [field \in UsageFields |-> OneWord]
NearMaxU16Usage == [field \in UsageFields |-> NearMaxU16Word]
MaxU16Usage == [field \in UsageFields |-> MaxU16Word]
OnePastU16Usage == [field \in UsageFields |-> OnePastU16Word]
NearMaxU32Usage == [field \in UsageFields |-> NearMaxU32Word]
MaxU32Usage == [field \in UsageFields |-> MaxU32Word]
OnePastU32Usage == [field \in UsageFields |-> OnePastU32Word]
MaxUsage == [field \in UsageFields |-> MaxU64Word]
NearMaxUsage == [field \in UsageFields |-> NearMaxU64Word]

ZeroBudget == [field \in BudgetFields |-> ZeroWord]
OneBudget == [field \in BudgetFields |-> OneWord]
MaxBudget == [field \in BudgetFields |-> BudgetFieldMax(field)]

UsageCases == {
    ZeroUsage,
    OneUsage,
    NearMaxU16Usage,
    MaxU16Usage,
    OnePastU16Usage,
    NearMaxU32Usage,
    MaxU32Usage,
    OnePastU32Usage,
    NearMaxUsage,
    MaxUsage
}
BudgetCases == {ZeroBudget, OneBudget, MaxBudget}

AddFits(usage, budget) ==
    \A field \in UsageFields : AddWord(usage[field], BudgetAmount(budget, field)).tag = "Ok"

SubFits(usage, budget) ==
    \A field \in UsageFields : SubWord(usage[field], BudgetAmount(budget, field)).tag = "Ok"

AddValue(usage, budget) ==
    [field \in UsageFields |-> AddWord(usage[field], BudgetAmount(budget, field)).value]

SubValue(usage, budget) ==
    [field \in UsageFields |-> SubWord(usage[field], BudgetAmount(budget, field)).value]

AddResult(usage, budget) ==
    IF AddFits(usage, budget)
    THEN [tag |-> "Ok", value |-> AddValue(usage, budget)]
    ELSE [tag |-> "Err", error |-> "Overflow"]

SubResult(usage, budget) ==
    IF SubFits(usage, budget)
    THEN [tag |-> "Ok", value |-> SubValue(usage, budget)]
    ELSE [tag |-> "Err", error |-> "Underflow"]

ResultTypeOK(result) ==
    IF result.tag = "Ok"
    THEN UsageTypeOK(result.value)
    ELSE /\ result.tag = "Err"
         /\ result.error \in {"Overflow", "Underflow"}

VARIABLES usage, previous_usage, last_result, last_op, status, previous_status, phase

vars == <<usage, previous_usage, last_result, last_op, status, previous_status, phase>>

Init ==
    /\ usage \in UsageCases
    /\ previous_usage = usage
    /\ last_result = [tag |-> "Ok", value |-> usage]
    /\ last_op = "Init"
    /\ status = "Running"
    /\ previous_status = status
    /\ phase = "Ready"

TryAdd(budget) ==
    /\ BudgetTypeOK(budget)
    /\ phase = "Ready"
    /\ status = "Running"
    /\ LET result == AddResult(usage, budget) IN
       /\ previous_usage' = usage
       /\ previous_status' = status
       /\ last_result' = result
       /\ last_op' = "Add"
       /\ phase' = "Done"
       /\ usage' = IF result.tag = "Ok" THEN result.value ELSE usage
       /\ IF result.tag = "Ok"
          THEN status' = "Running"
          ELSE status' \in GracefulErrorStatuses

TrySubtract(budget) ==
    /\ BudgetTypeOK(budget)
    /\ phase = "Ready"
    /\ status = "Running"
    /\ LET result == SubResult(usage, budget) IN
       /\ previous_usage' = usage
       /\ previous_status' = status
       /\ last_result' = result
       /\ last_op' = "Subtract"
       /\ phase' = "Done"
       /\ usage' = IF result.tag = "Ok" THEN result.value ELSE usage
       /\ IF result.tag = "Ok"
          THEN status' = "Running"
          ELSE status' \in GracefulErrorStatuses

TerminalStutter ==
    /\ phase = "Done"
    /\ UNCHANGED vars

Next ==
    \/ TerminalStutter
    \/ \E budget \in BudgetCases :
        \/ TryAdd(budget)
        \/ TrySubtract(budget)

Spec == Init /\ [][Next]_vars

TypeOK ==
    /\ UsageTypeOK(usage)
    /\ UsageTypeOK(previous_usage)
    /\ ResultTypeOK(last_result)
    /\ last_op \in {"Init", "Add", "Subtract"}
    /\ phase \in Phases
    /\ status \in Statuses
    /\ previous_status \in Statuses

ErrLeavesUsageUnchanged ==
    IF last_result.tag = "Err" THEN usage = previous_usage ELSE TRUE

OkResultApplied ==
    IF last_result.tag = "Ok" THEN usage = last_result.value ELSE TRUE

AddErrIsOverflow ==
    IF last_op = "Add" /\ last_result.tag = "Err"
    THEN last_result.error = "Overflow"
    ELSE TRUE

SubtractErrIsUnderflow ==
    IF last_op = "Subtract" /\ last_result.tag = "Err"
    THEN last_result.error = "Underflow"
    ELSE TRUE

ActiveRunsMatchesRust ==
    IF last_op = "Add" /\ last_result.tag = "Ok"
    THEN usage["max_active_runs"] = AddWord(previous_usage["max_active_runs"], OneWord).value
    ELSE IF last_op = "Subtract" /\ last_result.tag = "Ok"
    THEN usage["max_active_runs"] = SubWord(previous_usage["max_active_runs"], OneWord).value
    ELSE TRUE

ErrStatusIsGraceful ==
    IF last_result.tag = "Err" THEN status \in GracefulErrorStatuses ELSE TRUE

OkStatusIsRunning ==
    IF last_result.tag = "Ok" THEN status = "Running" ELSE TRUE

ErrStartedFromRunning ==
    IF last_result.tag = "Err" THEN previous_status = "Running" ELSE TRUE

THEOREM Spec => []TypeOK
THEOREM Spec => []ErrLeavesUsageUnchanged
THEOREM Spec => []OkResultApplied
THEOREM Spec => []AddErrIsOverflow
THEOREM Spec => []SubtractErrIsUnderflow
THEOREM Spec => []ActiveRunsMatchesRust
THEOREM Spec => []ErrStatusIsGraceful
THEOREM Spec => []OkStatusIsRunning
THEOREM Spec => []ErrStartedFromRunning

====
