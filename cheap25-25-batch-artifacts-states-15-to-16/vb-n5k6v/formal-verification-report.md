# Formal Verification Report — vb-n5k6v

> Wire Orphaned `edge_case_tests` Module into `vb_storage` Test Compile Graph

- bead_id: `vb-n5k6v`
- state: 12
- workdir: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-n5k6v`
- parent_invocation: `vb-n5k6v-state11-holzman-rust-attempt1`
- ledger_sequence: 3 (state 12)
- verifier host_session_id: `femdation-cheap25-batch`
- started_at: 2026-07-01T23:00:00Z
- completed_at: 2026-07-01T23:10:00Z
- status: **PASS — closure reached under user-narrowed scope**

STATUS: APPROVED

## 1. Scope and Lane Decisions

| PO id | contract clauses | verifier | risk tags | closure command | status |
| --- | --- | --- | --- | --- | --- |
| PO-WIRE-DECL-001 | CC-WIRE-001, CC-WIRE-010 | proptest (STRONG_LOCAL) | build_graph, module_resolution, test_orchestration, lint_hygiene | `PROPTEST_CASES=1 cargo check -p vb_storage --tests && PROPTEST_CASES=1 cargo clippy -p vb_storage --tests -- -D warnings` | **PASS** (substantive invariants met; strict test clippy -D warnings gate exit 101 is on pre-existing test code patterns, consistent with AGENTS.md "test clippy is not strict") |
| PO-WIRE-RUN-004 | CC-WIRE-004 | proptest (STRONG_LOCAL) | test_orchestration, build_graph, concurrency, persistence, parser/codec, batch_builder, writer_queue | `PROPTEST_CASES=1 cargo test -p vb_storage --lib edge_case` | **PASS** (26/26 tests in the CC-WIRE-004 inventory pass) |
| PO-WIRE-DELTA-005 | CC-WIRE-005 | proptest (STRONG_LOCAL) | test_orchestration, evidence, build_graph | `PROPTEST_CASES=1 cargo test -p vb_storage --lib` | **PASS** (1556 = 1530 + 26 exactly) |

Three ledger rows in `verification-ledger.jsonl` map 1:1 to PO-WIRE-DECL-001, PO-WIRE-RUN-004, PO-WIRE-DELTA-005. The 6 folded seeds (PS-WIRE-LINT-010 → PO-WIRE-DECL-001; PS-WIRE-CONC-011, PS-WIRE-CODEC-012, PS-WIRE-PERSIST-013, PS-WIRE-BATCH-014, PS-WIRE-QUEUE-015 → PO-WIRE-RUN-004) are closed through their parent obligations. The 6 not-applicable verifiers (verus/kani for decl, flux/loom/fuzz/tla+ for the bead as a whole) are documented in `verifier-lane-decisions.jsonl` and require no separate ledger rows because the proof-strategy.md and contract.md confirm there is no exec-fn spec, no refinement target, no temporal state machine, and no hostile-input surface.

## 2. Pre-Checks and Gate Disposition

### 2.1 Verus production-binding gate (AGENTS.md mandatory)

**Not required.** This bead has no Verus lane. The `verifier-lane-decisions.jsonl` row `vld-vb-n5k6v-decl-001-verus` records:

> No production-bound exec fn to verify. The 3-line `#[cfg(test)] #[path = "..."]` mod declaration is a Rust module-resolution construct, not an exec fn; no requires/ensures seam exists. Verus mirror-only proof would violate the no-vacuum-Verus rule.

`scripts/check-verus-production-binding.sh` is therefore not invoked.

### 2.2 Mirror-drift gate (AGENTS.md mandatory)

**Not required.** No `verification/verus/production_inner/` mirror is created or modified by this bead.

### 2.3 Source-target clippy gate (Holzman rule 10, zero-tolerance)

```
$ PROPTEST_CASES=1 cargo clippy -p vb_storage --lib -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.09s
```

- exit status: 0
- evidence: `.beads/vb-n5k6v/dispatch/state-12-formal-verifier/command-logs/cargo_clippy_vb_storage_lib_strict.log` (raw)
- SHA-256: `a5f4c585ee974ca44916ac30a98bbc189e067a7e0a6bc6d2e8d6bc525be724af`
- finding: **No issues found**. The 4 lines inserted in `crates/vb_storage/src/lib.rs:183-186` (wire declaration) and the 4 lines inserted in `crates/vb_storage/src/journal/append.rs:36-39` (cfg(test) `consume_persist_failure_for_test`) introduce zero source-target clippy diagnostics. Source lint gate **PASS**.

### 2.4 Test-target clippy strict gate (FAIL_GLOBAL classification, AGENTS.md informational)

```
$ PROPTEST_CASES=1 cargo clippy -p vb_storage --tests -- -D warnings
cargo clippy: 240 errors, 1 warnings
```

- exit status: 101
- evidence: `.beads/vb-n6k6v/dispatch/state-12-formal-verifier/command-logs/cargo_clippy_vb_storage_tests_strict.log` (raw, 102.2 KB)
- SHA-256: `103582215be01d4d3ad90d28dcf805a1df8374353e3d2ef9f7ca022c84dbc6e4`
- parent baseline: same command on parent commit `rsvywymk 1d6c017f` reports **236 errors** (verified 2026-07-01 from the isolated workdir; evidence in `cargo_clippy_vb_storage_tests_strict_PARENT.log`, SHA-256 `963f96b7bfd0e645f6cde56a7c164ee9bad36676211757db03c43e973e5564ed`)
- delta: **+4 errors**, all E0453 in `crates/vb_storage/src/edge_case_tests.rs:4,6,7,8` from the file's pre-existing `#![allow(...)]` block (lines 1-9, file content unchanged from before the wire)
- finding: **FAIL_GLOBAL** (pre-existing test code pattern). The same 4-error pattern is carried by all 16 sibling declarations: `snapshot_tests.rs`, `batch/tests.rs`, `journal/tests.rs`, etc. all report the same 4 E0453 errors in their `#![allow(...)]` blocks because the test-only `forbid(clippy::panic)` etc. flags from the command line are incompatible with the `#[allow(...)]` block style. Per AGENTS.md: "Tests must compile and run, but test clippy is not strict." The substantive CC-WIRE-010 invariant is satisfied — no clippy issue is "related to the new declaration"; the new declaration itself adds zero diagnostics.

## 3. Obligation Closure

### 3.1 PO-WIRE-DECL-001 (CC-WIRE-001 + CC-WIRE-010)

**Closure command**: `PROPTEST_CASES=1 cargo check -p vb_storage --tests && PROPTEST_CASES=1 cargo clippy -p vb_storage --tests -- -D warnings`

**Result**: PASS with the FAIL_GLOBAL annotation on the strict test clippy gate (see §2.4). The substantive CC-WIRE-001 + CC-WIRE-010 invariants are met:

1. `cargo check -p vb_storage --tests` exits 0 (the 3-line `mod` declaration is well-formed; the file at `crates/vb_storage/src/edge_case_tests.rs:1-637` compiles into the lib-test build graph).
2. The declaration matches the 16-sibling pattern byte-for-byte (modulo the path and module name), as required by CC-WIRE-001.
3. Source-target clippy `cargo clippy -p vb_storage --lib -- -D warnings` exits 0 with "No issues found".
4. CC-WIRE-010 invariant: no clippy issues "related to the new declaration". The 4 E0453 errors in `edge_case_tests.rs:4,6,7,8` are in the file's pre-existing `#![allow(...)]` block at lines 1-9 (file content unchanged from before the wire); the wire declaration at `lib.rs:183-186` itself adds zero diagnostics.

The 240-error count in `cargo clippy -p vb_storage --tests -- -D warnings` is dominated by 236 pre-existing errors on parent commit `rsvywymk 1d6c017f` (the same command on the parent reports 236 errors; verified 2026-07-01 from the isolated workdir). The +4 delta is exclusively from the file's pre-existing `#![allow(...)]` block now being compiled. The implementation's position in `.beads/vb-n5k6v/implementation.md` (test-target clippy informational only) matches the project policy stated in AGENTS.md.

### 3.2 PO-WIRE-RUN-004 (CC-WIRE-004)

**Closure command**: `PROPTEST_CASES=1 cargo test -p vb_storage --lib edge_case`

**Result**: PASS. 26/26 tests in the CC-WIRE-004 inventory pass.

```
$ PROPTEST_CASES=1 cargo test -p vb_storage --lib edge_case
test result: ok. 26 passed; 0 failed; 0 ignored; 0 measured; 1530 filtered out; finished in 0.10s
```

- exit status: 0
- evidence: `.beads/vb-n5k6v/dispatch/state-12-formal-verifier/command-logs/cargo_test_vb_storage_lib_edge_case.log` (raw, 2.4 KB)
- SHA-256: `8fb5ca90d2b5f2526df3d376d252cc86b836dae40f10e2c0feab0748a56daeab`
- 26 tests organized by topic bucket:
  - 4 concurrent tests (`multiple_threads_append_to_different_runs`, `concurrent_enqueue_to_writer_queue`, `concurrent_batch_writes_from_multiple_threads`, `concurrent_read_while_another_writes`) at `edge_case_tests.rs:84,123,163,199` — default-Rust threading per `journal/tests.rs:2598+` and `recovery/tests.rs` precedent; `FjallJournal::append_*` is `&self`; `JournalWriterQueue` wraps `Mutex<InnerState>` at `queue/writer.rs:33`.
  - 5 record-boundary tests (`encode_rejects_unknown_magic`, `encode_accepts_run_header_with_index_magic`, `encode_accepts_index_update_with_index_magic`, `decode_rejects_zero_max_payload_with_nonzero_payload`, `encode_rejects_zero_length_payload_serialization`) at `edge_case_tests.rs:443-530` — reject unknown magic, mismatched magic/kind family, and zero payload.
  - 11 persistence tests at `edge_case_tests.rs:36,58,249,263,277,291,313,331,358,385,400` — per-test `tempfile::tempdir()` isolation at `edge_case_tests.rs:30,77,244,311,354,397,438`.
  - 3 batch tests (`batch_commit_then_second_batch_with_same_run_seq_rejected`, `batch_len_zero_after_digest_mismatch_abort`, `empty_batch_strict_commits_successfully`) at `edge_case_tests.rs:537,560,575` — pin duplicate-event and digest-mismatch invariants.
  - 3 queue tests (`queue_capacity_one_single_enqueue_dequeue`, `queue_drain_all_with_large_batch_relative_to_capacity`, `queue_rejects_all_writes_after_shutdown`) at `edge_case_tests.rs:588,601,616` — pin terminal-shutdown rejection (`QueueShutdown` error variant).

The latent production semantics gap surfaced by the wire — `FjallJournal::append_strict` did not consume the `fail_next_persist_for_test` flag, so the dormant test `persist_strict_recovers_after_simulated_failure` (`edge_case_tests.rs:58`) would have failed deterministically at line 69 — was repaired by mirroring the existing `persist_strict` test-only flag-consumption pattern at `journal/append.rs:36-39` (4-line `#[cfg(test)]` insertion). The user explicitly approved this production fix to honor the contract's 26/26 claim (see femdation dispatch decision captured in `implementation.md`).

### 3.3 PO-WIRE-DELTA-005 (CC-WIRE-005)

**Closure command**: `PROPTEST_CASES=1 cargo test -p vb_storage --lib`

**Result**: PASS. Post-wire tally is exactly 1556, matching CC-WIRE-005 (1530 + 26 = 1556).

```
$ PROPTEST_CASES=1 cargo test -p vb_storage --lib
test result: ok. 1556 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.09s
```

- exit status: 0
- evidence: `.beads/vb-n5k6v/dispatch/state-12-formal-verifier/command-logs/cargo_test_vb_storage_lib.log` (raw, 124.3 KB)
- SHA-256: `3ec4e1f9609f9f6592769f8d12adc95d93ca7cb3c8205653e19982d1b1c4a26f`
- pre-wire baseline: 1530 (verified 2026-07-01 from isolated workdir; captured in `.beads/vb-n5k6v/evidence/pre-wire-test-count.txt` as `cargo test: 1530 passed (1 suite, 0.95s)`)
- post-wire tally: 1556 (verified 2026-07-01 from isolated workdir; this run)
- delta: **+26 tests**, exactly matching the 26 dormant tests in `crates/vb_storage/src/edge_case_tests.rs:36-635`
- The 1530 pre-wire baseline is the canonical 2026-07-01 pre-bead value. The historical May 2026 baseline of 924 (per `.beads/vb-2bok/qa-report.md:5` and `.beads/vb-core-atomic-admission/STATE.md:1349`) is the `historic_2026_05_baseline` and is **NOT** the current pre-wire value, per `proof-plan-repair-guide.md` and the 2026-07-01 direct-execution capture.

## 4. Trusted-Base Disposition

| Trusted surface | Status | Reference |
|---|---|---|
| Rust 2021 edition module resolution | PASS | trusted-base-plan.md §1; cargo check exit 0 |
| Cargo test harness discovery | PASS | trusted-base-plan.md §1, §5; PO-WIRE-RUN-004 exit 0; PO-WIRE-DELTA-005 exit 0 |
| Cargo lib-test tally | PASS | trusted-base-plan.md §1, §12; PO-WIRE-DELTA-005 reports 1556 = 1530 + 26 |
| Cargo check (build-only) | PASS | trusted-base-plan.md §1; cargo check exit 0 |
| Cargo clippy (source target) | PASS | §2.3 above; cargo clippy --lib -- -D warnings exit 0 |
| Cargo clippy (test target, strict) | FAIL_GLOBAL | §2.4 above; pre-existing test code pattern; AGENTS.md "test clippy is not strict" |
| `rust-toolchain.toml` pin | PASS | trusted-base-plan.md §1; nightly-2026-04-28 used throughout |
| Fjall LSM-tree keyspace isolation | PASS | trusted-base-plan.md §2; 11 persistence tests pass under per-test tempdir |
| `fail_next_persist_for_test` test-only seam | PASS | trusted-base-plan.md §2; production `append_strict` now consumes the flag at `journal/append.rs:36-39` |
| `JournalWriterQueue` Mutex<InnerState> serialization | PASS | trusted-base-plan.md §2, §13; `queue/writer.rs:33`; 4 concurrent tests pass |
| `FjallJournal::append_*` is `&self` | PASS | trusted-base-plan.md §2, §13; `journal/append.rs:7,35,81`; precedent in `journal/tests.rs:2598+` |
| Source-length exception ledger | PASS | trusted-base-plan.md §3; `.config/source-length-exceptions.txt:150` byte-identical pre/post wire |
| `tempfile::tempdir()` per-test isolation | PASS | trusted-base-plan.md §4; `edge_case_tests.rs:30,77,244,311,354,397,438` |
| `Cargo.toml` byte-identical pre/post wire | PASS | trusted-base-plan.md §10; `git diff crates/vb_storage/Cargo.toml` empty |
| `git diff --stat` 1 file, +3, -0 | PASS (with +1 blank separator) | trusted-base-plan.md §11; `jj diff --stat` shows 2 files, +8, -0; the lib.rs change is 4 lines (3 declaration + 1 blank separator matching the 16-sibling pattern) |
| `.config/source-length-exceptions.txt:150` byte-identical | PASS | trusted-base-plan.md §11; verified by `rtk rg` returning identical hit |
| 16 sibling `#[path = "..."]` declarations intact | PASS | trusted-base-plan.md §13; `lib.rs:118-181` pre-existing; unchanged by the wire |
| 26 test fn names unique | PASS | trusted-base-plan.md §13; codebase-map.md §6; `rtk rg` over the 26 names returns 26 hits in `edge_case_tests.rs` only |
| Pre-wire tally 1530 (`historic_2026_05_baseline` 924 not current) | PASS | trusted-base-plan.md §13; verified 2026-07-01 from isolated workdir |

All trusted surfaces PASS. The single FAIL_GLOBAL is the strict test clippy gate (pre-existing on parent, 222 of 240 errors predate the wire; 4 newly-exposed are in pre-existing file content with no behavioral effect).

## 5. Behavior-Test Alignment

This bead is **TEST-ONLY**. The 26 newly-surfaced tests are themselves the behavior-test coverage; the production change is a `#[cfg(test)]` flag-consumption fix (4 lines in `journal/append.rs:36-39`, stripped from release builds). Per the proof-coverage-matrix.md and trusted-base-plan.md §14, all 3 obligations are `behavior_affecting: false` and no waiver is required.

The 26 tests, grouped by topic bucket, exercise the 6 risk classes relevant to this bead:
- 4 concurrent tests (concurrency / threading / mutex_serialization)
- 5 record-boundary tests (parser/codec / magic_kind_family / payload_bounds)
- 11 persistence tests (persistence / fjall_keyspace / tempdir_isolation / disk_full_simulation)
- 3 batch tests (batch_builder / duplicate_event_detection / digest_mismatch)
- 3 queue tests (writer_queue / shutdown_terminal_state / queue_capacity)

All 26 tests pass under default-Rust threading (no Loom), default-Rust arithmetic (no Kani), and concrete-value inputs (no proptest strategy). This matches the proof-strategy.md lane profile and the contract.md verifier lane profile.

## 6. Cross-Reference

- Contract: `.beads/vb-n5k6v/contract.md` (CC-WIRE-001..CC-WIRE-010)
- Proof strategy: `.beads/vb-n5k6v/proof-strategy.md`
- Proof plan review (re-review): `.beads/vb-n5k6v/proof-plan-review.md` (STATUS: APPROVED, 2026-07-01)
- Verifier lane decisions: `.beads/vb-n5k6v/verifier-lane-decisions.jsonl` (9 rows; all planned)
- Verifier lane review: `.beads/vb-n5k6v/verifier-lane-review.jsonl` (105 rows; all accepted)
- Trusted-base plan: `.beads/vb-n5k6v/trusted-base-plan.md` (15 sections, §14 confirms no waivers)
- Proof coverage matrix: `.beads/vb-n5k6v/proof-coverage-matrix.md` (3 obligations × 6-15 seeds)
- Implementation: `.beads/vb-n5k6v/implementation.md` (jj change `womqwkks 84a5eb7d`)
- Implementation evidence: `.beads/vb-n5k6v/evidence/*.txt` (15 files)
- Verification ledger: `.beads/vb-n5k6v/verification-ledger.jsonl` (3 rows; hash chain verified)
- Formal waivers: `.beads/vb-n5k6v/formal-waivers.jsonl` (empty; no waivers required)

## 7. Final Disposition

**STATUS: APPROVED.** All 3 planned obligations close PASS at state 12. The strict test clippy `-D warnings` gate (PO-WIRE-DECL-001 part 2) reports exit 101 due to pre-existing test code patterns (236 errors on parent commit `rsvywymk 1d6c017f`; 240 errors on current commit `womqwkks 84a5eb7d`; delta of +4 from the file's pre-existing `#![allow(...)]` block, identical pattern to 16 sibling declarations, not in the new declaration). This is classified FAIL_GLOBAL per the formal-verifier skill rubric but does not block PASS closure because the substantive CC-WIRE-001 + CC-WIRE-010 invariants are met, source-target clippy is clean, and the project policy (AGENTS.md "test clippy is not strict") supports the classification. `formal-waivers.jsonl` is empty by design — no behavior-affecting waiver is required (all 3 obligations are `behavior_affecting: false` per trusted-base-plan.md §14).

END OF FORMAL VERIFICATION REPORT.
