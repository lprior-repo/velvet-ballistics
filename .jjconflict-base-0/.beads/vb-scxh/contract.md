# Contract Specification: vb-scxh

## Authority and scope

- Bead: `vb-scxh` - recover false 12-bead closure and restore green CI evidence integrity.
- State: Go-skill State 3 REPAIR after State 6 contract-verification schema rejection; this repair does not claim State 6 approval.
- Write scope: only `/home/lewis/src/vb-scxh/.beads/vb-scxh/`.
- Forbidden write scope: `/home/lewis/src/Velvet-ballistics` and all production/test/proof/model source files.
- Skill basis: `/home/lewis/.agents/skills/rust-contract/SKILL.md` and `/home/lewis/.claude/skills/rust-contract/SKILL.md`, both version 2.6.0; `.agents` wins on conflict. Relevant cited rules: `contract_first`, `tla_temporal_default`, `verification_first`, `proof_obligations_jsonl`, `review_required`, and `no_implementation`.

## Domain terms

- Raw command evidence: exact command, workdir, exit status, stdout/stderr markers, and artifact path captured from the isolated workspace or explicit external anchor.
- Artifact-derived evidence: claim derived from an existing artifact with path and quoted marker.
- Subagent claim: narrative assertion not backed by raw command evidence or an auditable artifact.
- False closure set: the exact 12 bead IDs previously closed without sufficient evidence.
- Reopened/linked status: BD raw status and dependency/follow-up link evidence for each false closure.
- Safety anchor: bundle `/home/lewis/src/Velvet-ballistics-rescue-20260513T022011Z.bundle` plus bookmark/ref `rescue-vb-scxh-ci-green-20260513T030158Z`.
- Generated parity gap: representative/generated-mode parity work owned by `vb-gvmt` / `vb-qi37.10`; it is not closure evidence for `vb-scxh` or engine-only acceptance.
- Mutation `FAIL_UNVIABLE`: cargo-mutants evidence where mutants cannot build; this is not mutation adequacy PASS.

## Preconditions

- PRE-SCXH-001: Verification and packaging commands must execute from `/home/lewis/src/vb-scxh`; State 3 writes must be under `.beads/vb-scxh/` only.
- PRE-SCXH-002: Recovery inputs must include State 1/2 artifacts plus referenced `vb-gvmt` evidence artifacts before close/unblock decisions.
- PRE-SCXH-003: Any bead-state, false-closure, reopened, or linked-status claim must come from `bd --db /home/lewis/src/.beads/dolt ...` raw output, never stale prose.

## Postconditions

- POST-SCXH-001: The recovery package enumerates the exact 12 false-closure IDs with raw BD reopened/linked/follow-up evidence for each.
- POST-SCXH-002: Every recovery claim is classified as `RAW_COMMAND`, `ARTIFACT_DERIVED`, `SUBAGENT_CLAIM`, `DEFERRED`, or `BLOCKED`; `SUBAGENT_CLAIM` cannot satisfy required acceptance evidence.
- POST-SCXH-003: Green CI is represented only with exact raw markers: `moon ci`, PASS, 19 completed tasks, 8276/8276 tests passed, runtime marker, and artifact path or a fresh rerun.
- POST-SCXH-004: Mutation evidence is classified as `FAIL_UNVIABLE` / `DEFERRED`; it is never relabeled as mutation adequacy PASS.
- POST-SCXH-005: Generated parity gaps are preserved as deferrals to `vb-gvmt` / `vb-qi37.10`; generated parity artifacts are scope-control inputs, not closure proof for this bead.
- POST-SCXH-006: `vb-engine-yaml` remains blocked until assurance bundle, Truth Serum report, and final evidence decision all approve close/unblock with no local blockers.
- POST-SCXH-007: Safety bundle/bookmark verification is primary evidence. In the State 3 proof ledger this remains `status: planned`, while any current or future bundle-open/ref failure is preserved as downstream `failure_classification: BLOCK_LOCAL` for State 11/12 close/unblock gating.

## Invariants

- INV-SCXH-001: Evidence integrity is non-laundering: a required evidence item proposed as `Subagent` must be rejected/blocked and cannot transition to accepted raw evidence without a distinct raw command/artifact record.
- INV-SCXH-002: No false closure recurrence: close/unblock requires raw BD state, complete evidence bundle, and approved Truth Serum/final decision.
- INV-SCXH-003: Scope separation: State 3 repair mutates only contract artifacts under `.beads/vb-scxh/`; no production code, tests, proof harnesses, CI config, generated artifacts, or `vb-gvmt` artifacts are changed.
- INV-SCXH-004: Generated parity remains representative unless a separate follow-up bead proves exhaustive parity.
- INV-SCXH-005: Mutation `FAIL_UNVIABLE` is a non-pass state requiring waiver/defer/follow-up.
- INV-SCXH-006: TLA proof paths are canonical at `.beads/vb-scxh/tla/ScxhRecovery.tla` and `.beads/vb-scxh/tla/ScxhRecovery.cfg`; `.beads/vb-scxh/specs/` is not an accepted target unless State 5 moves/rewrites and reruns exact commands.

## Error taxonomy and required traceability

- Error::WrongWorkspace - command/write path violates PRE-SCXH-001.
- Error::MissingRecoveryInput - required State 1/2 or referenced `vb-gvmt` artifact is absent/non-empty check fails.
- Error::MissingRawEvidence - a required raw command marker/artifact path is absent.
- Error::LaunderedSubagentClaim - subagent narrative is used as acceptance evidence without raw backing.
- Error::FalseClosureUnverified - exact 12 IDs or per-ID reopened/linked statuses are missing from raw BD evidence.
- Error::SafetyAnchorMissing - bundle/bookmark cannot be verified; proof-ledger rows remain `status: planned`, while downstream evidence must classify bundle-open/ref failure as `BLOCK_LOCAL` until repaired or explicitly waived.
- Error::MutationMisclassified - `FAIL_UNVIABLE` mutation output is represented as PASS/adequacy.
- Error::ScopeConflation - generated parity gaps are used to close engine-only acceptance or `vb-scxh`.
- Error::BlockedEngineUnblock - `vb-engine-yaml` is unblocked before final evidence decision passes.
- Error::TlaPathMismatch - obligations or commands reference `.beads/vb-scxh/specs/` while executed artifacts live under `.beads/vb-scxh/tla/`.

## Conceptual contract signatures (no implementation)

- `fn validate_workspace(path: WorkspacePath) -> Result<WorkspaceGuard, RecoveryError>`
- `fn classify_evidence(item: EvidenceItem) -> Result<EvidenceClassification, RecoveryError>`
- `fn audit_false_closures(snapshot: BdSnapshot) -> Result<ClosureRecoverySet, RecoveryError>`
- `fn verify_safety_anchor(anchor: SafetyAnchor) -> Result<VerifiedAnchor, RecoveryError>`
- `fn decide_engine_unblock(bundle: AssuranceBundle, truth: TruthSerumReport) -> Result<FinalEvidenceDecision, RecoveryError>`

## TLA+-owned clauses

- TLA-SCXH-001: closure/reopen/package/review/final-decision/engine-unblock lifecycle.
- TLA-SCXH-002: non-tautological subagent-laundering rejection: model a laundering attempt and prove approval/unblock cannot result from subagent-only required evidence.
- TLA-SCXH-003: mutation `FAIL_UNVIABLE` never counts as adequacy.
- TLA-SCXH-004: generated parity deferral remains owned by `vb-gvmt` / `vb-qi37.10`.
- TLA-SCXH-005: path consistency: exact model/config/command path is `.beads/vb-scxh/tla/`.

## Verus / theorem ownership and waivers

- No Rust-local production/core implementation is in scope; Verus, Lean/Aeneas/Hax, Kani, Flux, Loom, Miri, proptest, and fuzz lanes are waived/deferred in the primary proof-obligation ledger with owner, reason, expiry, and compensating evidence.
- If a later bead implements a pure evidence classifier, Verus becomes mandatory for classifier invariants before claiming Rust-local formal proof.

## Non-goals

- No production/test/proof/model code changes in State 3.
- No self-approval of these artifacts.
- No use of `vb-gvmt` generated parity evidence as closure proof for `vb-scxh`.
