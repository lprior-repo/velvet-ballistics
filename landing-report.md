# Landing Report — 2026-06-16

## Scope
Landed the 199 dirty/untracked working-tree items the previous landing pass
intentionally left out, organised into 26 scope-aligned commits per crate.

## Commits Created (26 total, all on `main`, pushed to `origin/main`)

| # | Hash | Message |
|---|------|---------|
| 1  | `283a65e0e` | chore(workspace): tighten expect_used to deny + extend proof_obligations.yaml |
| 2  | `eeff91d10` | refactor(vb_cli): replace ActionRegistry import with Vec<ActionContract> |
| 3  | `da7b8aafc` | test(vb_benchmark): update batched_atomicity bench, edge cases, and integration tests |
| 4  | `f59d47564` | test(vb_boundary_inventory): refresh api/error/parser/property/validation test suites |
| 5  | `80150f1e3` | feat(vb_compile): update mod_compile_lowering flux reducers + add proofs.rs |
| 6  | `624ff7fa5` | test(vb_compile): refresh 43 digest/foreach/proptest/xi2f integration test files |
| 7  | `30dff9750` | feat(vb_core): engine step, frame, policy, value, action + Verus run_frame/step_state/action_specs proofs |
| 8  | `fb4cf5011` | test(vb_core): refresh 10 behavior/proptest/kani tests including step_budget, action_ticket, slug_budget |
| 9  | `7efcd5e83` | test(vb_doc): update vb_doc_api test |
| 10 | `7cd78b9e7` | feat(vb_expr): wire verus.rs into bytecode/eval/lexer/parser + add api_edge_cases tests |
| 11 | `17fb84ec0` | refactor(vb_ipc): server impl split + flatten vb_5iebh Verus spec to single file |
| 12 | `5ffbedb81` | refactor(vb_proof_kernels): refresh envelope/profile/budget/step_state/taint + proptest profile_properties |
| 13 | `5299da5e9` | test(vb_queue_semantics): refresh queue_boundary + density_semantics test suites |
| 14 | `963cb0b76` | refactor(vb_runtime): shard timer_wheel/transitions/dispatch + recovery + lib + tests |
| 15 | `4283ccc26` | feat(vb_runtime): refresh 10 Verus proofs (facade, vb-0l9k0, vb_rxru0, vb_y9d3v) + remove kani_rxru0_action_harnesses |
| 16 | `149b37aa7` | test(vb_runtime): refresh dispatch_generic + recovery bdd/hydration integration tests |
| 17 | `edda9ecab` | feat(vb_storage): recovery types/snapshot/summary + 6 vb_mrwe6 kani harnesses + recovery_types_spec verus |
| 18 | `ee31fbb43` | test(vb_storage): refresh 11 admission/proptest/recovery/edge-case test files |
| 19 | `a75a38772` | refactor(vb_test_util): refresh fixture/seed/temp_keyspace + density_tests |
| 20 | `78881fb66` | test(vb_validate): refresh gates + gate_08_accessor tests + capability_schema_kani |
| 21 | `e99a4887f` | test(vb_yaml): refresh 3 kani harnesses (all_variants, panic_freedom, error_code) |
| 22 | `4234cbeac` | test(workspace_tests): refresh 2 benches + 1 idempotency + 6 integration/contract tests |
| 23 | `d447c8342` | chore(evidence): refresh .evidence/{vb-kyyf,verus} + formal-verification-report + verification-ledger |
| 24 | `4a83d42bd` | test(vb_yaml): refresh lib_tests + source_map_tests (missed from commit 21) |
| 25 | `9a269eb6c` | docs(verification): add Kani/verification gap analysis reports |
| 26 | `c75e721dd` | feat(verus): add 13 production-bound Verus specs for vb_boundary_inventory, vb_validate gates, vb_yaml |

## Per-commit File Lists

### 1. `283a65e0e` — workspace/contracts
- `Cargo.toml`
- `contracts/proof_obligations.yaml`

### 2. `eeff91d10` — vb_cli `ActionRegistry` refactor
- `crates/vb_cli/src/action_specs.rs`
- `crates/vb_cli/src/agent_io.rs`
- `crates/vb_cli/src/dispatcher.rs`

### 3. `da7b8aafc` — vb_benchmark
- `crates/vb_benchmark/benches/batched_atomicity.rs`
- `crates/vb_benchmark/src/tests/edge_cases.rs`
- `crates/vb_benchmark/tests/batched_atomicity_tests.rs`

### 4. `f59d47564` — vb_boundary_inventory tests
- `crates/vb_boundary_inventory/src/tests/api_tests.rs`
- `crates/vb_boundary_inventory/src/tests/error_tests.rs`
- `crates/vb_boundary_inventory/src/tests/parser_tests.rs`
- `crates/vb_boundary_inventory/src/tests/property_tests.rs`
- `crates/vb_boundary_inventory/src/tests/validation_tests.rs`

### 5. `80150f1e3` — vb_compile mod_compile_lowering flux + new proofs.rs
- `crates/vb_compile/src/mod_compile_lowering.rs`
- `crates/vb_compile/src/mod_compile_lowering/part_01.rs`
- `crates/vb_compile/src/mod_compile_lowering/part_04.rs`
- `crates/vb_compile/src/mod_compile_lowering/proptest_nested_foreach.rs`
- `crates/vb_compile/src/mod_compile_lowering/reduce_body_width.flux`
- `crates/vb_compile/src/mod_compile_lowering/reduce_foreach.flux`
- `crates/vb_compile/src/mod_compile_lowering/reduce_nested_next.flux`
- `crates/vb_compile/src/mod_compile_lowering/reduce_offset.flux`
- `crates/vb_compile/src/mod_compile_lowering/reduce_overflow.flux`
- `crates/vb_compile/src/mod_compile_lowering/proofs.rs` (new)

### 6. `624ff7fa5` — vb_compile tests (43 files)
- `crates/vb_compile/src/tests/save_digest_unit_tests.rs`
- `crates/vb_compile/src/tests/validation_edge_cases.rs`
- `crates/vb_compile/tests/digest_ask_determinism.rs`
- `crates/vb_compile/tests/digest_ask_empty_prompt.rs`
- `crates/vb_compile/tests/digest_ask_explicit_arm.rs`
- `crates/vb_compile/tests/digest_ask_prompt_sensitivity.rs`
- `crates/vb_compile/tests/digest_ask_timeout_sensitivity.rs`
- `crates/vb_compile/tests/digest_compilation_pipeline.rs`
- `crates/vb_compile/tests/digest_duplicate_parity.rs`
- `crates/vb_compile/tests/digest_field_coverage.rs`
- `crates/vb_compile/tests/digest_repeat_unit.rs`
- `crates/vb_compile/tests/digest_set_finish_regression.rs`
- `crates/vb_compile/tests/digest_structural_fields.rs`
- `crates/vb_compile/tests/digest_yaml_e2e.rs`
- `crates/vb_compile/tests/finish_digest_integration.rs`
- `crates/vb_compile/tests/finish_digest_structural.rs`
- `crates/vb_compile/tests/foreach_at_once_tests.rs`
- `crates/vb_compile/tests/idempotency_parity.rs`
- `crates/vb_compile/tests/integration_choose_body.rs`
- `crates/vb_compile/tests/proptest/proptest_choose_depth.rs`
- `crates/vb_compile/tests/proptest/proptest_choose_emission.rs`
- `crates/vb_compile/tests/proptest/proptest_choose_fallthrough.rs`
- `crates/vb_compile/tests/proptest/proptest_choose_otherwise.rs`
- `crates/vb_compile/tests/proptest/proptest_choose_width.rs`
- `crates/vb_compile/tests/proptest_digest_ask_ordering.rs`
- `crates/vb_compile/tests/proptest_digest_ask_prompt_sensitivity.rs`
- `crates/vb_compile/tests/proptest_digest_ask_timeout_sensitivity.rs`
- `crates/vb_compile/tests/proptest_digest_determinism.rs`
- `crates/vb_compile/tests/proptest_digest_foreach.rs`
- `crates/vb_compile/tests/proptest_nested_foreach_roundtrip.rs`
- `crates/vb_compile/tests/proptest_save_canonical_name.rs`
- `crates/vb_compile/tests/proptest_save_digest_prefix.rs`
- `crates/vb_compile/tests/proptest_secret_results_digest_sensitivity.rs`
- `crates/vb_compile/tests/repeat_digest_integration.rs`
- `crates/vb_compile/tests/repeat_digest_proptest.rs`
- `crates/vb_compile/tests/together_digest_sensitivity.rs`
- `crates/vb_compile/tests/v1_primitive_lowering.rs`
- `crates/vb_compile/tests/vb_8mdp_7_collect_lowering_props.rs`
- `crates/vb_compile/tests/vb_a001_for_each_topology.rs`
- `crates/vb_compile/tests/vb_core_yaml_e2e_chain_strict_yaml.rs`
- `crates/vb_compile/tests/vb_xi2f_compile_source_proptest.rs`
- `crates/vb_compile/tests/vb_xi2f_error_variant_proptest.rs`
- `crates/vb_compile/tests/vb_xi2f_nested_do_lowering.rs`

### 7. `30dff9750` — vb_core source + verus (12 files)
- `crates/vb_core/src/action.rs`
- `crates/vb_core/src/engine/step.rs`
- `crates/vb_core/src/frame.rs`
- `crates/vb_core/src/lib.rs`
- `crates/vb_core/src/policy/mod.rs`
- `crates/vb_core/src/policy/runtime_limits_profile.rs`
- `crates/vb_core/src/value.rs`
- `crates/vb_core/src/verification/mod.rs`
- `crates/vb_core/src/verification/verus/run_frame_new_exec_proofs.rs`
- `crates/vb_core/src/verification/verus/step_state_absorbing_proofs.rs`
- `crates/vb_core/src/verification/verus/vb_rxru0_action_specs.rs`
- `crates/vb_core/src/workflow/lifecycle.rs`

### 8. `fb4cf5011` — vb_core tests (10 files)
- `crates/vb_core/src/replay/ops/tests.rs`
- `crates/vb_core/tests/action_ticket_kani_panic_free.rs`
- `crates/vb_core/tests/action_ticket_mock_field.rs`
- `crates/vb_core/tests/legacy_7field_deserialize.rs`
- `crates/vb_core/tests/proptest_core_types.rs`
- `crates/vb_core/tests/proptest_supported_codes.rs`
- `crates/vb_core/tests/proptest_symbolic_code.rs`
- `crates/vb_core/tests/resource_contract_type_integrity.rs`
- `crates/vb_core/tests/vb_5m8w_step_budget_suspension.rs`
- `crates/vb_core/tests/vb_ajc40_slug_budget_prop.rs`

### 9. `7efcd5e83` — vb_doc
- `crates/vb_doc/tests/vb_doc_api.rs`

### 10. `7cd78b9e7` — vb_expr (12 files, includes 4 new verus)
- `crates/vb_expr/src/bytecode/mod.rs`
- `crates/vb_expr/src/bytecode/verus.rs` (new)
- `crates/vb_expr/src/eval/mod.rs`
- `crates/vb_expr/src/eval/verus.rs` (new)
- `crates/vb_expr/src/lexer/mod.rs`
- `crates/vb_expr/src/lexer/verus.rs` (new)
- `crates/vb_expr/src/lib.rs`
- `crates/vb_expr/src/parser/mod.rs`
- `crates/vb_expr/src/parser/verus.rs` (new)
- `crates/vb_expr/src/tests.rs`
- `crates/vb_expr/src/tests/api_edge_cases.rs`
- `crates/vb_expr/src/tests/edge_cases.rs`

### 11. `17fb84ec0` — vb_ipc server + verus (6 files, 1 new + 2 deletes)
- `crates/vb_ipc/src/server/impl_.rs`
- `crates/vb_ipc/src/server/impl_tests.rs`
- `crates/vb_ipc/src/server/mod.rs`
- `crates/vb_ipc/src/verification/verus/vb_5iebh.rs` (new — replaces `vb_5iebh/mod.rs`)
- `crates/vb_ipc/src/verification/flux/vb_5iebh/mod.rs` (deleted)
- `crates/vb_ipc/src/verification/verus/vb_5iebh/mod.rs` (deleted)

### 12. `5ffbedb81` — vb_proof_kernels (8 files)
- `crates/vb_proof_kernels/src/envelope_header.rs`
- `crates/vb_proof_kernels/src/profile_contract/master.rs`
- `crates/vb_proof_kernels/src/resource_budget.rs`
- `crates/vb_proof_kernels/src/step_state.rs`
- `crates/vb_proof_kernels/src/taint.rs`
- `crates/vb_proof_kernels/tests/density_budget_contracts.rs`
- `crates/vb_proof_kernels/tests/proptest/vb_esq9_1/profile_properties.rs`
- `crates/vb_proof_kernels/tests/proptest/vb_esq9_1/profile_property_cases/gap_detection.rs`

### 13. `5299da5e9` — vb_queue_semantics
- `crates/vb_queue_semantics/src/tests/queue_boundary.rs`
- `crates/vb_queue_semantics/tests/density_semantics.rs`

### 14. `963cb0b76` — vb_runtime source (10 files)
- `crates/vb_runtime/src/engine/tests/mod.rs`
- `crates/vb_runtime/src/lib.rs`
- `crates/vb_runtime/src/primitives/collect/tests.rs`
- `crates/vb_runtime/src/recovery.rs`
- `crates/vb_runtime/src/recovery/tests.rs`
- `crates/vb_runtime/src/shard/impl_parts/dispatch.rs`
- `crates/vb_runtime/src/shard/tests.rs`
- `crates/vb_runtime/src/shard/timer_wheel.rs`
- `crates/vb_runtime/src/shard/timer_wheel/tests.rs`
- `crates/vb_runtime/src/shard/transitions.rs`

### 15. `4283ccc26` — vb_runtime verus + kani delete (11 files)
- `crates/vb_runtime/src/verification/kani/kani_rxru0_action_harnesses.rs` (deleted)
- `crates/vb_runtime/src/verification/verus/runtime_facade_api.rs`
- `crates/vb_runtime/src/verification/verus/runtime_facade_typed_errors.rs`
- `crates/vb_runtime/src/verification/verus/runtime_module_topology.rs`
- `crates/vb_runtime/src/verification/verus/vb-0l9k0/helpers.rs`
- `crates/vb_runtime/src/verification/verus/vb-0l9k0/mod.rs`
- `crates/vb_runtime/src/verification/verus/vb-0l9k0/numeric_timer.rs`
- `crates/vb_runtime/src/verification/verus/vb-0l9k0/pending_timer.rs`
- `crates/vb_runtime/src/verification/verus/vb-0l9k0/timer_wheel.rs`
- `crates/vb_runtime/src/verification/verus/vb_rxru0_action_verus.rs`
- `crates/vb_runtime/src/verification/verus/vb_y9d3v_action_fence.rs`

### 16. `149b37aa7` — vb_runtime integration tests
- `crates/vb_runtime/tests/dispatch_generic_properties.rs`
- `crates/vb_runtime/tests/recovery_bdd_tests.rs`
- `crates/vb_runtime/tests/recovery_hydration_tests.rs`

### 17. `edda9ecab` — vb_storage recovery + kani verus (14 files, 1 new verus)
- `crates/vb_storage/src/recovery/recovery_unit_tests.rs`
- `crates/vb_storage/src/recovery/replay/summary.rs`
- `crates/vb_storage/src/recovery/replay/summary/tests.rs`
- `crates/vb_storage/src/recovery/snapshot_write.rs`
- `crates/vb_storage/src/recovery/tests.rs`
- `crates/vb_storage/src/recovery/types.rs`
- `crates/vb_storage/src/verification/kani/vb_mrwe6_architecture_binding.rs`
- `crates/vb_storage/src/verification/kani/vb_mrwe6_atomic_index.rs`
- `crates/vb_storage/src/verification/kani/vb_mrwe6_completion_policy.rs`
- `crates/vb_storage/src/verification/kani/vb_mrwe6_duplicate_schedule.rs`
- `crates/vb_storage/src/verification/kani/vb_mrwe6_queue_intent.rs`
- `crates/vb_storage/src/verification/kani/vb_mrwe6_recovery_reliance.rs`
- `crates/vb_storage/src/verification/mod.rs`
- `crates/vb_storage/src/verification/verus/recovery_types_spec.rs` (new)

### 18. `ee31fbb43` — vb_storage tests (11 files)
- `crates/vb_storage/src/admission/tests.rs`
- `crates/vb_storage/tests/accepted_artifact_red_phase.rs`
- `crates/vb_storage/tests/proptest_ps_001_digest_binding.rs`
- `crates/vb_storage/tests/proptest_ps_003_size_bound.rs`
- `crates/vb_storage/tests/proptest_ps_006_artifact_digest_match.rs`
- `crates/vb_storage/tests/proptest_vb_vzcuf_PS_001.rs`
- `crates/vb_storage/tests/proptest_vb_vzcuf_PS_002.rs`
- `crates/vb_storage/tests/proptest_vb_vzcuf_PS_004.rs`
- `crates/vb_storage/tests/proptest_vb_vzcuf_PS_006.rs`
- `crates/vb_storage/tests/recovery_property_tests.rs`
- `crates/vb_storage/tests/vb_storage_edge_cases.rs`

### 19. `a75a38772` — vb_test_util
- `crates/vb_test_util/src/fixture.rs`
- `crates/vb_test_util/src/seed.rs`
- `crates/vb_test_util/src/temp_keyspace.rs`
- `crates/vb_test_util/tests/density_tests.rs`

### 20. `78881fb66` — vb_validate
- `crates/vb_validate/src/gate_08_accessor/tests.rs`
- `crates/vb_validate/src/gates.rs`
- `crates/vb_validate/tests/capability_schema_kani.rs`

### 21. `e99a4887f` — vb_yaml kani
- `crates/vb_yaml/src/kani/kani_all_variants_registered.rs`
- `crates/vb_yaml/src/kani/kani_panic_freedom.rs`
- `crates/vb_yaml/src/kani/kani_yaml_error_code.rs`

### 22. `4234cbeac` — workspace_tests (9 files)
- `crates/workspace_tests/benches/action_dispatch.rs`
- `crates/workspace_tests/benches/velvet_ballistics.rs`
- `crates/workspace_tests/idempotency_suite/tests/vb_ko29_5_public_idempotency.rs`
- `crates/workspace_tests/tests/contracts_as_data_kani.rs`
- `crates/workspace_tests/tests/contracts_as_data_kani/contracts_kani_harness.rs`
- `crates/workspace_tests/tests/integration_runtime_storage_fault_tolerance.rs`
- `crates/workspace_tests/tests/integration_storage_runtime_recovery.rs`
- `crates/workspace_tests/tests/integration_storage_runtime_validate_pipeline.rs`
- `crates/workspace_tests/tests/vb_qi37_1_1_red_recovery_contract_test.rs`

### 23. `d447c8342` — evidence + reports
- `.evidence/vb-kyyf/storage-replay-resume.md` (force-added, .gitignore line 88)
- `.evidence/verus/summary.txt` (force-added, .gitignore line 88)
- `formal-verification-report.md`
- `verification-ledger.jsonl`

### 24. `4a83d42bd` — vb_yaml tests (missed from commit 21)
- `crates/vb_yaml/src/lib_tests.rs`
- `crates/vb_yaml/src/source_map_tests.rs`

### 25. `9a269eb6c` — untracked verification docs
- `KANI_GAP_ANALYSIS.md`
- `VERIFICATION-GAP-ANALYSIS.md`
- `kani-report.md`

### 26. `c75e721dd` — untracked verus specs (13 files, all new)
- `verification/verus/vb_boundary_inventory.rs`
- `verification/verus/vb_validate_gate_07.rs`
- `verification/verus/vb_validate_gate_08.rs`
- `verification/verus/vb_validate_gate_13.rs`
- `verification/verus/vb_validate_idempotency_contract.rs`
- `verification/verus/vb_yaml_ast_well_formedness.rs`
- `verification/verus/vb_yaml_duplicate_key_detection.rs`
- `verification/verus/vb_yaml_error_kind_mapping.rs`
- `verification/verus/vb_yaml_is_primitive.rs`
- `verification/verus/vb_yaml_limit_enforcement.rs`
- `verification/verus/vb_yaml_production_bindings.rs`
- `verification/verus/vb_yaml_profile_validation.rs`
- `verification/verus/vb_yaml_source_span_validity.rs`

## Push Output
```
To https://github.com/lprior-repo/velvet-ballistics.git
   2ea093e74..c75e721dd  main -> main
```

## Files Intentionally Left Uncommitted

All 7 remaining `??` files are local build artifacts and were not committed per
the auto-skip rule:

| Path | Reason |
|------|--------|
| `crates/vb_compile/libproofs.rlib` | Cargo build artifact (.rlib) — should be in `target/`; this one slipped to source tree |
| `vb_validate_gate_07` | 4.1M ELF 64-bit executable (Verus build product) |
| `vb_validate_gate_08` | 4.1M ELF 64-bit executable (Verus build product) |
| `vb_validate_gate_13` | 4.1M ELF 64-bit executable (Verus build product) |
| `vb_validate_idempotency_contract` | 4.1M ELF 64-bit executable (Verus build product) |
| `velvet-ballistics:verify-kani-vb-ipc_dir/` | Moon task working dir (contains `kani/x86_64-unknown-linux-gnu/CACHEDIR.TAG`) |
| `velvet-ballistics:verify-kani-vb-storage_dir/` | Moon task working dir (contains `kani/x86_64-unknown-linux-gnu/CACHEDIR.TAG`) |

## Discipline Notes
- No `git commit --amend` was used. Commit 24 was created as a new commit when
  the missed `vb_yaml/src/lib_tests.rs` and `vb_yaml/src/source_map_tests.rs`
  surfaced after commit 21.
- `.evidence/` files in commit 23 required `git add -f` because the
  directory-level `.gitignore` rule at line 88 (`/.evidence/`) suppresses
  new additions even though earlier files in the directory are tracked.
- No pre-commit hooks were installed (only `.sample` files in `.git/hooks/`),
  so no gates ran during commit.
- Push succeeded first attempt; no rebase conflict or hook rejection.
- `git pull --rebase` ran before push and reported no upstream changes.

## Final State
- Branch: `main`
- HEAD: `c75e721dd` (matches `origin/main`)
- Working tree: 7 untracked build artifacts (intentionally skipped)
- No unpushed commits
