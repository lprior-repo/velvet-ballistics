---- MODULE RetryFSM ----
EXTENDS Naturals

\* Obligations: VB-REPLAY-004, VB-REPLAY-005.

CONSTANTS MaxRetries, MaxTime

VARIABLES retryState, attemptCount, backoffUntil, now

vars == <<retryState, attemptCount, backoffUntil, now>>

States == {"ready", "backoff", "done", "exhausted"}

ValidRetryState ==
  /\ retryState \in States
  /\ attemptCount \in 0..MaxRetries
  /\ now \in 0..MaxTime
  /\ backoffUntil \in 0..MaxTime

BoundedState == ValidRetryState

MaxAttemptsRespected == attemptCount <= MaxRetries

BackoffDurationPositive == retryState = "backoff" => backoffUntil > now

Init ==
  /\ retryState = "ready"
  /\ attemptCount = 0
  /\ backoffUntil = 0
  /\ now = 0

AttemptRetry ==
  /\ retryState = "ready"
  /\ attemptCount < MaxRetries
  /\ now < MaxTime
  /\ attemptCount' = attemptCount + 1
  /\ retryState' = "backoff"
  /\ backoffUntil' = now + 1
  /\ UNCHANGED now

StartBackoff == AttemptRetry

EndBackoff ==
  /\ retryState = "backoff"
  /\ now + 1 >= backoffUntil
  /\ retryState' = "ready"
  /\ now' = backoffUntil
  /\ UNCHANGED <<attemptCount, backoffUntil>>

Tick ==
  /\ retryState = "backoff"
  /\ now + 1 < backoffUntil
  /\ now < MaxTime
  /\ now' = now + 1
  /\ UNCHANGED <<retryState, attemptCount, backoffUntil>>

ExhaustRetries ==
  /\ retryState = "ready"
  /\ attemptCount = MaxRetries
  /\ retryState' = "exhausted"
  /\ UNCHANGED <<attemptCount, backoffUntil, now>>

ResetRetry ==
  /\ retryState \in {"done", "exhausted"}
  /\ retryState' = "ready"
  /\ attemptCount' = 0
  /\ backoffUntil' = 0
  /\ UNCHANGED now

Complete ==
  /\ retryState = "ready"
  /\ retryState' = "done"
  /\ UNCHANGED <<attemptCount, backoffUntil, now>>

Next == AttemptRetry \/ EndBackoff \/ Tick \/ ExhaustRetries \/ ResetRetry \/ Complete

Spec == Init /\ [][Next]_vars /\ WF_vars(AttemptRetry) /\ WF_vars(EndBackoff) /\ WF_vars(ExhaustRetries) /\ WF_vars(Complete)

EventuallyExhaustedOrDone == <>(retryState \in {"done", "exhausted"})

====
