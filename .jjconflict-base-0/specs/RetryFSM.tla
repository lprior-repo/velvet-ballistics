(* RetryFSM.tla
 *
 * Finite-state machine model for retry transitions.
 * Safety: once actionAttempts >= maxAttempts for a (run,step), no further retry
 * transitions are allowed; the next ActionFailed results in Failed state.
 * Liveness: every retryable failed action eventually reaches terminal or exhaustion.
 *)

---- MODULE RetryFSM ----

EXTENDS Integers, Sequences, TLC

MAX_U16 == 65535

CONSTANT RunId, StepId, MaxAttemptsValue

ASSUME MaxAttemptsValue \in 1..MAX_U16

VARIABLES
    runs,
    actionAttempts,
    framePC,
    stepState,
    maxAttempts,
    retryPolicy,
    stepHasRetryCheck,
    last_error

Runs == RunId
Steps == StepId
MaxAttempts == MaxAttemptsValue
StepStates == {"Pending", "Running", "Failed"}
RetryPolicies == {"Retryable", "NonRetryable"}
FailureTypes == {"Retryable", "NonRetryable"}
ErrorKinds == {"None", "NonRetryableFailure", "RetryExhausted"}

(* Helper: determine next state for a failure action
 * Returns [state |-> "Failed", attempts |-> attempts, pc |-> pc] for non-retryable or exhausted
 * Returns [state |-> "Running", attempts |-> attempts+1, pc |-> step] for retry allowed
 *)
FailureOutcome(run, step, failureType) ==
    CASE failureType = "NonRetryable" \/ ~stepHasRetryCheck[run][step] ->
        [state |-> "Failed", attempts |-> actionAttempts[run][step], pc |-> framePC[run], error |-> "NonRetryableFailure"]
      [] actionAttempts[run][step] + 1 < maxAttempts[run][step] ->
        [state |-> "Running", attempts |-> actionAttempts[run][step] + 1, pc |-> step, error |-> "None"]
      [] OTHER ->
        [state |-> "Failed", attempts |-> actionAttempts[run][step] + 1, pc |-> framePC[run], error |-> "RetryExhausted"]

(* Init action *)
Init ==
    /\ runs = {}
    /\ actionAttempts = [run \in Runs |-> [step \in Steps |-> 0]]
    /\ framePC = [run \in Runs |-> 1]
    /\ stepState = [run \in Runs |-> [step \in Steps |-> "Pending"]]
    /\ maxAttempts = [run \in Runs |-> [step \in Steps |-> MaxAttempts]]
    /\ retryPolicy = [run \in Runs |-> [step \in Steps |-> "Retryable"]]
    /\ stepHasRetryCheck = [run \in Runs |-> [step \in Steps |-> TRUE]]
    /\ last_error = [run \in Runs |-> [step \in Steps |-> "None"]]

(* Add a run to the model *)
AddRun(run) ==
    /\ run \notin runs
    /\ runs' = runs \cup {run}
    /\ actionAttempts' = [actionAttempts EXCEPT ![run] = [step \in Steps |-> 0]]
    /\ stepState' = [stepState EXCEPT ![run] = [step \in Steps |-> "Pending"]]
    /\ framePC' = [framePC EXCEPT ![run] = 1]
    /\ last_error' = [last_error EXCEPT ![run] = [step \in Steps |-> "None"]]
    /\ UNCHANGED <<maxAttempts, retryPolicy, stepHasRetryCheck>>

(* Mark step as running *)
StartStep(run, step) ==
    /\ run \in runs
    /\ stepState[run][step] = "Pending"
    /\ \A other \in Steps : stepState[run][other] # "Running"
    /\ stepState' = [stepState EXCEPT ![run][step] = "Running"]
    /\ framePC' = [framePC EXCEPT ![run] = step]
    /\ last_error' = [last_error EXCEPT ![run][step] = "None"]
    /\ UNCHANGED <<runs, actionAttempts, maxAttempts, retryPolicy, stepHasRetryCheck>>

(* ActionFailed handler - core retry logic
 * Guard: actionAttempts < maxAttempts (except for NonRetryable which has no guard)
 * Uses FailureOutcome helper to determine next state based on failure type and attempt count
 * Note: attempt parameter removed as it was not used in the action body
 *)
ActionFailed(run, step, failureType) ==
    /\ run \in runs
    /\ stepState[run][step] = "Running"
    /\ IF failureType = "NonRetryable" \/ ~stepHasRetryCheck[run][step] THEN TRUE
       ELSE actionAttempts[run][step] < maxAttempts[run][step]
    /\ LET outcome == FailureOutcome(run, step, failureType) IN
        /\ stepState' = [stepState EXCEPT ![run][step] = outcome.state]
        /\ actionAttempts' = [actionAttempts EXCEPT ![run][step] = outcome.attempts]
        /\ framePC' = [framePC EXCEPT ![run] = outcome.pc]
        /\ last_error' = [last_error EXCEPT ![run][step] = outcome.error]
        /\ UNCHANGED <<maxAttempts, retryPolicy, runs, stepHasRetryCheck>>

(* Stale completion rejection
 * stale and current values are derived from actionAttempts state
 *)
StaleCompletionRejected(run, step) ==
    /\ stepState[run][step] = "Running"
    /\ actionAttempts[run][step] > 0
    /\ actionAttempts' = actionAttempts
    /\ framePC' = framePC
    /\ stepState' = stepState
    /\ UNCHANGED <<maxAttempts, retryPolicy, runs, stepHasRetryCheck, last_error>>

TerminalStutter ==
    /\ runs = Runs
    /\ \A run \in Runs, step \in Steps : stepState[run][step] # "Running"
    /\ UNCHANGED <<runs, actionAttempts, framePC, stepState, maxAttempts, retryPolicy, stepHasRetryCheck, last_error>>

(* Next relation
 * Removed existential quantification over attempt and stale/current values to prevent state explosion.
 * All values are now derived from state.
 *)
Next ==
    \E run \in Runs, step \in Steps, ftype \in {"Retryable", "NonRetryable"} :
        \/ AddRun(run)
        \/ StartStep(run, step)
        \/ ActionFailed(run, step, ftype)
        \/ StaleCompletionRejected(run, step)
        \/ TerminalStutter

vars == <<runs, actionAttempts, framePC, stepState, maxAttempts, retryPolicy, stepHasRetryCheck, last_error>>

ActionFailedRetryable(run, step) == ActionFailed(run, step, "Retryable")

(* Liveness is only meaningful under fairness. Without this, TLC correctly finds
 * a stuttering counterexample where a Running step never receives another
 * failure event. Weak fairness per (run, step) says: if a retryable failure
 * remains continuously enabled for a running step, it eventually occurs. *)
Fairness ==
    \A run \in Runs, step \in Steps : WF_vars(ActionFailedRetryable(run, step))

(* Spec *)
Spec == Init /\ [][Next]_vars /\ Fairness

TypeOK ==
    /\ runs \in SUBSET Runs
    /\ actionAttempts \in [Runs -> [Steps -> 0..MAX_U16]]
    /\ framePC \in [Runs -> Steps]
    /\ stepState \in [Runs -> [Steps -> StepStates]]
    /\ maxAttempts \in [Runs -> [Steps -> 1..MAX_U16]]
    /\ retryPolicy \in [Runs -> [Steps -> RetryPolicies]]
    /\ stepHasRetryCheck \in [Runs -> [Steps -> BOOLEAN]]
    /\ last_error \in [Runs -> [Steps -> ErrorKinds]]
    /\ \A run \in Runs, step \in Steps : actionAttempts[run][step] <= maxAttempts[run][step]

(* Safety: No double retry after exhaustion
 * Once actionAttempts >= maxAttempts for a (run,step), stepState must be Failed.
 * This ensures no further retry transitions are allowed after exhaustion.
 *)
NoDoubleRetryAfterExhaustion ==
    \A run \in Runs, step \in Steps :
        actionAttempts[run][step] >= maxAttempts[run][step]
            => stepState[run][step] = "Failed"

RetryExhaustionIsTyped ==
    \A run \in Runs, step \in Steps :
        actionAttempts[run][step] >= maxAttempts[run][step]
            => last_error[run][step] = "RetryExhausted"

NoSilentSaturation ==
    \A run \in Runs, step \in Steps :
        stepState[run][step] = "Running"
            => actionAttempts[run][step] < maxAttempts[run][step]

(* FIXED: Removed vacuous NoStaleCompletion (proved actionAttempts >= 0 always true by type bounds).
 * The StaleCompletionRejected action is a no-op that records a stale event; the meaningful
 * safety is NoDoubleRetryAfterExhaustion which prevents retry after max attempts. *)

(* Safety: Frame PC reset on retry *)
FramePCResetOnRetry ==
    \A run \in Runs :
        run \in runs /\ (\E step \in Steps : stepState[run][step] = "Running") /\ framePC[run] \in Steps
            => stepState[run][framePC[run]] = "Running"

(* Liveness: every running step eventually reaches Failed under retryable-failure fairness. *)
EventuallyTerminalOrExhausted ==
    \A run \in Runs, step \in Steps :
        (run \in runs /\ stepState[run][step] = "Running") ~> stepState[run][step] = "Failed"

THEOREM Spec => []NoDoubleRetryAfterExhaustion
THEOREM Spec => []RetryExhaustionIsTyped
THEOREM Spec => []NoSilentSaturation
THEOREM Spec => []FramePCResetOnRetry

(* Liveness: eventually a failed state is reached *)
THEOREM Spec => EventuallyTerminalOrExhausted

====
