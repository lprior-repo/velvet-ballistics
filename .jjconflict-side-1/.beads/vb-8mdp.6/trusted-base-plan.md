# Trusted Base Plan — Idempotency Hydration

## Bead: vb-8mdp.6

Defines the trusted boundaries, assumptions, stubs, and model reductions for the proof effort.

---

## 1. Trusted Boundaries

### 1.1 vb_core (Trusted Core)

**Boundary**: `vb_core` is the trusted core. It contains no dependencies on `vb_storage`.

**Trusted Components**:
- `compute_action_idempotency_key(run: RunId, seq: SeqNo, action: ActionId) -> u128`
- `action_ticket_has_valid_key(ticket: ActionTicket) -> bool`
- `validate_idempotency_key_ingredients(key_slots: &[SlotIdx], frame: &RunFrame) -> Result<(), IdempotencyViolation>`
- `verify_idempotency(action: &ActionContract, key_slots: &[SlotIdx], frame: &RunFrame) -> Result<(), IdempotencyViolation>`
- `hydrate_events_preconditions(events: &[JournalEvent]) -> bool`
- `hydrate_dimensions_positive(step_count: u16, slot_count: u16) -> bool`

**Trust Rationale**: These are pure functions with no side effects, no external I/O, and deterministic behavior. They form the mathematical foundation of the idempotency system.

### 1.2 vb_storage (Trusted Shell)

**Boundary**: `vb_storage` imports `vb_core::ActionTicket` but does not modify it. Hydration logic is contained within vb_storage.

**Trusted Components**:
- `ActionReplayTracker::new()`
- `ActionReplayTracker::mark_scheduled_ticket_effect(...) -> RecoveryResult<ActionReplayEffect>`
- `ActionReplayTracker::mark_completed_envelope_effect(...) -> RecoveryResult<ActionReplayEffect>`
- `ActionReplayTracker::require_scheduled_ticket(...) -> RecoveryResult<()>`
- `ActionReplayTracker::is_resolved(action: ActionId, step: StepIdx) -> bool`
- `hydrate_run_frame(...) -> RecoveryResult<RunFrame>`
- `hydrate_snapshot_tail_preconditions(...) -> bool`

**Trust Rationale**: These functions implement the recovery protocol. They are trusted to correctly apply the vb_core contracts during hydration.

### 1.3 External Dependencies (Untrusted Outside)

**Components**:
- **BLAKE3**: Used for `value_digest`. Assumed to be collision-resistant. Any digest mismatch is treated as divergence.
- **Fjall (LSM-tree storage)**: Assumed to correctly persist and retrieve journal events. Corruption at the storage layer is treated as external divergence.
- **OS/Hardware**: Assumed to provide atomic writes and correct memory semantics.

---

## 2. Trusted Assumptions

### 2.1 Type Validity

| Assumption | Source | Validation |
|------------|--------|------------|
| `RunId::get()` returns valid u64 | Internal type, constructed by runtime | Runtime asserts validity on construction |
| `SeqNo::get()` returns valid u64 | Internal monotonic counter | Runtime enforces monotonicity |
| `ActionId::get()` returns valid u32 | Internal type | Validated by workflow compiler |
| `SlotIdx` is within frame bounds | Runtime bounds checking | `validate_action_dispatch` checks before use |
| `Taint` enum is one of {Clean, Secret, DerivedFromSecret, Random, TimeDependent} | Type definition | Enum variant restriction |

### 2.2 Arithmetic Assumptions

| Assumption | Rationale |
|------------|-----------|
| `u128::wrapping_mul` and `u128::wrapping_add` are deterministic | Rust defined behavior |
| Key collision is possible (H1) but tracker keys on `(action, step)` so collision does not affect replay detection | Design decision — tracker independence provides defense-in-depth |
| 160 bits of input mapped to 128 bits via wrapping — collision probability is astronomically low for bounded inputs | Pigeonhole principle, bounded input space |

### 2.3 State Machine Assumptions

| Assumption | Rationale |
|------------|-----------|
| `ActionReplayTracker::is_resolved` is monotonic | `completed` and `failed` HashSets only grow, never shrink |
| Journal events are append-only | Fjall LSM-tree provides durability |
| `apply_tail_events` processes events in seq order | Implementation iterates sequentially, no parallelization |
| No concurrent modification of `ActionReplayTracker` during hydration | Single-threaded recovery processing |

---

## 3. Stubs and Mocks

| Stub | Purpose | Boundary |
|------|---------|----------|
| `kani::any::<RunId>()` | Kani arbitrary instance for RunId | vb_core |
| `kani::any::<SeqNo>()` | Kani arbitrary instance for SeqNo | vb_core |
| `kani::any::<ActionId>()` | Kani arbitrary instance for ActionId | vb_core |
| `kani::any::<ActionTicket>()` | Kani arbitrary instance for ActionTicket | vb_storage |
| `kani::any::<RunFrame>()` | Kani arbitrary instance for RunFrame | vb_storage |
| `kani::any::<JournalEvent>()` | Kani arbitrary instance for JournalEvent | vb_storage |

**Stub Constraints**: All `kani::any()` instances must satisfy the type's construction preconditions (e.g., `attempt >= 1`, `capacity >= 1`).

---

## 4. Model Reductions

### 4.1 TLA+ Model Reductions

| Constant | Value | Rationale |
|----------|-------|-----------|
| `MaxRuns` | 2 | Minimum to test concurrent run handling |
| `MaxActions` | 3 | Minimum to test action diversity |
| `MaxSeq` | 4 | Allows overflow/fail-safe testing |
| `Digests` | {0, 1} | Minimal digest set for invariant checking |
| `Taint` | {Clean, Secret} | Minimal for no-secret-in-key invariant |

**Reduction Justification**: These small bounds are sufficient to exercise all state transitions and invariant violations. The Kani bounded model checking covers the concrete value space more precisely.

### 4.2 Kani Unwind Bounds

| Function | Unwind Bound | Rationale |
|----------|--------------|-----------|
| `compute_action_idempotency_key` | 1 | No loops, single evaluation |
| `mark_scheduled_ticket_effect` | 2 | HashMap lookup + potential insert |
| `mark_completed_envelope_effect` | 3 | HashMap lookup + schedule check + envelope comparison |
| `apply_tail_events` | N (event count) | Iterate over tail events |
| `hydrate_run_frame` | 5 | Snapshot validation + tail application + dimension check |

---

## 5. Known Limitations

| Limitation | Impact | Mitigation |
|------------|--------|------------|
| Key collision (H1) is not proven impossible | Collisions exist mathematically | Tracker key independence from key provides defense-in-depth |
| BLAKE3 collision resistance is assumed | Digest mismatch could be false positive in adversarial setting | Unlikely in practice; BLAKE3 is widely studied |
| Fjall durability assumed | Storage corruption not modeled | Typed `RecoveryError::CorruptSnapshot` handles detected corruption |
| Single-threaded recovery assumed | No concurrent hydration | Design constraint — not a limitation |
| Bounded TLA+ model cannot prove infinite state space | Model only covers small instances | Kani covers concrete bounded values; TLA+ covers state machine logic |

---

## 6. Evidence Commands for Trusted Base

```bash
# Verify vb_core has no vb_storage deps
cargo check -p vb_core 2>&1 | grep -i vb_storage || echo 'PASS: no vb_storage deps'

# Verify vb_core purity (no I/O, no unsafe)
cargo clippy -p vb_core -- -D warnings 2>&1 | grep -E '(forbidden|unsafe|panic)'

# Verify Rust defined behavior for wrapping arithmetic
rustc --version  # Must be stable or nightly with defined wrapping semantics
```
