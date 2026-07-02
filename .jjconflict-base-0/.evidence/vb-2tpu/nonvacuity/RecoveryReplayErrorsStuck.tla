---- MODULE RecoveryReplayErrorsStuck ----

EXTENDS TLC

VARIABLE pending_errors

RecoveryErrors == {"NoRecoveryData", "CorruptSnapshot"}

Init == pending_errors = RecoveryErrors

Next == UNCHANGED pending_errors

vars == <<pending_errors>>

Spec == Init /\ [][Next]_vars

EventuallyAllRecoveryErrorsCovered == <>(pending_errors = {})

====
