# Martin Fowler Test Plan: vb-l2d7

## Happy Path Tests
- `given_resolved_taint_nodes_when_master_doc_is_scanned_then_all_sections_agree_on_joined_taint`
  - Given: the master doc has been reconciled after DRIFT-1.
  - When: normative node semantics, taint lattice, and drift register sections are scanned.
  - Then: `EvalExpr`, `BuildObject`, `BuildList`, and `Finish` all describe resolved joined-taint behavior consistently.
- `given_finish_semantics_when_doc_reviewed_then_finished_signal_carries_value_and_taint_without_unverified_rejection_claim`
  - Given: the `Finish` section has been updated.
  - When: a reviewer compares it to the resolved DRIFT-1 contract.
  - Then: the doc says `Finish` reads result-slot taint and emits `EngineSignal::Finished(SlotValue, Taint)` without claiming unverified runtime rejection behavior.
- `given_reconciled_taint_docs_when_control_flow_section_reviewed_then_v1_non_goal_is_preserved`
  - Given: data-flow taint wording has been corrected.
  - When: the control-flow taint section is reviewed.
  - Then: the doc still states that v1 does not track control-flow taint.

## Error Path Tests
- `given_seeded_stale_clean_only_sentence_when_scan_runs_then_stale_clean_only_taint_text_is_reported`
  - Given: a document fragment says `EvalExpr` is always `Clean` or has no operand taint join.
  - When: the doc consistency scan runs.
  - Then: `Error::StaleCleanOnlyTaintText` is reported.
- `given_unsupported_implementation_claim_when_evidence_audit_runs_then_unsupported_evidence_claim_is_reported`
  - Given: a document sentence claims tests, CI, generated parity, or release readiness without evidence.
  - When: the evidence audit runs.
  - Then: `Error::UnsupportedEvidenceClaim` is reported.
- `given_data_flow_reconciliation_wording_when_control_flow_review_runs_then_control_flow_taint_conflation_is_reported_if_implied`
  - Given: doc wording implies secret branch conditions taint public results in v1.
  - When: the control-flow taint review runs.
  - Then: `Error::ControlFlowTaintConflation` is reported.
- `given_state1_out_of_scope_diff_when_reviewed_then_out_of_scope_change_is_reported`
  - Given: a State 1 diff includes production code, tests, proof code, bead status changes, commits, or pushes.
  - When: scope review runs.
  - Then: `Error::OutOfScopeChange` is reported.

## Edge Case Tests
- `given_empty_field_object_when_doc_reviewed_then_join_identity_is_clear_or_not_overclaimed`
  - Given: object fields may be empty.
  - When: BuildObject taint wording is reviewed.
  - Then: the doc either states the empty join identity or avoids unsupported edge-case claims.
- `given_empty_item_list_when_doc_reviewed_then_join_identity_is_clear_or_not_overclaimed`
  - Given: list items may be empty.
  - When: BuildList taint wording is reviewed.
  - Then: the doc either states the empty join identity or avoids unsupported edge-case claims.
- `given_multiple_contributor_taints_when_doc_reviewed_then_secret_dominates_joined_result`
  - Given: contributors include `Clean`, `DerivedFromSecret`, and `Secret`.
  - When: the lattice wording is reviewed.
  - Then: the highest taint dominates according to `Clean < DerivedFromSecret < Secret`.

## Contract Verification Tests
- `given_state1_workspace_when_artifacts_are_written_then_all_paths_are_under_vb_l2d7`
- `given_state1_artifacts_when_written_then_only_bead_artifact_directory_changes`
- `given_current_master_doc_when_scanned_then_contradictory_taint_wording_is_detected_before_edit`
- `given_doc_evidence_claims_when_reviewed_then_unverified_claims_are_removed_or_marked_pending`
- `given_taint_vocabulary_when_doc_fragments_are_checked_then_single_lattice_vocabulary_is_used`
- `given_doc_consistency_change_when_overclaim_scan_runs_then_no_parity_or_release_readiness_claim_is_added`
- `taint_join_laws`
  - Given: generated or enumerated taint triples over `Clean`, `DerivedFromSecret`, and `Secret`.
  - When: downstream executable Rust companion evidence evaluates join identity, commutativity, associativity, idempotence, and monotonicity.
  - Then: the implementation agrees with the Lean lattice model.
- `joined_taint_propagation`
  - Given: workflows or runtime fixtures that make `EvalExpr`, `BuildObject`, `BuildList`, and `Finish` consume mixed taint inputs.
  - When: downstream executable Rust companion evidence runs.
  - Then: each resolved node realizes the abstract joined-taint rule or reports a typed error instead of silently producing stale Clean-only taint.
- `given_workspace_lint_config_when_audited_then_unsafe_index_arithmetic_cast_and_panic_paths_are_denied`
  - Given: later code work touches Rust source.
  - When: downstream agents run `moon run :lint-src`, `moon run :check`, `moon run :verify-standard`, and audit `Cargo.toml` workspace lints.
  - Then: `unsafe_code`, unchecked indexing/slicing, unchecked arithmetic, unchecked casts, unwrap/expect/panic/todo/unimplemented/dbg, ignored must-use results, and non-railway fallible paths remain denied.
- `given_waiver_when_reviewed_then_owner_expiry_clause_ids_and_compensating_evidence_are_present`
  - Given: a verification layer is waived.
  - When: independent review reads waiver text.
  - Then: the waiver names clause IDs, waived layer, reason, compensating evidence, owner, and expiry/follow-up.

## Given/When/Then Scenarios

### Scenario 1: Reconcile resolved node semantics
Given: DRIFT-1 is documented as resolved for `EvalExpr`, `BuildObject`, `BuildList`, and `Finish`.
When: downstream documentation reconciliation updates the normative node semantics and propagation table.
Then:
- `EvalExpr` output taint is described as the join of loaded slot operands.
- `BuildObject` output taint is described as the join of field slot taints.
- `BuildList` output taint is described as the join of item slot taints.
- `Finish` emits result-slot taint with the terminal signal.
- No stale Clean-only wording remains for those resolved data-flow nodes.

### Scenario 2: Preserve evidence boundaries
Given: the doc contains DRIFT-1 resolution evidence but broader generated-mode parity remains a stated gap.
When: downstream reconciliation edits status wording.
Then:
- The doc does not claim new tests, formal proof, CI, or release readiness unless evidence exists.
- Full primitive parity remains a separate evidence requirement.
- Documentation consistency is not treated as implementation proof.

### Scenario 3: Preserve v1 control-flow taint non-goal
Given: v1 does not track control-flow taint.
When: data-flow taint wording is corrected to joined propagation.
Then:
- The control-flow taint section remains explicit.
- Secret branch conditions are not described as tainting results through DRIFT-1.
- Any v2 control-flow taint work remains out of scope.

### Scenario 4: Fail closed on unsupported evidence
Given: a proposed doc sentence says the implementation is fully verified for taint propagation.
When: the evidence audit cannot find concrete verification artifacts.
Then:
- The sentence is removed or marked pending.
- The downstream state reports `Error::UnsupportedEvidenceClaim` until corrected.

### Scenario 5: Lean clauses require executable companions
Given: INV-002, POST-001, and INV-003 are Lean-owned deterministic taint clauses.
When: downstream proof/test planning consumes this contract.
Then:
- INV-002 has `cargo test -p vb_core taint_join_laws` and Kani-if-present evidence.
- POST-001 has `cargo test -p vb_runtime joined_taint_propagation` or equivalent named workspace evidence.
- INV-003 has an executable doc consistency command proving data-flow taint wording does not imply control-flow taint tracking.
- `moon run :verify-proof` and `moon run :verify-standard` are selected as applicable evidence boundaries.

### Scenario 6: Waivers are complete and expiring
Given: fuzzing, concurrency, Miri/cargo-careful, performance, assembly, API, release-provenance, or Lean shell clauses are waived.
When: independent review validates the waiver.
Then:
- Each waiver names the exact clause IDs or explicitly states all `vb-l2d7` clauses.
- Each waiver names the waived layer, reason, compensating evidence, owner, and expiry/follow-up.
- The waiver expires immediately if downstream scope adds the waived technical surface.
