---- MODULE EngineYamlRecovery ----
EXTENDS Naturals, FiniteSets

\* Obligations: PO-004 / TLA-REC-001 / vb-jpq7.3.
\* Recovery has no YAML parser transition. Hydration requires complete durable
\* records plus a usable latest snapshot authority. Corrupt latest snapshot
\* records, payload mismatches, bounded EventSeq overflow, and missing first
\* tail events fail closed with typed errors instead of silently hydrating.

CONSTANTS RequiredRecords, MaxSeq

VARIABLES run_state, durable_records, recovery_source, snapshot_status,
          snapshot_seq, tail_first, seq, error

vars == <<run_state, durable_records, recovery_source, snapshot_status,
          snapshot_seq, tail_first, seq, error>>

RunStates == {"stored", "recovering", "hydrated", "failed_closed"}
RecoverySources == {"durable", "corrupt", "mismatch", "empty"}
SnapshotStatuses == {"none", "valid", "bad_magic", "digest_mismatch",
                     "postcard_failed", "wrong_run", "wrong_seq"}
SnapshotFailureStatuses == SnapshotStatuses \ {"none", "valid"}
ErrorCodes == {"none", "BadMagic", "PayloadDigestMismatch",
               "PostcardDecodeFailed", "WrongRun", "SequenceGap",
               "EventSeqOverflow", "IncompleteEvidence"}

SnapshotUsable == snapshot_status \in {"none", "valid"}
TailStart == IF snapshot_status = "valid" THEN snapshot_seq + 1 ELSE 0
TailStartDefined == snapshot_status # "valid" \/ snapshot_seq < MaxSeq
CompleteEvidence == durable_records = RequiredRecords /\ recovery_source = "durable"

SnapshotError ==
  CASE snapshot_status = "bad_magic" -> "BadMagic"
    [] snapshot_status = "digest_mismatch" -> "PayloadDigestMismatch"
    [] snapshot_status = "postcard_failed" -> "PostcardDecodeFailed"
    [] snapshot_status = "wrong_run" -> "WrongRun"
    [] snapshot_status = "wrong_seq" -> "SequenceGap"
    [] OTHER -> "IncompleteEvidence"

TypeOK ==
  /\ MaxSeq \in Nat
  /\ run_state \in RunStates
  /\ durable_records \in SUBSET RequiredRecords
  /\ recovery_source \in RecoverySources
  /\ snapshot_status \in SnapshotStatuses
  /\ snapshot_seq \in 0..MaxSeq
  /\ tail_first \in 0..MaxSeq
  /\ seq \in 0..MaxSeq
  /\ error \in ErrorCodes

InitRecovery ==
  /\ TypeOK
  /\ run_state = "stored"
  /\ error = "none"
  /\ seq = 0

BeginRecovery ==
  /\ run_state = "stored"
  /\ run_state' = "recovering"
  /\ UNCHANGED <<durable_records, recovery_source, snapshot_status,
                  snapshot_seq, tail_first, seq, error>>

RejectCorruptLatestSnapshot ==
  /\ run_state = "recovering"
  /\ snapshot_status \in SnapshotFailureStatuses
  /\ run_state' = "failed_closed"
  /\ error' = SnapshotError
  /\ UNCHANGED <<durable_records, recovery_source, snapshot_status,
                  snapshot_seq, tail_first, seq>>

RejectSnapshotOverflow ==
  /\ run_state = "recovering"
  /\ snapshot_status = "valid"
  /\ snapshot_seq = MaxSeq
  /\ run_state' = "failed_closed"
  /\ error' = "EventSeqOverflow"
  /\ UNCHANGED <<durable_records, recovery_source, snapshot_status,
                  snapshot_seq, tail_first, seq>>

RejectMissingFirstTail ==
  /\ run_state = "recovering"
  /\ CompleteEvidence
  /\ SnapshotUsable
  /\ TailStartDefined
  /\ tail_first # TailStart
  /\ run_state' = "failed_closed"
  /\ error' = "SequenceGap"
  /\ UNCHANGED <<durable_records, recovery_source, snapshot_status,
                  snapshot_seq, tail_first, seq>>

FailClosedRecovery ==
  /\ run_state = "recovering"
  /\ ~CompleteEvidence
  /\ SnapshotUsable
  /\ run_state' = "failed_closed"
  /\ error' = "IncompleteEvidence"
  /\ UNCHANGED <<durable_records, recovery_source, snapshot_status,
                  snapshot_seq, tail_first, seq>>

HydrateFromDurableRecords ==
  /\ run_state = "recovering"
  /\ CompleteEvidence
  /\ SnapshotUsable
  /\ TailStartDefined
  /\ tail_first = TailStart
  /\ run_state' = "hydrated"
  /\ seq' = TailStart
  /\ error' = "none"
  /\ UNCHANGED <<durable_records, recovery_source, snapshot_status,
                  snapshot_seq, tail_first>>

Replay ==
  /\ run_state = "hydrated"
  /\ seq < MaxSeq
  /\ seq' = seq + 1
  /\ UNCHANGED <<run_state, durable_records, recovery_source,
                  snapshot_status, snapshot_seq, tail_first, error>>

Stutter == UNCHANGED vars

RecoveryProgress == BeginRecovery \/ RejectCorruptLatestSnapshot
                    \/ RejectSnapshotOverflow \/ RejectMissingFirstTail
                    \/ FailClosedRecovery \/ HydrateFromDurableRecords \/ Replay

Next == RecoveryProgress \/ Stutter

Spec == InitRecovery /\ [][Next]_vars /\ WF_vars(RecoveryProgress)

NoRuntimeYaml == recovery_source # "yaml"

FailClosedRecoveryInv ==
  run_state = "hydrated" => CompleteEvidence /\ SnapshotUsable /\ tail_first = TailStart

NoSilentEmptyFrame ==
  recovery_source = "empty" => run_state # "hydrated"

CorruptLatestSnapshotFailsClosed ==
  snapshot_status \in SnapshotFailureStatuses => run_state # "hydrated"

StrictTailStartInv ==
  run_state = "hydrated" => tail_first = TailStart /\ seq >= TailStart /\ error = "none"

SnapshotOverflowFailsClosed ==
  snapshot_status = "valid" /\ snapshot_seq = MaxSeq => run_state # "hydrated"

FailedClosedHasTypedError ==
  run_state = "failed_closed" => error # "none"

RecoveryEventuallyHydratesOrFailsClosed == <>(run_state \in {"hydrated", "failed_closed"})

====
