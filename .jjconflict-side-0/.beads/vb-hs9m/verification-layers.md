# Verification Layers — vb-hs9m

## Layer Assignment Summary

| Contract Clause | Primary Layer | Secondary Layer | Waiver |
|-----------------|--------------|-----------------|--------|
| INV-001 (TraceRing boundedness) | `kani` | `unit-test` | — |
| INV-001 (TraceRing FIFO) | `unit-test` (BDD adversarial) | `kani` | — |
| INV-001 (TraceRing dropped monotonic) | `unit-test` | `kani` | — |
| INV-001 (has_terminal_event_for_run stability) | `unit-test` | `kani` | — |
| PRE-001 (capacity > 0) | `verus` spec fn | `unit-test` | — |
| POST-002 (push returns bool, dropped incr) | `kani` | `unit-test` | — |
| POST-003 (drain empties ring) | `unit-test` | `kani` | — |
| POST-004 (drain_for_run filter) | `unit-test` | `kani` | — |
| POST-005 (has_terminal_event_for_run) | `unit-test` | `kani` | — |
| INV-002 (EvidenceBundle required fields) | `kani` (OBL-002) | `unit-test` | — |
| PRE-004 (parse accepts any &str) | `kani` (OBL-001) | `unit-test` | — |
| POST-006 (validate returns empty on valid) | `kani` (OBL-002) | `unit-test` | — |
| POST-007 (parse format validation) | `kani` (OBL-001) | `unit-test` | — |
| POST-008 (round-trip Yaml) | `proptest` (OBL-005) | `kani` | — |
| POST-008 (round-trip JSON) | `proptest` (OBL-006) | `kani` | — |
| POST-008 (round-trip Postcard) | `proptest` (OBL-007) | `miri` (OBL-008) | — |
| INV-003 (Scenario uniqueness) | `unit-test` | — | — |
| INV-003 (Scenario non-empty fields) | `unit-test` | — | — |
| POST-009 (catalog non-empty) | `unit-test` | — | — |
| POST-010 (validate_catalog) | `integration-test` | `unit-test` | — |
| INV-004 (evidence_path format) | `unit-test` | — | — |
| ERR variants (Error taxonomy) | `unit-test` | — | — |

---

## Defense-in-Depth: TraceRing

### Layer 1 — Kani (Proof, Bounded Model Check)
- **Target:** `crates/vb_runtime/src/trace.rs`
- **Claim:** `len() <= capacity`, `dropped` non-decreasing, FIFO ordering, `has_terminal_event_for_run` correctness
- **Harness:** `kani/verify_trace_ring_bounds.rs` (referenced from codebase-map, exists in kani/ directory)
- **Command:** `cargo kani --harness verify_trace_ring_bounds --tests`
- **Expected evidence:** Kani reports 0 model-checking failures for all traced functions

### Layer 2 — Unit Test (BDD Adversarial)
- **Target:** `crates/vb_runtime/src/trace.rs`
- **Claim:** Adversarial overflow, concurrent drain/flush, terminal event detection
- **Command:** `cargo test --package vb_runtime -- trace --`
- **Expected evidence:** All 1077+ trace tests pass

### Layer 3 — Miri (UB Check)
- **Target:** `crates/vb_runtime/src/trace.rs`
- **Claim:** No undefined behavior in trace operations
- **Command:** `cargo +nightly miri test --package vb_runtime -- trace`
- **Expected evidence:** Miri reports 0 UB violations

---

## Defense-in-Depth: EvidenceBundle

### Layer 1 — Kani (Proof, Panic Freedom)
- **Target:** `xtask/tests/bundle_tests.rs`
- **Claim:** OBL-001: `parse_bundle_schema_version` never panics on arbitrary `String`
- **Command:** `cargo kani --harness schema_version_parse_non_panic`
- **Expected evidence:** Kani reports 0 panics

### Layer 2 — Kani (Proof, Validation Correctness)
- **Target:** `xtask/tests/bundle_tests.rs`
- **Claim:** OBL-002: `validate_bundle` returns empty vec iff all required fields non-empty
- **Command:** `cargo kani --harness validator_correctness`
- **Expected evidence:** Kani proves validation correctness for all `EvidenceBundle` inputs

### Layer 3 — Kani (Proof, I/O Safety)
- **Target:** `xtask/tests/bundle_tests.rs`
- **Claim:** OBL-003: `write_bundle` and `read_bundle` never panic
- **Command:** `cargo kani --harness write_read_non_panic`
- **Expected evidence:** Kani reports 0 panics

### Layer 4 — Proptest (Property, Round-Trip)
- **Target:** `xtask/tests/bundle_tests.rs`
- **Claim:** OBL-005: YAML round-trip identity; OBL-006: JSON round-trip; OBL-007: Postcard round-trip
- **Command:** `cargo test --test bundle_tests -- --test-threads=1 round_trip`
- **Expected evidence:** All round-trip property tests pass for 1000 iterations

### Layer 5 — Miri (UB Check)
- **Target:** `xtask/tests/bundle_tests.rs`
- **Claim:** OBL-008: No UB in postcard serialization
- **Command:** `cargo +nightly miri test --test bundle_tests`
- **Expected evidence:** Miri reports 0 UB violations

---

## Defense-in-Depth: Scenario/Catalog

### Layer 1 — Unit Test (Catalog Validation)
- **Target:** `crates/workspace_tests/src/acceptance_catalog.rs`
- **Claim:** All catalog validation errors are triggered correctly for invalid inputs
- **Command:** `cargo test --package workspace_tests validate_catalog`
- **Expected evidence:** All catalog validation unit tests pass

### Layer 2 — Integration Test (BDD Catalog Gate)
- **Target:** `crates/workspace_tests/tests/vb_hxm0_acceptance_catalog.rs`
- **Claim:** `test_catalog_lists_every_master_doc_behavior_by_scenario_id`, `test_catalog_maps_existing_tests_to_covered_scenarios`, `test_catalog_gate_fails_when_behavior_has_no_scenario`, `test_catalog_gate_fails_when_scenario_has_no_test_target`
- **Command:** `cargo test --test vb_hxm0_acceptance_catalog`
- **Expected evidence:** All 4 catalog gate tests pass

---

## Defense-in-Depth: Persistence/Evidence Path

### Layer 1 — Unit Test (Path Construction)
- **Target:** `xtask/src/evidence/persistence.rs`
- **Claim:** `evidence_path` and `bundle_path` return correctly formatted paths
- **Command:** `cargo test --package xtask evidence::persistence::tests`
- **Expected evidence:** All path construction tests pass

### Layer 2 — Unit Test (Write/Read Cycle)
- **Target:** `xtask/src/evidence/persistence.rs`
- **Claim:** Evidence written to path can be read back
- **Command:** `cargo test --package xtask evidence::persistence::integration`
- **Expected evidence:** All evidence I/O integration tests pass

---

## Verus Scope

**No Verus proof obligations for vb-hs9m.** The existing `verification/verus/run_frame_invariant.rs` and `verification/verus/signals_invariant.rs` are pre-existing artifacts outside this bead's scope. They are referenced for context only.

---

## TLA+ Scope

**No TLA+ obligations for vb-hs9m** (see `tla-spec.md` for full non-applicability rationale).

---

## Theorem Scope

**No Lean/Aeneas/Hax obligations for vb-hs9m** (see `lean-contract.md` for full waiver rationale).

---

## Waivers

| Clause ID | Waiver Reason | Compensating Evidence | Owner |
|----------|---------------|----------------------|-------|
| TLA+ overall | No temporal/protocol/workflow/state-over-time behavior in bead scope | Kani + unit tests + integration tests | rust-contract state 3 |
| Lean/Aeneas/Hax | No algebraic theorem kernel requires proof-assistant extraction | Kani + proptest + unit tests | rust-contract state 3 |
| Verus run_frame_invariant | Pre-existing artifact, not authored by vb-hs9m | Existing Verus proof in `verification/verus/` | pre-existing |
| Verus signals_invariant | Pre-existing artifact, not authored by vb-hs9m | Existing Verus proof in `verification/verus/` | pre-existing |
