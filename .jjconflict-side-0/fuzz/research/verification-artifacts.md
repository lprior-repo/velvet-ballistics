# Velvet-Ballistics: Complete Verification & Testing Artifact Inventory

**Date**: 2026-05-24  
**Scope**: Full repository audit of all verification, testing, proof, fuzzing, and contract artifacts  
**Repository**: `/home/lewis/src/velvet-ballistics`

---

## 1. TEST PLANS (16 files)

All located at repo root, each covers a specific testing domain:

| File | Domain | Size | Status |
|------|--------|------|--------|
| `test-plan-and-or-shortcircuit.md` | AND/OR short-circuit fix (Section 46 violation) | 434 lines | Plan complete |
| `test-plan-arrayqueue.md` | ArrayQueue lock-free SPSC migration (Section 50) | 303 lines | Plan complete |
| `test-plan-attempt-number.md` | `$attempt.number` restriction not implemented | 601 lines | Plan complete |
| `test-plan-benchmarks.md` | Section 39 missing benchmarks (12 groups) | 721 lines | Plan complete |
| `test-plan-bounded-queue.md` | Bounded action completion queue (LETHAL-5) | 323 lines | Plan complete |
| `test-plan-enum-mismatch.md` | SideEffect/RetrySafety enum mismatch (Section 65) | 300 lines | Plan complete |
| `test-plan-f64-contradiction.md` | F64 arithmetic contradiction (LETHAL-3) | 286 lines | Plan complete |
| `test-plan-helper-coverage.md` | Section 46 helper function coverage gaps | 867 lines | Plan complete |
| `test-plan-journal-event-fuzz.md` | journal_event fuzz target (LETHAL-7) | 267 lines | Plan complete |
| `test-plan-property-tests.md` | Section 38 property tests (11 invariants) | 936 lines | Plan complete |
| `test-plan-remaining-lethals.md` | Lethal cross-cutting C.1–C.25 | 736 lines | Plan complete |
| `test-plan-slot-written-ordering.md` | SlotWritten-before-PC-advance ordering | 257 lines | Plan complete |
| `test-plan-tick-shard.md` | Runtime::tick_shard API & ShardDirective | 437 lines | Plan complete |
| `test-plan-trybuild.md` | trybuild silent pass (MAJOR-2) | 165 lines | Plan complete |
| `test-plan-ui-command.md` | UI command missing (LETHAL-6) | 256 lines | Plan complete |
| `test-plan-validate-taint.md` | validate_taint SecretResultLeak pass-through (LETHAL-1) | 572 lines | Plan complete |

**Total test plan coverage**: ~7,560 lines across 16 domains.

---

## 2. TEST REVIEWS (24 files)

### test-review-*.md (19 files)

| File | Domain Reviewed |
|------|-----------------|
| `test-review-and-or-shortcircuit.md` | AND/OR short-circuit fix review |
| `test-review-arrayqueue.md` | ArrayQueue migration review |
| `test-review-attempt-number.md` | $attempt.number restriction review |
| `test-review-benchmarks.md` | Benchmarks plan review |
| `test-review-bounded-queue.md` | Bounded queue review |
| `test-review-enum-mismatch.md` | Enum mismatch review |
| `test-review-f64-contradiction.md` | F64 contradiction review |
| `test-review-helper-coverage.md` | Helper coverage review |
| `test-review-journal-event-fuzz.md` | Journal event fuzz review |
| `test-review-property-tests.md` | Property tests plan review |
| `test-review-remaining-lethals.md` | Remaining lethals review |
| `test-review-round2-a5-a8.md` | Round 2 a5-a8 review |
| `test-review-round2-blackhat.md` | Round 2 black-hat review |
| `test-review-round2-c1-c25.md` | Round 2 c1-c25 review |
| `test-review-slot-written-ordering.md` | SlotWritten ordering review |
| `test-review-tick-shard.md` | tick_shard review |
| `test-review-trybuild.md` | trybuild review |
| `test-review-ui-command.md` | UI command review |
| `test-review-validate-taint.md` | validate_taint review |

### test-suite-review-*.md (5 files)

| File | Domain Reviewed |
|------|-----------------|
| `test-suite-review-a1-a4.md` | Suite review a1-a4 |
| `test-suite-review-a5-a8.md` | Suite review a5-a8 |
| `test-suite-review-b1-b4.md` | Suite review b1-b4 |
| `test-suite-review-b5-b6-helpers.md` | Suite review b5-b6 helpers |
| `test-suite-review-c1-c25.md` | Suite review c1-c25 |

---

## 3. VERIFICATION/ DIRECTORY

```
verification/
├── kani/
│   └── harnesses/
│       └── resource_bounds_harnesses.rs           # 1 Kani harness
├── tla/                                           # 22 TLA+ specs w/ configs
│   ├── AcceptedCliAdmission.tla + .cfg
│   ├── AtomicAcceptedRunAdmission.tla + .cfg
│   ├── CapabilityLifecycle.tla + 7 .cfg variants
│   ├── ConcurrencyControl.tla + .cfg
│   ├── EngineYamlAdmission.tla + .cfg
│   ├── EngineYamlIngress.tla + .cfg
│   ├── EngineYamlRecovery.tla + .cfg
│   ├── EngineYamlRunLifecycle.tla + .cfg
│   ├── IdempotencySafety.tla + .cfg
│   ├── IpcSyncEvidence.tla + 2 .cfg variants
│   ├── LifecycleJournal.tla + .cfg
│   ├── RecoveryCrashRestart.tla + .cfg
│   ├── RecoveryHydration.tla + .cfg
│   ├── RetryFSM.tla + .cfg
│   ├── StepBudgetSuspension.tla + .cfg
│   ├── TimerWheel.tla + .cfg + 6 coverage .cfgs
│   ├── V1PrimitiveLowering.tla + .cfg
│   ├── VbKyyfReplayDeterminism.tla + .cfg
│   ├── Vt2fRuntimeLifecycle.tla + .cfg
│   ├── Vt2fStrictAdmission.tla + .cfg
│   ├── WorkflowBoundedAdmission.tla + .cfg
│   ├── YamlE2eChain.tla + .cfg
│   ├── specs/
│   │   ├── ActionRouting.tla + .cfg
│   │   ├── MultiShardRuntime.tla + .cfg
│   │   ├── RunLifecycle.tla + .cfg
│   │   ├── ShardProcessing.tla + .cfg
│   │   └── TimerWheel.tla + .cfg
│   └── states/                                    # TLC model-checking state dumps (3 runs)
│       ├── 26-05-24-05-21-15/  (IdempotencySafety, ~100 states)
│       ├── 26-05-24-05-29-15/  (VbKyyfReplayDeterminism, ~369 states)
│       └── 26-05-24-05-34-21/  (IdempotencySafety, ~100 states)
└── verus/                                         # 44 Verus .rs proof files + 2 .md
    ├── accepted_artifact_admission_decision.rs
    ├── accepted_cli_digest_binding.rs
    ├── accepted_envelope_model.rs
    ├── accepted_run_atomic_admission.rs
    ├── admission_artifact_model.rs
    ├── budget_bounded.rs
    ├── budget_monotonic.rs
    ├── capability_artifact_model.rs
    ├── diagnostic_envelope_verus.rs
    ├── idempotency_certificate_summary.rs
    ├── idempotency_decision.rs
    ├── idempotency_replay_tracker.rs
    ├── ipc_capacity_bounds.rs
    ├── ipc_runtime_transitions.rs
    ├── ipc_strict_admission.rs
    ├── proof-review.md                              # Verus proof review document
    ├── recovery_hydration_contracts.rs
    ├── recovery_production_mapping.md               # Recovery-to-production mapping doc
    ├── recovery_verification.rs
    ├── resource_budget.rs
    ├── run_frame_invariant.rs
    ├── run_loop_termination.rs
    ├── signals_invariant.rs
    ├── signals_try_take.rs
    ├── step_budget.rs
    ├── step_state_machine.rs
    ├── strict_admission_witness.rs
    ├── taint_lattice.rs
    ├── v1_primitive_lowering.rs
    ├── value_store_invariant.rs
    ├── vb_ahfl_bounds_production.rs
    ├── vb_ahfl_graph_events_production.rs
    ├── vb_ahfl_metadata_envelope_production.rs
    ├── vb_ahfl_redaction_production.rs
    ├── vb_ahfl_ui_artifact_contract.rs
    ├── vb_cli_commands_journal_trace.rs
    ├── vb_jpq724_events_for_run_production.rs
    ├── vb_kyyf_normalization.rs
    ├── vb_oewy_bdd_runner_invariant.rs
    ├── vb_rpch_action_tracker.rs
    ├── vb_rpch_digest_check.rs
    ├── vb_rpch_hydrate_preconditions.rs
    ├── vb_rpch_replay_invariants.rs
    ├── vb_rpch_replay_refinement.rs
    ├── vb_rpch_unsupported_state.rs
    └── yaml_e2e_digest_roles.rs
```

**Verification summary**:
- **TLA+**: 22 spec files in `verification/tla/`, 5 in `verification/tla/specs/`, plus 3 TLC state dump directories
- **Verus**: 42 Rust proof files + 2 documentation files
- **Kani** (verification dir): 1 harness under `verification/kani/harnesses/`

---

## 4. KANI/ DIRECTORY (Root-level Kani harnesses)

Located at `/kani/` (repo root). Contains 20 files total:

### Root-level harnesses (13 files):
| File | Description |
|------|-------------|
| `admission_atomic_sequence_k01_k03.rs` | Admission atomic sequence verification |
| `decision_table_at_least_once_rejected.rs` | Decision table: at-least-once rejection |
| `decision_table_deterministic_rejected.rs` | Decision table: deterministic rejection |
| `decision_table_ok_branch.rs` | Decision table: OK branch |
| `decision_table_unsafe_rejected.rs` | Decision table: unsafe rejection |
| `gate_07_stack.rs` | Gate 7: stack verification |
| `gate_09_slots.rs` | Gate 9: slots verification |
| `gate_10_node.rs` | Gate 10: node verification |
| `gate_11_loop.rs` | Gate 11: loop verification |
| `gate_12_14_15.rs` | Gates 12/14/15 verification |
| `idempotency_gate_parity.rs` | Idempotency gate parity |
| `is_statically_idempotent_contract.rs` | Static idempotency contract |
| `pipeline.rs` | Pipeline verification |

### Idempotency verification harnesses (6 files):
| File | Description |
|------|-------------|
| `verify_idempotency_all_clean.rs` | All-clean idempotency case |
| `verify_idempotency_missing_key.rs` | Missing key rejection |
| `verify_idempotency_random_in_key.rs` | Random-in-key recovery |
| `verify_idempotency_secret_in_key.rs` | Secret-in-key recovery |
| `verify_idempotency_single_error.rs` | Single error case |
| `verify_idempotency_time_in_key.rs` | Time-in-key recovery |

### Bead-specific subdirectories (2):
- `vb-qi37.14.1/` (6 harnesses): `step_once_bounds`, `step_once_error`, `step_once_pc_bounds`, `step_once_slot_init`, `step_once_state_mapping`, `taint_validity`
- `vb-qi37.7.3/` (1 harness): `validation_proofs`

**Total Kani harnesses**: ~26 files across root and subdirectories.

---

## 5. CONTRACTS/ DIRECTORY

```
contracts/
├── accepted_artifacts.cue          # CUE schema for accepted artifacts
├── cli_envelope.cue                # CUE schema for CLI envelope
├── cli_envelope_instance.cue       # CUE instance
├── diagnostics.cue                 # CUE schema for diagnostics
├── evidence_bundle.cue             # CUE schema for evidence bundle
├── gate_output.cue                  # CUE schema for gate output
├── invariants.yaml                 # Contract invariants in YAML
├── manifest.cue                    # CUE manifest
├── perf-budget.yaml                # Performance budget contract
├── proof_obligations.yaml          # Proof obligations contract
├── ui_tokens.cue                   # CUE schema for UI tokens
├── ui_tokens_instance.cue          # CUE instance
├── tla/
│   └── ContractsAsData.tla + .cfg  # TLA+ contracts-as-data spec
└── verus/
    ├── contracts_as_data_spec.rs              # Verus contracts-as-data spec
    └── vb_qi37_16_5_lifecycle_journal_storage.rs  # Lifecycle journal storage Verus spec
```

**Contract formats**: CUE (8 files), YAML (2 files), TLA+ (1 file), Verus/Rust (2 files)

---

## 6. SPECS/ DIRECTORY

```
specs/
├── admission_header_before_ack.tla + .cfg     # Admission ordering spec
├── AskAnswerLifecycle.tla + .cfg              # Ask/answer lifecycle spec
├── idempotency_gate/
│   └── IdempotencyGate.tla + .cfg             # Idempotency gate spec
├── LifecycleJournal.tla + .cfg                 # Lifecycle journal spec
├── ResumeStateMachine.tla + .cfg               # Resume state machine
├── RetryFSM.tla + .cfg (+ _test.cfg)           # Retry FSM spec
├── RetryJournal.tla + .cfg (+ _test.cfg)       # Retry journal spec
├── tla/                                        # Additional TLA+ specs
│   ├── AttemptTracking.tla + .cfg
│   ├── BoundedAdmission.tla + .cfg
│   ├── BudgetArithmetic.tla + .cfg
│   ├── JournalBeforeDispatch.tla + .cfg
│   ├── RecoveryFrameHydration.tla + .cfg
│   ├── RecoveryReplay.tla + .cfg
│   ├── RecoveryReplayFull.tla + 2 .cfgs        # ~232 lines, 6 invariants, 144k+ states checked
│   ├── ShardOwnership.tla + .cfg
│   ├── ShardScheduler.tla + .cfg
│   ├── StepState.tla + .cfg
│   └── TaintLattice.tla + .cfg
└── vb_qi37_2_5/
    ├── BoundednessSlice.tla + .cfg
    └── NestedBoundednessAdmission.tla + .cfg
```

**TLA+ specs**: 20 `.tla` files across `specs/`

---

## 7. EVIDENCE/ DIRECTORY

```
evidence/
├── benchmark-evidence.jsonl                           # Benchmark evidence data
├── benchmark-logs/                                    # 3 benchmark log files
│   ├── bench_engine_step_once_save_const_single_transition.log
│   ├── engine_run_until_blocked_budget_10_small_workflow.log
│   └── ipc_frame_decode.log
├── proof-evidence.md                                  # Proof evidence document
├── proof-writer-report.md                             # Proof writer report
└── specs/
    ├── proof-writer-report.md                         # Proof writer report for specs
    └── RecoveryReplayFull.tla + .cfg                  # Recovery replay full spec
```

---

## 8. PROOF-*.MD FILES (Root level)

| File | Description |
|------|-------------|
| `proof-review-kani-god-rule-1.md` | Kani GOD RULE 1 audit — hardcoded shape review. 7 CRITICAL fixed, 5 GOOD, overall REJECTED→FIXES APPLIED→PENDING KANI EXECUTION |
| `proof-review.md` | vb-rpch proof review — APPROVED. RecoveryReplayFull.tla with 6 invariants, 144k+ TLC states |
| `proof-writer-report.md` | TLA+ repair report — fixed 3 semantic defects in RecoveryReplayFull.tla |
| `proof-repair-guide.md` | Proof repair guide for vb-rpch — 4 repair steps for Verus annotations |
| `proof-findings.jsonl` | Machine-readable proof findings |
| `kani-list.json` | Kani harness listing |
| `verification-ledger.jsonl` | Verification ledger |

### Additional verification root files:
| File | Description |
|------|-------------|
| `formal-verification-report.md` | Formal verification report |
| `final-verification-report.md` | Final verification report |
| `contract-verification-review.md` | Contract verification review |
| `moon-rust-verification.yml` | Moon CI verification tasks |
| `diagnostic_envelope_verus` | (Symlink or artifact, content TBD) |
| `recovery_hydration_contracts` | (Symlink or artifact, content TBD) |
| `recovery_verification` | (Symlink or artifact, content TBD) |
| `vb_ahfl_ui_artifact_contract` | (Symlink or artifact, content TBD) |

---

## 9. BIG-ASS-TESTING-TO-FIX.md

**239 lines** — Comprehensive 4-round audit of all velvet-ballistics crates.

### Key findings:
- **4 rounds × 12 agents = 48 subagent reviews**
- **Cumulative totals across all 4 rounds**:
  - 50 LETHAL findings
  - 15 APPROVED crate reviews
  - 29 REJECTED crate reviews
  - 80+ CRITICAL GAPS

### 8 MUST_FIX LETHALs (blocking shipping):
1. validate_taint SecretResultLeak rejection (Section 47)
2. AND/OR short-circuit (Section 46)
3. F64 arithmetic contradiction (Section 46)
4. tick_shard missing (Section 30)
5. bounded action completion queue missing (Section 4)
6. ui command missing (Section 33)
7. journal_event fuzz target missing (DRIFT-2)
8. SlotWritten-before-PC-advance untested (DRIFT-2)

### 6 SHOULD_FIX quality degradations:
1. ArrayQueue vs channel (Section 50)
2. trybuild silent pass (Section 36)
3. 11/11 property tests missing (Section 38)
4. ~24/40 benchmarks missing (Section 39)
5. $attempt.number restriction not implemented
6. SideEffect/RetrySafety enum mismatch

### Forbidden pattern violations (Round 4):
- 4 crates missing `#![forbid(unsafe_code)]`: vb_benchmark, vb_boundary_inventory, vb_proof_kernels, workspace_tests
- 7 crates with 418+ `expect()` in production
- 7 crates with 518+ `unwrap()` in production
- 5 crates CLEAN: vb_cli, vb_doc, vb_ui_makepad, vb_ui_snapshot, vb_yaml

---

## 10. VELVET-BALLISTICS-MASTER.md

**5,828 lines** — Authoritative build plan, lifecycle tracker, architecture contract.

### Testing mandates referenced:
- **Section 36**: Statement + branch + path coverage; all helper functions exercised; error paths exercised
- **Section 37**: Fuzz for IR deserialization, YAML/JSON parse, expression evaluation, IPC encoding, collect_page pagination
- **Section 38**: 11 property tests — constant_folding, bytecode_ast_parity, digest_stability, layout_stability, bound_enforcement, for_each_ordering, taint_propagation, arithmetic_overflow, concurrency_safety, resource_budget, error_recovery
- **Section 39**: 22 benchmark groups — expression_eval, IR_traversal, YAML_parse, collect_page, action_dispatch, SlotWritten_write, codegen, validation, IPC_send, IPC_throughput, compile_throughput, evaluate_throughput, memory_footprint, cold_start, warm_throughput, pagination_cost, action_queuing, timer_wheel_tick, snapshot_save/restore, digest_computation, taint_check, budget_enforcement
- **Section 46**: 10 helpers (empty, unique, contains, starts_with, ends_with, has, append, append_if, merge, sum); no short-circuit; no F64 evaluation
- **Section 47**: Taint passed through Finish; no rejection of Secret/DerivedFromSecret
- **Section 50**: ArrayQueue for IPC (lock-free SPSC); crossbeam_channel FORBIDDEN
- **Section 66**: Definition of Done — all property tests pass, all fuzz pass, all benchmarks pass, coverage >= threshold

### Mandatory tooling:
Formatting/linting, test runners (cargo test, nextest, miri, mutants, llvm-cov), property/fuzz (proptest, cargo-fuzz, arbitrary, trybuild, insta), feature matrix (cargo hack), advisory reports (audit, deny, vet, geiger, machete, semver-checks, public-api, bloat), performance (criterion, iai-callgrind, flamegraph, samply/perf, hyperfine, valgrind), nightly/dynamic verification (miri, sanitizers, coverage).

---

## 11. FUZZ/ DIRECTORY

### Fuzz target source files (16 targets):
| File | Target |
|------|--------|
| `check_doc_taint_consistency_accepts_arbitrary_markdown.rs` | Doc taint consistency |
| `decode_record.rs` | Record decoding |
| `expr_eval.rs` | Expression evaluation |
| `journal_event.rs` | Journal event decoding |
| `lex_expr.rs` | Expression lexing |
| `ui_redaction_artifact.rs` | UI redaction |
| `vb_5xs4_generated_source_mapping.rs` | Generated source mapping |
| `vb_5xs4_inventory_report.rs` | Inventory report |
| `vb_5xs4_label_sufficiency.rs` | Label sufficiency |
| `vb_5xs4_scan_source_text.rs` | Source text scanning |
| `vb_f04l_yaml_compiler_compile.rs` | YAML compiler compile |
| `vb_storage_codec.rs` | Storage codec |
| `fuzz_targets.rs` | (Combined target module) |
| `src/lib.rs` | Fuzz library code |

### Issues noted (from BIG-ASS-TESTING):
- `generated_compare` is a STUB (deserializes but discards all results)
- `compiled_ir`, `ipc_frame`, `expression` fuzz targets discard decode results
- `decode_record` uses `.ok()` suppressing all failures
- `collect_page` pagination fuzz target MISSING entirely
- Zero corpus entries for compiled_ir/generated_compare/ipc_frame/expression
- `journal_event.rs` EXISTS now (was a LETHAL-7 blocker, now present)

---

## 12. WORKSPACE_TESTS CRATE

Comprehensive integration test crate at `crates/workspace_tests/`:

- **18 benchmark files** under `benches/` covering: action_dispatch, action_queuing, array_queue, cold_start, collect_page, ir_traversal, memory_footprint, pagination_cost, rtrb, snapshot_restore, snapshot_save, timer_wheel_tick, and bead-specific benches
- **100+ test files** under `tests/` covering: BDD scenarios, contracts integration, diagnostic code ranges, compile-codegen-runtime E2E, storage recovery integration, admission integration/proptests, CLI behavior tests, boundary inventory contracts, document reconciliation, canonical spelling validation, quality gates, and more
- **Library modules** for acceptance catalog, BDD runner, boundary inventory, quality/test-loop-inventory

---

## 13. CRATE PRODUCTION TEST COVERAGE

- **562 test functions** found across all crates (grep for `#[test]`)
- **20 crates** under `crates/`: vb_core, vb_yaml, vb_validate, vb_expr, vb_compile, vb_storage, vb_runtime, vb_ipc, vb_codegen, vb_cli, vb_benchmark, vb_ui, vb_ui_model, vb_ui_snapshot, vb_ui_makepad, vb_doc, vb_boundary_inventory, vb_proof_kernels, vb_verification, workspace_tests

---

## SUMMARY

| Artifact Category | Count | Status |
|------------------|-------|--------|
| Test Plans (MD) | 16 | All documented |
| Test Reviews (MD) | 19 | All documented |
| Test Suite Reviews (MD) | 5 | All documented |
| TLA+ Specs | 43 | Across verification/tla (22), specs/ (20), contracts/tla (1) |
| Verus Proofs | 44 | Under verification/verus/ |
| Kani Harnesses | 26 | Under kani/ (root) + verification/kani/ |
| Domain Contracts | 13 | CUE (8), YAML (2), TLA+ (1), Verus/Rust (2) |
| Fuzz Targets | 16 | Under fuzz/fuzz_targets/ |
| Benchmark Files | 18 | Under workspace_tests/benches/ |
| Integration Tests | 100+ | Under workspace_tests/tests/ |
| Crate Test Functions | 562 | Across all crates |
| Proof Documents (MD) | 10 | Root-level proof/fuzz verification reports |
| Evidence Files | 6 | Under evidence/ |
| BIG-ASS-TESTING Audits | 4 rounds, 48 reviews | 50 LETHALs found, 8 MUST_FIX |

### Key Risks & Gaps (from BIG-ASS-TESTING audit):
- **8 MUST_FIX LETHALs** blocking shipping
- **50 total LETHAL findings** across 4 rounds
- **25 LETHALs in Round 4 alone** (most recent)
- Fuzz infrastructure has stubs discarding results
- CI gates insufficient (only 3/25 LETHALs caught by CI)
- Property tests mostly missing (11/11 required, some are empty placeholders)
- ~24/40 benchmarks missing
- 418+ `expect()` and 518+ `unwrap()` calls in production across 7 crates
