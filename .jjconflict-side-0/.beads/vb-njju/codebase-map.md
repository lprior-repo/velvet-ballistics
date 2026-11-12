# vb-njju State 2 codebase map

Bead: `vb-njju` — `bdd: Mutation fuzz and property coverage closure scenarios`.

Workspace evidence:
- Manifest read: `/home/lewis/src/femdation-vb-njju/.beads/vb-njju/dispatch-manifest-state2-explore-1.json` lines 1-18 declares isolated workdir `/home/lewis/src/femdation-vb-njju`, source checkout `/home/lewis/src/velvet-ballistics`, and required outputs.
- State read: `/home/lewis/src/femdation-vb-njju/.beads/vb-njju/STATE.md` lines 21-32 records isolation proof and State 2 explore routing.
- Baseline read: `/home/lewis/src/femdation-vb-njju/.beads/vb-njju/baseline-report.md` lines 10-21 records pre-edit baseline and no code/test/proof/config edits before State 2.
- Bead source read command from isolated workdir: `bd --db "/home/lewis/src/velvet-ballistics/.beads" show vb-njju`; output states acceptance tests `test_mutation_gate_fails_when_admission_branch_removed`, `test_fuzz_smoke_runs_yaml_ipc_journal_compiled_ir_targets`, `test_property_gate_fails_when_generated_ir_comparison_ignores_taint`, and `test_unsafe_boundary_fuzz_missing_causes_release_gate_failure`.

Master contract evidence:
- `/home/lewis/src/femdation-vb-njju/velvet-ballistics-MASTER.md` lines 1440-1450: public API existence is not enough; completion requires tests, fuzz/property coverage, benchmark evidence where applicable, and bead closure evidence.
- `/home/lewis/src/femdation-vb-njju/velvet-ballistics-MASTER.md` lines 1532-1543: required fuzz harnesses are `yaml_events`, `expression`, `ipc_frame`, `journal_event`, and `compiled_ir` under `fuzz/src/bin/*.rs`.
- `/home/lewis/src/femdation-vb-njju/velvet-ballistics-MASTER.md` lines 1546-1562: required proptest coverage includes constant folding, bytecode/AST parity, digest stability, layout stability, replay determinism, snapshot equivalence, ordering invariants, bound enforcement, state machine, and taint safety.
- `/home/lewis/src/femdation-vb-njju/velvet-ballistics-MASTER.md` lines 1619-1667: required gates include `coverage`, `mutants-smoke`, `fuzz-smoke`, `miri`, and `cargo fuzz build`.
- `/home/lewis/src/femdation-vb-njju/velvet-ballistics-MASTER.md` lines 4599-4604 and 4648-4657: AI deep closure expects scoped `cargo mutants`, `cargo llvm-cov`, and `cargo fuzz build` lanes.

Existing BDD/catalog surfaces:
- `/home/lewis/src/femdation-vb-njju/crates/workspace_tests/src/acceptance_catalog.rs` lines 5-20 defines the `Scenario` row shape with Given/When/Then, public surface, fixture, expected outcome/error, related bead, evidence target, and deferred follow-up.
- `/home/lewis/src/femdation-vb-njju/crates/workspace_tests/src/acceptance_catalog.rs` lines 287-303 has `VB-BDD-CATALOG-010`, a nearby quality scenario requiring executable/assertion-strong evidence through `quality::test_loop_inventory`.
- `/home/lewis/src/femdation-vb-njju/crates/workspace_tests/src/acceptance_catalog.rs` lines 310-405 validates non-empty Given/When/Then, exact assertions, public-surface/non-private requirement, isolated fixture, and evidence disposition.
- `/home/lewis/src/femdation-vb-njju/crates/workspace_tests/tests/vb_hxm0_acceptance_catalog.rs` lines 77-98 verifies catalog rows match master behaviors and writes traceability evidence for existing BDD rows.
- `/home/lewis/src/femdation-vb-njju/crates/workspace_tests/tests/vb_hxm0_acceptance_catalog.rs` lines 186-228 maps existing executable tests to covered BDD scenarios and lists deferred follow-up beads.

Mutation plan and survivor-policy surfaces:
- `/home/lewis/src/femdation-vb-njju/docs/current-api-mutation-plan.md` lines 1-4 says the document is a plan, not mutation pass evidence; full mutation/coverage evidence is owned elsewhere.
- `/home/lewis/src/femdation-vb-njju/docs/current-api-mutation-plan.md` lines 5-79 enumerates helper semantics, runtime recovery, generated parity, CLI/IPC/storage, UI model targets, owner beads, critical survivor policy, scoped mutation command, and 90% threshold.
- `/home/lewis/src/femdation-vb-njju/crates/workspace_tests/src/quality/current_api_mutation_plan.rs` lines 35-100 codifies required mutation-plan sections and required terms.
- `/home/lewis/src/femdation-vb-njju/crates/workspace_tests/tests/vb_c3k9_current_api_mutation_plan.rs` lines 7-35 validates the plan covers every required section; lines 225-233 require critical survivors to become blocker evidence or follow-up beads.

Fuzz surfaces:
- Glob evidence found `fuzz/src/bin/yaml_events.rs`, `fuzz/src/bin/ipc_frame.rs`, `fuzz/src/bin/journal_event.rs`, `fuzz/src/bin/compiled_ir.rs`, and many additional bins including `generated_compare.rs`, `storage_envelope_fuzz_boundary.rs`, `ipc_frame_fuzz_boundary.rs`, `binary_payload_fuzz_boundary.rs`, and `admission_fuzz.rs`.
- `/home/lewis/src/femdation-vb-njju/fuzz/Cargo.toml` lines 39-86 declares master-required bins `yaml_events`, `expression`, `ipc_frame`, `journal_event`, `compiled_ir`, plus `generated_compare`.
- `/home/lewis/src/femdation-vb-njju/fuzz/src/lib.rs` lines 182-189 implements `fuzz_yaml_events` over `vb_yaml` parser/profile/source-map APIs.
- `/home/lewis/src/femdation-vb-njju/fuzz/src/lib.rs` lines 191-241 implements `fuzz_ipc_frame` over public IPC header/payload decoding with length assertions.
- `/home/lewis/src/femdation-vb-njju/fuzz/src/lib.rs` lines 368-388 implements `fuzz_compiled_ir`, but only asserts successful workflows have node/slot count; this is smoke-like and not enough for generated-vs-IR taint parity closure.
- `/home/lewis/src/femdation-vb-njju/fuzz/src/lib.rs` lines 410-428 implements `fuzz_generated_compare`, asserting validation and construction agree, but it does not compare generated execution against IR output/taint.
- `/home/lewis/src/femdation-vb-njju/fuzz/src/lib.rs` lines 2210-2288 implements `fuzz_ipc_frame_boundary` with typed boundary-error checks; line 2264 uses `unwrap()` in fuzz code, which is allowed by local fuzz crate lint allowances but remains a risk for zero-unwrap governance optics.

Property coverage surfaces:
- Grep evidence found proptest usage in `crates/vb_codegen/src/proptests.rs`, `crates/vb_storage/src/proptests.rs`, `crates/vb_runtime/src/engine/tests.rs`, and many `crates/workspace_tests/tests/*.rs` files.
- `/home/lewis/src/femdation-vb-njju/crates/vb_codegen/src/proptests.rs` lines 349-376 checks generated Rust and IR match finished signals and slots for a fixed six-step workflow; scope for vb-njju should extend this to taint/typed-error evidence or add BDD gate that fails if comparison ignores taint.
- `/home/lewis/src/femdation-vb-njju/crates/vb_storage/src/proptests.rs` lines 248-310 begins recovery proptest invariants for deterministic replay.
- `/home/lewis/src/femdation-vb-njju/crates/vb_runtime/src/engine/tests.rs` lines 1482-1514 contains runtime properties for step budget and idempotency key separation.
- Grep evidence also found `crates/workspace_tests/tests/proptest_validation.rs`, `contracts_as_data_props.rs`, `vb_qi37_4_2_strict_runtime_admission.rs`, `vb_qi37_12_state8_silent_discard_contract.rs`, and `vb_core_yaml_e2e_chain_contract.rs` using `proptest!`.

Gate wiring surfaces:
- `/home/lewis/src/femdation-vb-njju/.moon/tasks/all.yml` lines 290-309 defines `coverage` as a smoke llvm-cov lane over one `vb_core` test.
- `/home/lewis/src/femdation-vb-njju/.moon/tasks/all.yml` lines 311-342 defines `mutants-smoke` as a deterministic `vb_core/src/diagnostic.rs` slice, not current touched semantic closure.
- `/home/lewis/src/femdation-vb-njju/.moon/tasks/all.yml` lines 344-371 defines `fuzz-smoke` as `cargo fuzz build` only; it builds targets but does not run hostile seeds.
- `/home/lewis/src/femdation-vb-njju/.moon/tasks/all.yml` lines 273-288 defines focused `miri` over three named tests.

Recommended delivery shape:
1. Add `vb-njju` rows to `crates/workspace_tests/src/acceptance_catalog.rs` using the existing `Scenario` struct: one mutation gate, one fuzz smoke gate, one property/generated parity gate, one unsafe-boundary-fuzz-missing release-failure gate.
2. Add a new workspace acceptance test file, likely `/home/lewis/src/femdation-vb-njju/crates/workspace_tests/tests/vb_njju_mutation_fuzz_property_closure.rs`, that drives public surfaces only: catalog validation, mutation-plan validation, Moon/fuzz manifest inspection, and proptest/fuzz target presence checks.
3. Keep production code untouched unless State 3 finds there is no public quality API to validate enough evidence; then expose a workspace-test helper rather than private crate internals.
4. Required verifier modes for implementation closure: targeted nextest for workspace_tests, targeted nextest/proptest for `vb_codegen`, `vb_storage`, `vb_runtime` if tests change there, `moon run :fuzz-smoke`, `moon run :mutants-smoke`, `moon run :coverage`, plus scoped `cargo mutants --package velvet-ballistics-workspace-tests --test vb_njju_mutation_fuzz_property_closure` when feasible.

Risks/blockers:
- Existing Moon `fuzz-smoke` only builds fuzz targets; bead acceptance says `fuzz_smoke_runs_yaml_ipc_journal_compiled_ir_targets`, so State 3 may need tests proving the intended run command/seed coverage or a harness around fuzz bin stdin execution.
- Existing Moon `mutants-smoke` mutates only `vb_core/src/diagnostic.rs`; it will not prove admission branch survivor closure. A BDD gate should fail if mutation evidence is scoped to an unrelated file.
- Existing property parity in `vb_codegen` checks finished signals/slots but not clearly taint mismatch; the bead explicitly names generated IR comparison ignoring taint.
- Some fuzz code intentionally allows `.unwrap()` and silent `drop`/early-return smoke oracles; State 3 should avoid claiming those targets as strong behavioral proof unless the BDD scenario checks typed errors/exact assertions.
- Catalog currently covers general executable/assertion-strong tests through `VB-BDD-CATALOG-010`, but no `vb-njju` scenario rows exist yet.
