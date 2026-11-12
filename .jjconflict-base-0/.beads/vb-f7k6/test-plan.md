# Test Plan: vb-f7k6 — Timer Wheel Freshness, Overflow, and Lifecycle Parity

## Summary

- Bead: `vb-f7k6` / Add TLA+ Timer Wheel Model.
- Planning state: State 7 only. This artifact specifies tests; it does not implement tests or production code.
- Startup doctrine read: `/home/lewis/.claude/skills/test-planner/SKILL.md` and `/home/lewis/.agents/skills/test-planner/SKILL.md`; files match, and `.agents` remains authoritative if a future conflict appears.
- Testing philosophy read: `/home/lewis/.agents/skills/test-planner/references/testing-philosophy.md`; plan follows behavior-first, public-API, integration-heavy, exact-assertion testing.
- Approved inputs: `contract.md`, `contract-verification-review.md` (`STATUS: APPROVED`), `proof-review.md` (`STATUS: APPROVED`), `traceability-matrix.jsonl`, `proof-obligations.planned.jsonl`, and `proof-evidence.md`.
- Behaviors identified: 14.
- Trophy allocation target: 5 unit/calc or pure-helper tests, 14 integration/runtime tests, 1 e2e/CI workflow gate, 5 static/formal gates.
- Proptest invariants: 6, conditional on State 10 exposing pure transition/deadline helpers; otherwise keep as implementation-review blockers, not vacuum models.
- Fuzz targets: 0 required; current scope has no parser/deserialization/untrusted byte boundary.
- Kani harnesses: 4 conditional/strongly recommended for State 10 if pure bounded arithmetic/transition helpers exist; no hardcoded Kani shapes.
- Mutation threshold: `>= 90%` killed for scoped runtime timer code; all stale-fire, overflow, lifecycle, and bi-index mutants must die.

## 1. Requirement / Proof / Test Trace Map

| Requirement | Contract clauses | Proof obligations | Planned runtime tests | CI/formal gates |
|---|---|---|---|---|
| `REQ-TW-BOUNDED-OVERFLOW` | `PRE-002`, `PRE-003`, `POST-009`, `INV-001`, `INV-002`, `ERR-DeadlineOverflow` | `PO-001` / `TLA-TW-001`, `VERUS-TW-001` waiver | `timer_insert_returns_deadline_overflow_and_mutates_no_indexes_when_now_plus_duration_exceeds_bound`; `runtime_suspends_or_errors_without_wrap_when_timer_deadline_overflows` | `tlc -config verification/tla/TimerWheel.cfg verification/tla/TimerWheel.tla`; `cargo test -p vb_runtime timer`; conditional Kani/proptest helpers |
| `REQ-TW-INSERT-REPLACE-BIINDEX` | `PRE-001`, `POST-001`, `POST-002`, `INV-003`, `INV-004`, `INV-006` | `PO-002` / `TLA-TW-002` | `timer_insert_creates_exactly_one_indexed_timer_when_run_is_active`; `timer_replace_removes_old_deadline_and_generation_before_new_timer_is_observable`; `timer_replacement_invalidates_captured_old_token_when_old_timer_fires` | Main TLC; runtime timer tests; mutation gate |
| `REQ-TW-CANCEL-COMPLETE` | `PRE-004`, `POST-003`, `INV-005` | `PO-003` / `TLA-TW-003` | `timer_cancel_removes_run_from_run_index_and_all_deadline_buckets`; `timer_cancel_is_idempotent_success_when_run_has_no_timer`; `timer_fire_after_cancel_returns_invalid_timer_fire_and_leaves_state_unchanged` | Main TLC; `TimerWheelCoverageStaleAfterCancel.cfg`; Loom |
| `REQ-TW-FIRE-DUE-ONLY-LIVENESS` | `PRE-005`, `POST-004`, `POST-005`, `POST-006`, `INV-007`, `INV-008`, `INV-011` | `PO-004` / `TLA-TW-004` | `fire_expired_returns_only_deadlines_at_or_before_now`; `fire_expired_removes_returned_timers_from_both_indexes`; `fire_expired_retains_future_deadlines`; `next_deadline_reports_earliest_remaining_deadline` | Main TLC checks `DueOnlyFires`, `FireRemovesReturned`, `DueTimerEventuallyFireable`, deadlock |
| `REQ-TW-STALE-FIRE-REJECTED-NONVACUOUS` | `PRE-006`, `POST-007`, `INV-009`, `ERR-InvalidTimerFire` | `PO-005` / `TLA-TW-005`, `PO-007`, `PO-011` | Failing-first State 10 tests for stale after cancel, stale after replacement, wrong generation, wrong deadline, wrong kind, absent timer, and terminal target | Main TLC; seven coverage probes; Loom; runtime timer tests after authority binding |
| `REQ-TW-LIFECYCLE-DEADLOCK-NONVACUOUS` | `PRE-007`, `POST-008`, `INV-010`, `INV-011`, `ERR-RunNotTimerMutable`, `ERR-InvalidRunId`, `ERR-IndexInvariantViolation` | `PO-006` / `TLA-TW-006` | `timer_insert_cancel_fire_return_lifecycle_error_when_run_is_cancelled_shutdown_completed_failed_or_suspended_overflow`; `timer_fired_cannot_resurrect_terminal_run`; invalid run and fail-closed corruption tests where public API exposes them | Main TLC; `TimerWheelCoverageTerminalRejected.cfg`; runtime tests |
| `REQ-TW-LOOM-STALE-FIRE-CANCEL-REPLACE-RACE` | `PRE-006`, `PRE-007`, `POST-002`, `POST-003`, `POST-007`, `POST-008`, `INV-005`, `INV-006`, `INV-009`, `INV-010`, `INV-011` | `PO-007` / `LOOM-TW-001` | Runtime tests mirror Loom outcome lattice for cancel, replace, terminal. Loom remains target-design until State 10 binds production metadata. | `cargo xtask loom --model timer_fired_cancel` |
| `REQ-TW-RUNTIME-BEHAVIOR-PARITY` | all timer clauses except deadlock-only `INV-011` | `PO-008` / `TEST-TW-001` | Full runtime timer suite through public APIs: `TimerWheel`, `Runtime::timer_fired`, `ShardCommand::TimerFired` | `/usr/bin/env cargo test -p vb_runtime timer`; `moon ci` |
| `REQ-TW-PRODUCTION-AUTHORITY-BINDING` | `PRE-006`, `POST-007`, `INV-006`, `INV-009`, `ERR-InvalidTimerFire` | `PO-011` / `AUTH-TW-001` | Failing-first tests must fail against current RunId-only delivery: stale captured `TimerFired` after replacement/cancel/terminal must yield exact `InvalidTimerFire` and no mutation. | Required State 10 acceptance gate; review must stop if production does not carry/derive `(generation, deadline, kind)` equivalent token |

## 2. Behavior Inventory

1. Timer insertion creates exactly one active timer when the run is active and deadline addition is bounded.
2. Timer insertion rejects overflow without wrapping and without index mutation when `now + duration` exceeds the encoded deadline bound.
3. Timer replacement removes the old run/deadline/generation entry before the new timer is observable.
4. Timer replacement invalidates a captured old `TimerFired` token when the old event is delivered after replacement.
5. Timer cancellation removes all run and deadline index entries when a run has an active timer.
6. Timer cancellation is idempotent success when the run exists but has no active timer.
7. `fire_expired(now)` returns only timers whose deadline is `<= now`.
8. `fire_expired(now)` removes returned timers from both indexes and keeps future timers indexed.
9. `next_deadline` reports the earliest remaining deadline and reflects insert, replacement, fire, and cancel transitions.
10. Runtime `TimerFired` accepts only metadata/token matching the current pending timer authority.
11. Runtime `TimerFired` rejects stale events after cancel, replacement, absent timer, wrong generation, wrong deadline, or wrong kind with exact `TimerError::InvalidTimerFire` and no state mutation.
12. Timer operations after cancellation, shutdown, completion, failure, or suspended-overflow return exact lifecycle errors and cannot resurrect or mutate the run.
13. Invalid `RunId` returns exact `TimerError::InvalidRunId` wherever the public API can receive one.
14. Corrupted bi-index state fails closed with exact `TimerError::IndexInvariantViolation` if public/test-only construction can expose invariant breakage.

## 3. Trophy Allocation

| Layer | Count | Scope | Rationale |
|---|---:|---|---|
| Static/formal | 5 | TLA main model, TLA coverage probes, Loom model, source lint/feature gates, mutation gate | High-risk timer behavior is temporal/concurrent; existing approved proofs remain mandatory evidence. |
| Unit / calc | 5 | Checked deadline helper, pure bi-index transition helpers, due partition helper, freshness-token comparison, lifecycle mutability predicate | Only if State 10 exposes implementation-bound pure helpers. Do not create proof-only duplicate models. |
| Integration | 14 | `TimerWheel` + runtime/shard public surfaces with real in-memory runtime state | Widest layer because contract concerns component boundaries and actual runtime authority. |
| E2E / CI | 1 | `moon ci` includes build, tests, lints, and verification tasks if wired | One high-level workflow gate is enough; user-facing CLI behavior is not the feature. |

Deviation from default trophy ratio: formal/static share is intentionally higher than 5% because the approved contract is a formal timer-wheel safety/liveness artifact and stale fire is concurrency-sensitive.

## 4. BDD Scenarios

### Behavior: Timer insertion creates one active indexed timer

Test name: `timer_insert_creates_exactly_one_indexed_timer_when_run_is_active`

Given an active, timer-mutable run with no pending timer and bounded `now`/`duration` whose checked sum is within the deadline bound.
When the public timer insertion surface schedules a timer of kind `Wait` or `Ask`.
Then the result is an exact success snapshot containing the computed deadline and current freshness token.
And `len()` is exactly `1`.
And `next_deadline()` is exactly the computed deadline.
And `get_kind(run)` is exactly the inserted kind.
And proof obligations mapped: `PO-002`, `PO-008`.

### Behavior: Timer insertion rejects overflow without mutation

Test name: `timer_insert_returns_deadline_overflow_and_mutates_no_indexes_when_now_plus_duration_exceeds_bound`

Given an active run and a pre-existing timer snapshot.
And `now` and `duration` encode the maximum-bound overflow case (`duration > MAX_TIME - now`).
When insertion/replacement is attempted.
Then the result is exactly `Err(TimerError::DeadlineOverflow)` or the runtime's contract-equivalent overflow/suspend error.
And no deadline wraps to a small value.
And the pre-existing timer snapshot is unchanged in both run and deadline indexes.
And the run is in the exact suspended/error state required by the production API, not active with a wrapped timer.
And proof obligations mapped: `PO-001`, `PO-008`.

### Behavior: Replacement removes old index and old authority

Test name: `timer_replace_removes_old_deadline_and_generation_before_new_timer_is_observable`

Given an active run with an old timer at `deadline_old`, `kind_old`, `token_old`.
When the same run is scheduled with a different `deadline_new`, `kind_new`, and resulting `token_new`.
Then exactly one timer exists for the run.
And no deadline bucket contains `(run, deadline_old, kind_old, token_old)`.
And `next_deadline`/`get_kind` reflect the new timer where applicable.
And `token_old != token_new` or the production authority equivalently distinguishes old from new.
And proof obligations mapped: `PO-002`, `PO-011`.

### Behavior: Stale timer after replacement is rejected

Test name: `runtime_timer_fired_returns_invalid_timer_fire_when_old_replaced_timer_event_arrives`

This is a failing-first State 10 test.

Given a runtime run with an old timer and a captured `TimerFired` event carrying `token_old` equivalent to `(generation_old, deadline_old, kind_old)`.
And the run replaces that timer with a new timer before `token_old` is delivered.
When `Runtime::timer_fired` or `ShardCommand::TimerFired` handles the old captured event.
Then the result is exactly `Err(TimerError::InvalidTimerFire)` or exact shard/runtime error variant mapped to `InvalidTimerFire`.
And the run remains in its pre-delivery non-resurrected state.
And the new timer remains current if still pending.
And no workflow progress, wake, acknowledgement, or side effect occurs for the stale event.
And proof obligations mapped: `PO-005`, `PO-007`, `PO-011`.

### Behavior: Stale timer after cancel is rejected

Test name: `runtime_timer_fired_returns_invalid_timer_fire_when_cancelled_timer_event_arrives`

This is a failing-first State 10 test.

Given a runtime run with a pending timer and a captured `TimerFired` event carrying the timer authority token.
And the run cancels the timer before the captured event is delivered.
When the captured event is delivered.
Then the result is exactly `Err(TimerError::InvalidTimerFire)` or exact shard/runtime error variant mapped to `InvalidTimerFire`.
And `len()` remains `0` for that run.
And no run state is resurrected or progressed by the stale event.
And proof obligations mapped: `PO-003`, `PO-005`, `PO-007`, `PO-011`.

### Behavior: Wrong timer token fields are rejected

Test names:
- `runtime_timer_fired_returns_invalid_timer_fire_when_generation_is_wrong`
- `runtime_timer_fired_returns_invalid_timer_fire_when_deadline_is_wrong`
- `runtime_timer_fired_returns_invalid_timer_fire_when_kind_is_wrong`

Given a runtime run with a current pending timer authority token.
When a `TimerFired` event is delivered with exactly one mismatched authority component: generation, deadline, or kind.
Then the result is exactly `Err(TimerError::InvalidTimerFire)`.
And the current timer/run state is unchanged.
And the test asserts the exact unchanged snapshot, not only `is_err()`.
And proof obligations mapped: `PO-005`, `PO-011`.

### Behavior: Absent timer fire is rejected

Test name: `runtime_timer_fired_returns_invalid_timer_fire_when_run_has_no_pending_timer`

Given an active known run with no pending timer.
When a `TimerFired` event for that run is delivered with any bounded authority token.
Then the result is exactly `Err(TimerError::InvalidTimerFire)`.
And no timer or run transition is created.
And proof obligations mapped: `PO-005`, `PO-008`, `PO-011`.

### Behavior: Terminal target fire is rejected

Test names:
- `runtime_timer_fired_returns_run_not_timer_mutable_when_run_is_shutdown`
- `runtime_timer_fired_returns_run_not_timer_mutable_when_run_is_completed`
- `runtime_timer_fired_returns_run_not_timer_mutable_when_run_is_failed`
- `runtime_timer_fired_returns_run_not_timer_mutable_when_run_is_cancelled`
- `runtime_timer_fired_returns_run_not_timer_mutable_when_run_is_suspended_overflow`

Given a run in each terminal or timer-immutable lifecycle state.
When insertion, cancellation with mutation intent, or `TimerFired` delivery is attempted.
Then the result is exactly `Err(TimerError::RunNotTimerMutable)` or the exact public runtime lifecycle error mapped to the contract.
And no timer is inserted.
And no run state changes because of the timer event.
And proof obligations mapped: `PO-006`, `PO-007`, `PO-008`.

### Behavior: Cancel removes all index entries

Test name: `timer_cancel_removes_run_from_run_index_and_every_deadline_bucket`

Given an active run with a pending timer in both indexes.
When cancellation is requested for that run.
Then the result is an exact success snapshot showing no active timer for the run.
And `len()` decreases by exactly one.
And no `fire_expired` call at or after the old deadline can emit that run.
And proof obligations mapped: `PO-003`, `PO-008`.

### Behavior: Cancel without timer is idempotent

Test name: `timer_cancel_returns_success_without_mutation_when_run_has_no_timer`

Given a known active run with no pending timer and a snapshot of all timers.
When cancellation is requested for that run.
Then the result is exact success with the same timer snapshot.
And no error is returned for absence alone.
And proof obligations mapped: `PO-003`, `PO-008`.

### Behavior: Fire expired emits only due timers

Test name: `fire_expired_returns_only_timers_with_deadline_at_or_before_now`

Given timers at `deadline < now`, `deadline == now`, and `deadline > now`.
When `fire_expired(now)` is called.
Then returned entries are exactly the `< now` and `== now` timers with their authority metadata.
And the future timer is not returned.
And proof obligations mapped: `PO-004`, `PO-008`.

### Behavior: Fire expired removes due timers and retains future timers

Test name: `fire_expired_removes_returned_timers_and_retains_future_timers`

Given a timer wheel containing due and future timers.
When `fire_expired(now)` returns due timers.
Then every returned timer is absent from `run_index` and `deadline_index`.
And every future timer remains present with exact original metadata.
And `next_deadline()` is exactly the earliest remaining future deadline, or `None` if no timers remain.
And proof obligations mapped: `PO-004`, `PO-008`.

### Behavior: Invalid run id is rejected

Test name: `timer_operation_returns_invalid_run_id_when_run_is_unknown`

Given a `RunId` outside the known run set/store.
When insert, cancel, or `TimerFired` is attempted through the public runtime surface.
Then the result is exactly `Err(TimerError::InvalidRunId)` or exact public error mapped to the contract.
And no timer index changes.
And proof obligations mapped: `PO-006`, `PO-008`.

### Behavior: Index invariant violation fails closed

Test name: `timer_operation_returns_index_invariant_violation_when_indexes_are_incoherent`

Given a test-only/publicly constructible corrupted snapshot where `run_index` and `deadline_index` disagree.
When the first public operation validates or observes that inconsistency.
Then the result is exactly `Err(TimerError::IndexInvariantViolation)`.
And the operation does not silently repair, wrap, duplicate, or emit an unauthorized timer.
And proof obligations mapped: `PO-006`, `PO-008`.

If no public/test-only constructor can create incoherence without violating encapsulation, this scenario becomes a review assertion: production encapsulation must make `IndexInvariantViolation` unreachable except via fail-closed defensive checks.

## 5. Proptest Invariants

These are required if State 10 exposes implementation-bound pure helpers. They must target production helpers only. Do not write duplicate proof-only models.

### Proptest: checked deadline construction

- Invariant: For any bounded `now` and `duration`, success returns exactly `now + duration` and the value is within `0..=MAX_TIME`; failure occurs exactly when `duration > MAX_TIME - now`.
- Strategy: Generate `now in 0..=MAX_TIME`, `duration in 0..=MAX_DURATION`, with edge-weighted cases `{0, 1, MAX_TIME - 1, MAX_TIME, MAX_TIME + 1 if representable}`.
- Anti-invariant: Any wrapping/saturating success when mathematical addition exceeds `MAX_TIME` must fail the property.
- Requirements: `REQ-TW-BOUNDED-OVERFLOW`; proof `PO-001`.

### Proptest: insert/replace preserves one active timer per run

- Invariant: Any sequence of valid insert/replace operations leaves at most one active timer per run and exact mirror entries in both indexes.
- Strategy: Generate bounded sequences of `(run, now, duration, kind)` over finite runs/kinds with overflow cases filtered into separate overflow property.
- Anti-invariant: A run appearing in two deadline buckets or with stale generation in the active set must fail.
- Requirements: `REQ-TW-INSERT-REPLACE-BIINDEX`; proof `PO-002`.

### Proptest: cancel completeness

- Invariant: After cancelling any known run, that run appears in no active timer and no deadline bucket; cancelling again leaves the snapshot identical.
- Strategy: Generate bounded timer snapshots reachable through public insert/replace operations, then cancel arbitrary known runs.
- Anti-invariant: Any cancelled run emitted by later `fire_expired(MAX_TIME)` must fail.
- Requirements: `REQ-TW-CANCEL-COMPLETE`; proof `PO-003`.

### Proptest: due partition

- Invariant: `fire_expired(now)` partitions timers exactly into returned `deadline <= now` and retained `deadline > now`; the union equals the pre-state active set and the intersection is empty.
- Strategy: Generate reachable snapshots over bounded deadlines and random `now`.
- Anti-invariant: Returning a future timer or retaining a due timer must fail.
- Requirements: `REQ-TW-FIRE-DUE-ONLY-LIVENESS`; proof `PO-004`.

### Proptest: freshness-token authority

- Invariant: `TimerFired` mutates only when `(run, generation, deadline, kind)` or production-equivalent token exactly matches the current pending timer and the run is timer-mutable.
- Strategy: Generate current pending timer plus event metadata with zero or more mismatched fields.
- Anti-invariant: Any mismatched metadata causing progress, timer removal, resurrection, or acknowledgement must fail.
- Requirements: `REQ-TW-STALE-FIRE-REJECTED-NONVACUOUS`, `REQ-TW-PRODUCTION-AUTHORITY-BINDING`; proofs `PO-005`, `PO-011`.

### Proptest: terminal immutability

- Invariant: Timer operations against `Cancelled`, `Shutdown`, `Completed`, `Failed`, or `SuspendedOverflow` never create a timer or mutate run state through timer delivery.
- Strategy: Generate terminal lifecycle state, optional stale/captured timer metadata, and timer operation kind.
- Anti-invariant: Any terminal state gaining a timer or progressing from `TimerFired` must fail.
- Requirements: `REQ-TW-LIFECYCLE-DEADLOCK-NONVACUOUS`; proof `PO-006`.

## 6. Fuzz Targets

No fuzz target is required for this bead because the approved delivery scope and proof obligations identify no parser, deserialization, raw byte, network, JSON/TOML/binary, or untrusted string input boundary in the timer-wheel contract.

Conditional trigger: if State 10 introduces serialized timer tokens, persisted timer events, or external command/API parsing for timer metadata, add a fuzz target for that parser before implementation acceptance. Corpus seeds must include empty input, truncated token, max numeric fields, overflow numeric fields, wrong kind, wrong generation, wrong deadline, duplicate fields, and unknown run.

## 7. Kani Harnesses

Kani is not currently mandatory in `PO-010`; use it only if State 10 creates implementation-bound pure helpers. Never hardcode one dummy `RunFrame`, `WorkflowParts`, or timer snapshot.

### Kani Harness: deadline addition never wraps

- Property: For all bounded `now` and `duration`, checked deadline construction either returns exact bounded sum or exact `DeadlineOverflow` with no mutation marker.
- Bound: Full primitive bounds where tractable; otherwise explicit finite bound matching TLA constants plus hardware max edge cases.
- Rationale: Arithmetic wrap is release-critical and easy for mutation/property tests to miss at hardware boundaries.

### Kani Harness: generated timer snapshots preserve bi-index mirror

- Property: For arbitrary reachable timer snapshots and one insert/replace/cancel/fire transition, `run_index` and `deadline_index` remain exact mirrors.
- Bound: Small finite run/timer count, with `kani::Arbitrary` or safe exhaustive generators for core structures.
- Rationale: Mirrors `INV-003`/`INV-004` without relying on a single hand-shaped example.

### Kani Harness: stale metadata cannot authorize mutation

- Property: For arbitrary current timer authority and arbitrary fired metadata, mutation is possible only under exact authority equality and timer-mutable lifecycle.
- Bound: Finite generations/deadlines/kinds/run states; arbitrary mismatch combinations.
- Rationale: This is the State 10 production authority binding gate.

### Kani Harness: terminal lifecycle absorbs timer mutation

- Property: For every terminal/timer-immutable run state and arbitrary timer operation, next state has no active timer and no timer-induced lifecycle progress.
- Bound: Full finite lifecycle enum, bounded token fields.
- Rationale: Prevents no-resurrection regressions not covered by one scenario.

## 8. Loom / TLA / Runtime Commands

Mandatory proof and test commands to preserve in CI/evidence:

```bash
tlc -config verification/tla/TimerWheel.cfg verification/tla/TimerWheel.tla
tlc -config verification/tla/TimerWheelCoverageValidDelivery.cfg verification/tla/TimerWheel.tla
tlc -config verification/tla/TimerWheelCoverageStaleAfterCancel.cfg verification/tla/TimerWheel.tla
tlc -config verification/tla/TimerWheelCoverageStaleAfterReplace.cfg verification/tla/TimerWheel.tla
tlc -config verification/tla/TimerWheelCoverageWrongGeneration.cfg verification/tla/TimerWheel.tla
tlc -config verification/tla/TimerWheelCoverageWrongDeadline.cfg verification/tla/TimerWheel.tla
tlc -config verification/tla/TimerWheelCoverageWrongKind.cfg verification/tla/TimerWheel.tla
tlc -config verification/tla/TimerWheelCoverageTerminalRejected.cfg verification/tla/TimerWheel.tla
cargo xtask loom --model timer_fired_cancel
/usr/bin/env cargo test -p vb_runtime timer
moon ci
```

Expected TLC coverage-probe semantics: the seven coverage configs are PASS as reachability evidence only when TLC exits with the expected invariant-violation witness for the named `Missing*` invariant. The main TLC config must exit `0`.

State 10 implementation acceptance command subset:

```bash
/usr/bin/env cargo test -p vb_runtime timer_fired
/usr/bin/env cargo test -p vb_runtime timer
cargo xtask loom --model timer_fired_cancel
moon ci
```

## 9. Mutation Checkpoints

Threshold: `>= 90%` killed for scoped timer runtime files. These mutants must not survive:

- Replace checked addition with wrapping/saturating/unchecked addition — killed by overflow scenario and deadline proptest/Kani.
- Change overflow predicate from `>` to `>=` or `<` — killed by exact boundary tests at `duration == MAX_TIME - now` and `duration == MAX_TIME - now + 1`.
- Skip old deadline removal during replacement — killed by replacement bi-index test and due partition property.
- Preserve old generation/token after replacement — killed by stale-after-replacement failing-first test.
- Validate only `RunId` for `TimerFired` — killed by wrong generation/deadline/kind tests and State 10 authority binding test.
- Ignore deadline in token comparison — killed by `runtime_timer_fired_returns_invalid_timer_fire_when_deadline_is_wrong`.
- Ignore kind in token comparison — killed by `runtime_timer_fired_returns_invalid_timer_fire_when_kind_is_wrong`.
- Treat cancelled/terminal stale fire as success/no-op instead of exact error — killed by exact error variant scenarios.
- Return future timers from `fire_expired` by changing `<= now` to `>= now` or removing comparison — killed by due-only tests.
- Fail to remove emitted timers from one index — killed by fire-removal mirror assertions.
- Let terminal/cancelled/shutdown/completed/failed runs insert timers — killed by lifecycle scenarios.
- Convert `InvalidRunId` to generic lifecycle error — killed by exact invalid-run scenario.
- Silently repair corrupted indexes instead of fail-closed error where such state is publicly constructible — killed by index invariant scenario.

Suggested scoped mutation command after tests exist:

```bash
cargo mutants --package vb_runtime --file crates/vb_runtime/src/shard/timer_wheel.rs --file crates/vb_runtime/src/runtime.rs --file crates/vb_runtime/src/shard/types.rs --file crates/vb_runtime/src/shard/lifecycle/chunk_002.rs
```

Do not run whole-workspace mutation by default; scope to the bead blast radius.

## 10. Combinatorial Coverage Matrix

| Scenario | Input class | Expected output | Layer |
|---|---|---|---|
| Insert active run | known active run, bounded deadline | Exact success snapshot; one timer; mirrored indexes | Integration/unit helper |
| Insert unknown run | unknown `RunId` | Exact `TimerError::InvalidRunId`; no mutation | Integration |
| Insert overflow | `duration > MAX_TIME - now` | Exact `TimerError::DeadlineOverflow`/suspend; no wrap; no index mutation | Unit/integration/Kani |
| Replace same run | old timer then new valid timer | Old entry absent; new entry present; old token invalid | Integration/proptest |
| Cancel active timer | known run with timer | Timer absent from all indexes; exact success | Integration/proptest |
| Cancel absent timer | known run without timer | Exact idempotent success; unchanged snapshot | Integration |
| Fire due timers | deadlines `< now` and `== now` | Exact returned set; removed from indexes | Integration/proptest |
| Fire future timers | deadlines `> now` | Not returned; retained exactly | Integration/proptest |
| Stale after cancel | captured token then cancel then deliver | Exact `InvalidTimerFire`; no mutation/no resurrection | Failing-first integration/Loom |
| Stale after replacement | old captured token then replace then deliver | Exact `InvalidTimerFire`; new timer/run unchanged | Failing-first integration/Loom |
| Wrong generation | current timer plus wrong generation event | Exact `InvalidTimerFire`; unchanged snapshot | Integration/proptest/Kani |
| Wrong deadline | current timer plus wrong deadline event | Exact `InvalidTimerFire`; unchanged snapshot | Integration/proptest/Kani |
| Wrong kind | current timer plus wrong kind event | Exact `InvalidTimerFire`; unchanged snapshot | Integration/proptest/Kani |
| Valid timer fire | exact current token, active run | Exact expected run transition and timer removal | Integration/TLA coverage |
| Terminal timer fire | terminal/cancelled/shutdown/completed/failed/suspended-overflow | Exact lifecycle error or `InvalidTimerFire` per public taxonomy; no resurrection | Integration/Loom |
| Invalid run fire | unknown run event | Exact `InvalidRunId`; no timer creation | Integration |
| Corrupt index | incoherent indexes if constructible | Exact `IndexInvariantViolation`; fail closed | Unit/integration |

## 11. CI Gates and Acceptance Criteria

State 8 test-writing is not complete until:

1. Every BDD scenario above has an executable test through public APIs, with exact success values or exact error variants.
2. Current baseline tests are not treated as sufficient for State 10: stale `TimerFired` after cancel/replacement/terminal must be failing-first against RunId-only authority and pass only after production carries/derives freshness metadata/token.
3. Runtime tests assert no mutation/no resurrection using observable state snapshots, not interaction mocks.
4. TLA main model and all coverage probes remain wired as evidence; main config exits `0`, coverage probes produce expected reachability witnesses.
5. Loom `timer_fired_cancel` passes and remains aligned with production authority semantics after State 10.
6. `moon ci` passes.
7. Scoped mutation testing kills all critical mutants listed above and reaches at least `90%` kill rate, or records explicit bead follow-up for any surviving non-critical mutant.
8. No test only asserts `is_ok()` or `is_err()`; every assertion checks exact value, exact error variant, and relevant unchanged state.
9. No production, test, or proof code claims TLA/Loom stale-fire evidence is production-bound until `PO-011` is implemented and tested.

## Open Questions

- State 10 must decide the concrete production representation of timer authority: explicit `(generation, deadline, kind)` fields, opaque token derived from them, or equivalent current-pending-timer capability. Tests must assert behavior through the chosen public surface.
- Confirm the exact production deadline boundary type (`u64`, `usize`, encoded `Instant` horizon, or helper-specific bounded type) before implementing overflow boundary tests.
- Confirm the exact public error mapping for `Runtime::timer_fired`/`ShardCommand::TimerFired`; if public errors wrap `TimerError`, tests must assert the precise wrapper and source variant.
