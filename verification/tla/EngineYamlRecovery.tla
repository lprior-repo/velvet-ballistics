---- MODULE EngineYamlRecovery ----
EXTENDS Naturals, FiniteSets

\* Obligations: PO-004 / TLA-REC-001.
\* Recovery has no YAML parser transition. Success requires complete durable
\* records and digest match; all other cases fail closed.

CONSTANT RequiredRecords

VARIABLES run_state, durable_records, recovery_source, seq

vars == <<run_state, durable_records, recovery_source, seq>>

InitRecovery ==
  /\ run_state = "stored"
  /\ durable_records \in SUBSET RequiredRecords
  /\ recovery_source \in {"durable", "corrupt", "mismatch", "empty"}
  /\ seq = 0

BeginRecovery ==
  /\ run_state = "stored"
  /\ run_state' = "recovering"
  /\ UNCHANGED <<durable_records, recovery_source, seq>>

CompleteEvidence == durable_records = RequiredRecords /\ recovery_source = "durable"

HydrateFromDurableRecords ==
  /\ run_state = "recovering"
  /\ CompleteEvidence
  /\ run_state' = "hydrated"
  /\ UNCHANGED <<durable_records, recovery_source, seq>>

DetectMismatch ==
  /\ run_state = "recovering"
  /\ recovery_source \in {"corrupt", "mismatch"}
  /\ run_state' = "failed_closed"
  /\ UNCHANGED <<durable_records, recovery_source, seq>>

FailClosedRecovery ==
  /\ run_state = "recovering"
  /\ ~CompleteEvidence
  /\ run_state' = "failed_closed"
  /\ UNCHANGED <<durable_records, recovery_source, seq>>

Replay ==
  /\ run_state = "hydrated"
  /\ seq < 3
  /\ seq' = seq + 1
  /\ UNCHANGED <<run_state, durable_records, recovery_source>>

Stutter == UNCHANGED vars

RecoveryProgress == BeginRecovery \/ HydrateFromDurableRecords \/ DetectMismatch
                    \/ FailClosedRecovery \/ Replay

Next == RecoveryProgress \/ Stutter

Spec == InitRecovery /\ [][Next]_vars /\ WF_vars(RecoveryProgress)

NoRuntimeYaml == recovery_source # "yaml"

FailClosedRecoveryInv ==
  run_state = "hydrated" => CompleteEvidence

NoSilentEmptyFrame ==
  recovery_source = "empty" => run_state # "hydrated"

RecoveryEventuallyHydratesOrFailsClosed == <>(run_state \in {"hydrated", "failed_closed"})

====
