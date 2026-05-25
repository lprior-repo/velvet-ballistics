# vb-njju Test Plan — State 7

## Overview

Bead `vb-njju` covers BDD mutation/fuzz/property closure scenarios for release evidence validation.
State 6 confirmed: **MUT-ADM-001 PASS** (mutation) and **FUZZ-SMOKE-001 PASS** (fuzz smoke).

This plan covers all remaining **planned** proof obligations (`status: planned`) that require test execution evidence before State 8 black-hat review and landing.

---

## Evidence Status Summary

| Obligation    | Layer        | Status       | Notes                                      |
|---------------|--------------|--------------|--------------------------------------------|
| MUT-ADM-001   | cargo-mutants | **PASS**     | State 6 approved                           |
| FUZZ-SMOKE-001 | cargo-fuzz   | **PASS**     | State 6 approved                           |
| BDD-CAT-001   | proptest     | planned      | catalog row validation                     |
| MUT-PLAN-002  | cargo-mutants | planned      | mutation plan scope validation             |
| FUZZ-BUILD-002| cargo-fuzz   | planned      | fuzz binary build                          |
| PROP-TAINT-001| proptest     | planned      | taint parity in generated-vs-IR oracle     |
| PROP-REPLAY-002| proptest    | planned      | deterministic replay invariant             |
| BOUNDARY-FUZZ-001 | cargo-fuzz | planned      | boundary inventory contract                |
| BOUNDARY-REL-002| gauntlet-all | planned      | release gate fails on missing boundary fuzz |
| TRACE-JSONL-001| static-scan | planned      | JSONL traceability                         |
| TLA-WAIVE-001 | waiver       | planned      | TLA+ non-applicability                     |
| LEAN-WAIVE-001| waiver       | planned      | Lean/Aeneas/Hax non-applicability          |

---

## Test Obligations

### TO-001: BDD-CAT-001 — Acceptance Catalog Validation

**Contract clause:** PRE-001, PRE-002, POST-005, INV-001, INV-006
**Target:** `crates/workspace_tests/src/acceptance_catalog.rs`
**Proof obligation:** vb-njju scenarios use public acceptance catalog rows with non-empty Given/When/Then, exact evidence disposition, isolated fixtures, and correct related-bead fields.

#### Sub-obligations

| Sub-test                                              | Command | Expected |
|-------------------------------------------------------|---------|----------|
| `vb_hxm0_acceptance_catalog` catalog shape validation | `cargo test --package velvet-ballistics-workspace-tests --test vb_hxm0_acceptance_catalog` | `test result: ok`; all 4 vb-njju rows present |
| `vb_njju_catalog_rows_exist_and_validate`             | (included in above) | BDD-NJJU-001 through BDD-NJJU-004 all pass validate_catalog |
| Public-surface isolation check                        | (included in validate_catalog) | no "private"/"helper" surface, all fixtures contain "isolated" |
| Duplicate ID rejection                                | (included in validate_catalog) | no duplicate scenario IDs |

**Executable test location:** `crates/workspace_tests/tests/vb_njju_mutation_fuzz_property_closure.rs` → `vb_njju_catalog_rows_exist_and_validate`

**Evidence:** `target/test-output/vb_hxm0_acceptance_catalog.log`

**Fail-closed behavior:**
- Missing scenario row → `EvidenceError::MissingScenario`
- Empty Given/When/Then → `EvidenceError::MissingGivenWhenThen`
- Non-isolated fixture → `EvidenceError::SharedFixture`
- Private surface → `EvidenceError::PrivateSurface`

---

### TO-002: MUT-PLAN-002 — Mutation Plan Scope Validation

**Contract clause:** PRE-003, POST-001, INV-003
**Target:** `crates/workspace_tests/tests/vb_c3k9_current_api_mutation_plan.rs`
**Proof obligation:** Mutation plan names critical survivor policy, exact scope, and admission branch closure rather than unrelated smoke.

#### Sub-obligations

| Sub-test | Command | Expected |
|---------|---------|----------|
| Mutation plan test suite | `cargo test --package velvet-ballistics-workspace-tests --test vb_c3k9_current_api_mutation_plan` | `test result: ok` |
| Validates admission scope is named | (inline in test) | plan must reference admission-branch scope, not diagnostic.rs |
| Validates blocking disposition | (inline in test) | non-blocking evidence → `EvidenceError::ReleaseGateWouldPassUnsafely` |
| Rejects unrelated mutation scope | (inline in test) | unrelated scope → `EvidenceError::UnrelatedMutationScope` |

**Evidence:** `target/test-output/vb_c3k9_current_api_mutation_plan.log`

**Rationale for standalone test:** The `vb_njju_mutation_fuzz_property_closure.rs` tests the *gate logic* (TO-003 below). `MUT-PLAN-002` validates the *mutation plan document* that feeds the gate — a distinct artifact requiring separate evidence that the plan itself names correct scope and survivor policy.

---

### TO-003: FUZZ-BUILD-002 — Fuzz Binary Build Check

**Contract clause:** PRE-004, POST-002, INV-002
**Target:** `fuzz/Cargo.toml`
**Proof obligation:** Required fuzz binaries yaml_events, ipc_frame, journal_event, and compiled_ir build successfully.

#### Sub-obligations

| Sub-test | Command | Expected |
|---------|---------|----------|
| Build all fuzz targets | `cargo fuzz build --target x86_64-unknown-linux-gnu` (or default target) | exit 0 |
| Verify yaml_events binary | Check `fuzz/target/x86_64-unknown-linux-gnu/release/yaml_events` exists | file present |
| Verify ipc_frame binary | Check `fuzz/target/x86_64-unknown-linux-gnu/release/ipc_frame` exists | file present |
| Verify journal_event binary | Check `fuzz/target/x86_64-unknown-linux-gnu/release/journal_event` exists | file present |
| Verify compiled_ir binary | Check `fuzz/target/x86_64-unknown-linux-gnu/release/compiled_ir` exists | file present |

**Evidence:** `target/test-output/cargo-fuzz-build.log`

**Fail-closed behavior:** Missing any declared binary → fuzz-smoke evidence is BuildOnly (INV-002).

**Note:** This is a build-only check. Runnable evidence is covered by FUZZ-SMOKE-001 (already PASS).

---

### TO-004: PROP-TAINT-001 — Taint Parity Property Gate

**Contract clause:** POST-003, PRE-005, INV-004
**Target:** `crates/vb_codegen/src/proptests.rs` (or companion `proptests/generated_ir_taint_parity.rs`)
**Proof obligation:** Generated-vs-IR property gate fails if taint comparison is ignored.

#### Sub-obligations

| Sub-test | Command | Expected |
|---------|---------|----------|
| Taint parity property test | `cargo test --package vb_codegen --lib -- fixed_six_step_emitted_rust_and_ir_match_finished_signal_and_slots` | test passes with taint included in parity check |
| Taint-ignored variant assertion | (inline in test) | when taint is omitted but slots/signals match → `EvidenceError::TaintParityIgnored` |
| Proptest harness (if separate file) | `cargo test --package vb_codegen --lib generated_ir_taint_parity` | harness compiles and runs |

**Evidence:** `target/test-output/vb_codegen_taint_parity.log`

**Fail-closed behavior:** Property test must assert that omitting taint from the parity check causes a failure equivalent to `EvidenceError::TaintParityIgnored`.

**Property statement (natural language):**
> For all workflow inputs accepted by `validate_generated_subset`, `generated_rust_result.taint == ir_result.taint` is a required parity condition alongside result slots and signals. If taint is omitted from the equality oracle while slots and signals match, the property fails.

---

### TO-005: PROP-REPLAY-002 — Deterministic Replay Invariant

**Contract clause:** PRE-005
**Target:** `crates/vb_storage/src/proptests.rs`
**Proof obligation:** Replay and snapshot properties remain deterministic while taint parity closure is added.

#### Sub-obligations

| Sub-test | Command | Expected |
|---------|---------|----------|
| Deterministic replay invariant | `cargo test --package vb_storage --lib proptests::ppi_001_deterministic_replay_invariant` | test passes; no regression from vb-njju property closure |
| Taint in replay digest parity | (included in above or companion) | replay digest includes taint field parity |

**Evidence:** `target/test-output/vb_storage_replay_props.log`

**Fail-closed behavior:** Test fails if adding taint parity to generated-vs-IR comparison breaks deterministic replay.

---

### TO-006: BOUNDARY-FUZZ-001 — Boundary Inventory Contract

**Contract clause:** POST-004, PRE-006, INV-005
**Target:** `crates/workspace_tests/tests/vb_y1zq_boundary_inventory_contract.rs`
**Proof obligation:** Unsafe/decoder/binary boundary inventory exposes required fuzz evidence or approved blocker/follow-up for each boundary.

#### Sub-obligations

| Sub-test | Command | Expected |
|---------|---------|----------|
| Boundary inventory contract test | `cargo test --package velvet-ballistics-workspace-tests --test vb_y1zq_boundary_inventory_contract` | test passes |
| Every unsafe boundary has fuzz evidence | (inline validation) | each boundary has either `has_fuzz: true` or `approved_blocker: true` |
| Boundary list covers decoder, IPC, binary surfaces | (inline validation) | no unknown/unmapped boundary types pass silently |

**Evidence:** `target/test-output/vb_y1zq_boundary_inventory_contract.log`

**Fail-closed behavior:**
- Empty boundary list → `EvidenceError::ReleaseGateWouldPassUnsafely`
- Any boundary missing fuzz AND blocker → `EvidenceError::UnsafeBoundaryFuzzMissing`

---

### TO-007: BOUNDARY-REL-002 — Release Gate Boundary Fuzz Failure

**Contract clause:** POST-004, INV-005
**Target:** `crates/workspace_tests/tests/vb_njju_mutation_fuzz_property_closure.rs`
**Proof obligation:** Release gate fails when unsafe boundary fuzz evidence is missing.

#### Sub-obligations

| Sub-test | Command | Expected |
|---------|---------|----------|
| `test_unsafe_boundary_fuzz_missing_causes_release_gate_failure` | `cargo test --package velvet-ballistics-workspace-tests --test vb_njju_mutation_fuzz_property_closure test_unsafe_boundary_fuzz_missing_causes_release_gate_failure` | test passes (fail-closed behavior demonstrated) |

**Evidence:** `target/test-output/vb_njju_mutation_fuzz_property_closure.log`

**Fail-closed behavior demonstrated by test:**
- Missing fuzz without approved blocker → `EvidenceError::UnsafeBoundaryFuzzMissing`
- Missing fuzz with approved blocker → `Ok(())` (waiver accepted)
- Present fuzz → `Ok(())`

**Note:** This is the unit-level gate behavior test. The full release gate integration requires BOUNDARY-FUZZ-001 (inventory) to also pass.

---

### TO-008: TRACE-JSONL-001 — JSONL Traceability Validation

**Contract clause:** INV-006, POST-006
**Target:** `.beads/vb-njju/proof-obligations.jsonl`, `.beads/vb-njju/traceability-matrix.jsonl`
**Proof obligation:** Contract clauses are machine-traceable to executable evidence obligations.

#### Sub-obligations

| Sub-test | Command | Expected |
|---------|---------|----------|
| JSONL parse proof-obligations | `python3 -c "import json, pathlib; [json.loads(l) for l in pathlib.Path('.beads/vb-njju/proof-obligations.jsonl').read_text().splitlines() if l.strip()]"` | exit 0, no JSONDecodeError |
| JSONL parse traceability-matrix | `python3 -c "import json, pathlib; [json.loads(l) for l in pathlib.Path('.beads/vb-njju/traceability-matrix.jsonl').read_text().splitlines() if l.strip()]"` | exit 0, no JSONDecodeError |
| All contract clauses covered | (derived from matrix) | every PRE-*, POST-*, INV-* maps to ≥1 proof obligation |
| All proof obligations map to contract clause | (derived from obligations) | every obligation has a contract_clause field |

**Evidence:** stdout/stderr of above python commands (exit code 0 is sufficient)

---

### TO-009: TLA-WAIVE-001 — TLA+ Non-applicability Waiver

**Contract clause:** INV-006
**Target:** `.beads/vb-njju/tla-spec.md`
**Proof obligation:** TLA+ is non-applicable because vb-njju has no temporal workflow/protocol/concurrency behavior.

#### Sub-obligations

| Sub-test | Command | Expected |
|---------|---------|----------|
| Waiver review acceptance | Independent reviewer accepts temporal non-applicability | waiver recorded in `contract-verification-review.md` |

**Evidence:** `contract-verification-review.md` with TLA-WAIVE-001 accepted.

**Rationale:** vb-njju defines static/release-gate evidence closure — no scheduler, lease, retry, protocol, or concurrent lifecycle. Finite evidence lattice modeled in tla-spec.md with explicit non-applicability rationale.

---

### TO-010: LEAN-WAIVE-001 — Lean/Aeneas/Hax Non-applicability Waiver

**Contract clause:** INV-006
**Target:** `.beads/vb-njju/lean-contract.md`
**Proof obligation:** Lean/Aeneas/Hax theorem kernel is non-applicable because no theorem-critical kernel is introduced.

#### Sub-obligations

| Sub-test | Command | Expected |
|---------|---------|----------|
| Waiver review acceptance | Independent reviewer accepts theorem non-applicability | waiver recorded in `contract-verification-review.md` |

**Evidence:** `contract-verification-review.md` with LEAN-WAIVE-001 accepted.

**Rationale:** vb-njju evidence invariants are classification and acceptance-test fail-closed behavior, not algebraic theorems beyond Rust tests/property/mutation.

---

## Traceability Matrix

| Test Obligation | Contract Clauses | Proof Obligations Covered |
|-----------------|------------------|--------------------------|
| TO-001 BDD-CAT-001 | PRE-001, PRE-002, POST-005, INV-001, INV-006 | BDD-CAT-001 |
| TO-002 MUT-PLAN-002 | PRE-003, POST-001, INV-003 | MUT-PLAN-002 |
| TO-003 FUZZ-BUILD-002 | PRE-004, POST-002, INV-002 | FUZZ-BUILD-002 |
| TO-004 PROP-TAINT-001 | POST-003, PRE-005, INV-004 | PROP-TAINT-001 |
| TO-005 PROP-REPLAY-002 | PRE-005 | PROP-REPLAY-002 |
| TO-006 BOUNDARY-FUZZ-001 | POST-004, PRE-006, INV-005 | BOUNDARY-FUZZ-001 |
| TO-007 BOUNDARY-REL-002 | POST-004, INV-005 | BOUNDARY-REL-002 |
| TO-008 TRACE-JSONL-001 | INV-006, POST-006 | TRACE-JSONL-001 |
| TO-009 TLA-WAIVE-001 | INV-006 | TLA-WAIVE-001 |
| TO-010 LEAN-WAIVE-001 | INV-006 | LEAN-WAIVE-001 |

---

## Execution Order (Recommended)

```
1. TO-008 TRACE-JSONL-001   # prerequisite sanity check; pure static validation
2. TO-001 BDD-CAT-001        # catalog shape + vb-njju row presence
3. TO-002 MUT-PLAN-002        # mutation plan scope
4. TO-003 FUZZ-BUILD-002      # fuzz binary build (independent of run evidence)
5. TO-004 PROP-TAINT-001      # taint parity property (codegen crate)
6. TO-005 PROP-REPLAY-002     # deterministic replay invariant (storage crate)
7. TO-006 BOUNDARY-FUZZ-001  # boundary inventory contract
8. TO-007 BOUNDARY-REL-002   # release gate fail-closed behavior (closure test)
9. TO-009 TLA-WAIVE-001      # waiver review (independent, can run in parallel with tests)
10. TO-010 LEAN-WAIVE-001    # waiver review (independent, can run in parallel with tests)
```

---

## Pass Criteria for State 7 → 8 Handoff

All 10 test obligations must report **PASS** or **WAIVED** (for TO-009, TO-010) before black-hat review.

| Obligation | Required Result |
|-----------|----------------|
| TO-001 | PASS (cargo test exit 0) |
| TO-002 | PASS (cargo test exit 0) |
| TO-003 | PASS (cargo fuzz build exit 0 + binary presence) |
| TO-004 | PASS (cargo test exit 0) |
| TO-005 | PASS (cargo test exit 0) |
| TO-006 | PASS (cargo test exit 0) |
| TO-007 | PASS (cargo test exit 0) |
| TO-008 | PASS (python3 exit 0) |
| TO-009 | WAIVED (contract-verification-review.md acceptance) |
| TO-010 | WAIVED (contract-verification-review.md acceptance) |

---

## Risk Summary

| Risk | Level | Mitigation |
|------|-------|------------|
| codegen proptest harness not yet written for PROP-TAINT-001 | critical | TO-004 must include the proptest harness creation as part of the obligation execution; if harness does not exist, create minimal one-shot property in `vb_codegen/src/proptests.rs` |
| boundary inventory may not expose machine-readable list | high | TO-006 should read from the public boundary inventory API; if only informal, document as blocker evidence |
| mutation plan test vb_c3k9 may not exist | medium | TO-002 reads from `vb_c3k9_current_api_mutation_plan.rs`; if file is absent, treat as MISSING evidence and create stub per MUT-PLAN-002 scope |
