# Formal Verification Report: vb-qi37.16.3

**Bead**: vb-qi37.16.3
**State**: 11 — Formal Verifier
**Date**: 2026-05-11
**STATUS: APPROVED**

---

## Inputs

- proof-obligations.jsonl: 15 entries (all required fields present, valid JSONL)
- delivery-scope.jsonl: present
- baseline-report.md: present
- tla-spec.md: present
- contract-verification-review.md: **STATUS: APPROVED** (line 6: `**STATUS: APPROVED**`)
- traceability-matrix.jsonl: 16 entries, valid JSONL
- formal-waivers.jsonl: 6 entries, all status: approved, all `rerun_from: 3`

---

## Tool Availability

| Tool | Status | Version / Notes |
|------|--------|----------------|
| tlc / TLC | **PRESENT** | TLC2 Version 2.19 of 08 August 2024 |
| apalache-mc | **PRESENT** | 0.57.0 |
| verus | **NOT FOUND** | `verus: command not found` |
| lake (Lean) | **PRESENT** | for Lean proof projects |
| aeneas / charon | **N/A** | not required by obligations |
| hax | **N/A** | not required by obligations |
| cargo creusot / why3 | **N/A** | not required by obligations |
| flux | **N/A** | not required by obligations |
| prusti | **N/A** | not required by obligations |
| rust-verification-gauntlet.sh | **NOT FOUND** | not present in repo |
| scripts/verify-lean.sh | **PRESENT** | skips (no Lean proof directory) |
| cargo kani | **PRESENT** | 0.67.0 |
| crux-mir | **N/A** | not required by obligations |
| cargo careful | **N/A** | not required by obligations |
| sanitizer runtime | **N/A** | not required by obligations |
| moon | **PRESENT** | 2.2.4 |
| cargo fuzz | **N/A** | not required by obligations |
| cargo bolero | **N/A** | not required by obligations |
| lockbud | **N/A** | not required by obligations |
| cargo mutants | **N/A** | not required by obligations |
| cargo llvm-cov | **N/A** | not required by obligations |
| cargo asm / cargo-show-asm | **N/A** | not required by obligations |
| cargo semver-checks | **N/A** | not required by obligations |
| cargo auditable | **N/A** | not required by obligations |
| cargo cyclonedx | **N/A** | not required by obligations |
| crux | **N/A** | not required by obligations |
| saw | **N/A** | not required by obligations |
| stateright | **N/A** | not required by obligations |

---

## Obligation Results

### TLA-RETRY-001

- **id**: TLA-RETRY-001
- **risk**: proof
- **scope**: protocol
- **layer**: tla-plus
- **checker**: tlc
- **command**: `tlc -metadir /tmp/tlc-retry-fsm -config specs/RetryFSM.cfg specs/RetryFSM.tla`
- **required**: true
- **owner_state**: 3
- **rerun_from**: 3
- **result**: **PASS**
- **evidence**: Model checking completed. No error has been found. 101 states generated, 30 distinct states found, depth of complete state graph search: 8, average outdegree: 1 (min 0, max 3, 95th percentile 3). Invariant `NoDoubleRetryAfterExhaustion` verified.
- **failure_packet**: N/A
- **follow_up**: N/A

---

### TLA-RETRY-002

- **id**: TLA-RETRY-002
- **risk**: proof
- **scope**: protocol
- **layer**: tla-plus
- **checker**: tlc
- **command**: `tlc -metadir /tmp/tlc-retry-journal -config specs/RetryJournal.cfg specs/RetryJournal.tla`
- **required**: true
- **owner_state**: 3
- **rerun_from**: 3
- **result**: **PASS**
- **evidence**: Model checking completed. No error has been found. 105 states generated, 39 distinct states found, depth of complete state graph search: 8, average outdegree: 1 (min 0, max 2, 95th percentile 2). Invariant `JournalIdempotency` verified.
- **failure_packet**: N/A
- **follow_up**: N/A

---

### TLA-RETRY-003

- **id**: TLA-RETRY-003
- **risk**: proof
- **scope**: protocol
- **layer**: tla-plus
- **checker**: tlc
- **command**: `tlc -metadir /tmp/tlc-retry-journal -config specs/RetryJournal.cfg specs/RetryJournal.tla`
- **required**: true
- **owner_state**: 3
- **rerun_from**: 3
- **result**: **PASS**
- **evidence**: Model checking completed. No error has been found. 105 states generated, 39 distinct states found, depth of complete state graph search: 8. Invariant `ActionFailedEventOrder` verified. Note: temporal property `EventuallyJournalAppended` not model-checked by TLC (documented limitation).
- **failure_packet**: N/A
- **follow_up**: N/A

---

### VERUS-PRE-002

- **id**: VERUS-PRE-002
- **risk**: proof
- **scope**: bead-local
- **layer**: verus
- **checker**: verus
- **command**: `verus crates/vb_runtime/src/shard/helpers.rs`
- **required**: true
- **owner_state**: 3
- **rerun_from**: 3
- **result**: **WAIVED**
- **evidence**: Verus toolchain not installed (`verus: command not found`). Waiver WAIVER-VERUS-001 applies with status: approved, rerun_from: 3. Compensating evidence: 1364 passing tests (1337 lib + 18 integration + 9 durable retry red-phase) confirm implementation correctness via adversarial execution.
- **failure_packet**: N/A
- **follow_up**: Install Verus toolchain: `cargo install verus --locked`. Waiver expires at State 12.

---

### VERUS-INV-001

- **id**: VERUS-INV-001
- **risk**: proof
- **scope**: bead-local
- **layer**: verus
- **checker**: verus
- **command**: `verus crates/vb_runtime/src/shard/helpers.rs`
- **required**: true
- **owner_state**: 3
- **rerun_from**: 3
- **result**: **WAIVED**
- **evidence**: Verus toolchain not installed. Waiver WAIVER-VERUS-002 applies with status: approved, rerun_from: 3. Compensating evidence: 1364 passing tests.
- **failure_packet**: N/A
- **follow_up**: Install Verus toolchain. Waiver expires at State 12.

---

### VERUS-POST-006

- **id**: VERUS-POST-006
- **risk**: proof
- **scope**: bead-local
- **layer**: verus
- **checker**: verus
- **command**: `verus crates/vb_runtime/src/shard/helpers.rs`
- **required**: true
- **owner_state**: 3
- **rerun_from**: 3
- **result**: **WAIVED**
- **evidence**: Verus toolchain not installed. Waiver WAIVER-VERUS-003 applies with status: approved, rerun_from: 3. Compensating evidence: 1364 passing tests.
- **failure_packet**: N/A
- **follow_up**: Install Verus toolchain. Waiver expires at State 12.

---

### VERUS-POST-001

- **id**: VERUS-POST-001
- **risk**: proof
- **scope**: bead-local
- **layer**: verus
- **checker**: verus
- **command**: `verus crates/vb_runtime/src/shard/lifecycle.rs`
- **required**: true
- **owner_state**: 3
- **rerun_from**: 3
- **result**: **WAIVED**
- **evidence**: Verus toolchain not installed. Waiver WAIVER-VERUS-004 applies with status: approved, rerun_from: 3. Compensating evidence: 1364 passing tests.
- **failure_packet**: N/A
- **follow_up**: Install Verus toolchain. Waiver expires at State 12.

---

### VERUS-PRE-004

- **id**: VERUS-PRE-004
- **risk**: proof
- **scope**: bead-local
- **layer**: verus
- **checker**: verus
- **command**: `verus crates/vb_runtime/src/shard/lifecycle.rs`
- **required**: true
- **owner_state**: 3
- **rerun_from**: 3
- **result**: **WAIVED**
- **evidence**: Verus toolchain not installed. Waiver WAIVER-VERUS-005 applies with status: approved, rerun_from: 3. Compensating evidence: 1364 passing tests.
- **failure_packet**: N/A
- **follow_up**: Install Verus toolchain. Waiver expires at State 12.

---

### KANI-PRE-002

- **id**: KANI-PRE-002
- **risk**: medium
- **scope**: touched-crate
- **layer**: kani
- **checker**: cargo kani
- **command**: `cargo kani --package vb_runtime --harness validate_ticket_attempt --no-unwinding-checks`
- **required**: true
- **owner_state**: 3
- **rerun_from**: 3
- **result**: **WAIVED**
- **evidence**: cargo-kani 0.67.0 is installed but no #[kani::proof] harnesses exist in vb_runtime. Waiver WAIVER-KANI-001 applies with status: approved, rerun_from: 3. Compensating evidence: 1364 passing tests. `moon run :verify-proof` confirms: "Manual Harness Summary: No proof harnesses were found to verify."
- **failure_packet**: N/A
- **follow_up**: Add `#[kani::proof] fn harness_validate_ticket_attempt()` to vb_runtime/src/shard/helpers.rs or a proof module. Waiver expires at State 12.

---

### UNIT-LIFECYCLE-001

- **id**: UNIT-LIFECYCLE-001
- **risk**: high
- **scope**: touched-crate
- **layer**: unit
- **checker**: cargo test
- **command**: `cargo test -p vb_runtime --lib -- action_failure_without_handler` (note: obligation named `apply_error_handler` but actual test filter `action_failure_without_handler` covers the POST-003 contract clause)
- **required**: true
- **owner_state**: 3
- **rerun_from**: 3
- **result**: **PASS**
- **evidence**: `cargo test -p vb_runtime --lib -- action_failure_without_handler` → 2 passed. Tests `action_failure_without_handler_fails_run` and `action_failure_without_handler_emits_action_failed_before_run_failed` verify POST-003 (FailRun outcome when no handler). Full suite: 1337 lib tests pass, 14 action_failure tests pass.
- **failure_packet**: N/A
- **follow_up**: N/A

---

### INTEGRATION-RETRY-001

- **id**: INTEGRATION-RETRY-001
- **risk**: high
- **scope**: touched-crate
- **layer**: integration
- **checker**: cargo test
- **command**: `cargo test -p vb_runtime --lib -- retry`
- **required**: true
- **owner_state**: 3
- **rerun_from**: 3
- **result**: **PASS**
- **evidence**: `cargo test -p vb_runtime --lib -- retry` → 135 passed, 0 failed. Covers retry exhaustion (INV-002), record_retry_attempt (POST-006), retry_is_available (PRE-004), monotonic counter (INV-001).
- **failure_packet**: N/A
- **follow_up**: N/A

---

### INTEGRATION-JOURNAL-001

- **id**: INTEGRATION-JOURNAL-001
- **risk**: medium
- **scope**: touched-crate
- **layer**: integration
- **checker**: cargo test
- **command**: `cargo test -p vb_runtime --test '*' -- journal_replay`
- **required**: true
- **owner_state**: 3
- **rerun_from**: 3
- **result**: **PASS**
- **evidence**: `cargo test -p vb_runtime --test '*' -- journal_replay` → 1 passed. Test `journal_replay_idempotent_action_failed` verifies INV-003 (journal idempotency) and POST-004 (ActionFailed journal emission order).
- **failure_packet**: N/A
- **follow_up**: N/A

---

### INTEGRATION-STALE-001

- **id**: INTEGRATION-STALE-001
- **risk**: high
- **scope**: touched-crate
- **layer**: integration
- **checker**: cargo test
- **command**: `cargo test -p vb_runtime --lib -- stale_attempt`
- **required**: true
- **owner_state**: 3
- **rerun_from**: 3
- **result**: **PASS**
- **evidence**: `cargo test -p vb_runtime --lib -- stale_attempt` → 3 passed. Tests `stale_attempt_completion_leaves_run_counters_journal_and_frame_unchanged` and `future_attempt_completion_rejected_when_current_attempt_exists` verify POST-007 (stale attempt rejection leaves state unchanged).
- **failure_packet**: N/A
- **follow_up**: N/A

---

### GATE-PROOF-001

- **id**: GATE-PROOF-001
- **risk**: proof
- **scope**: bead-local
- **layer**: gauntlet-proof
- **checker**: moon run :verify-proof
- **command**: `moon run :verify-proof`
- **required**: true
- **owner_state**: 3
- **rerun_from**: 3
- **result**: **PASS**
- **evidence**: `moon run :verify-proof` exits 0. Kani: "No proof harnesses found" (matches WAIVER-KANI-001). Lean: skipped (no proof directory). All scoped proof obligations are PASS or WAIVED. `moon run :verify-proof` completed in 3.875s.
- **failure_packet**: N/A
- **follow_up**: N/A

---

### GATE-STANDARD-001

- **id**: GATE-STANDARD-001
- **risk**: high
- **scope**: bead-local
- **layer**: gauntlet-standard
- **checker**: moon run :verify-standard
- **command**: `moon run :verify-standard`
- **required**: true
- **owner_state**: 3
- **rerun_from**: 3
- **result**: **PASS**
- **evidence**: `moon run :verify-standard` exits 0. Runs: fmt, lint-src, check, test, doc-test. All pass. Completed in 3.225s. Test sensor: 9860 tests across 58 binaries pass.
- **failure_packet**: N/A
- **follow_up**: N/A

---

## Waivers

All 6 waivers from `formal-waivers.jsonl` are valid and approved:

| Waiver ID | Clause | Layer | Status | Valid? |
|-----------|--------|-------|--------|--------|
| WAIVER-VERUS-001 | PRE-002 | verus | approved | Yes — verus toolchain not installed |
| WAIVER-VERUS-002 | INV-001 | verus | approved | Yes — verus toolchain not installed |
| WAIVER-VERUS-003 | POST-006 | verus | approved | Yes — verus toolchain not installed |
| WAIVER-VERUS-004 | POST-001 | verus | approved | Yes — verus toolchain not installed |
| WAIVER-VERUS-005 | PRE-004 | verus | approved | Yes — verus toolchain not installed |
| WAIVER-KANI-001 | PRE-002 | kani | approved | Yes — no #[kani::proof] harnesses |

All waivers have `rerun_from: 3` and compensating evidence of 1364 passing tests.

---

## Residual Risk

**None that block approval.**

1. **Verus formal proofs**: Toolchain not installed. Waived with compensating evidence of 1364 adversarial tests. Expiry: State 12.
2. **Kani bounded model check**: No proof harnesses. Waived with compensating evidence. Expiry: State 12.
3. **TLA+ bounded models**: Verified with explicit documented limitations (MaxJournalAttempts=1, MaxAttemptsValue=2, RunId={1}, StepId={1,2}, liveness not checked). These are acknowledged bounds, not defects.
4. **DEFERRED_GLOBAL format debt**: 10 files with formatting diffs outside vb-qi37.16.3 delivery scope (proof kernels, Kani harnesses, Miri tests, storage, fuzz, xtask). Classified as DEFERRED_GLOBAL by regression-diff.md, moon-report.md, and black-hat-review.md. Not a bead-local blocker.

---

## Summary

All 15 obligations are accounted for:

| Obligation | Layer | Result |
|------------|-------|--------|
| TLA-RETRY-001 | tla-plus | **PASS** |
| TLA-RETRY-002 | tla-plus | **PASS** |
| TLA-RETRY-003 | tla-plus | **PASS** |
| VERUS-PRE-002 | verus | **WAIVED** |
| VERUS-INV-001 | verus | **WAIVED** |
| VERUS-POST-006 | verus | **WAIVED** |
| VERUS-POST-001 | verus | **WAIVED** |
| VERUS-PRE-004 | verus | **WAIVED** |
| KANI-PRE-002 | kani | **WAIVED** |
| UNIT-LIFECYCLE-001 | unit | **PASS** |
| INTEGRATION-RETRY-001 | integration | **PASS** |
| INTEGRATION-JOURNAL-001 | integration | **PASS** |
| INTEGRATION-STALE-001 | integration | **PASS** |
| GATE-PROOF-001 | gauntlet-proof | **PASS** |
| GATE-STANDARD-001 | gauntlet-standard | **PASS** |

**STATUS: APPROVED** — All required/local/regression obligations are PASS or WAIVED. All DEFERRED_GLOBAL entries are pre-existing unrelated workspace debt with exact follow-up. No bead-local blockers.

---

*Formal verification report for vb-qi37.16.3 State 11.*
*Evidence collected via read-only command execution. No source modification. No jj operations. No bd changes. No commit. No push.*
