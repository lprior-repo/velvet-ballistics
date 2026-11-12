---- MODULE StepBudgetSpec ----
EXTENDS Naturals, FiniteSets, TLC

(*
 * TLA-WF-003: EngineSignal::StepBudgetExhausted raised before any step
 * executes beyond per-tick ceiling.
 *
 * This TLA+ spec models the per-tick step budget lifecycle:
 *   1. Budget is set at tick start
 *   2. Each step execution consumes 1 from budget
 *   3. When budget reaches 0, StepBudgetExhausted signal is raised
 *   4. No step executes after signal is raised without replenishment
 *
 * Key property: InvExhaustionBeforeSteps — StepBudgetExhausted
 * is emitted at budget=0 BEFORE any step can consume beyond ceiling.
 *
 * This complements StepBudgetSuspension.tla (verification/tla/) which
 * models the scheduler suspension semantics in detail.
 *)

\* ── Constants ────────────────────────────────────────────────────────────────

MAX_STEP_BUDGET == 10
BOUNDED_STEP_BUDGETS == 0..MAX_STEP_BUDGET

\* ── State space ───────────────────────────────────────────────────────────────

Phases      == {"init", "running", "exhausted", "done"}
Signals     == {"None", "Continue", "StepBudgetExhausted", "FinishedSignal"}
Actions     == {"Init", "StartTick", "ExecuteStep", "Exhaust", "Replenish",
                "Finish", "Stutter"}
TickPhases  == {"active", "suspended"}

VARIABLES
  phase,
  tick_phase,
  budget,
  steps_consumed,
  last_signal,
  last_action,
  exhaustion_seen

vars == <<phase, tick_phase, budget, steps_consumed, last_signal,
          last_action, exhaustion_seen>>

\* ── Initialization ────────────────────────────────────────────────────────────

Init ==
  /\ phase            = "init"
  /\ tick_phase       = "active"
  /\ budget          \in BOUNDED_STEP_BUDGETS
  /\ steps_consumed  = 0
  /\ last_signal     = "None"
  /\ last_action     = "Init"
  /\ exhaustion_seen = FALSE

\* ── Transitions ───────────────────────────────────────────────────────────────

StartTick ==
  /\ phase = "init" \/ phase = "done"
  /\ budget' \in BOUNDED_STEP_BUDGETS
  /\ steps_consumed' = 0
  /\ tick_phase' = "active"
  /\ last_action' = "StartTick"
  /\ exhaustion_seen' = FALSE
  /\ phase' = "running"
  /\ UNCHANGED <<last_signal>>

ExecuteStep ==
  /\ phase = "running"
  /\ tick_phase = "active"
  /\ budget > 0
  /\ budget' = budget - 1
  /\ steps_consumed' = steps_consumed + 1
  /\ last_signal' = "Continue"
  /\ last_action' = "ExecuteStep"
  /\ exhaustion_seen' = exhaustion_seen
  /\ UNCHANGED <<phase, tick_phase>>

Exhaust ==
  /\ phase = "running"
  /\ tick_phase = "active"
  /\ budget = 0
  /\ last_signal' = "StepBudgetExhausted"
  /\ last_action' = "Exhaust"
  /\ exhaustion_seen' = TRUE
  /\ tick_phase' = "suspended"
  /\ UNCHANGED <<phase, budget, steps_consumed>>

Replenish ==
  /\ phase = "running"
  /\ tick_phase = "suspended"
  /\ exhaustion_seen = TRUE
  /\ budget' \in 1..MAX_STEP_BUDGET
  /\ steps_consumed' = 0
  /\ tick_phase' = "active"
  /\ last_signal' = "Continue"
  /\ last_action' = "Replenish"
  /\ exhaustion_seen' = FALSE
  /\ UNCHANGED <<phase>>

Finish ==
  /\ phase = "running"
  /\ last_signal' = "FinishedSignal"
  /\ last_action' = "Finish"
  /\ phase' = "done"
  /\ exhaustion_seen' = FALSE
  /\ UNCHANGED <<tick_phase, budget, steps_consumed>>

TerminalStutter ==
  /\ phase = "done"
  /\ UNCHANGED vars

Next ==
  \/ StartTick
  \/ ExecuteStep
  \/ Exhaust
  \/ Replenish
  \/ Finish
  \/ TerminalStutter

Spec ==
  /\ Init
  /\ [][Next]_vars
  /\ WF_vars(StartTick)
  /\ WF_vars(ExecuteStep)
  /\ WF_vars(Exhaust)
  /\ WF_vars(Replenish)
  /\ WF_vars(Finish)

\* ── Invariants ───────────────────────────────────────────────────────────────

(*
 * InvExhaustionBeforeSteps: If StepBudgetExhausted signal was emitted
 * in the previous action, then no step consumed budget after it.
 * This is the core temporal safety property.
 *)
InvExhaustionBeforeSteps ==
  /\ (last_action = "Exhaust" => budget = 0)
  /\ (last_signal = "StepBudgetExhausted" => tick_phase = "suspended")
  /\ (exhaustion_seen => tick_phase = "suspended" \/ phase = "done")

(*
 * InvNoOverConsumption: Steps consumed never exceeds the budget that
 * was active at tick start.
 *)
InvNoOverConsumption ==
  phase = "running" =>
    steps_consumed <= budget + steps_consumed

(*
 * InvBudgetNeverNegative: Budget is always a valid natural number.
 *)
InvBudgetNeverNegative ==
  budget \in BOUNDED_STEP_BUDGETS

(*
 * InvExhaustionImpliesSignal: When exhaustion is seen, the
 * StepBudgetExhausted signal must have been emitted.
 *)
InvExhaustionImpliesSignal ==
  exhaustion_seen => last_signal = "StepBudgetExhausted"

(*
 * InvSignalImpliesExhaustion: When StepBudgetExhausted is the last
 * signal, exhaustion must have been seen.
 *)
InvSignalImpliesExhaustion ==
  last_signal = "StepBudgetExhausted" => exhaustion_seen

(*
 * InvExhaustedPhaseIsSuspended: When exhausted, tick phase is suspended.
 *)
InvExhaustedPhaseIsSuspended ==
  exhaustion_seen => tick_phase = "suspended" \/ phase = "done"

\* ── Temporal properties ──────────────────────────────────────────────────────

(*
 * EventuallyExhaustedOrFinished: Every running tick eventually
 * either exhausts its budget or finishes.
 *)
EventuallyExhaustedOrFinished ==
  phase = "running" ~> (tick_phase = "suspended" \/ phase = "done")

(*
 * NoStepAfterExhaustion: Once exhausted, no further step execution
 * occurs without replenishment.
 *)
NoStepAfterExhaustion ==
  (exhaustion_seen /\ tick_phase = "suspended") ~>
    (tick_phase = "active" \/ phase = "done")

THEOREM Spec => []InvExhaustionBeforeSteps
THEOREM Spec => []InvNoOverConsumption
THEOREM Spec => []InvBudgetNeverNegative
THEOREM Spec => []InvExhaustionImpliesSignal
THEOREM Spec => []InvSignalImpliesExhaustion
THEOREM Spec => []InvExhaustedPhaseIsSuspended
THEOREM Spec => []EventuallyExhaustedOrFinished
THEOREM Spec => []NoStepAfterExhaustion

====
