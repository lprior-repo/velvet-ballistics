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

## Open Finding Buckets

- `vb_boundary_inventory`: PATCHED — registered/repaired proptest suite, replaced weak validation/API/parser/error assertions with exact variants/values, pinned required-evidence outcomes, discovery candidate counts/paths, and stable-id normalization. Evidence: `rtk cargo test -p vb_boundary_inventory` — PASS, `233 passed`.
- `vb_core` aggregate budget: PATCHED PARTIAL — current diff replaces aggregate source-token/vacuum tests and exact payload/property gaps in aggregate integration tests. Still needs any remaining internal `src/budget/tests.rs` exact-field cleanup and Kani execution proof.
- `vb_core` general: PATCHED PARTIAL — current diff strengthens `section36`/`proptest_core_types`; still needs a fresh post-patch audit of `section38`, display exactness, handle full-range domains, and Kani exact variants.
- `vb_proof_kernels`: CRC placeholder/vacuum coverage, generated-IR/replay taxonomy, exact `Policy::within`, resource budget boundaries.
- `vb_yaml`: PATCHED PARTIAL — current diff fixes exact variants/fields in lib/profile/adversarial/source-map/event tests; still needs parser/profile/source-map property/fuzz coverage.
- `vb_validate`: PATCHED PARTIAL — current diff fixes capability/idempotency/red-phase/Kani accessor exactness; still needs broad source-level `{ .. }` cleanup and hostile-input executable coverage.
- `vb_expr`: orphan eval tests, OOB exact errors, parser/lexer exact payloads, non-finite float, real generated properties, Kani shape gaps.
- `vb_compile`: full compile-chain strict YAML, exact idempotency/topology diagnostics, `compile_source` parity, primitive payload validation, nested duplicate keys.
- `vb_storage`: disabled atomic admission suite, persisted envelope assertions, exact `JournalError`/`TrimError`, trim identity/reopen durability, Kani persistence gap.
- `vb_runtime`: recovery/resume/lifecycle/timer/taint vacuum and exactness gaps, stale/latest attempt correctness, fixture silent returns, timer identity/order.
- `vb_doc`/`vb_benchmark`: tautologies, exact doc errors, patch-plan exactness, pending evidence negative, regression/metadata fields.
- `vb_ipc`: silent-pass client tests, exact nested frame errors, FIFO identity/order, payload boundaries, Kani exactness.
- `vb_cli`: lifecycle state setup, JSON parse/shape, trace order/full entries, run-step exact outputs/errors, admission exact rejection, deliver sink variants. UI-only envelope items excluded.
- `xtask`: workspace/test compile state, exact error variant coverage, scheduler set property, stdout JSON parsing.
- `workspace_tests`: non-codegen/non-UI tautologies and either-outcome tests need exact behavior or removal.
- Proof parity/Kani: hardcoded Kani shapes and Verus/TLA behavior parity gaps remain open unless backed by executable behavior tests.

## Next Round Rule

Patch one crate bucket at a time, run its scoped tests, then update this ledger from `OPEN` to `PATCHED` only after `jj diff` proves the file changes are present.
