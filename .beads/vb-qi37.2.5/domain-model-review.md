# Domain Model Review — vb-qi37.2.5

## Bead Identity
- **Bead**: vb-qi37.2.5
- **Title**: quality: Boundedness adversarial tests
- **State**: 3 (Contract and type model)
- **Focus**: Type-model analysis of boundedness domain types

---

## Type Model Analysis

### StepBudget — Bounded Counter

```rust
pub struct StepBudget {
    remaining: u64,  // private; only mutated via try_take
}
```

**Observations**:
- `remaining` field is private — no direct external mutation possible
- `new(value)` clamps to `MAX_STEP_BUDGET` — caller cannot construct an invalid budget
- `try_take` uses `saturating_sub` — never panics on subtraction
- `MAX: Self` constant provides a public ceiling reference
- Invariant `remaining <= MAX_STEP_BUDGET` is enforced by construction

**Scott Wlaschin Assessment**:
- ✅ No `bool` flags for domain decisions
- ✅ No `Option` as state machine
- ✅ Illegal states unrepresentable (cannot construct `remaining > MAX_STEP_BUDGET`)
- ✅ `try_take` returns `Result<bool, EngineError>` — exhaustive, railway-oriented
- ✅ No primitive obsession — `StepBudget` is a named domain type wrapping `u64`

**Type Integrity Gate**: PASS — no primitive obsession, no boolean control flags.

---

### ValueStore — Capped Arena

```rust
pub struct ValueStore {
    symbols: Vec<Box<str>>,
    lists: Vec<Box<[SlotValue]>>,
    objects: Vec<Box<[ObjectField]>>,
    blobs: Vec<Bytes>,
    max_arena_entries: u64,  // private cap
}
```

**Observations**:
- `max_arena_entries` is private; only set at construction time via `with_max_slots`
- All insert paths (`insert_list`, `insert_object`, etc.) go through `check_arena_cap()`
- `check_arena_cap` computes `total_arena_count` and compares against `max_arena_entries`
- `CoreError::BudgetExceeded` returned on cap violation — typed error, not panic
- `total_arena_count()` is a public observer for the current count

**Scott Wlaschin Assessment**:
- ✅ Cap is set once at construction and never changes — immutability enforced
- ✅ All insert paths go through the cap check — single validation boundary
- ✅ Error taxonomy is explicit: `CoreError::BudgetExceeded { budget, limit }`
- ⚠️ `Vec` internals expose mutability — `symbols`, `lists`, etc. are `pub` fields
  - **Risk**: Downstream code could bypass the arena cap by direct vector mutation
  - **Mitigation**: The `ValueStore` struct itself is not `pub` (it's `pub(crate)` or private);
    construction is only via `ValueStore::new()` and `with_max_slots()`

**Type Integrity Gate**: PASS with note — cap bypass risk is mitigated by crate-level privacy.

---

### WholeWorkflowBudget — Computed Aggregate

```rust
pub struct WholeWorkflowBudget {
    pub max_total_steps: u64,
    pub max_total_slots: u64,
    pub max_fanout: u16,
    pub max_nesting_depth: u16,
    pub max_steps_executable: u32,
    pub max_action_tickets: u32,
    // ... 15 fields total
}
```

**Observations**:
- All fields are public — this is a **value object** (compute-once, then read)
- Fields use appropriate integer widths: `u64` for counts, `u16` for bounded dimensions
- `compute()` is the only constructor; returns `Result<Self, WorkflowError>`
- No `Option` fields — all dimensions always present
- `PartialEq, Eq, Clone, Debug` derived — proper value semantics

**Scott Wlaschin Assessment**:
- ✅ Value object pattern — computed once, immutable after
- ✅ Domain fields use semantic names, not primitives
- ✅ `u16` for bounded dimensions (fanout, nesting depth) — correct width for limits
- ✅ No `bool` state flags
- ⚠️ `max_total_steps: u64` — could theoretically exceed `MAX_STEPS_PER_WORKFLOW` (65_535)
  before `compute` returns `WorkflowError::StepCountOverflow`; the error is returned rather
  than clamping — **this is correct fail-closed behavior**

**Type Integrity Gate**: PASS — value object with explicit error on invalid construction.

---

### BoundednessPolicy — Validation Policy

```rust
pub struct BoundednessPolicy {
    pub max_total_steps: u64,
    pub max_fanout: u16,
    pub max_nesting_depth: u16,
    // ...
}
```

**Observations**:
- Companion to `WholeWorkflowBudget` — validates computed budget against policy
- `validate()` returns `Result<(), BudgetError>` — fail-closed on policy violation
- `DEFAULT` constant provides safe conservative defaults

**Type Integrity Gate**: PASS.

---

### EngineSignal — State Enum

```rust
pub enum EngineSignal {
    Continue,
    StepBudgetExhausted,
    Yielded { ... },
    Finished { ... },
    Error { ... },
}
```

**Observations**:
- Enum covers all terminal and non-terminal states
- `run_until_blocked` matches exhaustively on `EngineSignal`
- No boolean control flags — explicit variant per behavior

**Type Integrity Gate**: PASS — explicit state transitions, exhaustive matching enforced.

---

## Transition Map

| Function | Input Domain | Output Domain | Error Domain |
|----------|-------------|--------------|--------------|
| `StepBudget::new` | `u64` (any) | `StepBudget { remaining: clamp(u64, MAX_STEP_BUDGET) }` | — (total, no error) |
| `StepBudget::try_take` | `&mut self` | `Ok(true)` / `Ok(false)` | `EngineError::StepCounterOverflow` |
| `ValueStore::with_max_slots` | `u16` cap | `ValueStore { max_arena_entries: u64 }` | — |
| `ValueStore::insert_*` | `ValueStore + item` | `Ok(id)` | `CoreError::BudgetExceeded` |
| `WholeWorkflowBudget::compute` | `nodes + entry + contract` | `WholeWorkflowBudget` | `WorkflowError` variants |
| `BoundednessPolicy::validate` | `&BoundednessPolicy + &WholeWorkflowBudget` | `Ok(())` | `BudgetError` |
| `run_until_blocked` | `Workflow + RunFrame + StepBudget + ValueStore` | `EngineSignal` | `EngineError` |

---

## Type Repair Assessment

**No type repairs required** for this bead's scope. The existing type model:

1. Makes illegal `StepBudget` states unrepresentable (private `remaining`, clamped constructor)
2. Uses explicit error enums instead of panics for all boundedness failures
3. Applies `Result` railway-oriented composition throughout
4. Has no `bool` state flags or `Option`-as-state-machine patterns in the boundedness core

The existing Verus specs in `verification/verus/resource_budget.rs` and `verification/verus/step_budget.rs`
already cover the critical invariants.

---

## Risk Tags Validated

| Risk Tag | Type Evidence |
|----------|--------------|
| `boundedness` | `StepBudget` capped to `MAX_STEP_BUDGET`, `ValueStore` capped to `max_arena_entries`, `WholeWorkflowBudget` returns error on overflow |
| `performance` | `run_until_blocked` bounded by `budget.remaining` iterations |
| `user-visible-behavior` | All failures are typed `Result` variants returned to caller |
| `persistence` | `ValueStore` arena growth bounded by `max_arena_entries` cap |
| `public-api` | All public signatures use domain types, not primitives |
