---- MODULE AcceptedCliAdmission ----
EXTENDS Naturals

\* Obligations: PO-001 / TLA-001, TLA-002, TLA-003, INV-001,
\* INV-005, POST-001, POST-002, POST-004, ERR-005, ERR-006.
\* Finite verifier-only model for strict CLI accepted-artifact admission.

Bool == {TRUE, FALSE}
Phases == {"parsed", "compiled", "source_persisted", "artifact_persisted",
           "admitted", "boundary_persisted", "acknowledged", "rejected"}
Inputs == {"yaml", "raw"}

VARIABLES phase, source_persisted, artifact_persisted, boundary_persisted,
          run_state_inserted, acknowledged, digest_match, proof_valid,
          gate_valid, capability_valid, storage_write_ok, input_kind

vars == <<phase, source_persisted, artifact_persisted, boundary_persisted,
          run_state_inserted, acknowledged, digest_match, proof_valid,
          gate_valid, capability_valid, storage_write_ok, input_kind>>

ArtifactValid == digest_match /\ proof_valid /\ gate_valid /\ capability_valid
Terminal == phase \in {"acknowledged", "rejected"}
FailureInput == input_kind = "raw" \/ ~storage_write_ok \/ ~ArtifactValid

Init ==
  /\ phase = "parsed"
  /\ source_persisted = FALSE
  /\ artifact_persisted = FALSE
  /\ boundary_persisted = FALSE
  /\ run_state_inserted = FALSE
  /\ acknowledged = FALSE
  /\ digest_match \in Bool
  /\ proof_valid \in Bool
  /\ gate_valid \in Bool
  /\ capability_valid \in Bool
  /\ storage_write_ok \in Bool
  /\ input_kind \in Inputs

CompileYaml ==
  /\ phase = "parsed"
  /\ input_kind = "yaml"
  /\ phase' = "compiled"
  /\ UNCHANGED <<source_persisted, artifact_persisted, boundary_persisted,
                  run_state_inserted, acknowledged, digest_match, proof_valid,
                  gate_valid, capability_valid, storage_write_ok, input_kind>>

RejectRawStrictBypass ==
  /\ phase = "parsed"
  /\ input_kind = "raw"
  /\ phase' = "rejected"
  /\ UNCHANGED <<source_persisted, artifact_persisted, boundary_persisted,
                  run_state_inserted, acknowledged, digest_match, proof_valid,
                  gate_valid, capability_valid, storage_write_ok, input_kind>>

PersistSource ==
  /\ phase = "compiled"
  /\ phase' = "source_persisted"
  /\ source_persisted' = TRUE
  /\ UNCHANGED <<artifact_persisted, boundary_persisted, run_state_inserted,
                  acknowledged, digest_match, proof_valid, gate_valid,
                  capability_valid, storage_write_ok, input_kind>>

PersistArtifact ==
  /\ phase = "source_persisted"
  /\ storage_write_ok
  /\ phase' = "artifact_persisted"
  /\ artifact_persisted' = TRUE
  /\ UNCHANGED <<source_persisted, boundary_persisted, run_state_inserted,
                  acknowledged, digest_match, proof_valid, gate_valid,
                  capability_valid, storage_write_ok, input_kind>>

RejectBeforeArtifact ==
  /\ phase \in {"compiled", "source_persisted"}
  /\ ~storage_write_ok
  /\ phase' = "rejected"
  /\ UNCHANGED <<source_persisted, artifact_persisted, boundary_persisted,
                  run_state_inserted, acknowledged, digest_match, proof_valid,
                  gate_valid, capability_valid, storage_write_ok, input_kind>>

RejectInvalidArtifact ==
  /\ phase = "artifact_persisted"
  /\ ~ArtifactValid
  /\ phase' = "rejected"
  /\ UNCHANGED <<source_persisted, artifact_persisted, boundary_persisted,
                  run_state_inserted, acknowledged, digest_match, proof_valid,
                  gate_valid, capability_valid, storage_write_ok, input_kind>>

RuntimeAdmission ==
  /\ phase = "artifact_persisted"
  /\ ArtifactValid
  /\ phase' = "admitted"
  /\ run_state_inserted' = TRUE
  /\ UNCHANGED <<source_persisted, artifact_persisted, boundary_persisted,
                  acknowledged, digest_match, proof_valid, gate_valid,
                  capability_valid, storage_write_ok, input_kind>>

PersistAcceptedRunBoundary ==
  /\ phase = "admitted"
  /\ storage_write_ok
  /\ phase' = "boundary_persisted"
  /\ boundary_persisted' = TRUE
  /\ UNCHANGED <<source_persisted, artifact_persisted, run_state_inserted,
                  acknowledged, digest_match, proof_valid, gate_valid,
                  capability_valid, storage_write_ok, input_kind>>

RejectBoundaryWriteFailure ==
  /\ phase = "admitted"
  /\ ~storage_write_ok
  /\ phase' = "rejected"
  /\ UNCHANGED <<source_persisted, artifact_persisted, boundary_persisted,
                  run_state_inserted, acknowledged, digest_match, proof_valid,
                  gate_valid, capability_valid, storage_write_ok, input_kind>>

Acknowledge ==
  /\ phase = "boundary_persisted"
  /\ phase' = "acknowledged"
  /\ acknowledged' = TRUE
  /\ UNCHANGED <<source_persisted, artifact_persisted, boundary_persisted,
                  run_state_inserted, digest_match, proof_valid, gate_valid,
                  capability_valid, storage_write_ok, input_kind>>

TerminalStutter == Terminal /\ UNCHANGED vars

Progress == CompileYaml \/ RejectRawStrictBypass \/ PersistSource \/ PersistArtifact
        \/ RejectBeforeArtifact \/ RejectInvalidArtifact \/ RuntimeAdmission
        \/ PersistAcceptedRunBoundary \/ RejectBoundaryWriteFailure
        \/ Acknowledge

Next == Progress \/ TerminalStutter

Spec == Init /\ [][Next]_vars
        /\ WF_vars(CompileYaml)
        /\ WF_vars(RejectRawStrictBypass)
        /\ WF_vars(PersistSource)
        /\ WF_vars(PersistArtifact)
        /\ WF_vars(RejectBeforeArtifact)
        /\ WF_vars(RejectInvalidArtifact)
        /\ WF_vars(RuntimeAdmission)
        /\ WF_vars(PersistAcceptedRunBoundary)
        /\ WF_vars(RejectBoundaryWriteFailure)
        /\ WF_vars(Acknowledge)

TypeOk ==
  /\ phase \in Phases
  /\ source_persisted \in Bool
  /\ artifact_persisted \in Bool
  /\ boundary_persisted \in Bool
  /\ run_state_inserted \in Bool
  /\ acknowledged \in Bool
  /\ digest_match \in Bool
  /\ proof_valid \in Bool
  /\ gate_valid \in Bool
  /\ capability_valid \in Bool
  /\ storage_write_ok \in Bool
  /\ input_kind \in Inputs

RunRequiresAcceptedArtifact ==
  run_state_inserted => source_persisted /\ artifact_persisted /\ ArtifactValid

AckRequiresAtomicBoundary ==
  acknowledged => boundary_persisted /\ source_persisted /\ artifact_persisted /\ run_state_inserted

InvalidArtifactRejectsBeforeAck ==
  artifact_persisted /\ ~ArtifactValid => ~acknowledged

DigestBindingForAcceptedOutcomes ==
  (run_state_inserted \/ acknowledged) => digest_match

FailClosedRejection ==
  phase = "rejected" => ~acknowledged /\ ~boundary_persisted

StrictRawBypassNeverAdmitted ==
  input_kind = "raw" => ~run_state_inserted /\ ~acknowledged

EventuallyAcceptedOrRejected == <>(phase = "acknowledged" \/ phase = "rejected")

FailureEventuallyRejected == FailureInput => <>(phase = "rejected")

====
