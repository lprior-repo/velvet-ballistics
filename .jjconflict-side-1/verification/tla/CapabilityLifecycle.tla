---- MODULE CapabilityLifecycle ----
EXTENDS Naturals

\* Obligations: CAP-CARD-002, GATE-MISMATCH-003, DRIVE-CONTRACT-006,
\* LEGACY-BYPASS-007.  Finite lifecycle model for Strict/Journaled
\* capability admission and Do execution.  Relaxed policy is out of scope.

CONSTANT CanonicalGate

GateCounts == {0, 2, CanonicalGate}
CapabilityCounts == 0..2
Bool == {TRUE, FALSE}

VARIABLES gate_count, required_count, grant_count, contracts_present,
          legacy_path, admission, run_allocated, journaled, drive_state

vars == <<gate_count, required_count, grant_count, contracts_present,
          legacy_path, admission, run_allocated, journaled, drive_state>>

Init ==
  /\ gate_count \in GateCounts
  /\ required_count \in CapabilityCounts
  /\ grant_count \in CapabilityCounts
  /\ contracts_present \in Bool
  /\ legacy_path \in Bool
  /\ admission = "pending"
  /\ run_allocated = FALSE
  /\ journaled = FALSE
  /\ drive_state = "idle"

ExactProfile == required_count = grant_count
GateMatches == gate_count = CanonicalGate
ProtectedSubmit == required_count > 0

DenyGateMismatch ==
  /\ admission = "pending"
  /\ ~GateMatches
  /\ admission' = "denied"
  /\ run_allocated' = FALSE
  /\ journaled' = FALSE
  /\ UNCHANGED <<gate_count, required_count, grant_count, contracts_present,
                  legacy_path, drive_state>>

DenyCapabilityProfile ==
  /\ admission = "pending"
  /\ GateMatches
  /\ ~ExactProfile
  /\ admission' = "denied"
  /\ run_allocated' = FALSE
  /\ journaled' = FALSE
  /\ UNCHANGED <<gate_count, required_count, grant_count, contracts_present,
                  legacy_path, drive_state>>

DenyLegacyBypass ==
  /\ admission = "pending"
  /\ legacy_path
  /\ ProtectedSubmit
  /\ admission' = "denied"
  /\ run_allocated' = FALSE
  /\ journaled' = FALSE
  /\ UNCHANGED <<gate_count, required_count, grant_count, contracts_present,
                  drive_state, legacy_path>>

AcceptAdmission ==
  /\ admission = "pending"
  /\ GateMatches
  /\ ExactProfile
  /\ ~legacy_path
  /\ admission' = "admitted"
  /\ run_allocated' = TRUE
  /\ journaled' = TRUE
  /\ UNCHANGED <<gate_count, required_count, grant_count, contracts_present,
                  legacy_path, drive_state>>

DriveDoWithoutContracts ==
  /\ admission = "admitted"
  /\ ~contracts_present
  /\ drive_state' = "denied"
  /\ UNCHANGED <<gate_count, required_count, grant_count, contracts_present,
                  legacy_path, admission, run_allocated, journaled>>

DriveDoWithContracts ==
  /\ admission = "admitted"
  /\ contracts_present
  /\ ExactProfile
  /\ drive_state' = "awaiting"
  /\ UNCHANGED <<gate_count, required_count, grant_count, contracts_present,
                  legacy_path, admission, run_allocated, journaled>>

Stutter == UNCHANGED vars

Next == DenyGateMismatch \/ DenyCapabilityProfile \/ DenyLegacyBypass
        \/ AcceptAdmission \/ DriveDoWithoutContracts \/ DriveDoWithContracts
        \/ Stutter

Spec == Init /\ [][Next]_vars

ExactProfileRequired ==
  admission = "admitted" => GateMatches /\ ExactProfile /\ ~legacy_path

ExcessGrantDenied ==
  grant_count > required_count => admission # "admitted"

NoAdmissionOnGateMismatch ==
  ~GateMatches => admission # "admitted"

NoRunAllocatedOnDeniedAdmission ==
  admission = "denied" => ~run_allocated /\ ~journaled

NoDoAwaitingWithoutContract ==
  drive_state = "awaiting" => contracts_present

ContractedDoRequiresExactGrant ==
  drive_state = "awaiting" => contracts_present /\ ExactProfile

NoLegacyBypassForProtectedSubmit ==
  legacy_path /\ ProtectedSubmit => admission # "admitted"

====
