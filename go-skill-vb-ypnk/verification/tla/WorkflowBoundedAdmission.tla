---- MODULE WorkflowBoundedAdmission ----
EXTENDS Naturals

\* Obligations: PO-001, PO-002, PO-003 / TLA-ADM-001,
\* TLA-ADM-002, TLA-RUN-001. Finite model of bounded workflow
\* certificate creation, capacity reservation, acknowledgment, capped
\* run-state creation, step-budget exhaustion, and fail-closed rejection.

BudgetValues == 0..3
StepBudgets == 0..3
ValueSlotCounts == 0..3

ArtifactStates == {"valid", "invalid"}
CertificateStates == {"none", "valid", "invalid"}
RunStates == {"none", "capped", "blocked", "terminal"}
Outcomes == {"pending", "rejected", "acked", "running", "blocked", "terminal"}
RunnableOutcomes == {"acked", "running", "blocked", "terminal"}
TerminalOutcomes == {"rejected", "blocked", "terminal"}

VARIABLES artifactState, certificate, requestedBudget, capacity, usage,
          reservation, runState, valueSlots, stepBudget, outcome

vars == <<artifactState, certificate, requestedBudget, capacity, usage,
          reservation, runState, valueSlots, stepBudget, outcome>>

Init ==
  /\ artifactState \in ArtifactStates
  /\ certificate = "none"
  /\ requestedBudget \in BudgetValues
  /\ capacity \in BudgetValues
  /\ usage \in BudgetValues
  /\ reservation = 0
  /\ runState = "none"
  /\ valueSlots = 0
  /\ stepBudget \in StepBudgets
  /\ outcome = "pending"

ComputeCertificate ==
  /\ outcome = "pending"
  /\ artifactState = "valid"
  /\ certificate = "none"
  /\ certificate' = "valid"
  /\ UNCHANGED <<artifactState, requestedBudget, capacity, usage,
                  reservation, runState, valueSlots, stepBudget, outcome>>

RejectInvalidCertificate ==
  /\ outcome = "pending"
  /\ artifactState = "invalid"
  /\ certificate = "none"
  /\ certificate' = "invalid"
  /\ outcome' = "rejected"
  /\ reservation' = 0
  /\ runState' = "none"
  /\ valueSlots' = 0
  /\ UNCHANGED <<artifactState, requestedBudget, capacity, usage, stepBudget>>

ReserveCapacity ==
  /\ outcome = "pending"
  /\ certificate = "valid"
  /\ reservation = 0
  /\ usage + requestedBudget <= capacity
  /\ reservation' = requestedBudget
  /\ usage' = usage + requestedBudget
  /\ UNCHANGED <<artifactState, certificate, requestedBudget, capacity,
                  runState, valueSlots, stepBudget, outcome>>

RejectOverCapacity ==
  /\ outcome = "pending"
  /\ certificate = "valid"
  /\ reservation = 0
  /\ usage + requestedBudget > capacity
  /\ outcome' = "rejected"
  /\ reservation' = 0
  /\ runState' = "none"
  /\ valueSlots' = 0
  /\ UNCHANGED <<artifactState, certificate, requestedBudget, capacity,
                  usage, stepBudget>>

AckRun ==
  /\ outcome = "pending"
  /\ certificate = "valid"
  /\ reservation = requestedBudget
  /\ usage <= capacity
  /\ outcome' = "acked"
  /\ UNCHANGED <<artifactState, certificate, requestedBudget, capacity, usage,
                  reservation, runState, valueSlots, stepBudget>>

CreateCappedRunState ==
  /\ outcome = "acked"
  /\ certificate = "valid"
  /\ runState = "none"
  /\ runState' = "capped"
  /\ valueSlots' \in 0..requestedBudget
  /\ outcome' = "running"
  /\ UNCHANGED <<artifactState, certificate, requestedBudget, capacity, usage,
                  reservation, stepBudget>>

ExecuteStep ==
  /\ outcome = "running"
  /\ runState = "capped"
  /\ stepBudget > 0
  /\ stepBudget' = stepBudget - 1
  /\ UNCHANGED <<artifactState, certificate, requestedBudget, capacity, usage,
                  reservation, runState, valueSlots, outcome>>

ExhaustStepBudget ==
  /\ outcome = "running"
  /\ runState = "capped"
  /\ stepBudget = 0
  /\ outcome' = "blocked"
  /\ runState' = "blocked"
  /\ UNCHANGED <<artifactState, certificate, requestedBudget, capacity, usage,
                  reservation, valueSlots, stepBudget>>

ReleaseReservation ==
  /\ outcome \in {"blocked", "terminal"}
  /\ reservation > 0
  /\ usage >= reservation
  /\ usage' = usage - reservation
  /\ reservation' = 0
  /\ outcome' = "terminal"
  /\ runState' = "terminal"
  /\ UNCHANGED <<artifactState, certificate, requestedBudget, capacity,
                  valueSlots, stepBudget>>

FailClosed ==
  /\ outcome = "rejected"
  /\ runState = "none"
  /\ reservation = 0
  /\ valueSlots = 0
  /\ UNCHANGED vars

TerminalStutter ==
  /\ outcome \in TerminalOutcomes
  /\ UNCHANGED vars

Next == ComputeCertificate \/ RejectInvalidCertificate \/ ReserveCapacity
        \/ RejectOverCapacity \/ AckRun \/ CreateCappedRunState
        \/ ExecuteStep \/ ExhaustStepBudget \/ ReleaseReservation \/ FailClosed
        \/ TerminalStutter

Spec ==
  /\ Init
  /\ [][Next]_vars
  /\ WF_vars(ComputeCertificate)
  /\ WF_vars(RejectInvalidCertificate)
  /\ WF_vars(ReserveCapacity)
  /\ WF_vars(RejectOverCapacity)
  /\ WF_vars(AckRun)
  /\ WF_vars(CreateCappedRunState)
  /\ WF_vars(ExecuteStep)
  /\ WF_vars(ExhaustStepBudget)
  /\ WF_vars(ReleaseReservation)

NoAckWithoutCertificate ==
  outcome \in RunnableOutcomes => certificate = "valid"

NoAckOverCapacity ==
  outcome \in RunnableOutcomes => usage <= capacity

NoUncappedRunState ==
  outcome \in {"running", "blocked", "terminal"} =>
    /\ runState # "none"
    /\ valueSlots <= requestedBudget

FailClosedNotRunnable ==
  outcome = "rejected" =>
    /\ runState = "none"
    /\ reservation = 0
    /\ valueSlots = 0

StepBudgetNeverNegative ==
  stepBudget \in StepBudgets

NonTerminalProgressEnabled ==
  outcome \notin TerminalOutcomes => ENABLED Next

EventuallyAckOrReject ==
  outcome = "pending" ~> outcome \in TerminalOutcomes \/ outcome \in RunnableOutcomes

EventuallyBlockedOrTerminal ==
  outcome = "running" ~> outcome \in {"blocked", "terminal"}

====
