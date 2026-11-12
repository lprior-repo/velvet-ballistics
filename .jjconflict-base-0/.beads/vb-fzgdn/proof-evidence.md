# Proof Evidence: vb-fzgdn State 5 Attempt 2

invocation_id: vb-fzgdn-state5-proof-writer-attempt2
bead: vb-fzgdn
state: 5
seq: 8

## Production Binding Audit

Every verification artifact in this delivery was audited for production code binding. Below is the evidence per artifact.

### Kani Harness Production Bindings

| Harness | Production Function(s) Called | Production Type(s) Used |
|---------|------------------------------|------------------------|
| PS-001-harness.rs | `TimerWheel::insert()`, `TimerWheel::get_entry()` | `TimerWheel`, `RunId`, `PendingTimerKind`, `Instant` |
| PS-002-harness.rs | `PendingTimer::matches_authority()` | `PendingTimer`, `PendingTimerKind`, `StepIdx` |
| PS-003-harness.rs | `PendingTimer::matches_authority()` | `PendingTimer`, `PendingTimerKind` |
| PS-004-harness.rs | `u64::checked_add(1)` (same pattern as `Shard::next_pending_timer_generation`) | `u64` |
| PS-005-harness.rs | `TimerWheel::insert()`, `cancel()`, `get_kind()`, `len()`, `is_empty()` | `TimerWheel`, `RunId`, `PendingTimerKind` |
| PS-006-harness.rs | `timer_registration_required()` | `RunState`, `CompiledWorkflow`, `CompiledNodeKind`, `RunFrame` |
| PS-007-harness.rs | `TimerWheel::insert()`, `fire_expired()`, `next_deadline()`, `len()` | `TimerWheel`, `RunId`, `PendingTimerKind` |
| PS-008-harness.rs | `TimerWheel::insert()`, `cancel()`, `len()`, `is_empty()`, `get_entry()` | `TimerWheel`, `RunId`, `PendingTimerKind` |
| PS-009-harness.rs | `TimerWheel::insert()`, `fire_expired()`, `get_entry()`, `len()`, `is_empty()` | `TimerWheel`, `RunId`, `PendingTimerKind` |
| PS-010-harness.rs | `TimerWheel::insert()`, `fire_expired()`, `get_entry()`, `len()`, `is_empty()` | `TimerWheel`, `RunId`, `PendingTimerKind` |

### Proptest Production Bindings

| Property File | Production API Exercised |
|---------------|-------------------------|
| ps_001_property.rs | `TimerWheel::new`, `insert`, `get_entry`, generation tracking |
| ps_002_property.rs | `PendingTimer` struct, `matches_authority()` |
| ps_003_property.rs | `PendingTimer::matches_authority()` with adversarial inputs |
| ps_004_property.rs | `u64::checked_add(1)` pattern |
| ps_005_property.rs | `TimerWheel::insert`, `cancel`, `len`, `get_kind`, `is_empty` |
| ps_006_property.rs | `timer_registration_required()` with real `CompiledWorkflow`, `RunState` |
| ps_007_property.rs | `TimerWheel::insert`, `fire_expired`, `next_deadline`, `len` |
| ps_008_property.rs | `TimerWheel::insert`, `cancel`, `len`, `is_empty`, `next_deadline` |
| ps_009_property.rs | `TimerWheel::insert`, `fire_expired`, `len`, `is_empty` |
| ps_010_property.rs | `TimerWheel::insert`, `fire_expired`, `get_entry`, `len`, `is_empty` |

### Fuzz Production Binding

| Fuzz Target | Production Entry Point |
|-------------|----------------------|
| `ps_006_fuzz.rs` | `timer_registration_required()` with `CompiledNodeKind` fixtures |

### Loom Model Production Correspondence

| Loom Model | Production Analogue |
|------------|-------------------|
| PS-001-model.rs | `TimerWheel::insert`, `cancel`, `fire_expired` — concurrent insert/remove/expire |
| PS-002-model.rs | `PendingTimer::matches_authority` — concurrent authority reads |
| PS-007-model.rs | Monotonic clock advancement — `fire_expired` uses `range(..=now)` which is monotonic |
| PS-009-model.rs | Zero-duration fire — `deadline <= now` fires immediately |
| PS-010-model.rs | Atomic fire+enqueue — `handle_timer` swap_remove before enqueue |

### Flux Refinement Production Correspondences

| Refinement File | Production Type/Function Refined |
|-----------------|--------------------------------|
| PS-001-refinements.rs | `TimerWheelError::GenerationExhausted`, `checked_add(1)` pattern |
| PS-002-refinements.rs | `PendingTimer`, `PendingTimerKind`, `matches_authority` |
| PS-003-refinements.rs | `PendingTimerKind` enum, authority check pattern |
| PS-004-refinements.rs | `checked_add(1)`, generation advancement |
| PS-005-refinements.rs | `TimerWheel::get_kind`, duplicate key handling |
| PS-006-refinements.rs | `timer_registration_required()` |
| PS-007-refinements.rs | `fire_expired`, `range(..=now)`, Instant <= comparison |
| PS-008-refinements.rs | `ShardConfig`, capacity bounds, `MAX_COMMAND_QUEUE_CAPACITY` |
| PS-009-refinements.rs | Zero-duration deadline ≤ now check |
| PS-010-refinements.rs | `swap_remove`, atomic timer removal, command queue capacity |

## Assumptions

1. `std::time::Instant` is treated as opaque; Kani and Verus cannot reason about wall-clock but can verify numeric generation and authority patterns.
2. BTreeMap/HashMap operations in Kani are trusted; Kani's STD library support is limited.
3. Looms models use simplified data structures that structurally correspond to production types but use loom-safe primitives.
4. Verus proofs model production behavior at the spec level; full implementation binding requires extern_spec to production Rust code.

## Blocker Evidence

None. All artifacts successfully reference production types and functions. Deep execution blocked only by PENDING_FORMAL_EXECUTION (deferred to State 8).
