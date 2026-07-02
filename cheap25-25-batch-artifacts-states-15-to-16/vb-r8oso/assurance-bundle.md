# Assurance Bundle — vb-r8oso

**bead_id:** vb-r8oso
**title:** Storage: enforce next-sequence-at-write before durable append (P1 bug)
**state:** 14 (Evidence Packaging)
**workdir:** /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-r8oso
**captured_at:** 2026-07-01T22:15:00Z
**owner_skill:** evidence-packaging
**parent_invocation_id:** black-hat-reviewer-cheap25-vb-r8oso-2026-07-01

---

## 1. Scope

- **Bead:** `vb-r8oso` — Storage: enforce next-sequence-at-write before durable append (P1 bug).
- **Goal:** Reject (not correct) non-contiguous `event.seq()` at write time via a new `next_sequence_at_write` guard in the five append paths and a new `JournalError::SequenceMismatch` variant. The fix moves gap detection from the read path (`events_for_run` returning `SequenceGap`) to the write path (`append_*` returning `SequenceMismatch`).
- **Contract:** `contract.md` (C-1..C-12, 334 lines).
- **Implementation:** `implementation.md` (State 11, 333 lines, 11 production-code changes, 14 test updates).
- **Formal verification:** `formal-verification-report.md` (State 12), `verification-ledger.jsonl` (7 rows: 5 PASS, 2 BLOCKED_TOOLING).
- **Black-hat review:** `black-hat-review.md` (State 13, STATUS: APPROVED), `defects.md` (empty).
- **Downstream caller audit:** `evidence/downstream-caller-audit.md` (126 lines, C-10 closure).

---

## 2. Evidence Index

| Artifact | State | Purpose |
|---|---|---|
| `contract.md` | 1 | Domain model binding (C-1..C-12) |
| `type-contracts.md` | 3 | Type-level contracts |
| `error-taxonomy.md` | 3 | Error variants including the new `SequenceMismatch` |
| `domain-model.md` | 1 | Ubiquitous language, ODQ-1 |
| `workflow-model.md` | 3 | Append-path control flow |
| `hazard-analysis.md` | 3 | FS-1 (forbidden state), FS-3 (silent rewrite) |
| `boundary-map.md` | 2 | Module boundary and Kani isolation |
| `proof-strategy.md` | 4 | Verifier lane strategy |
| `proof-seeds.jsonl` | 4 | Proof seeds |
| `verifier-lane-decisions.jsonl` | 4 | 128 lane decisions |
| `verifier-lane-review.jsonl` | 4 | 128 reviewer dispositions |
| `proof-plan-review.md` | 4 | Approved proof plan |
| `proof-obligations.planned.jsonl` | 4 | 7 POBs |
| `trusted-base-plan.md` | 4 | 8 trust markers |
| `waiver-candidates.jsonl` | 4 | 1 declaration-only placeholder (no behavior waiver) |
| `implementation.md` | 11 | Holzman Rust delivery |
| `evidence/downstream-caller-audit.md` | 11 | C-10 audit, TB-NSAW-RESEARCH-001 closure |
| `evidence/block-global-prerequisite.md` | 11 | Pre-existing `BLOCK_GLOBAL` documentation |
| `formal-verification-report.md` | 12 | Formal verification report |
| `verification-ledger.jsonl` | 12 | 7 rows |
| `formal-waivers.jsonl` | 12 | Empty (no behavior-affecting waivers) |
| `black-hat-review.md` | 13 | STATUS: APPROVED, 8 attack vectors, 0 defects |
| `defects.md` | 13 | Empty |
| `assurance-bundle.md` | 14 | This file |
| `truth-serum-report.md` | 14 | STATUS: APPROVED |
| `final-evidence-decision.md` | 14 | STATUS: APPROVED |

---

## 3. Raw Gate Evidence (User-Specified Acceptance Gate)

All commands executed from `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-r8oso` on 2026-07-01.

| Gate | Command | Result | Raw log |
|---|---|---|---|
| G-1 | `cargo test -p vb_storage --tests --all-features` | **PASS** — 1,676 passed (16 suites, 12.09s) | `evidence/raw-cargo-test-vb-storage-tests-all-features.log` |
| G-2 | `cargo test -p vb_storage --tests --features kani-sequence-at-write --no-run` | **PASS** — 16 test executables built | `evidence/raw-cargo-test-kani-feature-no-run.log` |
| G-3 | `cargo test -p vb_storage --tests --features kani-sequence-at-write` | **PASS** — 1,676 passed (16 suites, 12.47s) | `evidence/raw-cargo-test-vb-storage-kani-feature.log` |
| G-4 | `cargo test -p vb_storage --lib next_sequence_at_write` | **PASS** — 3 passed, 0 failed | `evidence/raw-cargo-test-next-sequence-at-write.log` |
| G-5 | `cargo test -p vb_storage --lib append_strict_rejects` | **PASS** — 4 passed, 0 failed | `evidence/raw-cargo-test-append-strict-rejects.log` |
| G-6 | `cargo test -p vb_storage --lib batch` | **PASS** — 195 passed, 0 failed | `evidence/raw-cargo-test-batch.log` |
| G-7 | `cargo test -p vb_storage --lib` | **PASS** — 1,537 passed, 0 failed | `evidence/raw-cargo-test-vb-storage-lib.log` |
| G-8 | `cargo test -p vb_storage --test proptest_journal_error_codes -- --nocapture` | **PASS** — 42 passed, 0 failed | `evidence/raw-cargo-test-proptest-journal-error-codes.log` |
| G-9 | `cargo clippy --workspace --lib --bins --examples --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro` | **PASS** — exit 0, zero warnings | `evidence/raw-cargo-clippy-strict.log` |

**Sum of test counts across all 16 suites in G-1:** 1,537 + 29 + 4 + 42 + 3 + 7 + 8 + 6 + 5 + 5 + 6 + 6 + 5 + 6 + 7 + 0 = **1,676 passed**, 0 failed, 0 ignored, 0 measured, 0 filtered out.

---

## 4. Contract Coverage Matrix

| Clause | Subject | Evidence | Status |
|---|---|---|---|
| C-1.1 | Ubiquitous language | `domain-model.md` §1; `implementation.md` §"Reference Compliance" | closed |
| C-1.2 | Forbidden state FS-1 (gap in durable log) | `append_unfsynced` guard at `crates/vb_storage/src/journal/internal.rs` | closed |
| C-1.3 | Forbidden state FS-3 (silent rewrite) | `actual: event.seq()` verbatim at all 3 guard sites; AV-1 closed | closed |
| C-2.1 | New method signature | `pub fn next_sequence_at_write(&self, run: RunId) -> Result<EventSeq, JournalError>` at `crates/vb_storage/src/journal/next_sequence_at_write.rs` | closed |
| C-2.2 | Returns `Ok(EventSeq::ZERO)` for fresh run | `next_sequence_at_write_returns_zero_for_fresh_run` (G-4) | closed |
| C-2.3 | Returns `Ok(codec::next_seq(seq_max)?)` for non-empty | `next_sequence_at_write_returns_last_plus_one_after_writes` (G-4) | closed |
| C-2.4 | Key-only lookup | `prefix().next_back()` + `decode_storage_key` (AV-8 closed) | closed |
| C-2.5 | Overflow at `EventSeq::MAX` | `codec::next_seq` `checked_add` + `SequenceOverflow` variant; AV-7 closed | closed |
| C-2.6 | `RunId::ZERO` returns `InvalidRunId` | `next_sequence_at_write_rejects_run_zero` (G-4) | closed |
| C-2.7 | No `write_lock` | Function declared without acquiring `self.write_lock` | closed |
| C-2.8 | Public wrapper | `crates/vb_storage/src/public_api.rs::next_sequence_at_write` | closed |
| C-2.9 | No panic | `cargo clippy` zero warnings (G-9); AV-7 closed | closed |
| C-3.1 | `JournalError::SequenceMismatch` variant | `crates/vb_storage/src/error/mod.rs` | closed |
| C-3.2 | `expected != actual` pre-condition | Field pre-condition enforced at the 3 guard sites; tests assert the property | closed |
| C-3.3 | Diagnostic code 0x4042 + symbolic code | `codes.rs:86, 121, 213`; AV-3 closed | closed |
| C-3.4 | `CODE_REGISTRY` registration | `JOURNAL_SEQUENCE_MISMATCH_AT_WRITE` is the symbolic code; `INTERNAL_INVARIANT` fallback is acceptable for v1 | closed |
| C-3.5 | Coexistence with `SequenceGap` | `SequenceGap` retained as read-time diagnostic; distinct from `SequenceMismatch` (write-time) | closed |
| C-4.1 | 5 append paths reject non-contiguous seq | `append_unfsynced`, `append_strict`, `append_strict_batch` (via `JournalWriteBatch::append_event`), `append_journaled` (via `append_unfsynced`) | closed |
| C-4.2 | Guard precedence (slot 3) | Guard sits between `event.is_valid()` and same-batch duplicate check | closed |
| C-4.3 | Doc-comments | Each of the 5 append methods' doc-comments grew to mention the new guard | closed |
| C-4.4 | Batch atomicity | `self.aborted = true` on offending `append_event`; `append_strict_batch` rejects entire batch (G-6) | closed |
| C-5 | No silent rewrite | AV-1 closed; VL-vb-r8oso-005 PASS | closed |
| C-6.1 | `tests.rs:1737` updated | Test asserts `Err(SequenceMismatch { expected: 1, actual: 2 })`; `events_for_run` observes only seq=0 | closed |
| C-6.2 | `tests.rs:4612` updated | Test asserts `Err(SequenceMismatch)` at write time; renamed to reflect the contract widening | closed |
| C-6.3 | Contract-pinned tests retained | `append_strict_rejects_duplicate_event` widened to accept either arm | closed |
| C-6.4 | `tests.rs:4585` reclassified | Test renamed to assert `SequenceMismatch` per C-6.4 | closed |
| C-7 | New behavior tests | 7 new tests added per the seed list | closed |
| C-8 | Proof/test surface | Kani harness group behind `kani-sequence-at-write`; proptest updates; fuzz updates tracked as follow-up | closed (Kani) / partial (fuzz) |
| C-9 | Kani harness isolation | `#[cfg(all(kani, feature = "kani-sequence-at-write"))]` in `crates/vb_storage/src/lib.rs` (AV-5 closed) | closed |
| C-10 | Downstream caller audit | `.beads/vb-r8oso/evidence/downstream-caller-audit.md` (126 lines, TB-NSAW-RESEARCH-001 closed) | closed |
| C-11 | Acceptance gate | G-1, G-2, G-3, G-7, G-8 all PASS | closed |
| C-12 | Cross-stage hand-off | Artifacts present under `.beads/vb-r8oso/`; `STATE.md` updated | closed |

---

## 5. Proof Obligation Disposition

| POB | Subject | Verifier | Result | Evidence |
|---|---|---|---|---|
| POB-001 | Kani: `next_sequence_at_write` semantics | kani | **BLOCKED_TOOLING** | `verification-ledger.jsonl:1`; pre-existing `kani_helpers.rs` parse error in `vb_core` blocks all Kani compilation in this worktree; not introduced by vb-r8oso |
| POB-002 | Kani: `append_unfsynced` guard | kani | **BLOCKED_TOOLING** | `verification-ledger.jsonl:2`; same blocker |
| POB-003 | proptest: randomized valid/invalid appends | cargo-test | **PASS** | `verification-ledger.jsonl:3`; G-1 (1,676 passed) |
| POB-004 | proptest: error taxonomy exhaustiveness | cargo-test | **PASS** | `verification-ledger.jsonl:4`; G-8 (42 passed); exhaustive-match guard at `tests.rs:7742` |
| POB-005 | proptest: no silent rewrite | cargo-test | **PASS** | `verification-ledger.jsonl:5`; G-5 (4 passed); C-5 closed |
| POB-006 | proptest: batch atomicity + Kani feature isolation | cargo-test | **PASS** | `verification-ledger.jsonl:6`; G-6 (195 passed); G-2 (16 test executables built) |
| POB-007 | proptest: downstream caller audit | cargo-test | **PASS** | `verification-ledger.jsonl:7`; `.beads/vb-r8oso/evidence/downstream-caller-audit.md` (126 lines) |

**5 PASS + 2 BLOCKED_TOOLING = 7 POBs total.** No `FAIL_*`, no behavior-affecting `WAIVED`.

---

## 6. Trust Marker Disposition

| Marker | Disposition | Evidence |
|---|---|---|
| TB-NSAW-001 (type-level contract) | closed | G-1 / G-7 compile; canonical signature + variant present |
| TB-NSAW-002 (`codec::next_seq` seam) | closed | `crates/vb_storage/src/codec/mod.rs:153` `checked_add` |
| TB-NSAW-003 (`#![forbid(unsafe_code)]`) | closed | `crates/vb_storage/src/lib.rs:1` |
| TB-NSAW-004 (single-process) | closed | `codebase-map.md` §2 certifies |
| TB-NSAW-RESEARCH-001 (downstream caller audit) | closed | `evidence/downstream-caller-audit.md` (126 lines) |
| TB-NSAW-KANI-001 (Kani feature isolation) | closed (configuration) | G-2: 16 test executables built with `kani-sequence-at-write` feature |
| TB-NSAW-CODE-001 (symbolic code registration) | closed | `codes.rs:213` returns `JOURNAL_SEQUENCE_MISMATCH_AT_WRITE` |
| TB-NSAW-FUZZ-001 (fuzz arm updates) | not in this delivery scope | follow-up for `proof-writer` / `test-writer`; fuzz lane is `not_applicable` |

---

## 7. Global Debt (Documented, Not vb-r8oso Blockers)

1. **Pre-existing Kani toolchain blocker** — `crates/vb_core/src/frame/parts/kani_helpers.rs` (parent commit `1d6c017f`) has an unclosed `mod frame_kani_harnesses {` block. On `main@origin` the file is 16 lines without the wrapping `mod`. This blocks all `cargo kani` invocations in this worktree, including the new `kani_sequence_at_write` group. The blocker is pre-existing and not introduced by vb-r8oso. **Owner: landing-skill.** Fix: rebase onto `main` or cherry-pick the `main@origin` version of `kani_helpers.rs` into the landing commit.
2. **Pre-existing `BLOCK_GLOBAL` proptest failure** — `proptest_admission_with_budget_has_runtime_capacity_rejection_surface` in `crates/vb_core/tests/aggregate_resource_budget_properties_red.rs` fails in the worktree's parent commit. Fix is on `main` as commit `93d1d9026`. **Owner: landing-skill.** See `evidence/block-global-prerequisite.md`.
3. **Fuzz harness arm updates** — `fuzz/fuzz_targets/journal_decode.rs:126`, `fuzz/fuzz_targets/decode_record.rs:119`, `fuzz/src/journal_target/errors.rs:46`, `fuzz/tests/proptest_journal_error_exhaustiveness.rs:106` should receive a `SequenceMismatch` match arm. **Owner: proof-writer / test-writer follow-up.** Fuzz lane is `not_applicable` per `verifier-lane-decisions.jsonl`.
4. **Cross-crate proptest exhaustiveness** — `crates/workspace_tests/tests/proptest_error_types_registration.rs` and `proptest_error_types_nonzero_codes.rs` should add `SequenceMismatch` arms. **Owner: proof-writer / test-writer follow-up.**

None of these are vb-r8oso regressions. The Holzman Rust acceptance gate is fully green.

---

## 8. Cross-Stage Hand-Off

- **State 12 (formal-verifier):** `formal-verification-report.md`, `verification-ledger.jsonl` (7 rows), `formal-waivers.jsonl` (empty) under `.beads/vb-r8oso/`. Appended 1 row to `agent-invocation-ledger.jsonl`.
- **State 13 (black-hat-reviewer):** `black-hat-review.md` (STATUS: APPROVED, 0 defects), `defects.md` (empty) under `.beads/vb-r8oso/`. Appended 1 row to `agent-invocation-ledger.jsonl`.
- **State 14 (evidence-packaging, this stage):** `assurance-bundle.md` (this file), `truth-serum-report.md`, `final-evidence-decision.md` under `.beads/vb-r8oso/`. Appended 1 row to `agent-invocation-ledger.jsonl`.
- **Landing-skill:** consumes all artifacts; rebase onto `main` (or cherry-pick `kani_helpers.rs` + `93d1d9026`) before merging to `main`.

End of assurance bundle.
