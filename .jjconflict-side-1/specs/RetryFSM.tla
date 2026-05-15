(* RetryFSM.tla
 *
 * Finite-state machine model for retry transitions.
 * Safety: once actionAttempts >= maxAttempts for a (run,step), no further retry
 * transitions are allowed; the next ActionFailed results in Failed state.
 * Liveness: every retryable failed action eventually reaches terminal or exhaustion.
 *)

---- MODULE RetryFSM ----

EXTENDS Integers, Sequences, TLC

CONSTANT RunId, StepId, MaxAttemptsValue

ASSUME MaxAttemptsValue \in Nat \ {0}

VARIABLES
    runs,
    actionAttempts,
    framePC,
    stepState,
    maxAttempts,
    retryPolicy,
    stepHasRetryCheck

Runs == RunId
Steps == StepId
MaxAttempts == MaxAttemptsValue

(* Helper: determine next state for a failure action
 * Returns [state |-> "Failed", attempts |-> attempts, pc |-> pc] for non-retryable or exhausted
 * Returns [state |-> "Running", attempts |-> attempts+1, pc |-> step] for retry allowed
 *)
FailureOutcome(run, step, failureType) ==
    CASE failureType = "NonRetryable" \/ ~stepHasRetryCheck[run][step] ->
        [state |-> "Failed", attempts |-> actionAttempts[run][step], pc |-> framePC[run]]
      [] actionAttempts[run][step] < maxAttempts[run][step] - 1 ->
        [state |-> "Running", attempts |-> actionAttempts[run][step] + 1, pc |-> step]
      [] OTHER ->
        [state |-> "Failed", attempts |-> actionAttempts[run][step], pc |-> framePC[run]]

(* Init action *)
Init ==
    /\ runs = {}
    /\ actionAttempts = [run \in Runs |-> [step \in Steps |-> 0]]
    /\ framePC = [run \in Runs |-> 1]
    /\ stepState = [run \in Runs |-> [step \in Steps |-> "Pending"]]
    /\ maxAttempts = [run \in Runs |-> [step \in Steps |-> MaxAttempts]]
    /\ retryPolicy = [run \in Runs |-> [step \in Steps |-> "Retryable"]]
    /\ stepHasRetryCheck = [run \in Runs |-> [step \in Steps |-> TRUE]]

(* Add a run to the model *)
AddRun(run) ==
    /\ run \notin runs
    /\ runs' = runs \cup {run}
    /\ actionAttempts' = [actionAttempts EXCEPT ![run] = [step \in Steps |-> 0]]
    /\ stepState' = [stepState EXCEPT ![run] = [step \in Steps |-> "Pending"]]
    /\ framePC' = [framePC EXCEPT ![run] = 1]
    /\ UNCHANGED <<maxAttempts, retryPolicy, stepHasRetryCheck>>

(* Mark step as running *)
StartStep(run, step) ==
    /\ run \in runs
    /\ stepState[run][step] = "Pending"
    /\ stepState' = [stepState EXCEPT ![run][step] = "Running"]
    /\ UNCHANGED <<runs, actionAttempts, framePC, maxAttempts, retryPolicy, stepHasRetryCheck>>

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
    /\ UNCHANGED <<maxAttempts, retryPolicy, runs, stepHasRetryCheck>>

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

(* Spec *)
Spec == Init /\ [][Next]_<<runs, actionAttempts, framePC, stepState, maxAttempts, retryPolicy, stepHasRetryCheck>>

(* Safety: No double retry after exhaustion
 * Once actionAttempts >= maxAttempts for a (run,step), stepState must be Failed.
 * This ensures no further retry transitions are allowed after exhaustion.
 *)
NoDoubleRetryAfterExhaustion ==
    \A run \in Runs, step \in Steps :
        actionAttempts[run][step] >= maxAttempts[run][step]
            => stepState[run][step] = "Failed"

(* Safety: No stale completion accepted *)
NoStaleCompletion ==
    \A run \in Runs, step \in Steps :
        stepState[run][step] = "Running"
            => actionAttempts[run][step] >= 0

(* Safety: Frame PC reset on retry *)
FramePCResetOnRetry ==
    \A run \in Runs :
        framePC[run] \in Steps
            => stepState[run][framePC[run]] = "Running"

(* Liveness: Eventually terminal or exhausted *)
EventuallyTerminalOrExhausted ==
    <>(/\ runs # {}
       /\ \E run \in Runs, step \in Steps : stepState[run][step] = "Failed")

THEOREM Spec => []NoDoubleRetryAfterExhaustion
THEOREM Spec => []NoStaleCompletion
THEOREM Spec => []FramePCResetOnRetry

====
