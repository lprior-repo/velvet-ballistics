# integrated-code-review (local5090)

**Reviewed snapshot:** `34b7faab7` (codex/vb-eyij7-combined)
**Baseline for migration/range diff:** `origin/main` (fetched 2026-10-21)
**Reviewer:** local5090 independent code reviewer
**Skill:** black-hat-reviewer (scoped; no Go-skill/proof-planning)
**Scope:** crate split (vb_expr / vb_yaml / vb_compile re-exports), runtime admission contract, recovery product typing, tests.
**Outcome:** 3 blocker findings → REJECTED. Do not advance to test review.

---

## 1. Contract & Bead Parity — Admission validation does not bind to workflow action IDs

**Finding:** Submitted `ActionContract`s are not validated against the compiled workflow's action step IDs. Any caller can submit a contract with `id = ActionId::new(999)` for a workflow with no such action step; the admission gate accepts it and only fails at execution time with `UnknownAction`.

**Source evidence:**
- `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:310-355` — `validate_submit_admission_with_contracts` checks: (a) duplicate contract IDs, (b) well-formed names, (c) capability grants. No workflow binding check.
- `crates/vb_runtime/src/runtime.rs:274,298` — submits call `validate_submit_admission_with_contracts(run, workflow.digest(), caps, &contracts)` but the workflow is never passed to the validation logic (only the digest). The compiled `CompiledWorkflow` with its nodes is passed separately to `ShardCommand::SubmitWithContracts` but never used for contract validation.

**Why this is a contract gap, not a design choice:**
- Compiled workflows expose `CompiledNodeKind::Do { action: ActionId, input }` (`crates/vb_core/src/workflow/mod.rs:683`).
- `CompiledWorkflow::node(step)` provides access to nodes (`crates/vb_core/src/workflow/mod.rs:136-138`).
- The runtime has access to the full compiled workflow at submit time and could iterate nodes to collect action IDs and validate submitted contracts against them. It simply does not.

**Severity:** BLOCKER

**Correction:** Add a check in `validate_submit_admission_with_contracts` (or `handle_submit_with_inputs_contracts_and_header_mode`) that iterates the compiled workflow's nodes, collects all `Do { action, .. }` action IDs, and requires that submitted contract action IDs are a subset of the collected IDs. Reject any contract with an action ID that does not appear in the compiled workflow.

---

## 2. Recovery command does not validate action contracts against the loaded artifact

**Finding:** The typed recovery command (`RecoverRunCommand`) carries `artifact_digest` but no action contract validation is performed during recovery. The shard's `handle_recover` loads the artifact, verifies the workflow digest, and uses `artifact.action_contracts.clone()` to build the RunState without checking that the loaded artifact's contracts match the recovered run's expected contracts.

**Source evidence:**
- `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:402-510` — `handle_recover` flow:
  1. Loads artifact by `artifact_digest`
  2. Verifies workflow digest matches `workflow_digest`
  3. Builds RunState with `action_contracts: artifact.action_contracts.clone()`
  - No contract validation against submitted or expected contracts.

**Severity:** BLOCKER

**Correction:** The recovery path must validate that the recovered artifact's action contracts are compatible with the recovered run state, including verifying the action ABI digests match (the runtime already computes `action_abi_digests_from_contracts` at `crates/vb_runtime/src/runtime.rs:547`).

---

## 3. Unrepresentable illegal states: recovery can accept any action ID at execution time

**Finding:** The admission layer accepts contracts for action IDs that are not in the compiled workflow, and the recovery layer accepts contracts from the loaded artifact without validation. There is no representation-level enforcement that contracts correspond to valid workflow actions.

**Source evidence:**
- Contract validation (`chunk_001.rs:310-355`) treats action IDs as opaque numeric values; it never checks that they are valid `Do` node action IDs in the workflow.
- `CompiledWorkflow` has no accessor like `action_ids()` or `has_action(ActionId)`, making the validation impossible without adding such accessors.

**Severity:** BLOCKER

**Correction:** Add an accessor to `CompiledWorkflow` (e.g., `action_ids() -> Vec<ActionId>` that iterates nodes and collects `Do { action, .. }` action IDs), then use it in admission validation and recovery validation. This makes it impossible to represent a run with a contract for a non-existent action.

---

## 4. Migration crate split — verified sound

**Finding:** The split of `vb_expr` and `vb_yaml` from `vb_compile` is structurally sound. All old public module paths remain accessible via re-exports. The migration_compat tests (9 tests) all pass.

**Verified via:**
- `crates/vb_compile/src/lib.rs:19-28` — re-exports from `vb_expr` and `vb_yaml`.
- `crates/vb_compile/src/migration_compat.rs` — compile-time path resolution tests.
- No production code references `vb_compile::expr_*` paths directly (`grep -r "vb_compile::expr_" crates/` returns 0 results).

**Severity:** OK (not a finding)

---

## 5. Typed RecoverRunCommand — structurally complete but lacks validation

**Finding:** The `RecoverRunCommand` type at `crates/vb_runtime/src/shard/types.rs:202-244` is well-designed and carries all necessary recovery context: `run`, `frame`, `artifact_digest`, `workflow_digest`, `next_seq`, `collect_states`, and `boundary`. The conversion from `ResumableRecoveryProduct` preserves all fields. However, this structural completeness is undermined by the admission validation gaps noted in Findings 1-3.

**Severity:** OK (structurally); underlying admission validation is the blocker.

---

## 6. Capability equality — strict exact-match verified

**Finding:** The capability equality check at `crates/vb_runtime/src/admission/parts/chunk_005_admit_core.rs:135-150` correctly enforces cardinality-exact equality (not just subset) between granted capabilities and the artifact's required capabilities. This matches VERUS-CARD-003 and is sound.

**Severity:** OK (not a finding)

---

## Test coverage assessment (informational; not the test reviewer's scope)

The current test suite for `validate_submit_admission_with_contracts` (`crates/vb_runtime/src/shard/lifecycle_tests/contract_validation_tests.rs`) and `vb_3l40a_contract_validation_tests` (`chunk_001.rs:279-395`) tests:
- Duplicate action ID rejection
- Empty name rejection
- Insufficient capability rejection
- Sufficient capability acceptance

**Missing test coverage (relevant to Findings 1-3):**
- No test verifies that a contract with an action ID not in the compiled workflow is rejected at admission time.
- No test verifies that a contract with an action ID in the workflow is accepted.
- These are not test failures; they are tests that have not been written. The test reviewer should request these before declaring test parity.

---

## Disposition

**REJECTED.**

The three blocker findings represent fundamental admission contract gaps. The runtime accepts action contracts for non-existent workflow actions and accepts recovered artifacts without contract validation. This violates the principle of parse-don't-validate (contracts should be parsed/validated into the run state at admission time, not at execution time) and leaves illegal states representable.

All three blockers can be resolved by:
1. Adding an `action_ids()` accessor to `CompiledWorkflow` that collects `Do { action, .. }` action IDs from all nodes.
2. Using it in `validate_submit_admission_with_contracts` to validate submitted contracts.
3. Using it in `handle_recover` to validate recovered artifact contracts.

The migration crate split (vb_expr/vb_yaml/vb_compile) and the typed recovery command infrastructure are structurally sound; the blockers are in admission validation logic, not in the migration or recovery infrastructure.
