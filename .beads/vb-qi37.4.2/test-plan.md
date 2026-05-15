# Test Plan: vb-qi37.4.2 — Phase 3 Hot-Path Execution Core (test-planner state 7)

## Summary

- Behaviors identified: 38
- Trophy allocation: 12 unit / 18 integration / 4 e2e / 4 static-analysis
- Proptest invariants: 9
- Fuzz targets: 2 (1 missing, 1 existing)
- Kani harnesses: 14 missing (formal waivers filed); compensating coverage via integration tests

## Phase 1: Evidence Packets

### Already-PASS nextest Tests (no further action required)

| Filter | Obligation | Evidence |
|--------|------------|----------|
| `step_state_invalid` | VB-CORE-STATE-003 | nextest: 1 passed (StepState invalid transitions rejected) |
| `resource_policy` | VB-CORE-RESOURCE-004-PROP | nextest: 1 passed (WholeWorkflowBudget within BoundednessPolicy::DEFAULT) |
| `ast_bytecode_equiv` | VB-EXPR-001 | nextest: 1 passed (AST/bytecode equivalence differential) |
| `serde_json_` | VB-UI-MODEL-envelope-002 | nextest: 1 passed (envelope JSON roundtrip) |
| `envelope_` | VB-UI-MODEL-envelope-001 | nextest: 18 passed (roundtrip, redaction, schema) |

### PASS Verification Lanes (compensating evidence for DEFERRED_GLOBAL)

| Lane | Obligations | Status |
|------|-------------|--------|
| Verus L4 | 19 (taint_lattice, signals, step_state_machine, step_budget, run_frame_invariant, resource_budget) | All PASS |
| TLA+ L3 | 13 (LifecycleJournal, RetryFSM, CapabilityLifecycle, ConcurrencyControl) | All PASS |
| Proptest/Differential L1 | 5 (resource_policy, ast_bytecode_equiv, idempotency_key_well_formed, envelope_, serde_json_) | All PASS |
| Fuzz L2 | 2 (expr_eval 500k, decode_record 1M) | PASS |
| Loom L3 | 1 (concurrency bounded_queue) | PASS |
| Static-scan L0 | 2 (clippy: no unsafe, no panic) | PASS |

### DEFERRED_GLOBAL Obligations (19 total)

All have formal waivers filed in `.beads/vb-qi37.4.2/formal-waivers.jsonl`.

**14 missing Kani harnesses** (scope: missing-artifact): compensating evidence = corresponding Verus proofs (19 PASS) + fuzz/proptest layers.

**1 missing fuzz target** (VB-IPC-DECODE-FUZZ, scope: missing-artifact): `ipc_decode` target absent; compensating = decode_record fuzz (1M runs).

**1 missing xtask** (VB-CORE-IDX-002, scope: missing-tool): forbidden-scan unavailable; compensating = clippy clean.

**2 downstream gauntlet** (GATE-001/002, scope: downstream-blocked): will self-resolve when upstream clears.

---

## 2. Gaps: BDD Scenarios Not Yet Written

The following contract clauses have Verus/proptest/fuzz compensating evidence but lack explicit BDD scenario tests in the integration/unit layers. These are the gaps the test-writer must close.

### Gap A: FiniteF64 Precondition Coverage (PRE-003)

**Contract**: `FiniteF64::new(value)` requires `value.is_finite()` — rejects NaN, ±infinity.

**Already covered**: proptest `finite_f64_property` (PASS). **Gap**: no explicit BDD scenario naming the exact error variant.

```rust
// GIVEN: a non-finite f64 value (NaN or infinity)
// WHEN:  FiniteF64::new(value) is called
// THEN:  CoreError::NonFiniteNumber is returned exactly
fn finite_f64_rejects_nan_when_constructed() -> CoreResult<()>
fn finite_f64_rejects_positive_infinity_when_constructed() -> CoreResult<()>
fn finite_f64_rejects_negative_infinity_when_constructed() -> CoreResult<()>
fn finite_f64_accepts_valid_finite_value() -> CoreResult<()>
```

### Gap B: RunFrame Construction Preconditions (PRE-001, POST-001)

**Contract**: `RunFrame::new` requires `step_count > 0` and `first_step.as_usize() < step_count`.

```rust
// GIVEN: step_count = 0 and valid run_id, first_step, slot_count
// WHEN:  RunFrame::new is called
// THEN:  CoreError::InvalidCompiledWorkflow { reason: "step_count_zero" }
fn run_frame_new_rejects_zero_step_count()

// GIVEN: first_step >= step_count with step_count > 0
// WHEN:  RunFrame::new is called
// THEN:  CoreError::InvalidProgramCounter { step } (first_step PC invalid)
fn run_frame_new_rejects_first_step_out_of_bounds()

// GIVEN: valid dimensions (step_count > 0, first_step < step_count)
// WHEN:  RunFrame::new succeeds
// THEN:  frame.states.len() == step_count, frame.slots.len() == slot_count,
//         frame.taint.len() == slot_count, all states = Pending, all taint = Clean
fn run_frame_new_returns_correct_dimensions()
fn run_frame_new_initializes_all_states_to_pending()
fn run_frame_new_initializes_all_taint_to_clean()
```

### Gap C: RunFrame Reinitialize Preconditions (PRE-001 variant)

**Contract**: `reinitialize` checks same preconditions as `new` and additionally rejects dimension changes.

```rust
// GIVEN: a valid RunFrame
// WHEN:  reinitialize is called with step_count = 0
// THEN:  CoreError::InvalidCompiledWorkflow { reason: "step_count_zero" }
fn run_frame_reinitialize_rejects_zero_step_count()

// GIVEN: a valid RunFrame
// WHEN:  reinitialize is called with different step_count than construction
// THEN:  CoreError::InvalidCompiledWorkflow { reason: "frame_dimension_mismatch" }
fn run_frame_reinitialize_rejects_dimension_mismatch_step_count()
fn run_frame_reinitialize_rejects_dimension_mismatch_slot_count()

// GIVEN: a valid RunFrame
// WHEN:  reinitialize is called with valid dimensions
// THEN:  states reset to Pending, taint reset to Clean, dimensions unchanged
fn run_frame_reinitialize_preserves_dimensions()
fn run_frame_reinitialize_resets_states_to_pending()
```

### Gap D: WholeWorkflowBudget::compute Precondition (PRE-002)

**Contract**: `entry.as_usize() < nodes.len()`.

```rust
// GIVEN: entry >= nodes.len() with valid nodes slice
// WHEN:  WholeWorkflowBudget::compute is called
// THEN:  WorkflowError::EntryOutOfBounds { entry }
fn budget_compute_rejects_entry_out_of_bounds()

// GIVEN: valid entry within bounds
// WHEN:  WholeWorkflowBudget::compute succeeds
// THEN:  every field <= BoundednessPolicy::DEFAULT corresponding limit
fn whole_workflow_budget_within_policy_limits()
```

### Gap E: AggregateResourceBudget::try_take Precondition (PRE-006)

**Contract**: `amount <= budget.remaining`.

```rust
// GIVEN: budget with known remaining, amount > remaining
// WHEN:  try_take(amount) is called
// THEN:  Err(AggregateResourceBudgetExhausted) or Err(StepBudgetExhausted)
fn budget_try_take_rejects_amount_exceeding_remaining()
fn budget_try_take_accepts_exact_remaining()
fn budget_try_take_accepts_zero()
```

### Gap F: IPC Decoder Rejects-Before-Allocation (INV-011, POST-007)

**Contract**: IPC decoder returns `Err` before allocating any buffer when `header_len < 60` or `payload_len > MAX_PAYLOAD`.

```rust
// GIVEN: a byte slice with header_len < 60
// WHEN:  IpcFrameDecoder::decode_header is called
// THEN:  IpcError::HeaderTooShort is returned, no buffer allocated
fn ipc_decoder_rejects_header_too_short_before_allocation()

// GIVEN: a byte slice with payload_len > MAX_PAYLOAD
// WHEN:  IpcFrameDecoder::decode_frame is called
// THEN:  IpcError::PayloadTooLarge is returned, no buffer allocated
fn ipc_decoder_rejects_payload_too_large_before_allocation()

// GIVEN: valid header_len >= 60 and valid magic
// WHEN:  decode_frame succeeds
// THEN:  Frame is returned with correct slot values
fn ipc_decoder_accepts_valid_frame()
```

### Gap G: Record Decoder Rejects-Before-Allocation (INV-012, POST-008)

**Contract**: All five validations (magic, schema, kind, payload_len, CRC) occur before any heap allocation.

```rust
// GIVEN: record bytes with invalid magic
// WHEN:  RecordDecoder::decode is called
// THEN:  StorageError::RecordMagicInvalid, no heap allocation
fn record_decoder_rejects_magic_before_allocation()

// GIVEN: record bytes with invalid schema
// WHEN:  RecordDecoder::decode is called
// THEN:  StorageError::RecordSchemaInvalid, no heap allocation
fn record_decoder_rejects_schema_before_allocation()

// GIVEN: record bytes with invalid kind
// WHEN:  RecordDecoder::decode is called
// THEN:  StorageError::RecordKindInvalid, no heap allocation
fn record_decoder_rejects_kind_before_allocation()

// GIVEN: record bytes with invalid payload_len
// WHEN:  RecordDecoder::decode is called
// THEN:  StorageError::RecordPayloadLenInvalid, no heap allocation
fn record_decoder_rejects_payload_len_before_allocation()

// GIVEN: record bytes with invalid CRC
// WHEN:  RecordDecoder::decode is called
// THEN:  StorageError::RecordCrcInvalid, no heap allocation
fn record_decoder_rejects_crc_before_allocation()

// GIVEN: valid record bytes (all validations pass)
// WHEN:  RecordDecoder::decode succeeds
// THEN:  Record is returned
fn record_decoder_accepts_valid_record()
```

### Gap H: Journal Sequence Monotonicity (INV-009, POST-009)

**Contract**: Journal entry sequence numbers are strictly monotonically increasing per shard.

```rust
// GIVEN: two journal entries for the same shard with seqN >= seqN+1
// WHEN:  replay is attempted
// THEN:  replay detects violation and returns error
fn journal_sequence_rejects_non_monotonic_entries()

// GIVEN: valid journal entries for a shard
// WHEN:  replay succeeds
// THEN:  entries are applied in strictly increasing seq order
fn journal_replay_applies_entries_in_seq_order()
```

### Gap I: Idempotency Key Well-Formedness (INV-014)

**Contract**: Idempotency keys must satisfy `idempotency_key_well_formed`.

```rust
// GIVEN: a normalized, non-empty, valid-alphabet key within size bounds
// WHEN:  idempotency_key_well_formed is checked
// THEN:  true is returned
fn idempotency_key_accepts_valid_normalized_key()

// GIVEN: an empty key
// WHEN:  idempotency_key_well_formed is checked
// THEN:  false is returned
fn idempotency_key_rejects_empty_key()

// GIVEN: a key exceeding maximum size
// WHEN:  idempotency_key_well_formed is checked
// THEN:  false is returned
fn idempotency_key_rejects_oversized_key()

// GIVEN: a key containing invalid alphabet characters
// WHEN:  idempotency_key_well_formed is checked
// THEN:  false is returned
fn idempotency_key_rejects_invalid_alphabet()
```

### Gap J: Concurrency Invariants (INV-015)

**Contract**: Each shard has a single owner; no cross-shard mutable aliasing.

```rust
// GIVEN: two frames on the same shard
// WHEN:  concurrent access is attempted
// THEN:  only one owner at a time; other waits or gets error
fn shard_enforces_single_owner()

// GIVEN: frames on different shards
// WHEN:  concurrent access is attempted
// THEN:  no aliasing; both can proceed in parallel
fn different_shards_allow_parallel_access()

// GIVEN: frame pool with bounded capacity
// WHEN:  pool is at capacity and new frame is requested
// THEN:  pool does not grow unbounded; old frames recycled or error returned
fn frame_pool_respects_bounded_capacity()
```

### Gap K: StepBudget try_take Postconditions (POST-003)

**Contract**: `try_take` returns `Ok(remaining)` where `remaining == old - amount`, or `Err(StepBudgetExhausted)`.

```rust
// GIVEN: StepBudget with remaining >= amount
// WHEN:  try_take(amount) succeeds
// THEN:  Ok(remaining) where remaining == old_remaining - amount
fn step_budget_try_take_returns_correct_remaining()

// GIVEN: StepBudget with remaining < amount
// WHEN:  try_take(amount) is called
// THEN:  Err(StepBudgetExhausted), remaining unchanged
fn step_budget_try_take_returns_exhausted_error()

// GIVEN: StepBudget at zero remaining
// WHEN:  try_take(0) is called
// THEN:  Ok(0)
fn step_budget_try_take_accepts_zero_from_exhausted()

// GIVEN: StepBudget remaining is monotonically non-increasing
// WHEN:  multiple try_take calls are made
// THEN:  remaining never increases
fn step_budget_remaining_is_monotonic()
```

### Gap L: EngineSignal Finished Canonical Form (INV-010, POST-004)

**Contract**: `EngineSignal::Finished` always carries `(SlotValue, Taint)`, never legacy `Finished(SlotValue)`.

```rust
// GIVEN: a step reaches finished state
// WHEN:  EngineSignal is emitted
// THEN:  signal is Finished(SlotValue, Taint) — Taint field always present
fn finish_signal_always_carries_taint()

// GIVEN: a slot value with taint = Secret
// WHEN:  Finished is emitted
// THEN:  Taint field is Secret
fn finish_signal_propagates_secret_taint()

// GIVEN: a slot value with taint = Clean
// WHEN:  Finished is emitted
// THEN:  Taint field is Clean
fn finish_signal_propagates_clean_taint()
```

### Gap M: Resource Budget Saturating Arithmetic (POST-010)

**Contract**: Sequential composition = saturating add at policy max; branch = max; loop = saturating multiply.

```rust
// GIVEN: two resource budgets, sequential composition
// WHEN:  budgets are combined sequentially
// THEN:  result = min(sum, BoundednessPolicy::DEFAULT), no overflow/panic
fn resource_budget_sequential_sum_is_bounded()

// GIVEN: two resource budgets, branch composition
// WHEN:  budgets are combined via branch
// THEN:  result = max(a, b)
fn resource_budget_branch_takes_max()

// GIVEN: resource budget, loop composition (iteration count * body budget)
// WHEN:  loop budget is computed
// THEN:  result = min(product, BoundednessPolicy::DEFAULT), no overflow/panic
fn resource_budget_loop_multiply_is_bounded()
fn resource_budget_loop_at_policy_max_saturates()
```

---

## 3. Integration Tests (Real Dependencies)

These tests compose multiple components with real implementations — no mocks.

### IT-1: RunFrame Lifecycle with Real Engine

```rust
// GIVEN: a valid compiled workflow with N steps and M slots
// WHEN:  RunFrame::new succeeds, then reinitialize is called
// THEN:  dimensions preserved, states reset, taint reset
fn run_frame_lifecycle_with_engine()
```

### IT-2: IPC Frame → Record Decode Pipeline

```rust
// GIVEN: a valid IPC frame bytes sequence
// WHEN:  frame is decoded then record is encoded from the frame data
// THEN:  roundtrip produces original data
fn ipc_frame_to_record_roundtrip()

// GIVEN: invalid IPC frame bytes (header too short)
// WHEN:  decode is attempted
// THEN:  IpcError::HeaderTooShort returned before any allocation
fn ipc_frame_decode_rejects_before_allocation_integration()
```

### IT-3: Journal Replay with Real Storage

```rust
// GIVEN: a journal with entries for a single shard in increasing seq order
// WHEN:  replay is executed
// THEN:  actions are dispatched in journal order; final state matches expected
fn journal_replay_preserves_order()

// GIVEN: a journal with out-of-order sequence numbers
// WHEN:  replay is attempted
// THEN:  error is returned before any action dispatch
fn journal_replay_rejects_out_of_order()
```

### IT-4: Taint Propagation Through Expression Evaluation

```rust
// GIVEN: an expression tree with mixed taint operands
// WHEN:  expression is evaluated
// THEN:  result taint = join of all operand taints per taint lattice rules
fn taint_propagates_through_expression_evaluation()
fn taint_join_is_associative_in_evaluation()
fn taint_join_is_commutative_in_evaluation()
```

### IT-5: Budget Exhaustion Through Workflow

```rust
// GIVEN: a workflow where step budget is exhausted mid-execution
// WHEN:  engine processes steps until budget depleted
// THEN:  EngineSignal::StepBudgetExhausted is emitted, no underflow
fn engine_emits_budget_exhausted_signal()
fn step_budget_exhaustion_is_deterministic()
```

---

## 4. Proptest Invariants

### PI-1: Taint Lattice Laws

```rust
// Property: join is associative for all Taint combinations
proptest! {
    fn taint_join_associative(a: Taint, b: Taint, c: Taint) {
        assert_eq!(join(join(a, b), c), join(a, join(b, c)));
    }
}

// Property: join is commutative
proptest! {
    fn taint_join_commutative(a: Taint, b: Taint) {
        assert_eq!(join(a, b), join(b, a));
    }
}

// Property: join is idempotent
proptest! {
    fn taint_join_idempotent(a: Taint) {
        assert_eq!(join(a, a), a);
    }
}

// Property: Clean is identity
proptest! {
    fn taint_join_identity(a: Taint) {
        assert_eq!(join(Clean, a), a);
        assert_eq!(join(a, Clean), a);
    }
}

// Property: Secret is absorbing
proptest! {
    fn taint_join_secret_absorbing(a: Taint) {
        assert_eq!(join(Secret, a), Secret);
    }
}
```

### PI-2: StepBudget Monotonicity

```rust
proptest! {
    fn step_budget_never_increases(remaining: u32, amount: u32) {
        let mut budget = StepBudget::new(remaining);
        let before = budget.remaining();
        let _ = budget.try_take(amount);
        let after = budget.remaining();
        prop_assert!(after <= before);
    }
}
```

### PI-3: FiniteF64 Validity

```rust
proptest! {
    fn finite_f64_roundtrip(v: FiniteF64) {
        let raw = v.into_raw();
        let recovered = FiniteF64::new(raw);
        prop_assert!(recovered.is_ok());
    }
}

proptest! {
    fn nan_rejected(v in nan_strategy()) {
        prop_assert!(FiniteF64::new(v).is_err());
    }
}
```

### PI-4: RunFrame Dimension Stability

```rust
proptest! {
    fn frame_dimensions_immutable_after_reinit(
        run_id: RunId,
        first_step: StepIdx,
        step_count: u16,
        slot_count: u16,
    ) {
        let mut frame = RunFrame::new(run_id, first_step, step_count, slot_count)?;
        let original_step_count = frame.step_count;
        let original_slot_count = frame.slot_count;
        frame.reinitialize(run_id, first_step, step_count, slot_count)?;
        prop_assert_eq!(frame.step_count, original_step_count);
        prop_assert_eq!(frame.slot_count, original_slot_count);
    }
}
```

### PI-5: Idempotency Key Well-Formed Bounds

```rust
proptest! {
    fn idempotency_key_valid_keys_accepted(key: IdempotencyKey) {
        if key.as_bytes().len() > 0 && key.as_bytes().len() <= 64 {
            prop_assert!(idempotency_key_well_formed(&key));
        }
    }
}

proptest! {
    fn idempotency_key_empty_rejected() {
        prop_assert!(!idempotency_key_well_formed(&IdempotencyKey::default()));
    }
}
```

### PI-6: Resource Budget Boundedness

```rust
proptest! {
    fn workflow_budget_always_within_policy(nodes: Vec<CompiledNode>, entry: StepIdx) {
        let budget = WholeWorkflowBudget::compute(&nodes, entry, &Default::default())?;
        let policy = BoundednessPolicy::DEFAULT;
        // all fields bounded by policy
        prop_assert!(budget.step_count <= policy.max_step_count);
        prop_assert!(budget.slot_count <= policy.max_slot_count);
    }
}
```

### PI-7: StepState Transition Matrix Exhaustive

```rust
proptest! {
    fn all_valid_transitions_accepted(from: StepState, to: StepState) {
        if is_valid_transition(from, to) {
            let mut frame = RunFrame::new(...);
            frame.transition_state(to)?;
            prop_assert_eq!(frame.state(), to);
        }
    }
}

proptest! {
    fn all_invalid_transitions_rejected(from: StepState, to: StepState) {
        if !is_valid_transition(from, to) {
            let mut frame = RunFrame::new(...);
            prop_assert!(frame.transition_state(to).is_err());
        }
    }
}
```

### PI-8: Record Decoder Rejects-Before-Alloc

```rust
proptest! {
    fn record_decode_always_validates_before_allocation(record: ArbitraryRecord) {
        let bytes = record.encode();
        let before = get_heap_allocated_bytes();
        let result = RecordDecoder::decode(&bytes);
        let after = get_heap_allocated_bytes();
        // If decode failed, heap should not have grown
        if result.is_err() {
            prop_assert_eq!(after, before);
        }
    }
}
```

### PI-9: IPC Decoder Rejects-Before-Alloc

```rust
proptest! {
    fn ipc_decode_always_validates_before_allocation(frame: ArbitraryIpcFrame) {
        let bytes = frame.encode();
        let before = get_heap_allocated_bytes();
        let result = IpcFrameDecoder::decode_frame(&bytes);
        let after = get_heap_allocated_bytes();
        if result.is_err() {
            prop_assert_eq!(after, before);
        }
    }
}
```

---

## 5. Fuzz Targets

### FT-1: ipc_decode (MISSING — formal waiver filed)

**Risk**: panics, OOM, logic errors in IPC header parsing.

**Existing compensating evidence**: decode_record fuzz (1M runs), expr_eval fuzz (500k runs), TLA+ ConcurrencyControl.

**Recommendation**: Write `fuzz/fuzz_targets/ipc_decode.rs` targeting `vb_ipc::decode_frame_header` with:
- Corpus seeds: minimal valid header, header at each boundary (59, 60, 61 bytes), max payload frames
- Mutation: flip bytes in header fields, corrupt magic, corrupt length fields

### FT-2: envelope_redaction (existing via VB-UI-MODEL-envelope-001)

Already covered by 18 nextest tests plus `cargo fuzz run ui_redaction_artifact`.

---

## 6. Kani Harnesses (Missing — Formal Waivers Filed)

All 14 missing Kani harnesses have compensating evidence and formal waivers. Integration test coverage for each:

| Missing Harness | Compensating Coverage | Integration Gap Fill |
|-----------------|----------------------|---------------------|
| `kani_taint_propagation` | Verus taint_lattice (13 verified) + `test_taint_eval_expr_join` | Gap F (IT-4 above) |
| `kani_step_budget_zero/one` | Verus step_budget (6 verified) + Gap K | Gap K + IT-5 |
| `kani_step_budget` | Verus + proptest | Gap K |
| `kani_index_access` | Verus + clippy clean | INV-009 gap: `test_checked_index_validates_bounds` |
| `kani_resource_budget_bounded` | Verus resource_budget + `resource_policy` | Gap M + IT-5 |
| `kani_ipc_header` (x2) | TLA+ + decode_record fuzz | Gap F + IT-2 |
| `kani_ipc_header_rejects_oversize` | TLA+ | Gap F |
| `kani_record_magic/schema/kind/payload_len/crc` | decode_record fuzz (1M) | Gap G + IT-2 |
| `kani_expr_stack` | expr_eval fuzz (500k) | Gap A + IT-4 |

---

## 7. Mutation Checkpoints

Threshold: ≥90% mutation kill rate.

### Critical Mutations

| Function | Mutation | Must Be Caught By |
|----------|----------|-------------------|
| `join_taint` | Remove Secretabsorbing case | `test_taint_no_downgrade_Secret` |
| `join_taint` | Remove DerivedFromSecret absorbing | `test_taint_no_downgrade_DerivedFromSecret` |
| `StepBudget::try_take` | Allow underflow | `test_step_budget_try_take_no_underflow` |
| `RunFrame::reinitialize` | Allow dimension change | `test_frame_dimension_immutable_after_reinitialize` |
| `FiniteF64::new` | Accept NaN | `test_finite_f64_rejects_nan` |
| `EngineSignal::Finished` | Omit Taint field | `test_finish_signal_carries_taint` |
| `RecordDecoder::decode` | Skip CRC check | `test_record_crc_validation` |
| `RecordDecoder::decode` | Skip magic check | `test_record_magic_validation` |
| `IpcFrameDecoder::decode_header` | Skip header_len check | `test_ipc_header_validation_rejects_short` |
| `IpcFrameDecoder::decode_frame` | Skip payload_len check | `test_ipc_header_validation_rejects_oversize` |

---

## 8. E2E Smoke Tests

### E2E-1: Full Workflow Execution

```bash
# GIVEN: a minimal compiled workflow with 3 steps
# WHEN:  velvet-ballistics executes the workflow
# THEN:  exit code 0, correct slot values in output
cargo run -- --workflow test-workflows/minimal.yaml
```

### E2E-2: Budget Exhaustion Signal

```bash
# GIVEN: a workflow with step_budget = 1
# WHEN:  workflow requires > 1 step
# THEN:  engine signals StepBudgetExhausted, workflow fails gracefully
```

### E2E-3: IPC Frame Rejection

```bash
# GIVEN: an oversized IPC frame (> MAX_PAYLOAD)
# WHEN:  frame is submitted to engine
# THEN:  IpcError::PayloadTooLarge returned, no crash
```

### E2E-4: Envelope Redaction in CLI Output

```bash
# GIVEN: an envelope with sensitive data marked as Secret taint
# WHEN:  velvet-ballistics outputs the envelope as JSON
# THEN:  redacted fields are null or [REDACTED], not actual values
```

---

## 9. Open Questions

| # | Question | Owner | Resolution |
|---|----------|-------|------------|
| OQ-1 | Does `idempotency_key_well_formed` accept Unicode or ASCII-only? | spec | Ascii-only per contract; confirm or update |
| OQ-2 | What is `MAX_PAYLOAD` constant value for IPC frames? | vb_ipc | Needed for Gap F boundary test |
| OQ-3 | Is there a maximum retry count for journal replay? | vb_storage | Needed for Gap H |
| OQ-4 | What is the `BoundednessPolicy::DEFAULT` values for each field? | vb_core::budget | Needed for Gap M assertion values |
| OQ-5 | Does `ui_redaction_artifact` fuzz target exist? | fuzz/fuzz_targets | Need to confirm for FT-2 |
