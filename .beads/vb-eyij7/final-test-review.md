# Final Test Review — vb-eyij7

**Reviewed snapshot:** `a47c885d` (codex/vb-eyij7-candidate)
**Reviewer:** local3090 independent test reviewer
**Skill:** test-reviewer (suite review)
**Contracts:** vb-eyij7, vb-3l40a
**Related code review:** integrated-code-review.md (REJECTED at 34b7faab — admission validation gaps)
**Outcome:** 2 BLOCKER findings → REJECTED. Do not advance.

---

## Test Execution Summary

### Commands executed

```bash
env RUSTC_WRAPPER= CARGO_BUILD_JOBS=2 timeout 300 cargo test \
  -p velvet-ballistics-workspace-tests \
  --test vb_eyij7_migration_regression
```
Result: 33 passed, 0 failed, 0 ignored. Duration: 0.02s.

```bash
env RUSTC_WRAPPER= CARGO_BUILD_JOBS=2 timeout 300 cargo test \
  -p velvet-ballistics-workspace-tests \
  --test vb_qi37_25_quality_gates
```
Result: 3 passed, 0 failed, 0 ignored. Duration: 0.10s.

```bash
env RUSTC_WRAPPER= CARGO_BUILD_JOBS=2 timeout 300 cargo test \
  -p vb_runtime --lib "shard::lifecycle::tests"
```
Result: 105 passed, 0 failed, 0 ignored. Duration: 0.16s.

**Total tests executed:** 141. **Total passed:** 141. **Total failed:** 0. **Total ignored:** 0.

Note: `cargo test -p vb_runtime` (without `--lib`) fails to compile due to missing `input_slots` fields in integration test files (`durable_retry_red_phase.rs`, `vb_jggy_lifecycle_tests.rs`). This is a vb-3l40a compile-fix scope issue, not a lifecycle test issue.

---

## Findings

### 1. BLOCKER — No test for action ID workflow binding at admission time

**File:** `crates/vb_runtime/src/shard/lifecycle_tests/contract_validation_tests.rs` (new, 78 lines)

The two contract validation tests cover:
- `validate_contracts_rejects_duplicate_action_ids` — duplicate contract ID rejection
- `validate_contracts_accepts_well_formed_contracts` — well-formed contract acceptance

**Missing coverage:**
- No test verifies that a contract with an action ID **not in the compiled workflow** is rejected at admission time.
- No test verifies that a contract with an action ID **in the compiled workflow** is accepted.

This is exactly the gap identified in the integrated code review (Findings 1 and 3): submitted `ActionContract`s are not validated against the compiled workflow's action step IDs. Any caller can submit a contract with `id = ActionId::new(999)` for a workflow with no such action step; the admission gate accepts it and only fails at execution time.

**Concrete correction:**
- Add test: submit contract with `ActionId::new(999)` for a workflow whose only `Do` node uses `ActionId::new(0)`. Assert `ContractValidationError` with a message indicating the action ID is not in the workflow.
- Add positive test: submit contract with `ActionId::new(0)` for a workflow that has `Do { action: ActionId::new(0), ... }`. Assert `Ok`.

**Disposition:** open (cannot accept while code blockers open)

---

### 2. BLOCKER — No test for recovery artifact contract validation

**File:** `crates/vb_runtime/src/shard/lifecycle_tests/action_recovery_tests.rs` (new, 273 lines)

The single action recovery test (`recover_restores_action_contracts_schedules_completes_and_finishes`) covers the happy path:
- Recovery restores action contracts from the artifact (exact match assertion)
- Do action scheduled with correct ticket fields
- Action completion via real API
- Run finishes with correct journal events (exactly one RunFinished)
- Duplicate completion rejected with `RunNotFound`
- Journal unchanged after duplicate completion

**Missing coverage:**
- The test's contract store returns the same contract that was originally used. There is no test where recovery loads an artifact with **different** action contracts than expected and asserts a validation error.
- This is the gap identified in the integrated code review (Finding 2): recovery does not validate the loaded artifact's action contracts against the recovered run's expected contracts.

**Concrete correction:**
- Add test: create a recovery scenario where the artifact store returns action contracts that differ from what the recovered run expects (e.g., different action IDs or different contract parameters). Assert that recovery fails with an appropriate error (e.g., `Recovery` with contract validation message).

**Disposition:** open (cannot accept while code blockers open)

---

### 3. LOW — Weak helper-only tests in recovery_context_tests.rs

**File:** `crates/vb_runtime/src/shard/lifecycle_tests/recovery_context_tests.rs` (new, 85 lines)

Three of the four tests in this file are weak:

- `recover_seeds_nonzero_journal_sequence` (line 52) — only calls `seed_journal_sequence` and asserts `Ok`. Does not verify that the seeded sequence affects subsequent journal behavior or recovery.
- `recover_handles_pending_action_boundary` (line 61) — only tests the error path (`RunNotFound` for non-existent run). Does not test successful reinjection for an existing run.
- `recover_handles_open_ask_boundary` (line 80) — only asserts a constant value (`RecoveredRunBoundaryKind::None`). Does not test actual recovery with an open ask boundary.

These are compile-time proof that functions exist, not behavior tests.

**Concrete correction:**
- Add recovery test that seeds a journal sequence and then verifies recovery starts replay at the correct sequence.
- Add recovery test that successfully reinjects a pending action for an existing run and drives it to completion.
- Add recovery test that recovers a run with an open ask boundary and verifies the ask can still be answered.

**Disposition:** open

---

### 4. LOW — Explicit panics in test helpers

**Files:** `chunk_001.rs` (lines 23, 342), `chunk_002.rs` (line 100)

Three test-internal `panic!` calls:
- `chunk_001.rs:23` — `panic!("timer_command requires a live captured timer entry")` in helper function
- `chunk_001.rs:342` — `panic!("expected AdmissionCapabilityDenied, got: {err:?}")` in test assertion
- `chunk_002.rs:100` — `panic!("pending action ticket must exist for run {run:?}")` in test assertion

These are test-internal panics, not production panics. Acceptable in tests per AGENTS.md (no `panic` in production code). Not a finding requiring correction, but noted for completeness.

**Disposition:** owner_approved_no_action (tests, not production)

---

### 5. INFO — Migration regression tests are strong

**File:** `crates/workspace_tests/tests/vb_eyij7_migration_regression.rs` (636 lines, 33 tests)

Comprehensive coverage:
- Exact workspace member equality (11 members) via toml parsing (3 tests)
- Cargo metadata resolves exactly 11 packages with correct names (1 test)
- YAML API identity and behavior parity (6 tests, all with exact error variant assertions)
- Expression API identity (4 tests)
- ExprError conversion from CoreError preserves specific variants (6 tests, all exact variant + field assertions)
- vb_compile re-exports work (3 tests)
- Canonical product naming parsed from manifests (3 tests)
- No duplicate old implementation modules (2 tests)
- Fuzz targets reachable (2 tests)
- Expression error behavior parity (3 tests, exact variant assertions)

All 33 pass. Strong behavioral coverage with concrete assertions.

**Disposition:** owner_approved_no_action

---

### 6. INFO — Quality gate tests are strong

**File:** `crates/workspace_tests/tests/vb_qi37_25_quality_gates.rs` (3 tests)

Each test creates a fixture that should fail the workspace assertions script, then verifies the exact error output:
- Package name drift reports exact member and expected name
- Binary alias reports exact allowed binary set
- Feature drift reports exact expected feature set

All 3 pass with exact stderr assertions.

**Disposition:** owner_approved_no_action

---

### 7. INFO — Action recovery test is strong for happy path

**File:** `crates/vb_runtime/src/shard/lifecycle_tests/action_recovery_tests.rs`

The single test is comprehensive for the happy path:
- Uses real journal (not mocked)
- Exact output value assertion (`I64(42)`)
- Exact completion count (1 ActionCompleted or Envelope event)
- Exact RunFinished event count (1) with run ID and result slot verification
- Whole journal unchanged after duplicate tick
- Duplicate completion produces `RunNotFound` error

**Disposition:** owner_approved_no_action

---

## Disposition

**REJECTED.**

The two blocker findings represent test coverage gaps that directly correspond to the code review blockers:
1. No test for action ID workflow binding at admission time
2. No test for recovery artifact contract validation

The tests are structurally sound and pass, but they do not cover the specific contract gaps identified in the integrated code review. The migration regression tests (33) and quality gate tests (3) are strong and comprehensive. The runtime lifecycle tests (105) are extensive but the new contract/recovery tests (7 total across 3 new files) are insufficient to close the admission validation gaps.

All blocker findings can be resolved by:
1. Adding admission validation tests that verify action ID binding to workflow (Finding 1)
2. Adding recovery validation tests that verify artifact contract compatibility (Finding 2)

The weak helper-only tests (Finding 3) should be strengthened but are not blockers.
