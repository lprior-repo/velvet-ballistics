# Codebase Map: vb-fzgdn — deterministic delayed-action timer seam

## Workspace and input gate
- Isolated worktree: `/home/lewis/isolated/velvet-ballistics-fresh-replacements/vb-fzgdn` (verified with `pwd -P`).
- Branch: `fresh/vb-fzgdn`; current commit/base observed as `46cf61591`; source checkout `/home/lewis/src/velvet-ballistics` treated as control-plane only.
- Inputs read: `.beads/vb-fzgdn/STATE.md`, `baseline-report.md`, `global-readiness-report.md`, seed `delivery-scope.jsonl`, and prior capped evidence under `/home/lewis/isolated/velvet-ballistics-main-review/vb-nru0/.beads/vb-nru0/`.
- Prior capped context: `vb-nru0/proof-review.md` rejects State 6 because Rust-local obligations PO-001..PO-028 were unclosed and TLA PO-029..PO-035 were only finite design smoke, with no production deterministic numeric timer seam.
- External Restate inspiration file checked: `/tmp/opencode/restate/crates/worker/src/partition/state_machine/tests/delayed_send.rs` is MISSING (`test -e` exit `1`); do not infer external API shape.

## Target seam summary
This bead should replace the capped path by introducing or contracting a VB-native deterministic delayed-action/timer seam. Current runtime timer authorities are `std::time::Instant`-based and manually fired via typed authority; no numeric replayable tick/delay API or delayed-action admission registry was located.

## Files and symbols mapped

### Public runtime facade
- `crates/vb_runtime/src/runtime.rs` lines 369-390:
  - `Runtime::timer_fired(run)` fail-closes with `RuntimeError::InvalidTimerFire` because run-only delivery has no authority.
  - `Runtime::capture_timer_entry(run)` returns `TimerEntry` authority.
  - `Runtime::timer_entry_fired(entry)` enqueues `ShardCommand::TimerFired { run, generation, deadline, kind }`.
- Gap: no located `ScheduledTick`, numeric `TimerDeadline`, `advance_to_tick`, `schedule_delayed_action`, or deterministic delayed action API.

### Internal timer state and command authority
- `crates/vb_runtime/src/shard/types.rs` lines 29-42, 151-161, 620-641:
  - `PendingTimerKind::{Wait, Ask}`.
  - `PendingTimer { step, kind, generation, deadline: Instant }`.
  - `PendingTimer::matches_authority(generation, deadline, kind)` validates exact authority.
  - `ShardCommand::TimerFired` carries `Instant` deadline.
  - `Shard` owns `pending_timers: IndexMap<RunId, PendingTimer>`.
- Gap: timer state is not numeric/replayable at this seam.

### Timer registration transition
- `crates/vb_runtime/src/shard/transitions.rs` lines 122-173:
  - `Shard::await_timer` journals `WaitScheduled`/`AskScheduled`, computes generation with `next_pending_timer_generation`, then inserts `PendingTimer { deadline: Instant::now() }`.
  - Generation overflow maps to `RuntimeError::InvalidTimerFire`.
- Major collision: `Instant::now()` is a wall-clock capture on the timer registration path; downstream design must replace/inject deterministic numeric time or isolate it away from the acceptance path.

### Timer fire handling
- `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs` lines 64-99:
  - `Shard::handle_timer` rejects missing or mismatched authority before removing pending state.
  - On valid authority, it `swap_remove`s the timer, calls `advance_after_timer_fire`, journals `WaitResolved` for waits, flushes evidence, then applies drive result.
- This is the validation-before-mutation seam for stale/duplicate/wrong-generation/wrong-kind timer tests and Kani.

### Timer helper functions
- `crates/vb_runtime/src/shard/helpers.rs` lines 136-181:
  - `timer_registration_required` registers for `WaitUntil`, `WaitEvent` with timeout, and `Ask` with timeout.
  - `advance_after_timer_fire` validates timer kind against workflow node kind, marks the step succeeded, and moves PC to `node.next`.
- Gap: helper reads timeout-slot presence but does not compute numeric timeout/deadline values.

### TimerWheel module
- `crates/vb_runtime/src/shard/timer_wheel.rs` lines 19-158:
  - `TimerEntry { run, generation, deadline: Instant, kind }`.
  - `TimerWheel::insert`, `cancel`, `fire_expired(now: Instant)`, `next_deadline`, `get_entry`.
  - Uses `BTreeMap<Instant, Vec<TimerEntry>>` plus per-run map; replacement increments generation and overflow returns `TimerWheelError::GenerationExhausted`.
- Status: useful as old behavior reference, but not sufficient for numeric deterministic replay.

### Runtime errors
- `crates/vb_runtime/src/error/mod.rs` lines 7-80:
  - Relevant existing variants: `QueueFull`, `RunNotFound`, `InvalidActionCompletion`, `StaleAttempt`, `AttemptBeyondMax`, `InvalidTimerFire`, `CommandQueueCapacityExceeded`.
  - No explicit delayed-action admission, invalid tick order, duplicate-key, or timer-capacity error was found.

### Core workflow timer nodes
- `crates/vb_core/src/workflow/mod.rs` lines 696-709:
  - Timer-related node kinds: `WaitUntil { deadline_slot }`, `WaitEvent { timeout_slot }`, `Ask { timeout_slot }`, `AskResume { answer }`.
- These provide domain hooks for numeric deadline/timeout extraction, but current runtime registration ignores the actual slot value for deadline computation.

## Existing tests/proofs located
- `crates/vb_runtime/tests/timer_wheel_behavior_tests.rs`: crate-level `TimerWheel` tests; Instant-based.
- `crates/workspace_tests/tests/vb_test_runtime_queue_timer_behavior.rs`: workspace TimerWheel behavior tests through public module; Instant-based.
- `crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs`: public runtime action/tick acceptance style and useful fixture source.
- `crates/workspace_tests/Cargo.toml`: explicit `[[test]]` list does not include `restate_delayed_action_timer_tests`; new integration tests need a stanza.
- `crates/vb_runtime/src/verification/kani/kani_shard_lifecycle_harnesses.rs`: existing Kani lane for timer authority, generation, and action stale fencing, but it uses `std::time::Instant` and includes some `unwrap()` in generator setup; not a complete fresh proof lane for numeric deterministic delayed actions.
- `verification/tla/TimerWheel.tla`: bounded numeric timer-wheel model with finite `TIMES`, checked deadline add, generation, stale fire rejection, and terminal no-resurrection; useful design smoke but not bound to production Rust.
- Prior capped TLA context: `vb-nru0/proof-review.md` states `verification/tla/vb_nru0_delayed_action_timer.tla` remained finite design smoke only and did not close Rust implementation obligations.

## Gaps that downstream must close
1. Define numeric timer types/API: e.g. `TimerTick`, `TimerDuration`, `TimerDeadline`, checked add, deterministic clock/tick injection, and authority that carries numeric deadline rather than `Instant`.
2. Decide public seam: runtime facade method(s), shard-only seam, or test-support seam for deterministic delayed action dispatch.
3. Define delayed-action admission semantics: capacity, duplicate idempotency key behavior, invalid tick ordering before mutation, zero-delay immediate dispatch, and preservation of original scheduled tick.
4. Bridge workflow timeout slots (`WaitUntil`, `WaitEvent`, `Ask`) to numeric values with typed validation errors.
5. Preserve validation-before-mutation in `handle_timer`/successor seam and prove stale authorities cannot mutate run state.
6. Replace or adapt `TimerWheel` and `PendingTimer` away from wall-clock `Instant` where behavior-affecting deterministic replay is required.

## Risk tags
- `temporal`, `determinism`, `replay`, `public-api`, `bounded-resource`, `idempotency`, `validation-before-mutation`, `journal`, `tla-design-smoke`, `kani`, `proptest`, `migration`.

## Recommended downstream owners
- `rust-contract`: define VB-native delayed-action/timer ubiquitous language and illegal-state-free numeric time types.
- `proof-planner`: require Rust-local Kani/proptest/Flux or Verus obligations for checked numeric time, stale fire rejection, duplicate-key idempotency, capacity bounds, and zero-delay immediate path; TLA may remain design smoke only unless bridged.
- `test-planner`/`test-writer`: add public behavior tests only after the public seam is contracted; use existing runtime acceptance style and explicit workspace test manifest stanza.
- Implementation owner: `functional-rust`/`holzman-rust` after contract/proof/test plan.

## Recommended focused evidence commands
- `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_delayed_action_timer_tests` once the test file exists.
- `cargo nextest run -p vb_runtime --test timer_wheel_behavior_tests` for old wheel regression if retained.
- `KANI_FEATURES=<fresh-feature> bash scripts/kani-list.sh vb_runtime` and scoped `cargo kani -p vb_runtime --features <fresh-feature> --harness <name>` only after proof plan approval.
- TLA smoke should use a checked-in jar path that exists; prior cap noted `tools/tla2tools.jar` path failure.
