# TLA+ Temporal Model Plan: vb-scxh

## Boundary

- Temporal behavior: false-closure discovery, BD reopen/link verification, evidence classification, laundering attempt rejection, safety-anchor verification, bundle packaging, Truth Serum review, final decision, close, and engine unblock.
- Rust/core behavior excluded: no production Rust state transition is in scope for this recovery bead.
- External systems abstracted: BD, git bundle/bookmark storage, `moon ci`, mutation report, generated parity follow-up beads, and subagent reports.

## Canonical artifact path

- Module/model path: `.beads/vb-scxh/tla/ScxhRecovery.tla`.
- Config path: `.beads/vb-scxh/tla/ScxhRecovery.cfg`.
- Exact command: `tlc -config .beads/vb-scxh/tla/ScxhRecovery.cfg .beads/vb-scxh/tla/ScxhRecovery.tla`.
- `.beads/vb-scxh/specs/` is intentionally rejected for this repair unless State 5 moves/rewrites artifacts and reruns the exact moved command.

## TLA+-owned clauses

- TLA-SCXH-001 -> `.beads/vb-scxh/tla/ScxhRecovery.tla::NoEngineUnblockBeforeApprovedEvidence` and `FalseClosuresVerifiedBeforeClose`.
- TLA-SCXH-002 -> `.beads/vb-scxh/tla/ScxhRecovery.tla::NoAcceptanceFromSubagentRequiredEvidence` with an explicit `AttemptLaunderSubagentEvidence` action/candidate.
- TLA-SCXH-003 -> `.beads/vb-scxh/tla/ScxhRecovery.tla::MutationUnviableNeverCountsAsAdequacy`.
- TLA-SCXH-004 -> `.beads/vb-scxh/tla/ScxhRecovery.tla::GeneratedParityGapRemainsDeferred`.
- TLA-SCXH-005 -> `.beads/vb-scxh/tla/ScxhRecovery.tla::SafetyAnchorRequiredForApproval`.

## Model shape

- Variables:
  - `closures`: finite set containing exactly 12 disputed closure IDs or 12 abstract closure slots until State 11 supplies IDs.
  - `closureState`: closure -> `Closed | Reopened | Linked | Verified | MissingBdEvidence`.
  - `requiredEvidence`: named evidence items: `bd_12_ids`, `green_ci`, `safety_bundle`, `safety_bookmark`, `mutation_classification`, `parity_deferral`, `truth_serum_decision`.
  - `candidateEvidence`: proposed evidence source for each item, including a subagent-only candidate.
  - `classification`: evidence item -> `Raw | Artifact | Subagent | Deferred | Blocked`.
  - `launderingAttempted`: boolean or set of items attempted as subagent-only acceptance.
  - `bundleState`: `Empty | Draft | Packaged | TruthSerumReviewed | DecisionReady | Blocked`.
  - `safetyAnchorState`: `Unknown | Verified | Missing | OpenFailed`.
  - `mutationState`: `Unknown | Pass | Fail | FailUnviable | Deferred`.
  - `parityGapState`: `Unknown | DeferredToFollowup | ExhaustivelyProven`.
  - `engineBlocked`: boolean.
- Init action: `InitRecovery` starts closures unverified, evidence unknown/missing, safety anchor unknown or open-failed, mutation fail-unviable/deferred, parity deferred/unknown, engine blocked.
- Required actions: `RecordRawEvidence`, `RecordArtifactEvidence`, `RecordSubagentClaim`, `AttemptLaunderSubagentEvidence`, `RejectLaunderedEvidence`, `ReopenClosure`, `LinkClosure`, `VerifySafetyAnchor`, `MarkSafetyAnchorBlocked`, `ClassifyMutationUnviable`, `DeferParityGap`, `PackageBundle`, `TruthSerumReject`, `TruthSerumAccept`, `MakeFinalDecision`, `CloseRecoveryBead`, `UnblockEngine`.
- State constraints: finite closure/evidence sets; include 12 closures, green CI, safety bundle, safety bookmark, mutation, parity deferral, and one subagent-only candidate.

## Required properties

- Safety invariants:
  - `NoEngineUnblockBeforeApprovedEvidence`: `engineBlocked = FALSE` implies approved final decision, all required evidence satisfied by raw/artifact-backed records, no blocker, all closures verified.
  - `NoAcceptanceFromSubagentRequiredEvidence`: if required evidence is supplied only by `Subagent`, then `PackageBundle`, `TruthSerumAccept`, `MakeFinalDecision`, and `UnblockEngine` cannot accept it.
  - `LaunderingAttemptRejected`: after `AttemptLaunderSubagentEvidence`, the item is `Blocked` or remains unsatisfied until separate raw/artifact evidence is recorded.
  - `FalseClosuresVerifiedBeforeClose`: recovery close implies all 12 closures have BD raw reopened/linked/verified evidence.
  - `SafetyAnchorRequiredForApproval`: `OpenFailed` or `Missing` safety anchor prevents approved final decision and engine unblock.
  - `MutationUnviableNotPass`: `FailUnviable` never implies adequacy pass.
  - `ParityGapOwnershipPreserved`: representative generated parity does not imply exhaustive parity; gap remains deferred to `vb-gvmt` / `vb-qi37.10` unless separately proven.
- Temporal stance:
  - If all required raw/artifact evidence is present and Truth Serum approves, eventually `DecisionReady` may be reached.
  - If required raw evidence is missing or safety anchor is `OpenFailed`, the model may reach terminal `Blocked`; approval/unblock remains impossible.
  - If liveness/fairness is not configured in State 5 TLC, State 5 must record a waiver removing liveness claims from evidence; safety invariants alone are not closure proof.
- Fairness: weak fairness may be applied to `PackageBundle`, `TruthSerumAccept/Reject`, `MakeFinalDecision` if State 5 configures temporal properties.
- Deadlock: no deadlock before terminal `DecisionReady`, `Closed`, or `Blocked` states.
- Refinement: State 11/12 artifacts refine model actions by exact raw command outputs, artifact markers, and final decisions under `.beads/vb-scxh/`.

## Evidence command

- Required exact command after State 5 repair: `tlc -config .beads/vb-scxh/tla/ScxhRecovery.cfg .beads/vb-scxh/tla/ScxhRecovery.tla`.
- Expected evidence: TLC exits 0, records no invariant violations, uses `.beads/vb-scxh/tla/` paths, and names the strengthened laundering and safety-anchor invariants.

## Waivers

- No waiver for TLA safety coverage of closure/evidence lifecycle.
- Liveness/fairness evidence may be waived only explicitly in State 5 if config remains invariant-only; such waiver must state that TLA PASS proves safety only, not eventual closure.
