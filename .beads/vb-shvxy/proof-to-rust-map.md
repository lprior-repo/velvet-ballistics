# Proof-to-Rust Bridge Map: vb-shvxy (State 7)

bridge_skill: proof-to-implementation
bridge_invocation_id: vb-shvxy-state7-proof-to-implementation-attempt2
bridge_state: 7
proof_review_invocation_id: vb-shvxy-state6-proof-reviewer-attempt1
proof_review_status: APPROVED

## Provenance

| Field | Value |
|---|---|
| Proof reviewer invocation | vb-shvxy-state6-proof-reviewer-attempt1 |
| This bridge invocation | vb-shvxy-state7-proof-to-implementation-attempt2 |
| Self-approval risk | None — bridge is peer of proof-reviewer, not same agent |
| Proof review artifacts | proof-review.md, proof-findings.jsonl |

## Bridge Context

This is a **tooling infrastructure bead** (`vb-shvxy`). All 11 proof obligations are `behavior_affecting: false`. The "implementation" targets are scripts, CI configuration (`moon ci`), documentation, and verifier wrapper infrastructure — not production Rust behavior. The 5 closure obligations (PO-012K through PO-012L) are deferred to State 10 (formal-verifier).

All mapping rows use `mapping_status: planned` (allowed at State 7).

## Open Decisions from proof-to-implementation-input.md

1. **Kani harness pass-through**: `scripts/kani-list.sh` remains inventory-only for State 7. Execution harnesses use direct `cargo kani` invocation in `.moon/tasks/kani.yml`. A `--harness` pass-through is not needed for inventory obligations.

2. **Missing `vb_runtime/kani-diagnostic-codes` feature**: Not restored. PO-003 correctly documented that `vb_runtime/Cargo.toml` does not declare this feature and the tooling fails closed. Existing features in `vb_core` are sufficient.

3. **Moon CI kani-list integration**: `.moon/tasks/kani.yml` currently uses direct `cargo kani --harness` calls. A new `verify-kani-inventory` moon task should invoke `scripts/kani-list.sh` to validate harness inventory non-vacuity before execution tasks run.

4. **Flux proof registry**: No separate registry needed at this stage. Package-level Flux smoke is sufficient; named Flux artifact wiring is documented in the bridge for downstream behavior obligations.

5. **Proptest guard location**: Installed at `scripts/guard-zero-tests.sh`. The latent pipefragility (`set -euo pipefail` interaction with grep) is documented as an implementation obligation for State 11.

6. **Cargo target triple**: `.cargo/config.toml` does not set a default target triple. Moon fuzz-smoke already specifies `--target x86_64-unknown-linux-gnu`. Direct fuzz commands should always specify `--target`.

7. **Loom cfg/dependency**: Loom remains a dev-dependency with `#[cfg(loom)]` gating. Integration tests pending loom dependency promotion are deferred to downstream behavior obligations.

## Proof-to-Rust Mapping Matrix

| Proof ID | Claim | Behavior Affecting | Rust Source Refs | Behavior Test Refs | Refinement Harness Refs | Verifier | Evidence Command | Rerun From |
|---|---|---|---|---|---|---|---|---|
| PO-001 | Kani inventory for vb_core (176 harnesses) | false | `scripts/kani-list.sh::kani_list_inventory`, `.moon/tasks/kani.yml::verify-kani` | `tests/tooling/script_inventory_ci_test` | `.evidence/kani-list/vb_core.json::json_validation` | kani | `bash scripts/kani-list.sh vb_core` | 7 |
| PO-002 | Kani inventory for vb_runtime (6 harnesses) | false | `scripts/kani-list.sh::kani_list_inventory`, `.moon/tasks/kani.yml::verify-kani` | `tests/tooling/runtime_script_inventory_ci_test` | `.evidence/kani-list/vb_runtime.json::json_validation` | kani | `bash scripts/kani-list.sh vb_runtime` | 7 |
| PO-003 | Kani feature gate: undeclared features fail closed | false | `scripts/kani-list.sh::KANI_FEATURES_feature_gate`, `crates/vb_runtime/Cargo.toml::kani-diagnostic-codes_feature_absent` | `tests/tooling/feature_gate_fail_closed_ci_test` | `.evidence/kani-list/feature_gate_validation::fail_closed_evidence` | kani | `KANI_FEATURES=vb_runtime/kani-diagnostic-codes bash scripts/kani-list.sh vb_runtime` | 7 |
| PO-004 | Flux-rs package check for vb_core | false | `scripts/flux-check-package.sh::flux_package_check` | `tests/tooling/package_smoke_ci_test` | `.evidence/flux-check/vb_core_smoke::flux_compilation_log` | flux-rs | `bash scripts/flux-check-package.sh vb_core` | 7 |
| PO-005 | Flux-rs unsupported selector rejection | false | `scripts/flux-check-package.sh::unsupported_selector_guard` | `tests/tooling/selector_rejection_ci_test` | `.evidence/flux-check/selector_guard::exit_2_evidence` | flux-rs | `bash scripts/flux-check-package.sh vb_core --lib` | 7 |
| PO-006 | Proptest zero-test detector fail-closed | false | `scripts/guard-zero-tests.sh::zero_test_detector` | `tests/tooling/zero_test_fail_closed_ci_test` | `.evidence/guard-zero-tests/zero_applicable::exit_1_evidence` | proptest | `bash scripts/guard-zero-tests.sh -- cargo test -p vb_core --test aggregate_resource_budget_properties_red -- nonexistent_filter_xyz` | 7 |
| PO-007 | Proptest non-vacuous execution (5 tests) | false | `scripts/guard-zero-tests.sh::applicable_count_gate`, `crates/vb_core/tests/aggregate_resource_budget_properties_red.rs::proptest_tests` | `tests/tooling/non_zero_test_ci_gate` | `.evidence/guard-zero-tests/5_applicable::exit_0_evidence` | proptest | `bash scripts/guard-zero-tests.sh -- cargo test -p vb_core --test aggregate_resource_budget_properties_red` | 7 |
| PO-008 | Cargo-fuzz target registration (58 targets) | false | `fuzz/Cargo.toml::fuzz_target_registry`, `.moon/tasks/all.yml::fuzz-smoke` | `tests/tooling/target_registry_ci_test` | `.evidence/fuzz-list/fuzz_target_inventory::58_targets_evidence` | cargo-fuzz | `cargo fuzz list` | 7 |
| PO-009 | Cargo-fuzz GNU target build | false | `.moon/tasks/all.yml::fuzz-smoke`, `.cargo/config.toml::alias-nightly-feature-cargo-probe` | `tests/tooling/target_build_ci_test` | `.evidence/fuzz-build/gnu_target_compile::all_58_compile_evidence` | cargo-fuzz | `cargo fuzz build --target x86_64-unknown-linux-gnu` | 7 |
| PO-010 | Loom model compilation (13 tests) | false | `crates/vb_runtime/src/models/loom::loom_model_tests`, `crates/vb_runtime/Cargo.toml::loom_dev_dependency` | `tests/tooling/model_compile_ci_test` | `.evidence/loom-test/13_passed_cfg_loom::compile_execute_evidence` | loom | `RUSTFLAGS="--cfg loom" cargo test -p vb_runtime --lib -- models::loom` | 7 |
| PO-011 | Loom model enumeration (5 models) | false | `scripts/loom-list.sh::loom_model_enumeration`, `xtask/src/loom.rs::LOOM_MODELS_const_array` | `tests/tooling/model_enumeration_ci_test` | `.evidence/loom-list/5_models::enumeration_evidence` | loom | `bash scripts/loom-list.sh` | 7 |

## State 8–12 Roadmap

### State 8: Test Planning (`test-planner`)
- Plan behavior tests for each tooling script and CI integration point
- Coverage targets:
  - `scripts/kani-list.sh`: inventory JSON validity, feature gate fail-closed, missing package handling
  - `scripts/flux-check-package.sh`: package smoke pass, unsupported selector rejection, missing args
  - `scripts/guard-zero-tests.sh`: zero-test rejection, non-zero acceptance, unparseable output, cargo test failure passthrough
  - `scripts/loom-list.sh`: model enumeration, xtask failure handling, empty model list
  - Fuzz lane: target registration, GNU build smoke
  - Loom lane: cfg compilation, model execution
- Each test must be independent of verifier harnesses

### State 9: Test Writing (`test-writer`)
- Write failing-first behavior tests (bash-based, runnable in CI)
- Key test patterns:
  - Exit-code assertions
  - Output format validation
  - Error message pattern matching
  - Non-vacuous count assertions
- Tests may use a simple bash test framework or direct `bash -c` invocations

### State 10: Test Review (`test-reviewer`)
- Adversarial review of behavior tests
- Enforce: sharp assertions, deterministic execution, public interface testing
- Reject: tests that depend on verifier harness execution, tests without count assertions

### State 11: Implementation (`holzman-rust`)
- Fix `scripts/guard-zero-tests.sh` pipefragility (FIND-SHVXY-001)
- Integrate scripts into `.moon/tasks/`:
  - Add `verify-kani-inventory` task to `.moon/tasks/kani.yml`
  - Add `verify-flux-smoke` task to `.moon/tasks/` (new flux.yml or all.yml)
  - Add `verify-proptest-non-vacuous` task referencing `guard-zero-tests.sh`
  - Add `verify-fuzz-inventory` task to `.moon/tasks/all.yml`
  - Add `verify-loom-compile` task to `.moon/tasks/all.yml`
- Create `.moon/tasks/verifier-tooling.yml` for consolidated pre-commit verifier readiness
- Document feature-gate behavior in script headers
- Ensure all scripts have execute permissions (`chmod +x`)
- Track all untracked files from FIND-SHVXY-002 into git

### State 12: Formal Closure (`formal-verifier`)
- Re-run all evidence commands fresh
- Classify evidence: SetupHealth, Inventory, BehaviorProof (none for this bead)
- Enforce `applicable_count > 0` for all non-vacuous evidence
- Close verification-ledger.jsonl with PASS rows for all 16 obligations
- Resolve trusted-based-ledger pending dispositions (TB-006, TB-007, TB-008)
- Generate `formal-verification-report.md` and `refinement-verification-report.md`
- Produce `proof-test-source-alignment.md` with final parity matrix

## Bridge Review Handoff

- Artifacts for `proof-reviewer`:
  - `.beads/vb-shvxy/proof-to-rust-map.md` (this file)
  - `.beads/vb-shvxy/rust-refinement-obligations.jsonl`
  - `.beads/vb-shvxy/proof-review.md` (input, State 6 APPROVED)
- Review criteria (from `bridge-review-rubric.md`):
  - No file-only source refs
  - All behavior-affecting rows have independent behavior tests (N/A — all rows are `behavior_affecting: false`)
  - No verifier harness reused as behavior test
  - Every row has a refinement harness ref separate from behavior test refs
  - No behavior waivers
  - Every row has an evidence path

## Unresolved Mapping Gaps

None. All 16 proof obligations (PO-001 through PO-012L) have corresponding bridge rows. The 5 closure obligations (PO-012K through PO-012L) are deferred to State 10 with `mapping_status: planned`. No behavior-affecting proofs exist in this bead; all evidence is inventory/tooling-smoke.
