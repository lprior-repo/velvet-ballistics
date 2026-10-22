# Code Review: vb-gl41l (Durable CLI Retry)

**Reviewer:** LOCAL RTX3090 Qwen3.8-27B (omp provider: local-3090-llama)
**Frozen commit:** f6a3631b50f11846c012b6f7d189bba73861dad5
**Baseline:** 685a70fc3ea7fb15e4f3f5a53e3c5803c23e244a
**Date:** 2026-10-21
**Status:** APPROVED

## Scope

Review of durable retry feature: checkpoint capture at failure boundary, checkpoint persistence, CLI `retry` command, and `Runtime::recover_retry` recovery path. No Go-skill or formal proof planning invoked per user flow.

## Changed Files

- `crates/vb_runtime/src/run_checkpoint.rs` — Checkpoint capture/validation/restore
- `crates/vb_runtime/src/shard/transitions.rs` — Failure boundary checkpoint production
- `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs` — `handle_recover` with checkpoint restore
- `crates/vb_runtime/src/shard/lifecycle/chunk_005.rs` — `apply_terminal_failed` checkpoint trigger
- `crates/vb_runtime/src/runtime.rs` — `recover_retry` public API
- `crates/vb_runtime/src/shard/types.rs` — `RecoverRunCommand::for_retry`
- `crates/velvet_ballistics/src/run_ops.rs` — CLI `cmd_retry` implementation
- `crates/velvet_ballistics/src/app_impl.rs` — Retry command wiring
- `crates/velvet_ballistics/src/commands_journal.rs` — Retry analysis

## Verification Against Required Semantics

| Requirement | Status | Evidence |
|-------------|--------|----------|
| Terminal action failure produces durable checkpoint | ✓ | `handle_action_failure` → `fail_run_state_with_retry_checkpoint` (chunk_001.rs:692) |
| Engine failure produces durable checkpoint | ✓ | `apply_terminal_failed` → `fail_run_state_with_retry_checkpoint` (chunk_005.rs:346) |
| Checkpoint at actual failed step (not inferred) | ✓ | `analyze_retry` derives `failed_at_step` from `ActionFailedEvent`/`StepFailed` journal events |
| No last-success+1/saturating/MAX inference | ✓ | CLI exact comparison `observed_failed_step != requested_step` (run_ops.rs:111) |
| Checked run_attempt increment; overflow rejects without mutation | ✓ | `checked_add` + `NonZeroU16::new` (transitions.rs:292-300); Ok(()) on overflow |
| Reset only failed step, preserve successful prefix | ✓ | `capture` resets only `retry_step` to Pending (run_checkpoint.rs:123-128) |
| Checkpoint payload digest bound before activation | ✓ | BLAKE3 digest computed and verified before/after serialization (transitions.rs:321; run_ops.rs:262-265) |
| Legacy missing authority rejected | ✓ | `MissingAdmissionAuthority` error if no admission (run_checkpoint.rs:105) |
| Failed run remains failed until explicit retry | ✓ | Run inserted to `terminal_runs`; checkpoint is evidence-only |
| No automatic recovery from checkpoint presence | ✓ | Recovery requires explicit `recover_retry` call |
| Artifact/workflow/input/contract identity bound | ✓ | `validate_against` checks all identities (run_checkpoint.rs:149-198) |

## Code Quality Observations

**Strengths:**
- Clean separation: capture (runtime failure boundary) vs. restore (CLI recovery)
- Fail-closed design: any capture/validation/serialization failure yields no checkpoint, not partial authority
- Digest verification both at production and consumption
- Full state capture including slots, taint, PC, action_attempts, admission, collect_states, action_contracts
- Proper use of `NonZeroU16` for attempt tracking

**Non-blocking observations:**
- `analyze_retry` determines `pending_step` from `StepStarted`/`StepFailed`/`ActionFailedEvent` but not `RunFailedEvent`. If engine fails before any step, `failed_at_step` is None — CLI rejects, which is correct behavior.
- The `checkpoint_roundtrips_full_retry_authority` test verifies checkpoint structure but does not exercise actual recovery execution from the checkpoint. The `durable_retry_reopens_and_completes_failed_step` test covers end-to-end recovery.

## Findings

No blocking defects found. Implementation matches contract and master requirements.

## Verdict

**APPROVED** — Code is correct, complete, and aligns with the durable retry contract. No source edits required.