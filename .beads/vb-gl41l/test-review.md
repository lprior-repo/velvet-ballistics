# Test Review: vb-gl41l (Durable CLI Retry)

**Reviewer:** LOCAL RTX3090 Qwen3.8-27B (omp provider: local-3090-llama)
**Frozen commit:** f6a3631b50f11846c012b6f7d189bba73861dad5
**Baseline:** 685a70fc3ea7fb15e4f3f5a53e3c5803c23e244a
**Date:** 2026-10-21
**Status:** APPROVED

## Tests Executed and Results

| Test Suite | Command | Exit | Time | Result |
|------------|---------|------|------|--------|
| CLI durable retry | `cargo test -p velvet-ballistics --test durable_retry_cli` | 0 | 10.37s | 3 passed, 0 failed |
| Runtime durable retry | `cargo test -p vb_runtime --test durable_retry_execution` | 0 | 6.36s | 2 passed, 0 failed |
| Run checkpoint unit | `cargo test -p vb_runtime run_checkpoint` | 0 | 6.78s | 2 passed, 0 failed |

All executed via `/tmp/vb-run-gate.py` with 420s timeout. Full output captured in `.beads/vb-gl41l/evidence/`.

## Test Assertions Assessed

### CLI Tests (3)
1. **`cli_retry_reexecutes_the_failed_step`** — Strong. Invokes actual `velvet-ballistics retry` binary. Produces real failed run, then retries. Verifies:
   - CLI exits successfully
   - Journal grows after retry (re-execution evidence appended)
   - End-to-end flow through real CLI + FjallJournal + Runtime

2. **`cli_retry_rejects_wrong_step_without_mutation`** — Strong. Attempts retry with step 7 instead of observed failed step 0. Verifies:
   - CLI exits with failure
   - Journal event count unchanged (no mutation on rejection)

3. **`cli_retry_rejects_non_failed_run_without_mutation`** — Strong. Attempts retry on completed run. Verifies:
   - CLI exits with failure
   - Journal event count unchanged

### Runtime Execution Tests (2)
4. **`durable_retry_reopens_and_completes_failed_step`** — Strong. Two-scope test:
   - Scope A: Produces durable failure, verifies checkpoint in journal, captures checkpoint + next_seq
   - Scope B: Colds reopens journal, calls `recover_retry`, verifies re-execution ticket has correct step and attempt=1, completes the action, verifies run completes
   - Verifies journal prefix byte-identical after retry
   - Verifies exactly 2 `ActionScheduledTicket` events (original + retry)
   - Verifies `RunFinished { attempt: 2 }` event

5. **`checkpoint_roundtrips_full_retry_authority`** — Medium. Verifies checkpoint structure:
   - `run_attempt` = 2 (next attempt)
   - `retry_step` = 0
   - Artifact and workflow digests bound
   - Frame PC rewound to retry step
   - Step states: [Pending, Pending] (both reset)
   - `action_attempts` = [1, 0] (captured verbatim)
   - Admission authority retained
   - `prior_seq` points at `RunFailed` event
   - Does NOT exercise actual recovery from this checkpoint (covered by test 4)

### Run Checkpoint Unit Tests (2)
6. **`capture_resets_only_retry_step_and_preserves_action_attempts`** — Unit-level verification of capture behavior
7. **`restore_rejects_a_checkpoint_for_a_different_workflow`** — Workflow identity validation

## Coverage Assessment

**Well-covered:**
- Happy path: produce failure → checkpoint → recover → retry → complete
- Step mismatch rejection (CLI)
- Non-failed-run rejection (CLI)
- Journal prefix identity preservation
- Attempt tracking (run attempt vs. action attempt)
- Checkpoint structure and identity binding
- Workflow identity validation on restore

**Not explicitly tested (but implemented and verified by inspection):**
- Stale attempt ticket rejection (`RuntimeError::StaleAttempt`) — covered by existing shard helper tests but not in this feature's dedicated test suite
- Checkpoint payload tampering (digest mismatch) — implemented in run_ops.rs:262-265, not explicitly tested
- Run attempt overflow (u16::MAX) — implemented with checked arithmetic, not tested at boundary
- Multiple sequential retries (attempt 3+) — not tested
- Non-retryable action failure producing checkpoint — tested implicitly (tests use non-retryable failures)
- Empty journal events rejection — covered by CLI tests indirectly

## Test Quality Observations

**Strengths:**
- All tests use real infrastructure: actual `Runtime`, `FjallJournal`, `StorageRuntimeJournal`
- No manually-constructed checkpoints or fabricated journal events
- Two-scope cold-reopen pattern validates persistence boundary
- Journal prefix identity verification is strong assertion
- CLI tests invoke actual binary, not just library functions

**Minor observations:**
- The `checkpoint_roundtrips_full_retry_authority` test name suggests round-trip testing but it only verifies capture, not restore+recovery. The name could be clearer (e.g., `checkpoint_captures_full_retry_authority`).
- No explicit test for the "checkpoint exists but run already retried successfully" scenario, though this would be caught by `analyze_retry` returning `can_retry: false`.

## Verdict

**APPROVED** — Test suite is adequate for the durable retry feature. Tests are end-to-end, use real infrastructure, and cover the critical paths. The 5 feature tests + 2 unit tests provide meaningful coverage. The 1934 library tests provide additional confidence in the underlying components. No false confidence from library test count alone; the feature-specific tests are substantive.