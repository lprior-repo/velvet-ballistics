# ARCHITECTURAL DRIFT REPORT
## Target: `/home/lewis/src/velvet-ballistics/crates/vb_runtime/src/shard/helpers.rs`

---

## EXECUTIVE SUMMARY

**File:** `helpers.rs`  
**Total Lines:** 2,492  
**Violation:** CATASTROPHIC - exceeds 300-line limit by **830%**

---

## VIOLATION BREAKDOWN

### 1. LINE COUNT VIOLATION

| Section | Lines | Description |
|---------|-------|-------------|
| Production helpers | 1-366 | Actual helper functions |
| Tests + fixtures | 368-2492 | Test code and workflow factories |
| **TOTAL** | **2492** | **830% of 300-line limit** |

The test/fixture code (≈2,100 lines) is **67%** of the file.

---

### 2. DDD SMELL: "HELPERS" MODULE NAME

The name `helpers.rs` is a **DDD anti-pattern**. It indicates:

- **Missing domain concepts** - functionality is grouped by utility, not by domain
- **Low cohesion** - unrelated domain behaviors thrown together
- **Primitive obsession** - raw types used where domain types should exist

---

## HELPER FUNCTION INVENTORY & DOMAIN CATEGORIZATION

### PRODUCTION FUNCTIONS (Lines 1-366)

| # | Function | Lines | Domain | Hidden Concept |
|---|----------|-------|--------|----------------|
| 1 | `seed_input_slots` | 16-26 | **SlotHydration** | Input seeding for deterministic execution |
| 2 | `validate_action_completion` | 29-44 | **ActionCompletion** | Action ticket validation |
| 3 | `action_input_slot` | 47-58 | **StepNavigation** | Input slot extraction |
| 4 | `action_output_slot` | 61-70 | **StepNavigation** | Output slot extraction |
| 5 | `validate_ticket_attempt` | 72-94 | **AttemptValidation** | Internal attempt bounds checking |
| 6 | `normalize_scheduled_ticket` | 97-114 | **AttemptTracking** | Ticket normalization |
| 7 | `advance_after_action_completion` | 117-134 | **PCAdvancement** | Program counter progression |
| 8 | `timer_registration_required` | 137-147 | **TimerScheduling** | Timer requirement detection |
| 9 | `advance_after_timer_fire` | 150-181 | **TimerFireHandling** | Post-timer state advancement |
| 10 | `new_action_attempts` | 184-186 | **AttemptTrackingFactory** | Attempt counter allocation |
| 11 | `record_scheduled_attempt` | 189-198 | **AttemptTracking** | Attempt recording |
| 12 | `retry_metadata_exists` | 211-222 | **RetryPolicyDiscovery** | Retry check detection |
| 13 | `retry_policy_after_action` | 225-271 | **RetryPolicyExtraction** | Policy reading from slot |
| 14 | `record_retry_attempt` | 274-294 | **RetryStateManagement** | Retry state mutation |
| 15 | `find_error_handler_for_failure` | 297-325 | **ErrorHandlerDiscovery** | Error route lookup |
| 16 | `error_handler_on_node` | 327-341 | **ErrorHandlerDiscovery** | Internal error handler check |
| 17 | `result_slot_for_finished_run` | 344-352 | **RunCompletion** | Result slot extraction |
| 18 | `snapshot_from_state` | 355-366 | **StateSnapshotting** | Inspect snapshot creation |

---

## DOMAIN CONCEPT EXTRACTION

### Domain 1: ATTEMPT TRACKING (`AttemptTracking`)

**Hidden in:** `new_action_attempts`, `record_scheduled_attempt`, `normalize_scheduled_ticket`, `validate_ticket_attempt`

**Problems:**
```rust
// Raw u16 everywhere - PRIMITIVE OBSESSION
pub fn new_action_attempts(step_count: u16) -> Box<[u16]> {
    vec![0; usize::from(step_count)].into_boxed_slice()
}
```

**Missing type:**
```rust
/// Attempt counter for a single step
pub struct AttemptCounter(Box<[u16]>);

impl AttemptCounter {
    pub fn new(step_count: u16) -> Self;
    pub fn get(&self, step: StepIdx) -> Option<u16>;
    pub fn set_max(&mut self, step: StepIdx, attempt: u16);
    pub fn normalize(&self, ticket_attempt: u16, capacity: u16) -> RuntimeResult<u16>;
}
```

---

### Domain 2: RETRY POLICY (`RetryPolicy`)

**Hidden in:** `retry_metadata_exists`, `retry_policy_after_action`, `record_retry_attempt`, `validate_retry_attempt`

**Problems:**
```rust
// Unvalidated I64 from slot - PRIMITIVE OBSESSION
let SlotValue::I64(max_attempts) = *state.frame.read_slot(policy_slot)... else { ... }
let max_attempts = u16::try_from(max_attempts)...;
```

**Missing type:**
```rust
/// Retry policy extracted from workflow slots
pub struct RetryPolicy {
    pub max_attempts: u16,
    pub base_delay_ms: u64,
    pub exponential_backoff: bool,
}

impl RetryPolicy {
    pub fn from_slot(state: &RunState, step: StepIdx) -> RuntimeResult<Self>;
    pub fn can_retry(&self, current_attempt: u16) -> bool;
    pub fn next_attempt(&self, current: u16) -> RuntimeResult<u16>;
}
```

---

### Domain 3: ACTION COMPLETION VALIDATION (`ActionCompletion`)

**Hidden in:** `validate_action_completion`, `action_input_slot`, `action_output_slot`, `advance_after_action_completion`

**Missing type:**
```rust
/// Validated action completion
pub struct ActionCompletion {
    pub step: StepIdx,
    pub action: ActionId,
    pub attempt: u16,
}

impl ActionCompletion {
    pub fn validate(state: &RunState, ticket: ActionTicket) -> RuntimeResult<()>;
    pub fn input_slot(state: &RunState, step: StepIdx) -> RuntimeResult<SlotIdx>;
    pub fn output_slot(state: &RunState, step: StepIdx) -> RuntimeResult<SlotIdx>;
    pub fn advance(state: &mut RunState, step: StepIdx) -> RuntimeResult<()>;
}
```

---

### Domain 4: TIMER MANAGEMENT (`TimerManagement`)

**Hidden in:** `timer_registration_required`, `advance_after_timer_fire`

**Missing type:**
```rust
/// Timer management for wait/ask steps
pub struct TimerManagement;

impl TimerManagement {
    pub fn must_register(state: &RunState, step: StepIdx) -> bool;
    pub fn advance_after_fire(state: &mut RunState, timer: PendingTimer) -> RuntimeResult<()>;
}
```

---

### Domain 5: ERROR HANDLING (`ErrorHandling`)

**Hidden in:** `find_error_handler_for_failure`, `error_handler_on_node`

**Missing type:**
```rust
/// Error handler routing
pub struct ErrorHandler;

impl ErrorHandler {
    pub fn find_for_failure(
        workflow: &CompiledWorkflow,
        failed: StepIdx,
    ) -> Option<(StepIdx, Option<SlotIdx>)>;
}
```

---

### Domain 6: STATE SNAPSHOTTING (`StateSnapshotting`)

**Hidden in:** `snapshot_from_state`, `result_slot_for_finished_run`

**Missing type:**
```rust
/// Run state inspection
pub struct RunInspector;

impl RunInspector {
    pub fn snapshot(run: RunId, correlation: u64, state: &RunState) -> InspectSnapshot;
    pub fn result_slot(state: &RunState) -> Option<SlotIdx>;
}
```

---

## PRIMITIVE OBSESSION VIOLATIONS

| Location | Raw Type | Missing Domain Type |
|----------|----------|---------------------|
| `ticket.attempt == 0` checks | `u16` | `AttemptNumber` |
| `ticket.capacity == 0` checks | `u16` | `AttemptCapacity` |
| `u16::try_from(max_attempts)` | `i64→u16` | `MaxAttempts` |
| `*attempt.checked_add(1)` | `u16` | `AttemptCounter` |
| `state.action_attempts.get(ticket.step.as_usize())` | `usize` index | `StepAttempts` |
| `SlotValue::I64` pattern matching | `i64` | `PolicyValue` |

---

## SCOTTL WLASCHIN DDD VIOLATIONS

### 1. **Primitive Obsession**
- Raw `u16` for attempts, capacities, indices
- Raw `i64` for policy values extracted from slots
- No `newtype` wrappers for domain semantics

### 2. **Feature Envy** (multiple instances)
```rust
// This function envies RunState's internals
fn validate_ticket_attempt(state: &crate::shard::types::RunState, ticket: ActionTicket) {
    let current = state.action_attempts.get(ticket.step.as_usize())...  // Feature envy
}
```

### 3. **Data Class**
- `RunState` is passed to almost every helper, which manipulate its internals
- Should have methods ON `RunState`, not free functions that reach into it

### 4. **Missing Domain Types**
- No `AttemptCounter` - raw `Box<[u16]>`
- No `RetryPolicy` - raw slot reads with type coercion
- No `ActionCompletion` - scattered validation logic

---

## REFACTORING prescription

### Phase 1: Extract Domain Modules

```
vb_runtime/src/shard/
├── attempt_tracking.rs     # NEW: AttemptCounter, new_action_attempts, record_scheduled_attempt
├── retry_policy.rs         # NEW: RetryPolicy, retry_metadata_exists, retry_policy_after_action
├── action_completion.rs    # NEW: validate_action_completion, action_input_slot, etc.
├── timer_management.rs     # NEW: timer_registration_required, advance_after_timer_fire
├── error_handling.rs       # NEW: find_error_handler_for_failure
├── state_inspection.rs     # NEW: snapshot_from_state, result_slot_for_finished_run
└── helpers.rs              # RETIRE - delete after extraction
```

### Phase 2: Create Value Objects

```rust
// vb_core/attempt.rs - NEW crate or module
pub struct AttemptNumber(u16);
pub struct AttemptCapacity(u16);
pub struct AttemptCounter(Box<[u16]>);

// vb_core/retry.rs
pub struct RetryPolicy {
    pub max_attempts: AttemptCapacity,
    pub base_delay_ms: u64,
    pub exponential_backoff: bool,
}
```

### Phase 3: Move Tests to Integration Layer

Tests currently in `helpers.rs` should move to:
```
crates/workspace_tests/
├── shard/
│   ├── attempt_tracking_tests.rs
│   ├── retry_policy_tests.rs
│   └── ...
```

---

## RECOMMENDED ACTION

**IMMEDIATE REFACTORING REQUIRED**

The file must be broken into 6+ domain modules. The 300-line hard limit must be respected.

**Priority:**
1. **P0** - Extract `attempt_tracking.rs` (most used, highest coupling)
2. **P0** - Extract `retry_policy.rs` (complex slot-reading logic)
3. **P1** - Extract `action_completion.rs`
4. **P1** - Extract `timer_management.rs` + `error_handling.rs`
5. **P2** - Move all tests to `workspace_tests/`
6. **P3** - Delete `helpers.rs`

---

## EVIDENCE

- File path: `/home/lewis/src/velvet-ballistics/crates/vb_runtime/src/shard/helpers.rs`
- Total lines: 2,492
- Production code: ~350 lines (exceeds limit alone)
- Test code: ~2,100 lines
- Helper functions: 18 public + 2 private
- Domain concepts hidden: 6+
- Primitive obsession instances: 8+
