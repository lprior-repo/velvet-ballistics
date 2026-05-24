# Test Quality Audit Progress Ledger

Bead: `vb-umoy`
Workspace: `/home/lewis/src/vb-umoy-test-audit-gpt55`

This ledger is the durable audit checkpoint between agent rounds. Beads remain the issue tracker; this file records evidence and coverage state so minor findings are not dropped.

## Scope Policy

- Fix every behavior-test weakness, including minor exact-field/assertion gaps.
- Exclude `vb_codegen` and codegen-specific workspace tests; codegen should move out.
- Exclude UI-only/deferred `vb_ui_model` coverage; UI should move out.
- Treat any subagent change as untrusted until it appears in `jj diff` in this workspace.

## Current Persisted Checkpoint

Verified after latest multi-subagent reconciliation, current persisted patch set covers these buckets:

- `crates/vb_boundary_inventory/src/tests/api_tests.rs`
- `crates/vb_boundary_inventory/src/tests/error_tests.rs`
- `crates/vb_boundary_inventory/src/tests/parser_tests.rs`
- `crates/vb_boundary_inventory/src/tests/property_tests.rs`
- `crates/vb_boundary_inventory/src/tests/validation_tests.rs`
- `crates/vb_core/tests/aggregate_resource_budget_kani_red.rs`
- `crates/vb_core/tests/aggregate_resource_budget_properties_red.rs`
- `crates/vb_core/tests/aggregate_resource_budget_red.rs`
- `crates/vb_core/tests/aggregate_resource_budget_snapshot_red.rs`
- `crates/vb_core/tests/proptest_core_types.rs`
- `crates/vb_core/tests/section36_mandatory_coverage.rs`
- `crates/vb_validate/src/kani_gate_08_accessor.rs`
- `crates/vb_validate/tests/capability_schema_kani.rs`
- `crates/vb_validate/tests/idempotency_contract_red.rs`
- `crates/vb_validate/tests/red_phase_validation.rs`
- `crates/vb_yaml/src/events_tests.rs`
- `crates/vb_yaml/src/lib_tests.rs`
- `crates/vb_yaml/src/profile_error_variants_tests.rs`
- `crates/vb_yaml/src/profile_tests.rs`
- `crates/vb_yaml/src/profile_tests_adversarial.rs`
- `crates/vb_yaml/src/source_map_tests.rs`

Scoped verification after reconciliation:

- `rtk cargo test -p vb_boundary_inventory` — PASS, `233 passed`.
- `rtk cargo test -p vb_core --test aggregate_resource_budget_red --test aggregate_resource_budget_properties_red --test aggregate_resource_budget_snapshot_red --test aggregate_resource_budget_kani_red --test section36_mandatory_coverage --test proptest_core_types` — PASS, `153 passed`.
- `rtk cargo test -p vb_validate` — PASS, `963 passed`.
- `rtk cargo test -p vb_yaml` — PASS, `232 passed`.
- `rtk cargo test -p velvet-ballastics --test vb_qi37_14_1_run_step --test cli_trace_integration --test cli_vb_m214_bdd_scenarios --test lifecycle_integration --test deliver_sink_integration --test admission_evidence_integration -- --test-threads=1` — PASS, `155 passed`.
- `cargo test -p vb_proof_kernels` — PASS, `231 passed`.
- `cargo test -p vb_expr` — PASS, `649 passed`.
- `cargo test -p vb_storage` — PASS, `1126 passed`.
- `rtk cargo test -p vb_runtime` — PASS, `1816 passed`.
- `cargo test -p vb_doc -p vb_benchmark` — PASS, `117 passed`.
- `cargo test -p vb_ipc --lib` — PASS, `691 passed`.
- `rtk cargo test -p vb_compile` — PASS, `290 passed`.
- `rtk cargo check -p vb_ipc --lib` — PASS after symbolic IPC Kani harness edit.
- `cargo kani -p vb_ipc --harness kani_ipc_header_rejects_oversize_payload` — BLOCKED by unrelated existing `vb_core` Kani compile errors before target harness.
- `cargo test -p vb_yaml -p vb_validate` — PASS, `1196 passed` after residual exactness cleanup.

## Open Finding Buckets

- `vb_boundary_inventory`: PATCHED — registered/repaired proptest suite, replaced weak validation/API/parser/error assertions with exact variants/values, pinned required-evidence outcomes, discovery candidate counts/paths, and stable-id normalization. Evidence: `rtk cargo test -p vb_boundary_inventory` — PASS, `233 passed`.
- `vb_core` aggregate budget: PATCHED PARTIAL — current diff replaces aggregate source-token/vacuum tests and exact payload/property gaps in aggregate integration tests. Still needs any remaining internal `src/budget/tests.rs` exact-field cleanup and Kani execution proof.
- `vb_core` general: PATCHED PARTIAL — current diff strengthens `section36`/`proptest_core_types`; still needs a fresh post-patch audit of `section38`, display exactness, handle full-range domains, and Kani exact variants.
- `vb_proof_kernels`: PATCHED — removed false CRC/vacuous header tests and added generated-IR/replay taxonomy, exact `Policy::within`, and resource-budget saturation coverage. Evidence: `cargo test -p vb_proof_kernels` — PASS, `231 passed`.
- `vb_yaml`: PATCHED PARTIAL — current diff fixes exact variants/fields in lib/profile/adversarial/source-map/event tests, including residual event/span/count assertions. Still needs parser/profile/source-map property/fuzz coverage.
- `vb_validate`: PATCHED PARTIAL — current diff fixes capability/idempotency/red-phase/Kani accessor exactness plus residual missing-field exactness. Still needs broad source-level `{ .. }` cleanup and hostile-input executable coverage.
- `vb_expr`: PATCHED PARTIAL — exact OOB/type/parser/lexer payloads and generated expression properties patched. Evidence: `cargo test -p vb_expr` — PASS, `649 passed`. Kani shape gaps remain open under proof parity.
- `vb_compile`: PATCHED PARTIAL — current diff hardens idempotency parity, error variant, canonical YAML diagnostic, and secret-finish IR shape tests. Evidence: `rtk cargo test -p vb_compile` — PASS, `290 passed`. Full strict-YAML compile-chain and nested duplicate-key gaps still need post-patch audit.
- `vb_storage`: PATCHED PARTIAL — current diff hardens atomic admission and trim tests plus persisted accepted-envelope readback. Evidence: `cargo test -p vb_storage` — PASS, `1126 passed`. Kani persistence gap remains open under proof parity.
- `vb_runtime`: PATCHED PARTIAL — current diff hardens lifecycle attempt/state/journal assertions, timer identity/order triples, and recovery slot/taint/error assertions. Evidence: `rtk cargo test -p vb_runtime` — PASS, `1816 passed`. Remaining broad runtime recovery/resume/tick-shard gaps require post-patch audit.
- `vb_doc`/`vb_benchmark`: PATCHED PARTIAL — current diff removes doc tautologies and hardens doc errors/patch plans plus benchmark regression/metadata fields. Evidence: `cargo test -p vb_doc -p vb_benchmark` — PASS, `117 passed`.
- `vb_ipc`: PATCHED PARTIAL — current diff hardens server/helper/IPC exactness and selected protocol tests. Evidence: `cargo test -p vb_ipc --lib` — PASS, `691 passed`. Kani exactness remains open under proof parity.
- `vb_cli`: PATCHED PARTIAL — current parent commit hardens non-UI/non-codegen lifecycle, trace, run-step, admission, and deliver-sink tests. UI-only envelope items excluded. Still needs post-patch audit for remaining CLI smoke/either-outcome tests outside touched targets.
- `xtask`: workspace/test compile state, exact error variant coverage, scheduler set property, stdout JSON parsing.
- `workspace_tests`: non-codegen/non-UI tautologies and either-outcome tests need exact behavior or removal.
- Proof parity/Kani: OPEN PARTIAL — current diff symbolically hardens `crates/vb_ipc/src/kani_ipc_header_rejects_oversize.rs` with exact payload-limit variants and covers, but Kani execution is blocked by unrelated existing `vb_core` Kani compile errors. Other hardcoded Kani shapes and Verus/TLA behavior parity gaps remain open.

## Next Round Rule

Patch one crate bucket at a time, run its scoped tests, then update this ledger from `OPEN` to `PATCHED` only after `jj diff` proves the file changes are present.
