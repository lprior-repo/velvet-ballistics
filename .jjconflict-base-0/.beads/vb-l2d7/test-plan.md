# Test Plan: vb-l2d7 — Reconcile Joined-Taint Documentation Drift

## Summary
- Behaviors identified: 22 contract behaviors plus 7 typed error behaviors.
- Trophy allocation: 24 unit / 15 integration / 1 e2e / 5 static gate groups. Unit density satisfies the 5x gate for 4 public contract signatures (`4 * 5 = 20` minimum) while integration/static scans remain the widest behavior layer for real document evidence.
- Proptest invariants: 5 required executable companion groups for taint lattice, patch-plan generation, evidence-claim combinations, abstract propagation, and seeded doc fragments.
- Fuzz targets: 1 active hostile-input Markdown/doc checker target: `check_doc_taint_consistency_accepts_arbitrary_markdown`.
- Kani harnesses: 2 required-if-present/added companion harness groups for taint lattice and finite resolved-node propagation.
- Lean companion checks: 3 theorem groups: `join_is_lub`, `resolved_nodes_use_joined_taint`, `data_flow_join_does_not_track_control_flow`.
- Mutation threshold: `cargo-mutants` must kill >=90% of relevant doc-scan/test mutations.
- Red phase expectation: tests/scans must fail before the reconciliation because `velvet-ballistics-MASTER.md` currently contains stale Clean-only wording at lines around 2550-2552 and conflicting Finish/evidence wording around 609/658.

## 1. Behavior Inventory

1. State 1 artifact writer confines writes to `/home/lewis/src/vb-l2d7/.beads/vb-l2d7/` when producing planning artifacts.
2. State 2 documentation edit confines target to `/home/lewis/src/vb-l2d7/velvet-ballistics-MASTER.md` when reconciling the master doc.
3. Scope guard rejects implementation, proof, harness, test, bead status, commit, or push changes when operating in State 1.
4. Pre-edit doc scan detects contradiction when master doc contains resolved DRIFT-1 joined-taint statements and stale Clean-only resolved-node statements.
5. Evidence audit requires every implementation-evidence claim to cite concrete evidence or be marked pending when wording references tests, CI, source inspection, formal reports, parity, or release readiness.
6. Reconciled master doc presents `EvalExpr` output taint as the join of contributing loaded slot taints when describing resolved DRIFT-1 semantics.
7. Reconciled master doc presents `BuildObject` output taint as the join of contributing field slot taints when describing resolved DRIFT-1 semantics.
8. Reconciled master doc presents `BuildList` output taint as the join of contributing item slot taints when describing resolved DRIFT-1 semantics.
9. Reconciled master doc presents `Finish` as reading result-slot value and taint and emitting `EngineSignal::Finished(SlotValue, Taint)` when describing terminal runtime signals.
10. Reconciled master doc excludes unverified runtime rejection claims when describing `Finish` taint behavior.
11. Reconciled master doc contains no statement that `EvalExpr`, `BuildObject`, or `BuildList` are always `Clean` after DRIFT-1 resolution.
12. Reconciled master doc contains no statement that `EvalExpr`, `BuildObject`, or `BuildList` have no taint join after DRIFT-1 resolution.
13. DRIFT-1 status wording remains bounded to cited evidence when mentioning resolved status.
14. DRIFT-1 status wording does not claim generated/IR parity, release readiness, or full end-to-end verification when those artifacts are not verified.
15. Master doc preserves explicit v1 non-goal wording for control-flow taint when data-flow joined taint is reconciled.
16. Master doc uses one taint lattice vocabulary `Clean < DerivedFromSecret < Secret` when describing resolved data-flow taint semantics.
17. Taint join wording preserves domination by the highest contributing taint when contributors include mixed lattice values.
18. Empty `BuildObject` field-list wording either states a verified join identity or avoids unsupported empty-join claims when object fields are empty.
19. Empty `BuildList` item-list wording either states a verified join identity or avoids unsupported empty-join claims when list items are empty.
20. Traceability review maps every PRE, POST, INV, ERR, theorem, gate, and waiver clause to tests, verification layer, proof obligation, and evidence artifact.
21. Waiver review verifies each waiver names clause IDs, waived layer, reason, compensating evidence, owner, and expiry/follow-up condition when a verification layer is waived.
22. Repo safety gate preserves exact lint/check requirements when later code changes are made.
23. Path review reports `Error::WrongWorkspace` when artifact or doc edit target is outside `/home/lewis/src/vb-l2d7`.
24. Diff scope review reports `Error::OutOfScopeChange` when State 1 includes production code, proof code, harness code, tests, bead status change, commit, or push.
25. Doc consistency scan reports `Error::StaleCleanOnlyTaintText` when stale Clean-only or no-join wording remains for resolved nodes.
26. Evidence audit reports `Error::UnsupportedEvidenceClaim` when wording claims unverified implementation/test/formal/parity/release evidence.
27. Vocabulary audit reports `Error::TaintVocabularyConflict` when contradictory lattice or propagation terms are used for the same taint rule.
28. Control-flow review reports `Error::ControlFlowTaintConflation` when data-flow joined-taint wording implies v1 tracks branch-condition taint.
29. Traceability review reports `Error::MissingTraceability` when any contract clause lacks mapped tests, verification layers, or proof obligations.

## 2. Trophy Allocation

| Behavior(s) | Layer | Tool/Technique | Rationale |
|---|---|---|---|
| 1-3, 23-24 | Integration/manual QA | `git diff --name-only`, path allowlist review, artifact tree review | Public behavior is repository-state effect, not pure logic; use real workspace paths. |
| 4, 11-12, 15-16, 25, 27-28 | Integration/static scan | `python scripts/check-doc-taint-consistency.py velvet-ballistics-MASTER.md` plus seeded fixture fragments | Main risk is doc text contradiction across sections; scan real doc and seeded fragments. |
| 5, 10, 13-14, 26 | Integration/manual evidence audit | `evidence-bounded-wording-report.md` review against concrete cited artifacts | Evidence claims require human-readable provenance, not private implementation checks. |
| 6-9 | Integration + proptest companion | real master doc scan; exact target `vb_runtime::taint::proptests::joined_taint_propagation`; command `cargo test -p vb_runtime joined_taint_propagation` | Documentation behavior must agree with runtime evidence boundary; use real runtime fixtures if implementation evidence is claimed. |
| 17 | Unit/proptest + Lean | `cargo test -p vb_core taint_join_laws`; Lean `Velvet.TaintLattice.join_is_lub` | Finite lattice laws are pure deterministic semantics. |
| 18-19 | Integration/manual QA + unit if join identity is implemented | doc review; optional pure join identity tests if empty join is specified | Empty contributor wording must either be evidence-backed or omitted. |
| 20 | Integration/JSONL validation | `jq -c . proof-obligations.jsonl`, `jq -c . traceability-matrix.jsonl`, clause coverage audit | Traceability spans artifact files and cannot be tested through a private method. |
| 21 | Integration/manual QA | waiver metadata checklist | Waiver validity is a documented process contract. |
| 22 | Static gates | `moon run :lint-src`, `moon run :check`, `moon run :verify-standard`, Cargo lint audit | Repo safety guarantees are enforced by compile/lint configuration and canonical Moon lanes. |
| Full workflow | E2E/acceptance | pre-scan -> reconcile doc -> post-scan -> evidence audit -> `moon run :verify-standard` | One black-box acceptance path proves the bead outcome with real inputs. |

### Unit Density Gate — 24 Named Unit Cases

These named unit cases are mandatory for the four public contract signatures in `contract.md:49-53`. They test pure report construction, validation decisions, and exact typed errors through public APIs only. Minimum required: 20. Planned: 24.

| # | Public contract signature | Unit test name | Input class | Exact expected result |
|---:|---|---|---|---|
| 1 | `plan_taint_doc_reconciliation` | `plan_taint_doc_reconciliation_returns_patch_plan_when_doc_has_stale_nodes_and_evidence_is_bounded` | valid master doc snapshot with stale resolved-node rows and bounded evidence policy | `Ok(DocPatchPlan { target: MasterDoc, edits: [EvalExprJoin, BuildObjectJoin, BuildListJoin, FinishCarriesTaint], evidence_actions: NoUnsupportedClaims })` |
| 2 | `plan_taint_doc_reconciliation` | `plan_taint_doc_reconciliation_returns_noop_plan_when_doc_is_already_consistent` | already reconciled snapshot | `Ok(DocPatchPlan { edits: [], contradiction_count: 0, status: AlreadyConsistent })` |
| 3 | `plan_taint_doc_reconciliation` | `plan_taint_doc_reconciliation_preserves_control_flow_non_goal_when_data_flow_is_reconciled` | stale data-flow text plus explicit v1 control-flow non-goal | `Ok(DocPatchPlan { preserved_sections: [ControlFlowTaintNonGoal], forbidden_edits: [], added_claims: [] })` |
| 4 | `plan_taint_doc_reconciliation` | `plan_taint_doc_reconciliation_returns_wrong_workspace_when_target_is_outside_vb_l2d7` | target path outside `/home/lewis/src/vb-l2d7` | `Err(DocReconcileError::WrongWorkspace { path })` |
| 5 | `plan_taint_doc_reconciliation` | `plan_taint_doc_reconciliation_returns_unsupported_evidence_claim_when_policy_forbids_uncited_claim` | doc snapshot with uncited implementation/test/formal/parity claim | `Err(DocReconcileError::UnsupportedEvidenceClaim { sentence, required: ConcreteArtifactOrPendingMarker })` |
| 6 | `plan_taint_doc_reconciliation` | `plan_taint_doc_reconciliation_returns_control_flow_conflation_when_patch_would_claim_v1_branch_taint` | candidate wording implies secret branch condition taints result in v1 | `Err(DocReconcileError::ControlFlowTaintConflation { sentence })` |
| 7 | `scan_for_stale_clean_only_text` | `scan_for_stale_clean_only_text_returns_empty_report_when_resolved_nodes_use_joined_taint` | reconciled normative rows | `Ok(ContradictionReport { stale_clean_only: [], no_join_claims: [] })` |
| 8 | `scan_for_stale_clean_only_text` | `scan_for_stale_clean_only_text_reports_eval_expr_always_clean` | `EvalExpr | Always Clean` fragment | `Err(DocReconcileError::StaleCleanOnlyTaintText { node: EvalExpr, phrase: "Always Clean" })` |
| 9 | `scan_for_stale_clean_only_text` | `scan_for_stale_clean_only_text_reports_eval_expr_no_operand_join` | `EvalExpr ... No taint join of expression operands` fragment | `Err(DocReconcileError::StaleCleanOnlyTaintText { node: EvalExpr, phrase: "No taint join" })` |
| 10 | `scan_for_stale_clean_only_text` | `scan_for_stale_clean_only_text_reports_build_object_always_clean` | `BuildObject | Always Clean` fragment | `Err(DocReconcileError::StaleCleanOnlyTaintText { node: BuildObject, phrase: "Always Clean" })` |
| 11 | `scan_for_stale_clean_only_text` | `scan_for_stale_clean_only_text_reports_build_object_no_field_join` | `BuildObject ... no join of field taints` fragment | `Err(DocReconcileError::StaleCleanOnlyTaintText { node: BuildObject, phrase: "no join of field taints" })` |
| 12 | `scan_for_stale_clean_only_text` | `scan_for_stale_clean_only_text_reports_build_list_always_clean` | `BuildList | Always Clean` fragment | `Err(DocReconcileError::StaleCleanOnlyTaintText { node: BuildList, phrase: "Always Clean" })` |
| 13 | `scan_for_stale_clean_only_text` | `scan_for_stale_clean_only_text_reports_build_list_no_item_join` | `BuildList ... no join of item taints` fragment | `Err(DocReconcileError::StaleCleanOnlyTaintText { node: BuildList, phrase: "no join of item taints" })` |
| 14 | `validate_evidence_bounded_wording` | `validate_evidence_bounded_wording_returns_report_when_all_claims_are_cited_or_pending` | claims with concrete artifact citations or `pending/unverified` markers | `Ok(EvidenceBoundedReport { unsupported_claims: [], cited_claims: n, pending_claims: m })` |
| 15 | `validate_evidence_bounded_wording` | `validate_evidence_bounded_wording_reports_uncited_test_claim` | `tests prove joined taint` without test artifact | `Err(DocReconcileError::UnsupportedEvidenceClaim { claim_kind: TestEvidence, sentence })` |
| 16 | `validate_evidence_bounded_wording` | `validate_evidence_bounded_wording_reports_uncited_formal_claim` | `Lean proves implementation parity` without formal artifact | `Err(DocReconcileError::UnsupportedEvidenceClaim { claim_kind: FormalEvidence, sentence })` |
| 17 | `validate_evidence_bounded_wording` | `validate_evidence_bounded_wording_reports_uncited_release_claim` | `release ready` wording without release artifact | `Err(DocReconcileError::UnsupportedEvidenceClaim { claim_kind: ReleaseReadiness, sentence })` |
| 18 | `validate_evidence_bounded_wording` | `validate_evidence_bounded_wording_reports_uncited_generated_parity_claim` | generated Rust/IR parity claim without evidence | `Err(DocReconcileError::UnsupportedEvidenceClaim { claim_kind: GeneratedParity, sentence })` |
| 19 | `validate_taint_vocabulary_consistency` | `validate_taint_vocabulary_consistency_returns_report_for_single_lattice` | only `Clean < DerivedFromSecret < Secret` and joined propagation terms | `Ok(TaintVocabularyReport { lattice: [Clean, DerivedFromSecret, Secret], conflicts: [] })` |
| 20 | `validate_taint_vocabulary_consistency` | `validate_taint_vocabulary_consistency_reports_wrong_order` | `Clean < Secret < DerivedFromSecret` | `Err(DocReconcileError::TaintVocabularyConflict { conflict: WrongOrder, sentence })` |
| 21 | `validate_taint_vocabulary_consistency` | `validate_taint_vocabulary_consistency_reports_unknown_taint_term` | `Private`, `Sensitive`, or non-contract lattice term in normative text | `Err(DocReconcileError::TaintVocabularyConflict { conflict: UnknownTerm, term })` |
| 22 | `validate_taint_vocabulary_consistency` | `validate_taint_vocabulary_consistency_reports_downgrade_wording` | `Secret downgrades to Clean` | `Err(DocReconcileError::TaintVocabularyConflict { conflict: Downgrade, sentence })` |
| 23 | `validate_taint_vocabulary_consistency` | `validate_taint_vocabulary_consistency_reports_control_flow_conflation` | joined data-flow wording says v1 tracks branch-condition taint | `Err(DocReconcileError::ControlFlowTaintConflation { sentence })` |
| 24 | `scan_for_stale_clean_only_text` | `scan_for_stale_clean_only_text_reports_write_slot_only_semantics_for_resolved_nodes` | `write_slot not write_slot_with_taint` stale phrase for resolved node | `Err(DocReconcileError::StaleCleanOnlyTaintText { node, phrase: "write_slot" })` |

## 3. BDD Scenarios

### Behavior: `plan_taint_doc_reconciliation` returns a complete evidence-bounded patch plan
Test function: `fn plan_taint_doc_reconciliation_returns_patch_plan_when_doc_has_stale_nodes_and_evidence_is_bounded()`

Given: `MasterDocSnapshot` contains resolved DRIFT-1 joined-taint evidence plus stale Clean-only/no-join text for `EvalExpr`, `BuildObject`, and `BuildList`, and `EvidencePolicy` requires concrete artifact citations or `pending/unverified` markers.
When: `plan_taint_doc_reconciliation(input, evidence_policy)` is called.
Then: it returns exactly `Ok(DocPatchPlan { target: MasterDoc, edits: [EvalExprJoin, BuildObjectJoin, BuildListJoin, FinishCarriesTaint], stale_text_removed: [EvalExprAlwaysClean, EvalExprNoOperandJoin, BuildObjectAlwaysClean, BuildObjectNoFieldJoin, BuildListAlwaysClean, BuildListNoItemJoin], evidence_actions: EvidenceBounded, preserved_non_goals: [ControlFlowTaintV1NonGoal] })`.
And: the plan contains no production-code, proof-code, harness-code, test-code, bead-status, commit, or push action.

Error variants:
Given: `MasterDocSnapshot.path` or output target is outside `/home/lewis/src/vb-l2d7`.
When: `plan_taint_doc_reconciliation(input, evidence_policy)` is called.
Then: it returns exactly `Err(DocReconcileError::WrongWorkspace { path })`.

Given: the planned action set includes production code, proof code, harness code, test code, bead status update, commit, or push.
When: `plan_taint_doc_reconciliation(input, evidence_policy)` is called.
Then: it returns exactly `Err(DocReconcileError::OutOfScopeChange { change_kind, path_or_operation })`.

Given: the candidate doc wording claims implementation/test/formal/generated-parity/release evidence without a concrete cited artifact and without a `pending/unverified` marker.
When: `plan_taint_doc_reconciliation(input, evidence_policy)` is called.
Then: it returns exactly `Err(DocReconcileError::UnsupportedEvidenceClaim { sentence, claim_kind, required: ConcreteArtifactOrPendingMarker })`.

Given: the candidate doc wording implies v1 tracks branch-condition/control-flow taint as part of DRIFT-1.
When: `plan_taint_doc_reconciliation(input, evidence_policy)` is called.
Then: it returns exactly `Err(DocReconcileError::ControlFlowTaintConflation { sentence })`.

### Behavior: `scan_for_stale_clean_only_text` detects all resolved-node stale wording
Test function: `fn scan_for_stale_clean_only_text_returns_report_when_no_stale_text_remains()`

Given: `MasterDocSnapshot` has normative taint sections where `EvalExpr`, `BuildObject`, and `BuildList` all use joined-taint wording and no stale Clean-only/no-join phrases.
When: `scan_for_stale_clean_only_text(doc)` is called.
Then: it returns exactly `Ok(ContradictionReport { stale_clean_only: [], no_join_claims: [], write_slot_only_claims: [], scanned_nodes: [EvalExpr, BuildObject, BuildList, Finish] })`.

Error variants:
Given: a normative `EvalExpr`, `BuildObject`, or `BuildList` fragment says `Always Clean`.
When: `scan_for_stale_clean_only_text(doc)` is called.
Then: it returns exactly `Err(DocReconcileError::StaleCleanOnlyTaintText { node, phrase: "Always Clean" })`.

Given: a normative `EvalExpr`, `BuildObject`, or `BuildList` fragment says `No taint join`, `no join of field taints`, or `no join of item taints`.
When: `scan_for_stale_clean_only_text(doc)` is called.
Then: it returns exactly `Err(DocReconcileError::StaleCleanOnlyTaintText { node, phrase })` with the matching phrase.

### Behavior: `validate_evidence_bounded_wording` accepts only cited or pending evidence claims
Test function: `fn validate_evidence_bounded_wording_returns_report_when_claims_are_cited_or_pending()`

Given: `MasterDocSnapshot` contains DRIFT-1 evidence sentences and `EvidenceIndex` maps every implementation/test/formal/source/CI claim to a concrete artifact or marks the claim `pending/unverified`.
When: `validate_evidence_bounded_wording(doc, evidence)` is called.
Then: it returns exactly `Ok(EvidenceBoundedReport { unsupported_claims: [], cited_claims: evidence.cited_count, pending_claims: evidence.pending_count, forbidden_claims: [] })`.

Error variant:
Given: a sentence claims implementation evidence, CI evidence, formal proof, generated/IR parity, or release readiness and `EvidenceIndex` has no artifact citation or pending marker for it.
When: `validate_evidence_bounded_wording(doc, evidence)` is called.
Then: it returns exactly `Err(DocReconcileError::UnsupportedEvidenceClaim { sentence, claim_kind, required: ConcreteArtifactOrPendingMarker })`.

### Behavior: `validate_taint_vocabulary_consistency` enforces one lattice and no control-flow conflation
Test function: `fn validate_taint_vocabulary_consistency_returns_report_when_lattice_terms_are_consistent()`

Given: `MasterDocSnapshot` uses only `Clean < DerivedFromSecret < Secret`, `join_taint`, `join`, `least upper bound`, and `joined taint` for resolved data-flow taint semantics.
When: `validate_taint_vocabulary_consistency(doc)` is called.
Then: it returns exactly `Ok(TaintVocabularyReport { lattice: [Clean, DerivedFromSecret, Secret], propagation_rule: JoinedDataFlowTaint, conflicts: [], control_flow_scope: ExplicitV1NonGoal })`.

Error variants:
Given: a normative taint section uses a contradictory order, unknown term, downgrade wording, or incompatible propagation term.
When: `validate_taint_vocabulary_consistency(doc)` is called.
Then: it returns exactly `Err(DocReconcileError::TaintVocabularyConflict { conflict, sentence_or_term })`.

Given: a normative taint section says joined data-flow taint means v1 tracks secret branch-condition taint.
When: `validate_taint_vocabulary_consistency(doc)` is called.
Then: it returns exactly `Err(DocReconcileError::ControlFlowTaintConflation { sentence })`.

### Behavior: State 1 artifacts stay inside bead directory
Test function: `fn state1_artifacts_remain_under_bead_directory_when_written()`

Given: a State 1 agent writes planning artifacts for bead `vb-l2d7`.
When: artifact paths are reviewed.
Then: the changed artifact paths are exactly under `/home/lewis/src/vb-l2d7/.beads/vb-l2d7/`, including `test-plan.md`.
And: no production source, proof source, test source, bead status file, commit, or push side effect is present.

### Behavior: Downstream doc edit targets only the master doc
Test function: `fn doc_edit_targets_master_doc_when_reconciling_taint_status()`

Given: a downstream documentation reconciliation patch for `vb-l2d7`.
When: `git diff --name-only` is reviewed.
Then: the only downstream content target is `/home/lewis/src/vb-l2d7/velvet-ballistics-MASTER.md`, except evidence reports explicitly required under `.beads/vb-l2d7/`.

Error variant:
Given: the patch edits `/tmp/velvet-ballistics-MASTER.md` or `/home/lewis/src/Velvet-ballistics/velvet-ballistics-MASTER.md` instead.
When: path review runs.
Then: `Err(DocReconcileError::WrongWorkspace)` is reported with the offending path.

### Behavior: Scope guard rejects State 1 out-of-scope changes
Test function: `fn scope_guard_reports_out_of_scope_change_when_state1_diff_touches_code_or_tests()`

Given: a State 1 diff includes any `crates/**`, `tests/**`, `benches/**`, Lean/proof file, bead status update, commit, or push side effect.
When: diff scope review runs.
Then: `Err(DocReconcileError::OutOfScopeChange)` is reported with the exact changed path or forbidden operation.

### Behavior: Pre-edit contradiction scan detects stale Clean-only text
Test function: `fn pre_edit_scan_detects_contradictory_taint_wording_when_clean_only_text_remains()`

Given: the current master doc contains resolved DRIFT-1 lines for joined taint and stale lines such as `EvalExpr Always Clean`, `BuildObject Always Clean`, `BuildList Always Clean`, or `No taint join`.
When: the pre-edit contradiction scan runs.
Then: the report contains each matching stale phrase with its line/section and classifies the document as requiring reconciliation.

### Behavior: Evidence audit bounds implementation claims
Test function: `fn evidence_audit_marks_claims_pending_when_no_concrete_artifact_exists()`

Given: a doc sentence claims source inspection, test coverage, CI success, formal proof, generated-mode parity, or release readiness.
When: the evidence audit compares the sentence to named artifacts.
Then: the report maps the sentence to a concrete artifact path/command or marks it `pending/unverified`.

Error variant:
Given: a sentence says `full generated Rust and IR parity is verified` without a cited artifact.
When: the evidence audit runs.
Then: `Err(DocReconcileError::UnsupportedEvidenceClaim)` is reported for that exact sentence.

### Behavior: EvalExpr doc uses joined input taint
Test function: `fn eval_expr_doc_describes_joined_operand_taint_when_reconciled()`

Given: the reconciled master doc.
When: normative node semantics, taint propagation table, and DRIFT-1 register sections are scanned.
Then: every normative `EvalExpr` taint statement says output taint equals the join of loaded/contributing slot operand taints.
And: no normative `EvalExpr` statement says output taint is always `Clean` or has no operand taint join.

### Behavior: BuildObject doc uses joined field taint
Test function: `fn build_object_doc_describes_joined_field_taint_when_reconciled()`

Given: the reconciled master doc.
When: normative `BuildObject` sections are scanned.
Then: every normative `BuildObject` taint statement says output taint equals the join of contributing field slot taints.
And: no normative `BuildObject` statement says output taint is always `Clean` or has no field taint join.

### Behavior: BuildList doc uses joined item taint
Test function: `fn build_list_doc_describes_joined_item_taint_when_reconciled()`

Given: the reconciled master doc.
When: normative `BuildList` sections are scanned.
Then: every normative `BuildList` taint statement says output taint equals the join of contributing item slot taints.
And: no normative `BuildList` statement says output taint is always `Clean` or has no item taint join.

### Behavior: Finish doc emits value and taint
Test function: `fn finish_doc_emits_slot_value_and_taint_when_reconciled()`

Given: the reconciled master doc.
When: `Finish`, `EngineSignal`, runtime signal, and taint propagation sections are reviewed.
Then: `Finish` is described as reading result-slot value and result-slot taint.
And: the terminal signal is exactly `EngineSignal::Finished(SlotValue, Taint)`.

### Behavior: Finish doc excludes unverified rejection claims
Test function: `fn finish_doc_excludes_unverified_rejection_claim_when_evidence_is_missing()`

Given: compile-time or runtime rejection wording for `Secret` finish results.
When: evidence audit cannot cite concrete validation evidence.
Then: the reconciled doc removes the claim or marks it as pending/gap instead of stating it as implemented behavior.

### Behavior: Stale Clean-only wording is absent after reconciliation
Test function: `fn doc_scan_finds_no_clean_only_text_when_reconciliation_is_complete()`

Given: the reconciled master doc.
When: the doc consistency scan searches normative taint sections.
Then: the report lists zero occurrences of stale resolved-node claims matching `Always Clean`, `always Clean`, `No taint join`, `no join of field taints`, `no join of item taints`, or `write_slot-only` semantics for `EvalExpr`, `BuildObject`, and `BuildList`.

Error variant:
Given: a seeded document fragment says `BuildList | Always Clean — no join of item taints`.
When: the doc consistency scan runs.
Then: `Err(DocReconcileError::StaleCleanOnlyTaintText)` is reported and identifies `BuildList` plus the stale phrase.

### Behavior: DRIFT-1 status remains evidence-bounded
Test function: `fn drift1_status_remains_evidence_bounded_when_doc_is_reconciled()`

Given: the reconciled DRIFT-1 section.
When: evidence audit reviews each resolved-status sentence.
Then: each sentence is limited to cited DRIFT-1 evidence, named tests, named source paths, or explicit pending/gap wording.
And: no sentence claims broader parity, release readiness, or full verification unless such evidence is cited.

### Behavior: v1 control-flow taint non-goal is preserved
Test function: `fn control_flow_taint_non_goal_remains_when_data_flow_taint_is_reconciled()`

Given: data-flow joined-taint wording has been corrected.
When: the v1 taint scope and control-flow taint sections are reviewed.
Then: the doc still states that v1 does not track control-flow taint.
And: secret branch conditions are not described as tainting public results under DRIFT-1.

Error variant:
Given: wording says `a Secret branch condition taints the returned Clean slot in v1`.
When: control-flow taint review runs.
Then: `Err(DocReconcileError::ControlFlowTaintConflation)` is reported for the exact sentence.

### Behavior: Taint vocabulary is consistent
Test function: `fn taint_vocabulary_uses_single_lattice_when_doc_fragments_are_checked()`

Given: normative taint sections in the reconciled master doc.
When: vocabulary audit runs.
Then: the only lattice ordering is `Clean < DerivedFromSecret < Secret`.
And: resolved data-flow operations use `join` and `joined taint` wording consistently.

Error variant:
Given: a section says `Private > Secret > Clean` or says `DerivedFromSecret downgrades to Clean`.
When: vocabulary audit runs.
Then: `Err(DocReconcileError::TaintVocabularyConflict)` is reported with the conflicting term/order.

### Behavior: Mixed contributor taints are dominated by highest taint
Test function: `fn joined_taint_is_secret_when_any_contributor_is_secret()`

Given: contributing input taints include `Clean`, `DerivedFromSecret`, and `Secret`.
When: lattice wording and executable companion taint-join tests are reviewed.
Then: the joined result is exactly `Secret`.

### Behavior: Empty BuildObject contributors are evidence-bounded
Test function: `fn build_object_empty_fields_do_not_overclaim_join_identity_when_unverified()`

Given: `BuildObject` has an empty field list.
When: `BuildObject` taint wording is reviewed.
Then: the doc either states a verified empty-join identity with citation or avoids making an unsupported identity claim.

### Behavior: Empty BuildList contributors are evidence-bounded
Test function: `fn build_list_empty_items_do_not_overclaim_join_identity_when_unverified()`

Given: `BuildList` has an empty item list.
When: `BuildList` taint wording is reviewed.
Then: the doc either states a verified empty-join identity with citation or avoids making an unsupported identity claim.

### Behavior: Traceability is complete
Test function: `fn traceability_review_reports_complete_mapping_when_all_contract_clauses_are_mapped()`

Given: `contract.md`, `verification-layers.md`, `proof-obligations.jsonl`, and `traceability-matrix.jsonl`.
When: traceability review runs.
Then: every PRE, POST, INV, ERR, theorem, gate, and waiver clause appears in the matrix with at least one test, proof obligation, tool/layer, evidence artifact, and review artifact.

Error variant:
Given: `INV-003` is absent from `traceability-matrix.jsonl` or has an empty `tests` list.
When: traceability review runs.
Then: `Err(DocReconcileError::MissingTraceability)` is reported for `INV-003`.

### Behavior: Waivers are complete and expire correctly
Test function: `fn waiver_review_accepts_waiver_when_metadata_and_expiry_are_complete()`

Given: a waiver for concurrency, Miri/cargo-careful, performance, assembly, API compatibility, release provenance, or Lean shell clauses.
When: waiver metadata review runs.
Then: the waiver names clause IDs, waived layer, reason, compensating evidence, owner, and expiry/follow-up condition.
And: the waiver states immediate revocation if downstream scope adds the waived surface.
And: no waiver is accepted for `check_doc_taint_consistency_accepts_arbitrary_markdown`; the Markdown/doc checker hostile-input boundary requires active fuzz coverage.

### Behavior: Repo safety gates hold for later code changes
Test function: `fn repo_safety_gates_pass_when_later_code_changes_are_made()`

Given: downstream scope expands to Rust code changes.
When: `moon run :lint-src`, `moon run :check`, `moon run :verify-standard`, and Cargo lint audit run.
Then: `unsafe_code = "forbid"` and denies for `unwrap_used`, `expect_used`, `panic`, `panic_in_result_fn`, `todo`, `unimplemented`, `dbg_macro`, `indexing_slicing`, `string_slice`, `get_unwrap`, `arithmetic_side_effects`, `as_conversions`, and `let_underscore_must_use` are enforced.
And: fallible paths use typed `Result<T, Error>` instead of panic/unwrap/expect.

### Behavior: Full reconciliation acceptance path succeeds
Test function: `fn full_reconciliation_pipeline_succeeds_when_doc_and_evidence_are_consistent()`

Given: the master doc has been reconciled and required evidence reports are present.
When: downstream acceptance runs pre/post doc scans, JSONL validation, evidence audit, waiver review, companion tests if evidence is claimed, and `moon run :verify-standard`.
Then: all scans produce zero stale contradictions, all evidence claims are cited or pending, JSONL files parse one object per line, required companion checks are named or executed, and the standard verification lane exits 0.

## 4. Proptest Invariants

### Proptest: `taint_join_laws`
Invariant: For all `a`, `b`, `c` in `{Clean, DerivedFromSecret, Secret}`, join is commutative, associative, idempotent, monotonic, and has `Clean` as identity if the implementation/doc model claims an identity.
Strategy: Generate exhaustive finite triples from the three taint values; also generate non-empty lists of taints for list join.
Expected exact properties:
- `join(a, b) == join(b, a)`.
- `join(join(a, b), c) == join(a, join(b, c))`.
- `join(a, a) == a`.
- `a <= join(a, b)` and `b <= join(a, b)` under `Clean < DerivedFromSecret < Secret`.
- `join(Clean, x) == x` when identity is in scope.
Anti-invariant: Any model where `Secret` joined with a lower taint returns `Clean` or `DerivedFromSecret` must fail with an exact counterexample.

### Proptest: `joined_taint_propagation`
Invariant: For every generated resolved-node fixture, `EvalExpr`, `BuildObject`, and `BuildList` output taint equals the join of explicitly contributing input slot taints, and `Finish` signal taint equals the result-slot taint.
Strategy: Generate finite slot maps with taints from `{Clean, DerivedFromSecret, Secret}`; generate node fixtures for `EvalExpr` with loaded operands, `BuildObject` with field slots, `BuildList` with item slots, and `Finish` with result slot. Include empty field/item collections only if the implementation has a specified identity.
Expected exact properties:
- `EvalExpr.output_taint == join(loaded_operand_taints)`.
- `BuildObject.output_taint == join(field_slot_taints)`.
- `BuildList.output_taint == join(item_slot_taints)`.
- `Finish.signal == EngineSignal::Finished(expected_value, result_slot_taint)`.
Anti-invariant: A generated fixture where any contributor is `Secret` but output taint is `Clean` must fail with the node kind and contributing slot set.

### Proptest: `plan_taint_doc_reconciliation_contract_properties`
Exact target name: `plan_taint_doc_reconciliation_contract_properties`.
Exact module path: `vb_doc::reconcile::proptests::plan_taint_doc_reconciliation_contract_properties`.
Exact command: `cargo test -p vb_doc plan_taint_doc_reconciliation_contract_properties`.
Invariant: For any generated `MasterDocSnapshot` and `EvidencePolicy`, `plan_taint_doc_reconciliation(input, evidence_policy)` returns a deterministic `DocPatchPlan` that removes exactly stale resolved-node contradictions, preserves explicit v1 control-flow non-goals, and fails with the exact contract error variant for the first violated precondition/evidence rule.
Strategy: Generate structured master-doc snapshots from independent dimensions:
- Workspace path class: valid `/home/lewis/src/vb-l2d7/velvet-ballistics-MASTER.md`; invalid outside workspace; invalid artifact path outside `.beads/vb-l2d7`.
- Scope action set: doc-only edit; production-code edit; proof-code edit; harness/test edit; bead status/commit/push operation.
- Node wording map for `EvalExpr`, `BuildObject`, `BuildList`, `Finish`: joined-taint wording; `Always Clean`; `No taint join`; `write_slot`-only; missing section; duplicate contradictory rows.
- Evidence policy: strict citations required; pending markers accepted; unsupported claims forbidden.
- Evidence claim set: none; cited source/test/formal claim; uncited implementation claim; uncited generated parity claim; uncited release-readiness claim.
- Control-flow scope wording: explicit v1 non-goal; absent non-goal; conflated v1 branch-condition taint claim.
Expected exact properties:
- Valid workspace + doc-only scope + stale resolved-node rows + bounded evidence returns `Ok(DocPatchPlan { target: MasterDoc, edits: [...] })` where edits contain `EvalExprJoin` iff generated `EvalExpr` wording is stale, `BuildObjectJoin` iff generated `BuildObject` wording is stale, `BuildListJoin` iff generated `BuildList` wording is stale, and `FinishCarriesTaint` iff `Finish` wording lacks `EngineSignal::Finished(SlotValue, Taint)`.
- Already consistent generated snapshots return `Ok(DocPatchPlan { edits: [], contradiction_count: 0, status: AlreadyConsistent })` and do not invent implementation evidence claims.
- Any invalid workspace path returns exactly `Err(DocReconcileError::WrongWorkspace { path })` and no patch plan.
- Any generated out-of-scope State 1 action returns exactly `Err(DocReconcileError::OutOfScopeChange { change_kind, path_or_operation })` and no patch plan.
- Any uncited generated evidence/parity/release claim under strict policy returns exactly `Err(DocReconcileError::UnsupportedEvidenceClaim { sentence, claim_kind, required: ConcreteArtifactOrPendingMarker })` and no patch plan.
- Any generated control-flow conflation returns exactly `Err(DocReconcileError::ControlFlowTaintConflation { sentence })` and no patch plan.
- For valid patch plans, every planned edit path is `/home/lewis/src/vb-l2d7/velvet-ballistics-MASTER.md`, every artifact path is under `/home/lewis/src/vb-l2d7/.beads/vb-l2d7/`, and `preserved_non_goals` contains `ControlFlowTaintV1NonGoal` when the generated input contained that non-goal.
Anti-invariant: A generated snapshot containing `EvalExpr | Always Clean — no taint join` and strict bounded evidence must never return `Ok(DocPatchPlan { edits: [] ... })`; it must either include `EvalExprJoin` in the patch plan or return a more specific contract error if a higher-priority precondition is also generated invalid.
Red/green command expectations:
- Red: `cargo test -p vb_doc plan_taint_doc_reconciliation_contract_properties` must fail when stale-node generators can produce `Ok` plans without matching edit actions, when invalid paths are accepted, or when unsupported evidence claims are silently planned.
- Green: `cargo test -p vb_doc plan_taint_doc_reconciliation_contract_properties` must pass with generated cases covering all path classes, all scope action classes, all resolved-node stale phrase classes, all evidence-claim classes, and control-flow conflation.

### Proptest: `validate_evidence_bounded_wording_claim_combinations`
Exact target name: `validate_evidence_bounded_wording_claim_combinations`.
Exact module path: `vb_doc::evidence::proptests::validate_evidence_bounded_wording_claim_combinations`.
Exact command: `cargo test -p vb_doc validate_evidence_bounded_wording_claim_combinations`.
Invariant: For any generated mixture of clean-only wording, tainted/joined wording, and evidence claim combinations, `validate_evidence_bounded_wording(doc, evidence)` accepts only claims that are concretely cited or explicitly pending/unverified, and returns the exact unsupported-claim variant for every uncited implementation/test/formal/generated-parity/release claim.
Strategy: Generate document fragments as lists of independently generated sentences with labels:
- Taint wording class: stale clean-only (`Always Clean`, `No taint join`, `write_slot`-only), valid tainted/joined wording (`join_taint`, `least upper bound`, `EngineSignal::Finished(SlotValue, Taint)`), neutral non-evidence text, explicit v1 control-flow non-goal, control-flow conflation text.
- Evidence claim kind: source inspection, implementation behavior, unit/integration test, CI/Moon lane, Lean/formal proof, Kani/proof harness, generated Rust/IR parity, release readiness, performance/API/SBOM claim.
- Evidence support state: concrete artifact citation present in `EvidenceIndex`; explicit `pending/unverified/gap` marker; missing evidence; citation points to unrelated artifact; duplicate conflicting evidence states.
- Claim strength: bounded (`may`, `pending`, `identified as gap`), asserted (`is verified`, `passes`, `proves`, `release-ready`), or negated non-claim.
Expected exact properties:
- Fragments with only neutral text, valid joined-taint wording, stale clean-only wording that makes no implementation-evidence claim, and evidence claims that are either cited or pending return `Ok(EvidenceBoundedReport { unsupported_claims: [], cited_claims: n, pending_claims: m, forbidden_claims: [] })`.
- Stale clean-only wording alone does not produce `UnsupportedEvidenceClaim`; it remains the responsibility of `scan_for_stale_clean_only_text` to return `DocReconcileError::StaleCleanOnlyTaintText`.
- Valid tainted/joined wording with cited artifacts returns `Ok(EvidenceBoundedReport { unsupported_claims: [], cited_claims: n, pending_claims: 0, forbidden_claims: [] })`.
- Valid tainted/joined wording with explicit `pending/unverified/gap` markers returns `Ok(EvidenceBoundedReport { unsupported_claims: [], cited_claims: 0, pending_claims: m, forbidden_claims: [] })`.
- Any generated uncited asserted implementation/source/test/CI/formal/Kani/generated-parity/release/performance/API/SBOM claim returns exactly `Err(DocReconcileError::UnsupportedEvidenceClaim { sentence, claim_kind, required: ConcreteArtifactOrPendingMarker })` where `claim_kind` matches the generated claim label.
- A generated unrelated citation is treated as missing evidence and returns exactly `Err(DocReconcileError::UnsupportedEvidenceClaim { sentence, claim_kind, required: ConcreteArtifactOrPendingMarker })`.
- Control-flow conflation wording that also makes an evidence claim must not be accepted; evidence validation returns `UnsupportedEvidenceClaim` for uncited evidence assertions, while the vocabulary/control-flow validator is separately required to return `ControlFlowTaintConflation` for the same generated sentence.
Anti-invariant: A generated sentence `DRIFT-1 generated Rust and IR parity is verified` with no matching artifact in `EvidenceIndex` must never return `Ok(EvidenceBoundedReport { unsupported_claims: [] ... })`.
Red/green command expectations:
- Red: `cargo test -p vb_doc validate_evidence_bounded_wording_claim_combinations` must fail if any uncited asserted evidence claim is accepted, if stale clean-only wording is misclassified as `UnsupportedEvidenceClaim`, or if unrelated citations count as concrete evidence.
- Green: `cargo test -p vb_doc validate_evidence_bounded_wording_claim_combinations` must pass with generated combinations spanning clean-only wording, tainted/joined wording, neutral wording, every evidence claim kind, every support state, and bounded vs asserted claim strength.

### Proptest: `doc_taint_consistency_seeded_fragments`
Invariant: For generated normative doc fragments, stale Clean-only/no-join phrases for resolved nodes are rejected, while joined-taint phrases using the approved vocabulary are accepted.
Strategy: Generate small Markdown table rows/paragraphs containing one node name from `{EvalExpr, BuildObject, BuildList, Finish}` and one taint phrase class from `{joined, always_clean, no_join, conflicting_vocab, control_flow_conflation}`.
Expected exact properties:
- Joined phrases for resolved data-flow nodes produce an empty contradiction list.
- `always_clean` or `no_join` phrases for `EvalExpr`, `BuildObject`, or `BuildList` produce `DocReconcileError::StaleCleanOnlyTaintText`.
- Conflicting vocabulary phrases produce `DocReconcileError::TaintVocabularyConflict`.
- Control-flow conflation phrases produce `DocReconcileError::ControlFlowTaintConflation`.
Anti-invariant: A fragment containing `EvalExpr | Always Clean — no taint join` must never be classified as acceptable.

## 5. Fuzz Targets

Active fuzzing is mandatory for one selected hostile-input Markdown/doc boundary: target `check_doc_taint_consistency_accepts_arbitrary_markdown`, path `fuzz/fuzz_targets/check_doc_taint_consistency_accepts_arbitrary_markdown.rs`, command `cargo fuzz run check_doc_taint_consistency_accepts_arbitrary_markdown`. The target must consume arbitrary Markdown/doc text and enforce the exact pass/error properties listed in this section.

### Fuzz Target: `check_doc_taint_consistency_accepts_arbitrary_markdown`
Exact target path: `fuzz/fuzz_targets/check_doc_taint_consistency_accepts_arbitrary_markdown.rs`.
Exact command: `cargo fuzz run check_doc_taint_consistency_accepts_arbitrary_markdown`.
Input type: arbitrary UTF-8/bytes converted lossily to document text.
Exact input strategy: generate arbitrary `&[u8]`, convert with `String::from_utf8_lossy`, and pass the resulting document text to the public doc consistency checker used by `scripts/check-doc-taint-consistency.py velvet-ballistics-MASTER.md`. The harness must also inject seeded normative Markdown rows for resolved nodes so random prefixes/suffixes, Unicode dash variants, malformed tables, fenced code blocks, and very long lines surround known stale and valid phrases.
Risk: panic, catastrophic regex backtracking, unbounded memory/time growth, false acceptance of stale Clean-only/no-join phrases, false rejection of valid joined-taint wording, Unicode dash/case/spacing bypasses.
Required pass/error properties:
- No panic for arbitrary byte input, malformed UTF-8 converted lossily, empty input, very long lines, repeated Markdown tables, or nested code fences.
- No OOM or catastrophic regex behavior; bounded execution time must be enforced in the fuzz runner.
- Input containing normative-looking `EvalExpr`, `BuildObject`, or `BuildList` stale Clean-only/no-join phrases must return `Err(DocReconcileError::StaleCleanOnlyTaintText { node, phrase })`.
- Input containing `write_slot`-only semantics for resolved nodes paired with `not write_slot_with_taint` must return `Err(DocReconcileError::StaleCleanOnlyTaintText { node, phrase: "write_slot" })`.
- Input containing conflicting lattice vocabulary must return `Err(DocReconcileError::TaintVocabularyConflict { .. })`.
- Input containing v1 control-flow taint conflation must return `Err(DocReconcileError::ControlFlowTaintConflation { .. })`.
- Input containing valid joined-taint wording for resolved nodes must return a report with zero stale contradictions; Unicode dash variants, extra whitespace, or Markdown table formatting must not change that expected pass outcome.
Corpus seeds:
- `| EvalExpr | Always Clean — no taint join of expression operands. |`
- `| BuildObject | Always Clean — no join of field taints |`
- `| BuildList | Always Clean — no join of item taints |`
- `Finish emits EngineSignal::Finished(SlotValue, Taint)`
- `EvalExpr writes with write_slot (not write_slot_with_taint)`
- `Clean < Secret < DerivedFromSecret`
- `v1 joined taint tracks secret branch-condition taint`
- valid row: `| EvalExpr | output taint is join_taint over loaded slot taints |`
- valid row: `| BuildObject | output taint is the join of field slot taints |`
- valid row: `| BuildList | output taint is the join of item slot taints |`
- malformed Markdown tables, empty documents, very long lines, mixed Unicode dashes, repeated node names, fenced code blocks containing stale phrases, and normative headings containing stale phrases.
Required evidence command: `cargo fuzz run check_doc_taint_consistency_accepts_arbitrary_markdown`. The downstream evidence report must name `fuzz/fuzz_targets/check_doc_taint_consistency_accepts_arbitrary_markdown.rs` and attach the fuzz run result to `doc-consistency-scan.md` or `formal-verification-report.md`.

## 6. Kani Harnesses

### Kani Harness: `taint_join_laws`
Property: For all finite taint pairs/triples, join is least upper bound under `Clean < DerivedFromSecret < Secret`; no join result is below either operand.
Bound: exhaustive over three enum values and triples; no heap or loop bound beyond finite list length 3 unless implementation uses list join.
Rationale: The lattice is small enough for complete bounded proof and is Lean-owned by `Velvet.TaintLattice.join_is_lub`.
Command: `KANI_REQUIRED=1 KANI_CMD='cargo kani -p vb_core --harness taint_join_laws' moon run :verify-proof` when harness exists or is added.

### Kani Harness: `resolved_node_taint_propagation_finite_slots`
Property: For bounded slot arrays and resolved node kinds, computed output/signal taint equals the join of contributing slot taints; no stale `Clean` default is reachable when contributors include `DerivedFromSecret` or `Secret`.
Bound: maximum 4 slots, maximum 4 contributors per node, taint enum of 3 values, node kind enum restricted to `EvalExpr`, `BuildObject`, `BuildList`, `Finish`.
Rationale: This proves the finite state/slot propagation skeleton that backs Lean theorem `Velvet.TaintPropagation.resolved_nodes_use_joined_taint` if executable runtime evidence is added.
Command: `KANI_REQUIRED=1 KANI_CMD='cargo kani -p vb_runtime --harness resolved_node_taint_propagation_finite_slots' moon run :verify-proof` if such harness exists or downstream implementation adds it.

## 7. Mutation Testing Checkpoints

Threshold: mutation suite must kill >=90% of relevant mutations. Surviving mutations require either a new test/scenario or a documented non-applicability waiver.

Critical mutations and required catching tests:

- Replace `join` with constant `Clean` for `EvalExpr` -> caught by `eval_expr_doc_describes_joined_operand_taint_when_reconciled` and `joined_taint_propagation`.
- Replace `join` with first operand only for `BuildObject` -> caught by `build_object_doc_describes_joined_field_taint_when_reconciled` and mixed contributor proptest where later field is `Secret`.
- Replace `join` with last operand only for `BuildList` -> caught by `build_list_doc_describes_joined_item_taint_when_reconciled` and mixed contributor proptest where earlier item is `Secret`.
- Drop `Taint` from `EngineSignal::Finished(SlotValue, Taint)` wording -> caught by `finish_doc_emits_slot_value_and_taint_when_reconciled`.
- Keep `Compile-time validation rejects Secret finish results` as unconditional implementation fact without evidence -> caught by `finish_doc_excludes_unverified_rejection_claim_when_evidence_is_missing` and `evidence_audit_marks_claims_pending_when_no_concrete_artifact_exists`.
- Remove stale-phrase scan for `Always Clean` -> caught by `doc_scan_finds_no_clean_only_text_when_reconciliation_is_complete` seeded fragment test.
- Remove stale-phrase scan for `No taint join` -> caught by `pre_edit_scan_detects_contradictory_taint_wording_when_clean_only_text_remains`.
- Remove node name `BuildObject` or `BuildList` from scan allowlist -> caught by node-specific BDD scenarios.
- Change lattice order to `Clean < Secret < DerivedFromSecret` -> caught by `taint_vocabulary_uses_single_lattice_when_doc_fragments_are_checked` and `taint_join_laws`.
- Treat control-flow taint as included in DRIFT-1 -> caught by `control_flow_taint_non_goal_remains_when_data_flow_taint_is_reconciled`.
- Delete evidence-bounded wording audit -> caught by `drift1_status_remains_evidence_bounded_when_doc_is_reconciled`.
- Delete traceability mapping for any clause -> caught by `traceability_review_reports_complete_mapping_when_all_contract_clauses_are_mapped`.
- Delete waiver expiry metadata -> caught by `waiver_review_accepts_waiver_when_metadata_and_expiry_are_complete`.
- Permit paths outside `/home/lewis/src/vb-l2d7` -> caught by `doc_edit_targets_master_doc_when_reconciling_taint_status` WrongWorkspace variant.
- Permit State 1 code/test/proof changes -> caught by `scope_guard_reports_out_of_scope_change_when_state1_diff_touches_code_or_tests`.

Recommended commands after tests are implemented:
- `cargo mutants --package vb_core --filter taint` for lattice code if touched.
- `cargo mutants --package vb_runtime --filter taint` for runtime companion code if touched.
- Seeded mutation mode for doc scanner fixtures, to prove stale phrase changes are killed.
- `moon run :verify-deep` if mutation/coverage is integrated into the Moon lane.

## 8. Combinatorial Coverage Matrix

| Scenario | Input Class | Expected Output | Layer |
|---|---|---|---|
| State 1 artifact path happy path | only `.beads/vb-l2d7/test-plan.md` changes | changed paths all start with `/home/lewis/src/vb-l2d7/.beads/vb-l2d7/` | integration/manual |
| Wrong workspace | artifact or doc edit under `/tmp` or source repo path | `Err(DocReconcileError::WrongWorkspace)` with offending path | integration/manual |
| State 1 out-of-scope code | `crates/**` or `tests/**` changed in State 1 | `Err(DocReconcileError::OutOfScopeChange)` with changed path | integration/manual |
| Pre-edit contradiction | current master doc with resolved DRIFT-1 plus stale Clean-only rows | contradiction report includes all stale resolved-node phrases | integration/static |
| EvalExpr happy path | doc row says joined loaded operand taints | accepted; normalized rule equals `EvalExpr -> join(loaded_operand_taints)` | integration/static |
| EvalExpr stale text | doc row says `Always Clean` or `No taint join` | `Err(DocReconcileError::StaleCleanOnlyTaintText)` naming `EvalExpr` | unit/static fixture |
| BuildObject happy path | doc row says joined field slot taints | accepted; normalized rule equals `BuildObject -> join(field_slot_taints)` | integration/static |
| BuildObject stale text | doc row says `Always Clean — no join of field taints` | `Err(DocReconcileError::StaleCleanOnlyTaintText)` naming `BuildObject` | unit/static fixture |
| BuildList happy path | doc row says joined item slot taints | accepted; normalized rule equals `BuildList -> join(item_slot_taints)` | integration/static |
| BuildList stale text | doc row says `Always Clean — no join of item taints` | `Err(DocReconcileError::StaleCleanOnlyTaintText)` naming `BuildList` | unit/static fixture |
| Finish happy path | doc says reads result slot value and taint | exact phrase/model includes `EngineSignal::Finished(SlotValue, Taint)` | integration/static/manual |
| Finish unverified rejection claim | doc says runtime/compile-time rejects Secret without evidence | `Err(DocReconcileError::UnsupportedEvidenceClaim)` or pending/gap marker required | integration/manual |
| Evidence supported claim | claim cites exact source/test/report artifact | evidence report maps sentence to cited artifact | integration/manual |
| Evidence unsupported claim | claim lacks artifact | `Err(DocReconcileError::UnsupportedEvidenceClaim)` with sentence | integration/manual |
| Lattice order happy path | `Clean < DerivedFromSecret < Secret` | accepted; lattice order normalized exactly | unit/static fixture + Lean |
| Lattice conflict | contradictory order or downgrade wording | `Err(DocReconcileError::TaintVocabularyConflict)` | unit/static fixture |
| Control-flow non-goal happy path | doc says v1 does not track control-flow taint | accepted; non-goal section remains present | integration/static/manual |
| Control-flow conflation | doc says secret branch taints result in v1 | `Err(DocReconcileError::ControlFlowTaintConflation)` | integration/manual |
| Empty object fields | no fields | either verified identity cited or no identity claim made | integration/manual |
| Empty list items | no items | either verified identity cited or no identity claim made | integration/manual |
| Taint join Clean/Clean | `Clean`, `Clean` | joined result exactly `Clean` | unit/proptest |
| Taint join Clean/Derived | `Clean`, `DerivedFromSecret` | joined result exactly `DerivedFromSecret` | unit/proptest |
| Taint join Clean/Secret | `Clean`, `Secret` | joined result exactly `Secret` | unit/proptest |
| Taint join Derived/Derived | `DerivedFromSecret`, `DerivedFromSecret` | joined result exactly `DerivedFromSecret` | unit/proptest |
| Taint join Derived/Secret | `DerivedFromSecret`, `Secret` | joined result exactly `Secret` | unit/proptest |
| Taint join Secret/Secret | `Secret`, `Secret` | joined result exactly `Secret` | unit/proptest |
| Traceability complete | all clauses mapped | report lists zero missing mappings | integration/JSONL/manual |
| Traceability missing | any clause absent from matrix | `Err(DocReconcileError::MissingTraceability)` with clause ID | integration/JSONL/manual |
| Waiver complete | waiver has clause IDs/layer/reason/evidence/owner/expiry | waiver accepted with expiry condition | integration/manual |
| Waiver incomplete | waiver lacks owner or expiry | review rejects waiver with missing field list | integration/manual |
| Later code safety | Rust source touched | `moon run :lint-src`, `moon run :check`, `moon run :verify-standard` exit 0 and Cargo lint audit matches contract | static gate |
| Full E2E acceptance | reconciled doc and reports | zero stale contradictions; evidence bounded; JSONL valid; Moon standard lane exits 0 | e2e |

## 9. Doc Consistency Scan Plan

Required scan inputs:
- Real document: `/home/lewis/src/vb-l2d7/velvet-ballistics-MASTER.md`.
- Seeded stale fragments for `EvalExpr`, `BuildObject`, `BuildList`, and `Finish`.
- Seeded evidence-overclaim fragments.
- Seeded vocabulary conflict fragments.
- Seeded control-flow conflation fragments.

Required scan classes and exact expectations:
- Stale Clean-only phrases: produce `DocReconcileError::StaleCleanOnlyTaintText` with node name and phrase.
- Joined-taint phrases: accepted only if they use `join`/least-upper-bound semantics for contributing slot taints.
- Finish signal phrases: accepted only if they include `EngineSignal::Finished(SlotValue, Taint)` and do not add unverified rejection claims.
- Evidence claims: accepted only with concrete artifact citation or explicit `pending/unverified/gap` wording.
- Vocabulary: accepted only with `Clean < DerivedFromSecret < Secret`.
- Control-flow taint: accepted only when v1 non-goal remains explicit.

## 10. Lean and Proof Companion Checks

Lean-owned clauses:
- `INV-002` -> `Velvet.TaintLattice.join_is_lub`.
- `POST-001` -> `Velvet.TaintPropagation.resolved_nodes_use_joined_taint`.
- `INV-003` -> `Velvet.TaintPropagation.data_flow_join_does_not_track_control_flow`.

Required proof companion expectations:
- Lean may prove only the finite abstract model and documentation abstraction relation.
- Lean must not claim source conformance, generated Rust parity, CI status, runtime journal behavior, or release readiness.
- Every Lean-owned clause must have executable companion evidence named in the downstream verification report:
  - `cargo test -p vb_core taint_join_laws` for `INV-002`.
  - Exact target `vb_runtime::taint::proptests::joined_taint_propagation`; command `cargo test -p vb_runtime joined_taint_propagation` for `POST-001`.
  - `python scripts/check-doc-taint-consistency.py velvet-ballistics-MASTER.md` for `INV-003`.
  - `moon run :verify-proof` only when Lean/Kani artifacts exist or are added.

## 11. Commands and Red-Phase Expectations

### Planning/artifact validation commands
- `test -s /home/lewis/src/vb-l2d7/.beads/vb-l2d7/test-plan.md`
- `jq -c . /home/lewis/src/vb-l2d7/.beads/vb-l2d7/proof-obligations.jsonl >/dev/null`
- `jq -c . /home/lewis/src/vb-l2d7/.beads/vb-l2d7/traceability-matrix.jsonl >/dev/null`

### Downstream red phase commands
- `python scripts/check-doc-taint-consistency.py velvet-ballistics-MASTER.md` must fail before reconciliation because current master doc has stale Clean-only/no-join rows for `EvalExpr`, `BuildObject`, and `BuildList`.
- `cargo fuzz run check_doc_taint_consistency_accepts_arbitrary_markdown` using `fuzz/fuzz_targets/check_doc_taint_consistency_accepts_arbitrary_markdown.rs` must find the seeded stale Clean-only/no-join fragments before the checker accepts green evidence; any panic, OOM, catastrophic-regex case, or false acceptance of stale normative phrases is a red failure.
- Evidence audit must fail or mark pending before reconciliation for unconditional claims not backed by concrete artifacts, especially any `Finish` rejection or broad parity/release-readiness wording.
- `cargo test -p vb_core taint_join_laws` must fail/not exist before the test state if companion tests have not been written yet; after test implementation, it must fail red against any incorrect join implementation.
- `cargo test -p vb_runtime joined_taint_propagation` must fail/not exist before the test state if companion tests have not been written yet; after test implementation, it must fail red against stale Clean-only propagation.

### Downstream green/acceptance commands
- `python scripts/check-doc-taint-consistency.py velvet-ballistics-MASTER.md`
- `cargo fuzz run check_doc_taint_consistency_accepts_arbitrary_markdown` using `fuzz/fuzz_targets/check_doc_taint_consistency_accepts_arbitrary_markdown.rs`; passing evidence means no panic/OOM/catastrophic regex and exact typed errors for stale Clean-only/no-join, write-slot-only, vocabulary-conflict, and control-flow-conflation generated inputs
- `cargo test -p vb_core taint_join_laws`
- `cargo test -p vb_runtime joined_taint_propagation` for exact target `vb_runtime::taint::proptests::joined_taint_propagation`; passing evidence means `EvalExpr`, `BuildObject`, and `BuildList` output taint equals joined contributor taint and `Finish` emits `EngineSignal::Finished(SlotValue, Taint)` with result-slot taint
- `KANI_REQUIRED=1 KANI_CMD='cargo kani -p vb_core --harness taint_join_laws' moon run :verify-proof` if harness exists or is added
- `moon run :lint-src` if code changes are made
- `moon run :check` if code changes are made
- `moon run :verify-standard` after downstream doc/test work adds executable checks
- `moon run :verify-proof` if Lean/Kani proof artifacts are added

## Open Questions

- None blocking this test plan. Downstream implementation must decide whether empty `BuildObject`/`BuildList` joins have a verified identity; until verified, documentation must avoid claiming the identity.
