---- MODULE ArtifactAdmission ----
EXTENDS Naturals, FiniteSets, Sequences, TLC

\* Obligations: TLA-ARTIFACT-001
\* Finite-state model of artifact admission lifecycle.
\* Models the gate_count mismatch between vb_storage (ADMISSION_GATE_COUNT=2)
\* and vb_runtime (REQUIRED_GATE_COUNT=15).
\*
\* CanonicalGate=15 mirrors vb_runtime::admission::REQUIRED_GATE_COUNT.
\* ADMISSION_GATE_COUNT=2 mirrors vb_storage admission constant.

CONSTANT CanonicalGate
ASSUME CanonicalGate = 15

VARIABLES artifactDigest, artifactState, gateCount, proofFlags,
          policy, errorMsg

vars == <<artifactDigest, artifactState, gateCount, proofFlags,
          policy, errorMsg>>

ADMISSION_GATE_COUNT == 2
REQUIRED_GATE_COUNT == CanonicalGate

FlagField == {"bounded", "taint_safe", "retry_safe", "replayable", "durable"}

Init ==
    /\ artifactState = "Pending"
    /\ gateCount \in 0..CanonicalGate
    /\ proofFlags \in [FlagField -> BOOLEAN]
    /\ policy \in {"Relaxed", "Journaled", "Strict"}
    /\ errorMsg = ""
    /\ artifactDigest = [digest |-> 0]

\* All proof flags must be true for Strict/Journaled admission.
AllFlagsTrue ==
    /\ proofFlags["bounded"] = TRUE
    /\ proofFlags["taint_safe"] = TRUE
    /\ proofFlags["retry_safe"] = TRUE
    /\ proofFlags["replayable"] = TRUE
    /\ proofFlags["durable"] = TRUE

\* ---- Artifact Submission ----

SubmitArtifact ==
    /\ artifactState = "Pending"
    /\ policy = "Relaxed"
    /\ artifactState' = "Stored"
    /\ gateCount' = 0
    /\ proofFlags' = [f \in FlagField |-> FALSE]
    /\ errorMsg' = ""
    /\ UNCHANGED <<artifactDigest, policy>>

LoadForAdmission ==
    /\ artifactState = "Pending"
    /\ policy \in {"Journaled", "Strict"}
    /\ artifactState' = "Stored"
    /\ gateCount' = ADMISSION_GATE_COUNT
    /\ proofFlags' = [f \in FlagField |-> TRUE]
    /\ errorMsg' = ""
    /\ UNCHANGED <<artifactDigest, policy>>

\* ---- Artifact Admission (Strict) ----

AdmitStrict ==
    /\ artifactState = "Stored"
    /\ policy = "Strict"
    /\ gateCount = REQUIRED_GATE_COUNT
    /\ AllFlagsTrue
    /\ artifactState' = "Admitted"
    /\ errorMsg' = ""
    /\ UNCHANGED <<gateCount, proofFlags, artifactDigest, policy>>

AdmitRelaxed ==
    /\ artifactState = "Stored"
    /\ policy = "Relaxed"
    /\ artifactState' = "Admitted"
    /\ errorMsg' = ""
    /\ UNCHANGED <<gateCount, proofFlags, artifactDigest, policy>>

RejectGateCount ==
    /\ artifactState = "Stored"
    /\ policy = "Strict"
    /\ gateCount # REQUIRED_GATE_COUNT
    /\ artifactState' = "Rejected"
    /\ errorMsg' = "InvalidGateCount"
    /\ UNCHANGED <<gateCount, proofFlags, artifactDigest, policy>>

RejectProofFlag ==
    /\ artifactState = "Stored"
    /\ policy \in {"Journaled", "Strict"}
    /\ ~AllFlagsTrue
    /\ artifactState' = "Rejected"
    /\ errorMsg' = "MissingProofFlag"
    /\ UNCHANGED <<gateCount, proofFlags, artifactDigest, policy>>

Stutter == UNCHANGED vars

Next ==
    \/ SubmitArtifact
    \/ LoadForAdmission
    \/ AdmitStrict
    \/ AdmitRelaxed
    \/ RejectGateCount
    \/ RejectProofFlag
    \/ Stutter

Spec == Init /\ [][Next]_vars

\* ---- Invariants ----

\* Artifact admitted under Strict implies gate_count=15 and all flags true.
ArtifactAdmittedImpliesValidGateCount ==
    artifactState = "Admitted" /\ policy = "Strict"
        => gateCount = REQUIRED_GATE_COUNT /\ AllFlagsTrue

\* Strict policy rejects artifacts with gate_count=2.
StrictPolicyRejectsTwoGate ==
    artifactState = "Rejected" /\ policy = "Strict" /\ errorMsg = "InvalidGateCount"
        => gateCount = 2

EventuallyStoredOrRejected ==
    <> (\/ artifactState = "Stored" \/ artifactState = "Rejected")

====