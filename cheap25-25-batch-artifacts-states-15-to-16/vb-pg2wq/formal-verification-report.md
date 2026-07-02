# Formal Verification Report — vb-pg2wq

## Bead

- **Bead**: vb-pg2wq
- **Description**: Tests: make duplicate-event test assert one exact contract (P1 bug)
- **Phase**: State 12 — Formal Verification
- **Workspace**: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-pg2wq/wt
- **jj workspace**: cheap25-vb-pg2wq
- **jj root**: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-pg2wq/wt
- **Invocation ID**: formal-verifier-vb-pg2wq-state12
- **Started**: 2026-07-01T22:15:00Z
- **Completed**: 2026-07-01T22:18:30Z

## Executive Summary

| Classification | Count | Details |
|----------------|-------|---------|
| PASS | 3 | PO-vb-pg2wq-001, PO-vb-pg2wq-002, PO-vb-pg2wq-003 |
| FAIL_LOCAL | 0 | |
| FAIL_REGRESSION | 0 | |
| FAIL_GLOBAL | 0 | |
| BLOCKED_TOOLING | 0 | |
| BLOCKED_DEAD_CODE | 0 | |
| WAIVED | 0 | |

**Final State**: All 3 proptest obligations PASS. 6 weak `matches!(.., JournalError::DuplicateEvent { .. })` occurrences rewritten to field-bound `let-else` + `assert_eq!` pattern (functionally equivalent to the planned `matches!(..) if guard` form, mirrors reference strong pattern at `crates/vb_storage/src/tests.rs:1344-1367`). Production contract at `crates/vb_storage/src/batch/append_event.rs:61-67` preserved verbatim.

---

## Obligation Closure Roll-up

| Obligation | Risk | Verifier | Functions | Classification | Evidence |
|------------|------|----------|-----------|----------------|----------|
| PO-vb-pg2wq-001 | field_sensitivity | proptest | ps001, ps003, ps008, ps009 | **PASS** | `evidence/state12_test_ps001_duplicate_rejected.log`, `state12_test_ps003_dup_fields.log`, `state12_test_ps008_dup_before_queue.log`, `state12_test_ps009_dup_rejected.log`, `state12_vb_storage_all_tests_full.log` |
| PO-vb-pg2wq-002 | field_sensitivity | proptest | ps004_no_persist, ps004_empty_commit_after_rej | **PASS** | `evidence/state12_test_ps004_no_persist.log`, `state12_test_ps004_empty_commit_after_rej.log`, `state12_vb_storage_all_tests_full.log` |
| PO-vb-pg2wq-003 | equality | proptest (source-lint umbrella) | cross-cutting pattern-discipline scan over 4 target files | **PASS** | `evidence/state12_weak_pattern_scan.txt` (0 hits), `state12_check_test_integrity.log` (PASS), `state12_cargo_fmt.log` (drift in 3 unrelated files — see Residual Risks) |

---

## Gate Execution Details (State 12)

All commands executed from `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-pg2wq/wt` (jj root).

### PO-vb-pg2wq-001 (field-bound guard — 4 functions)

| Function | Command | Exit | Result | Evidence |
|----------|---------|------|--------|----------|
| ps001_duplicate_rejected | `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_001 ps001_duplicate_rejected --no-fail-fast` | 0 | 1 passed; 0 failed; 6 filtered out; finished in 1.44s | `evidence/state12_test_ps001_duplicate_rejected.log` |
| ps003_dup_fields | `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_003 ps003_dup_fields --no-fail-fast` | 0 | 1 passed; 0 failed; 5 filtered out; finished in 1.57s | `evidence/state12_test_ps003_dup_fields.log` |
| ps008_dup_before_queue | `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_008 ps008_dup_before_queue --no-fail-fast` | 0 | 1 passed; 0 failed; 4 filtered out; finished in 1.55s | `evidence/state12_test_ps008_dup_before_queue.log` |
| ps009_dup_rejected | `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_009 ps009_dup_rejected --no-fail-fast` | 0 | 1 passed; 0 failed; 5 filtered out; finished in 1.51s | `evidence/state12_test_ps009_dup_rejected.log` |

**Raw stdout (ps001):**
```
running 1 test
test ps001_duplicate_rejected ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out; finished in 1.44s
```

**Aggregate**: All 4 functions pass under the field-bound assertion. Production contract at `crates/vb_storage/src/batch/append_event.rs:61-67` returns `Err(JournalError::DuplicateEvent { run: event.run_id(), seq: event.seq() })`, which exactly matches the proptest inputs `RunId::new(run)` / `EventSeq::new(seq)` (or `EventSeq::new(0)` for the fixed-seq variant). No silent-overwrite regression (`Ok(())` rejected by `let-else` arm). No sibling-variant confusion (any non-`DuplicateEvent` variant rejected by exhaustive `let-else` destructure).

### PO-vb-pg2wq-002 (field-bound guard + secondary invariants — 2 functions)

| Function | Command | Exit | Result | Evidence |
|----------|---------|------|--------|----------|
| ps004_no_persist | `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_004 ps004_no_persist --no-fail-fast` | 0 | 1 passed; 0 failed; 4 filtered out; finished in 1.57s | `evidence/state12_test_ps004_no_persist.log` |
| ps004_empty_commit_after_rej | `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_004 ps004_empty_commit_after_rej --no-fail-fast` | 0 | 1 passed; 0 failed; 4 filtered out; finished in 1.56s | `evidence/state12_test_ps004_empty_commit_after_rej.log` |

**Secondary assertions preserved verbatim** (per `contract.md` §File 3 Function A/B):

- `ps004_no_persist`: `prop_assert!(b2.is_aborted())`, `prop_assert!(matches!(commit_result, Err(JournalError::BatchAborted)))`, `prop_assert_eq!(events.len(), 1)` — all present and passing.
- `ps004_empty_commit_after_rej`: `prop_assert!(b2.is_aborted())`, `prop_assert!(matches!(commit_result, Err(JournalError::BatchAborted)))` — all present and passing.

### PO-vb-pg2wq-003 (cross-cutting pattern-discipline + source-lint)

| Gate | Command | Exit | Result | Evidence |
|------|---------|------|--------|----------|
| weak-pattern scan | `rtk rg -n -- 'JournalError::DuplicateEvent \{ \.\. \}' crates/vb_storage/tests/proptest_vb_vzcuf_PS_001.rs crates/vb_storage/tests/proptest_vb_vzcuf_PS_003.rs crates/vb_storage/tests/proptest_vb_vzcuf_PS_004.rs crates/vb_storage/tests/proptest_vb_vzcuf_PS_008.rs crates/vb_storage/tests/proptest_vb_vzcuf_PS_009.rs` | 0 | 0 hits across all 5 files | `evidence/state12_weak_pattern_scan.txt` |
| test-integrity | `bash scripts/check-test-integrity.sh` | 0 | test integrity: PASS base=@- | `evidence/state12_check_test_integrity.log` |
| fmt (full repo) | `cargo fmt --all --check` | 1 | drift in 3 unrelated files (see Residual Risks); 5 changed test files formatting-clean | `evidence/state12_cargo_fmt.log` |

### Regression Sweep (cargo test -p vb_storage)

| Command | Exit | Result | Evidence |
|---------|------|--------|----------|
| `cargo test -p vb_storage --tests --no-fail-fast` | 0 | **1669 passed; 0 failed; 16 suites** (full log: 1766 lines) | `evidence/state12_vb_storage_all_tests_full.log`, `evidence/state12_vb_storage_test_results.txt` |

**Suite breakdown (sum = 1669):**

| Suite | Count |
|-------|-------|
| vb_storage lib (lib unit tests) | 1530 |
| proptest_vb_vzcuf_PS_001 | 7 |
| proptest_vb_vzcuf_PS_003 | 6 |
| proptest_vb_vzcuf_PS_004 | 5 |
| proptest_vb_vzcuf_PS_007 | 6 |
| proptest_vb_vzcuf_PS_008 | 5 |
| proptest_vb_vzcuf_PS_009 | 6 |
| proptest_vb_vzcuf_PS_005 | 3 |
| proptest_vb_vzcuf_PS_006 | 4 |
| proptest_vb_vzcuf_PS_002 | 29 |
| proptest_vb_vzcuf_PS_010 | 5 |
| journal_recovery tests | 42 |
| recovery_property_tests | 7 |
| vb_core_atomic_admission_red | 0 |
| (plus other internal suites) | 8 + 6 = 14 |
| **Total** | **1669** |

---

## Verdict

All 3 proof obligations PASS. No failures, no waivers, no BLOCKED_TOOLING, no BLOCKED_DEAD_CODE.

- **3 obligations PASS**: All proptest functions strengthened to field-bound assertions pass; all secondary invariants preserved; cross-cutting pattern-discipline scan shows 0 remaining weak `..` patterns in target files; cargo test sweep shows 1669 passed, 0 failed across 16 suites.
- **0 waivers**: `waiver-candidates.jsonl` empty by design (test-only scope per `contract.md` Obligation 6).
- **0 regressions**: Production contract at `crates/vb_storage/src/batch/append_event.rs:61-67` preserved verbatim; no `Cargo.toml` modified; no forbidden constructs introduced.

---

## Production-binding Gate (MANDATORY)

Pre-check `bash scripts/check-verus-production-binding.sh` is **NOT required** for this bead: no Verus spec files were added or modified by this bead. This bead strengthens test assertions against an existing production contract; no Verus mirror is in scope per `contract.md` §Obligation 6 (No Production Change) and the proof-plan-review.md §Validated Strengths.

Pre-check `bash scripts/check-production-inner-drift.sh` is **NOT required**: no `production_inner/*` mirror files are in scope.

---

## Residual Risks

### RR-1: Pre-existing BLOCK_GLOBAL `cargo fmt --all --check` drift (NOT introduced by this bead)

`cargo fmt --all --check` exits non-zero with drift in 3 unrelated files:
- `crates/vb_core/src/lib.rs:26` (mod declaration ordering: `pub mod value` and `pub mod time`)
- `crates/vb_core/src/time.rs:71` (trailing blank line)
- `crates/vb_runtime/src/frame_pool/tests.rs:85, 114, 139` (long-line wrapping on `assert_eq!` calls)

**None of these files are in this bead's scope.** The 5 changed test files (`proptest_vb_vzcuf_PS_001/003/004/008/009.rs`) are formatting-clean. This drift is documented as BLOCK_GLOBAL prerequisite repair across multiple cheap25 beads (e.g., `vb-cn2v4`); it is not a bead defect.

**Disposition**: PASS_LOCAL for PO-003 source-lint scan. Pattern-discipline weak-pattern scan returns 0 hits in the 4 target files; `check-test-integrity.sh` returns 0; test runner returns 1669 passed; cargo fmt drift is out of scope.

### RR-2: BLOCK_GLOBAL compile errors in `vb_compile/tests/common/mod.rs`

Pre-existing compile errors in `crates/vb_compile/tests/common/mod.rs` (unresolved `vb_compile::WorkflowSourceParts`, 13 cascading `new is private` errors). These errors are NOT in this bead's scope (`-p vb_storage` is in scope; `vb_compile` is not). `cargo test -p vb_storage --tests --no-fail-fast` is the canonical in-scope gate and returns 1669 passed. Documented in `implementation.md` §Residual Risks.

---

## Bridge Plan (proof → Rust implementation)

Every obligation has a 1:1 file/function/line target:

| Obligation | Proptest function | File:line | Production target |
|------------|-------------------|-----------|-------------------|
| PO-vb-pg2wq-001 | ps001_duplicate_rejected | `crates/vb_storage/tests/proptest_vb_vzcuf_PS_001.rs:69-79` | `crates/vb_storage/src/batch/append_event.rs:61-67` |
| PO-vb-pg2wq-001 | ps003_dup_fields | `crates/vb_storage/tests/proptest_vb_vzcuf_PS_003.rs:55-65` | `crates/vb_storage/src/batch/append_event.rs:61-67` |
| PO-vb-pg2wq-001 | ps008_dup_before_queue | `crates/vb_storage/tests/proptest_vb_vzcuf_PS_008.rs:27-36` | `crates/vb_storage/src/batch/append_event.rs:61-67` |
| PO-vb-pg2wq-001 | ps009_dup_rejected | `crates/vb_storage/tests/proptest_vb_vzcuf_PS_009.rs:27-37` | `crates/vb_storage/src/batch/append_event.rs:61-67` |
| PO-vb-pg2wq-002 | ps004_no_persist | `crates/vb_storage/tests/proptest_vb_vzcuf_PS_004.rs:39-54` | `crates/vb_storage/src/batch/append_event.rs:61-67` + `append_event.rs:62` (self.aborted) + `commit.rs` (BatchAborted) |
| PO-vb-pg2wq-002 | ps004_empty_commit_after_rej | `crates/vb_storage/tests/proptest_vb_vzcuf_PS_004.rs:84-98` | same as above |
| PO-vb-pg2wq-003 | (cross-cutting) | `crates/vb_storage/tests/proptest_vb_vzcuf_PS_001/003/004/008/009.rs` | `crates/vb_storage/src/batch/append_event.rs:61-67` |

The Kani binding-strengthened lane decision (`VLD-vb-pg2wq-seed-kani-binding-strengthened`) records that the existing Kani harness at `crates/vb_storage/src/kani_vb_vzcuf_ps004.rs:48-59` already models the field-bound `DuplicateEvent { run, seq }` contract; no new Kani harness is required. The runtime test rewrite strengthens the runtime↔Kani alignment.

---

## Verification Ledger Summary

See `verification-ledger.jsonl` for the 3-row detailed ledger with raw command text and evidence references. Schema `verification-ledger/v1`; all rows are `classification: PASS`, `exit_status: 0`, `behavior_affecting: false`.

---

## Formal Waivers Summary

`formal-waivers.jsonl` is **empty** (zero bytes). No waivers required. Every planned obligation is bounded to the test surface and PASSES; the production contract is preserved verbatim; no behavior-affecting waiver is made.

---

## Files

- `formal-verification-report.md` (this file)
- `verification-ledger.jsonl` (3 rows; verification-ledger/v1 schema)
- `formal-waivers.jsonl` (empty; non-behavior waiver not required)
- `evidence/state12_test_ps001_duplicate_rejected.log`
- `evidence/state12_test_ps003_dup_fields.log`
- `evidence/state12_test_ps004_no_persist.log`
- `evidence/state12_test_ps004_empty_commit_after_rej.log`
- `evidence/state12_test_ps008_dup_before_queue.log`
- `evidence/state12_test_ps009_dup_rejected.log`
- `evidence/state12_vb_storage_all_tests_full.log` (1766 lines; raw `cargo test` output)
- `evidence/state12_vb_storage_test_results.txt` (per-suite summary)
- `evidence/state12_weak_pattern_scan.txt` (0 hits)
- `evidence/state12_check_test_integrity.log` (PASS)
- `evidence/state12_cargo_fmt.log` (drift in 3 unrelated files — see RR-1)
- `evidence/state12_cargo_fmt_changed_files.log` (same)
- `evidence/state12_input_artifact_hashes.txt` (input SHA-256 inventory)
- `evidence/state12_output_artifact_hashes.txt` (output SHA-256 inventory)

---

## Closure

All planned obligations closed. PASS = 3, FAIL = 0, WAIVED = 0. Ready for State 13 (black-hat review).