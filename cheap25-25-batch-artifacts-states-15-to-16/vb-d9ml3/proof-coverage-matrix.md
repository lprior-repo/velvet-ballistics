# Proof Coverage Matrix — vb-d9ml3 (Storage trim/snapshot key length cap, P1)

> Schema companion to `proof-obligations.planned.jsonl` and
> `verifier-lane-decisions.jsonl`. This matrix is the human-readable
> binding for the (requirement_id, contract_clause) → (proof_seed_id,
> verifier, proof_obligation_id, lane_decision_id) coverage map. The
> schema-level rows are authoritative; this Markdown mirrors them and
> adds the per-requirement rationale and the cross-lane depth.

Bead ID: `vb-d9ml3`
Planner invocation: `proof-planner-vb-d9ml3-state4`
Workdir: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-d9ml3`
Owner state: 4
Captured: 2026-07-01

---

## Coverage summary

| Metric | Count |
|---|---:|
| Requirements in scope | 10 (REQ-CAP-001..008, REQ-CAP-009, REQ-CAP-010) |
| Contract clauses in scope | 10 (CC-CAP-001..010) |
| Proof seeds in scope | 16 (PS-CAP-CONST-001, PS-CAP-UNIT-001..004, PS-CAP-PROPTEST-001..002, PS-CAP-ENCODER-001, PS-CAP-WORKFLOW-001, PS-CAP-CROSS-CRATE-001, PS-CAP-REGRESSION-001, PS-CAP-KANI-OMIT-001, PS-CAP-VERUS-OMIT-001, PS-CAP-FLUX-OMIT-001, PS-CAP-FUZZ-OMIT-001, PS-CAP-LOOM-OMIT-001) |
| Required lane decisions | 5 (VLD-001..005) |
| Not-applicable lane decisions | 5 (VLD-006..010) |
| Proof obligations | 5 (PO-001-UNIT, PO-001-REGRESSION, PO-002-INTEGRATION, PO-003-PROPTEST, PO-004-LINT) |
| Waiver candidates | 7 (WVR-001..007, all `behavior_affecting: false`) |
| Behavior-affecting obligations | 0 (cap is enforcement, not change) |

---

## Requirement-by-requirement coverage

### REQ-CAP-001 — const-alias equality (CC-CAP-001)

| Field | Value |
|---|---|
| `requirement_id` | `REQ-CAP-001` |
| `contract_clause` | `CC-CAP-001` |
| `domain_claim` | `MAX_TRIM_KEY_LEN == MAX_SNAPSHOT_KEY_LEN == JOURNAL_KEY_BYTES == 17` (compile-time) |
| `proof_seed_id` | `PS-CAP-CONST-001`, `PS-CAP-ENCODER-001` |
| `verifier` | `proptest` (routed because schema has no separate `unit` verifier) |
| `proof_obligation_id` | `PO-001-UNIT` |
| `lane_decision_id` | `VLD-001` |
| `verifier_lane_decision.applicability` | `required` |
| `production_target` | `crates/vb_storage/src/constants.rs::MAX_TRIM_KEY_LEN` (also `MAX_SNAPSHOT_KEY_LEN`, `JOURNAL_KEY_BYTES`) |
| `command` | `PROPTEST_CASES=10 cargo test -p vb_storage --lib max_key_len_aliases_equal_journal_key_bytes` |
| `expected_evidence` | `test result: ok. 1 passed; 0 failed`; prop_assert_eq! passes for both aliases; PROPTEST_CASES=10 budget recorded; anti-invariant: any drift of either alias from JOURNAL_KEY_BYTES would surface as a property failure. |
| `behavior_affecting` | `false` |
| `waiver_id` | (none; no waiver required for this obligation) |
| `trusted_base_refs` | `["TB-CAP-001"]` |

### REQ-CAP-002 — `latest_durable_snapshot_seq` rejects overlong snapshot key (CC-CAP-002, CC-CAP-010)

| Field | Value |
|---|---|
| `requirement_id` | `REQ-CAP-002` |
| `contract_clause` | `CC-CAP-002`, `CC-CAP-010` |
| `domain_claim` | `latest_durable_snapshot_seq` returns `Err(TrimError::IncompleteTrim { deleted_count: 0 })` for any raw key whose `key.len() != MAX_SNAPSHOT_KEY_LEN` (i.e., != 17); the only `Ok(Some(seq))` path is length == 17 with a valid `RunSnapshot` decode. |
| `proof_seed_id` | `PS-CAP-UNIT-001`, `PS-CAP-PROPTEST-001` |
| `verifier` | `proptest` (both integration and proptest lanes routed through this single verifier; distinct artifacts and commands) |
| `proof_obligation_ids` | `PO-002-INTEGRATION`, `PO-003-PROPTEST` |
| `lane_decision_ids` | `VLD-003` (proptest), `VLD-004` (integration) |
| `verifier_lane_decision.applicability` | both `required` |
| `production_target` | `crates/vb_storage/src/trimming/logic.rs::latest_durable_snapshot_seq` |
| `commands` | (1) `PROPTEST_CASES=1 cargo test -p vb_storage --lib latest_durable_snapshot_seq_rejects_overlong_snapshot_key ...` (2) `PROPTEST_CASES=10000 cargo test -p vb_storage --lib proptest_key_cap_roundtrip --release` |
| `expected_evidence` | (1) `test result: ok. 6 passed; 0 failed` — 3 new overlong tests + 3 existing tests continue to assert `Err(TrimError::IncompleteTrim { .. })` (2) `test result: ok. 1 passed; 0 failed` with PROPTEST_CASES=10000 and the full length space 0..=256; anti-invariant: any non-canonical key that yields `Ok(Some(seq))` or panics is a test failure (invalid input class). |
| `behavior_affecting` | `false` (both rows) |
| `waiver_ids` | `WVR-003` (kani), `WVR-004` (kani) |
| `trusted_base_refs` | `[]` (both rows; no trust markers) |

### REQ-CAP-003 — `trim_events_for_run` rejects overlong event key (CC-CAP-003, CC-CAP-010)

| Field | Value |
|---|---|
| `requirement_id` | `REQ-CAP-003` |
| `contract_clause` | `CC-CAP-003`, `CC-CAP-010` |
| `domain_claim` | `trim_events_for_run` returns `Err(TrimError::IncompleteTrim { deleted_count })` at the first raw key whose `key.len() != MAX_TRIM_KEY_LEN` (i.e., != 17); the LSM batch is not committed. |
| `proof_seed_id` | `PS-CAP-UNIT-002`, `PS-CAP-PROPTEST-002` |
| `verifier` | `proptest` (integration lane routed through the proptest verifier vocabulary) |
| `proof_obligation_id` | `PO-002-INTEGRATION` (the integration test in PO-002 covers this requirement via the augmented test `trim_events_for_run_fails_closed_on_overlong_event_key`) |
| `lane_decision_id` | `VLD-004` |
| `verifier_lane_decision.applicability` | `required` |
| `production_target` | `crates/vb_storage/src/trimming/logic.rs::trim_events_for_run` |
| `command` | `PROPTEST_CASES=1 cargo test -p vb_storage --lib trim_events_for_run_fails_closed_on_overlong_event_key` |
| `expected_evidence` | `test result: ok. N passed; 0 failed`; the new overlong test plants a 24-byte raw key under `PREFIX_RUN_EVENT` after a valid 17-byte event and asserts `Err(TrimError::IncompleteTrim { .. })` with diagnostic code 0x4102; anti-invariant: any deviation from the typed error on a non-canonical key is a test failure. |
| `behavior_affecting` | `false` |
| `waiver_id` | (none; kani waiver WVR-003 is also cited at the lane level but the obligation is covered by the integration test) |
| `trusted_base_refs` | `[]` |

### REQ-CAP-004 — `count_trimmable_events` rejects overlong event key (CC-CAP-004, CC-CAP-010)

| Field | Value |
|---|---|
| `requirement_id` | `REQ-CAP-004` |
| `contract_clause` | `CC-CAP-004`, `CC-CAP-010` |
| `domain_claim` | `count_trimmable_events` (via `trim_eligibility_diagnostic`) returns `Err(JournalError::Trim(Box::new(TrimError::IncompleteTrim { deleted_count })))` at the first raw key whose `key.len() != MAX_TRIM_KEY_LEN` (i.e., != 17). |
| `proof_seed_id` | `PS-CAP-UNIT-003` |
| `verifier` | `proptest` (integration lane routed through the proptest verifier vocabulary) |
| `proof_obligation_id` | `PO-002-INTEGRATION` (the integration test in PO-002 covers this requirement via the augmented test `trim_eligibility_diagnostic_fails_closed_on_overlong_event_key`) |
| `lane_decision_id` | `VLD-004` |
| `verifier_lane_decision.applicability` | `required` |
| `production_target` | `crates/vb_storage/src/trimming/logic.rs::count_trimmable_events` |
| `command` | `PROPTEST_CASES=1 cargo test -p vb_storage --lib trim_eligibility_diagnostic_fails_closed_on_overlong_event_key` |
| `expected_evidence` | `test result: ok. N passed; 0 failed`; the new overlong test plants a 24-byte raw key under `PREFIX_RUN_EVENT` and asserts `Err(JournalError::Trim(inner))` where `inner` is `TrimError::IncompleteTrim { .. }` with diagnostic code 0x4102; anti-invariant: any deviation from the wrapped typed error on a non-canonical key is a test failure. |
| `behavior_affecting` | `false` |
| `waiver_id` | (none) |
| `trusted_base_refs` | `[]` |

### REQ-CAP-005 — `TrimError::IncompleteTrim` shape and 0x4102 preservation (CC-CAP-005)

| Field | Value |
|---|---|
| `requirement_id` | `REQ-CAP-005` |
| `contract_clause` | `CC-CAP-005` |
| `domain_claim` | `TrimError::IncompleteTrim { deleted_count: u64 }` shape and 0x4102 diagnostic code are preserved verbatim; `error_code_tests.rs:~246` continues to assert the 0x4102 propagation through `JournalError::Trim(inner).diagnostic_code() -> inner.diagnostic_code()`. |
| `proof_seed_id` | `PS-CAP-UNIT-004`, `PS-CAP-VERUS-OMIT-001` |
| `verifier` | `proptest` (regression gate routed through the proptest verifier vocabulary) |
| `proof_obligation_id` | `PO-001-REGRESSION` |
| `lane_decision_id` | `VLD-002` |
| `verifier_lane_decision.applicability` | `required` |
| `production_target` | `crates/vb_storage/src/trimming/mod.rs::TrimError::IncompleteTrim` |
| `command` | `PROPTEST_CASES=1 cargo test -p vb_storage --lib journal_error_trim_wrapper_delegates_incomplete_trim_code` |
| `expected_evidence` | `test result: ok. 1 passed; 0 failed`; both `assert_eq!(wrapped.diagnostic_code(), TrimError::INCOMPLETE_TRIM_CODE)` and `assert_ne!(wrapped.diagnostic_code(), JournalError::FJALL_CODE)` hold; anti-invariant: any divergence of `INCOMPLETE_TRIM_CODE` from 0x4102 or any break in the delegation chain surfaces as a property failure. |
| `behavior_affecting` | `false` |
| `waiver_id` | `WVR-002` (verus omitted) |
| `trusted_base_refs` | `[]` |

### REQ-CAP-006 — fail-closed workflow (CC-CAP-006)

| Field | Value |
|---|---|
| `requirement_id` | `REQ-CAP-006` |
| `contract_clause` | `CC-CAP-006` |
| `domain_claim` | The trim scanners abort on the first non-canonical observation; the LSM batch is not committed when Err is returned; `trim_eligibility_diagnostic` propagates Err through `JournalError::Trim(inner)`. |
| `proof_seed_id` | `PS-CAP-WORKFLOW-001`, `PS-CAP-LOOM-OMIT-001` |
| `verifier` | `proptest` (integration lane; the integration tests in PO-002 already pin the fail-closed invariant) |
| `proof_obligation_id` | `PO-002-INTEGRATION` (the integration test row covers the workflow invariant) |
| `lane_decision_id` | `VLD-004` |
| `verifier_lane_decision.applicability` | `required` |
| `production_target` | `crates/vb_storage/src/trimming/logic.rs::latest_durable_snapshot_seq`, `::trim_events_for_run`, `::count_trimmable_events` (all three call sites, lines 36, 77, 222) |
| `command` | `PROPTEST_CASES=1 cargo test -p vb_storage --lib snapshot_tests trimming::tests` |
| `expected_evidence` | `test result: ok. N passed; 0 failed`; all three new overlong tests + all three existing fail-closed tests pass; the LSM batch is not committed when Err is returned (verified by the post-fix `journal.snapshot(run, EventSeq::new(5))` lookup at `snapshot_tests.rs:243-247` which confirms the valid snapshot is still present). |
| `behavior_affecting` | `false` |
| `waiver_id` | (none; loom waiver VLD-010 is also cited at the lane level) |
| `trusted_base_refs` | `[]` |

### REQ-CAP-007 — counter progress preservation (CC-CAP-007)

| Field | Value |
|---|---|
| `requirement_id` | `REQ-CAP-007` |
| `contract_clause` | `CC-CAP-007` |
| `domain_claim` | When the trim scanner aborts, the `deleted_count` field preserves the partial progress count. |
| `proof_seed_id` | (none — covered by the structural `IncompleteTrim { .. }` pattern in the existing tests; the contract permits any counter value) |
| `verifier` | (no separate obligation; the counter invariant is implicitly covered by the existing structural assertions in `trimming/tests.rs:929, 984` and the new overlong cases) |
| `proof_obligation_id` | (none — this requirement is satisfied by the structural `IncompleteTrim { .. }` pattern; PO-002-INTEGRATION's `assert!(matches!(err, TrimError::IncompleteTrim { .. }))` and the structural-assertion preservation required by CC-CAP-009) |
| `lane_decision_id` | (none) |
| `verifier_lane_decision.applicability` | (n/a; no separate lane decision required) |
| `production_target` | `crates/vb_storage/src/trimming/logic.rs::trim_events_for_run` (line 78), `::count_trimmable_events` (line 224) |
| `command` | (covered by PO-002-INTEGRATION's `cargo test -p vb_storage --lib trimming::tests` invocation) |
| `expected_evidence` | (covered by the matches! pattern in PO-002-INTEGRATION) |
| `behavior_affecting` | `false` |
| `waiver_id` | (none) |
| `trusted_base_refs` | `[]` |

### REQ-CAP-008 — zero cross-crate change (CC-CAP-008)

| Field | Value |
|---|---|
| `requirement_id` | `REQ-CAP-008` |
| `contract_clause` | `CC-CAP-008` |
| `domain_claim` | The implementation makes zero changes outside `vb_storage`; `cargo check --workspace` continues to pass with the new aliases visible only to `vb_storage`. |
| `proof_seed_id` | `PS-CAP-CROSS-CRATE-001` |
| `verifier` | `proptest` (lint lane routed through the proptest verifier vocabulary) |
| `proof_obligation_id` | `PO-004-LINT` |
| `lane_decision_id` | `VLD-005` |
| `verifier_lane_decision.applicability` | `required` |
| `production_target` | `crates/vb_storage/src/trimming/logic.rs` (full file as the static-analysis target; line refs are 36, 77, 222) |
| `command` | `PROPTEST_CASES=1 bash -c 'set -euo pipefail; moon run :lint-src; cargo check --workspace; ...'` |
| `expected_evidence` | `lint-src` exit 0; `cargo check --workspace` exit 0; `rg -n 'key\.len\(\) != 17' crates/vb_storage/src/trimming/logic.rs` returns no matches; anti-invariant: any remaining magic-17 literal at the named-cap replacement sites is a contract violation. |
| `behavior_affecting` | `false` |
| `waiver_ids` | `WVR-005` (cargo-fuzz), `WVR-006` (verus), `WVR-007` (kani) |
| `trusted_base_refs` | `[]` |

### REQ-CAP-009 — existing tests continue to pass (CC-CAP-009)

| Field | Value |
|---|---|
| `requirement_id` | `REQ-CAP-009` |
| `contract_clause` | `CC-CAP-009` |
| `domain_claim` | All existing tests at `snapshot_tests.rs:208-248` and `trimming/tests.rs:875-987` pass post-fix without modification of their assertion structure; the structural `Err(TrimError::IncompleteTrim { deleted_count: 0 })` assertion at `snapshot_tests.rs:235` is preserved verbatim. |
| `proof_seed_id` | `PS-CAP-REGRESSION-001` |
| `verifier` | `proptest` (regression gate routed through the proptest verifier vocabulary) |
| `proof_obligation_id` | `PO-002-INTEGRATION` (the integration test row includes the existing tests) and `PO-004-LINT` (the lint row includes the existing test invocations) |
| `lane_decision_id` | `VLD-004`, `VLD-005` |
| `verifier_lane_decision.applicability` | both `required` |
| `production_target` | `crates/vb_storage/src/snapshot_tests.rs:208-248`, `crates/vb_storage/src/trimming/tests.rs:875-987` |
| `command` | `PROPTEST_CASES=1 cargo test -p vb_storage --lib snapshot_tests trimming::tests` (within PO-002-INTEGRATION) and `PROPTEST_CASES=1 bash -c '...; cargo test -p vb_storage --lib snapshot_tests; cargo test -p vb_storage --lib trimming::tests; ...'` (within PO-004-LINT) |
| `expected_evidence` | both cargo test invocations return `test result: ok`; the structural assertions at `snapshot_tests.rs:235` and `trimming/tests.rs:929, 984` are unmodified. |
| `behavior_affecting` | `false` |
| `waiver_id` | (none) |
| `trusted_base_refs` | `[]` |

### REQ-CAP-010 — three new overlong test cases (CC-CAP-010)

| Field | Value |
|---|---|
| `requirement_id` | `REQ-CAP-010` |
| `contract_clause` | `CC-CAP-010` |
| `domain_claim` | Three new test cases are added (one per magic-17 site) that plant an overlong raw key (length > 17) under the appropriate prefix and assert the typed error. |
| `proof_seed_id` | `PS-CAP-UNIT-001`, `PS-CAP-UNIT-002`, `PS-CAP-UNIT-003` |
| `verifier` | `proptest` (integration lane) |
| `proof_obligation_id` | `PO-002-INTEGRATION` (the integration test row includes the three new overlong cases) |
| `lane_decision_id` | `VLD-004` |
| `verifier_lane_decision.applicability` | `required` |
| `production_target` | (test-side; the new tests are co-located with the existing tests at `snapshot_tests.rs:~248`, `trimming/tests.rs:~932`, `trimming/tests.rs:~987`) |
| `command` | `PROPTEST_CASES=1 cargo test -p vb_storage --lib latest_durable_snapshot_seq_rejects_overlong_snapshot_key trim_events_for_run_fails_closed_on_overlong_event_key trim_eligibility_diagnostic_fails_closed_on_overlong_event_key` |
| `expected_evidence` | `test result: ok. 3 passed; 0 failed` for the three new tests; each plants a 24-byte raw key under the appropriate prefix and asserts `Err(TrimError::IncompleteTrim { .. })`; anti-invariant: any non-canonical key that does not surface the typed error is a test failure. |
| `behavior_affecting` | `false` |
| `waiver_id` | (none) |
| `trusted_base_refs` | `[]` |

---

## Cross-reference: requirements ↔ obligations ↔ lane decisions ↔ waivers

| Requirement | Obligations | Lane decisions | Waivers |
|---|---|---|---|
| REQ-CAP-001 | PO-001-UNIT | VLD-001 | (none) |
| REQ-CAP-002 | PO-002-INTEGRATION, PO-003-PROPTEST | VLD-003, VLD-004 | WVR-003, WVR-004 |
| REQ-CAP-003 | PO-002-INTEGRATION | VLD-004 | (none) |
| REQ-CAP-004 | PO-002-INTEGRATION | VLD-004 | (none) |
| REQ-CAP-005 | PO-001-REGRESSION | VLD-002 | WVR-002 |
| REQ-CAP-006 | PO-002-INTEGRATION | VLD-004 | (none) |
| REQ-CAP-007 | (covered by PO-002-INTEGRATION) | (covered by VLD-004) | (none) |
| REQ-CAP-008 | PO-004-LINT | VLD-005 | WVR-005, WVR-006, WVR-007 |
| REQ-CAP-009 | PO-002-INTEGRATION, PO-004-LINT | VLD-004, VLD-005 | (none) |
| REQ-CAP-010 | PO-002-INTEGRATION | VLD-004 | (none) |

## Cross-reference: proof seeds ↔ obligations

| Proof seed | Obligations that cite it |
|---|---|
| PS-CAP-CONST-001 | PO-001-UNIT |
| PS-CAP-UNIT-001 | PO-002-INTEGRATION, WVR-003 |
| PS-CAP-UNIT-002 | PO-002-INTEGRATION |
| PS-CAP-UNIT-003 | PO-002-INTEGRATION |
| PS-CAP-UNIT-004 | PO-001-REGRESSION |
| PS-CAP-PROPTEST-001 | PO-003-PROPTEST, WVR-004 |
| PS-CAP-PROPTEST-002 | PO-002-INTEGRATION (via the trim loop integration test) |
| PS-CAP-ENCODER-001 | PO-003-PROPTEST (encoder-side length invariant) |
| PS-CAP-WORKFLOW-001 | PO-002-INTEGRATION (fail-closed invariant) |
| PS-CAP-CROSS-CRATE-001 | PO-004-LINT, WVR-005, WVR-006, WVR-007 |
| PS-CAP-REGRESSION-001 | PO-002-INTEGRATION, PO-004-LINT |
| PS-CAP-KANI-OMIT-001 | VLD-006 |
| PS-CAP-VERUS-OMIT-001 | VLD-007, WVR-001, WVR-002 |
| PS-CAP-FLUX-OMIT-001 | VLD-008 |
| PS-CAP-FUZZ-OMIT-001 | VLD-009 |
| PS-CAP-LOOM-OMIT-001 | VLD-010 |

---

## Defense-in-depth check

| Cross-lane | Primary lane | Companion lane | Coverage rationale |
|---|---|---|---|
| Const-alias equality (CC-CAP-001) | VLD-001 (proptest unit) | (none — single lane is sufficient for a compile-time invariant) | Compile-time invariant; one lane is enough. |
| Variant preservation (CC-CAP-005) | VLD-002 (proptest unit) | (none — single lane is sufficient) | Existing regression test; one lane is enough. |
| Overlong-key rejection (CC-CAP-002/003/004) | VLD-004 (proptest integration) | VLD-003 (proptest length roundtrip) | Integration exercises the production path; proptest exercises the full length space. |
| Zero cross-crate change (CC-CAP-008) | VLD-005 (proptest lint) | (none — single lane is sufficient for a static-source check) | Static-source check; one lane is enough. |
| Fail-closed workflow (CC-CAP-006) | VLD-004 (proptest integration) | (none — covered by the integration tests) | Integration tests already pin the fail-closed invariant. |
| Counter progress (CC-CAP-007) | (covered by VLD-004's structural pattern) | (none) | The `matches!(err, TrimError::IncompleteTrim { .. })` pattern is sufficient per the contract. |
| Existing tests pass (CC-CAP-009) | VLD-004, VLD-005 | (none) | Regression gate is the existing tests. |
| New overlong cases (CC-CAP-010) | VLD-004 | (none) | New tests co-located with the existing tests. |

Defense-in-depth depth: 1-2 lanes per requirement. The bead is a low-blast-radius internal fix; one or two lanes per requirement is the appropriate depth (per `references/defense-depth-matrix.md`, which requires ≥1 lane for behavior-relevant obligations; this bead's obligations are all `behavior_affecting: false` and the existing tests + const-alias chain provide sufficient coverage).

---

## Self-audit against `references/plan-quality-gates.md`

| Gate | Status | Note |
|---|---|---|
| Gate 1 — Schema compliance | PASS | validator exit 0; no schema errors. |
| Gate 2 — Lane decision coverage | PASS | 10 VLD rows (5 required + 5 not_applicable) covering the 16 proof seeds. |
| Gate 3 — Obligation pairing | PASS | every required VLD has ≥1 paired PO; every PO has a `target` that parses as `path::symbol`. |
| Gate 4 — Implementation binding | PASS | every `target` parses as `path::symbol`; no Verus obligations so no `external_body`/`assume`/`axiom` risk. |
| Gate 5 — Evidence specificity | PASS | every `command` is exact; every `workdir` is absolute; every `expected_evidence` cites a concrete tool marker (`test result: ok`, `clippy` exit 0, `rg` zero matches). |
| Gate 6 — Resource governance | PASS | every proptest obligation includes `PROPTEST_CASES`; `model_bounds.cases` is set. |
| Gate 7 — Waiver discipline | PASS | 7 waiver rows, all `behavior_affecting: false`; each has a concrete `reason`, `boundary_proof`, `compensating_evidence`, ISO-8601 `expiry`, and `owner`. |
| Gate 8 — Trust marker ledger | PASS | one `TB-CAP-001` row in `trusted-base-plan.md` for the const alias chain. |
| Gate 9 — Cross-reference integrity | PASS | every `behavior_affecting: false` PO row; no `rust-refinement-obligation/v1` rows required; the requirement ↔ obligation ↔ lane ↔ waiver cross-reference is complete. |
| Gate 10 — Mirror parity | OUT_OF_SCOPE | the planner does not modify the skill tree; the skill tree is at `~/.agents/skills/proof-planner/` and `~/.opencode/skill/proof-planner/`. |

END OF PROOF COVERAGE MATRIX.