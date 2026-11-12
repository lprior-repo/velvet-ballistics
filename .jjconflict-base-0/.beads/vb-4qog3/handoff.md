# Handoff — vb-4qog3 Independent LOCAL5090 Review of vb-hz9v8 B/C/D

Reviewer: LOCAL5090 (through OMP)
Workspace: `/home/lewis/src/isoloated/velvet-ballistics-vb-4qog3-local5090-review`
Source: Frozen at `0d348f5ae0aba7a9cce32d4ad0b990e66da07d2b`
Baseline: `423d8ae8b667d82b4d36e57d29d7ef2aede08bdf`

## Scope

Independent code review and test review of the frozen vb-hz9v8 B/C/D implementation.
No production modifications by reviewer. New independent adversarial tests written
in `crates/vb_runtime/tests/vb_4qog3_independent_review.rs`.

Finding A (stale `ActionTicket`) is owned by LOCAL5090 `vb-wop2u` and is NOT
in scope for this review.

## Reviews

- **Code review:** `.beads/vb-4qog3/code-review.md` — VERDICT: APPROVED
- **Test review:** `.beads/vb-4qog3/test-review.md` — VERDICT: APPROVED

## Test Execution

All tests run through `/tmp/vb-run-gate.py` with true exit codes. Raw logs in
`.beads/vb-4qog3/evidence/`.

- `cargo check -p vb_runtime` → 0
- `durable_retry_boundary` → 10 passed
- `durable_retry_execution` → 2 passed
- `durable_retry_cli` → 3 passed
- `recovery unit tests` → 0 failed
- **Independent adversarial tests** → 9 passed

**Reviewer test fixes (fallible conversions):** Replaced two `.unwrap()` calls
on `NonZeroU16::new(99)` (lines 489/572 of `vb_4qog3_independent_review.rs`) with
fallible `ok_or("99 must be nonzero")?` pattern. Removed 5 unnecessary `mut`
qualifiers on runtime variables (lines 278/433/468/494/562). All changes confined
to reviewer test file only. Final rustfmt applied. All 9 tests pass via
`/tmp/vb-run-gate.py` with exit code 0.

**Evidence wording correction:** Development used ad-hoc `cargo test | head/tail/grep`
commands for iteration; the final `/tmp/vb-run-gate.py` invocation and its exit
code/log metadata constitute the true exit evidence. Raw logs for each development
call are in `.beads/vb-4qog3/evidence/`.

**Rejection test scope note (D):** Rejection tests (D) and cross-mutation tests
assert that rejected retry attempts (wrong `run_attempt`, wrong `prior_seq`,
cross-run checkpoints, already-consumed checkpoints) fail without mutating
journal length, event count, or journal bytes. They do not assert individual
journal byte/frame/counter immutability at the physical storage layer;
journal-level equality is the verified invariant.

## Key Findings

### B — CLI Durability (HIGH)
`cmd_retry` correctly uses `shared_strict` (line 177, `run_ops.rs`). Verified the
full SyncAll chain: `shared_strict` → `append_strict` → `JournalWriteBatch::strict()`
→ `fjall::PersistMode::SyncAll`. Comment documentation is accurate.

### C — Auto Recovery of Failed Run (CRITICAL)
`run_is_currently_terminal` (full.rs:125-141) correctly excludes `RetryCheckpoint`
from current-attempt computation. Used in runtime.rs:873 to return `CannotResume`.
No over-rejection of genuinely suspended/active runs.

### D — Raw `recover_retry` Validation (MEDIUM)
`validate_retry_checkpoint_against_durable_chain` (runtime.rs:453-593) implements
all 8 validation checks from the contract. Uses checked arithmetic throughout.
No state/journal mutation on validation failure. Journal head check correctly
detects already-consumed checkpoints.

## Source Changes (Reviewer-Only)

- `crates/vb_runtime/tests/vb_4qog3_independent_review.rs` — new test file with 9
  independent adversarial tests covering B, C, D and cross-cutting no-mutation
  requirements.

No production code changes. No changes to author's tests. No changes to helper
scripts or .gitignore.

## Limitations

- CLI durability test is integration-level, not payload-tamper level (author's
  CLI boundary is verified by source inspection)
- Finding A oracle remains RED (owned by vb-wop2u)
- B (strict durability barrier) verification is source inspection + runtime reopen
  behavior, not real CLI fault/power-cut injection
- No Verus/Flux/Kani fleet run (per instructions)

## Status

Bead remains OPEN pending merge/main. Root owns final publication/review/CI/main
and bead remote sync.