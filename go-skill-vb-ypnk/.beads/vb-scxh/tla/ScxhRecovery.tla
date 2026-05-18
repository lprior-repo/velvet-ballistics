---- MODULE ScxhRecovery ----
EXTENDS Naturals, FiniteSets, TLC

(* State 5 proof artifact for vb-scxh.
   Obligations: TLA-SCXH-001 through TLA-SCXH-005.
   The model abstracts BD, git, CI, and reports as evidence transitions. *)

CONSTANTS Closures, Evidence

VARIABLES closureState,
          classification,
          bundleState,
          finalDecision,
          recoveryClosed,
          engineBlocked,
          mutationAdequacy,
          parityExhaustive,
          launderingStatus

ClosureStates == {"Unverified", "Reopened", "Linked", "Verified"}
Classifications == {"Missing", "Raw", "Artifact", "Subagent", "Deferred", "Blocked"}
BundleStates == {"Empty", "Draft", "Packaged", "Reviewed", "Blocked"}
DecisionStates == {"Pending", "Approved", "Rejected"}
LaunderingStates == {"None", "Attempted", "Rejected"}

RequiredEvidence == {"green_ci", "safety_bundle", "safety_bookmark"}
SubagentOnlyEvidence == {"subagent_only_claim"}
MutationEvidence == "mutation_fail_unviable"
ParityEvidence == "parity_gap"

TypeOK ==
    /\ closureState \in [Closures -> ClosureStates]
    /\ classification \in [Evidence -> Classifications]
    /\ bundleState \in BundleStates
    /\ finalDecision \in DecisionStates
    /\ recoveryClosed \in BOOLEAN
    /\ engineBlocked \in BOOLEAN
    /\ mutationAdequacy \in BOOLEAN
    /\ parityExhaustive \in BOOLEAN
    /\ launderingStatus \in LaunderingStates

AllClosuresVerified ==
    \A c \in Closures : closureState[c] \in {"Reopened", "Linked", "Verified"}

AllRequiredRaw ==
    \A e \in RequiredEvidence : classification[e] = "Raw"

NoEvidenceBlocked ==
    \A e \in Evidence : classification[e] # "Blocked"

NoSubagentRequired ==
    \A e \in RequiredEvidence : classification[e] # "Subagent"

EvidenceApproved ==
    /\ bundleState = "Reviewed"
    /\ finalDecision = "Approved"
    /\ AllRequiredRaw
    /\ AllClosuresVerified
    /\ NoEvidenceBlocked
    /\ NoSubagentRequired
    /\ classification[MutationEvidence] = "Deferred"
    /\ classification[ParityEvidence] = "Deferred"
    /\ mutationAdequacy = FALSE
    /\ parityExhaustive = FALSE
    /\ launderingStatus # "Attempted"

Init ==
    /\ closureState = [c \in Closures |-> "Unverified"]
    /\ classification = [e \in Evidence |-> "Missing"]
    /\ bundleState = "Empty"
    /\ finalDecision = "Pending"
    /\ recoveryClosed = FALSE
    /\ engineBlocked = TRUE
    /\ mutationAdequacy = FALSE
    /\ parityExhaustive = FALSE
    /\ launderingStatus = "None"

RecordRequiredRawEvidence ==
    /\ \E e \in RequiredEvidence : classification[e] \in {"Missing", "Artifact"}
    /\ classification' = [e \in Evidence |->
            IF e \in RequiredEvidence /\ classification[e] \in {"Missing", "Artifact"}
            THEN "Raw" ELSE classification[e]]
    /\ UNCHANGED <<closureState, bundleState, finalDecision, recoveryClosed,
                  engineBlocked, mutationAdequacy, parityExhaustive,
                  launderingStatus>>

RecordArtifactEvidence ==
    /\ classification["green_ci"] = "Missing"
    /\ classification' = [classification EXCEPT !["green_ci"] = "Artifact"]
    /\ UNCHANGED <<closureState, bundleState, finalDecision, recoveryClosed,
                  engineBlocked, mutationAdequacy, parityExhaustive,
                  launderingStatus>>

RecordSubagentClaim ==
    /\ classification["subagent_only_claim"] = "Missing"
    /\ classification' = [classification EXCEPT !["subagent_only_claim"] = "Subagent"]
    /\ UNCHANGED <<closureState, bundleState, finalDecision, recoveryClosed,
                  engineBlocked, mutationAdequacy, parityExhaustive,
                  launderingStatus>>

AttemptLaunderSubagentEvidence ==
    /\ launderingStatus = "None"
    /\ classification["safety_bundle"] # "Raw"
    /\ classification' = [classification EXCEPT
            !["safety_bundle"] = "Subagent",
            !["subagent_only_claim"] = "Subagent"]
    /\ launderingStatus' = "Attempted"
    /\ UNCHANGED <<closureState, bundleState, finalDecision, recoveryClosed,
                  engineBlocked, mutationAdequacy, parityExhaustive>>

MarkMissingEvidenceBlocked ==
    /\ \E e \in RequiredEvidence : classification[e] = "Missing"
    /\ classification' = [e \in Evidence |->
            IF e \in RequiredEvidence /\ classification[e] = "Missing" THEN "Blocked" ELSE classification[e]]
    /\ UNCHANGED <<closureState, bundleState, finalDecision, recoveryClosed,
                  engineBlocked, mutationAdequacy, parityExhaustive,
                  launderingStatus>>

ReopenClosures ==
    /\ \E c \in Closures : closureState[c] = "Unverified"
    /\ closureState' = [c \in Closures |-> "Reopened"]
    /\ UNCHANGED <<classification, bundleState, finalDecision, recoveryClosed,
                  engineBlocked, mutationAdequacy, parityExhaustive,
                  launderingStatus>>

LinkClosures ==
    /\ \A c \in Closures : closureState[c] \in {"Reopened", "Linked", "Verified"}
    /\ \E c \in Closures : closureState[c] = "Reopened"
    /\ closureState' = [c \in Closures |-> "Linked"]
    /\ UNCHANGED <<classification, bundleState, finalDecision, recoveryClosed,
                  engineBlocked, mutationAdequacy, parityExhaustive,
                  launderingStatus>>

VerifyClosures ==
    /\ \A c \in Closures : closureState[c] \in {"Reopened", "Linked", "Verified"}
    /\ closureState' = [c \in Closures |-> "Verified"]
    /\ UNCHANGED <<classification, bundleState, finalDecision, recoveryClosed,
                  engineBlocked, mutationAdequacy, parityExhaustive,
                  launderingStatus>>

ClassifyMutationUnviable ==
    /\ classification' = [classification EXCEPT ![MutationEvidence] = "Deferred"]
    /\ mutationAdequacy' = FALSE
    /\ UNCHANGED <<closureState, bundleState, finalDecision, recoveryClosed,
                  engineBlocked, parityExhaustive, launderingStatus>>

DeferParityGap ==
    /\ classification' = [classification EXCEPT ![ParityEvidence] = "Deferred"]
    /\ parityExhaustive' = FALSE
    /\ UNCHANGED <<closureState, bundleState, finalDecision, recoveryClosed,
                  engineBlocked, mutationAdequacy, launderingStatus>>

PackageBundle ==
    /\ bundleState \in {"Empty", "Draft"}
    /\ bundleState' = "Packaged"
    /\ UNCHANGED <<closureState, classification, finalDecision, recoveryClosed,
                  engineBlocked, mutationAdequacy, parityExhaustive,
                  launderingStatus>>

TruthSerumRejectLaunderedEvidence ==
    /\ bundleState = "Packaged"
    /\ launderingStatus = "Attempted"
    /\ classification["safety_bundle"] = "Subagent"
    /\ classification' = [classification EXCEPT !["safety_bundle"] = "Blocked"]
    /\ bundleState' = "Blocked"
    /\ finalDecision' = "Rejected"
    /\ launderingStatus' = "Rejected"
    /\ UNCHANGED <<closureState, recoveryClosed, engineBlocked,
                  mutationAdequacy, parityExhaustive>>

TruthSerumReject ==
    /\ bundleState = "Packaged"
    /\ ~EvidenceApproved
    /\ bundleState' = "Blocked"
    /\ finalDecision' = "Rejected"
    /\ UNCHANGED <<closureState, classification, recoveryClosed, engineBlocked,
                  mutationAdequacy, parityExhaustive, launderingStatus>>

TruthSerumAccept ==
    /\ bundleState = "Packaged"
    /\ AllRequiredRaw
    /\ AllClosuresVerified
    /\ NoEvidenceBlocked
    /\ NoSubagentRequired
    /\ classification[MutationEvidence] = "Deferred"
    /\ classification[ParityEvidence] = "Deferred"
    /\ bundleState' = "Reviewed"
    /\ UNCHANGED <<closureState, classification, finalDecision, recoveryClosed,
                  engineBlocked, mutationAdequacy, parityExhaustive,
                  launderingStatus>>

MakeFinalDecision ==
    /\ bundleState = "Reviewed"
    /\ finalDecision = "Pending"
    /\ AllRequiredRaw
    /\ AllClosuresVerified
    /\ NoEvidenceBlocked
    /\ NoSubagentRequired
    /\ classification[MutationEvidence] = "Deferred"
    /\ classification[ParityEvidence] = "Deferred"
    /\ finalDecision' = "Approved"
    /\ UNCHANGED <<closureState, classification, bundleState, recoveryClosed,
                  engineBlocked, mutationAdequacy, parityExhaustive,
                  launderingStatus>>

CloseRecoveryBead ==
    /\ EvidenceApproved
    /\ recoveryClosed' = TRUE
    /\ UNCHANGED <<closureState, classification, bundleState, finalDecision,
                  engineBlocked, mutationAdequacy, parityExhaustive,
                  launderingStatus>>

UnblockEngine ==
    /\ EvidenceApproved
    /\ recoveryClosed
    /\ engineBlocked' = FALSE
    /\ UNCHANGED <<closureState, classification, bundleState, finalDecision,
                  recoveryClosed, mutationAdequacy, parityExhaustive,
                  launderingStatus>>

StutterTerminal ==
    /\ (engineBlocked = FALSE \/ bundleState = "Blocked")
    /\ UNCHANGED <<closureState, classification, bundleState, finalDecision,
                  recoveryClosed, engineBlocked, mutationAdequacy, parityExhaustive,
                  launderingStatus>>

Next ==
    \/ RecordRequiredRawEvidence
    \/ RecordArtifactEvidence
    \/ RecordSubagentClaim
    \/ AttemptLaunderSubagentEvidence
    \/ MarkMissingEvidenceBlocked
    \/ ReopenClosures
    \/ LinkClosures
    \/ VerifyClosures
    \/ ClassifyMutationUnviable
    \/ DeferParityGap
    \/ PackageBundle
    \/ TruthSerumRejectLaunderedEvidence
    \/ TruthSerumReject
    \/ TruthSerumAccept
    \/ MakeFinalDecision
    \/ CloseRecoveryBead
    \/ UnblockEngine
    \/ StutterTerminal

Spec == Init /\ [][Next]_<<closureState, classification, bundleState, finalDecision,
                     recoveryClosed, engineBlocked, mutationAdequacy, parityExhaustive,
                     launderingStatus>>

NoEngineUnblockBeforeApprovedEvidence ==
    (engineBlocked = FALSE) => (EvidenceApproved /\ recoveryClosed)

FalseClosuresVerifiedBeforeClose ==
    recoveryClosed => AllClosuresVerified

NoAcceptanceFromSubagentRequiredEvidence ==
    finalDecision = "Approved" => NoSubagentRequired

LaunderingAttemptRejected ==
    /\ (launderingStatus = "Attempted") =>
        /\ finalDecision # "Approved"
        /\ engineBlocked = TRUE
        /\ classification["safety_bundle"] = "Subagent"
    /\ (launderingStatus = "Rejected") =>
        /\ finalDecision = "Rejected"
        /\ bundleState = "Blocked"
        /\ engineBlocked = TRUE
        /\ classification["safety_bundle"] = "Blocked"

MutationUnviableNotPass ==
    classification[MutationEvidence] = "Deferred" => mutationAdequacy = FALSE

ParityGapOwnershipPreserved ==
    classification[ParityEvidence] = "Deferred" => parityExhaustive = FALSE

SafetyAnchorRequiredForApproval ==
    finalDecision = "Approved" =>
        /\ classification["safety_bundle"] = "Raw"
        /\ classification["safety_bookmark"] = "Raw"

====
