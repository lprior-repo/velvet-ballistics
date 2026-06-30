---- MODULE StepBudgetSuspension ----
EXTENDS Naturals

\* Obligations: PO-001, PO-002, PO-003, PO-004, PO-005, PO-006.
\* Purpose: bounded temporal model for StepBudgetExhausted as a
\* non-terminal scheduler suspension.  The model keeps the production
\* MAX_U64/MAX_STEP_BUDGET constants explicit while TLC explores a small
\* representative slice.  Invalid arithmetic is routed to InvariantViolation;
\* zero budget never wraps, starts a step, or reaches a terminal state.

MAX_STEP_BUDGET == 10000
MAX_U16 == 65535
MAX_U64 == [w3 |-> MAX_U16, w2 |-> MAX_U16, w1 |-> MAX_U16, w0 |-> MAX_U16]
ABOVE_MAX_U64_REPRESENTATIVE == MAX_STEP_BUDGET + 1
U64_OVERFLOW_REPRESENTATIVE == MAX_STEP_BUDGET + 2
ZERO_UNDERFLOW_REPRESENTATIVE == MAX_STEP_BUDGET + 3
INVALID_ARITHMETIC_REPRESENTATIVE == ABOVE_MAX_U64_REPRESENTATIVE

SmallBudgets == 0..3
StepBudgetRepresentatives == SmallBudgets \cup {MAX_STEP_BUDGET - 3,
                                             MAX_STEP_BUDGET - 2,
                                             MAX_STEP_BUDGET - 1,
                                             MAX_STEP_BUDGET}
DecrementCheckRepresentatives == 1..3 \cup {MAX_STEP_BUDGET - 2,
                                            MAX_STEP_BUDGET - 1,
                                            MAX_STEP_BUDGET}
AboveU64Representatives == {ABOVE_MAX_U64_REPRESENTATIVE, U64_OVERFLOW_REPRESENTATIVE}
UnderflowRepresentatives == {ZERO_UNDERFLOW_REPRESENTATIVE}
ArithmeticSinkRepresentatives == AboveU64Representatives \cup UnderflowRepresentatives
IsRepresentativeBudget(b) == b \in StepBudgetRepresentatives
                             \/ b \in ArithmeticSinkRepresentatives
IsU64RepresentativeInput(b) == b \in StepBudgetRepresentatives
                              \/ b \in AboveU64Representatives
PcValues == 0..1
FrameValues == 0..1

RunnableState == "Runnable"
RunningState == "RunningStep"
BudgetSuspendedState == "SuspendedBudget"
ActionSuspendedState == "SuspendedAction"
WaitSuspendedState == "SuspendedWait"
AskSuspendedState == "SuspendedAsk"
FinishedState == "Finished"
TypedErrorState == "TypedError"
InvariantViolationState == "InvariantViolation"

TerminalStates == {FinishedState, TypedErrorState, InvariantViolationState}
ExternalSuspensions == {ActionSuspendedState, WaitSuspendedState, AskSuspendedState}
RunStates == {RunnableState, RunningState, BudgetSuspendedState,
              ActionSuspendedState, WaitSuspendedState, AskSuspendedState,
              FinishedState, TypedErrorState, InvariantViolationState}

Signals == {"None", "Continue", "StepBudgetExhausted", "AwaitingAction",
           "AwaitingWait", "AwaitingAsk", "FinishedSignal", "TypedErrorSignal",
           "InvariantViolationSignal"}

EvidenceEvents == {"StepStarted", "StepSucceeded", "SlotWritten", "DriveContinue",
                  "Suspended", "FinishedEvent", "TypedErrorEvent",
                  "ArithmeticErrorEvent"}

Actions == {"Init", "ClampMaxU64", "TakeStep", "CompleteContinue", "CompleteFinished",
           "CompleteTypedError", "BlockOnAction", "BlockOnWait", "BlockOnAsk",
           "ExhaustBudget", "ReplenishBudget", "ResumeExternal",
           "InjectArithmeticFault", "ArithmeticError", "TerminalStutter",
           "InvariantViolationStutter", "ModelBoundReached", "ZeroUnderflowSink"}

VARIABLES pc, frame, budget, run_state, last_signal, evidence,
          consumed_steps, completed_steps, reschedule_pending, arith_error,
          step_success_emitted,
          last_action, prior_pc, prior_frame, prior_budget, suspend_pc,
          suspend_frame, suspend_completed

vars == <<pc, frame, budget, run_state, last_signal, evidence,
          consumed_steps, completed_steps, reschedule_pending, arith_error,
          step_success_emitted,
          last_action, prior_pc, prior_frame, prior_budget, suspend_pc,
          suspend_frame, suspend_completed>>

U64Representable(b) == b \in StepBudgetRepresentatives
AboveMaxU64(b) == b \in AboveU64Representatives
OutOfRangeBudget(b) == b \in ArithmeticSinkRepresentatives \/ AboveMaxU64(b)
ClampMaxU64 == MAX_STEP_BUDGET
ClampStepBudget(b) == IF b \in StepBudgetRepresentatives THEN b
                       ELSE INVALID_ARITHMETIC_REPRESENTATIVE
ValidBudget(b) == b \in StepBudgetRepresentatives
InvalidBudget(b) == OutOfRangeBudget(b) \/ b \notin StepBudgetRepresentatives
CanDecrementOne(b) == ValidBudget(b) /\ b # 0
BudgetAfterTake(b) == IF CanDecrementOne(b) THEN b - 1 ELSE ZERO_UNDERFLOW_REPRESENTATIVE
ClampedBudgetWithinBounds(b) == ClampStepBudget(b) \in StepBudgetRepresentatives

Init ==
  \/ /\ budget \in SmallBudgets
     /\ pc \in PcValues
     /\ frame \in FrameValues
     /\ run_state = RunnableState
     /\ last_signal = "None"
     /\ evidence = {}
     /\ consumed_steps = 0
     /\ completed_steps = 0
     /\ reschedule_pending = FALSE
     /\ arith_error = FALSE
     /\ step_success_emitted = FALSE
     /\ last_action = "Init"
     /\ prior_pc = pc
     /\ prior_frame = frame
     /\ prior_budget = budget
     /\ suspend_pc = pc
     /\ suspend_frame = frame
     /\ suspend_completed = completed_steps
  \/ /\ budget = MAX_STEP_BUDGET
      /\ pc \in PcValues
      /\ frame \in FrameValues
     /\ run_state = RunnableState
     /\ last_signal = "None"
     /\ evidence = {}
     /\ consumed_steps = 0
     /\ completed_steps = 0
     /\ reschedule_pending = FALSE
     /\ arith_error = FALSE
     /\ step_success_emitted = FALSE
     /\ last_action = "Init"
     /\ prior_pc = pc
     /\ prior_frame = frame
     /\ prior_budget = budget
      /\ suspend_pc = pc
      /\ suspend_frame = frame
      /\ suspend_completed = completed_steps
  \/ /\ budget = ClampMaxU64
      /\ pc \in PcValues
      /\ frame \in FrameValues
      /\ run_state = RunnableState
      /\ last_signal = "None"
      /\ evidence = {}
      /\ consumed_steps = 0
      /\ completed_steps = 0
      /\ reschedule_pending = FALSE
      /\ arith_error = FALSE
      /\ step_success_emitted = FALSE
      /\ last_action = "ClampMaxU64"
      /\ prior_pc = pc
      /\ prior_frame = frame
      /\ prior_budget = MAX_STEP_BUDGET
      /\ suspend_pc = pc
      /\ suspend_frame = frame
      /\ suspend_completed = completed_steps
  \/ /\ budget \in ArithmeticSinkRepresentatives
      /\ pc \in PcValues
      /\ frame \in FrameValues
      /\ run_state = InvariantViolationState
      /\ last_signal = "InvariantViolationSignal"
      /\ evidence = {"ArithmeticErrorEvent"}
     /\ consumed_steps = 0
     /\ completed_steps = 0
     /\ reschedule_pending = FALSE
     /\ arith_error = TRUE
     /\ step_success_emitted = FALSE
      /\ last_action = IF budget = ZERO_UNDERFLOW_REPRESENTATIVE THEN "ZeroUnderflowSink" ELSE "ArithmeticError"
     /\ prior_pc = pc
     /\ prior_frame = frame
     /\ prior_budget = budget
     /\ suspend_pc = pc
     /\ suspend_frame = frame
     /\ suspend_completed = completed_steps

RememberPre ==
  /\ prior_pc' = pc
  /\ prior_frame' = frame
  /\ prior_budget' = budget

TakeStep ==
  /\ run_state = RunnableState
  /\ CanDecrementOne(budget)
  /\ consumed_steps < 3
  /\ RememberPre
  /\ budget' = BudgetAfterTake(budget)
  /\ run_state' = RunningState
  /\ last_signal' = "Continue"
  /\ evidence' = evidence \cup {"StepStarted"}
  /\ consumed_steps' = consumed_steps + 1
  /\ completed_steps' = completed_steps
  /\ reschedule_pending' = FALSE
  /\ arith_error' = FALSE
  /\ step_success_emitted' = FALSE
  /\ last_action' = "TakeStep"
  /\ UNCHANGED <<pc, frame, suspend_pc, suspend_frame, suspend_completed>>

CompleteContinue ==
  /\ run_state = RunningState
  /\ completed_steps < 1
  /\ RememberPre
  /\ pc' = (pc + 1) % 2
  /\ frame' \in FrameValues
  /\ run_state' = RunnableState
  /\ last_signal' = "Continue"
  /\ evidence' = evidence \cup {"StepSucceeded", "SlotWritten", "DriveContinue"}
  /\ completed_steps' = completed_steps + 1
  /\ reschedule_pending' = FALSE
  /\ arith_error' = FALSE
  /\ step_success_emitted' = TRUE
  /\ last_action' = "CompleteContinue"
  /\ UNCHANGED <<budget, consumed_steps, suspend_pc, suspend_frame, suspend_completed>>

CompleteFinished ==
  /\ run_state = RunningState
  /\ RememberPre
  /\ run_state' = FinishedState
  /\ last_signal' = "FinishedSignal"
  /\ evidence' = evidence \cup {"StepSucceeded", "SlotWritten", "FinishedEvent"}
  /\ completed_steps' = completed_steps + 1
  /\ reschedule_pending' = FALSE
  /\ arith_error' = FALSE
  /\ step_success_emitted' = TRUE
  /\ last_action' = "CompleteFinished"
  /\ UNCHANGED <<pc, frame, budget, consumed_steps, suspend_pc, suspend_frame, suspend_completed>>

CompleteTypedError ==
  /\ run_state = RunningState
  /\ RememberPre
  /\ run_state' = TypedErrorState
  /\ last_signal' = "TypedErrorSignal"
  /\ evidence' = evidence \cup {"TypedErrorEvent"}
  /\ completed_steps' = completed_steps
  /\ reschedule_pending' = FALSE
  /\ arith_error' = FALSE
  /\ step_success_emitted' = FALSE
  /\ last_action' = "CompleteTypedError"
  /\ UNCHANGED <<pc, frame, budget, consumed_steps, suspend_pc, suspend_frame, suspend_completed>>

BlockOnAction ==
  /\ run_state = RunningState
  /\ RememberPre
  /\ run_state' = ActionSuspendedState
  /\ last_signal' = "AwaitingAction"
  /\ evidence' = evidence \cup {"Suspended"}
  /\ reschedule_pending' = FALSE
  /\ arith_error' = FALSE
  /\ step_success_emitted' = FALSE
  /\ last_action' = "BlockOnAction"
  /\ UNCHANGED <<pc, frame, budget, consumed_steps, completed_steps,
                  suspend_pc, suspend_frame, suspend_completed>>

BlockOnWait ==
  /\ run_state = RunningState
  /\ RememberPre
  /\ run_state' = WaitSuspendedState
  /\ last_signal' = "AwaitingWait"
  /\ evidence' = evidence \cup {"Suspended"}
  /\ reschedule_pending' = FALSE
  /\ arith_error' = FALSE
  /\ step_success_emitted' = FALSE
  /\ last_action' = "BlockOnWait"
  /\ UNCHANGED <<pc, frame, budget, consumed_steps, completed_steps,
                  suspend_pc, suspend_frame, suspend_completed>>

BlockOnAsk ==
  /\ run_state = RunningState
  /\ RememberPre
  /\ run_state' = AskSuspendedState
  /\ last_signal' = "AwaitingAsk"
  /\ evidence' = evidence \cup {"Suspended"}
  /\ reschedule_pending' = FALSE
  /\ arith_error' = FALSE
  /\ step_success_emitted' = FALSE
  /\ last_action' = "BlockOnAsk"
  /\ UNCHANGED <<pc, frame, budget, consumed_steps, completed_steps,
                  suspend_pc, suspend_frame, suspend_completed>>

ExhaustBudget ==
  /\ run_state = RunnableState
  /\ budget = 0
  /\ RememberPre
  /\ run_state' = BudgetSuspendedState
  /\ last_signal' = "StepBudgetExhausted"
  /\ evidence' = evidence \cup {"DriveContinue", "Suspended"}
  /\ reschedule_pending' = TRUE
  /\ arith_error' = FALSE
  /\ step_success_emitted' = FALSE
  /\ last_action' = "ExhaustBudget"
  /\ suspend_pc' = pc
  /\ suspend_frame' = frame
  /\ suspend_completed' = completed_steps
  /\ UNCHANGED <<pc, frame, budget, consumed_steps, completed_steps>>

ReplenishBudget ==
  /\ run_state = BudgetSuspendedState
  /\ reschedule_pending = TRUE
  /\ RememberPre
  /\ budget' \in 1..3 \cup {MAX_STEP_BUDGET}
  /\ run_state' = RunnableState
  /\ last_signal' = "Continue"
  /\ evidence' = evidence \cup {"DriveContinue"}
  /\ reschedule_pending' = FALSE
  /\ arith_error' = FALSE
  /\ step_success_emitted' = FALSE
  /\ last_action' = "ReplenishBudget"
  /\ UNCHANGED <<pc, frame, consumed_steps, completed_steps, suspend_pc,
                  suspend_frame, suspend_completed>>

ResumeExternal ==
  /\ run_state \in ExternalSuspensions
  /\ RememberPre
  /\ run_state' \in {RunnableState, TypedErrorState}
  /\ last_signal' \in {"Continue", "TypedErrorSignal"}
  /\ evidence' = evidence
  /\ reschedule_pending' = FALSE
  /\ arith_error' = FALSE
  /\ step_success_emitted' = FALSE
  /\ last_action' = "ResumeExternal"
  /\ UNCHANGED <<pc, frame, budget, consumed_steps, completed_steps,
                  suspend_pc, suspend_frame, suspend_completed>>

InjectArithmeticFault ==
  /\ run_state \notin TerminalStates
  /\ InvalidBudget(budget)
  /\ RememberPre
  /\ budget' = INVALID_ARITHMETIC_REPRESENTATIVE
  /\ run_state' = InvariantViolationState
  /\ last_signal' = "InvariantViolationSignal"
  /\ evidence' = evidence \cup {"ArithmeticErrorEvent"}
  /\ arith_error' = TRUE
  /\ step_success_emitted' = FALSE
  /\ reschedule_pending' = FALSE
  /\ last_action' = "InjectArithmeticFault"
  /\ UNCHANGED <<pc, frame, consumed_steps, completed_steps,
                  suspend_pc, suspend_frame, suspend_completed>>

ArithmeticError ==
  /\ InvalidBudget(budget)
  /\ run_state # InvariantViolationState
  /\ RememberPre
  /\ run_state' = InvariantViolationState
  /\ last_signal' = "InvariantViolationSignal"
  /\ evidence' = evidence \cup {"ArithmeticErrorEvent"}
  /\ arith_error' = TRUE
  /\ step_success_emitted' = FALSE
  /\ reschedule_pending' = FALSE
  /\ last_action' = "ArithmeticError"
  /\ UNCHANGED <<pc, frame, budget, consumed_steps, completed_steps,
                  suspend_pc, suspend_frame, suspend_completed>>

ModelBoundReached ==
  /\ run_state = RunnableState
  /\ consumed_steps = 3
  /\ RememberPre
  /\ run_state' = FinishedState
  /\ last_signal' = "FinishedSignal"
  /\ evidence' = evidence \cup {"FinishedEvent"}
  /\ reschedule_pending' = FALSE
  /\ arith_error' = FALSE
  /\ step_success_emitted' = FALSE
  /\ last_action' = "ModelBoundReached"
  /\ UNCHANGED <<pc, frame, budget, consumed_steps, completed_steps,
                  suspend_pc, suspend_frame, suspend_completed>>

TerminalStutter ==
  /\ run_state \in {FinishedState, TypedErrorState}
  /\ RememberPre
  /\ last_action' = "TerminalStutter"
  /\ step_success_emitted' = FALSE
  /\ UNCHANGED <<pc, frame, budget, run_state, last_signal, evidence,
                  consumed_steps, completed_steps, reschedule_pending, arith_error,
                  suspend_pc, suspend_frame, suspend_completed>>

InvariantViolationStutter ==
  /\ run_state = InvariantViolationState
  /\ RememberPre
  /\ last_action' = "InvariantViolationStutter"
  /\ step_success_emitted' = FALSE
  /\ UNCHANGED <<pc, frame, budget, run_state, last_signal, evidence,
                  consumed_steps, completed_steps, reschedule_pending, arith_error,
                  suspend_pc, suspend_frame, suspend_completed>>

NextCore == TakeStep \/ CompleteContinue \/ CompleteFinished \/ CompleteTypedError
            \/ BlockOnAction \/ BlockOnWait \/ BlockOnAsk \/ ExhaustBudget
            \/ ReplenishBudget \/ ResumeExternal \/ InjectArithmeticFault
            \/ ArithmeticError \/ ModelBoundReached

Next == NextCore \/ TerminalStutter \/ InvariantViolationStutter

Spec ==
  /\ Init
  /\ [][Next]_vars
  /\ WF_vars(ReplenishBudget)
  /\ WF_vars(TakeStep)
  /\ WF_vars(CompleteContinue)
  /\ WF_vars(CompleteFinished)
  /\ WF_vars(CompleteTypedError)
  /\ WF_vars(BlockOnAction)
  /\ WF_vars(BlockOnWait)
  /\ WF_vars(BlockOnAsk)
  /\ WF_vars(ResumeExternal)

TypeOK ==
  /\ pc \in PcValues
  /\ frame \in FrameValues
  /\ IsRepresentativeBudget(budget)
  /\ run_state \in RunStates
  /\ last_signal \in Signals
  /\ evidence \subseteq EvidenceEvents
  /\ consumed_steps \in 0..3
  /\ completed_steps \in 0..3
  /\ reschedule_pending \in BOOLEAN
  /\ arith_error \in BOOLEAN
  /\ step_success_emitted \in BOOLEAN
  /\ last_action \in Actions
  /\ prior_pc \in PcValues
  /\ prior_frame \in FrameValues
  /\ IsRepresentativeBudget(prior_budget)
  /\ suspend_pc \in PcValues
  /\ suspend_frame \in FrameValues
  /\ suspend_completed \in 0..3

ExecutableU64ClampSemantics ==
  /\ MAX_U16 = 65535
  /\ MAX_U64 = [w3 |-> 65535, w2 |-> 65535, w1 |-> 65535, w0 |-> 65535]
  /\ MAX_STEP_BUDGET < MAX_U16
  /\ ABOVE_MAX_U64_REPRESENTATIVE = MAX_STEP_BUDGET + 1
  /\ U64_OVERFLOW_REPRESENTATIVE = MAX_STEP_BUDGET + 2
  /\ ZERO_UNDERFLOW_REPRESENTATIVE = MAX_STEP_BUDGET + 3
  /\ \A b \in StepBudgetRepresentatives : ClampedBudgetWithinBounds(b)
  /\ ClampMaxU64 = MAX_STEP_BUDGET
  /\ ClampStepBudget(MAX_STEP_BUDGET) = MAX_STEP_BUDGET
  /\ ClampStepBudget(ABOVE_MAX_U64_REPRESENTATIVE) = INVALID_ARITHMETIC_REPRESENTATIVE
  /\ IsU64RepresentativeInput(ABOVE_MAX_U64_REPRESENTATIVE)
  /\ ~U64Representable(ABOVE_MAX_U64_REPRESENTATIVE)
  /\ AboveMaxU64(ABOVE_MAX_U64_REPRESENTATIVE)

ExecutableDecrementAndSinkSemantics ==
  /\ BudgetAfterTake(0) = ZERO_UNDERFLOW_REPRESENTATIVE
  /\ \A b \in DecrementCheckRepresentatives :
       (b # 0 => /\ CanDecrementOne(b)
                 /\ BudgetAfterTake(b) \in StepBudgetRepresentatives
                 /\ BudgetAfterTake(b) + 1 = b)
  /\ \A b \in ArithmeticSinkRepresentatives : OutOfRangeBudget(b)

U64ArithmeticModelChecks ==
  /\ ExecutableU64ClampSemantics
  /\ ExecutableDecrementAndSinkSemantics

BudgetWithinBounds ==
  /\ U64ArithmeticModelChecks
  /\ (run_state # InvariantViolationState => ValidBudget(budget))

NoBudgetUnderflowOrWrap ==
  /\ ~(run_state # InvariantViolationState /\ InvalidBudget(budget))
  /\ ~(last_action = "TakeStep" /\ prior_budget = 0)
  /\ ~(last_action = "TakeStep" /\ budget = ZERO_UNDERFLOW_REPRESENTATIVE)
  /\ (last_action = "ZeroUnderflowSink" => run_state = InvariantViolationState)
  /\ (budget \in AboveU64Representatives => run_state = InvariantViolationState)
  /\ (last_action = "TakeStep" => budget + 1 = prior_budget)
  /\ (last_action = "TakeStep" /\ prior_budget = MAX_STEP_BUDGET => budget = MAX_STEP_BUDGET - 1)

ExhaustionNonTerminal ==
  last_signal = "StepBudgetExhausted" =>
    /\ run_state = BudgetSuspendedState
    /\ run_state \notin {FinishedState, TypedErrorState, InvariantViolationState}
    /\ reschedule_pending = TRUE

ExhaustionPreservesRunState ==
  last_action = "ExhaustBudget" =>
    /\ pc = prior_pc
    /\ frame = prior_frame
    /\ completed_steps = suspend_completed
    /\ pc = suspend_pc
    /\ frame = suspend_frame

EvidenceRequiresConsumedBudget ==
  (("StepStarted" \in evidence) \/ ("StepSucceeded" \in evidence) \/ ("SlotWritten" \in evidence))
    => consumed_steps + completed_steps > 0

NoSucceededOnExternalSuspend ==
  last_action \in {"BlockOnAction", "BlockOnWait", "BlockOnAsk"} =>
    /\ step_success_emitted = FALSE

LegacyTerminalExhaustionForbidden ==
  last_action = "ExhaustBudget" => run_state \notin {FinishedState, TypedErrorState}

DisjointSuspensions ==
  /\ run_state = BudgetSuspendedState => last_signal = "StepBudgetExhausted"
  /\ run_state = ActionSuspendedState => last_signal = "AwaitingAction"
  /\ run_state = WaitSuspendedState => last_signal = "AwaitingWait"
  /\ run_state = AskSuspendedState => last_signal = "AwaitingAsk"

NoDeadlockExceptTerminal ==
  run_state \notin TerminalStates => ENABLED NextCore

BudgetSuspensionEventuallyReschedulable ==
  run_state = BudgetSuspendedState ~> reschedule_pending = TRUE

FreshBudgetEventuallyProgresses ==
  run_state = BudgetSuspendedState ~>
    run_state \in {RunnableState, RunningState, ActionSuspendedState, WaitSuspendedState,
                  AskSuspendedState, FinishedState, TypedErrorState, InvariantViolationState}

MaxBudgetRunnableEventuallyDecrements ==
  (run_state = RunnableState /\ budget = MAX_STEP_BUDGET /\ consumed_steps < 3) ~>
    (last_action = "TakeStep" /\ prior_budget = MAX_STEP_BUDGET /\ budget = MAX_STEP_BUDGET - 1)

OutOfRangeEventuallyErrors ==
  InvalidBudget(budget) ~> run_state = InvariantViolationState

====
