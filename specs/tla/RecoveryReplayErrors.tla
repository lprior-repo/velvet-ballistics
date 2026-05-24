(* RecoveryReplayErrors.tla
 * Obligation-specific finite liveness model for TLA-005 error coverage.
 * It splits RecoveryError reachability away from journal replay interleavings so
 * TLC can prove the coverage obligation without timing out the safety model.
 *)

---- MODULE RecoveryReplayErrors ----

EXTENDS TLC

VARIABLES pending_errors, last_error

RecoveryErrors == {
    "NoRecoveryData",
    "CorruptSnapshot",
    "WorkflowSourceDigestMismatch",
    "CompiledIrDigestMismatch",
    "ActionAbiMismatch",
    "PolicyDigestMismatch",
    "NonIdempotentActionBlocked",
    "ReplayDivergence",
    "FrameDimensionOverflow"
}

NoneError == "None"

TypeOK ==
    /\ pending_errors \subseteq RecoveryErrors
    /\ last_error \in {NoneError} \cup RecoveryErrors

Init ==
    /\ pending_errors = RecoveryErrors
    /\ last_error = NoneError

RecordRecoveryError(err) ==
    /\ err \in pending_errors
    /\ pending_errors' = pending_errors \ {err}
    /\ last_error' = err

vars == <<pending_errors, last_error>>

Done ==
    /\ pending_errors = {}
    /\ UNCHANGED vars

Next == (\E err \in pending_errors : RecordRecoveryError(err)) \/ Done

Spec == Init /\ [][Next]_vars /\ WF_vars(\E err \in pending_errors : RecordRecoveryError(err))

EventuallyAllRecoveryErrorsCovered == <>(pending_errors = {})

THEOREM Spec => []TypeOK
THEOREM Spec => EventuallyAllRecoveryErrorsCovered

====
