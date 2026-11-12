---- MODULE NestedBoundednessAdmission ----

EXTENDS Naturals, TLC

CONSTANTS MaxDimension, MaxStoreCount

VARIABLES pc, computed_budget, policy, store_count, max_store_count, diagnostic

Vars == <<pc, computed_budget, policy, store_count, max_store_count, diagnostic>>

BudgetRecord == [steps: 0..MaxDimension, fanout: 0..MaxDimension, nesting: 0..MaxDimension]
Diagnostics == {"None", "Accepted", "TotalStepsExceeded", "FanoutExceeded", "NestingDepthExceeded", "ValueCapExceeded"}
TerminalPc == {"Accepted", "Rejected"}

WithinPolicy(b, p) ==
    /\ b.steps <= p.steps
    /\ b.fanout <= p.fanout
    /\ b.nesting <= p.nesting

OverLimitDiagnostic(b, p) ==
    IF b.steps > p.steps THEN "TotalStepsExceeded"
    ELSE IF b.fanout > p.fanout THEN "FanoutExceeded"
    ELSE IF b.nesting > p.nesting THEN "NestingDepthExceeded"
    ELSE "Accepted"

InitAdmission ==
    /\ pc = "Init"
    /\ computed_budget = [steps |-> 0, fanout |-> 0, nesting |-> 0]
    /\ policy = [steps |-> 2, fanout |-> 2, nesting |-> 2]
    /\ store_count = 0
    /\ max_store_count = MaxStoreCount
    /\ diagnostic = "None"

ComputeBudget ==
    /\ pc = "Init"
    /\ \E budget \in BudgetRecord :
        /\ computed_budget' = budget
        /\ pc' = "Computed"
        /\ UNCHANGED <<policy, store_count, max_store_count, diagnostic>>

AcceptWithinLimit ==
    /\ pc = "Computed"
    /\ WithinPolicy(computed_budget, policy)
    /\ pc' = "Accepted"
    /\ diagnostic' = "Accepted"
    /\ UNCHANGED <<computed_budget, policy, store_count, max_store_count>>

RejectOverLimit ==
    /\ pc = "Computed"
    /\ ~WithinPolicy(computed_budget, policy)
    /\ pc' = "Rejected"
    /\ diagnostic' = OverLimitDiagnostic(computed_budget, policy)
    /\ diagnostic' # "Accepted"
    /\ UNCHANGED <<computed_budget, policy, store_count, max_store_count>>

InsertValue ==
    /\ pc = "Accepted"
    /\ store_count < max_store_count
    /\ store_count' = store_count + 1
    /\ UNCHANGED <<pc, computed_budget, policy, max_store_count, diagnostic>>

RejectValueGrowth ==
    /\ pc = "Accepted"
    /\ store_count = max_store_count
    /\ pc' = "Rejected"
    /\ diagnostic' = "ValueCapExceeded"
    /\ UNCHANGED <<computed_budget, policy, store_count, max_store_count>>

TerminalStutter ==
    /\ pc = "Rejected"
    /\ UNCHANGED Vars

Next ==
    \/ ComputeBudget
    \/ AcceptWithinLimit
    \/ RejectOverLimit
    \/ InsertValue
    \/ RejectValueGrowth
    \/ TerminalStutter

Spec ==
    /\ InitAdmission
    /\ [][Next]_Vars
    /\ WF_Vars(ComputeBudget)
    /\ WF_Vars(AcceptWithinLimit)
    /\ WF_Vars(RejectOverLimit)
    /\ WF_Vars(RejectValueGrowth)

RejectsOverPolicy == pc = "Accepted" => WithinPolicy(computed_budget, policy)
StoreCountWithinCap == store_count <= max_store_count
TypedTerminalOutcome == pc \in TerminalPc => diagnostic \in Diagnostics /\ diagnostic # "None"
EventuallyAcceptOrRejectAdmission == <> (pc \in TerminalPc)

StateConstraint ==
    /\ store_count \in 0..MaxStoreCount
    /\ max_store_count = MaxStoreCount
    /\ computed_budget \in BudgetRecord

====
