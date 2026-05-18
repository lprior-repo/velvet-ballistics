---- MODULE SilentDiscardLifecycle ----

\* Obligations: TLA-ACK-001, TLA-REC-002, TLA-DEADLOCK-011.
\* Bead-local finite model for required persist failure, recovery corruption,
\* and runtime diagnostic cause preservation. This model is intentionally
\* abstract: concrete Rust I/O, decoding, and error constructors are shell
\* refinements owned by later implementation and test states.

VARIABLES op, persist, ack, recovery, runtime, diagnostic

vars == <<op, persist, ack, recovery, runtime, diagnostic>>

Ops == {"none", "journal_append", "batch_commit", "engine_drive"}
PersistStates == {"not_started", "pending", "ok", "failed"}
AckStates == {"none", "success", "typed_error"}
RecoveryStates == {"none", "decode_absent", "decode_valid", "decode_corrupt", "decode_truncated", "success_empty", "success_value", "fail_closed"}
RuntimeStates == {"idle", "engine_failed", "terminal_failure"}
DiagnosticStates == {"none", "cause_present", "cause_preserved"}

Init ==
  /\ op = "none"
  /\ persist = "not_started"
  /\ ack = "none"
  /\ recovery = "none"
  /\ runtime = "idle"
  /\ diagnostic = "none"

StartMutation ==
  /\ op = "none"
  /\ op' \in {"journal_append", "batch_commit"}
  /\ persist' = "pending"
  /\ ack' = "none"
  /\ UNCHANGED <<recovery, runtime, diagnostic>>

PersistOk ==
  /\ persist = "pending"
  /\ persist' = "ok"
  /\ UNCHANGED <<op, ack, recovery, runtime, diagnostic>>

PersistFail ==
  /\ persist = "pending"
  /\ persist' = "failed"
  /\ diagnostic' = IF diagnostic = "cause_preserved" THEN "cause_preserved" ELSE "cause_present"
  /\ UNCHANGED <<op, ack, recovery, runtime>>

AckSuccess ==
  /\ persist = "ok"
  /\ ack = "none"
  /\ ack' = "success"
  /\ UNCHANGED <<op, persist, recovery, runtime, diagnostic>>

ReturnTypedError ==
  /\ persist = "failed"
  /\ ack = "none"
  /\ ack' = "typed_error"
  /\ diagnostic' = "cause_preserved"
  /\ UNCHANGED <<op, persist, recovery, runtime>>

BeginRecovery ==
  /\ recovery = "none"
  /\ recovery' \in {"decode_absent", "decode_valid", "decode_corrupt", "decode_truncated"}
  /\ UNCHANGED <<op, persist, ack, runtime, diagnostic>>

HydrateSuccess ==
  /\ recovery \in {"decode_absent", "decode_valid"}
  /\ recovery' \in {"success_empty", "success_value"}
  /\ recovery = "decode_valid" => recovery' = "success_value"
  /\ recovery = "decode_absent" => recovery' = "success_empty"
  /\ UNCHANGED <<op, persist, ack, runtime, diagnostic>>

HydrateFailClosed ==
  /\ recovery \in {"decode_corrupt", "decode_truncated"}
  /\ recovery' = "fail_closed"
  /\ diagnostic' = IF diagnostic = "cause_preserved" THEN "cause_preserved" ELSE "cause_present"
  /\ UNCHANGED <<op, persist, ack, runtime>>

EngineDriveFail ==
  /\ runtime = "idle"
  /\ runtime' = "engine_failed"
  /\ diagnostic' = "cause_present"
  /\ UNCHANGED <<op, persist, ack, recovery>>

RuntimeTerminalFail ==
  /\ runtime = "engine_failed"
  /\ diagnostic \in {"cause_present", "cause_preserved"}
  /\ runtime' = "terminal_failure"
  /\ diagnostic' = "cause_preserved"
  /\ UNCHANGED <<op, persist, ack, recovery>>

LifecycleComplete ==
  /\ (op \in {"journal_append", "batch_commit"} => ack # "none")
  /\ recovery \in {"none", "success_empty", "success_value", "fail_closed"}
  /\ runtime \in {"idle", "terminal_failure"}
  /\ \/ op # "none"
     \/ recovery # "none"
     \/ runtime # "idle"

ResetLifecycle ==
  /\ LifecycleComplete
  /\ op' = "none"
  /\ persist' = "not_started"
  /\ ack' = "none"
  /\ recovery' = "none"
  /\ runtime' = "idle"
  /\ diagnostic' = "none"

Next == StartMutation \/ PersistOk \/ PersistFail \/ AckSuccess \/ ReturnTypedError
        \/ BeginRecovery \/ HydrateSuccess \/ HydrateFailClosed
        \/ EngineDriveFail \/ RuntimeTerminalFail \/ ResetLifecycle

Spec == Init /\ [][Next]_vars
        /\ WF_vars(ReturnTypedError)
        /\ WF_vars(HydrateFailClosed)
        /\ WF_vars(RuntimeTerminalFail)

TypeOK ==
  /\ op \in Ops
  /\ persist \in PersistStates
  /\ ack \in AckStates
  /\ recovery \in RecoveryStates
  /\ runtime \in RuntimeStates
  /\ diagnostic \in DiagnosticStates

NoAckAfterFailedRequiredPersist ==
  persist = "failed" => ack # "success"

CorruptionDoesNotHydrateEmptySuccess ==
  recovery \in {"decode_corrupt", "decode_truncated", "fail_closed"} => recovery # "success_empty"

DiagnosticCausePreserved ==
  runtime = "terminal_failure" => diagnostic = "cause_preserved"

PersistFailureEventuallyTypedError ==
  [](persist = "failed" => <>(ack = "typed_error"))

RecoveryCorruptionEventuallyFailClosed ==
  [](recovery \in {"decode_corrupt", "decode_truncated"} => <>(recovery = "fail_closed"))

EngineFailureEventuallyCausePreserved ==
  [](runtime = "engine_failed" => <>(diagnostic = "cause_preserved"))

====
