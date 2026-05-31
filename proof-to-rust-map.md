# Proof-To-Rust Bridge Map: vb-fzgdn

## Metadata
- **bead**: vb-fzgdn
- **state**: 7 (proof-to-implementation bridge, RETRY attempt 2)
- **invocation_id**: vb-fzgdn-state7-proof-to-implementation-attempt2
- **proof_review_invocation_id**: vb-fzgdn-state6-proof-reviewer-attempt2
- **previous_bridge_invocation_id**: vb-fzgdn-state7-proof-to-implementation-attempt1 (REJECTED)
- **bridge_review_invocation_id**: vb-fzgdn-state7-proof-reviewer-attempt1
- **bridge_review_disposition**: REJECTED (HIGH: 3 wrong line ranges, 5 PS-007 obligations targeting nonexistent API)
- **mapping_status**: planned (allowed at State 7)
- **source_checkout**: /home/lewis/src/velvet-ballistics
- **isolated_workspace**: /home/lewis/isolated/velvet-ballistics-fresh-replacements/vb-fzgdn

## Changes from Attempt 1 (Retry)

This attempt addresses all findings from proof-to-rust-review.md (F-BR-001 through F-BR-006):

1. **F-BR-001 (HIGH)**: Corrected 3 wrong line ranges verified against production `grep -n` output:
   - `await_timer:123-163` → `await_timer:137-177` (transitions.rs)
   - `next_pending_timer_generation:165-173` → `next_pending_timer_generation:179-187` (transitions.rs)
   - `handle_timer:64-84` / `handle_timer:64-99` → `handle_timer:78-113` (lifecycle/chunk_002.rs)

2. **F-BR-002 (HIGH)**: Removed all references to nonexistent `advance_clock_to` API from PS-007 (POB-028..032). Remapped all 5 obligations to `fire_expired` with proper line numbers. Documented numeric tick refactoring as "deferred to State 12."

3. **F-BR-003 (MEDIUM)**: Added compensating-coverage weakness note in GOD RULE 2 deferral section (Kani `unwrap()`, `Instant::now()` opacity).

4. **F-BR-004 (MEDIUM)**: Updated `evidence_workdir` in all RRO rows to point at isolated workspace where proof artifacts actually live (`/home/lewis/isolated/velvet-ballistics-fresh-replacements/vb-fzgdn`).

5. **F-BR-005 (LOW)**: Corrected all minor off-by-one errors:
   - `error/mod.rs::CommandQueueCapacityExceeded:74-80` → `:75-80`
   - `await_timer:151-159` (PS-002 PendingTimer construction) → `await_timer:165-173`
   - `await_timer:131` (slot validation call site) → `await_timer:145`

6. **F-BR-006 (LOW)**: Provenance chain verified; ledger sequence 12 added.

## Summary
Maps 46 proof obligations to Rust production source locations, independent behavior tests, and separate refinement harnesses. All mapping_status values are `planned` (valid for State 7; closure required by State 12). All line numbers verified against production code via `grep -n`.

10 Verus obligations carry `mapping_status: deferred_to_state11` per proof-reviewer finding F-vb-fzgdn-002-R2 (GOD RULE 2: Verus proofs disconnected from production code). 5 Loom obligations carry `mapping_status: planned` with a note about local-type limitation per finding F-vb-fzgdn-012-R2.

## GOD RULE 2 Deferral
Per proof-review.md (State 6 Attempt 2, finding F-vb-fzgdn-002-R2): all 10 Verus proofs define local types within proof files and prove properties about those local models — not about production code. Zero `extern_spec` bindings, zero `requires`/`ensures` on production `exec fn`. This is the GOD RULE 2 anti-pattern: "prove properties of an enum defined in verification/verus/ and call it a day."

The review was REJECTED but the Verus gap is **documented and deferred to State 11** (rerun_from: 11 on affected obligations). The 10 Verus obligations (POB-001, 006, 011, 015, 019, 023, 028, 033, 037, 042) remain in the obligation set with their Kani/Flux/Proptest/Loom/Fuzz lanes providing compensating coverage during State 7→State 12 bridge execution.

### Compensating Coverage Weakness (F-BR-003 fix)
The compensating coverage through Kani + Proptest + Flux lanes has known weaknesses that must be tracked to State 11/12 closure:
- Kani harness `PS-001-harness.rs` uses `unwrap()` on lines 16, 27, 29 (project rule violation: "No unsafe, unwrap, expect, panic, todo, unimplemented, or dbg").
- Kani harnesses use `Instant::now()` which is opaque to Kani's symbolic engine (no verification possible for time-dependent paths).
- Some Kani harnesses use hardcoded values (e.g., `RunId::new(1)`) rather than `kani::any()` — partial GOD RULE 1 concern.

## Production Code Map

### Seed PS-001: Deadline Arithmetic Safety (5 obligations: POB-001..005)

| Obligation | Verifier | Rust Target | Production Source Ref |
|---|---|---|---|
| POB-001 | verus | Deadline overflow-proof construction | `crates/vb_runtime/src/shard/transitions.rs::Shard::await_timer:137-177` (numeric deadline), `crates/vb_runtime/src/shard/timer_wheel.rs::TimerWheel::insert:61-78` (numeric insert), `crates/vb_runtime/src/shard/types.rs::PendingTimer:36-54` (numeric deadline field) |
| POB-002 | kani | Deadline no-panic/overflow | Same as POB-001 |
| POB-003 | flux-rs | Deadline type bounds | Same as POB-001 minus PendingTimer ref |
| POB-004 | proptest | Deadline property test | Same as POB-001 minus PendingTimer ref |
| POB-005 | loom | Concurrent deadline safety | `timer_wheel.rs::insert:61-78`, `cancel:93-104`, `fire_expired:109-128` |

### Seed PS-002: Numeric-Only Timer State (5 obligations: POB-006..010)

| Obligation | Verifier | Rust Target | Production Source Ref |
|---|---|---|---|
| POB-006 | verus | No Instant in behavior-affecting state | `crates/vb_runtime/src/shard/transitions.rs::Shard::await_timer:165-173` (PendingTimer construction with Instant::now()), `crates/vb_runtime/src/shard/types.rs::PendingTimer:36-54` (deadline: Instant field), `crates/vb_runtime/src/shard/types.rs::ShardCommand::TimerFired:152-161` (deadline: Instant field) |
| POB-007 | kani | Same | Same as POB-006 |
| POB-008 | flux-rs | Same | Same as POB-006 minus TimerFired ref |
| POB-009 | proptest | Same | Same as POB-006 minus TimerFired ref |
| POB-010 | loom | Same + concurrent authority validation | `types.rs::PendingTimer:36-54`, `PendingTimer::matches_authority:46-53`, `Shard::pending_timers:630` |

### Seed PS-003: Authority Validation (4 obligations: POB-011..014)

| Obligation | Verifier | Rust Target | Production Source Ref |
|---|---|---|---|
| POB-011 | verus | Authority validation guards all mutation | `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs::Shard::handle_timer:78-113` (authority gate at lines 85-89), `crates/vb_runtime/src/shard/types.rs::PendingTimer::matches_authority:46-53` |
| POB-012 | kani | Same | Same as POB-011 |
| POB-013 | flux-rs | Same | Same as POB-011 |
| POB-014 | proptest | Same | Same as POB-011 |

### Seed PS-004: Generation Exhaustion (4 obligations: POB-015..018)

| Obligation | Verifier | Rust Target | Production Source Ref |
|---|---|---|---|
| POB-015 | verus | Generation checked_add, never wraps | `crates/vb_runtime/src/shard/transitions.rs::Shard::next_pending_timer_generation:179-187` (checked_add(1)), `crates/vb_runtime/src/shard/timer_wheel.rs::TimerWheel::next_generation:80-88` (checked_add with TimerWheelError::GenerationExhausted:36) |
| POB-016 | kani | Same | Same as POB-015 |
| POB-017 | flux-rs | Same | Same as POB-015 |
| POB-018 | proptest | Same | Same as POB-015 |

### Seed PS-005: Duplicate Delayed-Action Key (4 obligations: POB-019..022)

| Obligation | Verifier | Rust Target | Production Source Ref |
|---|---|---|---|
| POB-019 | verus | Duplicate idempotency, conflict typing | `crates/vb_runtime/src/shard/transitions.rs::Shard::await_timer:137-177` (pending_timers.insert at line 165), `crates/vb_runtime/src/shard/types.rs::Shard::pending_timers:630` (IndexMap<RunId, PendingTimer>) |
| POB-020 | kani | Same | Same as POB-019 |
| POB-021 | flux-rs | Same | Same as POB-019 |
| POB-022 | proptest | Same | Same as POB-019 |

### Seed PS-006: Slot Validation (5 obligations: POB-023..027)

| Obligation | Verifier | Rust Target | Production Source Ref |
|---|---|---|---|
| POB-023 | verus | Slot-derived timer values validated before mutation | `crates/vb_core/src/nodes.rs::CompiledNodeKind::WaitUntil:154-155` (deadline_slot), `WaitEvent:156-160` (timeout_slot), `Ask:162-165` (timeout_slot), `crates/vb_runtime/src/shard/helpers.rs::timer_registration_required:137-147`, `crates/vb_runtime/src/shard/transitions.rs::Shard::await_timer:145` (call site) |
| POB-024 | kani | Same | Same as POB-023 |
| POB-025 | flux-rs | Same | Same as POB-023 minus nodes.rs refs |
| POB-026 | proptest | Same | Same as POB-023 minus nodes.rs refs |
| POB-027 | cargo-fuzz | Same (fuzz boundary) | Same as POB-023 minus nodes.rs refs |

### Seed PS-007: Fire-Expired Ordering (5 obligations: POB-028..032)

NOTE: Remapped from numeric-clock-advancement concept to existing production `fire_expired` (timer_wheel.rs:109-128). Numeric tick refactoring deferred to State 12.

| Obligation | Verifier | Rust Target | Production Source Ref |
|---|---|---|---|
| POB-028 | verus | Monotonic deadline, deterministic fire ordering | `crates/vb_runtime/src/shard/timer_wheel.rs::TimerWheel::fire_expired:109-128` (deadline ordering), `insert:61-78`, `next_deadline:132-134` |
| POB-029 | kani | Same | `fire_expired:109-128`, `insert:61-78`, `cancel:93-104` |
| POB-030 | flux-rs | Same | `fire_expired:109-128` |
| POB-031 | proptest | Same | `fire_expired:109-128` |
| POB-032 | loom | Same + concurrent fire ordering | `fire_expired:109-128`, `insert:61-78` |

### Seed PS-008: Capacity Bounds (4 obligations: POB-033..036)

| Obligation | Verifier | Rust Target | Production Source Ref |
|---|---|---|---|
| POB-033 | verus | Capacity rejection leaves state unchanged | `crates/vb_runtime/src/shard/types.rs::ShardCommandQueue::enqueue:568-572` (QueueFull), `ShardCommandQueue::new:538-549` (CommandQueueCapacityExceeded), `MAX_COMMAND_QUEUE_CAPACITY:508` |
| POB-034 | kani | Same | `enqueue:568-572`, `new:538-549` |
| POB-035 | flux-rs | Same | `new:538-549`, `MAX_COMMAND_QUEUE_CAPACITY:508`, `is_valid_command_queue_capacity:512-514` |
| POB-036 | proptest | Same | `enqueue:568-572`, `new:538-549` |

### Seed PS-009: Zero Duration (5 obligations: POB-037..041)

| Obligation | Verifier | Rust Target | Production Source Ref |
|---|---|---|---|
| POB-037 | verus | Zero-duration deterministic branch | `crates/vb_runtime/src/shard/transitions.rs::Shard::await_timer:137-177` (timer admission, zero-duration handling in numeric domain), `crates/vb_runtime/src/shard/helpers.rs::timer_registration_required:137-147`, `crates/vb_runtime/src/shard/types.rs::PendingTimer:36-54` |
| POB-038 | kani | Same | `Shard::await_timer:137-177` |
| POB-039 | flux-rs | Same | `Shard::await_timer:137-177` |
| POB-040 | proptest | Same | `Shard::await_timer:137-177` |
| POB-041 | loom | Same + concurrent zero-duration interleaving | `Shard::await_timer:137-177`, `Shard::pending_timers:630` |

### Seed PS-010: Atomic Fire+Enqueue (5 obligations: POB-042..046)

| Obligation | Verifier | Rust Target | Production Source Ref |
|---|---|---|---|
| POB-042 | verus | Atomic fire: no partial mutation | `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs::Shard::handle_timer:78-113` (full fire sequence), `crates/vb_runtime/src/error/mod.rs::RuntimeError::CommandQueueCapacityExceeded:75-80`, `crates/vb_runtime/src/shard/types.rs::ShardCommandQueue::enqueue:568-572` |
| POB-043 | kani | Same | `handle_timer:78-113`, `enqueue:568-572` |
| POB-044 | flux-rs | Same | `handle_timer:78-113`, `enqueue:568-572` |
| POB-045 | proptest | Same | `handle_timer:78-113`, `enqueue:568-572` |
| POB-046 | loom | Same + concurrent fire contention | `handle_timer:78-113`, `enqueue:568-572`, `pending_timers:630` |

## Deferred / Unresolved Gaps

1. **GOD RULE 2 — Verus proofs (POB-001, 006, 011, 015, 019, 023, 028, 033, 037, 042):** Disconnected from production code. Deferred to State 11 (rerun_from: 11). Compensating coverage through Kani + Proptest + Flux lanes for same seed+requirement pairs. See "Compensating Coverage Weakness" note above.

2. **Loom local types (POB-005, 010, 032, 041, 046):** Loom models use locally-defined types instead of wrapping production types. Per finding F-vb-fzgdn-012-R2, this is documented with mitigation (meaningful concurrent interleaving patterns mirror production). Will require bisimulation evidence or waiver by State 12.

3. **Clock advancement numeric tick refactoring:** PS-007 (POB-028..032) now maps to production `fire_expired` (timer_wheel.rs:109-128). The function currently accepts `now: Instant`; numeric tick domain refactoring is deferred to State 12. The obligations document that `fire_expired` is the closest existing behavior for deadline-driven expiration.

4. **Numeric deadline fields not yet migrated:** PS-001 and PS-002 obligations target production locations that currently store `Instant` (e.g., `PendingTimer::deadline:41`, `ShardCommand::TimerFired::deadline:158`, `Shard::await_timer` line 171 `Instant::now()`). The bridge maps to the exact lines where numeric replacement must occur. Full materialization requires State 12 implementation.

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
| POB-001 | Deadline overflow-proof construction | true | Shard::await_timer:137-177, TimerWheel::insert:61-78, PendingTimer:36-54 | timer_deadline_safety_test.rs | timer_deadline_refinement.rs | verus | verus --crate-type=lib | 11 |
| POB-002 | Deadline no-panic/overflow | true | Shard::await_timer:137-177, TimerWheel::insert:61-78, PendingTimer:36-54 | timer_deadline_safety_test.rs | timer_deadline_refinement.rs | kani | cargo kani -p vb_runtime --harness ps_001_check | 5 |
| POB-003 | Deadline type bounds | true | Shard::await_timer:137-177, TimerWheel::insert:61-78 | timer_deadline_safety_test.rs | timer_deadline_refinement.rs | flux-rs | cargo flux -p vb_runtime | 5 |
| POB-004 | Deadline property test | true | Shard::await_timer:137-177, TimerWheel::insert:61-78 | timer_deadline_safety_test.rs | timer_deadline_refinement.rs | proptest | cargo test -p vb_runtime --test proptest -- ps_001 | 5 |
| POB-005 | Concurrent deadline safety | true | insert:61-78, cancel:93-104, fire_expired:109-128 | timer_deadline_safety_test.rs | timer_deadline_refinement.rs | loom | cargo test -p vb_runtime --test loom -- ps_001 | 5 |
| POB-006 | No Instant in behavior state | true | Shard::await_timer:165-173, PendingTimer:36-54, TimerFired:152-161 | numeric_timer_state_test.rs | numeric_state_refinement.rs | verus | verus --crate-type=lib | 11 |
| POB-007 | No Instant, Kani check | true | Shard::await_timer:165-173, PendingTimer:36-54, TimerFired:152-161 | numeric_timer_state_test.rs | numeric_state_refinement.rs | kani | cargo kani -p vb_runtime --harness ps_002_check | 5 |
| POB-008 | Numeric-only field refinements | true | Shard::await_timer:165-173, PendingTimer:36-54 | numeric_timer_state_test.rs | numeric_state_refinement.rs | flux-rs | cargo flux -p vb_runtime | 5 |
| POB-009 | Numeric state property | true | Shard::await_timer:165-173, PendingTimer:36-54 | numeric_timer_state_test.rs | numeric_state_refinement.rs | proptest | cargo test -p vb_runtime --test proptest -- ps_002 | 5 |
| POB-010 | Concurrent numeric state | true | PendingTimer:36-54, matches_authority:46-53, pending_timers:630 | numeric_timer_state_test.rs | numeric_state_refinement.rs | loom | cargo test -p vb_runtime --test loom -- ps_002 | 5 |
| POB-011 | Authority validation guards mutation | true | Shard::handle_timer:78-113, matches_authority:46-53 | authority_validation_test.rs | authority_refinement.rs | verus | verus --crate-type=lib | 11 |
| POB-012 | Authority validation no-panic | true | Shard::handle_timer:78-113, matches_authority:46-53 | authority_validation_test.rs | authority_refinement.rs | kani | cargo kani -p vb_runtime --harness ps_003_check | 5 |
| POB-013 | Authority variant constraints | true | Shard::handle_timer:78-113, matches_authority:46-53 | authority_validation_test.rs | authority_refinement.rs | flux-rs | cargo flux -p vb_runtime | 5 |
| POB-014 | Authority mismatch property | true | Shard::handle_timer:78-113, matches_authority:46-53 | authority_validation_test.rs | authority_refinement.rs | proptest | cargo test -p vb_runtime --test proptest -- ps_003 | 5 |
| POB-015 | Generation never wraps | true | next_pending_timer_generation:179-187, next_generation:80-88, GenerationExhausted:36 | generation_exhaustion_test.rs | generation_refinement.rs | verus | verus --crate-type=lib | 11 |
| POB-016 | Generation exhaustion no-panic | true | next_pending_timer_generation:179-187, next_generation:80-88 | generation_exhaustion_test.rs | generation_refinement.rs | kani | cargo kani -p vb_runtime --harness ps_004_check | 5 |
| POB-017 | Generation type bounds | true | next_pending_timer_generation:179-187, next_generation:80-88 | generation_exhaustion_test.rs | generation_refinement.rs | flux-rs | cargo flux -p vb_runtime | 5 |
| POB-018 | Generation property | true | next_pending_timer_generation:179-187, next_generation:80-88 | generation_exhaustion_test.rs | generation_refinement.rs | proptest | cargo test -p vb_runtime --test proptest -- ps_004 | 5 |
| POB-019 | Duplicate key idempotency | true | Shard::await_timer:137-177, pending_timers:630 | duplicate_key_test.rs | duplicate_key_refinement.rs | verus | verus --crate-type=lib | 11 |
| POB-020 | Duplicate key no-panic | true | Shard::await_timer:137-177, pending_timers:630 | duplicate_key_test.rs | duplicate_key_refinement.rs | kani | cargo kani -p vb_runtime --harness ps_005_check | 5 |
| POB-021 | Duplicate key index constraints | true | Shard::await_timer:137-177, pending_timers:630 | duplicate_key_test.rs | duplicate_key_refinement.rs | flux-rs | cargo flux -p vb_runtime | 5 |
| POB-022 | Duplicate key property | true | Shard::await_timer:137-177, pending_timers:630 | duplicate_key_test.rs | duplicate_key_refinement.rs | proptest | cargo test -p vb_runtime --test proptest -- ps_005 | 5 |
| POB-023 | Slot validation before mutation | true | WaitUntil:154-155, WaitEvent:156-160, Ask:162-165, timer_registration_required:137-147, Shard::await_timer:145 | slot_validation_test.rs | slot_validation_refinement.rs | verus | verus --crate-type=lib | 11 |
| POB-024 | Slot validation no-panic | true | WaitUntil:154-155, WaitEvent:156-160, Ask:162-165, timer_registration_required:137-147, Shard::await_timer:145 | slot_validation_test.rs | slot_validation_refinement.rs | kani | cargo kani -p vb_runtime --harness ps_006_check | 5 |
| POB-025 | Slot validation type bounds | true | timer_registration_required:137-147, Shard::await_timer:145 | slot_validation_test.rs | slot_validation_refinement.rs | flux-rs | cargo flux -p vb_runtime | 5 |
| POB-026 | Slot validation property | true | timer_registration_required:137-147, Shard::await_timer:145 | slot_validation_test.rs | slot_validation_refinement.rs | proptest | cargo test -p vb_runtime --test proptest -- ps_006 | 5 |
| POB-027 | Slot validation fuzz boundary | true | timer_registration_required:137-147, Shard::await_timer:145 | slot_validation_test.rs | slot_validation_refinement.rs | cargo-fuzz | cargo fuzz run ps_006_fuzz -- -max_total_time=300 | 5 |
| POB-028 | Monotonic deadline, deterministic fire | true | fire_expired:109-128, insert:61-78, next_deadline:132-134 | clock_advancement_test.rs | clock_advancement_refinement.rs | verus | verus --crate-type=lib | 11 |
| POB-029 | Fire-expired no-panic | true | fire_expired:109-128, insert:61-78, cancel:93-104 | clock_advancement_test.rs | clock_advancement_refinement.rs | kani | cargo kani -p vb_runtime --harness ps_007_check | 5 |
| POB-030 | Fire-expired deadline ordering refinements | true | fire_expired:109-128 | clock_advancement_test.rs | clock_advancement_refinement.rs | flux-rs | cargo flux -p vb_runtime | 5 |
| POB-031 | Fire-expired property | true | fire_expired:109-128 | clock_advancement_test.rs | clock_advancement_refinement.rs | proptest | cargo test -p vb_runtime --test proptest -- ps_007 | 5 |
| POB-032 | Concurrent fire ordering | true | fire_expired:109-128, insert:61-78 | clock_advancement_test.rs | clock_advancement_refinement.rs | loom | cargo test -p vb_runtime --test loom -- ps_007 | 5 |
| POB-033 | Capacity error leaves state | true | enqueue:568-572, new:538-549, MAX_COMMAND_QUEUE_CAPACITY:508 | capacity_bounds_test.rs | capacity_refinement.rs | verus | verus --crate-type=lib | 11 |
| POB-034 | Capacity bounds no-panic | true | enqueue:568-572, new:538-549 | capacity_bounds_test.rs | capacity_refinement.rs | kani | cargo kani -p vb_runtime --harness ps_008_check | 5 |
| POB-035 | Capacity type bounds | true | new:538-549, MAX_COMMAND_QUEUE_CAPACITY:508, is_valid_command_queue_capacity:512-514 | capacity_bounds_test.rs | capacity_refinement.rs | flux-rs | cargo flux -p vb_runtime | 5 |
| POB-036 | Capacity property | true | enqueue:568-572, new:538-549 | capacity_bounds_test.rs | capacity_refinement.rs | proptest | cargo test -p vb_runtime --test proptest -- ps_008 | 5 |
| POB-037 | Zero-duration determinism | true | Shard::await_timer:137-177, timer_registration_required:137-147, PendingTimer:36-54 | zero_duration_test.rs | zero_duration_refinement.rs | verus | verus --crate-type=lib | 11 |
| POB-038 | Zero-duration no-panic | true | Shard::await_timer:137-177 | zero_duration_test.rs | zero_duration_refinement.rs | kani | cargo kani -p vb_runtime --harness ps_009_check | 5 |
| POB-039 | Zero-duration refinement | true | Shard::await_timer:137-177 | zero_duration_test.rs | zero_duration_refinement.rs | flux-rs | cargo flux -p vb_runtime | 5 |
| POB-040 | Zero-duration property | true | Shard::await_timer:137-177 | zero_duration_test.rs | zero_duration_refinement.rs | proptest | cargo test -p vb_runtime --test proptest -- ps_009 | 5 |
| POB-041 | Concurrent zero-duration | true | Shard::await_timer:137-177, pending_timers:630 | zero_duration_test.rs | zero_duration_refinement.rs | loom | cargo test -p vb_runtime --test loom -- ps_009 | 5 |
| POB-042 | Atomic fire+enqueue | true | Shard::handle_timer:78-113, CommandQueueCapacityExceeded:75-80, enqueue:568-572 | atomic_fire_enqueue_test.rs | atomic_fire_enqueue_refinement.rs | verus | verus --crate-type=lib | 11 |
| POB-043 | Atomic fire no-panic | true | Shard::handle_timer:78-113, enqueue:568-572 | atomic_fire_enqueue_test.rs | atomic_fire_enqueue_refinement.rs | kani | cargo kani -p vb_runtime --harness ps_010_check | 5 |
| POB-044 | Fire+enqueue capacity refinement | true | Shard::handle_timer:78-113, enqueue:568-572 | atomic_fire_enqueue_test.rs | atomic_fire_enqueue_refinement.rs | flux-rs | cargo flux -p vb_runtime | 5 |
| POB-045 | Atomic fire property | true | Shard::handle_timer:78-113, enqueue:568-572 | atomic_fire_enqueue_test.rs | atomic_fire_enqueue_refinement.rs | proptest | cargo test -p vb_runtime --test proptest -- ps_010 | 5 |
| POB-046 | Concurrent fire contention | true | Shard::handle_timer:78-113, enqueue:568-572, pending_timers:630 | atomic_fire_enqueue_test.rs | atomic_fire_enqueue_refinement.rs | loom | cargo test -p vb_runtime --test loom -- ps_010 | 5 |

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

## Line Number Verification

All line numbers verified against production code via `grep -n`:
- `transitions.rs::Shard::await_timer` starts at line 137, ends at 177
- `transitions.rs::Shard::next_pending_timer_generation` starts at line 179, ends at 187
- `lifecycle/chunk_002.rs::Shard::handle_timer` starts at line 78, ends at 113
- `transitions.rs::Shard::await_timer` (timer_registration_required call) is at line 145
- `transitions.rs::Shard::await_timer` (PendingTimer construction) is at lines 165-173
- `error/mod.rs::RuntimeError::CommandQueueCapacityExceeded` is at lines 75-80
- `timer_wheel.rs::next_deadline` is at lines 132-134
- All other refs verified and correct
