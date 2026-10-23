# WorkflowParts migration manifest

Pinned source: `b612001890c4b1d69ad20945899eed8d1c533e3f`. No listed source file was modified.

## Count reconciliation

- Literal substring `WorkflowParts {`: 646 hits in 203 files. This includes the distinct `SpecWorkflowParts` name.
- Whole identifier after masking Rust comments and strings: 645 occurrences in 202 files.
- Classified: 571 explicit literals, 70 return signatures, 3 declarations, 1 trait implementation header.
- Earlier base `6b591076ebc4423dbe01e320bd738f4b9eaa5aa7` had 202 literal-substring files. That historical 202-file figure is not a count of constructors.
- Newly present substring file(s): `crates/vb_core/tests/vb_8eocj_budget_depth.rs`.

The lists below are exact line locations at the pinned base, not a migration patch. Return signatures need no field addition. Struct-update literals inherit new fields from their base value. Historical evidence and verification mirrors require explicit ownership decisions, not bulk replacement.

## Inventory grouped by crate or repository area

| Area | Files | Explicit literals | Other occurrences |
| --- | ---: | ---: | ---: |
| .beads | 5 | 13 | 0 |
| fuzz | 13 | 17 | 5 |
| kani | 9 | 16 | 0 |
| vb_compile | 5 | 8 | 2 |
| vb_core | 48 | 280 | 40 |
| vb_ipc | 1 | 1 | 0 |
| vb_runtime | 50 | 103 | 1 |
| vb_storage | 9 | 19 | 2 |
| vb_validate | 13 | 13 | 15 |
| velvet_ballistics | 8 | 16 | 4 |
| verification | 2 | 1 | 1 |
| workspace_tests | 39 | 84 | 4 |

### .beads

- `.beads/vb-hz9v8/evidence/durable_retry_stale_probe_raw_red.rs`: literals 64.
- `.beads/vb-qi37.16.2/durable_resume_red_phase.rs`: literals 600, 637.
- `.beads/vb-qi37.16.3/red-phase/durable_retry_red_phase_tests.rs`: literals 71, 131, 257, 395, 607, 666.
- `.beads/vb-rcizj/evidence/baseline.rs`: literals 134, 174, 235.
- `.beads/vb-tf5lr/evidence/durable_retry_stale_probe.rs`: literals 64.

### fuzz

- `fuzz/fuzz_targets/fuzz_retry_normalize_ticket.rs`: literals 72.
- `fuzz/fuzz_targets/fuzz_retry_postcard_codec.rs`: literals 131.
- `fuzz/fuzz_targets/fuzz_retry_postcard_codec_flux.rs`: literals none; other 153 (struct declaration).
- `fuzz/fuzz_targets/fuzz_retry_record_attempt.rs`: literals 75.
- `fuzz/fuzz_targets/fuzz_retry_validate_completion.rs`: literals 69.
- `fuzz/src/expression_target.rs`: literals 63, 146.
- `fuzz/src/journal_target/admission.rs`: literals 37, 51, 98; other 97 (return signature).
- `fuzz/src/journal_target/readback.rs`: literals 84, 108; other 107 (return signature).
- `fuzz/src/validation_target/capability.rs`: literals 81; other 54 (return signature).
- `fuzz/src/validation_target/verifier.rs`: literals 198.
- `fuzz/src/workflow_target/accessor.rs`: literals 19.
- `fuzz/src/workflow_target/budget.rs`: literals 48; other 47 (return signature).
- `fuzz/tests/proptest_digest_coherence.rs`: literals 72, 102.

### kani

- `kani/gate_07_stack.rs`: literals 26, 74.
- `kani/gate_09_slots.rs`: literals 35, 76, 121.
- `kani/gate_10_node.rs`: literals 63, 133, 203, 272.
- `kani/gate_11_loop.rs`: literals 62, 140.
- `kani/vb-qi37.14.1/step_once_bounds_harness.rs`: literals 41.
- `kani/vb-qi37.14.1/step_once_error_harness.rs`: literals 33.
- `kani/vb-qi37.14.1/step_once_pc_bounds_harness.rs`: literals 18.
- `kani/vb-qi37.14.1/step_once_slot_init_harness.rs`: literals 20.
- `kani/vb-qi37.14.1/step_once_state_mapping_harness.rs`: literals 35.

### vb_compile

- `crates/vb_compile/src/kani_foreach_parity.rs`: literals 105, 436; other 49 (return signature).
- `crates/vb_compile/src/mod_compile_lowering/part_01.rs`: literals 62.
- `crates/vb_compile/src/mod_compile_lowering/part_05_ir.rs`: literals 23.
- `crates/vb_compile/src/mod_compile_lowering/part_08.rs`: literals 150.
- `crates/vb_compile/tests/vb_xi2f_error_variant_proptest.rs`: literals 37, 155, 203; other 25 (return signature).

### vb_core

- `crates/vb_core/src/budget/tests.rs`: literals 3850.
- `crates/vb_core/src/budget_vb_8mdp_7_prop_tests.rs`: literals 240.
- `crates/vb_core/src/engine/choose.rs`: literals 290.
- `crates/vb_core/src/engine/error_routing.rs`: literals 184, 244, 352; other 183 (return signature), 243 (return signature).
- `crates/vb_core/src/engine/expr_eval/accessors.rs`: literals 225.
- `crates/vb_core/src/engine/expr_eval/core.rs`: literals 158, 320.
- `crates/vb_core/src/engine/expr_eval/ops.rs`: literals 231.
- `crates/vb_core/src/engine/expr_eval/ops_text_list.rs`: literals 358.
- `crates/vb_core/src/engine/expr_eval/tests.rs`: literals 29, 1564.
- `crates/vb_core/src/engine/node_helpers.rs`: literals 99.
- `crates/vb_core/src/engine/run_loop.rs`: literals 57, 170.
- `crates/vb_core/src/engine/step/test_support.rs`: literals 35, 143, 167, 199.
- `crates/vb_core/src/engine/step/tests_action_resume.rs`: literals 281.
- `crates/vb_core/src/engine/step/tests_basic.rs`: literals 47, 108, 181.
- `crates/vb_core/src/engine/step/tests_state.rs`: literals 15, 118.
- `crates/vb_core/src/engine/step/tests_value_nodes.rs`: literals 25, 68, 129.
- `crates/vb_core/src/engine/tests/integration_accessor.rs`: literals 279.
- `crates/vb_core/src/engine/tests/integration_basic.rs`: literals 294, 359, 390; other 293 (return signature).
- `crates/vb_core/src/engine/tests/integration_budget.rs`: literals 31.
- `crates/vb_core/src/engine/tests/integration_choose.rs`: literals 281, 358.
- `crates/vb_core/src/engine/tests/integration_error_routing_behavior.rs`: literals 42, 1291, 1378.
- `crates/vb_core/src/engine/tests/integration_eval.rs`: literals 85, 136, 184, 234.
- `crates/vb_core/src/engine/tests/integration_object_list_behavior.rs`: literals 644, 719, 895, 1003, 1042.
- `crates/vb_core/src/engine/tests/integration_run_loop_behavior.rs`: literals 40, 80, 108, 149, 177, 236, 254.
- `crates/vb_core/src/engine/tests/integration_step_behavior.rs`: literals 47, 91, 142, 196, 257, 310, 375, 440, 479, 557, 594, 632, 670, 733, 799, 900, 1008, 1221, 1278, 1347, 1655, 1715, 1750, 1822, 1926, 1980; other 46 (return signature).
- `crates/vb_core/src/engine/tests/integration_taint_propagation.rs`: literals 309, 676, 941, 1243, 1341, 1825.
- `crates/vb_core/src/engine/tests/integration_workflow.rs`: literals 51, 88, 142, 236, 275, 313, 357, 413, 466, 564, 639, 737, 935, 975.
- `crates/vb_core/src/engine/validate.rs`: literals 388, 452, 463, 479, 504, 651, 682, 706, 741, 772, 798, 829, 856, 887, 914, 944, 974, 1001, 1028, 1057, 1086, 1113, 1141, 1169, 1199, 1232, 1277, 1340; other 387 (return signature).
- `crates/vb_core/src/engine/validate/tests.rs`: literals 16, 78, 89, 105, 130, 277, 308, 332, 367, 398, 424, 455, 482, 513, 540, 570, 600, 627, 654, 683, 712, 739, 767, 795, 823, 856, 901, 964; other 15 (return signature).
- `crates/vb_core/src/engine/validate/tests/red_phase_behavior_tests.rs`: literals 59, 77, 150, 284, 495, 522, 549, 582, 612, 638, 660, 682, 713, 741, 768, 796, 824, 851, 876, 911, 928, 945, 965, 987, 1059, 1093, 1119, 1153, 1177, 1233, 1264, 1281, 1326, 1372, 1414, 1478, 1572; other 58 (return signature), 76 (return signature), 1325 (return signature).
- `crates/vb_core/src/engine/validate/tests/red_phase_tests.rs`: literals 23; other 22 (return signature).
- `crates/vb_core/src/kani_step_harnesses.rs`: literals 53, 160, 234, 297, 444.
- `crates/vb_core/src/kani_workflow_arbitrary.rs`: literals none; other 351 (trait impl).
- `crates/vb_core/src/replay/kani_harnesses.rs`: literals 22.
- `crates/vb_core/src/replay/ops/tests.rs`: literals 25.
- `crates/vb_core/src/replay/step_tests.rs`: literals 22, 48.
- `crates/vb_core/src/replay/tests.rs`: literals 41.
- `crates/vb_core/src/workflow/mod.rs`: literals 192; other 191 (return signature), 337 (struct declaration).
- `crates/vb_core/src/workflow/proptest_collect_budget.rs`: literals 58; other 53 (return signature).
- `crates/vb_core/src/workflow/proptest_collect_traversal.rs`: literals 29; other 24 (return signature).
- `crates/vb_core/src/workflow/tests.rs`: literals 514, 561, 587, 632, 664, 699, 740, 781, 852, 932, 956, 988, 1127, 1153, 1356, 1689, 1905, 1973, 2076, 2176, 2214, 2250, 2302, 2354, 2388, 2422, 2458, 2516, 2557, 2596, 2641, 2690, 2729, 2764, 2798, 2831, 2916, 2969, 3013, 3049, 3095, 3153, 4343, 4434, 4531, 4577, 4646, 4697, 4742, 4770, 5167, 5236, 5261; other 504 (return signature), 513 (return signature), 560 (return signature), 820 (return signature), 835 (return signature), 850 (return signature), 1687 (return signature), 4242 (return signature), 4263 (return signature), 4280 (return signature), 4284 (return signature), 4342 (return signature), 4406 (return signature), 5165 (return signature).
- `crates/vb_core/tests/resource_contract_type_integrity.rs`: literals 81, 129.
- `crates/vb_core/tests/resource_contract_validation.rs`: literals 48; other 47 (return signature), 74 (return signature), 81 (return signature), 88 (return signature), 98 (return signature).
- `crates/vb_core/tests/section36_mandatory_coverage.rs`: literals 498, 598, 719, 825, 925, 1122, 1171, 1847, 2278; other 497 (return signature), 1170 (return signature), 1825 (return signature).
- `crates/vb_core/tests/section38_behavioral_properties.rs`: literals 25, 226, 273; other 225 (return signature).
- `crates/vb_core/tests/vb_5m8w_step_budget_suspension.rs`: literals 13.
- `crates/vb_core/tests/vb_8eocj_budget_depth.rs`: literals 669; other 668 (return signature).
- `crates/vb_core/tests/vb_qi37_2_5_boundedness_adversarial.rs`: literals 47, 86; other 85 (return signature).

### vb_ipc

- `crates/vb_ipc/src/server/trace.rs`: literals 200.

### vb_runtime

- `crates/vb_runtime/src/admission/evidence_test_support.rs`: literals 93.
- `crates/vb_runtime/src/admission/tests.rs`: literals 21.
- `crates/vb_runtime/src/engine/drive/tests/cat_branch.rs`: literals 133.
- `crates/vb_runtime/src/engine/drive/tests/common.rs`: literals 176.
- `crates/vb_runtime/src/engine/execute_tests.rs`: literals 53.
- `crates/vb_runtime/src/engine/tests.rs`: literals 838, 1042, 1114, 1192, 1649, 2166.
- `crates/vb_runtime/src/journal/tests/chunk_001.rs`: literals 30.
- `crates/vb_runtime/src/primitives/reduce/tests.rs`: literals 14.
- `crates/vb_runtime/src/primitives/reentry_tests.rs`: literals 308, 1211, 1275.
- `crates/vb_runtime/src/run_checkpoint_tests.rs`: literals 26.
- `crates/vb_runtime/src/runtime/shutdown_fixtures.rs`: literals 21, 128.
- `crates/vb_runtime/src/runtime_tests.rs`: literals 96, 136, 337, 397, 760, 3037.
- `crates/vb_runtime/src/shard/helpers/tests.rs`: literals 45, 84, 115, 177, 243, 299, 638, 706, 813, 899, 1035, 1152, 1189, 1502, 1762.
- `crates/vb_runtime/src/shard/impl_tests/chunk_001.rs`: literals 31.
- `crates/vb_runtime/src/shard/lifecycle_tests/action_recovery_tests.rs`: literals 117.
- `crates/vb_runtime/src/shard/lifecycle_tests/chunk_001.rs`: literals 53, 115, 154, 216, 265, 305.
- `crates/vb_runtime/src/shard/lifecycle_tests/chunk_002.rs`: literals 53, 200.
- `crates/vb_runtime/src/shard/lifecycle_tests/chunk_008.rs`: literals 20, 65.
- `crates/vb_runtime/src/shard/lifecycle_tests/contract_validation_tests.rs`: literals 51, 102, 143, 337, 431.
- `crates/vb_runtime/src/shard/tests/chunk_001.rs`: literals 62, 124.
- `crates/vb_runtime/src/shard/tests/chunk_002.rs`: literals 194, 243.
- `crates/vb_runtime/src/shard/tests/chunk_003.rs`: literals 53.
- `crates/vb_runtime/src/shard/tests/chunk_012.rs`: literals 298.
- `crates/vb_runtime/src/shard/tests/chunk_013.rs`: literals 54.
- `crates/vb_runtime/src/shard/tests/chunk_017.rs`: literals 108.
- `crates/vb_runtime/src/shard/tests/chunk_029.rs`: literals 371.
- `crates/vb_runtime/src/shard/tests/submission_parity.rs`: literals 134.
- `crates/vb_runtime/src/shard/tests/tick_shard_tests.rs`: literals 46, 88, 128.
- `crates/vb_runtime/src/verification/kani/kani_attempt_fence_harnesses.rs`: literals 74.
- `crates/vb_runtime/src/verification/kani/kani_engine_signals.rs`: literals 30.
- `crates/vb_runtime/src/verification/kani/kani_resume_state_machine.rs`: literals 79; other 75 (return signature).
- `crates/vb_runtime/src/verification/proptest/proptest_attempt_fence.rs`: literals 103.
- `crates/vb_runtime/src/verification/proptest/sxkz6_proptest.rs`: literals 49.
- `crates/vb_runtime/tests/durability_matrix_integration.rs`: literals 81, 111, 160, 230.
- `crates/vb_runtime/tests/durable_resume_red_phase.rs`: literals 680.
- `crates/vb_runtime/tests/durable_retry_boundary.rs`: literals 67.
- `crates/vb_runtime/tests/durable_retry_execution.rs`: literals 67.
- `crates/vb_runtime/tests/durable_retry_red_phase.rs`: literals 73, 136, 181, 247.
- `crates/vb_runtime/tests/durable_retry_run_attempt_authority.rs`: literals 72.
- `crates/vb_runtime/tests/policy_pin_durable_recovery.rs`: literals 69.
- `crates/vb_runtime/tests/proptest/ps_006_property.rs`: literals 26, 55.
- `crates/vb_runtime/tests/slot_validation_test.rs`: literals 67.
- `crates/vb_runtime/tests/support/verified_artifact.rs`: literals 93.
- `crates/vb_runtime/tests/ticket_authority_matrix.rs`: literals 69.
- `crates/vb_runtime/tests/vb_4qog3_independent_review.rs`: literals 65.
- `crates/vb_runtime/tests/vb_5m8w_step_budget_suspension_runtime.rs`: literals 21, 48.
- `crates/vb_runtime/tests/vb_jggy_lifecycle_tests.rs`: literals 134, 175, 237.
- `crates/vb_runtime/tests/vb_jggy_property_tests.rs`: literals 55.
- `crates/vb_runtime/tests/vb_qi37_12_2_resume_error_propagation.rs`: literals 746, 785.
- `crates/vb_runtime/tests/vb_r4fm9_event_generation.rs`: literals 64.

### vb_storage

- `crates/vb_storage/src/admission/tests.rs`: literals 201.
- `crates/vb_storage/src/executor/issuance_test_support.rs`: literals 86; other 85 (return signature).
- `crates/vb_storage/src/inputs/tests.rs`: literals 9.
- `crates/vb_storage/src/kani_admission.rs`: literals 30.
- `crates/vb_storage/src/proptests.rs`: literals 602, 619, 671, 729, 748, 821, 891.
- `crates/vb_storage/src/recovery/tests.rs`: literals 32, 687; other 31 (return signature).
- `crates/vb_storage/src/vb_2bok_durability_gate_tests.rs`: literals 45.
- `crates/vb_storage/tests/accepted_artifact_red_phase.rs`: literals 20, 303.
- `crates/vb_storage/tests/vb_core_atomic_admission_red.rs`: literals 156, 840, 862.

### vb_validate

- `crates/vb_validate/benches/capability_schema.rs`: literals 11; other 10 (return signature).
- `crates/vb_validate/src/gate_08_accessor/tests.rs`: literals 11; other 10 (return signature), 49 (return signature).
- `crates/vb_validate/src/gate_tests.rs`: literals 16; other 15 (return signature).
- `crates/vb_validate/src/red_phase_proptest.rs`: literals 22; other 21 (return signature).
- `crates/vb_validate/src/shared/tests.rs`: literals 11; other 10 (return signature).
- `crates/vb_validate/src/test_helpers.rs`: literals 10; other 9 (return signature).
- `crates/vb_validate/src/verification/gate_08_verus_proof.rs`: literals 147; other 146 (return signature).
- `crates/vb_validate/src/verification/kani_gate_08_accessor.rs`: literals 184; other 183 (return signature).
- `crates/vb_validate/src/verification/kani_gate_08_structural.rs`: literals none; other 35 (return signature).
- `crates/vb_validate/tests/capability_contract_schema.rs`: literals 41; other 14 (return signature).
- `crates/vb_validate/tests/gate_08_accessor_parity.rs`: literals 36; other 35 (return signature), 53 (return signature).
- `crates/vb_validate/tests/idempotency_contract_red.rs`: literals 76; other 75 (return signature).
- `crates/vb_validate/tests/red_phase_validation.rs`: literals 18, 252; other 17 (return signature).

### velvet_ballistics

- `crates/velvet_ballistics/src/run_compiled_input_tests.rs`: literals 23.
- `crates/velvet_ballistics/tests/admission_evidence_integration/chunk_001.rs`: literals 52, 104, 154.
- `crates/velvet_ballistics/tests/admission_evidence_integration/chunk_004.rs`: literals 86, 122.
- `crates/velvet_ballistics/tests/cli_integration.rs`: literals 40, 65; other 29 (return signature), 64 (return signature).
- `crates/velvet_ballistics/tests/cross_crate_adversarial.rs`: literals 19, 317, 440, 1406; other 18 (return signature).
- `crates/velvet_ballistics/tests/durable_retry_cli.rs`: literals 64, 104.
- `crates/velvet_ballistics/tests/ir_artifact_admission.rs`: literals 39; other 38 (return signature).
- `crates/velvet_ballistics/tests/lifecycle_integration.rs`: literals 57.

### verification

- `verification/verus/production_inner/try_from_parts_production.rs`: literals none; other 385 (struct declaration).
- `verification/verus/production_inner/vb_xi2f_compile_source_production.rs`: literals 717.

### workspace_tests

- `crates/workspace_tests/benches/action_queuing.rs`: literals 47.
- `crates/workspace_tests/benches/cold_start.rs`: literals 65.
- `crates/workspace_tests/benches/ir_traversal.rs`: literals 53, 141, 204.
- `crates/workspace_tests/benches/memory_footprint.rs`: literals 71.
- `crates/workspace_tests/benches/snapshot_restore.rs`: literals 60.
- `crates/workspace_tests/benches/snapshot_save.rs`: literals 62.
- `crates/workspace_tests/benches/vb-qi37-7-3-validation.rs`: literals 21; other 20 (return signature).
- `crates/workspace_tests/benches/velvet_ballistics.rs`: literals 959, 982, 1256, 1478, 1593, 1738.
- `crates/workspace_tests/tests/cancel_kill_lattice_tests.rs`: literals 68.
- `crates/workspace_tests/tests/integration_storage_runtime_validate_pipeline.rs`: literals 49.
- `crates/workspace_tests/tests/integration_validate_policy_enforcement.rs`: literals 28; other 27 (return signature).
- `crates/workspace_tests/tests/raii_introspection_registry_tests.rs`: literals 73.
- `crates/workspace_tests/tests/runtime_fjall_pending_action_recovery.rs`: literals 181.
- `crates/workspace_tests/tests/runtime_input_schema_kinds.rs`: literals 22.
- `crates/workspace_tests/tests/slot_written_ordering_integration_tests.rs`: literals 51.
- `crates/workspace_tests/tests/vb_79ffw_ticket_authority_probe.rs`: literals 63.
- `crates/workspace_tests/tests/vb_7x0ol_probe_registered_executor.rs`: literals 56.
- `crates/workspace_tests/tests/vb_8mdp_7_resource_admission_props.rs`: literals 47.
- `crates/workspace_tests/tests/vb_c1s0_orchestration_runtime_tests.rs`: literals 76.
- `crates/workspace_tests/tests/vb_c4hzw_corrupt_marker.rs`: literals 54.
- `crates/workspace_tests/tests/vb_c4hzw_emergency_reopen/fixtures.rs`: literals 39.
- `crates/workspace_tests/tests/vb_c4hzw_historical_replay/fixtures.rs`: literals 39.
- `crates/workspace_tests/tests/vb_c4hzw_legacy_admission/legacy_workflow.rs`: literals 21.
- `crates/workspace_tests/tests/vb_core_yaml_e2e_chain_contract.rs`: literals 448.
- `crates/workspace_tests/tests/vb_ko29_5_fixture/mod.rs`: literals 59.
- `crates/workspace_tests/tests/vb_kyyf_cross_run_determinism.rs`: literals 361.
- `crates/workspace_tests/tests/vb_lp2v_admission_proptests.rs`: literals 31, 315.
- `crates/workspace_tests/tests/vb_policy_dispatch_fail_closed/fixtures.rs`: literals 51.
- `crates/workspace_tests/tests/vb_qi37_4_2_strict_runtime_admission.rs`: literals 137, 179.
- `crates/workspace_tests/tests/vb_ssei_verification_admission_acceptance.rs`: literals 71.
- `crates/workspace_tests/tests/vb_test_cli_storage_io_behavior.rs`: literals 85.
- `crates/workspace_tests/tests/vb_test_core_admission_trigger_behavior.rs`: literals 104, 158, 209, 250, 443, 484, 679, 752, 1143.
- `crates/workspace_tests/tests/vb_test_core_workflow_slot_behavior.rs`: literals 47, 89, 141, 202, 486, 538, 576, 614, 660, 698, 753, 853, 889, 1074, 1108, 1153, 1197, 1243, 1302, 1350, 1381, 1430, 1474, 1545.
- `crates/workspace_tests/tests/vb_test_core_yaml_chain_behavior.rs`: literals 497, 546, 616, 742.
- `crates/workspace_tests/tests/vb_test_runtime_ipc_resource_behavior.rs`: literals 75, 105, 146.
- `crates/workspace_tests/tests/vb_test_runtime_lifecycle_state_behavior.rs`: literals 71.
- `crates/workspace_tests/tests/vb_test_validate_policy_enforce_behavior.rs`: literals 34; other 33 (return signature).
- `crates/workspace_tests/tests/vb_test_validate_yaml_parsing_behavior.rs`: literals 34; other 33 (return signature).
- `crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs`: literals 72.

## Constructor detail

`migration-sites.json` records every classified occurrence and each explicit literal's current symbols_count expression. It is generated from the exact Git objects, not the mutable checkout. The adjacent generator delivery file records the lexical masking/classification method; this is an inventory, not a Rust type-check or proof.

## Additional construction and validation routes

These routes are not all explicit `WorkflowParts {` literals and must not be
missed by a mechanical field-addition sweep:

- `crates/vb_core/src/workflow/mod.rs:19,37,63,84,191,337`: CompiledWorkflow
  declaration, validated constructor, test-util constructor, Kani constructor,
  `to_parts`, and serializable WorkflowParts declaration. A future approved
  change must preserve metadata in all three constructors and `to_parts`,
  validate declared count against real payloads, and introduce the module/export.
- `crates/vb_core/src/kani_workflow_arbitrary.rs:351,387`: trait implementation
  and its `Self {` constructor. Generate varied metadata and malformed cases;
  do not hardcode a single empty fixture as a correctness proof.
- `crates/vb_core/src/lib.rs:129`: current workflow re-export list. A new export
  is a separate API fence, not part of this patch.
- `verification/verus/production_inner/try_from_parts_production.rs:385` is a
  production-shape mirror with an explicit drift contract; coordinate its owner
  and the production-binding/drift gates. The Flux declaration in
  `fuzz/fuzz_targets/fuzz_retry_postcard_codec_flux.rs:153` is a documented
  refinement placeholder, not a production constructor. Do not count either as
  behavioral proof or blindly change its contract.
- Historical `.beads` evidence sources are archived evidence. Their listed
  literals are inventory, not authorization to rewrite old evidence.
- Struct-update literals (`..parts`) inherit metadata and require semantic
  digest/round-trip review, not necessarily an explicit field addition.

## Coordinated compiler and digest fences

All paths in this and subsequent sections are future coordination requirements,
not edits delivered by .4.1:

- `crates/vb_compile/src/expression_bytecode.rs`: existing reference resolver
  and expression compilation seam; add one fallible symbol sink only under the
  compiler owner's approved dependent-interner fence.
- `crates/vb_compile/src/mod_compile_lowering/canonical_scalar.rs` and
  `part_07.rs`: resolver/slot compiler ownership and shared interner lifetime.
- `crates/vb_compile/src/mod_compile_lowering/part_01.rs:26,62,77`:
  canonical source lowering, main parts construction, zeroed-parts Postcard
  digest; freeze and attach the actual table rather than only symbols_count.
- `crates/vb_compile/src/mod_compile_lowering/part_05_ir.rs:13,23` and
  `part_08.rs:145,150`: alternative low-level constructors/builders.
- `crates/vb_compile/src/mod_compile_lowering/part_05_digest.rs:200`:
  canonical source identity already includes both SecretField.key and .value.
  Preserve that relationship in compiled metadata; alias is the key and
  requirement is the declared environment/provider name, never its resolved
  value. Provider interpretation remains a separate Master decision.
- `crates/vb_compile/src/mod_compile_core.rs:159` and
  `crates/vb_compile/src/artifact.rs:168,177,182`: artifact emission and compiler
  verification/canonical digest. Both must see exactly the same versioned parts.
- `crates/vb_storage/src/admission.rs:166,448,464,471`: accepted-envelope
  canonical digest plus compiled-IR canonical digest and artifact emission.
  These are separate digest layers; do not hash a nonzero self-digest.
- `crates/vb_storage/src/admission/verification.rs:10,18,55,71,160,262`:
  EVIDENCE_VERSION is currently 3, prepare/verify paths, strict trailing-byte
  decode, zeroed-parts digest, and verifier implementation fingerprint. Any
  future metadata schema needs explicit version policy; add source_metadata.rs
  to the fingerprint when this validator becomes part of admission.

## Production decode entry points

- `crates/vb_storage/src/admission/verification.rs:71`: typed `take_from_bytes`.
- `crates/vb_storage/src/executor/packed.rs:124,145`: independent packed evidence
  schema gate and typed `take_from_bytes`; must not lag the primary verifier.
- `crates/vb_runtime/src/runtime.rs:981`: recovery artifact WorkflowParts decode.
- `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:389`: inferred typed
  WorkflowParts decode during shard recovery.
- `crates/velvet_ballistics/src/commands_ai_context.rs:257,266,268`: raw parts
  decode and accepted-artifact envelope fallback, then inner parts decode.
- `crates/velvet_ballistics/src/ipc_serve.rs:207`: resolved artifact/raw parts
  decoding for IPC workflow resolution.
- `crates/velvet_ballistics/src/run_compiled.rs:23`: direct compiled-file CLI
  decode, outside the accepted-artifact primary verifier path.

Each route needs the same explicit legacy/new-schema dispatch and bounded decode
policy. Serde defaults do not independently solve positional Postcard evolution.
A byte-length envelope cap does not prove hostile nested lengths cannot allocate
before validation. Do not claim the immutable constructor provides that decoder.

## Persistence and recovery fences

- `crates/vb_runtime/src/shard/lifecycle/execution_binding.rs:59`: hashes or
  serializes `workflow.to_parts()` to bind a direct workflow to execution.
- `crates/vb_storage/src/journal/source.rs`: raw/accepted compiled-IR record
  persistence and readback, including source-digest lookup.
- `crates/vb_storage/src/journal/admission.rs:25`: accepted-envelope decode and
  digest verification; nested IR changes affect its contract even without a
  direct WorkflowParts reference.
- `crates/vb_storage/src/journal/artifact_acceptance.rs`: atomic acceptance
  issuance, envelope rebinding, and durable batch write.
- `crates/vb_storage/src/batch/putters.rs:71` and
  `crates/vb_storage/src/codec/{mod.rs,envelope.rs,payload.rs,validation.rs}`:
  bounded compiled-record encoding/decoding.
  Preserve existing MAX_COMPILED_IR_BYTES and storage envelope version rules;
  this proposal introduces no new aggregate limit or storage format.
- `crates/vb_core/src/value_store.rs:81,95`: actual runtime arena cap and symbol
  insertion. The current cap is tied to max_slots and insertion does not
  deduplicate. Master must decide admitted prefix capacity and dynamic identity.
- `crates/vb_core/src/value_store/checkpoint.rs:32,44`: serializes and restores
  all arena symbols in ID order, with no compiled-prefix identity contract yet.
- `crates/vb_runtime/src/run_checkpoint.rs:94,152,204,252`: capture, workflow-bound
  validation, restore, and versioned checkpoint decode. Checkpoint restoration
  must verify the prefix, not append it again.
- `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:152,499`: fresh and
  noncheckpoint recovered ValueStore creation. Reserve/seed source payloads
  before external/action values; preserve existing full-checkpoint restore.
- `crates/vb_runtime/src/shard/helpers.rs:19`: ordinary input seeding currently
  sets clean taint. It is not an approved secret injection path.
- `crates/vb_storage/src/inputs/validation.rs`: handle-only external input
  validation cannot supply missing list/object/symbol payload graphs. A future
  structured input contract needs bounded payload transport and handle remap.
- `crates/vb_core/src/engine/expr_eval/ops.rs:15`: equality currently compares
  SlotValue handles; source/external text with equal bytes needs an explicit
  identity/equality solution before an end-to-end claim.
- `crates/vb_runtime/src/admission/parts/chunk_002_records.rs`: RunAdmission has
  no approved secret-availability receipt. Any later presence/revocation/recovery
  semantics require an explicit new version contract, with no raw secret bytes.

Reserved local `.3.7` drive/{loop_rearm.rs, transitions.rs, mod.rs, loop_step.rs}
files are intentionally not proposed as metadata implementation targets.

## Compatibility and fixture coordination

Priority existing coverage to extend under each owner's fence:

- `crates/workspace_tests/tests/artifact_evidence_support/{compiler_cases.rs,
  schema3_cases.rs,tamper_cases.rs}`: exact-schema compatibility, compiler artifact
  emission, and tamper rejection. Pin old version bytes before migration.
- `crates/workspace_tests/tests/{compiler_input_schema_kinds.rs,
  finish_input_binding.rs}`: round-trip fields and source/input bindings.
- `crates/vb_storage/tests/{accepted_artifact_red_phase.rs,
  vb_core_atomic_admission_red.rs}`: accepted IR decode and atomic persistence.
- `crates/velvet_ballistics/tests/{cli_integration.rs,
  cross_crate_adversarial.rs}`: direct compiled-file and artifact consumers.
- `fuzz/fuzz_targets/{compiled_ir.rs,expr_eval.rs}`,
  `fuzz/src/{expression_target.rs,journal_target/admission.rs,
  journal_target/readback.rs,workflow_target/compiled.rs,
  workflow_target/generated.rs}`, and `fuzz/tests/proptest_digest_coherence.rs`:
  hostile decode, round-trip, and digest oracles must include the actual table.

The adjacent test plan defines source-to-runtime acceptance, old/new wire cases,
secret presence-only behavior, and checkpoint prefix tests. None were run in the
cloud; the standalone source file remains unhooked pending native verification.
