# Truth Serum Report — vb-r8oso

**bead_id:** vb-r8oso
**title:** Storage: enforce next-sequence-at-write before durable append (P1 bug)
**state:** 14 (Truth Serum Audit)
**workdir:** /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-r8oso
**captured_at:** 2026-07-01T22:15:00Z
**owner_skill:** truth-serum
**parent_invocation_id:** evidence-packaging-cheap25-vb-r8oso-2026-07-01
**input_artifacts:**
- `formal-verification-report.md` (State 12)
- `verification-ledger.jsonl` (7 rows)
- `formal-waivers.jsonl` (empty)
- `black-hat-review.md` (State 13, STATUS: APPROVED)
- `defects.md` (empty)
- `implementation.md` (State 11)
- `contract.md` (C-1..C-12)
- `evidence/downstream-caller-audit.md` (126 lines)
- 9 raw evidence logs in `evidence/`

STATUS: APPROVED

---

## Audit

### 1. Cargo test claims

- **Claim:** `cargo test -p vb_storage --tests --all-features` passes 1,676 tests in 16 suites.
- **Audit:** Re-executed the command on 2026-07-01T21:50:00Z. Raw log at `evidence/raw-cargo-test-vb-storage-tests-all-features.log` (1,756 lines). All 16 `test result: ok.` lines confirm: 1,537 + 29 + 4 + 42 + 3 + 7 + 8 + 6 + 5 + 5 + 6 + 6 + 5 + 6 + 7 + 0 = **1,676 passed, 0 failed, 0 ignored, 0 measured, 0 filtered out.** Exit code 0.
- **Verdict:** TRUE.

- **Claim:** `cargo test -p vb_storage --tests --features kani-sequence-at-write` compiles and passes 1,676 tests.
- **Audit:** Re-executed. Raw log at `evidence/raw-cargo-test-vb-storage-kani-feature.log`. Same 16 suites, same 1,676 passed. Exit code 0. The `kani_sequence_at_write` module is excluded from compilation under `cargo test` because the harness is gated behind `#[cfg(all(kani, feature = "kani-sequence-at-write"))]`, and the `cargo test` build does not set `cfg(kani)`.
- **Verdict:** TRUE.

- **Claim:** `cargo test -p vb_storage --tests --features kani-sequence-at-write --no-run` compiles 16 test executables.
- **Audit:** Re-executed. Raw log at `evidence/raw-cargo-test-kani-feature-no-run.log`. All 16 `Executable ...` lines present. Exit code 0.
- **Verdict:** TRUE.

### 2. Kani claims

- **Claim:** The new Kani harness group is gated behind the `kani-sequence-at-write` feature.
- **Audit:** Read `crates/vb_storage/src/lib.rs` (lines 34-94) and `crates/vb_storage/Cargo.toml:23-29`. The module is registered as `#[cfg(all(kani, feature = "kani-sequence-at-write"))] pub mod kani_sequence_at_write;`. The feature is declared as `kani-sequence-at-write = []`. The `cargo test` build does not set `cfg(kani)`, so the harness is not compiled.
- **Verdict:** TRUE.

- **Claim:** The Kani verification lane is blocked by a pre-existing `kani_helpers.rs` parse error in `vb_core`.
- **Audit:** Re-executed `cargo kani list --features kani-sequence-at-write --format json` from `crates/vb_storage/`. Output:
  ```
  error: this file contains an unclosed delimiter
    --> crates/vb_core/src/frame/parts/kani_helpers.rs:22:7
     |
   1 | mod frame_kani_harnesses {
     |                          - unclosed delimiter
  ...
  22 |     }
     |      ^
  error: could not compile `vb_core` (lib) due to 1 previous error
  ```
  Exit code 101. The file is 22 lines on this worktree (parent `1d6c017f`) and 16 lines on `main@origin` (no wrapping `mod` block). The blocker is pre-existing and not introduced by vb-r8oso.
- **Verdict:** TRUE.

### 3. Proptest claims

- **Claim:** `proptest_journal_error_codes` has 42 tests, all passing.
- **Audit:** Re-executed `cargo test -p vb_storage --test proptest_journal_error_codes -- --nocapture`. Raw log at `evidence/raw-cargo-test-proptest-journal-error-codes.log`. All 42 tests listed (`missing_required_proof_flag_returns_correct_code` ... `wrong_run_returns_correct_code`) and final `test result: ok. 42 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s`. Exit code 0.
- **Verdict:** TRUE.

- **Claim:** `cargo test -p vb_storage --lib next_sequence_at_write` passes 3 tests.
- **Audit:** Re-executed. Raw log at `evidence/raw-cargo-test-next-sequence-at-write.log`. Tests listed: `next_sequence_at_write_rejects_run_zero`, `next_sequence_at_write_returns_zero_for_fresh_run`, `next_sequence_at_write_returns_last_plus_one_after_writes`. `test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 1534 filtered out; finished in 0.01s`. Exit code 0.
- **Verdict:** TRUE.

- **Claim:** `cargo test -p vb_storage --lib append_strict_rejects` passes 4 tests.
- **Audit:** Re-executed. Raw log at `evidence/raw-cargo-test-append-strict-rejects.log`. Tests: `append_strict_rejects_sequence_at_zero_for_run_with_history`, `append_strict_rejects_duplicate_sequence`, `append_strict_rejects_duplicate_event`, `append_strict_rejects_out_of_order_sequence`. `test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 1533 filtered out; finished in 0.01s`. Exit code 0.
- **Verdict:** TRUE.

- **Claim:** `cargo test -p vb_storage --lib batch` passes 195 tests.
- **Audit:** Re-executed. Raw log at `evidence/raw-cargo-test-batch.log`. Final `test result: ok. 195 passed; 0 failed; 0 ignored; 0 measured; 1342 filtered out; finished in 2.17s`. Exit code 0.
- **Verdict:** TRUE.

### 4. Downstream caller audit (C-10)

- **Claim:** No production caller in `crates/vb_runtime` or `crates/vb_storage::recovery` supplies a non-contiguous `event.seq()` to the guarded append methods.
- **Audit:** Re-read `evidence/downstream-caller-audit.md` (126 lines, committed by `holzman-rust` State 11). The grep `rg "append_journaled|append_strict|append_unfsynced|append_event" crates/vb_runtime/src/` (excluding test code) yields exactly two hits at `crates/vb_runtime/src/journal/chunk_002.rs:34-36` (the `StorageRuntimeJournal::append_storage_event` body). The event's `seq` originates from `RuntimeShard::journal_sequence_for(run)`, a per-run in-memory map that starts at `EventSeq::ZERO` and is incremented by `advance_journal_sequence` only after a successful append. The recovery tree has no production caller. TB-NSAW-RESEARCH-001 is closed.
- **Verdict:** TRUE.

### 5. Diagnostic code (C-3.3)

- **Claim:** `JournalError::SequenceMismatch` has diagnostic code `0x4042` and symbolic code `JOURNAL_SEQUENCE_MISMATCH_AT_WRITE`.
- **Audit:** Read `crates/vb_storage/src/error/codes.rs:86` (constant declaration: `pub const SEQUENCE_MISMATCH_AT_WRITE_CODE: DiagnosticCode = DiagnosticCode::new(0x4042)`), `:121` (diagnostic arm: `Self::SequenceMismatch { .. } => Self::SEQUENCE_MISMATCH_AT_WRITE_CODE`), `:213` (symbolic arm: `Self::SequenceMismatch { .. } => "JOURNAL_SEQUENCE_MISMATCH_AT_WRITE"`). The 0x4042 slot is the next-free entry in the 0x404x block (sibling of `REPLAY_KEY_MISMATCH_CODE = 0x4040` and `REPLAY_ENVELOPE_SEQUENCE_MISMATCH_CODE = 0x4041`).
- **Verdict:** TRUE.

### 6. Exhaustive match guard (C-3)

- **Claim:** The new `SequenceMismatch` variant is added to the `journal_error_match_covers_all_variants` exhaustive match guard at `crates/vb_storage/src/tests.rs:7725`.
- **Audit:** Read `tests.rs:7742` (`JournalError::SequenceMismatch { .. } => "sequence_mismatch_at_write"`). The lib test `journal_error_match_covers_all_variants` is included in the 1,537 lib tests that pass (G-7).
- **Verdict:** TRUE.

### 7. No silent rewrite (C-5)

- **Claim:** The implementation does not silently rewrite `event.seq()` to match `expected`.
- **Audit:** Read the three guard sites: `crates/vb_storage/src/journal/internal.rs` (append_unfsynced), `crates/vb_storage/src/journal/append.rs` (append_strict), `crates/vb_storage/src/batch/append_event.rs` (append_event). All three sites construct the `SequenceMismatch` variant with `actual: event.seq()` verbatim. No `&mut event`, no `event.seq = expected`, no `set_seq` method on `JournalEvent`. The lib tests `append_strict_rejects_out_of_order_sequence` (tests.rs:1737) and `adversarial_read_events_with_sequence_gap_returns_exact_gap` (tests.rs:4612) assert `expected != actual` and verify field equality against the originating `event.seq()`.
- **Verdict:** TRUE.

### 8. Clippy zero warnings

- **Claim:** `cargo clippy --workspace --lib --bins --examples --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro` exits 0 with zero warnings.
- **Audit:** Re-executed. Raw log at `evidence/raw-cargo-clippy-strict.log`. Final `Finished 'dev' profile [unoptimized + debuginfo] target(s) in 5.88s`. Exit code 0. No warnings.
- **Verdict:** TRUE.

### 9. Behavioral waivers

- **Claim:** No behavior-affecting waivers exist.
- **Audit:** Read `formal-waivers.jsonl` (empty, 0 lines). Read `waiver-candidates.jsonl` (1 row: `W-vb-r8oso-NONE-001`, a declaration-only placeholder that explicitly does not waive any behavior). The Kani BLOCKED_TOOLING rows in the verification ledger are not waivers; they are pre-existing toolchain failures documented as blockers, not behavior laundered as a pass.
- **Verdict:** TRUE.

### 10. Black-hat review

- **Claim:** Black-hat review of 8 attack vectors passes with 0 defects.
- **Audit:** Read `black-hat-review.md` (State 13). All 8 attack vectors (silent rewrite, variant arm omission, diagnostic code conflict, C-6 test regression, Kani feature isolation, downstream caller breakage, no-panic contract, key-only lookup discipline) are documented with source refs, attack patterns, dispositions, and supplementary evidence. `defects.md` is empty.
- **Verdict:** TRUE.

### 11. Audit-able evidence chain

- **Claim:** Every claim in this report is backed by a raw log artifact under `.beads/vb-r8oso/evidence/`.
- **Audit:** 9 raw log files present:
  1. `raw-cargo-test-vb-storage-tests-all-features.log` (G-1)
  2. `raw-cargo-test-kani-feature-no-run.log` (G-2)
  3. `raw-cargo-test-vb-storage-kani-feature.log` (G-3)
  4. `raw-cargo-test-next-sequence-at-write.log` (G-4)
  5. `raw-cargo-test-append-strict-rejects.log` (G-5)
  6. `raw-cargo-test-batch.log` (G-6)
  7. `raw-cargo-test-vb-storage-lib.log` (G-7)
  8. `raw-cargo-test-proptest-journal-error-codes.log` (G-8)
  9. `raw-cargo-clippy-strict.log` (G-9)
  Plus `downstream-caller-audit.md` (C-10 audit) and `block-global-prerequisite.md` (pre-existing `BLOCK_GLOBAL`).
- **Verdict:** TRUE.

---

## Decision

**STATUS: APPROVED.**

Every claim in the assurance bundle, formal verification report, and black-hat review is independently verifiable against the raw evidence. No behavior is waived. No proof is laundered as a pass. The 2 Kani BLOCKED_TOOLING rows are pre-existing toolchain failures in `vb_core` (parent commit `1d6c017f`) that block all `cargo kani` invocations in this worktree, including ones unrelated to vb-r8oso. The Holzman Rust acceptance gate (`cargo test -p vb_storage --tests --all-features` = 1,676 passed; `--features kani-sequence-at-write` compiles and passes) is fully green.

**Cross-claim coherence:**
- Implementation (State 11) is internally consistent with the contract (C-1..C-12).
- Formal verification (State 12) is consistent with the implementation.
- Black-hat review (State 13) verifies 8 attack vectors and finds 0 defects.
- This truth-serum audit (State 14) verifies 11 categories of claims and finds 0 false claims.

**Action:** APPROVE for State 14 closure. Hand off to `landing-skill` with the documented Kani and BLOCK_GLOBAL blockers for pre-merge rebase/cherry-pick.

End of truth serum report.
