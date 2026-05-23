---- MODULE EngineYamlAdmission ----
EXTENDS Naturals, FiniteSets

\* Obligations: PO-002 / TLA-ADMIT-001.
\* Abstracts strict admission as finite durable-record cases. Fjall batch
\* persistence is represented by one atomic success/failure decision.

CONSTANTS Artifacts, Gates, RequiredRecords

VARIABLES artifact_state, durable_records, ack_state, proof_gates

vars == <<artifact_state, durable_records, ack_state, proof_gates>>

TypeOK ==
  /\ artifact_state \in {"source", "validated", "compiled", "accepted_artifact",
                        "verified", "persisted"}
  /\ durable_records \subseteq Records
  /\ ack_state \in {"pending", "acked", "failed"}
  /\ proof_gates \subseteq Gates

InitAdmission ==
  /\ artifact_state = "source"
  /\ durable_records = {}
  /\ ack_state = "pending"
  /\ proof_gates = {}

ValidateYamlCold ==
  /\ artifact_state = "source"
  /\ artifact_state' = "validated"
  /\ UNCHANGED <<durable_records, ack_state, proof_gates>>

CompileNumericIr ==
  /\ artifact_state = "validated"
  /\ artifact_state' = "compiled"
  /\ UNCHANGED <<durable_records, ack_state, proof_gates>>

BuildArtifact ==
  /\ artifact_state = "compiled"
  /\ artifact_state' = "accepted_artifact"
  /\ UNCHANGED <<durable_records, ack_state, proof_gates>>

VerifyGates ==
  /\ artifact_state = "accepted_artifact"
  /\ proof_gates' = Gates
  /\ artifact_state' = "verified"
  /\ UNCHANGED <<durable_records, ack_state>>

PersistBatch ==
  /\ artifact_state = "verified"
  /\ proof_gates = Gates
  /\ durable_records' = RequiredRecords
  /\ artifact_state' = "persisted"
  /\ UNCHANGED <<ack_state, proof_gates>>

AckAccepted ==
  /\ artifact_state = "persisted"
  /\ durable_records = RequiredRecords
  /\ ack_state' = "acked"
  /\ UNCHANGED <<artifact_state, durable_records, proof_gates>>

FailBeforeAck ==
  /\ ack_state = "pending"
  /\ artifact_state # "persisted"
  /\ ack_state' = "failed"
  /\ UNCHANGED <<artifact_state, durable_records, proof_gates>>

Stutter == UNCHANGED vars

AdmissionProgress == ValidateYamlCold \/ CompileNumericIr \/ BuildArtifact \/ VerifyGates
                     \/ PersistBatch \/ AckAccepted \/ FailBeforeAck

Next == AdmissionProgress \/ Stutter

Spec == InitAdmission /\ [][Next]_vars /\ WF_vars(AdmissionProgress)

NoAckWithoutDurableAcceptedRecords ==
  ack_state = "acked" => durable_records = RequiredRecords /\ artifact_state = "persisted"

NoRawIrBypass ==
  ack_state = "acked" => proof_gates = Gates /\ artifact_state # "compiled"

AckOrFailState == ack_state \in {"pending", "acked", "failed"}

EventuallyAckOrFailBeforeAck == <>(ack_state \in {"acked", "failed"})

====
