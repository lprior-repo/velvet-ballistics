# Contract Specification: vb-l2d7

## Context
- Feature: Reconcile `velvet-ballistics-MASTER.md` so resolved DRIFT-1 joined-taint behavior is internally consistent.
- Scope: Documentation-only change planning. No production code, proof code, harness code, tests, bead status, commits, or pushes in State 1.
- Authoritative input: bead `vb-l2d7` and current master-doc text.
- Domain terms:
  - `Taint`: lattice `Clean < DerivedFromSecret < Secret`.
  - `join_taint`: least upper bound over contributing taints.
  - joined-taint behavior: `EvalExpr`, `BuildObject`, and `BuildList` write output taint equal to the join of contributing input slot taints; `Finish` reads and emits result-slot taint in `EngineSignal::Finished(SlotValue, Taint)`.
  - stale Clean-only text: master-doc wording that says the resolved nodes are always `Clean` or have no join after DRIFT-1 is resolved.
  - implementation evidence: claims backed by verified source inspection, tests, CI, formal reports, or other concrete artifacts.
- Assumptions:
  - This bead may correct contradictions in the master doc but must not assert new implementation evidence beyond already named DRIFT-1 evidence unless downstream states verify it.
  - Existing DRIFT-1 lines may be cited as status evidence, but any stronger claims must be marked pending or omitted.
- Open questions:
  - None blocking State 1. Downstream states must verify exact source/test evidence before strengthening evidence wording.

## Preconditions
- PRE-001: The edit target is only `/home/lewis/src/vb-l2d7/velvet-ballistics-MASTER.md` in later states.
- PRE-002: State 1 artifacts are written only under `/home/lewis/src/vb-l2d7/.beads/vb-l2d7/`.
- PRE-003: The bead scope is documentation reconciliation, not runtime implementation or test implementation.
- PRE-004: The current master doc contains both resolved DRIFT-1 joined-taint statements and stale Clean-only statements for one or more resolved nodes.
- PRE-005: Any wording that claims implementation evidence must be traceable to concrete evidence already present in the repo or explicitly marked as requiring verification.

## Postconditions
- POST-001: The planned downstream doc change makes the master doc internally consistent: `EvalExpr`, `BuildObject`, `BuildList`, and `Finish` sections agree with the resolved joined-taint behavior.
- POST-002: No remaining master-doc statement says `EvalExpr`, `BuildObject`, or `BuildList` are always `Clean` or have no taint join after the reconciliation.
- POST-003: `Finish` is described as reading result-slot value and taint and emitting `EngineSignal::Finished(SlotValue, Taint)` without adding unverified runtime rejection claims.
- POST-004: DRIFT-1 status wording remains evidence-bounded: it may state resolved status only as far as cited evidence supports; unverified parity or release-readiness claims remain excluded or marked as gaps.
- POST-005: The reconciliation preserves explicit v1 non-goal wording for control-flow taint unless a separate bead changes that scope.

## Invariants
- INV-001: Documentation must not claim implementation evidence that has not been verified.
- INV-002: All taint semantics in the master doc must use one lattice vocabulary: `Clean < DerivedFromSecret < Secret` and joined propagation for resolved data-flow operations.
- INV-003: Data-flow taint and control-flow taint remain distinct; resolving joined data-flow taint must not imply v1 tracks branch-condition taint.
- INV-004: Documentation consistency is not proof of IR/generated semantic parity, release readiness, or full end-to-end verification.
- INV-005: Later code work, if any, must preserve repo safety rules with exact gates: workspace lint configuration must keep `unsafe_code = "forbid"`; clippy/rust lints must deny `unwrap_used`, `expect_used`, `panic`, `panic_in_result_fn`, `todo`, `unimplemented`, `dbg_macro`, `indexing_slicing`, `string_slice`, `get_unwrap`, `arithmetic_side_effects`, `as_conversions`, and `let_underscore_must_use`; `moon run :lint-src`, `moon run :check`, and `moon run :verify-standard` must pass; fallible paths must use `Result<T, Error>` with `?`, `map`, `and_then`, or typed error conversion rather than panic/unwrap/expect.

## Error Taxonomy
- Error::WrongWorkspace - artifact or downstream doc edit target is outside `/home/lewis/src/vb-l2d7`.
- Error::OutOfScopeChange - attempted production code, proof code, harness code, tests, bead status, commit, or push in State 1.
- Error::StaleCleanOnlyTaintText - post-reconciliation doc scan still finds stale Clean-only wording for resolved nodes.
- Error::UnsupportedEvidenceClaim - doc wording claims implementation/test/formal evidence that downstream verification has not established.
- Error::TaintVocabularyConflict - doc uses conflicting terms for the same lattice or propagation rule.
- Error::ControlFlowTaintConflation - doc wording implies v1 tracks control-flow taint as part of DRIFT-1.
- Error::MissingTraceability - a contract clause lacks mapped tests, verification layers, or proof obligations.

## Contract Signatures
- `fn plan_taint_doc_reconciliation(input: MasterDocSnapshot, evidence_policy: EvidencePolicy) -> Result<DocPatchPlan, DocReconcileError>`
- `fn scan_for_stale_clean_only_text(doc: MasterDocSnapshot) -> Result<ContradictionReport, DocReconcileError>`
- `fn validate_evidence_bounded_wording(doc: MasterDocSnapshot, evidence: EvidenceIndex) -> Result<EvidenceBoundedReport, DocReconcileError>`
- `fn validate_taint_vocabulary_consistency(doc: MasterDocSnapshot) -> Result<TaintVocabularyReport, DocReconcileError>`

## Lean-Owned Clauses
- INV-002: pure lattice join model for `Clean < DerivedFromSecret < Secret`.
- POST-001: abstract node taint model for `EvalExpr`, `BuildObject`, `BuildList`, and `Finish` data-flow propagation.
- INV-003: abstract separation of data-flow taint from control-flow taint.

## Executable Companion Evidence Required For Lean-Owned Clauses
- INV-002: downstream proof/test state must add or identify executable Rust evidence for lattice laws: `cargo test -p vb_core taint_join_laws` and, if a Kani harness is present or added, `KANI_REQUIRED=1 KANI_CMD='cargo kani -p vb_core --harness taint_join_laws' moon run :verify-proof`.
- POST-001: downstream proof/test state must add or identify executable Rust evidence that `EvalExpr`, `BuildObject`, `BuildList`, and `Finish` realization agrees with the abstract taint model: `cargo test -p vb_runtime joined_taint_propagation` or an equivalent named workspace test, plus `moon run :verify-standard`.
- INV-003: downstream proof/test state must add or identify executable artifact evidence that data-flow taint wording and v1 control-flow non-goal wording coexist without contradiction: `python scripts/check-doc-taint-consistency.py velvet-ballistics-MASTER.md` if such a script is added, or an equivalent checked-in doc consistency command, plus `moon run :verify-standard`.

## Non-goals
- No implementation changes.
- No tests or harnesses written in State 1.
- No claim that generated Rust and IR mode are semantically equivalent.
- No claim that DRIFT-1 has new evidence beyond what downstream states verify.
- No v2 control-flow taint design.

## Independent Review Requirement
Downstream work must not consume these artifacts until an independent reviewer writes `/home/lewis/src/vb-l2d7/.beads/vb-l2d7/contract-verification-review.md` with `STATUS: APPROVED`.
