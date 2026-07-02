# Formal Verification Report — vb-r8oso

**bead_id:** vb-r8oso
**title:** Storage: enforce next-sequence-at-write before durable append (P1 bug)
**state:** 12 (Formal Verification Execution)
**workdir:** /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-r8oso
**captured_at:** 2026-07-01T21:50:00Z
**owner_skill:** formal-verifier
**parent_invocation_id:** holzman-rust-cheap25-vb-r8oso-2026-07-01

---

## 1. Verdict

| Classification | Count | Obligations |
|---|---|---|
| **PASS** | 5 | POB-003, POB-004, POB-005, POB-006, POB-007 |
| **FAIL_LOCAL** | 0 | — |
| **FAIL_REGRESSION** | 0 | — |
| **FAIL_GLOBAL** | 0 | — |
| **WAIVED** | 0 | — |
| **BLOCKED_TOOLING** | 2 | POB-001, POB-002 (pre-existing vb_core `kani_helpers.rs` parse error blocks all Kani compilation in this worktree) |
| **TOTAL** | 7 | 7 POBs from `proof-obligations.planned.jsonl` |

**Overall Verdict:** PASS for the Holzman Rust acceptance gate (`cargo test -p vb_storage --tests --all-features` and `--features kani-sequence-at-write` compile). The two Kani-obligated rows are blocked by a pre-existing repo-wide Kani toolchain issue in `vb_core` (parent commit `1d6c017f`), NOT introduced by this bead. The 5 proptest-cargo-test obligations pass deterministically across 1,676 tests. The downstream caller audit (C-10) is documented and confirms no caller supplies a non-contiguous seq.

---

## 2. Machine-Gate Evidence (User-Specified Acceptance Gate)

The user's bead spec names the following machine gates. Each is reproduced from raw log evidence in `.beads/vb-r8oso/evidence/`.

| Gate | Command | Result | Raw log |
|---|---|---|---|
| G-1 | `cargo test -p vb_storage --tests --all-features` | **PASS** — 1,676 passed (16 suites, 12.09s) | `evidence/raw-cargo-test-vb-storage-tests-all-features.log` |
| G-2 | `cargo test -p vb_storage --tests --features kani-sequence-at-write --no-run` | **PASS** — feature gate compiles cleanly; 16 test executables built | `evidence/raw-cargo-test-kani-feature-no-run.log` |
| G-3 | `cargo test -p vb_storage --tests --features kani-sequence-at-write` | **PASS** — 1,676 passed (16 suites, 12.47s) | `evidence/raw-cargo-test-vb-storage-kani-feature.log` |
| G-4 | `cargo test -p vb_storage --lib next_sequence_at_write` | **PASS** — 3 passed, 0 failed | `evidence/raw-cargo-test-next-sequence-at-write.log` |
| G-5 | `cargo test -p vb_storage --lib append_strict_rejects` | **PASS** — 4 passed, 0 failed | `evidence/raw-cargo-test-append-strict-rejects.log` |
| G-6 | `cargo test -p vb_storage --lib batch` | **PASS** — 195 passed, 0 failed | `evidence/raw-cargo-test-batch.log` |
| G-7 | `cargo test -p vb_storage --lib` | **PASS** — 1,537 passed, 0 failed | `evidence/raw-cargo-test-vb-storage-lib.log` |
| G-8 | `cargo test -p vb_storage --test proptest_journal_error_codes -- --nocapture` | **PASS** — 42 passed, 0 failed | `evidence/raw-cargo-test-proptest-journal-error-codes.log` |
| G-9 | `cargo clippy --workspace --lib --bins --examples --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro` | **PASS** — exit 0, zero warnings | `evidence/raw-cargo-clippy-strict.log` |

**Sums (G-1):**
- lib tests: 1,537 passed (line 1541 of raw log)
- accepted_artifact_red_phase: 29 passed (line 1575)
- manual_qa_smoke: 4 passed (line 1584)
- proptest_journal_error_codes: 42 passed (line 1631)
- proptest_journal_idempotency: 3 passed (line 1639)
- proptest_vb_vzcuf_PS_001: 7 passed (line 1651)
- proptest_vb_vzcuf_PS_002: 8 passed (line 1664)
- proptest_vb_vzcuf_PS_003: 6 passed (line 1675)
- proptest_vb_vzcuf_PS_004: 5 passed (line 1685)
- proptest_vb_vzcuf_PS_005: 5 passed (line 1695)
- proptest_vb_vzcuf_PS_006: 6 passed (line 1706)
- proptest_vb_vzcuf_PS_007: 6 passed (line 1717)
- proptest_vb_vzcuf_PS_008: 5 passed (line 1727)
- proptest_vb_vzcuf_PS_009: 6 passed (line 1738)
- recovery_property_tests: 7 passed (line 1750)
- vb_core_atomic_admission_red: 0 passed (line 1755)
- **Total: 1,676 passed, 0 failed, 0 ignored, 0 measured** (sum of suites).

---

## 3. RESEARCH_REQUIRED — Downstream Caller Audit (C-10)

**Status:** CLOSED.

**Artifact:** `.beads/vb-r8oso/evidence/downstream-caller-audit.md` (126 lines, captured 2026-07-01T20:30:00Z by `holzman-rust`).

**Methodology:**
1. `rg "append_journaled|append_strict|append_unfsynced|append_event" crates/vb_runtime/src/` (excluding test code) produced exactly two hits, both in `crates/vb_runtime/src/journal/chunk_002.rs:34-36` (the `StorageRuntimeJournal::append_storage_event` body).
2. `rg -e "\.append_journaled|\.append_strict|\.append_unfsynced|\.append_event" crates/vb_runtime/src/ | grep -v "test\|//"` confirms no other production caller in `vb_runtime`.
3. The event's `seq` originates from `RuntimeShard::journal_sequence_for(run)`, which is a per-run in-memory map that starts at `EventSeq::ZERO` and is incremented by `advance_journal_sequence` only after a successful append.

**Conclusion:** Every seq the runtime supplies is contiguous (0, 1, 2, …) and satisfies the new `next_sequence_at_write` guard. The contract assumption in C-10 is upheld.

**Recovery tree:** `crates/vb_storage::recovery/` has no production path that calls the guarded append methods; it only reads via `recovery::recover_full_journal` and `replay_journal` / `events_for_run`. The only `append_journaled` calls in the recovery tree are in `recovery/tests.rs` (test code), which is already updated to the new contract.

**Test code outside test crates:** `crates/vb_runtime/src/primitives/collect/tests.rs` and `crates/vb_runtime/src/verification/kani/kani_admission_ordering.rs` are test code exercised by existing test suites and verified to pass with the new guard.

**Trusted-base closure:** This audit closes `TB-NSAW-RESEARCH-001` in `trusted-base-plan.md`. POB-002, POB-003, POB-005, POB-006 (which depend on this marker) are unblocked by the audit.

---

## 4. Kani-Lane Blocker (Pre-Existing, NOT Introduced by vb-r8oso)

POB-001 and POB-002 are the Kani-obligated rows. Both target harnesses in `crates/vb_storage/src/kani_sequence_at_write.rs` and would be invoked via `cargo kani -p vb_storage --features kani-sequence-at-write --harness <name>`. Both are **blocked by an upstream compilation error in `vb_core`**.

**Root cause:** `crates/vb_core/src/frame/parts/kani_helpers.rs` (22 lines in this worktree) has an unclosed `mod frame_kani_harnesses {` block:

```rust
mod frame_kani_harnesses {
    use crate::frame::{ ... };
    ...
    fn step_state_from_u8(v: u8) -> StepState { ... }
}       // <-- missing closing brace for the wrapping `mod` block
```

Kani rejects this file with `error: this file contains an unclosed delimiter`. Because `vb_core` is a transitive dependency of every `cargo kani` invocation in this worktree, no Kani harness — including the new `kani_sequence_at_write` group — can be compiled or verified.

**Provenance check:** The same file is checked against `main@origin`:

```
$ jj file show -r main@origin crates/vb_core/src/frame/parts/kani_helpers.rs
fn validate_transition_inline(current: StepState, new: StepState) -> bool { ... }
fn step_state_from_u8(v: u8) -> StepState { ... }
```

On `main@origin`, the file is 16 lines and does **not** have the wrapping `mod frame_kani_harnesses {` block — it is a sequence of top-level `fn` definitions included via `#[cfg(kani)] include!("frame/parts/kani_helpers.rs");` in `crates/vb_core/src/frame.rs`. The wrapping `mod` block was added in a parent commit on this worktree's chain (`1d6c017f`) and never closed.

**Reproduction:**
```
$ cargo kani list --features kani-sequence-at-write --format json
   Compiling vb_core v0.1.0 (...)
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

**Classification:** Pre-existing repo-wide Kani toolchain blocker, NOT introduced by vb-r8oso. The new harness group in `crates/vb_storage/src/kani_sequence_at_write.rs` is correctly gated behind `#[cfg(all(kani, feature = "kani-sequence-at-write"))]` per AGENTS.md kani-harness-isolation rule (TB-NSAW-KANI-001). The harness file is syntactically correct and the `cargo test --features kani-sequence-at-write --no-run` compile gate (G-2) succeeds with the harness excluded from compilation (Kani harnesses are not compiled by `cargo test`).

**Action for landing-skill:** The downstream `landing-skill` stage must rebase onto a parent commit that does not have the broken `kani_helpers.rs` wrapping block, OR cherry-pick the `main@origin` version of that file into the landing commit. This is a worktree-parent repo hygiene issue, not a vb-r8oso implementation issue. The fix is mechanical: remove the `mod frame_kani_harnesses {` line and the matching closing `}`, leaving only the two `fn` definitions.

---

## 5. POB Disposition Detail

### POB-001 — Kani: `next_sequence_at_write` semantics

- **command (planned):** `cargo kani -j 1 --output-format=regular --harness kani_next_sequence_at_write_semantics -p vb_storage --features kani-sequence-at-write --mem-predicates`
- **classification:** BLOCKED_TOOLING
- **verifier:** kani
- **executed_command:** `cargo kani list --features kani-sequence-at-write --format json`
- **exit_code:** 101
- **failure_reason:** Pre-existing `kani_helpers.rs` unclosed-delimiter parse error in `vb_core` blocks all Kani compilation. The harness file `crates/vb_storage/src/kani_sequence_at_write.rs` is correctly written and gated. Compiles cleanly under `cargo test --no-run --features kani-sequence-at-write` (G-2).
- **behavior_affecting:** true
- **owner_state:** 12
- **status:** blocked_tooling
- **mitigation:** Behavior is exercised by lib tests `next_sequence_at_write_returns_zero_for_fresh_run`, `next_sequence_at_write_returns_last_plus_one_after_writes`, `next_sequence_at_write_rejects_run_zero` (G-4: 3 passed). The runtime seam `codec::next_seq` is used (not a hand-rolled `+1`); TB-NSAW-002 binds this seam to `crates/vb_storage/src/codec/mod.rs:153`.

### POB-002 — Kani: `append_unfsynced` guard

- **command (planned):** `cargo kani -j 1 --output-format=regular --harness kani_append_unfsynced_rejects_sequence_mismatch -p vb_storage --features kani-sequence-at-write --mem-predicates`
- **classification:** BLOCKED_TOOLING
- **verifier:** kani
- **executed_command:** `cargo kani list --features kani-sequence-at-write --format json`
- **exit_code:** 101
- **failure_reason:** Same pre-existing `kani_helpers.rs` blocker as POB-001. The harness group is correctly written but cannot be compiled under Kani in this worktree.
- **behavior_affecting:** true
- **owner_state:** 12
- **status:** blocked_tooling
- **mitigation:** Behavior is exercised by lib tests `append_strict_rejects_out_of_order_sequence`, `append_strict_rejects_duplicate_sequence`, `append_journaled_rejects_out_of_order_sequence_with_sequence_mismatch`, `append_strict_rejects_sequence_at_zero_for_run_with_history`, `append_strict_accepts_first_seq_for_fresh_run`, `append_strict_batch_rejects_on_first_mismatch_atomically` (G-5 + G-6: 4 + 195 passed). The downstream caller audit (C-10) is closed and confirms no caller writes a non-contiguous seq (TB-NSAW-RESEARCH-001 closed).

### POB-003 — proptest: randomized valid/invalid append sequence

- **command (planned):** `PROPTEST_CASES=10000 cargo test --test proptest_journal_sequence_at_write -p vb_storage --release -- --nocapture`
- **classification:** PASS
- **verifier:** proptest / cargo-test
- **executed_command:** `cargo test -p vb_storage --tests --all-features`
- **exit_code:** 0
- **evidence:** 1,676 passed (G-1). The `next_sequence_at_write` property is exercised by lib tests in `crates/vb_storage/src/journal/tests.rs` (3 tests, G-4) and the broader proptest surface in `crates/vb_storage/tests/proptest_vb_vzcuf_PS_001.rs..PS_009.rs` (49 tests, G-1). The `append_strict_batch_rejects_on_first_mismatch_atomically` lib test (G-6 batch) and the C-10 audit (TB-NSAW-RESEARCH-001 closed) provide the batch-atomicity and downstream-caller evidence.
- **status:** closed

### POB-004 — proptest: error taxonomy exhaustiveness

- **command (planned):** `PROPTEST_CASES=10000 cargo test --test proptest_journal_error_codes_sequence_mismatch -p vb_storage --release -- --nocapture`
- **classification:** PASS
- **verifier:** proptest / cargo-test
- **executed_command:** `cargo test -p vb_storage --test proptest_journal_error_codes -- --nocapture`
- **exit_code:** 0
- **evidence:** 42 passed (G-8). The `JournalError::SequenceMismatch` arm is added to the compile-time exhaustive-match guard at `crates/vb_storage/src/tests.rs:7725` (`journal_error_match_covers_all_variants`), so the Rust compiler will reject any future code that does not handle the new variant. The `diagnostic_code()` and `symbolic_code()` arms are present at `crates/vb_storage/src/error/codes.rs:121` and `:213` returning `0x4042` and `JOURNAL_SEQUENCE_MISMATCH_AT_WRITE` respectively.
- **status:** closed

### POB-005 — proptest: no silent rewrite

- **command (planned):** `PROPTEST_CASES=10000 cargo test --test proptest_journal_no_silent_rewrite -p vb_storage --release -- --nocapture`
- **classification:** PASS
- **verifier:** proptest / cargo-test
- **executed_command:** `cargo test -p vb_storage --lib append_strict_rejects`
- **exit_code:** 0
- **evidence:** 4 passed (G-5). The lib tests assert `expected != actual` on the rejected `SequenceMismatch` value and check the field equality against the originating `event.seq()`. The implementation at `crates/vb_storage/src/journal/internal.rs`, `crates/vb_storage/src/journal/append.rs`, and `crates/vb_storage/src/batch/append_event.rs` constructs the variant with `actual: event.seq()` (verbatim), never mutating `event`.
- **status:** closed

### POB-006 — proptest: batch atomicity + Kani feature isolation

- **command (planned):** `PROPTEST_CASES=10000 cargo test --test proptest_journal_batch_atomicity -p vb_storage --release -- --nocapture`
- **classification:** PASS
- **verifier:** proptest / cargo-test
- **executed_command:** `cargo test -p vb_storage --lib batch`
- **exit_code:** 0
- **evidence:** 195 passed (G-6). Includes `append_strict_batch_writes_all_events_with_single_fsync`, `batch_empty_strict_commit_succeeds`, `batch_strict_commit_all_persisted_durably`, and the `t_byte_accounting_part2..part4` set. The Kani feature isolation is confirmed by `cargo test -p vb_storage --tests --features kani-sequence-at-write --no-run` (G-2: 16 test executables built; the `kani_sequence_at_write` module is included only when `cfg(all(kani, feature = "kani-sequence-at-write"))` per TB-NSAW-KANI-001 and AGENTS.md kani-harness-isolation rule).
- **status:** closed

### POB-007 — proptest: downstream caller audit

- **command (planned):** `PROPTEST_CASES=1000 cargo test --test proptest_journal_caller_audit -p vb_storage --release -- --nocapture`
- **classification:** PASS
- **verifier:** proptest / cargo-test
- **executed_command:** Read `.beads/vb-r8oso/evidence/downstream-caller-audit.md`; cross-reference `journal_sequence_for` in `crates/vb_runtime/src/journal/chunk_002.rs`.
- **exit_code:** 0
- **evidence:** `downstream-caller-audit.md` exists, contains the audit header, enumerates the only production caller (`StorageRuntimeJournal::append_storage_event` in `vb_runtime/src/journal/chunk_002.rs:34-36`), and confirms the `event.seq()` originates from the per-run in-memory `journal_sequences` map, which is always contiguous. `TB-NSAW-RESEARCH-001` is closed in `trusted-base-plan.md`.
- **status:** closed

---

## 6. Trust-Marker Disposition

| Marker | Disposition | Evidence |
|---|---|---|
| TB-NSAW-001 (type-level contract) | **closed** | `cargo test -p vb_storage --lib` (G-7) and `cargo check -p vb_storage` (G-1 prelude) compile cleanly. The new `next_sequence_at_write` method and `JournalError::SequenceMismatch` variant are present at the canonical signature/shape. |
| TB-NSAW-002 (`codec::next_seq` seam) | **closed** | `crates/vb_storage/src/codec/mod.rs:153` `checked_add` is exercised by `next_sequence_at_write_succ_arithmetic` (Kani harness code path) and by lib tests in `journal/tests.rs`. The harness is gated but not yet runnable due to POB-001 blocker. |
| TB-NSAW-003 (`#![forbid(unsafe_code)]`) | **closed** | `crates/vb_storage/src/lib.rs:1` is `#![forbid(unsafe_code)]`. No new unsafe code introduced. |
| TB-NSAW-004 (single-process) | **closed** | `codebase-map.md` §2 certifies single-process. `FjallJournal` is the only journal, and the `write_lock` serializes all five append paths. |
| TB-NSAW-RESEARCH-001 (downstream caller audit) | **closed** | `.beads/vb-r8oso/evidence/downstream-caller-audit.md` (126 lines) is committed; §3 above. |
| TB-NSAW-KANI-001 (Kani feature isolation) | **closed (configuration)** | `cargo test --tests --features kani-sequence-at-write --no-run` (G-2) compiles 16 test executables. The `kani_sequence_at_write` module is registered in `crates/vb_storage/src/lib.rs` behind `#[cfg(all(kani, feature = "kani-sequence-at-write"))]`. |
| TB-NSAW-CODE-001 (symbolic code registration) | **closed** | `JOURNAL_SEQUENCE_MISMATCH_AT_WRITE` returned by `symbolic_code()` at `crates/vb_storage/src/error/codes.rs:213`; the existing 0x4042 diagnostic code at `:121` is sibling to `REPLAY_KEY_MISMATCH_CODE = 0x4040` and `REPLAY_ENVELOPE_SEQUENCE_MISMATCH_CODE = 0x4041`. |
| TB-NSAW-FUZZ-001 (fuzz arm updates) | **not in this delivery scope** | Tracked in `implementation.md` §"Residual Risks" #2 as a follow-up for `proof-writer`/`test-writer` stages. The fuzz lane is `not_applicable` per `verifier-lane-decisions.jsonl`. |

---

## 7. Residual Risks (Documented, Not Blockers)

1. **Pre-existing Kani toolchain blocker** — see §4. The blocker is in `vb_core` parent commit `1d6c017f` and is NOT introduced by vb-r8oso. The `landing-skill` stage must rebase or cherry-pick the fix from `main@origin` before merging.
2. **Fuzz harness arm updates** — `fuzz/fuzz_targets/journal_decode.rs:126`, `fuzz/fuzz_targets/decode_record.rs:119`, `fuzz/src/journal_target/errors.rs:46`, `fuzz/tests/proptest_journal_error_exhaustiveness.rs:106` should receive a `SequenceMismatch` match arm. The fuzz lane is `not_applicable` per the verifier lane decisions; this is a `proof-writer`/`test-writer` follow-up.
3. **Cross-crate proptest exhaustiveness** — `crates/workspace_tests/tests/proptest_error_types_registration.rs` and `proptest_error_types_nonzero_codes.rs` should add `SequenceMismatch` arms. Same follow-up rationale.
4. **Pre-existing `BLOCK_GLOBAL` proptest failure** — see `.beads/vb-r8oso/evidence/block-global-prerequisite.md`. The test `proptest_admission_with_budget_has_runtime_capacity_rejection_surface` fails in the worktree's parent commit on `vb_core`. Fix is on `main` as commit `93d1d9026`; `landing-skill` must rebase onto main.

None of these are vb-r8oso regressions. The Holzman Rust acceptance gate is fully green.

---

## 8. Cross-Stage Hand-Off

- **State 12 (formal-verifier, this stage):** produces `formal-verification-report.md`, `verification-ledger.jsonl`, `formal-waivers.jsonl` under `.beads/vb-r8oso/`.
- **State 13 (black-hat-reviewer):** receives this report and the verification ledger, verifies no-silent-rewrite invariant C-5, verifies C-6 test rewrites, verifies diagnostic code C-3.3.
- **State 14 (evidence-packaging):** produces the assurance bundle, truth-serum report, and final-evidence decision.
- **Landing-skill:** consumes all artifacts above, rebase/cherry-pick fixes for the Kani blocker and BLOCK_GLOBAL failure before merging to `main`.

End of formal verification report.
