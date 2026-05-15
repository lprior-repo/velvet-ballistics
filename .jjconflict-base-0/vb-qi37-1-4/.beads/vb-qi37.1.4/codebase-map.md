bead_id: vb-qi37.1.4
bead_title: runtime/recovery: Fail closed on incomplete recovery
phase: 2
updated_at: 2026-05-13T
attempt: 1

# Codebase Map — vb-qi37.1.4

## Bead Goal
Gate unsupported or incomplete recovery paths so runtime never resumes from missing journal data, mismatched artifacts, or partial snapshots as if recovery succeeded.

Acceptance criteria: Incomplete journal, missing snapshot base, artifact digest mismatch, and unsupported event variants all fail closed with typed diagnostics.

---

## Crate Scope

| Crate | Role | Relevance |
|-------|------|-----------|
| `vb_runtime` | Runtime recovery boundary | PRIMARY — `RuntimeRecoveryBoundary` trait, `DurableFrameRecoveryBoundary`, `SummaryRecoveryBoundary` |
| `vb_storage` | Storage journal/recovery | PRIMARY — `RecoveryError`, `verify_digests`, `recover_runtime_frame_seed`, journal replay |
| `vb_core` | Frame, slot, step types | USED — `RunFrame`, `RunId`, `StepIdx`, `SlotIdx`, `SlotValue`, `Taint` |
| `workspace_tests` | Contract test suite | PRIMARY — `vb_qi37_1_1_red_recovery_contract_test.rs` |

---

## Key Files and Symbols

### vb_runtime/src/recovery.rs (749 lines)
- `RuntimeRecoveryBoundary` trait — `summary()`, `hydrate_run_frame()`
- `DurableFrameRecoveryBoundary::from_seed(seed)` — full-frame hydration
- `SummaryRecoveryBoundary::from_summary(summary)` — summary-only, `hydrate_run_frame` → `UnsupportedFullRecoveryHydration`
- `recovery_boundary_from_hydration(hydration)` — factory, returns boxed trait
- `reject_unsupported_live_frame_state(seed)` — checks unsupported flags
- `empty_recovered_frame(seed)` — creates frame from seed metadata
- `apply_recovered_steps(frame, seed)` — maps `RecoveredStepState` → `RunFrame`
- `apply_recovered_slots(frame, seed)` — writes slot value+taint entries
- `apply_recovered_pc(frame, seed)` — sets program counter
- `mark_suspended(frame, step, state)` — Waiting/Asking step states
- Tests: 11 unit tests covering all unsupported state paths

### vb_storage/src/recovery/types.rs (371 lines)
- `RecoveryError` enum — `WorkflowSourceDigestMismatch`, `CompiledIrDigestMismatch`, `ActionAbiMismatch`, `PolicyDigestMismatch`, `NonIdempotentActionBlocked`, `ReplayDivergence`, `NoRecoveryData`, `CorruptSnapshot`, `TerminalStateMismatch`, `FrameDimensionOverflow`
- `UnsupportedRecoveryState` struct — `slot_values`, `slot_taint`, `action_payloads`, `pending_actions` bools; `SUPPORTED` constant; `union()` combinator
- `RecoveryRuntimeSummary` struct — run, first_seq, last_seq, workflow, counters, terminal
- `RecoveryFrameSeed` struct — summary + first_step, step_count, slot_count, pc, steps, slots, pending_actions, unsupported
- `RecoveryHydration` enum — `Summary(summary)` or `FrameSeed(seed)`
- `DigestCheck` enum — `WorkflowSourceOnly`, `WorkflowAndIr`, `Full`
- `RecoveredStepState` enum — `Running`, `Succeeded`, `Failed`, `Waiting`, `Asking`
- `RunSnapshot`, `ActionReplayTracker`

### vb_storage/src/recovery/recover.rs (134 lines)
- `check_workflow_source_digest(journal, run, expected)` — returns `WorkflowSourceDigestMismatch` or `NoRecoveryData`
- `check_compiled_ir_digest(expected, found)` — returns `CompiledIrDigestMismatch`
- `verify_digests(journal, run, workflow_digest, ir_digest, found_ir_digest, level)` — dispatches checks
- `recover_runtime_summary(journal, run)` — returns `RecoveryHydration`
- `recover_runtime_frame_seed(journal, run)` — returns `RecoveryFrameSeed`

### vb_storage/src/recovery/replay/summary.rs (1212 lines)
- `apply_summary_event(summary, event)` — updates counters; RUN-RESUMED/RETRIED/ANSWERED are no-ops (line 60-64)
- `recover_run_admission_from_events(events)` — finds last RunAdmission event
- `summarize_recovery_events(events)` — builds `RecoveryRuntimeSummary` from ordered events
- `recover_runtime_frame_seed_from_events(events)` — builds `RecoveryFrameSeed` with unsupported flags

### vb_storage/src/recovery/replay/core.rs
- `replay_events(events, tracker)` — replays events through ReplayEngine; RUN-RESUMED/RETRIED/ANSWERED are no-ops
- `recover_full_journal(journal, run)` — full journal replay
- `recover_snapshot_plus_tail(journal, run)` — snapshot + tail replay
- `load_snapshot(journal, run)` — loads `RunSnapshot`

### vb_storage/src/error.rs (494 lines)
- `JournalError` enum — full set of storage error types with diagnostic codes

### vb_storage/src/events.rs
- `JournalEvent` enum — all event variants including `RunResumed`, `RunRetried`, `RunAnswered`
- Line 243 comment: "Lifecycle events (RunResumed, RunRetried, RunAnswered) do not carry sequence numbers"

### crates/workspace_tests/tests/vb_qi37_1_1_red_recovery_contract_test.rs (896 lines)
Contract tests for vb-qi37.1.4 — tests INV-RC-001 through INV-RC-009:

| Test | Description | Status |
|------|-------------|--------|
| INV-RC-001 | reject `slot_values: true` | PASS (unit tests in recovery.rs) |
| INV-RC-002 | reject `slot_taint: true` | PASS (unit tests in recovery.rs) |
| INV-RC-003 | reject `action_payloads: true` | PASS (unit tests in recovery.rs) |
| INV-RC-004 | reject nonempty pending_actions + flag true | PASS (unit tests in recovery.rs) |
| INV-RC-005 | no action payload consumed when unsupported | PASS |
| INV-RC-006 | DigestCheck::Full requires action ABI verification | NOTE: implementation missing |
| INV-RC-007 | RunResumed/RunRetried/RunAnswered not silently dropped | PASS |
| INV-RC-008 | ActionAbiMismatch on digest mismatch | NOTE: implementation missing |
| INV-RC-009 | PolicyDigestMismatch on policy mismatch | NOTE: implementation missing |

---

## Identified Gaps (from contract tests)

### GAP-1: Action ABI digest verification (INV-RC-006, INV-RC-008)
- `verify_digests` at `DigestCheck::Full` does NOT verify action ABI digests
- `RecoveryError::ActionAbiMismatch` is defined but never returned by `verify_digests`
- **Required**: `verify_digests` must check action ABI digests when level is `Full`

### GAP-2: Policy digest verification (INV-RC-009)
- `verify_digests` at `DigestCheck::Full` does NOT verify policy digests
- `RecoveryError::PolicyDigestMismatch` is defined but never returned by `verify_digests`
- **Required**: `verify_digests` must check step policy digests when level is `Full`

### GAP-3: Lifecycle event handling in replay (INV-RC-007)
- Tests `inv_rc_007_run_resumed_appears_in_replay_output` and variants pass
- Implementation currently no-ops these events in `apply_summary_event` and `replay_events`
- The comment at `summary.rs:60-64` says they "do not carry sequence numbers and are not part of durable event log ordering"
- However, the test verifies they MUST appear in replay output — this is already working

---

## Risk Tags

| Risk | Category | Notes |
|------|----------|-------|
| Action ABI digest not verified | persistence, contract | GAP-1 — runtime could resume with mismatched action definitions |
| Policy digest not verified | persistence, contract | GAP-2 — runtime could resume with different policy |
| Lifecycle events silently dropped | persistence | INV-RC-007 — tests pass but implementation is no-op; must remain visible |
| Incomplete journal recovery | persistence | Already handled by `NoRecoveryData` error |
| Missing snapshot base | persistence | Already handled by `CorruptSnapshot`/`NoRecoveryData` errors |
| Artifact digest mismatch | persistence | Already handled by `WorkflowSourceDigestMismatch`/`CompiledIrDigestMismatch` |

---

## Verifier Modes Required

- `clippy` — all crates (no new warnings)
- `cargo test` — full test suite including workspace_tests
- `cargo build` — all crates
- No Kani/Loom/Miri required for this bead (no unsafe, no concurrency in recovery paths)

---

## Downstream Owners (for handoff)

| Artifact | Owner |
|----------|-------|
| `contract.md` | rust-contract |
| `proof-strategy.md` | proof-planner |
| `test-plan.md` | test-planner |
| Implementation | holzman-rust |
