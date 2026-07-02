# Final Evidence Decision — vb-r8oso

**bead_id:** vb-r8oso
**title:** Storage: enforce next-sequence-at-write before durable append (P1 bug)
**state:** 14 (Final Evidence Decision)
**workdir:** /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-r8oso
**captured_at:** 2026-07-01T22:15:00Z
**owner_skill:** evidence-packaging
**parent_invocation_id:** truth-serum-cheap25-vb-r8oso-2026-07-01

STATUS: APPROVED

---

## 1. Decision

The bead `vb-r8oso` is **APPROVED** for State 14 closure. The Holzman Rust acceptance gate is fully green; all 7 proof obligations are dispositioned (5 PASS, 2 BLOCKED_TOOLING with documented pre-existing root cause); the black-hat review finds 0 defects; the truth-serum audit verifies 11 categories of claims with 0 false claims.

`defects.md` is empty. `formal-waivers.jsonl` is empty. No behavior is laundered as a pass.

---

## 2. Required Raw Evidence

All commands executed from `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-r8oso` on 2026-07-01.

| # | Command | Exit | Result |
|---|---|---|---|
| 1 | `cargo test -p vb_storage --tests --all-features` | 0 | **PASS** — 1,676 passed (16 suites) |
| 2 | `cargo test -p vb_storage --tests --features kani-sequence-at-write --no-run` | 0 | **PASS** — 16 test executables built |
| 3 | `cargo test -p vb_storage --tests --features kani-sequence-at-write` | 0 | **PASS** — 1,676 passed (16 suites) |
| 4 | `cargo test -p vb_storage --lib next_sequence_at_write` | 0 | **PASS** — 3 passed |
| 5 | `cargo test -p vb_storage --lib append_strict_rejects` | 0 | **PASS** — 4 passed |
| 6 | `cargo test -p vb_storage --lib batch` | 0 | **PASS** — 195 passed |
| 7 | `cargo test -p vb_storage --lib` | 0 | **PASS** — 1,537 passed |
| 8 | `cargo test -p vb_storage --test proptest_journal_error_codes -- --nocapture` | 0 | **PASS** — 42 passed |
| 9 | `cargo clippy --workspace --lib --bins --examples --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro` | 0 | **PASS** — zero warnings |

**Raw log artifacts** (under `.beads/vb-r8oso/evidence/`):
- `raw-cargo-test-vb-storage-tests-all-features.log` (1,756 lines)
- `raw-cargo-test-kani-feature-no-run.log`
- `raw-cargo-test-vb-storage-kani-feature.log` (1,756 lines)
- `raw-cargo-test-next-sequence-at-write.log`
- `raw-cargo-test-append-strict-rejects.log`
- `raw-cargo-test-batch.log`
- `raw-cargo-test-vb-storage-lib.log`
- `raw-cargo-test-proptest-journal-error-codes.log`
- `raw-cargo-clippy-strict.log`
- `downstream-caller-audit.md` (126 lines, C-10 closure)
- `block-global-prerequisite.md` (pre-existing `BLOCK_GLOBAL` documentation)

---

## 3. Proof Obligation Disposition

| POB | Verifier | Result | Reason |
|---|---|---|---|
| POB-001 | kani | BLOCKED_TOOLING | Pre-existing `kani_helpers.rs` parse error in `vb_core` blocks all Kani compilation. Not introduced by vb-r8oso. |
| POB-002 | kani | BLOCKED_TOOLING | Same blocker. |
| POB-003 | cargo-test | PASS | 1,676 passed; 49 proptest cases; 3 next_sequence_at_write tests; 195 batch tests. |
| POB-004 | cargo-test | PASS | 42 proptest_journal_error_codes tests; exhaustive-match guard at `tests.rs:7742`. |
| POB-005 | cargo-test | PASS | 4 append_strict_rejects tests; `actual: event.seq()` verbatim at all 3 guard sites. |
| POB-006 | cargo-test | PASS | 195 batch tests; 16 test executables compile with `kani-sequence-at-write` feature. |
| POB-007 | cargo-test | PASS | `downstream-caller-audit.md` (126 lines); TB-NSAW-RESEARCH-001 closed. |

---

## 4. Contract Coverage (C-1..C-12)

- **C-1 (Domain Model):** FS-1 (forbidden state) closed; FS-3 (silent rewrite) closed.
- **C-2 (New Method):** Signature, return values, lookup discipline, overflow, identifier hygiene, locking, public wrapper, no-panic — all closed.
- **C-3 (New Variant):** Declaration, pre-condition, diagnostic code 0x4042, symbolic code `JOURNAL_SEQUENCE_MISMATCH_AT_WRITE`, coexistence with `SequenceGap` — all closed.
- **C-4 (Append Path Contract):** 5 paths reject, guard precedence slot 3, doc-comments, batch atomicity — all closed.
- **C-5 (No Silent Rewrite):** Closed.
- **C-6 (Existing Tests Update):** `tests.rs:1737` updated, `tests.rs:4612` updated, `tests.rs:4585` reclassified — all closed.
- **C-7 (New Behavior Tests):** 7 new tests added per the seed list.
- **C-8 (Proof/Test Surface):** Kani harness group behind feature; proptest updates; fuzz updates tracked as follow-up.
- **C-9 (Kani Harness Isolation):** `#[cfg(all(kani, feature = "kani-sequence-at-write"))]` registered.
- **C-10 (Downstream Caller Audit):** `downstream-caller-audit.md` (126 lines) committed; only production caller is `StorageRuntimeJournal::append_storage_event`; per-run counter is contiguous.
- **C-11 (Acceptance Gate):** G-1, G-2, G-3, G-7, G-8 all PASS.
- **C-12 (Cross-Stage Hand-Off):** All artifacts present under `.beads/vb-r8oso/`.

---

## 5. Global Gate Note

The Kani toolchain is blocked by a pre-existing `kani_helpers.rs` parse error in `vb_core` (parent commit `1d6c017f`). The blocker is unrelated to vb-r8oso and affects all `cargo kani` invocations in this worktree. The `landing-skill` stage must:

1. Rebase onto `main` (which contains the fix for `kani_helpers.rs` and for the `BLOCK_GLOBAL` proptest `93d1d9026`), or
2. Cherry-pick the `main@origin` version of `crates/vb_core/src/frame/parts/kani_helpers.rs` into the landing commit, and cherry-pick `93d1d9026` for the `BLOCK_GLOBAL` proptest.

Neither blocker is introduced by vb-r8oso. The Holzman Rust acceptance gate is fully green.

---

## 6. Action

- **APPROVED** for State 14 closure.
- **Hand off** to `landing-skill` with the Kani and `BLOCK_GLOBAL` blockers documented for pre-merge rebase/cherry-pick.
- **Do not** merge `main` here.

End of final evidence decision.
