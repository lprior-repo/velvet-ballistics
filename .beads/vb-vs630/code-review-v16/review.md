**Bead**: vb-vs630  
**State**: 3  
**Reviewer**: black-hat-reviewer  
**Source checkout**: /home/lewis/src/isoloated/vb-vs630-code-review-local3090-v16  
**Attempt**: 16  

## Gate Result
**STATUS: APPROVED**

---

## PHASE 1: Contract & Bead Parity

| Requirement | Status | Evidence |
|-------------|--------|----------|
| CLI stamps accepted_at_seq | ✅ | `prepare_stamped_submission` calls `prepare_artifact_with_sequence` |
| Counter increment under existing write_lock | ✅ | `commit_run_submission` calls helper after acquiring `_guard` |
| Crate-private lock-free prepare helper factored | ✅ | `prepare_artifact_with_sequence` in `artifact_acceptance.rs:115-123` |
| Digest rebinding cascade (4 fields) | ✅ | `bind_artifact_digest` in `prepare_artifact`; `scratch.validate_binding()` revalidates all 4 |
| Single atomic batch (counter + admission) | ✅ | `put_artifact_sequence` staged in same batch as `stage()`/`stage_acceptance()` |
| Per-run event sequences unchanged | ✅ | RunAccepted seq=0, RunAdmission seq=1 (submission.rs:104-122) |
| Default/API path unchanged | ✅ | `issue_next_artifact_sequence` uses refactored `issue_artifact` (artifact_acceptance.rs:188-204) |
| No deadlock | ✅ | `prepare_artifact_with_sequence` is lock-free; caller holds lock |
| Duplicate-run rejection precedes issuance | ✅ | `check_duplicate_run` called before `prepare_stamped_submission` (submission.rs:190-191) |
| Three policies accepted | ✅ | `validate_policy` matches Relaxed/Journaled/Strict (submission.rs:60-72) |
| Relaxed eventless | ✅ | `stage_acceptance` returns early for Relaxed (submission.rs:97-98) |
| Private rebind scratch validates final identities | ✅ | `scratch.validate_binding()` called in `prepare_stamped_submission` (submission.rs:173) |
| Error types match contract | ✅ | SequenceOverflow, MalformedKeyspaceRow{prefix:0,expected_len:8}, WriteLockPoisoned, RunAlreadyExists, Fjall(...), PostcardEncodeFailed, PayloadDigestMismatch |
| Arbitrary commit IO may have published | ✅ | No rollback claimed; contract acknowledged |
| Combined event+metadata bounds | ✅ | `append_event` checks `staged_bytes + staged_metadata_bytes + encoded_len`; `put_artifact_sequence` checks `current_bytes + new_metadata` |
| No fictional whole-payload bound / MAX_BATCH_COUNT putter cap | ✅ | Counter write has no MAX_BATCH_COUNT check; byte limit is per-batch not per-payload |
| None budget = unbounded with overflow checked | ✅ | `if let Some(limit)` guards all limit checks; `checked_add` always applied |
| Error reservation aborts batch before commit | ✅ | `put_artifact_sequence` sets `self.aborted = true` on compute failure |
| Old append soft budget semantics preserved | ✅ | `append_event` byte limit check returns error without aborting batch |

---

## PHASE 2: Farley Engineering Rigor

### New/Changed Function Lengths (nonblank/noncomment lines)

| Function | File | Lines | Limit | Status |
|----------|------|-------|-------|--------|
| `compute_put_artifact_sequence` | batch/artifact_sequence.rs | 23 | 25 | ✅ |
| `put_artifact_sequence` | batch/artifact_sequence.rs | 25 | 25 | ✅ |
| `prepare_artifact_with_sequence` | journal/artifact_acceptance.rs | 9 | 25 | ✅ |
| `issue_artifact` (changed) | journal/artifact_acceptance.rs | 17 | 25 | ✅ |
| `commit` (new sig) | inputs/submission.rs | 15 | 25 | ✅ |
| `check_duplicate_run` (new) | inputs/submission.rs | 9 | 25 | ✅ |
| `prepare_stamped_submission` (new) | inputs/submission.rs | 25 | 25 | ✅ |
| `commit_run_submission` (changed) | inputs/submission.rs | 15 | 25 | ✅ |

**Legacy existing (unchanged logic, noted only)**: `append_event` (~50 lines) — existing cold function, not rewritten per "don't blanket require unrelated legacy rewrite" directive.

### Parameter Counts

| Function | Params | Limit | Status |
|----------|--------|-------|--------|
| `compute_put_artifact_sequence` | 3 | 5 | ✅ |
| `put_artifact_sequence` | 1 (+self) | 5 | ✅ |
| `prepare_artifact_with_sequence` | 2 | 5 | ✅ |
| `issue_artifact` | 3 | 5 | ✅ |
| `commit` | 3 (+self) | 5 | ✅ |
| `check_duplicate_run` | 2 | 5 | ✅ |
| `prepare_stamped_submission` | 2 | 5 | ✅ |
| `commit_run_submission` | 2 (+self) | 5 | ✅ |

### Functional Core / Imperative Shell Separation
- Pure logic: counter read/increment, digest binding, validation — all in pure functions
- I/O: batch staging and commit — in shell functions
- No I/O hiding inside calculations

---

## PHASE 3: Holzman Rust (The Big 6)

| Rule | Status | Evidence |
|------|--------|----------|
| Zero `unsafe` | ✅ | `#![forbid(unsafe_code)]` in all 6 changed modules |
| Zero `.unwrap()`/`.expect()` | ✅ | All error handling via `?` or explicit `Result` |
| Zero `panic!`/`todo!`/`dbg!` | ✅ | None found in changed code |
| Zero `let mut` (unnecessary) | ✅ | Only `let mut batch = batch` in `commit` (needed for staging) |
| Checked arithmetic | ✅ | All arithmetic uses `checked_add`, `try_from`, `ok_or` |
| No unchecked indexing/slicing | ✅ | `try_into()` for array conversion; no direct indexing |
| Newtypes for domain values | ✅ | `IssuedArtifactSequence` wraps `NonZeroU64` |
| Parse, don't validate | ✅ | Digest verification at admission boundaries |

---

## PHASE 4: Ruthless Simplicity & DDD

| Check | Status |
|-------|--------|
| No Option-based state machines | ✅ |
| CUPID compliant | ✅ |
| No clever abstractions | ✅ |
| Boring, direct implementation | ✅ |
| No premature generalization | ✅ |

---

## PHASE 5: The Bitter Truth

The implementation is painfully obvious in the best way. No clever tricks, no hidden state, no abstraction layers. The factorization of `prepare_artifact_with_sequence` is clean and necessary. The scratch-clone-and-rebind approach in `prepare_stamped_submission` is straightforward and correct. Byte accounting is explicit and verified. The code reads like the contract was translated line-by-line into Rust.

No "junior developer trying to prove how smart they are" energy. This is competent, boring engineering.

---

## Findings (Ordered by Severity)

| Finding | Severity | File:Line | Status |
|---------|----------|-----------|--------|
| None | — | — | — |

No critical, high, medium, or low findings.

---

## Quality Gates

| Gate | Result | Evidence |
|------|--------|----------|
| Source compilation (cargo check V12) | ✅ | User-supplied: PASS |
| Existing storage tests (1658, V13) | ✅ | User-supplied: PASS, 0 skips |
| Code review (this document) | ✅ | All 5 phases pass |

**Note**: This review is CODE-ONLY. Tests have not been reviewed or run as part of this pass; they are pending separate independent gate per user instructions. No new test acceptance.

---

## Verdict

**STATUS: APPROVED**

### Summary
The vb-vs630 implementation correctly stamps CLI `RunSubmission` artifacts with the global `accepted_at_seq` counter while preserving the atomic multi-record publication boundary. The factorization into a crate-private lock-free prepare helper (`prepare_artifact_with_sequence`) enables both the default/API path and the CLI path to share issuance logic without deadlock. All four digest binding fields are correctly rebound and revalidated. The new `put_artifact_sequence` batch putter correctly accounts for the 19-byte metadata write (11-byte key + 8-byte counter value) in the combined event+metadata byte budget while preserving the soft-budget semantics of the existing `append_event` path. All contract acceptance criteria and coordination bounds are met. No findings.

---

## Required Repair Actions (if REJECTED)

None — APPROVED.