# Proof-To-Rust Bridge Map: vb-fzgdn

## Metadata
- **bead**: vb-fzgdn
- **state**: 7 (proof-to-implementation bridge)
- **invocation_id**: vb-fzgdn-state7-proof-to-implementation-attempt1
- **proof_review_invocation_id**: vb-fzgdn-state6-proof-reviewer-attempt2
- **proof_review_disposition**: REJECTED (GOD RULE 2 for Verus lane, deferred to State 11)
- **mapping_status**: planned (allowed at State 7)
- **source_checkout**: /home/lewis/src/velvet-ballistics

## Summary
Maps 46 proof obligations to Rust production source locations, independent behavior tests, and separate refinement harnesses. All mapping_status values are `planned` (valid for State 7; closure required by State 12).

10 Verus obligations carry `mapping_status: deferred_to_state11` per proof-reviewer finding F-vb-fzgdn-002-R2 (GOD RULE 2: Verus proofs disconnected from production code). 5 Loom obligations carry `mapping_status: planned` with a note about local-type limitation per finding F-vb-fzgdn-012-R2.

## GOD RULE 2 Deferral
Per proof-review.md (State 6 Attempt 2, finding F-vb-fzgdn-002-R2): all 10 Verus proofs define local types within proof files and prove properties about those local models — not about production code. Zero `extern_spec` bindings, zero `requires`/`ensures` on production `exec fn`. This is the GOD RULE 2 anti-pattern: "prove properties of an enum defined in verification/verus/ and call it a day."

The review was REJECTED but the Verus gap is **documented and deferred to State 11** (rerun_from: 11 on affected obligations). The 10 Verus obligations (POB-001, 006, 011, 015, 019, 023, 028, 033, 037, 042) remain in the obligation set with their Kani/Flux/Proptest/Loom/Fuzz lanes providing compensating coverage during State 7→State 12 bridge execution.

## Production Code Map

### Seed PS-001: Deadline Arithmetic Safety (5 obligations: POB-001..005)

| Obligation | Verifier | Rust Target | Production Source Ref |
|---|---|---|---|
| POB-001 | verus | Deadline overflow-proof construction | `crates/vb_runtime/src/shard/transitions.rs::Shard::await_timer:123-163` (numeric deadline), `crates/vb_runtime/src/shard/timer_wheel.rs::TimerWheel::insert:61-78` (numeric insert), `crates/vb_runtime/src/shard/types.rs::PendingTimer:36-54` (numeric deadline field) |
| POB-002 | kani | Deadline no-panic/overflow | Same as POB-001 |
| POB-003 | flux-rs | Deadline type bounds | Same as POB-001 |
| POB-004 | proptest | Deadline property test | Same as POB-001 |
| POB-005 | loom | Concurrent deadline safety | POB-001 refs + `timer_wheel.rs::TimerWheel::cancel:93-104`, `fire_expired:109-128` |

### Seed PS-002: Numeric-Only Timer State (5 obligations: POB-006..010)

| Obligation | Verifier | Rust Target | Production Source Ref |
|---|---|---|---|
| POB-006 | verus | No Instant in behavior-affecting state | `crates/vb_runtime/src/shard/transitions.rs::Shard::await_timer:151-159` (PendingTimer construction with Instant::now()), `crates/vb_runtime/src/shard/types.rs::PendingTimer:36-54` (deadline: Instant field), `crates/vb_runtime/src/shard/types.rs::ShardCommand::TimerFired:152-161` (deadline: Instant field) |
| POB-007 | kani | Same | Same as POB-006 |
| POB-008 | flux-rs | Same | Same as POB-006 |
| POB-009 | proptest | Same | Same as POB-006 |
| POB-010 | loom | Same + concurrent authority validation | POB-006 refs + `types.rs::PendingTimer::matches_authority:46-53`, `types.rs::Shard::pending_timers:630` |

### Seed PS-003: Authority Validation (4 obligations: POB-011..014)

| Obligation | Verifier | Rust Target | Production Source Ref |
|---|---|---|---|
| POB-011 | verus | Authority validation guards all mutation | `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs::Shard::handle_timer:64-99` (lines 71-84 authority gate), `crates/vb_runtime/src/shard/types.rs::PendingTimer::matches_authority:46-53` |
| POB-012 | kani | Same | Same as POB-011 |
| POB-013 | flux-rs | Same | Same as POB-011 |
| POB-014 | proptest | Same | Same as POB-011 |

### Seed PS-004: Generation Exhaustion (4 obligations: POB-015..018)

| Obligation | Verifier | Rust Target | Production Source Ref |
|---|---|---|---|
| POB-015 | verus | Generation checked_add, never wraps | `crates/vb_runtime/src/shard/transitions.rs::Shard::next_pending_timer_generation:165-173` (checked_add(1)), `crates/vb_runtime/src/shard/timer_wheel.rs::TimerWheel::next_generation:80-88` (checked_add with TimerWheelError::GenerationExhausted:36) |
| POB-016 | kani | Same | Same as POB-015 |
| POB-017 | flux-rs | Same | Same as POB-015 |
| POB-018 | proptest | Same | Same as POB-015 |

### Seed PS-005: Duplicate Delayed-Action Key (4 obligations: POB-019..022)

| Obligation | Verifier | Rust Target | Production Source Ref |
|---|---|---|---|
| POB-019 | verus | Duplicate idempotency, conflict typing | `crates/vb_runtime/src/shard/transitions.rs::Shard::await_timer:123-163` (pending_timers.insert at line 151), `crates/vb_runtime/src/shard/types.rs::Shard::pending_timers:630` (IndexMap<RunId, PendingTimer>) |
| POB-020 | kani | Same | Same as POB-019 |
| POB-021 | flux-rs | Same | Same as POB-019 |
| POB-022 | proptest | Same | Same as POB-019 |

### Seed PS-006: Slot Validation (5 obligations: POB-023..027)

| Obligation | Verifier | Rust Target | Production Source Ref |
|---|---|---|---|
| POB-023 | verus | Slot-derived timer values validated before mutation | `crates/vb_core/src/nodes.rs::CompiledNodeKind::WaitUntil:154-155` (deadline_slot), `WaitEvent:156-160` (timeout_slot), `Ask:162-165` (timeout_slot), `crates/vb_runtime/src/shard/helpers.rs::timer_registration_required:137-147`, `crates/vb_runtime/src/shard/transitions.rs::Shard::await_timer:131` (call site) |
| POB-024 | kani | Same | Same as POB-023 |
| POB-025 | flux-rs | Same | Same as POB-023 |
| POB-026 | proptest | Same | Same as POB-023 |
| POB-027 | cargo-fuzz | Same (fuzz boundary) | Same as POB-023 |

### Seed PS-007: Clock Advancement (5 obligations: POB-028..032)

| Obligation | Verifier | Rust Target | Production Source Ref |
|---|---|---|---|
| POB-028 | verus | Monotonic tick, deterministic fire ordering | `crates/vb_runtime/src/shard/timer_wheel.rs::TimerWheel::fire_expired:109-128` (deadline ordering), numeric clock advancement method (to be added to Shard) |
| POB-029 | kani | Same | Same as POB-028 |
| POB-030 | flux-rs | Same | Same as POB-028 |
| POB-031 | proptest | Same | Same as POB-028 |
| POB-032 | loom | Same + concurrent fire ordering | POB-028 refs + concurrent access to `TimerWheel::by_deadline` and `by_run` |

### Seed PS-008: Capacity Bounds (4 obligations: POB-033..036)

| Obligation | Verifier | Rust Target | Production Source Ref |
|---|---|---|---|
| POB-033 | verus | Capacity rejection leaves state unchanged | `crates/vb_runtime/src/shard/types.rs::ShardCommandQueue::enqueue:568-572` (QueueFull), `ShardCommandQueue::new:538-549` (CommandQueueCapacityExceeded), `MAX_COMMAND_QUEUE_CAPACITY:508` |
| POB-034 | kani | Same | Same as POB-033 |
| POB-035 | flux-rs | Same | Same as POB-033 |
| POB-036 | proptest | Same | Same as POB-033 |

### Seed PS-009: Zero Duration (5 obligations: POB-037..041)

| Obligation | Verifier | Rust Target | Production Source Ref |
|---|---|---|---|
| POB-037 | verus | Zero-duration deterministic branch | `crates/vb_runtime/src/shard/transitions.rs::Shard::await_timer:123-163` (timer admission, zero-duration handling in numeric domain) |
| POB-038 | kani | Same | Same as POB-037 |
| POB-039 | flux-rs | Same | Same as POB-037 |
| POB-040 | proptest | Same | Same as POB-037 |
| POB-041 | loom | Same + concurrent zero-duration interleaving | POB-037 refs + concurrent access to `Shard::pending_timers` |

### Seed PS-010: Atomic Fire+Enqueue (5 obligations: POB-042..046)

| Obligation | Verifier | Rust Target | Production Source Ref |
|---|---|---|---|
| POB-042 | verus | Atomic fire: no partial mutation | `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs::Shard::handle_timer:64-99` (full fire sequence), `crates/vb_runtime/src/error/mod.rs::RuntimeError::CommandQueueCapacityExceeded:74-80`, `crates/vb_runtime/src/shard/types.rs::ShardCommandQueue::enqueue:568-572` |
| POB-043 | kani | Same | Same as POB-042 |
| POB-044 | flux-rs | Same | Same as POB-042 |
| POB-045 | proptest | Same | Same as POB-042 |
| POB-046 | loom | Same + concurrent fire contention | POB-042 refs + concurrent fire of same run from multiple threads |

## Deferred / Unresolved Gaps

1. **GOD RULE 2 — Verus proofs (POB-001, 006, 011, 015, 019, 023, 028, 033, 037, 042):** Disconnected from production code. Deferred to State 11 (rerun_from: 11). Compensating coverage through Kani + Proptest + Flux lanes for same seed+requirement pairs.

2. **Loom local types (POB-005, 010, 032, 041, 046):** Loom models use locally-defined types instead of wrapping production types. Per finding F-vb-fzgdn-012-R2, this is documented with mitigation (meaningful concurrent interleaving patterns mirror production). Will require bisimulation evidence or waiver by State 12.

3. **Clock advancement API not yet implemented:** PS-007 (POB-028..032) targets a numeric `advance_clock_to` API that does not yet exist in production code. The bridge maps to `fire_expired` as the closest existing behavior but adds an implementation obligation for State 12.

4. **Numeric deadline fields not yet migrated:** PS-001 and PS-002 obligations target production locations that currently store `Instant` (e.g., `PendingTimer::deadline`, `ShardCommand::TimerFired::deadline`, `Shard::await_timer` line 157 `Instant::now()`). The bridge maps to the exact lines where numeric replacement must occur. Full materialization requires State 12 implementation.

## Behavior Test References (Planned)
All behavior tests are planned for State 8 (test planning) with materialization in State 12. Each RRO row names the planned test path pattern:

- `crates/vb_runtime/tests/behavior/timer_deadline_safety_test.rs`
- `crates/vb_runtime/tests/behavior/numeric_timer_state_test.rs`
- `crates/vb_runtime/tests/behavior/authority_validation_test.rs`
- `crates/vb_runtime/tests/behavior/generation_exhaustion_test.rs`
- `crates/vb_runtime/tests/behavior/duplicate_key_test.rs`
- `crates/vb_runtime/tests/behavior/slot_validation_test.rs`
- `crates/vb_runtime/tests/behavior/clock_advancement_test.rs`
- `crates/vb_runtime/tests/behavior/capacity_bounds_test.rs`
- `crates/vb_runtime/tests/behavior/zero_duration_test.rs`
- `crates/vb_runtime/tests/behavior/atomic_fire_enqueue_test.rs`

## Refinement Harness References (Planned)
Separate from behavior tests and verifier harnesses:

- `crates/vb_runtime/tests/refinement/timer_deadline_refinement.rs`
- `crates/vb_runtime/tests/refinement/numeric_state_refinement.rs`
- `crates/vb_runtime/tests/refinement/authority_refinement.rs`
- `crates/vb_runtime/tests/refinement/generation_refinement.rs`
- `crates/vb_runtime/tests/refinement/duplicate_key_refinement.rs`
- `crates/vb_runtime/tests/refinement/slot_validation_refinement.rs`
- `crates/vb_runtime/tests/refinement/clock_advancement_refinement.rs`
- `crates/vb_runtime/tests/refinement/capacity_refinement.rs`
- `crates/vb_runtime/tests/refinement/zero_duration_refinement.rs`
- `crates/vb_runtime/tests/refinement/atomic_fire_enqueue_refinement.rs`

## Bridge Matrix

| Proof ID | Claim | Behavior Affecting | Rust Source Refs | Behavior Test Refs | Refinement Harness Refs | Verifier | Evidence Command | Rerun From |
|---|---|---|---|---|---|---|---|---|
| POB-001 | Deadline overflow-proof construction | true | Shard::await_timer, TimerWheel::insert, PendingTimer | timer_deadline_safety_test.rs | timer_deadline_refinement.rs | verus | verus --crate-type=lib | 11 |
| POB-002 | Deadline no-panic/overflow | true | Shard::await_timer, TimerWheel::insert, PendingTimer | timer_deadline_safety_test.rs | timer_deadline_refinement.rs | kani | cargo kani -p vb_runtime --harness ps_001_check | 5 |
| POB-003 | Deadline type bounds | true | Shard::await_timer, TimerWheel::insert | timer_deadline_safety_test.rs | timer_deadline_refinement.rs | flux-rs | cargo flux -p vb_runtime | 5 |
| POB-004 | Deadline property test | true | Shard::await_timer, TimerWheel::insert | timer_deadline_safety_test.rs | timer_deadline_refinement.rs | proptest | cargo test -p vb_runtime --test proptest -- ps_001 | 5 |
| POB-005 | Concurrent deadline safety | true | TimerWheel::insert, cancel, fire_expired | timer_deadline_safety_test.rs | timer_deadline_refinement.rs | loom | cargo test -p vb_runtime --test loom -- ps_001 | 5 |
| POB-006 | No Instant in behavior state | true | Shard::await_timer, PendingTimer, ShardCommand::TimerFired | numeric_timer_state_test.rs | numeric_state_refinement.rs | verus | verus --crate-type=lib | 11 |
| POB-007 | No Instant, Kani check | true | Shard::await_timer, PendingTimer, ShardCommand::TimerFired | numeric_timer_state_test.rs | numeric_state_refinement.rs | kani | cargo kani -p vb_runtime --harness ps_002_check | 5 |
| POB-008 | Numeric-only field refinements | true | Shard::await_timer, PendingTimer | numeric_timer_state_test.rs | numeric_state_refinement.rs | flux-rs | cargo flux -p vb_runtime | 5 |
| POB-009 | Numeric state property | true | Shard::await_timer, PendingTimer | numeric_timer_state_test.rs | numeric_state_refinement.rs | proptest | cargo test -p vb_runtime --test proptest -- ps_002 | 5 |
| POB-010 | Concurrent numeric state | true | PendingTimer, matches_authority, pending_timers | numeric_timer_state_test.rs | numeric_state_refinement.rs | loom | cargo test -p vb_runtime --test loom -- ps_002 | 5 |
| POB-011 | Authority validation guards mutation | true | Shard::handle_timer, matches_authority | authority_validation_test.rs | authority_refinement.rs | verus | verus --crate-type=lib | 11 |
| POB-012 | Authority validation no-panic | true | Shard::handle_timer, matches_authority | authority_validation_test.rs | authority_refinement.rs | kani | cargo kani -p vb_runtime --harness ps_003_check | 5 |
| POB-013 | Authority variant constraints | true | Shard::handle_timer, matches_authority | authority_validation_test.rs | authority_refinement.rs | flux-rs | cargo flux -p vb_runtime | 5 |
| POB-014 | Authority mismatch property | true | Shard::handle_timer, matches_authority | authority_validation_test.rs | authority_refinement.rs | proptest | cargo test -p vb_runtime --test proptest -- ps_003 | 5 |
| POB-015 | Generation never wraps | true | next_pending_timer_generation, next_generation, GenerationExhausted | generation_exhaustion_test.rs | generation_refinement.rs | verus | verus --crate-type=lib | 11 |
| POB-016 | Generation exhaustion no-panic | true | next_pending_timer_generation, next_generation | generation_exhaustion_test.rs | generation_refinement.rs | kani | cargo kani -p vb_runtime --harness ps_004_check | 5 |
| POB-017 | Generation type bounds | true | next_pending_timer_generation, next_generation | generation_exhaustion_test.rs | generation_refinement.rs | flux-rs | cargo flux -p vb_runtime | 5 |
| POB-018 | Generation property | true | next_pending_timer_generation, next_generation | generation_exhaustion_test.rs | generation_refinement.rs | proptest | cargo test -p vb_runtime --test proptest -- ps_004 | 5 |
| POB-019 | Duplicate key idempotency | true | Shard::await_timer, pending_timers | duplicate_key_test.rs | duplicate_key_refinement.rs | verus | verus --crate-type=lib | 11 |
| POB-020 | Duplicate key no-panic | true | Shard::await_timer, pending_timers | duplicate_key_test.rs | duplicate_key_refinement.rs | kani | cargo kani -p vb_runtime --harness ps_005_check | 5 |
| POB-021 | Duplicate key index constraints | true | Shard::await_timer, pending_timers | duplicate_key_test.rs | duplicate_key_refinement.rs | flux-rs | cargo flux -p vb_runtime | 5 |
| POB-022 | Duplicate key property | true | Shard::await_timer, pending_timers | duplicate_key_test.rs | duplicate_key_refinement.rs | proptest | cargo test -p vb_runtime --test proptest -- ps_005 | 5 |
| POB-023 | Slot validation before mutation | true | WaitUntil, WaitEvent, Ask, timer_registration_required, Shard::await_timer | slot_validation_test.rs | slot_validation_refinement.rs | verus | verus --crate-type=lib | 11 |
| POB-024 | Slot validation no-panic | true | WaitUntil, WaitEvent, Ask, timer_registration_required, Shard::await_timer | slot_validation_test.rs | slot_validation_refinement.rs | kani | cargo kani -p vb_runtime --harness ps_006_check | 5 |
| POB-025 | Slot validation type bounds | true | timer_registration_required, Shard::await_timer | slot_validation_test.rs | slot_validation_refinement.rs | flux-rs | cargo flux -p vb_runtime | 5 |
| POB-026 | Slot validation property | true | timer_registration_required, Shard::await_timer | slot_validation_test.rs | slot_validation_refinement.rs | proptest | cargo test -p vb_runtime --test proptest -- ps_006 | 5 |
| POB-027 | Slot validation fuzz boundary | true | timer_registration_required, Shard::await_timer | slot_validation_test.rs | slot_validation_refinement.rs | cargo-fuzz | cargo fuzz run ps_006_fuzz -- -max_total_time=300 | 5 |
| POB-028 | Monotonic clock, determinism | true | TimerWheel::fire_expired, insert, next_deadline | clock_advancement_test.rs | clock_advancement_refinement.rs | verus | verus --crate-type=lib | 11 |
| POB-029 | Clock advancement no-panic | true | TimerWheel::fire_expired, insert, cancel | clock_advancement_test.rs | clock_advancement_refinement.rs | kani | cargo kani -p vb_runtime --harness ps_007_check | 5 |
| POB-030 | Monotonic tick refinements | true | TimerWheel::fire_expired | clock_advancement_test.rs | clock_advancement_refinement.rs | flux-rs | cargo flux -p vb_runtime | 5 |
| POB-031 | Clock advancement property | true | TimerWheel::fire_expired | clock_advancement_test.rs | clock_advancement_refinement.rs | proptest | cargo test -p vb_runtime --test proptest -- ps_007 | 5 |
| POB-032 | Concurrent fire ordering | true | TimerWheel::fire_expired, insert | clock_advancement_test.rs | clock_advancement_refinement.rs | loom | cargo test -p vb_runtime --test loom -- ps_007 | 5 |
| POB-033 | Capacity error leaves state | true | ShardCommandQueue::enqueue, new, MAX_COMMAND_QUEUE_CAPACITY | capacity_bounds_test.rs | capacity_refinement.rs | verus | verus --crate-type=lib | 11 |
| POB-034 | Capacity bounds no-panic | true | ShardCommandQueue::enqueue, new | capacity_bounds_test.rs | capacity_refinement.rs | kani | cargo kani -p vb_runtime --harness ps_008_check | 5 |
| POB-035 | Capacity type bounds | true | ShardCommandQueue::new, MAX_COMMAND_QUEUE_CAPACITY, is_valid_command_queue_capacity | capacity_bounds_test.rs | capacity_refinement.rs | flux-rs | cargo flux -p vb_runtime | 5 |
| POB-036 | Capacity property | true | ShardCommandQueue::enqueue, new | capacity_bounds_test.rs | capacity_refinement.rs | proptest | cargo test -p vb_runtime --test proptest -- ps_008 | 5 |
| POB-037 | Zero-duration determinism | true | Shard::await_timer, timer_registration_required, PendingTimer | zero_duration_test.rs | zero_duration_refinement.rs | verus | verus --crate-type=lib | 11 |
| POB-038 | Zero-duration no-panic | true | Shard::await_timer | zero_duration_test.rs | zero_duration_refinement.rs | kani | cargo kani -p vb_runtime --harness ps_009_check | 5 |
| POB-039 | Zero-duration refinement | true | Shard::await_timer | zero_duration_test.rs | zero_duration_refinement.rs | flux-rs | cargo flux -p vb_runtime | 5 |
| POB-040 | Zero-duration property | true | Shard::await_timer | zero_duration_test.rs | zero_duration_refinement.rs | proptest | cargo test -p vb_runtime --test proptest -- ps_009 | 5 |
| POB-041 | Concurrent zero-duration | true | Shard::await_timer, pending_timers | zero_duration_test.rs | zero_duration_refinement.rs | loom | cargo test -p vb_runtime --test loom -- ps_009 | 5 |
| POB-042 | Atomic fire+enqueue | true | Shard::handle_timer, CommandQueueCapacityExceeded, enqueue | atomic_fire_enqueue_test.rs | atomic_fire_enqueue_refinement.rs | verus | verus --crate-type=lib | 11 |
| POB-043 | Atomic fire no-panic | true | Shard::handle_timer, enqueue | atomic_fire_enqueue_test.rs | atomic_fire_enqueue_refinement.rs | kani | cargo kani -p vb_runtime --harness ps_010_check | 5 |
| POB-044 | Fire+enqueue capacity refinement | true | Shard::handle_timer, enqueue | atomic_fire_enqueue_test.rs | atomic_fire_enqueue_refinement.rs | flux-rs | cargo flux -p vb_runtime | 5 |
| POB-045 | Atomic fire property | true | Shard::handle_timer, enqueue | atomic_fire_enqueue_test.rs | atomic_fire_enqueue_refinement.rs | proptest | cargo test -p vb_runtime --test proptest -- ps_010 | 5 |
| POB-046 | Concurrent fire contention | true | Shard::handle_timer, enqueue, pending_timers | atomic_fire_enqueue_test.rs | atomic_fire_enqueue_refinement.rs | loom | cargo test -p vb_runtime --test loom -- ps_010 | 5 |

## Bridge Mapping Completeness

| Verifier | Obligations | Mapped | Deferred | Unresolved |
|---|---|---|---|---|
| verus | 10 | 10 (all deferred to State 11) | 10 | 0 |
| kani | 10 | 10 | 0 | 0 |
| flux-rs | 10 | 10 | 0 | 0 |
| proptest | 10 | 10 | 0 | 0 |
| loom | 5 | 5 (local-type limitation documented) | 0 | 0 |
| cargo-fuzz | 1 | 1 | 0 | 0 |
| **Total** | **46** | **46** | **10** | **0** |
